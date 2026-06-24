use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::cluster::{ClusterNodeId, PartitionId};
use crate::grid::hardening::ReplicatedValueRecord;
use crate::grid::ReplicatedSlot;

/// Inclusive key range that needs repair.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct KeyRange {
    /// First key in the mismatching range.
    pub start: String,
    /// Last key in the mismatching range.
    pub end: String,
}

impl KeyRange {
    /// Create a single-key repair range.
    pub fn single(key: impl Into<String>) -> Self {
        let key = key.into();
        Self {
            start: key.clone(),
            end: key,
        }
    }
}

/// Incremental repair watermark.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairToken {
    /// Last repaired key for the partition.
    pub last_key: Option<String>,
}

impl RepairToken {
    /// Return whether `key` is already covered by this token.
    pub fn covers(&self, key: &str) -> bool {
        self.last_key
            .as_deref()
            .map(|last| key <= last)
            .unwrap_or(false)
    }

    fn advance_to(&mut self, key: &str) {
        if self
            .last_key
            .as_deref()
            .map(|last| key > last)
            .unwrap_or(true)
        {
            self.last_key = Some(key.to_owned());
        }
    }
}

/// Compact Merkle representation for one partition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleTree {
    /// Partition covered by this tree.
    pub partition: PartitionId,
    leaves: BTreeMap<String, u64>,
}

impl MerkleTree {
    /// Build a tree from replicated records, filtering to `partition`.
    pub fn from_records(
        partition: PartitionId,
        records: &BTreeMap<String, ReplicatedValueRecord>,
    ) -> Self {
        let leaves = records
            .iter()
            .filter(|(_, record)| record.partition == partition)
            .map(|(key, record)| (key.clone(), hash_record(key, record)))
            .collect();
        Self { partition, leaves }
    }

    /// Build an empty tree for a partition.
    pub fn empty(partition: PartitionId) -> Self {
        Self {
            partition,
            leaves: BTreeMap::new(),
        }
    }

    /// Return the number of leaves.
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Return whether the tree has no leaves.
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    /// Return repair ranges for mismatching leaves only.
    pub fn diff(&self, other: &Self) -> Vec<KeyRange> {
        self.diff_after(other, &RepairToken::default())
    }

    /// Return repair ranges after the incremental watermark.
    pub fn diff_after(&self, other: &Self, watermark: &RepairToken) -> Vec<KeyRange> {
        let keys = self
            .leaves
            .keys()
            .chain(other.leaves.keys())
            .filter(|key| !watermark.covers(key))
            .cloned()
            .collect::<BTreeSet<_>>();

        keys.into_iter()
            .filter(|key| self.leaves.get(key) != other.leaves.get(key))
            .map(KeyRange::single)
            .collect()
    }
}

/// Repair execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairKind {
    /// Repair performed inline on the read path.
    ForegroundReadRepair,
    /// Scheduled background incremental repair.
    ScheduledIncremental,
}

/// Repair session state for one partition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairSession {
    /// Partition being repaired.
    pub partition: PartitionId,
    /// Peers participating in the session.
    pub peers: Vec<ClusterNodeId>,
    /// Incremental repaired watermark.
    pub repaired_watermark: RepairToken,
}

impl RepairSession {
    /// Create a repair session.
    pub fn new(partition: PartitionId, peers: Vec<ClusterNodeId>) -> Self {
        Self {
            partition,
            peers,
            repaired_watermark: RepairToken::default(),
        }
    }

    /// Run one deterministic repair diff pass.
    pub fn run(&mut self, left: &MerkleTree, right: &MerkleTree, kind: RepairKind) -> RepairReport {
        let before = self.repaired_watermark.clone();
        let ranges = left.diff_after(right, &self.repaired_watermark);
        for range in &ranges {
            self.repaired_watermark.advance_to(&range.end);
        }
        let skipped_repaired_ranges = left
            .diff(right)
            .into_iter()
            .filter(|range| before.covers(&range.end))
            .count();
        RepairReport {
            kind,
            partition: self.partition,
            ranges,
            repaired_watermark: self.repaired_watermark.clone(),
            skipped_repaired_ranges,
        }
    }
}

/// Report from one repair session run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairReport {
    /// Repair mode.
    pub kind: RepairKind,
    /// Partition repaired.
    pub partition: PartitionId,
    /// Ranges that need exchange.
    pub ranges: Vec<KeyRange>,
    /// Watermark after the run.
    pub repaired_watermark: RepairToken,
    /// Ranges skipped because incremental repair already covered them.
    pub skipped_repaired_ranges: usize,
}

impl RepairReport {
    /// Return the number of ranges exchanged.
    pub fn ranges_exchanged(&self) -> usize {
        self.ranges.len()
    }
}

/// Result of foreground read-repair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForegroundReadRepairOutcome {
    /// Freshest record served to the caller.
    pub served: Option<ReplicatedValueRecord>,
    /// Records that should be written back to divergent replicas.
    pub repairs: Vec<ReplicatedValueRecord>,
}

/// Select the freshest record and repair divergent replicas inline.
pub fn foreground_read_repair<I>(records: I) -> ForegroundReadRepairOutcome
where
    I: IntoIterator<Item = Option<ReplicatedValueRecord>>,
{
    let present = records.into_iter().flatten().collect::<Vec<_>>();
    let served = present
        .iter()
        .cloned()
        .reduce(|left, right| left.merge(right));
    let repairs = served
        .as_ref()
        .map(|fresh| {
            present
                .iter()
                .filter(|record| *record != fresh)
                .map(|_| fresh.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    ForegroundReadRepairOutcome { served, repairs }
}

fn hash_record(key: &str, record: &ReplicatedValueRecord) -> u64 {
    let mut hash = Fnv64::new();
    hash.bytes(key.as_bytes());
    hash.u32(record.partition.value());
    hash.u64(record.version);
    hash.u64(record.epoch.value());
    match &record.state {
        ReplicatedSlot::Value { value, version } => {
            hash.u8(1);
            hash.u64(*version);
            hash.bytes(value);
        }
        ReplicatedSlot::Tombstone {
            version,
            gc_eligible_after,
        } => {
            hash.u8(2);
            hash.u64(*version);
            hash.u64(
                gc_eligible_after
                    .map(|epoch| epoch.value())
                    .unwrap_or(u64::MAX),
            );
        }
    }
    hash.finish()
}

#[derive(Debug, Clone, Copy)]
struct Fnv64(u64);

impl Fnv64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    fn new() -> Self {
        Self(Self::OFFSET)
    }

    fn u8(&mut self, value: u8) {
        self.0 ^= u64::from(value);
        self.0 = self.0.wrapping_mul(Self::PRIME);
    }

    fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }

    fn bytes(&mut self, value: &[u8]) {
        for byte in value {
            self.u8(*byte);
        }
    }

    fn finish(self) -> u64 {
        self.0
    }
}
