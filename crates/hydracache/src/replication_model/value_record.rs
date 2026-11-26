//! Versioned replicated value records (moved from `grid::hardening` so the
//! browser simulator can use the product hint rules; TD-0016).

use serde::{Deserialize, Serialize};

use crate::cluster::ClusterEpoch;
use crate::replication_model::{PartitionId, ReplicatedSlot};

/// Monotonic version used by replicated value records.
pub type ValueVersion = u64;

/// Bytes after the operator-supplied replication boundary has sealed/redacted
/// them. Durable stores persist these bytes, never the original plaintext.
pub type SealedBytes = Vec<u8>;

/// Durable value-store format version registered in `docs/COMPAT.md`.
pub const REPLICATED_VALUE_RECORD_FORMAT_VERSION: u32 = 1;

/// Durable replicated value record keyed externally by cache key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicatedValueRecord {
    /// Partition that owns this value.
    pub partition: PartitionId,
    /// Monotonic value/tombstone version.
    pub version: ValueVersion,
    /// Authority epoch that produced this record.
    pub epoch: ClusterEpoch,
    /// Sealed value bytes or tombstone marker.
    pub state: ReplicatedSlot<SealedBytes>,
}

impl ReplicatedValueRecord {
    /// Create a live value record.
    pub fn value(
        partition: PartitionId,
        version: ValueVersion,
        epoch: ClusterEpoch,
        sealed: impl Into<SealedBytes>,
    ) -> Self {
        Self {
            partition,
            version,
            epoch,
            state: ReplicatedSlot::Value {
                value: sealed.into(),
                version,
            },
        }
    }

    /// Create a tombstone record.
    pub fn tombstone(
        partition: PartitionId,
        version: ValueVersion,
        epoch: ClusterEpoch,
        gc_eligible_after: Option<ClusterEpoch>,
    ) -> Self {
        Self {
            partition,
            version,
            epoch,
            state: ReplicatedSlot::Tombstone {
                version,
                gc_eligible_after,
            },
        }
    }

    /// Return whether this record is a tombstone.
    pub fn is_tombstone(&self) -> bool {
        self.state.is_tombstone()
    }

    /// Approximate bytes charged against the durable value-store budget.
    pub fn approx_bytes(&self) -> u64 {
        match &self.state {
            ReplicatedSlot::Value { value, .. } => value.len().max(1) as u64,
            ReplicatedSlot::Tombstone { .. } => 1,
        }
    }

    /// Return a deterministic checksum over the durable record payload.
    pub fn artifact_checksum(&self) -> u64 {
        let mut checksum = ArtifactChecksum::new();
        checksum.u32(REPLICATED_VALUE_RECORD_FORMAT_VERSION);
        checksum.u32(self.partition.value());
        checksum.u64(self.version);
        checksum.u64(self.epoch.value());
        match &self.state {
            ReplicatedSlot::Value { value, version } => {
                checksum.u8(1);
                checksum.u64(*version);
                checksum.bytes(value);
            }
            ReplicatedSlot::Tombstone {
                version,
                gc_eligible_after,
            } => {
                checksum.u8(2);
                checksum.u64(*version);
                checksum.u64(
                    gc_eligible_after
                        .map(ClusterEpoch::value)
                        .unwrap_or(u64::MAX),
                );
            }
        }
        checksum.finish()
    }

    /// Merge two records: higher `(version, epoch)` wins, and tombstones win
    /// ties so deletes cannot be undone by stale values.
    pub fn merge(self, other: Self) -> Self {
        match (self.version, self.epoch).cmp(&(other.version, other.epoch)) {
            std::cmp::Ordering::Greater => self,
            std::cmp::Ordering::Less => other,
            std::cmp::Ordering::Equal if self.is_tombstone() => self,
            std::cmp::Ordering::Equal => other,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ArtifactChecksum(u64);

impl ArtifactChecksum {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    pub(crate) fn new() -> Self {
        Self(Self::OFFSET)
    }

    pub(crate) fn u8(&mut self, value: u8) {
        self.0 ^= u64::from(value);
        self.0 = self.0.wrapping_mul(Self::PRIME);
    }

    pub(crate) fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }

    pub(crate) fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }

    pub(crate) fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.u8(*byte);
        }
    }

    pub(crate) fn finish(self) -> u64 {
        self.0
    }
}
