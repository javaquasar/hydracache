//! Pure replica-placement and consistency-level types.
//!
//! Built for every target, including `wasm32-unknown-unknown`, so the browser
//! simulator runs the product rules instead of a mirror (TD-0014). Everything
//! here depends only on [`crate::ClusterNodeId`], `serde` and `std`; the
//! networked grid that uses these types stays in `grid`.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::cluster::{ClusterEpoch, ClusterNodeId};

pub(crate) mod consistency_level;
pub(crate) mod hinted_handoff;
pub(crate) mod merkle_repair;
pub(crate) mod value_record;

/// Stable region identifier used by zone-aware placement.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RegionId(String);

impl RegionId {
    /// Create a region id.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the region id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for RegionId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for RegionId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Primary plus backup owners for one replicated key or partition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replicas {
    /// Primary owner selected by rendezvous ranking.
    pub primary: ClusterNodeId,
    /// Backup owners in deterministic rendezvous order.
    pub backups: Vec<ClusterNodeId>,
}

/// Rank `nodes` for `key` by rendezvous hashing and return the primary plus up
/// to `replication_factor - 1` backups, or `None` when there are no nodes.
///
/// This is the placement rule behind `ClusterReplicationStrategy` for
/// `RendezvousClusterOwnership`; it takes node ids instead of members so it
/// builds for every target (the browser simulator uses it). The score is FNV-1a
/// over `key 0xff node_id`, highest first, ties to the greater node id. Owners
/// are a cross-version contract: `tests/placement_golden.rs` pins them.
pub fn rank_replicas<'a, I>(key: &str, nodes: I, replication_factor: usize) -> Option<Replicas>
where
    I: IntoIterator<Item = &'a ClusterNodeId>,
{
    let mut ranked = nodes
        .into_iter()
        .map(|node| (rendezvous_score(key, node), node.clone()))
        .collect::<Vec<_>>();
    ranked.sort_by(|(left_score, left_node), (right_score, right_node)| {
        right_score
            .cmp(left_score)
            .then_with(|| right_node.cmp(left_node))
    });
    ranked.dedup_by(|(_, left_node), (_, right_node)| left_node == right_node);

    let mut nodes = ranked
        .into_iter()
        .map(|(_, node)| node)
        .take(replication_factor.max(1))
        .collect::<Vec<_>>();
    if nodes.is_empty() {
        return None;
    }
    let primary = nodes.remove(0);
    Some(Replicas::new(primary, nodes))
}

fn rendezvous_score(key: &str, node_id: &ClusterNodeId) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for byte in key.bytes().chain([0xff]).chain(node_id.as_str().bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

impl Replicas {
    /// Build a replica set and remove accidental duplicate backups.
    pub fn new(primary: impl Into<ClusterNodeId>, backups: Vec<ClusterNodeId>) -> Self {
        let primary = primary.into();
        let mut seen = BTreeSet::new();
        let backups = backups
            .into_iter()
            .filter(|backup| backup != &primary)
            .filter(|backup| seen.insert(backup.clone()))
            .collect();
        Self { primary, backups }
    }

    /// Return every readable owner, primary first.
    pub fn all_nodes(&self) -> Vec<ClusterNodeId> {
        let mut nodes = Vec::with_capacity(1 + self.backups.len());
        nodes.push(self.primary.clone());
        nodes.extend(self.backups.iter().cloned());
        nodes
    }

    /// Return the number of physical copies represented by this set.
    pub fn copy_count(&self) -> usize {
        1 + self.backups.len()
    }
}

/// Effective placement used while rebalance is in flight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveReplicationMap {
    /// Current committed natural placement.
    pub natural: Replicas,
    /// Readable owners during a move window.
    pub reading: Vec<ClusterNodeId>,
    /// In-flight placement, if a rebalance is moving ownership.
    pub pending: Option<Replicas>,
}

impl EffectiveReplicationMap {
    /// Create a map without pending movement.
    pub fn new(natural: Replicas) -> Self {
        let reading = natural.all_nodes();
        Self {
            natural,
            reading,
            pending: None,
        }
    }

    /// Create a map that reads from both committed and pending owners.
    pub fn with_pending(natural: Replicas, pending: Replicas) -> Self {
        let mut reading = natural.all_nodes();
        for node in pending.all_nodes() {
            if !reading.contains(&node) {
                reading.push(node);
            }
        }
        Self {
            natural,
            reading,
            pending: Some(pending),
        }
    }

    /// Return whether `node_id` belongs to the read set.
    pub fn is_readable_from(&self, node_id: &ClusterNodeId) -> bool {
        self.reading.iter().any(|node| node == node_id)
    }
}

/// Replication/quorum configuration for the 0.41 grid slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicationConfig {
    /// Total number of desired copies, including the primary.
    pub replication_factor: usize,
    /// Number of replicas that must answer a read.
    pub read_quorum: usize,
    /// Number of replicas that must acknowledge a write.
    pub write_quorum: usize,
    /// Backup acknowledgements required before returning to the caller.
    pub sync_backups: usize,
    /// Best-effort backups that do not block the caller.
    pub async_backups: usize,
    /// Maximum encoded value size accepted by value replication.
    pub max_replicated_entry_bytes: usize,
    /// Whether value replication is enabled.
    pub replicate_values: bool,
}

impl ReplicationConfig {
    /// Create a local-first config with value replication disabled.
    pub const fn local_first() -> Self {
        Self {
            replication_factor: 1,
            read_quorum: 1,
            write_quorum: 1,
            sync_backups: 0,
            async_backups: 0,
            max_replicated_entry_bytes: 0,
            replicate_values: false,
        }
    }

    /// Validate the olric-style replica/quorum invariants.
    pub fn validate(self) -> Result<(), ReplicationConfigError> {
        if self.replication_factor == 0 {
            return Err(ReplicationConfigError::ReplicationFactorZero);
        }
        if self.read_quorum == 0 || self.write_quorum == 0 {
            return Err(ReplicationConfigError::QuorumZero);
        }
        if self.read_quorum > self.replication_factor || self.write_quorum > self.replication_factor
        {
            return Err(ReplicationConfigError::QuorumExceedsReplicationFactor);
        }
        let requested_backups = self.sync_backups.saturating_add(self.async_backups);
        if requested_backups > self.replication_factor.saturating_sub(1) {
            return Err(ReplicationConfigError::BackupCountExceedsReplicationFactor);
        }
        if self.replicate_values && self.max_replicated_entry_bytes == 0 {
            return Err(ReplicationConfigError::MissingReplicatedEntryByteCap);
        }
        Ok(())
    }
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self::local_first()
    }
}

/// Errors returned by [`ReplicationConfig::validate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplicationConfigError {
    /// `replication_factor` must be at least one.
    ReplicationFactorZero,
    /// Read and write quorums must be at least one.
    QuorumZero,
    /// A quorum cannot exceed `replication_factor`.
    QuorumExceedsReplicationFactor,
    /// `sync_backups + async_backups` cannot exceed RF-1.
    BackupCountExceedsReplicationFactor,
    /// Value replication requires a byte cap before startup.
    MissingReplicatedEntryByteCap,
}

impl fmt::Display for ReplicationConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ReplicationFactorZero => "replication_factor must be at least 1",
            Self::QuorumZero => "read_quorum and write_quorum must be at least 1",
            Self::QuorumExceedsReplicationFactor => {
                "read_quorum/write_quorum cannot exceed replication_factor"
            }
            Self::BackupCountExceedsReplicationFactor => {
                "sync_backups + async_backups cannot exceed replication_factor - 1"
            }
            Self::MissingReplicatedEntryByteCap => {
                "replicate_values(true) requires max_replicated_entry_bytes"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ReplicationConfigError {}

/// Stable partition id used as a thin indirection over rendezvous ownership.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct PartitionId(u32);

impl PartitionId {
    /// Create a partition id from its numeric value.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Return the numeric partition id.
    pub const fn value(self) -> u32 {
        self.0
    }
}

/// Return the deterministic partition for a key.
///
/// A zero `partition_count` is normalized to one partition so callers cannot
/// accidentally divide by zero while validating a partially built config.
pub fn partition_for_key(key: impl AsRef<str>, partition_count: u32) -> PartitionId {
    let partition_count = partition_count.max(1);
    PartitionId((stable_key_hash(key.as_ref()) % u64::from(partition_count)) as u32)
}

fn stable_key_hash(key: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for byte in key.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Replicated value slot with tombstones participating in version ordering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicatedSlot<V> {
    /// Live value bytes or decoded value.
    Value {
        /// Stored value.
        value: V,
        /// Monotonic version.
        version: u64,
    },
    /// Deletion marker that beats values at the same version.
    Tombstone {
        /// Monotonic tombstone version.
        version: u64,
        /// Epoch after which GC is allowed, once repair confirmed every backup.
        gc_eligible_after: Option<ClusterEpoch>,
    },
}

impl<V> ReplicatedSlot<V> {
    /// Return the slot version.
    pub fn version(&self) -> u64 {
        match self {
            Self::Value { version, .. } | Self::Tombstone { version, .. } => *version,
        }
    }

    /// Return whether this slot is a tombstone.
    pub fn is_tombstone(&self) -> bool {
        matches!(self, Self::Tombstone { .. })
    }

    /// Merge two slots: higher version wins; on ties tombstone wins.
    pub fn merge(self, other: Self) -> Self {
        match self.version().cmp(&other.version()) {
            std::cmp::Ordering::Greater => self,
            std::cmp::Ordering::Less => other,
            std::cmp::Ordering::Equal if self.is_tombstone() => self,
            std::cmp::Ordering::Equal => other,
        }
    }
}
