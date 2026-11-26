use super::delta::consecutive;
use super::*;
use crate::imap_value_repair::{
    bytes_digest, is_prefix_at_most, repair_error, version, RepairManifest, RepairReceipt,
    RepairRequest, RepairStep, MAX_DELTA_ROWS, MAX_IMAGE_BYTES, MAX_IMAGE_ROWS,
};

const TRANSFER_KEY: &[u8] = b"repair";

#[derive(Serialize, Deserialize)]
struct PartitionImage {
    meta: PartitionJournal,
    rows: Vec<ImageRow>,
}

#[derive(Serialize, Deserialize)]
struct ImageRow {
    key: String,
    value: JournalRecord,
}

impl ValueJournal {
    /// Capture under the source partition lane while its writer permit is held.
    pub(crate) fn export_image(
        &self,
        partition: PartitionId,
        required: Option<PartitionValuePrefix>,
    ) -> CacheResult<Vec<u8>> {
        let stored = self.0.get(meta_key(partition)).map_err(storage_error)?;
        if stored
            .as_ref()
            .is_some_and(|bytes| bytes.len() > MAX_IMAGE_BYTES)
        {
            return Err(repair_error("partition journal exceeds the snapshot limit"));
        }
        let mut meta: PartitionJournal = stored
            .map(|bytes| serde_json::from_slice(&bytes).map_err(storage_error))
            .transpose()?
            .unwrap_or_default();
        if !meta.covers(required) {
            return Err(repair_error("owner lacks the certified prefix"));
        }
        // A certified prefix's image is stable across journal materialization.
        let materialized = meta
            .materialize_pending(required)
            .filter(|row| !row.outcome_only);
        meta.pending = None;
        let records = record_key(partition, "");
        let mut size = serde_json::to_vec(&meta).map_err(storage_error)?.len();
        let mut rows = Vec::new();
        for pair in self.0.scan_prefix(&records) {
            let (key, value) = pair.map_err(storage_error)?;
            let key = std::str::from_utf8(&key[records.len()..]).map_err(storage_error)?;
            if materialized.as_ref().is_some_and(|row| row.key == key) {
                continue;
            }
            size += key.len() + value.len();
            if size > MAX_IMAGE_BYTES || rows.len() == MAX_IMAGE_ROWS {
                return Err(repair_error("partition exceeds the staged snapshot limits"));
            }
            rows.push(ImageRow {
                key: key.to_owned(),
                value: serde_json::from_slice(&value).map_err(storage_error)?,
            });
        }
        if let Some(row) = materialized {
            if rows.len() == MAX_IMAGE_ROWS {
                return Err(repair_error("partition exceeds the staged snapshot limits"));
            }
            let index = rows.partition_point(|prior| prior.key < row.key);
            let value = JournalRecord::V2(JournalRecordV2::from_mutation(&row));
            rows.insert(
                index,
                ImageRow {
                    key: row.key,
                    value,
                },
            );
        }
        let bytes = serde_json::to_vec(&PartitionImage { meta, rows }).map_err(storage_error)?;
        if bytes.len() > MAX_IMAGE_BYTES {
            return Err(repair_error("encoded partition exceeds the snapshot limit"));
        }
        Ok(bytes)
    }

    /// Receive under the staging and partition locks. The caller validates
    /// the current placement before publishing or acknowledging an image.
    pub(crate) fn receive_image(
        &self,
        request: RepairRequest,
        admits_key: impl Fn(&str) -> bool,
    ) -> CacheResult<RepairReceipt> {
        let RepairRequest { manifest, step } = request;
        if !manifest.is_bounded() {
            return Err(repair_error("invalid snapshot size"));
        }
        let chunks = manifest.chunks();
        let mut previous = self
            .0
            .get(TRANSFER_KEY)
            .map_err(storage_error)?
            .map(|bytes| serde_json::from_slice::<RepairReceipt>(&bytes).map_err(storage_error))
            .transpose()?;
        if let Some(prior) = previous.as_mut().filter(|prior| prior.installed) {
            // Snapshot-only receipts have no separate delta cursor.
            prior.applied = prior.applied.or(prior.manifest.plan.required);
        }
        if previous.as_ref().is_some_and(|prior| {
            !prior.manifest.is_bounded()
                || prior.next > prior.manifest.chunks()
                || (prior.installed && prior.next != prior.manifest.chunks())
                || (prior.installed
                    && !is_prefix_at_most(prior.manifest.plan.required, prior.applied))
                || (!prior.installed && prior.applied.is_some())
        }) {
            return Err(repair_error("invalid persisted transfer cursor"));
        }
        let mut batch = sled::Batch::default();
        let mut state = match previous {
            Some(prior) if prior.manifest == manifest => prior,
            prior
                if matches!(step, RepairStep::Status)
                    && prior
                        .as_ref()
                        .is_none_or(|prior| supersedes(prior, &manifest)) =>
            {
                if let Some(prior) = prior {
                    remove_chunks(&mut batch, prior.next);
                }
                RepairReceipt {
                    manifest,
                    next: 0,
                    installed: false,
                    applied: None,
                }
            }
            _ => return Err(repair_error("conflicting or missing staged manifest")),
        };
        match step {
            RepairStep::Status => {}
            RepairStep::Chunk {
                index,
                digest,
                bytes,
            } => {
                if state.installed
                    || index > state.next
                    || state.manifest.chunk_len(index) != Some(bytes.len())
                    || bytes_digest(&bytes) != digest
                {
                    return Err(repair_error("chunk does not match the manifest or cursor"));
                }
                let key = chunk_key(index);
                if index == state.next {
                    batch.insert(&key, bytes);
                    state.next += 1;
                } else if self.0.get(key).map_err(storage_error)?.as_deref()
                    != Some(bytes.as_slice())
                {
                    return Err(repair_error("conflicting duplicate chunk"));
                }
            }
            RepairStep::Install => {
                if state.next != chunks {
                    return Err(repair_error("snapshot is incomplete"));
                }
                if !state.installed {
                    let mut bytes = Vec::with_capacity(state.manifest.image_len);
                    for index in 0..chunks {
                        let chunk = self
                            .0
                            .get(chunk_key(index))
                            .map_err(storage_error)?
                            .ok_or_else(|| repair_error("a verified chunk is missing"))?;
                        if state.manifest.chunk_len(index) != Some(chunk.len()) {
                            return Err(repair_error("stored chunk has the wrong size"));
                        }
                        bytes.extend_from_slice(&chunk);
                    }
                    if bytes_digest(&bytes) != state.manifest.plan.snapshot_digest {
                        return Err(repair_error("snapshot root checksum mismatch"));
                    }
                    self.stage_image(&mut batch, &state.manifest, &bytes, admits_key)?;
                    remove_chunks(&mut batch, chunks);
                    state.installed = true;
                    state.applied = state.manifest.plan.required;
                }
            }
            RepairStep::Delta { mutation } => {
                let plan = &state.manifest.plan;
                let next = mutation.certificate();
                if !state.installed
                    || next.partition != plan.partition
                    || next.epoch != plan.old_epoch
                    || mutation.owner != plan.owner
                    || !admits_key(&mutation.key)
                    || next.version.saturating_sub(version(plan.required)) > MAX_DELTA_ROWS
                {
                    return Err(repair_error("delta is outside the installed transfer"));
                }
                if state.applied != Some(next) {
                    if !consecutive(state.applied, next) {
                        return Err(repair_error("delta does not extend the recipient cursor"));
                    }
                    let prior = state.applied;
                    state.applied = Some(next);
                    batch.insert(
                        TRANSFER_KEY,
                        serde_json::to_vec(&state).map_err(storage_error)?,
                    );
                    if self.apply_with_batch(prior, *mutation, false, batch)?
                        != ApplyResult::Flushed
                    {
                        return Err(repair_error("recipient journal refused the next delta"));
                    }
                    return Ok(state);
                }
                if !self.load(plan.partition)?.covers(Some(next)) {
                    return Err(repair_error(
                        "duplicate delta is missing from the recipient journal",
                    ));
                }
            }
            RepairStep::Seal { prefix } => {
                if !state.installed
                    || state.applied != prefix
                    || !self.load(state.manifest.plan.partition)?.covers(prefix)
                {
                    return Err(repair_error("recipient has not reached the final prefix"));
                }
            }
        }
        batch.insert(
            TRANSFER_KEY,
            serde_json::to_vec(&state).map_err(storage_error)?,
        );
        self.0.apply_batch(batch).map_err(storage_error)?;
        self.0.flush().map_err(storage_error)?;
        Ok(state)
    }

    fn stage_image(
        &self,
        batch: &mut sled::Batch,
        manifest: &RepairManifest,
        bytes: &[u8],
        admits_key: impl Fn(&str) -> bool,
    ) -> CacheResult<()> {
        let partition = manifest.plan.partition;
        let required = manifest.plan.required;
        let PartitionImage { meta, rows } = serde_json::from_slice(bytes).map_err(storage_error)?;
        if meta.pending.is_some()
            || meta.visible != required
            || rows.len() > MAX_IMAGE_ROWS
            || meta.outcomes.len() > RETAINED_IDENTITIES
            || meta
                .visible
                .is_some_and(|prefix| prefix.partition != partition)
            || (meta.visible.is_none() && (!rows.is_empty() || !meta.outcomes.is_empty()))
        {
            return Err(repair_error(
                "image does not cover the required partition prefix",
            ));
        }
        for prefix in [record_key(partition, ""), expiry_key(partition, "")] {
            for (index, key) in self.0.scan_prefix(prefix).keys().enumerate() {
                if index == MAX_IMAGE_ROWS {
                    return Err(repair_error(
                        "recipient partition exceeds the snapshot limits",
                    ));
                }
                batch.remove(key.map_err(storage_error)?);
            }
        }
        let mut prior = None;
        for ImageRow { key, value } in rows {
            if prior.as_ref().is_some_and(|previous| previous >= &key) || !admits_key(&key) {
                return Err(repair_error("invalid snapshot key order or scope"));
            }
            let encoded = serde_json::to_vec(&value).map_err(storage_error)?;
            let entry = value.into_entry(partition, meta.visible)?;
            if let Some(deadline) = entry.expires_not_before_ms {
                let timer = ExpiryCandidate {
                    partition,
                    key: key.clone(),
                    version: entry.record.version,
                    deadline,
                };
                batch.insert(
                    expiry_key(partition, &key),
                    serde_json::to_vec(&timer).map_err(storage_error)?,
                );
            }
            batch.insert(record_key(partition, &key), encoded);
            prior = Some(key);
        }
        batch.insert(
            &meta_key(partition),
            serde_json::to_vec(&meta).map_err(storage_error)?,
        );
        Ok(())
    }
}

fn supersedes(prior: &RepairReceipt, next: &RepairManifest) -> bool {
    let old = &prior.manifest.plan;
    let new = &next.plan;
    if old.old_epoch != new.old_epoch {
        return old.old_epoch < new.old_epoch;
    }
    if old.partition != new.partition {
        return prior.installed;
    }
    old.owner == new.owner
        && old.target == new.target
        && version(old.required) < version(new.required)
        && is_prefix_at_most(prior.applied, new.required)
}

fn chunk_key(index: u32) -> [u8; 5] {
    let [a, b, c, d] = index.to_be_bytes();
    [b'x', a, b, c, d]
}

fn remove_chunks(batch: &mut sled::Batch, count: u32) {
    for index in 0..count {
        batch.remove(&chunk_key(index));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydracache::{ClusterGeneration, ClusterNodeId, PartitionReplicaRepair};
    use std::time::SystemTime;

    fn journal_directory() -> std::path::PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "hc-w10f-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        directory
    }

    fn member(node: &str) -> PartitionMember {
        PartitionMember {
            node: ClusterNodeId::new(node),
            generation: ClusterGeneration::new(1),
        }
    }

    fn write(
        journal: &ValueJournal,
        required: Option<PartitionValuePrefix>,
        key: &str,
        value: Option<Vec<u8>>,
        deadline: Option<u64>,
    ) -> PartitionValuePrefix {
        let epoch = ClusterEpoch::new(2);
        let record = next_record(PartitionId::new(7), epoch, required, value).unwrap();
        let mutation = Mutation {
            owner: member("owner"),
            backups: vec![member("old")],
            previous: required.map_or([0; 32], |prefix| prefix.digest),
            key: key.into(),
            identity: Some(format!("request-{}", record.version)),
            decision: Some(CachedDecision {
                request: [0; 32],
                response: ClientResponse::CasApplied {
                    new_version: record.version,
                },
            }),
            outcome_only: false,
            record,
            expires_not_before_ms: deadline,
        };
        let prefix = mutation.certificate();
        assert_eq!(
            journal.apply(required, mutation, true).unwrap(),
            ApplyResult::Flushed
        );
        prefix
    }

    fn manifest(required: Option<PartitionValuePrefix>, bytes: &[u8]) -> RepairManifest {
        RepairManifest {
            plan: PartitionReplicaRepair {
                partition: PartitionId::new(7),
                old_epoch: ClusterEpoch::new(3),
                new_epoch: ClusterEpoch::new(4),
                owner: member("owner"),
                replaced: member("old"),
                target: member("replacement"),
                required,
                snapshot_digest: bytes_digest(bytes),
                prior_backups: None,
            },
            scope: [2; 32],
            image_len: bytes.len(),
        }
    }

    fn request(manifest: &RepairManifest, step: RepairStep) -> RepairRequest {
        RepairRequest {
            manifest: manifest.clone(),
            step,
        }
    }

    fn chunk(index: u32, bytes: &[u8]) -> RepairStep {
        RepairStep::Chunk {
            index,
            digest: bytes_digest(bytes),
            bytes: bytes.to_vec(),
        }
    }

    #[test]
    fn reopened_journal_resumes_verified_chunks_and_installs_only_certified_data() {
        let source = ValueJournal::temporary();
        let first = write(&source, None, "ttl", Some(vec![0xaa; 20 * 1024]), Some(90));
        let second = write(&source, Some(first), "removed", None, None);
        let prefix = write(
            &source,
            Some(second),
            "latest",
            Some(b"certified".to_vec()),
            None,
        );
        let image_with_certified_pending =
            source.export_image(prefix.partition, Some(prefix)).unwrap();
        let _uncertified = write(
            &source,
            Some(prefix),
            "latest",
            Some(b"uncertified".to_vec()),
            None,
        );
        let partition = prefix.partition;
        let image = source.export_image(partition, Some(prefix)).unwrap();
        assert_eq!(image, image_with_certified_pending);
        let manifest = manifest(Some(prefix), &image);
        assert!(manifest.chunks() > 1);
        assert_eq!(image, source.export_image(partition, Some(prefix)).unwrap());
        assert!(source.export_image(partition, Some(second)).is_err());

        let directory = journal_directory();
        let accept = |key: &str| matches!(key, "ttl" | "removed" | "latest");
        let target = ValueJournal::open(&directory).unwrap();
        target.bind_scope("tenant", "ns", 1).unwrap();
        let parts: Vec<_> = image
            .chunks(crate::imap_value_repair::CHUNK_BYTES)
            .collect();
        assert_eq!(
            target
                .receive_image(request(&manifest, RepairStep::Status), accept)
                .unwrap()
                .next,
            0
        );
        assert!(target
            .receive_image(request(&manifest, chunk(1, parts[1])), accept)
            .is_err());
        assert_eq!(
            target
                .receive_image(request(&manifest, chunk(0, parts[0])), accept)
                .unwrap()
                .next,
            1
        );
        assert!(target
            .receive_image(request(&manifest, RepairStep::Install), accept)
            .is_err());
        assert!(!target.load(partition).unwrap().covers(Some(prefix)));
        drop(target);

        let target = ValueJournal::open(&directory).unwrap();
        target.bind_scope("tenant", "ns", 1).unwrap();
        assert_eq!(
            target
                .receive_image(request(&manifest, RepairStep::Status), accept)
                .unwrap()
                .next,
            1
        );
        assert_eq!(
            target
                .receive_image(request(&manifest, chunk(0, parts[0])), accept)
                .unwrap()
                .next,
            1
        );
        let mut wrong = parts[0].to_vec();
        wrong[0] ^= 1;
        assert!(target
            .receive_image(request(&manifest, chunk(0, &wrong)), accept)
            .is_err());
        for (index, bytes) in parts.iter().enumerate().skip(1) {
            let index = index as u32;
            assert_eq!(
                target
                    .receive_image(request(&manifest, chunk(index, bytes)), accept)
                    .unwrap()
                    .next,
                index + 1
            );
        }
        assert!(target
            .receive_image(request(&manifest, RepairStep::Install), |_| false)
            .is_err());
        assert!(!target.load(partition).unwrap().covers(Some(prefix)));
        for _ in 0..2 {
            assert!(
                target
                    .receive_image(request(&manifest, RepairStep::Install), accept)
                    .unwrap()
                    .installed
            );
        }
        assert!(target.0.scan_prefix(b"x").next().is_none());
        let meta = target.load(partition).unwrap();
        assert!(meta.covers(Some(prefix)));
        assert!(meta.pending.is_none());
        assert_eq!(
            meta.retained(Some(prefix), "request-3").unwrap().response,
            ClientResponse::CasApplied { new_version: 3 }
        );
        assert!(meta.retained(Some(prefix), "request-4").is_none());
        let latest = target
            .read_entry(partition, "latest", &meta, Some(prefix))
            .unwrap()
            .unwrap();
        assert_eq!(latest.value(), Some(b"certified".as_slice()));
        let removed = target
            .read_entry(partition, "removed", &meta, Some(prefix))
            .unwrap()
            .unwrap();
        assert!(removed.record.is_tombstone());
        let expiries = target.expiry_batch(&mut None, 90, 10).unwrap();
        assert_eq!(
            expiries,
            vec![ExpiryCandidate {
                partition,
                key: "ttl".into(),
                version: 1,
                deadline: 90
            }]
        );
        drop(target);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn root_mismatch_blocks_install_even_when_chunk_checksum_passes() {
        let journal = ValueJournal::temporary();
        let image = journal.export_image(PartitionId::new(7), None).unwrap();
        let manifest = manifest(None, &image);
        let mut wrong = image;
        wrong[0] ^= 1;
        assert_eq!(
            journal
                .receive_image(request(&manifest, RepairStep::Status), |_| true)
                .unwrap()
                .next,
            0
        );
        journal
            .receive_image(request(&manifest, chunk(0, &wrong)), |_| true)
            .unwrap();
        assert!(journal
            .receive_image(request(&manifest, RepairStep::Install), |_| true)
            .is_err());
        assert!(
            !journal
                .receive_image(request(&manifest, RepairStep::Status), |_| true)
                .unwrap()
                .installed
        );
        assert!(journal
            .0
            .get(meta_key(PartitionId::new(7)))
            .unwrap()
            .is_none());
    }

    #[test]
    fn a_new_epoch_replaces_the_cursor_and_an_oversized_transfer_leaves_it_intact() {
        let journal = ValueJournal::temporary();
        let image = journal.export_image(PartitionId::new(7), None).unwrap();
        let active = manifest(None, &image);
        journal
            .receive_image(request(&active, RepairStep::Status), |_| true)
            .unwrap();
        journal
            .receive_image(request(&active, chunk(0, &image)), |_| true)
            .unwrap();
        let mut incompatible = active.clone();
        incompatible.plan.snapshot_digest = [5; 32];
        assert!(journal
            .receive_image(request(&incompatible, RepairStep::Status), |_| true)
            .is_err());
        let mut oversized = active.clone();
        oversized.image_len = MAX_IMAGE_BYTES + 1;
        assert!(journal
            .receive_image(request(&oversized, RepairStep::Status), |_| true)
            .is_err());
        assert_eq!(
            journal
                .receive_image(request(&active, RepairStep::Status), |_| true)
                .unwrap()
                .next,
            1
        );
        let mut newer = active.clone();
        newer.plan.old_epoch = active.plan.new_epoch;
        newer.plan.new_epoch = ClusterEpoch::new(newer.plan.old_epoch.value() + 1);
        assert_eq!(
            journal
                .receive_image(request(&newer, RepairStep::Status), |_| true)
                .unwrap()
                .next,
            0
        );
        assert!(journal.0.get(chunk_key(0)).unwrap().is_none());
        assert!(journal
            .receive_image(request(&active, RepairStep::Install), |_| true)
            .is_err());
    }

    fn delta(
        manifest: &RepairManifest,
        prior: Option<PartitionValuePrefix>,
        key: &str,
        value: Option<Vec<u8>>,
    ) -> Mutation {
        let plan = &manifest.plan;
        Mutation {
            record: next_record(plan.partition, plan.old_epoch, prior, value).unwrap(),
            owner: plan.owner.clone(),
            backups: vec![plan.replaced.clone()],
            previous: prior.map_or([0; 32], |prefix| prefix.digest),
            key: key.to_owned(),
            identity: None,
            expires_not_before_ms: None,
            decision: None,
            outcome_only: false,
        }
    }

    #[test]
    fn live_delta_receipt_is_ordered_idempotent_and_seals_only_the_caught_up_prefix() {
        let source = ValueJournal::temporary();
        let directory = journal_directory();
        let target = ValueJournal::open(&directory).unwrap();
        let base = write(&source, None, "remove", Some(b"initial".to_vec()), None);
        let image = source.export_image(base.partition, Some(base)).unwrap();
        let manifest = manifest(Some(base), &image);
        source.start_capture(&manifest, &image).unwrap();
        assert_eq!(source.captured_image(&manifest).unwrap(), image);

        let mut ttl = delta(&manifest, Some(base), "ttl", Some(b"timed".to_vec()));
        ttl.expires_not_before_ms = Some(90);
        let first = ttl.certificate();
        let tombstone = delta(&manifest, Some(first), "remove", None);
        let second = tombstone.certificate();
        let mut outcome = delta(&manifest, Some(second), "remove", None);
        outcome.identity = Some("failed-cas".into());
        outcome.decision = Some(CachedDecision {
            request: [7; 32],
            response: ClientResponse::CasMismatch { current: None },
        });
        outcome.outcome_only = true;
        let last = outcome.certificate();
        for (previous, item) in [
            (base, &ttl),
            (base, &ttl),
            (first, &tombstone),
            (second, &outcome),
        ] {
            assert_eq!(
                source.apply(Some(previous), item.clone(), true).unwrap(),
                ApplyResult::Flushed
            );
        }
        let unseen = delta(
            &manifest,
            Some(last),
            "not-certified",
            Some(b"speculative".to_vec()),
        );
        assert_eq!(
            source.apply(Some(last), unseen, true).unwrap(),
            ApplyResult::Flushed
        );
        let (certified, _) = source
            .captured_deltas(&manifest, Some(base), Some(last))
            .unwrap();
        assert_eq!(certified, [ttl, tombstone, outcome]);
        assert!(source
            .captured_deltas(
                &manifest,
                Some(base),
                Some(PartitionValuePrefix {
                    digest: [4; 32],
                    ..last
                })
            )
            .is_err());

        target
            .receive_image(request(&manifest, RepairStep::Status), |_| true)
            .unwrap();
        let transfer = |item: &Mutation| RepairStep::Delta {
            mutation: Box::new(item.clone()),
        };
        assert!(target
            .receive_image(request(&manifest, transfer(&certified[0])), |_| true)
            .is_err());
        for (index, bytes) in image
            .chunks(crate::imap_value_repair::CHUNK_BYTES)
            .enumerate()
        {
            target
                .receive_image(request(&manifest, chunk(index as u32, bytes)), |_| true)
                .unwrap();
        }
        assert_eq!(
            target
                .receive_image(request(&manifest, RepairStep::Install), |_| true)
                .unwrap()
                .applied,
            Some(base)
        );
        assert!(target
            .receive_image(request(&manifest, transfer(&certified[1])), |_| true)
            .is_err());
        for _ in 0..2 {
            assert_eq!(
                target
                    .receive_image(request(&manifest, transfer(&certified[0])), |_| true)
                    .unwrap()
                    .applied,
                Some(first)
            );
        }
        let wrong_first = delta(&manifest, Some(base), "ttl", Some(b"diverged".to_vec()));
        assert!(target
            .receive_image(request(&manifest, transfer(&wrong_first)), |_| true)
            .is_err());
        drop(target);
        let target = ValueJournal::open(&directory).unwrap();
        let progress = target
            .receive_image(request(&manifest, RepairStep::Status), |_| true)
            .unwrap();
        assert_eq!(progress.applied, Some(first));
        assert!(target
            .receive_image(
                request(&manifest, RepairStep::Seal { prefix: Some(last) }),
                |_| true
            )
            .is_err());
        let (rest, _) = source
            .captured_deltas(&manifest, progress.applied, Some(last))
            .unwrap();
        for item in rest {
            target
                .receive_image(request(&manifest, transfer(&item)), |_| true)
                .unwrap();
        }
        assert_eq!(
            target
                .receive_image(
                    request(&manifest, RepairStep::Seal { prefix: Some(last) }),
                    |_| true
                )
                .unwrap()
                .applied,
            Some(last)
        );
        let meta = target.load(base.partition).unwrap();
        assert!(meta.covers(Some(last)));
        assert!(meta.retained(Some(last), "failed-cas").is_some());
        assert!(target
            .read_entry(base.partition, "remove", &meta, Some(last))
            .unwrap()
            .unwrap()
            .record
            .is_tombstone());
        assert!(target
            .read_entry(base.partition, "not-certified", &meta, Some(last))
            .unwrap()
            .is_none());
        assert_eq!(
            target.expiry_batch(&mut None, 90, 10).unwrap()[0].deadline,
            90
        );
        source.finish_capture(&manifest).unwrap();
        assert!(source.captured_transfer().unwrap().is_none());
        assert!(source.captured_image(&manifest).is_err());
        drop(target);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn a_full_delta_budget_backpressures_before_owner_apply() {
        let journal = ValueJournal::temporary();
        let partition = PartitionId::new(7);
        let image = journal.export_image(partition, None).unwrap();
        let manifest = manifest(None, &image);
        journal.start_capture(&manifest, &image).unwrap();
        let mut prior = None;
        for _ in 0..MAX_DELTA_ROWS {
            let item = delta(&manifest, prior, "key", Some(b"bounded".to_vec()));
            let next = item.certificate();
            assert_eq!(
                journal.apply(prior, item, true).unwrap(),
                ApplyResult::Flushed
            );
            prior = Some(next);
        }
        let excess = delta(&manifest, prior, "key", Some(b"too far".to_vec()));
        assert_eq!(
            journal.apply(prior, excess.clone(), true).unwrap(),
            ApplyResult::Busy
        );
        assert!(journal.load(partition).unwrap().covers(prior));
        assert!(!journal
            .load(partition)
            .unwrap()
            .covers(Some(excess.certificate())));
        journal.finish_capture(&manifest).unwrap();
        assert_eq!(
            journal.apply(prior, excess, true).unwrap(),
            ApplyResult::Flushed
        );
    }
}
