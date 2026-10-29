//! Sparse installed decisions and a metadata-only change cursor. No commands are evaluated here.
use super::*;

#[derive(Debug)]
pub(crate) struct CollectionReplicaCursor {
    entries: BTreeMap<Vec<u8>, [u8; 32]>,
    retained: BTreeMap<MutationIdentity, [u8; 32]>,
    fences: BTreeMap<String, u64>,
}

#[derive(Debug)]
pub(crate) struct CollectionReplicaPatch {
    entries: Vec<(Vec<u8>, StoredValue)>,
    removed_entries: Vec<Vec<u8>>,
    retained: Vec<(MutationIdentity, (RetainedCommand, RetainedOutcome))>,
    removed_outcomes: Vec<MutationIdentity>,
    fences: Vec<(String, u64)>,
    removed_fences: Vec<String>,
    revision: u64,
    destroyed: bool,
    observed_time: u64,
}
impl CollectionReplicaPatch {
    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CollectionDeltaMeasure {
    pub owned_bytes: usize,
    pub cursor_bytes: usize,
    pub entry_upserts: usize,
    pub entry_deletions: usize,
    pub outcome_upserts: usize,
    pub outcome_deletions: usize,
    pub fence_upserts: usize,
    pub fence_deletions: usize,
}

struct HashWriter(Sha256);
impl fmt::Write for HashWriter {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.0.update(text.as_bytes());
        Ok(())
    }
}
fn fingerprint(value: &impl fmt::Debug) -> [u8; 32] {
    let mut writer = HashWriter(Sha256::new());
    fmt::Write::write_fmt(&mut writer, format_args!("{value:?}")).expect("hash writer");
    writer.0.finalize().into()
}

fn outcome_bytes(outcome: &RetainedOutcome) -> usize {
    match outcome {
        RetainedOutcome::ServicePending { result_bytes } => *result_bytes,
        RetainedOutcome::Point(_) => 64,
        RetainedOutcome::Replacements(items) => items.iter().fold(64usize, |sum, item| {
            sum.saturating_add(64).saturating_add(item.key.len())
        }),
        RetainedOutcome::Service(value) => value["items"].as_array().map_or(
            64usize.saturating_add(value["value"].as_array().map_or(0, Vec::len)),
            |items| {
                items.iter().fold(
                    64usize.saturating_add(value["value"].as_array().map_or(0, Vec::len)),
                    |sum, item| {
                        sum.saturating_add(64)
                            .saturating_add(item["key"].as_array().map_or(0, Vec::len))
                            .saturating_add(item["value"].as_array().map_or(0, Vec::len))
                    },
                )
            },
        ),
    }
}
fn command_bytes(command: &RetainedCommand) -> usize {
    match command {
        RetainedCommand::Service(_) => 32,
        RetainedCommand::Point(PointCommand::Put { key, value, .. }) => {
            key.len().saturating_add(value.len()).saturating_add(16)
        }
        RetainedCommand::Point(PointCommand::Remove { key }) => key.len().saturating_add(8),
        RetainedCommand::ReplaceAll(items) => items.iter().fold(16usize, |sum, item| {
            sum.saturating_add(32)
                .saturating_add(item.key.len())
                .saturating_add(item.value.len())
        }),
    }
}

impl ReferenceCollectionMap {
    pub(crate) fn service_retirement_delta_bytes(&self, client: &str, through: u64) -> usize {
        let removed = self
            .retained
            .keys()
            .filter(|identity| identity.client == client && identity.sequence <= through)
            .fold(0usize, |sum, identity| {
                sum.saturating_add(24).saturating_add(identity.client.len())
            });
        if removed == 0 && self.retirement_fences.get(client) == Some(&through) {
            0
        } else {
            removed.saturating_add(128).saturating_add(client.len())
        }
    }
    pub(crate) fn service_entry_metadata(
        &self,
        key: &[u8],
        now: u64,
    ) -> Option<(u64, Option<u64>)> {
        self.entries
            .get(key)
            .filter(|entry| entry.expires_at.is_none_or(|at| at > now))
            .map(|entry| (entry.version, entry.expires_at))
    }
    pub(crate) fn service_expiry_delta_key_bytes(
        &self,
        now: u64,
        work: usize,
        command: &crate::value_plane_service_075::ServiceCommand,
    ) -> usize {
        let maintenance = self
            .expiry_index
            .iter()
            .take_while(|(at, _)| *at <= now)
            .take(work)
            .fold(0usize, |sum, (_, key)| {
                sum.saturating_add(32).saturating_add(key.len())
            });
        std::iter::once(&command.key)
            .chain(command.keys.iter())
            .chain(command.entries.iter().map(|entry| &entry.key))
            .fold(maintenance, |sum, key| {
                if self
                    .entries
                    .get(key)
                    .is_some_and(|entry| entry.expires_at.is_some_and(|at| at <= now))
                {
                    sum.saturating_add(32).saturating_add(key.len())
                } else {
                    sum
                }
            })
    }
    pub(crate) fn service_cursor_bytes(&self) -> usize {
        let entries = self.entries.keys().fold(128usize, |sum, key| {
            sum.saturating_add(48).saturating_add(key.len())
        });
        let retained = self.retained.keys().fold(entries, |sum, id| {
            sum.saturating_add(64).saturating_add(id.client.len())
        });
        self.retirement_fences.keys().fold(retained, |sum, client| {
            sum.saturating_add(24).saturating_add(client.len())
        })
    }
    pub(crate) fn service_replica_cursor(&self) -> CollectionReplicaCursor {
        CollectionReplicaCursor {
            entries: self
                .entries
                .iter()
                .map(|(key, value)| (key.clone(), fingerprint(value)))
                .collect(),
            retained: self
                .retained
                .iter()
                .map(|(id, decision)| (id.clone(), fingerprint(decision)))
                .collect(),
            fences: self.retirement_fences.clone(),
        }
    }
    pub(crate) fn service_delta_measure(
        &self,
        cursor: &CollectionReplicaCursor,
    ) -> CollectionDeltaMeasure {
        let mut measured = CollectionDeltaMeasure {
            owned_bytes: 128,
            cursor_bytes: self.service_cursor_bytes(),
            ..CollectionDeltaMeasure::default()
        };
        for (key, value) in &self.entries {
            if cursor.entries.get(key) != Some(&fingerprint(value)) {
                measured.entry_upserts += 1;
                measured.owned_bytes = measured
                    .owned_bytes
                    .saturating_add(24)
                    .saturating_add(key.len())
                    .saturating_add(value.value.len());
            }
        }
        for key in cursor.entries.keys() {
            if !self.entries.contains_key(key) {
                measured.entry_deletions += 1;
                measured.owned_bytes = measured
                    .owned_bytes
                    .saturating_add(16)
                    .saturating_add(key.len());
            }
        }
        for (id, decision) in &self.retained {
            if cursor.retained.get(id) != Some(&fingerprint(decision)) {
                measured.outcome_upserts += 1;
                measured.owned_bytes = measured
                    .owned_bytes
                    .saturating_add(64)
                    .saturating_add(id.client.len())
                    .saturating_add(command_bytes(&decision.0))
                    .saturating_add(outcome_bytes(&decision.1));
            }
        }
        for id in cursor.retained.keys() {
            if !self.retained.contains_key(id) {
                measured.outcome_deletions += 1;
                measured.owned_bytes = measured
                    .owned_bytes
                    .saturating_add(24)
                    .saturating_add(id.client.len());
            }
        }
        for (client, through) in &self.retirement_fences {
            if cursor.fences.get(client) != Some(through) {
                measured.fence_upserts += 1;
                measured.owned_bytes = measured
                    .owned_bytes
                    .saturating_add(24)
                    .saturating_add(client.len());
            }
        }
        for client in cursor.fences.keys() {
            if !self.retirement_fences.contains_key(client) {
                measured.fence_deletions += 1;
                measured.owned_bytes = measured
                    .owned_bytes
                    .saturating_add(16)
                    .saturating_add(client.len());
            }
        }
        measured
    }
    pub(crate) fn service_delta_debug<'a>(
        &'a self,
        cursor: &'a CollectionReplicaCursor,
    ) -> impl fmt::Debug + 'a {
        BorrowedDelta { map: self, cursor }
    }
    pub(crate) fn service_capture_delta(
        &self,
        cursor: &mut CollectionReplicaCursor,
    ) -> CollectionReplicaPatch {
        let patch = CollectionReplicaPatch {
            entries: self
                .entries
                .iter()
                .filter(|(key, value)| cursor.entries.get(*key) != Some(&fingerprint(*value)))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            removed_entries: cursor
                .entries
                .keys()
                .filter(|key| !self.entries.contains_key(*key))
                .cloned()
                .collect(),
            retained: self
                .retained
                .iter()
                .filter(|(id, decision)| cursor.retained.get(*id) != Some(&fingerprint(*decision)))
                .map(|(id, decision)| (id.clone(), decision.clone()))
                .collect(),
            removed_outcomes: cursor
                .retained
                .keys()
                .filter(|id| !self.retained.contains_key(*id))
                .cloned()
                .collect(),
            fences: self
                .retirement_fences
                .iter()
                .filter(|(client, through)| cursor.fences.get(*client) != Some(*through))
                .map(|(client, through)| (client.clone(), *through))
                .collect(),
            removed_fences: cursor
                .fences
                .keys()
                .filter(|client| !self.retirement_fences.contains_key(*client))
                .cloned()
                .collect(),
            revision: self.revision,
            destroyed: self.destroyed,
            observed_time: self.observed_time,
        };
        // Delete metadata before adding it, so the cursor never exceeds max(old,new) size.
        for key in &patch.removed_entries {
            cursor.entries.remove(key);
        }
        for id in &patch.removed_outcomes {
            cursor.retained.remove(id);
        }
        for client in &patch.removed_fences {
            cursor.fences.remove(client);
        }
        for (key, value) in &patch.entries {
            if let Some(hash) = cursor.entries.get_mut(key) {
                *hash = fingerprint(value);
            } else {
                cursor.entries.insert(key.clone(), fingerprint(value));
            }
        }
        for (id, decision) in &patch.retained {
            if let Some(hash) = cursor.retained.get_mut(id) {
                *hash = fingerprint(decision);
            } else {
                cursor.retained.insert(id.clone(), fingerprint(decision));
            }
        }
        for (client, through) in &patch.fences {
            if let Some(old) = cursor.fences.get_mut(client) {
                *old = *through;
            } else {
                cursor.fences.insert(client.clone(), *through);
            }
        }
        patch
    }
    pub(crate) fn service_replication_debug(&self) -> impl fmt::Debug + '_ {
        ProjectedState {
            map: self,
            patch: None,
        }
    }
    pub(crate) fn service_projected_debug<'a>(
        &'a self,
        patch: &'a CollectionReplicaPatch,
    ) -> impl fmt::Debug + 'a {
        ProjectedState {
            map: self,
            patch: Some(patch),
        }
    }
    pub(crate) fn service_index_valid(&self) -> bool {
        self.expiry_index.len()
            == self
                .entries
                .values()
                .filter(|value| value.expires_at.is_some())
                .count()
            && self.expiry_index.iter().all(|(at, key)| {
                self.entries
                    .get(key)
                    .is_some_and(|value| value.expires_at == Some(*at))
            })
    }
    pub(crate) fn service_validate_delta(
        &self,
        patch: &CollectionReplicaPatch,
    ) -> Result<(), CollectionError> {
        if patch.revision < self.revision
            || patch.observed_time < self.observed_time
            || (self.destroyed && !patch.destroyed)
        {
            return Err(CollectionError::CursorMismatch);
        }
        if !ordered_pairs(&patch.entries)
            || !ordered(&patch.removed_entries)
            || !ordered_pairs(&patch.retained)
            || !ordered(&patch.removed_outcomes)
            || !ordered_pairs(&patch.fences)
            || !ordered(&patch.removed_fences)
        {
            return Err(CollectionError::DuplicateKey);
        }
        for (key, value) in &patch.entries {
            self.check_bound("key_bytes", self.bounds.max_key_bytes, key.len())?;
            self.check_bound(
                "value_bytes",
                self.bounds.max_value_bytes,
                value.value.len(),
            )?;
            if value.version == 0
                || value.version > patch.revision
                || patch.removed_entries.binary_search(key).is_ok()
            {
                return Err(CollectionError::CursorMismatch);
            }
        }
        for (id, record) in &patch.retained {
            self.validate_mutation_identity(id)?;
            if matches!(record.1, RetainedOutcome::ServicePending { .. })
                || patch.removed_outcomes.binary_search(id).is_ok()
                || self
                    .retained
                    .get(id)
                    .is_some_and(|old| fingerprint(old) != fingerprint(record))
            {
                return Err(CollectionError::CursorMismatch);
            }
        }
        for (client, through) in &patch.fences {
            if identity_blank(client)
                || client.len() > 128
                || *through == 0
                || self
                    .retirement_fences
                    .get(client)
                    .is_some_and(|old| *old > *through)
                || patch.removed_fences.binary_search(client).is_ok()
            {
                return Err(CollectionError::CursorMismatch);
            }
        }
        if !patch.destroyed && !patch.removed_fences.is_empty() {
            return Err(CollectionError::CursorMismatch);
        }
        let (entries, retained, _, _, _, _) = self.service_projected_counts(patch);
        self.check_bound("entries", self.bounds.max_entries, entries)?;
        self.check_bound("retained_mutations", self.bounds.max_entries, retained)?;
        if patch.destroyed
            && (entries != 0 || retained != 0 || self.service_projected_counts(patch).2 != 0)
        {
            return Err(CollectionError::CursorMismatch);
        }
        Ok(())
    }
    /// entries, retained records, fences, values bytes, completed bytes, pending bytes.
    pub(crate) fn service_projected_counts(
        &self,
        patch: &CollectionReplicaPatch,
    ) -> (usize, usize, usize, usize, usize, usize) {
        let mut entries = 0usize;
        let mut values = 0usize;
        walk(
            &self.entries,
            &patch.entries,
            &patch.removed_entries,
            |key, value| {
                entries += 1;
                values = values
                    .saturating_add(24)
                    .saturating_add(key.len())
                    .saturating_add(value.value.len());
            },
        );
        let mut records = 0usize;
        let mut completed = 0usize;
        let mut pending = 0usize;
        walk(
            &self.retained,
            &patch.retained,
            &patch.removed_outcomes,
            |id, (_, outcome)| {
                records += 1;
                let base = id.client.len().saturating_add(48);
                if matches!(outcome, RetainedOutcome::ServicePending { .. }) {
                    pending = pending.saturating_add(base);
                }
                completed = completed
                    .saturating_add(base)
                    .saturating_add(outcome_bytes(outcome));
            },
        );
        let mut fences = 0usize;
        walk(
            &self.retirement_fences,
            &patch.fences,
            &patch.removed_fences,
            |client, _| {
                fences += 1;
                completed = completed.saturating_add(24).saturating_add(client.len());
            },
        );
        (entries, records, fences, values, completed, pending)
    }
    pub(crate) fn service_install_delta(&mut self, patch: CollectionReplicaPatch) {
        for key in patch.removed_entries {
            if let Some(value) = self.entries.remove(&key) {
                if let Some(at) = value.expires_at {
                    self.expiry_index.remove(&(at, key));
                }
            }
        }
        for id in patch.removed_outcomes {
            self.retained.remove(&id);
        }
        for client in patch.removed_fences {
            self.retirement_fences.remove(&client);
        }
        for (key, value) in patch.entries {
            if let Some(at) = self.entries.get(&key).and_then(|old| old.expires_at) {
                self.expiry_index.remove(&(at, key.clone()));
            }
            if let Some(at) = value.expires_at {
                self.expiry_index.insert((at, key.clone()));
            }
            self.entries.insert(key, value);
        }
        for (id, record) in patch.retained {
            self.retained.insert(id, record);
        }
        for (client, through) in patch.fences {
            self.retirement_fences.insert(client, through);
        }
        self.revision = patch.revision;
        self.destroyed = patch.destroyed;
        self.observed_time = patch.observed_time;
    }
}

fn ordered<K: Ord>(keys: &[K]) -> bool {
    keys.windows(2).all(|pair| pair[0] < pair[1])
}
fn ordered_pairs<K: Ord, V>(pairs: &[(K, V)]) -> bool {
    pairs.windows(2).all(|pair| pair[0].0 < pair[1].0)
}
fn walk<K: Ord, V>(
    base: &BTreeMap<K, V>,
    upserts: &[(K, V)],
    removed: &[K],
    mut visitor: impl FnMut(&K, &V),
) {
    let mut old = base.iter().peekable();
    let mut new = upserts.iter().peekable();
    loop {
        match (old.peek(), new.peek()) {
            (None, None) => break,
            (Some((key, _)), Some((inserted, _))) if *key >= inserted => {
                if *key == inserted {
                    old.next();
                }
                let (key, value) = new.next().expect("peeked insertion");
                visitor(key, value);
            }
            (Some(_), _) => {
                let (key, value) = old.next().expect("peeked base");
                if removed.binary_search(key).is_err() {
                    visitor(key, value);
                }
            }
            (None, Some(_)) => {
                let (key, value) = new.next().expect("peeked insertion");
                visitor(key, value);
            }
        }
    }
}
struct ProjectedMap<'a, K, V> {
    base: &'a BTreeMap<K, V>,
    upserts: &'a [(K, V)],
    removed: &'a [K],
}
impl<K: Ord + fmt::Debug, V: fmt::Debug> fmt::Debug for ProjectedMap<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut map = f.debug_map();
        walk(self.base, self.upserts, self.removed, |key, value| {
            map.entry(key, value);
        });
        map.finish()
    }
}
struct ProjectedState<'a> {
    map: &'a ReferenceCollectionMap,
    patch: Option<&'a CollectionReplicaPatch>,
}
impl fmt::Debug for ProjectedState<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let map = self.map;
        let mut state = f.debug_struct("ReferenceCollectionReplicaState");
        state.field("bounds", &map.bounds);
        if let Some(patch) = self.patch {
            state
                .field(
                    "entries",
                    &ProjectedMap {
                        base: &map.entries,
                        upserts: &patch.entries,
                        removed: &patch.removed_entries,
                    },
                )
                .field("revision", &patch.revision)
                .field("destroyed", &patch.destroyed)
                .field("map_id", &map.map_id)
                .field(
                    "retained",
                    &ProjectedMap {
                        base: &map.retained,
                        upserts: &patch.retained,
                        removed: &patch.removed_outcomes,
                    },
                )
                .field("observed_time", &patch.observed_time)
                .field(
                    "retirement_fences",
                    &ProjectedMap {
                        base: &map.retirement_fences,
                        upserts: &patch.fences,
                        removed: &patch.removed_fences,
                    },
                );
        } else {
            state
                .field("entries", &map.entries)
                .field("revision", &map.revision)
                .field("destroyed", &map.destroyed)
                .field("map_id", &map.map_id)
                .field("retained", &map.retained)
                .field("observed_time", &map.observed_time)
                .field("retirement_fences", &map.retirement_fences);
        }
        state.finish()
    }
}

struct BorrowedDelta<'a> {
    map: &'a ReferenceCollectionMap,
    cursor: &'a CollectionReplicaCursor,
}
impl fmt::Debug for BorrowedDelta<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct List<F>(F);
        impl<F: Fn(&mut fmt::Formatter<'_>) -> fmt::Result> fmt::Debug for List<F> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                (self.0)(f)
            }
        }
        let m = self.map;
        let c = self.cursor;
        f.debug_struct("CollectionReplicaPatch")
            .field(
                "entries",
                &List(|f: &mut fmt::Formatter<'_>| {
                    let mut list = f.debug_list();
                    for (key, value) in &m.entries {
                        if c.entries.get(key) != Some(&fingerprint(value)) {
                            list.entry(&(key, value));
                        }
                    }
                    list.finish()
                }),
            )
            .field(
                "removed_entries",
                &List(|f: &mut fmt::Formatter<'_>| {
                    let mut list = f.debug_list();
                    for key in c.entries.keys() {
                        if !m.entries.contains_key(key) {
                            list.entry(key);
                        }
                    }
                    list.finish()
                }),
            )
            .field(
                "retained",
                &List(|f: &mut fmt::Formatter<'_>| {
                    let mut list = f.debug_list();
                    for (id, record) in &m.retained {
                        if c.retained.get(id) != Some(&fingerprint(record)) {
                            list.entry(&(id, record));
                        }
                    }
                    list.finish()
                }),
            )
            .field(
                "removed_outcomes",
                &List(|f: &mut fmt::Formatter<'_>| {
                    let mut list = f.debug_list();
                    for id in c.retained.keys() {
                        if !m.retained.contains_key(id) {
                            list.entry(id);
                        }
                    }
                    list.finish()
                }),
            )
            .field(
                "fences",
                &List(|f: &mut fmt::Formatter<'_>| {
                    let mut list = f.debug_list();
                    for (client, through) in &m.retirement_fences {
                        if c.fences.get(client) != Some(through) {
                            list.entry(&(client, through));
                        }
                    }
                    list.finish()
                }),
            )
            .field(
                "removed_fences",
                &List(|f: &mut fmt::Formatter<'_>| {
                    let mut list = f.debug_list();
                    for client in c.fences.keys() {
                        if !m.retirement_fences.contains_key(client) {
                            list.entry(client);
                        }
                    }
                    list.finish()
                }),
            )
            .field("revision", &m.revision)
            .field("destroyed", &m.destroyed)
            .field("observed_time", &m.observed_time)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn changed() -> (ReferenceCollectionMap, CollectionReplicaPatch) {
        let mut map = ReferenceCollectionMap::new(CollectionBounds {
            max_key_bytes: 4,
            max_value_bytes: 4,
            max_entries: 8,
            ..CollectionBounds::default()
        })
        .unwrap();
        map.service_mode = true;
        let mut cursor = map.service_replica_cursor();
        map.put(b"a".to_vec(), b"one".to_vec(), Some(5), 0).unwrap();
        let patch = map.service_capture_delta(&mut cursor);
        let mut base = map.service_checkpoint_clone();
        base.entries.clear();
        base.expiry_index.clear();
        base.revision = 0;
        (base, patch)
    }
    #[test]
    fn sparse_patch_rejects_malformed_bounds_versions_order_pending_and_fences() {
        let (base, mut patch) = changed();
        patch.entries.push(patch.entries[0].clone());
        assert_eq!(
            base.service_validate_delta(&patch),
            Err(CollectionError::DuplicateKey)
        );
        let (base, mut patch) = changed();
        patch.entries[0].1.version = 0;
        assert_eq!(
            base.service_validate_delta(&patch),
            Err(CollectionError::CursorMismatch)
        );
        let (base, mut patch) = changed();
        patch.entries[0].0 = vec![1; 5];
        assert!(matches!(
            base.service_validate_delta(&patch),
            Err(CollectionError::BoundExceeded { .. })
        ));
        let (base, mut patch) = changed();
        patch.entries[0].1.value = vec![1; 5];
        assert!(matches!(
            base.service_validate_delta(&patch),
            Err(CollectionError::BoundExceeded { .. })
        ));
        let (base, mut patch) = changed();
        patch.removed_entries.push(b"a".to_vec());
        assert_eq!(
            base.service_validate_delta(&patch),
            Err(CollectionError::CursorMismatch)
        );
        let (base, mut patch) = changed();
        patch.retained.push((
            MutationIdentity::new("pending", 1),
            (
                RetainedCommand::Service([0; 32]),
                RetainedOutcome::ServicePending { result_bytes: 64 },
            ),
        ));
        assert_eq!(
            base.service_validate_delta(&patch),
            Err(CollectionError::CursorMismatch)
        );
        let (mut base, mut patch) = changed();
        base.retirement_fences.insert("client".into(), 3);
        patch.fences.push(("client".into(), 2));
        assert_eq!(
            base.service_validate_delta(&patch),
            Err(CollectionError::CursorMismatch)
        );
        let (mut base, mut patch) = changed();
        base.retirement_fences.insert("client".into(), 3);
        patch.removed_fences.push("client".into());
        assert_eq!(
            base.service_validate_delta(&patch),
            Err(CollectionError::CursorMismatch)
        );
        let (base, mut patch) = changed();
        patch.destroyed = true;
        assert_eq!(
            base.service_validate_delta(&patch),
            Err(CollectionError::CursorMismatch)
        );
    }

    #[test]
    fn sparse_patch_projection_matches_install_and_index_and_rejects_ledger_rewrites() {
        let (mut base, mut patch) = changed();
        let identity = MutationIdentity::new("receipt", 1);
        let record = (
            RetainedCommand::Service([1; 32]),
            RetainedOutcome::Service(serde_json::json!({"kind":"applied"})),
        );
        base.retained.insert(identity.clone(), record.clone());
        let mut rewritten = record;
        rewritten.0 = RetainedCommand::Service([2; 32]);
        patch.retained.push((identity, rewritten));
        assert_eq!(
            base.service_validate_delta(&patch),
            Err(CollectionError::CursorMismatch)
        );
        patch.retained.clear();
        base.service_validate_delta(&patch).unwrap();
        let projected = fingerprint(&base.service_projected_debug(&patch));
        base.service_install_delta(patch);
        assert_eq!(fingerprint(&base.service_replication_debug()), projected);
        assert!(base.service_index_valid());
        base.expiry_index.clear();
        assert!(!base.service_index_valid());
    }
}
