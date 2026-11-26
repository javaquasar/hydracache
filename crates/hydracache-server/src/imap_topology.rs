//! Partition placement from one fully applied Raft view.

use std::collections::BTreeSet;
use std::num::NonZeroU32;

use hydracache::{
    ClusterEpoch, ClusterGeneration, ClusterNodeId, ClusterReplicationStrategy, PartitionId,
    RendezvousClusterOwnership, ReplicationConfig, ReplicationConfigError,
};
use hydracache_cluster_raft::RaftPartitionSnapshot;
use thiserror::Error;

use crate::GridControlPlaneHandle;

/// An immutable view for calculating placements of explicit partition IDs.
#[derive(Debug)]
pub struct PartitionTopology {
    snapshot: RaftPartitionSnapshot,
    partition_count: NonZeroU32,
    replication_factor: usize,
}

/// A logical member identity bound to a specific process incarnation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionMember {
    pub node: ClusterNodeId,
    pub generation: ClusterGeneration,
}

/// Placement projected from one applied topology epoch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionPlacement {
    pub partition: PartitionId,
    pub epoch: ClusterEpoch,
    pub primary: PartitionMember,
    pub backups: Vec<PartitionMember>,
}

/// Reasons a partition placement cannot be built from the local view.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PartitionTopologyError {
    #[error("the control plane does not provide an applied Raft partition view")]
    MissingSnapshot,
    #[error("Raft applied index {applied} differs from commit index {committed}")]
    Unapplied { applied: u64, committed: u64 },
    #[error(transparent)]
    InvalidReplication(#[from] ReplicationConfigError),
    #[error("the Raft member roster conflicts with its applied epoch")]
    InvalidRoster,
    #[error("the roster has {available} members, but placement requires {required}")]
    InsufficientMembers { available: usize, required: usize },
    #[error("partition {0} is outside the configured partition count")]
    UnknownPartition(u32),
}

impl PartitionTopology {
    pub(crate) fn snapshot(&self) -> &RaftPartitionSnapshot {
        &self.snapshot
    }

    /// Read one coherent view from the control plane and validate its placement inputs.
    pub fn from_control_plane(
        control_plane: &dyn GridControlPlaneHandle,
        partition_count: NonZeroU32,
        replication: ReplicationConfig,
    ) -> Result<Self, PartitionTopologyError> {
        let snapshot = control_plane
            .partition_snapshot()
            .ok_or(PartitionTopologyError::MissingSnapshot)?;
        Self::from_snapshot(snapshot, partition_count, replication)
    }

    /// Validate a committed roster before doing any placement calculation.
    pub fn from_snapshot(
        snapshot: RaftPartitionSnapshot,
        partition_count: NonZeroU32,
        replication: ReplicationConfig,
    ) -> Result<Self, PartitionTopologyError> {
        if snapshot.applied_index != snapshot.commit_index {
            return Err(PartitionTopologyError::Unapplied {
                applied: snapshot.applied_index,
                committed: snapshot.commit_index,
            });
        }
        replication.validate()?;
        let mut members = BTreeSet::new();
        for member in &snapshot.members {
            if !member.is_member()
                || member.epoch > snapshot.epoch
                || !members.insert(&member.node_id)
            {
                return Err(PartitionTopologyError::InvalidRoster);
            }
        }
        if snapshot.members.len() < replication.replication_factor {
            return Err(PartitionTopologyError::InsufficientMembers {
                available: snapshot.members.len(),
                required: replication.replication_factor,
            });
        }
        Ok(Self {
            snapshot,
            partition_count,
            replication_factor: replication.replication_factor,
        })
    }

    /// Compute a staged placement using the existing rendezvous ranking.
    pub fn placement(
        &self,
        partition: PartitionId,
    ) -> Result<PartitionPlacement, PartitionTopologyError> {
        if partition.value() >= self.partition_count.get() {
            return Err(PartitionTopologyError::UnknownPartition(partition.value()));
        }
        let members = &self.snapshot.members;
        let ranked = RendezvousClusterOwnership
            .replicas_for_key(
                &format!("partition-{}", partition.value()),
                members,
                self.replication_factor,
            )
            .ok_or(PartitionTopologyError::InvalidRoster)?;
        let identify = |node: ClusterNodeId| {
            members
                .iter()
                .find(|member| member.node_id == node)
                .map(|member| PartitionMember {
                    node,
                    generation: member.generation,
                })
                .ok_or(PartitionTopologyError::InvalidRoster)
        };
        Ok(PartitionPlacement {
            partition,
            epoch: self.snapshot.epoch,
            primary: identify(ranked.primary)?,
            backups: ranked
                .backups
                .into_iter()
                .map(identify)
                .collect::<Result<_, _>>()?,
        })
    }
}
