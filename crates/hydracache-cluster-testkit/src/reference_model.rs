//! Independent, deterministic model of committed cluster metadata.
//!
//! The model deliberately depends only on the transport-neutral command types.
//! It does not call the Raft runtime or `InMemoryCluster`, so differential tests
//! can detect disagreement between committed command replay and materialized
//! runtime state.

use std::collections::{BTreeMap, BTreeSet};

use hydracache::{
    ClusterEpoch, ClusterGeneration, ClusterNodeId, ClusterRole, RaftMetadataCommand,
};
use hydracache_cluster_raft::RaftMetadataCommandEnvelope;

/// Materialized view produced by [`ReferenceMetadataModel`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceMetadataView {
    /// Highest committed metadata epoch observed by the model.
    pub epoch: ClusterEpoch,
    /// Admitted members and their process generations.
    pub members: BTreeMap<ClusterNodeId, ClusterGeneration>,
    /// Admitted clients and their process generations.
    pub clients: BTreeMap<ClusterNodeId, ClusterGeneration>,
    /// Stable command ids applied exactly once.
    pub command_ids: Vec<String>,
    /// Last explicitly committed topology, when present.
    pub committed_topology: Option<(ClusterEpoch, BTreeSet<ClusterNodeId>)>,
}

/// Small independent state machine for committed metadata commands.
#[derive(Debug, Clone, Default)]
pub struct ReferenceMetadataModel {
    epoch: ClusterEpoch,
    members: BTreeMap<ClusterNodeId, ClusterGeneration>,
    clients: BTreeMap<ClusterNodeId, ClusterGeneration>,
    command_ids: Vec<String>,
    applied_ids: BTreeSet<String>,
    committed_topology: Option<(ClusterEpoch, BTreeSet<ClusterNodeId>)>,
}

impl ReferenceMetadataModel {
    /// Create an empty reference state machine.
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply one committed envelope, returning `false` for an idempotent retry.
    pub fn apply(&mut self, envelope: &RaftMetadataCommandEnvelope) -> Result<bool, String> {
        if self.applied_ids.contains(&envelope.command_id) {
            return Ok(false);
        }
        self.apply_command(&envelope.command)?;
        self.applied_ids.insert(envelope.command_id.clone());
        self.command_ids.push(envelope.command_id.clone());
        Ok(true)
    }

    /// Apply a sequence in order.
    pub fn apply_all<'a>(
        &mut self,
        envelopes: impl IntoIterator<Item = &'a RaftMetadataCommandEnvelope>,
    ) -> Result<(), String> {
        for envelope in envelopes {
            self.apply(envelope)?;
        }
        Ok(())
    }

    /// Return the current materialized view.
    pub fn view(&self) -> ReferenceMetadataView {
        ReferenceMetadataView {
            epoch: self.epoch,
            members: self.members.clone(),
            clients: self.clients.clone(),
            command_ids: self.command_ids.clone(),
            committed_topology: self.committed_topology.clone(),
        }
    }

    fn apply_command(&mut self, command: &RaftMetadataCommand) -> Result<(), String> {
        match command {
            RaftMetadataCommand::MemberUpsert {
                node_id,
                generation,
                epoch,
            } => {
                reject_epoch_regression(self.epoch, *epoch)?;
                reject_generation_regression(&self.members, node_id, *generation)?;
                self.clients.remove(node_id);
                self.members.insert(node_id.clone(), *generation);
                self.epoch = *epoch;
            }
            RaftMetadataCommand::ClientUpsert {
                node_id,
                generation,
                epoch,
            } => {
                reject_epoch_regression(self.epoch, *epoch)?;
                reject_generation_regression(&self.clients, node_id, *generation)?;
                self.members.remove(node_id);
                self.clients.insert(node_id.clone(), *generation);
                self.epoch = *epoch;
            }
            RaftMetadataCommand::NodeLeft {
                node_id,
                role,
                epoch,
            } => {
                reject_epoch_regression(self.epoch, *epoch)?;
                let removed = match role {
                    ClusterRole::Member => self.members.remove(node_id).is_some(),
                    ClusterRole::Client => self.clients.remove(node_id).is_some(),
                    ClusterRole::Local => false,
                };
                if !removed {
                    return Err(format!(
                        "node-left references absent {role:?} node {node_id}"
                    ));
                }
                self.epoch = *epoch;
            }
            RaftMetadataCommand::CommitTopology { epoch, members } => {
                reject_epoch_regression(self.epoch, *epoch)?;
                let member_count = members.len();
                let members = members.iter().cloned().collect::<BTreeSet<_>>();
                if members.len() != member_count {
                    return Err("committed topology contains duplicate members".to_owned());
                }
                self.committed_topology = Some((*epoch, members));
            }
        }
        Ok(())
    }
}

fn reject_epoch_regression(current: ClusterEpoch, candidate: ClusterEpoch) -> Result<(), String> {
    if candidate < current {
        Err(format!(
            "metadata epoch regressed from {} to {}",
            current.value(),
            candidate.value()
        ))
    } else {
        Ok(())
    }
}

fn reject_generation_regression(
    nodes: &BTreeMap<ClusterNodeId, ClusterGeneration>,
    node_id: &ClusterNodeId,
    generation: ClusterGeneration,
) -> Result<(), String> {
    if nodes
        .get(node_id)
        .is_some_and(|current| *current > generation)
    {
        Err(format!(
            "generation for {node_id} regressed to {}",
            generation.value()
        ))
    } else {
        Ok(())
    }
}
