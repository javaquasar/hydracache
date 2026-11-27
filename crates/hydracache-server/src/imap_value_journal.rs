//! Durable apply and visibility for the W10c partition owner and its replicas.

use std::collections::BTreeMap;
use std::ops::Bound;
use std::path::Path;

use hydracache::{
    CacheError, CacheResult, ChecksummedReplicatedValueRecord, ClusterEpoch, PartitionId,
    PartitionValuePrefix, ReplicatedSlot, ReplicatedValueRecord,
};
use hydracache_client_protocol::ClientResponse;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::imap_topology::PartitionMember;

#[path = "imap_value_delta.rs"]
mod delta;
#[path = "imap_value_snapshot.rs"]
mod snapshot;
pub(crate) use delta::CapturedTransfer;

const RETAINED_IDENTITIES: usize = 128;
pub(crate) const RETAINED_EVENT_VERSIONS: u64 = 16;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct JournalEvent {
    pub version: u64,
    pub key: String,
    pub removed: bool,
    pub value: Option<Vec<u8>>,
}

#[derive(Serialize, Deserialize)]
struct ArchivedEvent {
    event: Option<ArchivedMutation>,
    digest: [u8; 32],
}

#[derive(Serialize, Deserialize)]
struct ArchivedMutation {
    key: String,
    value_digest: Option<[u8; 32]>,
}

impl JournalEvent {
    fn from_mutation(mutation: &Mutation, include_value: bool) -> Option<Self> {
        if mutation.outcome_only {
            return None;
        }
        let value = mutation.value();
        Some(Self {
            version: mutation.record.version,
            key: mutation.key.clone(),
            removed: value.is_none(),
            value: if include_value {
                value.map(<[u8]>::to_vec)
            } else {
                None
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct Mutation {
    pub owner: PartitionMember,
    pub backups: Vec<PartitionMember>,
    pub previous: [u8; 32],
    pub key: String,
    pub identity: Option<String>,
    pub record: ReplicatedValueRecord,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_not_before_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<CachedDecision>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub outcome_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CachedDecision {
    pub request: [u8; 32],
    pub response: ClientResponse,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum StoredDecision {
    Certified {
        #[serde(flatten)]
        decision: CachedDecision,
        prefix: PartitionValuePrefix,
    },
    Current(CachedDecision),
    Legacy([u8; 32]),
}

#[derive(Debug)]
pub(crate) struct StoredEntry {
    pub record: ReplicatedValueRecord,
    pub expires_not_before_ms: Option<u64>,
}

impl StoredEntry {
    pub fn value(&self) -> Option<&[u8]> {
        match &self.record.state {
            ReplicatedSlot::Value { value, .. } => Some(value),
            ReplicatedSlot::Tombstone { .. } => None,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct JournalRecordV2 {
    format: u16,
    sealed: ChecksummedReplicatedValueRecord,
    expires_not_before_ms: Option<u64>,
    digest: [u8; 32],
}

impl JournalRecordV2 {
    fn from_mutation(mutation: &Mutation) -> Self {
        let sealed = ChecksummedReplicatedValueRecord::seal(mutation.record.clone());
        let expires_not_before_ms = mutation.expires_not_before_ms;
        Self {
            format: 2,
            digest: hash((2u16, &sealed, expires_not_before_ms)),
            sealed,
            expires_not_before_ms,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum JournalRecord {
    V2(JournalRecordV2),
    Legacy(ChecksummedReplicatedValueRecord),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ExpiryCandidate {
    pub partition: PartitionId,
    pub key: String,
    pub version: u64,
    pub deadline: u64,
}

impl Mutation {
    pub fn wire_version(&self) -> u16 {
        if self.expires_not_before_ms.is_some() || self.decision.is_some() || self.outcome_only {
            2
        } else {
            1
        }
    }

    fn expiry(&self) -> Option<ExpiryCandidate> {
        Some(ExpiryCandidate {
            partition: self.record.partition,
            key: self.key.clone(),
            version: self.record.version,
            deadline: self.expires_not_before_ms?,
        })
    }

    pub fn certificate(&self) -> PartitionValuePrefix {
        PartitionValuePrefix {
            partition: self.record.partition,
            version: self.record.version,
            epoch: self.record.epoch,
            previous: self.previous,
            digest: hash(self),
        }
    }

    fn outcome_digest(&self) -> [u8; 32] {
        hash((&self.key, &self.identity, self.value()))
    }

    fn value(&self) -> Option<&[u8]> {
        match &self.record.state {
            ReplicatedSlot::Value { value, .. } => Some(value),
            ReplicatedSlot::Tombstone { .. } => None,
        }
    }
}

pub(crate) fn hash(input: impl Serialize) -> [u8; 32] {
    Sha256::digest(serde_json::to_vec(&input).expect("bounded value mutation serializes")).into()
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct PartitionJournal {
    visible: Option<PartitionValuePrefix>,
    pending: Option<Mutation>,
    outcomes: BTreeMap<String, StoredDecision>,
}

impl PartitionJournal {
    pub fn retained_count(&self, prefix: Option<PartitionValuePrefix>) -> usize {
        self.outcomes.len()
            + usize::from(
                self.matching_pending(prefix)
                    .is_some_and(|record| record.identity.is_some()),
            )
    }
    fn matching_pending(&self, required: Option<PartitionValuePrefix>) -> Option<&Mutation> {
        self.pending
            .as_ref()
            .filter(|pending| required == Some(pending.certificate()))
    }

    fn materialize_pending(&mut self, required: Option<PartitionValuePrefix>) -> Option<Mutation> {
        self.matching_pending(required)?;
        let previous = self.pending.take().expect("certified pending record");
        if let Some(identity) = &previous.identity {
            let decision = previous
                .decision
                .clone()
                .map(|decision| StoredDecision::Certified {
                    decision,
                    prefix: previous.certificate(),
                })
                .unwrap_or_else(|| StoredDecision::Legacy(previous.outcome_digest()));
            self.outcomes.insert(identity.clone(), decision);
        }
        self.visible = required;
        Some(previous)
    }

    pub fn covers(&self, required: Option<PartitionValuePrefix>) -> bool {
        self.visible == required || self.matching_pending(required).is_some()
    }

    pub fn retained(
        &self,
        required: Option<PartitionValuePrefix>,
        id: &str,
    ) -> Option<&CachedDecision> {
        self.retained_proven(required, id)
            .map(|(decision, _)| decision)
    }

    pub fn retained_proven(
        &self,
        required: Option<PartitionValuePrefix>,
        id: &str,
    ) -> Option<(&CachedDecision, Option<PartitionValuePrefix>)> {
        match self.outcomes.get(id) {
            Some(StoredDecision::Certified { decision, prefix }) => Some((decision, Some(*prefix))),
            Some(StoredDecision::Current(decision)) => Some((decision, None)),
            Some(StoredDecision::Legacy(_)) => None,
            None => {
                let pending = self.matching_pending(required)?;
                if pending.identity.as_deref() != Some(id) {
                    return None;
                }
                Some((pending.decision.as_ref()?, required))
            }
        }
    }

    pub fn pending_retry(
        &self,
        required: Option<PartitionValuePrefix>,
        epoch: ClusterEpoch,
    ) -> Option<&Mutation> {
        self.pending.as_ref().filter(|pending| {
            pending.record.epoch == epoch && required != Some(pending.certificate())
        })
    }
}

pub(crate) struct ValueJournal(sled::Db);

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ApplyResult {
    Flushed,
    Replay,
    Conflict,
    Busy,
}

impl ValueJournal {
    #[cfg(test)]
    pub fn temporary() -> Self {
        Self(sled::Config::new().temporary(true).open().unwrap())
    }
    pub fn open(path: &Path) -> CacheResult<Self> {
        sled::open(path).map(Self).map_err(storage_error)
    }

    pub(crate) fn has_value_state(&self) -> CacheResult<bool> {
        for key in self.0.iter().keys() {
            if key.map_err(storage_error)?.as_ref() != b"scope" {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn bind_scope(&self, tenant: &str, namespace: &str, generation: u64) -> CacheResult<()> {
        let expected =
            serde_json::to_vec(&(1u16, tenant, namespace, generation)).map_err(storage_error)?;
        if !self.0.is_empty() && !self.0.contains_key(b"scope").map_err(storage_error)? {
            return Err(storage_error("nonempty journal has no scope manifest"));
        }
        match self
            .0
            .compare_and_swap(b"scope", None::<&[u8]>, Some(expected.as_slice()))
            .map_err(storage_error)?
        {
            Ok(()) => self.0.flush().map(|_| ()).map_err(storage_error),
            Err(conflict) if conflict.current.as_deref() == Some(expected.as_slice()) => Ok(()),
            Err(_) => Err(storage_error("journal belongs to another format or scope")),
        }
    }

    pub fn load(&self, partition: PartitionId) -> CacheResult<PartitionJournal> {
        self.0
            .get(meta_key(partition))
            .map_err(storage_error)?
            .map(|bytes| serde_json::from_slice(&bytes).map_err(storage_error))
            .transpose()
            .map(Option::unwrap_or_default)
    }

    /// Called under the local partition lane. Persist the previous certified
    /// record and replacement pending record in the same batch.
    pub fn apply(
        &self,
        required: Option<PartitionValuePrefix>,
        mutation: Mutation,
        owner_request: bool,
    ) -> CacheResult<ApplyResult> {
        self.apply_with_batch(required, mutation, owner_request, sled::Batch::default())
    }

    fn apply_with_batch(
        &self,
        required: Option<PartitionValuePrefix>,
        mutation: Mutation,
        owner_request: bool,
        mut batch: sled::Batch,
    ) -> CacheResult<ApplyResult> {
        let partition = mutation.record.partition;
        let mut meta = self.load(partition)?;
        if meta.visible != required {
            let Some(previous) = meta.materialize_pending(required) else {
                return Ok(ApplyResult::Busy);
            };
            let version = previous.record.version;
            // Only a pending record matching the required Raft certificate can
            // become history. Keep decision-only versions to prove continuity.
            let event = (!previous.outcome_only).then(|| {
                let value_digest = previous.value().map(|value| {
                    batch.insert(event_value_key(partition, version), value);
                    Sha256::digest(value).into()
                });
                ArchivedMutation {
                    key: previous.key.clone(),
                    value_digest,
                }
            });
            let stored = ArchivedEvent {
                digest: hash((partition, version, &event)),
                event,
            };
            batch.insert(
                event_key(partition, version),
                serde_json::to_vec(&stored).map_err(storage_error)?,
            );
            if let Some(expired) = version.checked_sub(RETAINED_EVENT_VERSIONS) {
                batch.remove(event_key(partition, expired));
                batch.remove(event_value_key(partition, expired));
            }
            if !previous.outcome_only {
                let stored = JournalRecordV2::from_mutation(&previous);
                batch.insert(
                    record_key(partition, &previous.key),
                    serde_json::to_vec(&stored).map_err(storage_error)?,
                );
                let timer = expiry_key(partition, &previous.key);
                if let Some(expiry) = previous.expiry() {
                    batch.insert(timer, serde_json::to_vec(&expiry).map_err(storage_error)?);
                } else {
                    batch.remove(timer);
                }
            }
        }

        if let Some(identity) = &mutation.identity {
            if let Some(stored) = meta.outcomes.get(identity) {
                return Ok(
                    if owner_request
                        && matches!(stored, StoredDecision::Legacy(digest) if *digest == mutation.outcome_digest())
                    {
                        ApplyResult::Replay
                    } else {
                        ApplyResult::Conflict
                    },
                );
            }
            if meta.outcomes.len() >= RETAINED_IDENTITIES {
                return Ok(ApplyResult::Busy);
            }
        }
        if let Some(pending) = &meta.pending {
            if pending.record.epoch == mutation.record.epoch && pending != &mutation {
                return Ok(
                    if mutation.identity.is_some() && pending.identity == mutation.identity {
                        ApplyResult::Conflict
                    } else {
                        ApplyResult::Busy
                    },
                );
            }
            if pending.record.epoch > mutation.record.epoch {
                return Ok(ApplyResult::Busy);
            }
        }
        if owner_request && !self.capture_delta(&mut batch, required, &mutation)? {
            return Ok(ApplyResult::Busy);
        }
        meta.pending = Some(mutation);
        batch.insert(
            meta_key(partition).as_slice(),
            serde_json::to_vec(&meta).map_err(storage_error)?,
        );
        self.0.apply_batch(batch).map_err(storage_error)?;
        self.0.flush().map_err(storage_error)?;
        Ok(ApplyResult::Flushed)
    }

    /// A missing event slot cannot prove continuity. An existing empty slot is
    /// a certified decision that changed no value.
    pub fn certified_event(
        &self,
        partition: PartitionId,
        version: u64,
        required: PartitionValuePrefix,
        include_value: bool,
    ) -> CacheResult<Option<Option<JournalEvent>>> {
        if version > required.version {
            return Ok(None);
        }
        let meta = self.load(partition)?;
        if !meta.covers(Some(required)) {
            return Ok(None);
        }
        if version == required.version {
            if let Some(pending) = meta.matching_pending(Some(required)) {
                return Ok(Some(JournalEvent::from_mutation(pending, include_value)));
            }
        }
        let Some(bytes) = self
            .0
            .get(event_key(partition, version))
            .map_err(storage_error)?
        else {
            return Ok(None);
        };
        let ArchivedEvent { event, digest } =
            serde_json::from_slice(&bytes).map_err(storage_error)?;
        if digest != hash((partition, version, &event)) {
            return Err(storage_error("archived event digest mismatch"));
        }
        let Some(event) = event else {
            return Ok(Some(None));
        };
        let value = if let Some(expected) = event.value_digest.filter(|_| include_value) {
            let value = self
                .0
                .get(event_value_key(partition, version))
                .map_err(storage_error)?
                .ok_or_else(|| storage_error("archived event value is missing"))?;
            let digest: [u8; 32] = Sha256::digest(&value).into();
            if digest != expected {
                return Err(storage_error("archived event value digest mismatch"));
            }
            Some(value.to_vec())
        } else {
            None
        };
        Ok(Some(Some(JournalEvent {
            version,
            key: event.key,
            removed: event.value_digest.is_none(),
            value,
        })))
    }

    pub fn read_entry(
        &self,
        partition: PartitionId,
        key: &str,
        meta: &PartitionJournal,
        required: Option<PartitionValuePrefix>,
    ) -> CacheResult<Option<StoredEntry>> {
        if let Some(mutation) = meta
            .matching_pending(required)
            .filter(|mutation| !mutation.outcome_only && mutation.key == key)
        {
            return Ok(Some(StoredEntry {
                record: mutation.record.clone(),
                expires_not_before_ms: mutation.expires_not_before_ms,
            }));
        }
        let Some(bytes) = self
            .0
            .get(record_key(partition, key))
            .map_err(storage_error)?
        else {
            return Ok(None);
        };
        let stored: JournalRecord = serde_json::from_slice(&bytes).map_err(storage_error)?;
        stored.into_entry(partition, required).map(Some)
    }
}

impl JournalRecord {
    fn into_entry(
        self,
        partition: PartitionId,
        required: Option<PartitionValuePrefix>,
    ) -> CacheResult<StoredEntry> {
        let (sealed, expires_not_before_ms) = match self {
            JournalRecord::Legacy(sealed) => (sealed, None),
            JournalRecord::V2(stored) => {
                if stored.format != 2
                    || stored.digest
                        != hash((stored.format, &stored.sealed, stored.expires_not_before_ms))
                {
                    return Err(storage_error(
                        "invalid W10d journal record format or digest",
                    ));
                }
                (stored.sealed, stored.expires_not_before_ms)
            }
        };
        sealed
            .verify()
            .map_err(|_| storage_error("stored record checksum mismatch"))?;
        let record = sealed.record;
        if record.partition != partition
            || required.is_none_or(|prefix| record.version > prefix.version)
        {
            return Err(storage_error("record is outside the certified prefix"));
        }
        if record.is_tombstone() && expires_not_before_ms.is_some() {
            return Err(storage_error("tombstone has a deadline"));
        }
        Ok(StoredEntry {
            record,
            expires_not_before_ms,
        })
    }
}

impl ValueJournal {
    /// Bounded rotating scan of persisted timers and still-pending entries.
    pub fn expiry_batch(
        &self,
        cursor: &mut Option<Vec<u8>>,
        lower: u64,
        limit: usize,
    ) -> CacheResult<Vec<ExpiryCandidate>> {
        let start = cursor
            .take()
            .map_or(Bound::Included(vec![b'e']), Bound::Excluded);
        let mut expiries = Vec::new();
        let mut scan = self.0.range((start, Bound::Excluded(vec![b'n'])));
        for _ in 0..limit {
            let Some((key, data)) = scan.next().transpose().map_err(storage_error)? else {
                *cursor = None;
                break;
            };
            let expiry = match key.first() {
                Some(b'e') => Some(serde_json::from_slice(&data).map_err(storage_error)?),
                Some(b'm') => serde_json::from_slice::<PartitionJournal>(&data)
                    .map_err(storage_error)?
                    .pending
                    .and_then(|mutation| mutation.expiry()),
                _ => None,
            };
            if let Some(expiry) = expiry.filter(|expiry| expiry.deadline <= lower) {
                expiries.push(expiry);
            }
            *cursor = Some(key.to_vec());
        }
        Ok(expiries)
    }
}

fn expiry_key(partition: PartitionId, storage_key: &str) -> Vec<u8> {
    [
        b"e".as_slice(),
        &partition.value().to_be_bytes(),
        storage_key.as_bytes(),
    ]
    .concat()
}

fn meta_key(partition: PartitionId) -> [u8; 5] {
    let [a, b, c, d] = partition.value().to_be_bytes();
    [b'm', a, b, c, d]
}

fn event_prefix(partition: PartitionId) -> [u8; 5] {
    let [a, b, c, d] = partition.value().to_be_bytes();
    [b'h', a, b, c, d]
}

fn event_key(partition: PartitionId, version: u64) -> Vec<u8> {
    [event_prefix(partition).as_slice(), &version.to_be_bytes()].concat()
}

fn event_value_key(partition: PartitionId, version: u64) -> Vec<u8> {
    let mut key = event_key(partition, version);
    key.push(b'v');
    key
}

fn record_key(partition: PartitionId, storage_key: &str) -> Vec<u8> {
    [
        b"v".as_slice(),
        &partition.value().to_be_bytes(),
        storage_key.as_bytes(),
    ]
    .concat()
}

fn storage_error(error: impl std::fmt::Display) -> CacheError {
    CacheError::Backend(format!("W10c durable journal: {error}"))
}

pub(crate) fn next_record(
    partition: PartitionId,
    epoch: ClusterEpoch,
    previous: Option<PartitionValuePrefix>,
    bytes: Option<Vec<u8>>,
) -> Option<ReplicatedValueRecord> {
    let version = previous.map_or(0, |prefix| prefix.version).checked_add(1)?;
    Some(match bytes {
        Some(bytes) => ReplicatedValueRecord::value(partition, version, epoch, bytes),
        None => ReplicatedValueRecord::tombstone(partition, version, epoch, None),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_history_requires_certification_and_keeps_bounded_decision_slots() {
        let journal = ValueJournal::temporary();
        let partition = PartitionId::new(8);
        let epoch = ClusterEpoch::new(4);
        let mut required = None;
        for version in 1..=20 {
            let mutation = Mutation {
                owner: PartitionMember {
                    node: hydracache::ClusterNodeId::new("a"),
                    generation: hydracache::ClusterGeneration::new(1),
                },
                backups: vec![],
                previous: required.map_or([0; 32], |prefix: PartitionValuePrefix| prefix.digest),
                key: "event-key".to_owned(),
                identity: None,
                record: next_record(partition, epoch, required, Some(vec![version as u8])).unwrap(),
                expires_not_before_ms: None,
                decision: None,
                outcome_only: version == 19,
            };
            let certificate = mutation.certificate();
            assert_eq!(
                journal.apply(required, mutation, true).unwrap(),
                ApplyResult::Flushed
            );
            if let Some(prior) = required {
                assert_eq!(
                    journal
                        .certified_event(partition, version, prior, true)
                        .unwrap(),
                    None
                );
            }
            required = Some(certificate);
        }
        let required = required.unwrap();
        assert!(journal
            .certified_event(partition, 1, required, true)
            .unwrap()
            .is_none());
        assert_eq!(
            journal
                .certified_event(partition, 19, required, true)
                .unwrap(),
            Some(None)
        );
        let event = journal
            .certified_event(partition, 20, required, true)
            .unwrap()
            .flatten()
            .unwrap();
        assert_eq!(event.value, Some(vec![20]));
        assert_eq!(
            journal
                .certified_event(partition, 18, required, false)
                .unwrap()
                .flatten()
                .unwrap()
                .value,
            None
        );
        assert_eq!(
            journal.0.scan_prefix(event_prefix(partition)).count(),
            2 * RETAINED_EVENT_VERSIONS as usize - 1
        );
        journal
            .0
            .insert(event_value_key(partition, 18), b"tampered-value".as_slice())
            .unwrap();
        assert!(journal
            .certified_event(partition, 18, required, true)
            .is_err());
        assert!(journal
            .certified_event(partition, 18, required, false)
            .unwrap()
            .is_some());
        let key = event_key(partition, 18);
        let mut stored: ArchivedEvent =
            serde_json::from_slice(&journal.0.get(&key).unwrap().unwrap()).unwrap();
        stored.event.as_mut().unwrap().key = "tampered".into();
        journal
            .0
            .insert(key, serde_json::to_vec(&stored).unwrap())
            .unwrap();
        assert!(journal
            .certified_event(partition, 18, required, true)
            .is_err());
    }

    #[test]
    fn historical_record_is_checked_after_next_apply() {
        let journal = ValueJournal::temporary();
        journal.bind_scope("tenant", "orders", 1).unwrap();
        let partition = PartitionId::new(4);
        let epoch = ClusterEpoch::new(3);
        let owner = PartitionMember {
            node: "owner".into(),
            generation: hydracache::ClusterGeneration::new(1),
        };
        let first = Mutation {
            owner,
            backups: Vec::new(),
            previous: [0; 32],
            key: "key-a".into(),
            identity: None,
            record: next_record(partition, epoch, None, Some(b"one".to_vec())).unwrap(),
            expires_not_before_ms: None,
            decision: None,
            outcome_only: false,
        };
        let certificate = first.certificate();
        assert_eq!(
            journal.apply(None, first.clone(), true).unwrap(),
            ApplyResult::Flushed
        );
        let next = Mutation {
            previous: certificate.digest,
            key: "key-b".into(),
            record: next_record(partition, epoch, Some(certificate), Some(b"two".to_vec()))
                .unwrap(),
            ..first.clone()
        };
        assert_eq!(
            journal.apply(Some(certificate), next, true).unwrap(),
            ApplyResult::Flushed
        );
        let meta = journal.load(partition).unwrap();
        assert_eq!(
            journal
                .read_entry(partition, &first.key, &meta, Some(certificate))
                .unwrap()
                .as_ref()
                .and_then(StoredEntry::value)
                .map(<[u8]>::to_vec),
            Some(b"one".to_vec())
        );
        let mut damaged = ChecksummedReplicatedValueRecord::seal(first.record);
        damaged.checksum ^= 1;
        journal
            .0
            .insert(
                record_key(partition, &first.key),
                serde_json::to_vec(&damaged).unwrap(),
            )
            .unwrap();
        assert!(journal
            .read_entry(partition, &first.key, &meta, Some(certificate))
            .is_err());
    }

    #[test]
    fn legacy_rows_are_eternal_and_future_or_modified_deadlines_fail_closed() {
        let journal = ValueJournal::temporary();
        let partition = PartitionId::new(1);
        let epoch = ClusterEpoch::new(1);
        let sealed = ChecksummedReplicatedValueRecord::seal(ReplicatedValueRecord::value(
            partition,
            1,
            epoch,
            b"v".to_vec(),
        ));
        let required = Some(PartitionValuePrefix {
            partition,
            epoch,
            version: 1,
            previous: [0; 32],
            digest: [1; 32],
        });
        let key = record_key(partition, "fixture");
        journal
            .0
            .insert(&key, serde_json::to_vec(&sealed).unwrap())
            .unwrap();
        let meta = PartitionJournal::default();
        assert_eq!(
            journal
                .read_entry(partition, "fixture", &meta, required)
                .unwrap()
                .unwrap()
                .expires_not_before_ms,
            None
        );
        let mut record = JournalRecordV2 {
            format: 2,
            expires_not_before_ms: Some(7),
            digest: hash((2u16, &sealed, Some(7u64))),
            sealed,
        };
        journal
            .0
            .insert(&key, serde_json::to_vec(&record).unwrap())
            .unwrap();
        assert_eq!(
            journal
                .read_entry(partition, "fixture", &meta, required)
                .unwrap()
                .unwrap()
                .expires_not_before_ms,
            Some(7)
        );
        record.expires_not_before_ms = Some(8);
        journal
            .0
            .insert(&key, serde_json::to_vec(&record).unwrap())
            .unwrap();
        assert!(journal
            .read_entry(partition, "fixture", &meta, required)
            .is_err());
        record.format = 3;
        record.digest = hash((3u16, &record.sealed, record.expires_not_before_ms));
        journal
            .0
            .insert(&key, serde_json::to_vec(&record).unwrap())
            .unwrap();
        assert!(journal
            .read_entry(partition, "fixture", &meta, required)
            .is_err());
    }
}
