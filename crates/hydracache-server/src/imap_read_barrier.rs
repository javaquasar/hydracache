//! Check the owner-lane conditions of ADR-0024 before a value lookup.

use std::num::NonZeroU64;

use hydracache::{ClusterEpoch, PartitionId};
use thiserror::Error;

use crate::imap_topology::{PartitionMember, PartitionTopology, PartitionTopologyError};

/// The server-assigned invocation and authenticated scope covered by a read proof.
#[derive(Debug, PartialEq, Eq)]
pub struct ReadInvocation<'a> {
    pub invocation_id: NonZeroU64,
    pub tenant: &'a str,
    pub namespace: &'a str,
    pub namespace_generation: NonZeroU64,
    pub partition: PartitionId,
}

/// Inputs to the owner-lane safety check. The future Raft ReadIndex adapter must
/// obtain the quorum index for the invocation before assembling these inputs.
#[derive(Debug)]
pub struct OwnerReadBarrier<'a> {
    pub invocation: ReadInvocation<'a>,
    pub term: u64,
    pub epoch: ClusterEpoch,
    pub owner: PartitionMember,
    pub quorum_index: NonZeroU64,
    pub required_value_prefix: u64,
}

#[derive(Debug, PartialEq, Eq, Error)]
pub enum ReadBarrierError {
    #[error("read barrier belongs to a different invocation or scope")]
    WrongInvocation,
    #[error("read barrier term or epoch is stale")]
    StaleAuthority,
    #[error("read barrier owner is not the current local primary")]
    StaleOwner,
    #[error("local applied Raft index {applied} is below quorum index {required}")]
    MetadataBehind { applied: u64, required: u64 },
    #[error("local value prefix {applied} is below required prefix {required}")]
    ValueBehind { applied: u64, required: u64 },
    #[error(transparent)]
    Placement(#[from] PartitionTopologyError),
}

impl OwnerReadBarrier<'_> {
    /// Consume one proof and perform the lookup after verifying current authority.
    /// The caller reads the current topology and value prefix on the owner lane
    /// and keeps that lane through the synchronous lookup.
    pub fn lookup<T>(
        self,
        requested: &ReadInvocation<'_>,
        topology: &PartitionTopology,
        local: &PartitionMember,
        applied_value_prefix: u64,
        requested_value_prefix: u64,
        lookup: impl FnOnce() -> T,
    ) -> Result<T, ReadBarrierError> {
        if &self.invocation != requested {
            return Err(ReadBarrierError::WrongInvocation);
        }
        let snapshot = topology.snapshot();
        if self.term != snapshot.term || self.epoch != snapshot.epoch {
            return Err(ReadBarrierError::StaleAuthority);
        }
        let placement = topology.placement(self.invocation.partition)?;
        if &self.owner != local || self.owner != placement.primary {
            return Err(ReadBarrierError::StaleOwner);
        }
        let quorum_index = self.quorum_index.get();
        if snapshot.applied_index < quorum_index {
            return Err(ReadBarrierError::MetadataBehind {
                applied: snapshot.applied_index,
                required: quorum_index,
            });
        }
        let required = self.required_value_prefix.max(requested_value_prefix);
        if applied_value_prefix < required {
            return Err(ReadBarrierError::ValueBehind {
                applied: applied_value_prefix,
                required,
            });
        }
        Ok(lookup())
    }
}
