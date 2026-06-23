use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::cluster::{ClusterEpoch, ClusterNodeId};
use crate::grid::active_active::HybridLogicalClock;
use crate::grid::hardening::{ReplicatedValueRecord, ValueVersion};

/// Conflict-free value that can converge under active-active replication.
pub trait ConflictFreeValue: Clone + Send + Sync {
    /// Merge `other` into `self`.
    ///
    /// Implementations must be associative, commutative, and idempotent.
    fn merge(&mut self, other: &Self);
}

/// Grow-only counter CRDT.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GCounter {
    per_node: BTreeMap<ClusterNodeId, u64>,
}

impl GCounter {
    /// Create an empty counter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment one node component.
    pub fn increment(&mut self, node: impl Into<ClusterNodeId>, by: u64) {
        let entry = self.per_node.entry(node.into()).or_default();
        *entry = entry.saturating_add(by);
    }

    /// Return the component for one node.
    pub fn component(&self, node: &ClusterNodeId) -> u64 {
        self.per_node.get(node).copied().unwrap_or_default()
    }

    /// Return the counter value.
    pub fn value(&self) -> u64 {
        self.per_node.values().copied().sum()
    }

    /// Return a deterministic metadata-size approximation.
    pub fn metadata_bytes(&self) -> u64 {
        self.per_node
            .keys()
            .map(|node| node.as_str().len() as u64 + std::mem::size_of::<u64>() as u64)
            .sum()
    }
}

impl ConflictFreeValue for GCounter {
    fn merge(&mut self, other: &Self) {
        for (node, value) in &other.per_node {
            let entry = self.per_node.entry(node.clone()).or_default();
            *entry = (*entry).max(*value);
        }
    }
}

/// Positive-negative counter CRDT.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PnCounter {
    inc: GCounter,
    dec: GCounter,
}

impl PnCounter {
    /// Create an empty PN counter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment one node component.
    pub fn increment(&mut self, node: impl Into<ClusterNodeId>, by: u64) {
        self.inc.increment(node, by);
    }

    /// Decrement one node component.
    pub fn decrement(&mut self, node: impl Into<ClusterNodeId>, by: u64) {
        self.dec.increment(node, by);
    }

    /// Return the signed counter value.
    pub fn value(&self) -> i128 {
        i128::from(self.inc.value()) - i128::from(self.dec.value())
    }

    /// Return a deterministic metadata-size approximation.
    pub fn metadata_bytes(&self) -> u64 {
        self.inc
            .metadata_bytes()
            .saturating_add(self.dec.metadata_bytes())
    }
}

impl ConflictFreeValue for PnCounter {
    fn merge(&mut self, other: &Self) {
        self.inc.merge(&other.inc);
        self.dec.merge(&other.dec);
    }
}

/// Unique add/remove tag used by OR-set entries.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OrSetTag {
    /// Node that created the tag.
    pub node: ClusterNodeId,
    /// Node-local monotonically increasing counter.
    pub counter: u64,
}

impl OrSetTag {
    /// Create a tag.
    pub fn new(node: impl Into<ClusterNodeId>, counter: u64) -> Self {
        Self {
            node: node.into(),
            counter,
        }
    }
}

/// Observed-remove set CRDT.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(
    serialize = "T: Serialize + Ord",
    deserialize = "T: Deserialize<'de> + Ord"
))]
pub struct OrSet<T> {
    adds: BTreeMap<T, BTreeSet<OrSetTag>>,
    removes: BTreeSet<OrSetTag>,
}

impl<T> OrSet<T>
where
    T: Clone + Ord,
{
    /// Create an empty OR-set.
    pub fn new() -> Self {
        Self {
            adds: BTreeMap::new(),
            removes: BTreeSet::new(),
        }
    }

    /// Add a value with a unique tag.
    pub fn add(&mut self, value: T, tag: OrSetTag) {
        self.adds.entry(value).or_default().insert(tag);
    }

    /// Remove the value by recording every currently observed add tag.
    pub fn remove(&mut self, value: &T) {
        if let Some(tags) = self.adds.get(value) {
            self.removes.extend(tags.iter().cloned());
        }
    }

    /// Return whether the value is visible.
    pub fn contains(&self, value: &T) -> bool {
        self.adds
            .get(value)
            .map(|tags| tags.iter().any(|tag| !self.removes.contains(tag)))
            .unwrap_or(false)
    }

    /// Return visible values in deterministic order.
    pub fn values(&self) -> Vec<T> {
        self.adds
            .keys()
            .filter(|value| self.contains(value))
            .cloned()
            .collect()
    }

    /// Return a deterministic metadata-size approximation.
    pub fn metadata_bytes(&self) -> u64 {
        let adds = self
            .adds
            .values()
            .map(|tags| tags.len() as u64 * or_set_tag_bytes())
            .sum::<u64>();
        let removes = self.removes.len() as u64 * or_set_tag_bytes();
        adds.saturating_add(removes)
    }
}

impl<T> ConflictFreeValue for OrSet<T>
where
    T: Clone + Ord + Send + Sync,
{
    fn merge(&mut self, other: &Self) {
        for (value, tags) in &other.adds {
            self.adds
                .entry(value.clone())
                .or_default()
                .extend(tags.iter().cloned());
        }
        self.removes.extend(other.removes.iter().cloned());
    }
}

fn or_set_tag_bytes() -> u64 {
    std::mem::size_of::<u64>() as u64 * 2
}

/// Last-writer-wins register CRDT using HLC as the deterministic tie-break.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LwwRegister<T> {
    value: T,
    hlc: HybridLogicalClock,
    writer: ClusterNodeId,
}

impl<T> LwwRegister<T>
where
    T: Clone,
{
    /// Create a register value.
    pub fn new(value: T, hlc: HybridLogicalClock, writer: impl Into<ClusterNodeId>) -> Self {
        Self {
            value,
            hlc,
            writer: writer.into(),
        }
    }

    /// Return the stored value.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Return the HLC timestamp.
    pub fn hlc(&self) -> HybridLogicalClock {
        self.hlc
    }

    /// Return the writer node id.
    pub fn writer(&self) -> &ClusterNodeId {
        &self.writer
    }
}

impl<T> ConflictFreeValue for LwwRegister<T>
where
    T: Clone + Send + Sync,
{
    fn merge(&mut self, other: &Self) {
        if (other.hlc, &other.writer) > (self.hlc, &self.writer) {
            *self = other.clone();
        }
    }
}

/// Bounded-label CRDT merge counters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtMergeStats {
    /// CRDT merges by type name.
    pub merge_total: BTreeMap<String, u64>,
    /// Conflicts resolved by CRDT type name.
    pub conflict_resolved_total: BTreeMap<String, u64>,
    /// Latest observed metadata bytes by CRDT type name.
    pub metadata_bytes: BTreeMap<String, u64>,
}

impl CrdtMergeStats {
    /// Record one CRDT merge.
    pub fn record_merge(&mut self, kind: &'static str, conflict: bool, metadata_bytes: u64) {
        *self.merge_total.entry(kind.to_owned()).or_default() += 1;
        if conflict {
            *self
                .conflict_resolved_total
                .entry(kind.to_owned())
                .or_default() += 1;
        }
        self.metadata_bytes.insert(kind.to_owned(), metadata_bytes);
    }
}

/// Outcome of checking a CRDT update against an existing tombstone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TombstoneCrdtDecision {
    /// The CRDT update is newer than the tombstone and may apply.
    ApplyUpdate,
    /// The tombstone dominates; do not resurrect the key.
    KeepTombstone,
}

/// Enforce the A5 rule: a tombstone at an equal/higher `(version, epoch)` wins.
pub fn tombstone_crdt_decision(
    tombstone: &ReplicatedValueRecord,
    update_version: ValueVersion,
    update_epoch: ClusterEpoch,
) -> TombstoneCrdtDecision {
    if tombstone.is_tombstone()
        && (tombstone.version, tombstone.epoch) >= (update_version, update_epoch)
    {
        TombstoneCrdtDecision::KeepTombstone
    } else {
        TombstoneCrdtDecision::ApplyUpdate
    }
}

impl fmt::Display for TombstoneCrdtDecision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ApplyUpdate => "apply_update",
            Self::KeepTombstone => "keep_tombstone",
        })
    }
}
