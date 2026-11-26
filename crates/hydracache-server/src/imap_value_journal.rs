//! Durable apply and visibility for the W10c partition owner and its replicas.

use std::collections::BTreeMap;
use std::path::Path;

use hydracache::{
    CacheError, CacheResult, ChecksummedReplicatedValueRecord, ClusterEpoch, PartitionId,
    PartitionValuePrefix, ReplicatedSlot, ReplicatedValueRecord,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::imap_topology::PartitionMember;

const RETAINED_IDENTITIES: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct Mutation {
    pub owner: PartitionMember,
    pub backups: Vec<PartitionMember>,
    pub previous: [u8; 32],
    pub key: String,
    pub identity: Option<String>,
    pub record: ReplicatedValueRecord,
}

impl Mutation {
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

fn hash(input: impl Serialize) -> [u8; 32] {
    Sha256::digest(serde_json::to_vec(&input).expect("bounded value mutation serializes")).into()
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct PartitionJournal {
    visible: Option<PartitionValuePrefix>,
    pending: Option<Mutation>,
    outcomes: BTreeMap<String, [u8; 32]>,
}

impl PartitionJournal {
    fn matching_pending(&self, required: Option<PartitionValuePrefix>) -> Option<&Mutation> {
        self.pending
            .as_ref()
            .filter(|pending| required == Some(pending.certificate()))
    }

    pub fn covers(&self, required: Option<PartitionValuePrefix>) -> bool {
        self.visible == required || self.matching_pending(required).is_some()
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
        let partition = mutation.record.partition;
        let mut meta = self.load(partition)?;
        let mut batch = sled::Batch::default();
        if meta.visible != required {
            if meta.matching_pending(required).is_none() {
                return Ok(ApplyResult::Busy);
            }
            let previous = meta.pending.take().expect("certified pending record");
            let key = record_key(partition, &previous.key);
            batch.insert(
                key,
                serde_json::to_vec(&ChecksummedReplicatedValueRecord::seal(
                    previous.record.clone(),
                ))
                .map_err(storage_error)?,
            );
            if let Some(identity) = &previous.identity {
                meta.outcomes
                    .insert(identity.clone(), previous.outcome_digest());
            }
            meta.visible = required;
        }

        if let Some(identity) = &mutation.identity {
            if let Some(stored) = meta.outcomes.get(identity) {
                return Ok(if owner_request && *stored == mutation.outcome_digest() {
                    ApplyResult::Replay
                } else {
                    ApplyResult::Conflict
                });
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
        meta.pending = Some(mutation);
        batch.insert(
            meta_key(partition).as_slice(),
            serde_json::to_vec(&meta).map_err(storage_error)?,
        );
        self.0.apply_batch(batch).map_err(storage_error)?;
        self.0.flush().map_err(storage_error)?;
        Ok(ApplyResult::Flushed)
    }

    pub fn read(
        &self,
        partition: PartitionId,
        key: &str,
        meta: &PartitionJournal,
        required: Option<PartitionValuePrefix>,
    ) -> CacheResult<Option<Vec<u8>>> {
        if let Some(mutation) = meta
            .matching_pending(required)
            .filter(|mutation| mutation.key == key)
        {
            return Ok(mutation.value().map(<[u8]>::to_vec));
        }
        let Some(bytes) = self
            .0
            .get(record_key(partition, key))
            .map_err(storage_error)?
        else {
            return Ok(None);
        };
        let sealed: ChecksummedReplicatedValueRecord =
            serde_json::from_slice(&bytes).map_err(storage_error)?;
        sealed
            .verify()
            .map_err(|_| storage_error("stored record checksum mismatch"))?;
        let record = sealed.record;
        if record.partition != partition
            || required.is_none_or(|prefix| record.version > prefix.version)
        {
            return Err(storage_error("record is outside the certified prefix"));
        }
        Ok(match record.state {
            ReplicatedSlot::Value { value, .. } => Some(value),
            ReplicatedSlot::Tombstone { .. } => None,
        })
    }
}

fn meta_key(partition: PartitionId) -> [u8; 5] {
    let [a, b, c, d] = partition.value().to_be_bytes();
    [b'm', a, b, c, d]
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
                .read(partition, &first.key, &meta, Some(certificate))
                .unwrap(),
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
            .read(partition, &first.key, &meta, Some(certificate))
            .is_err());
    }
}
