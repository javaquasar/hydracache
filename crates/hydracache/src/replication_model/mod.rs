//! Pure replica-placement and consistency-level types.
//!
//! Built for every target, including `wasm32-unknown-unknown`, so the browser
//! simulator runs the product rules instead of a mirror (TD-0014). Everything
//! here depends only on [`crate::ClusterNodeId`], `serde` and `std`; the
//! networked grid that uses these types stays in `grid`.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::cluster::ClusterNodeId;

pub(crate) mod consistency_level;

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
