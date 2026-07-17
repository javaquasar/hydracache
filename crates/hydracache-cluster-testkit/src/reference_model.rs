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

/// External metadata intent consumed independently by the reference model and
/// by a runtime adapter in differential tests.
///
/// The model derives the expected epoch, command, and stable command id from
/// this intent. It never needs to inspect a runtime-produced command envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceMetadataIntent {
    /// Admit or refresh a member process.
    JoinMember {
        /// Stable node id.
        node_id: ClusterNodeId,
        /// Process generation supplied by the caller.
        generation: ClusterGeneration,
    },
    /// Admit or refresh a client process.
    JoinClient {
        /// Stable node id.
        node_id: ClusterNodeId,
        /// Process generation supplied by the caller.
        generation: ClusterGeneration,
    },
    /// Remove an admitted node at the supplied generation.
    Leave {
        /// Stable node id.
        node_id: ClusterNodeId,
        /// Generation used to fence the leave.
        generation: ClusterGeneration,
    },
}

impl ReferenceMetadataIntent {
    /// Build a generation-1 member admission.
    pub fn member(node_id: impl Into<ClusterNodeId>) -> Self {
        Self::JoinMember {
            node_id: node_id.into(),
            generation: ClusterGeneration::new(1),
        }
    }

    /// Build a generation-1 client admission.
    pub fn client(node_id: impl Into<ClusterNodeId>) -> Self {
        Self::JoinClient {
            node_id: node_id.into(),
            generation: ClusterGeneration::new(1),
        }
    }

    /// Build a generation-1 leave.
    pub fn leave(node_id: impl Into<ClusterNodeId>) -> Self {
        Self::Leave {
            node_id: node_id.into(),
            generation: ClusterGeneration::new(1),
        }
    }
}

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

    /// Apply one external intent and return the independently predicted
    /// committed envelope.
    pub fn apply_intent(
        &mut self,
        intent: &ReferenceMetadataIntent,
    ) -> Result<RaftMetadataCommandEnvelope, String> {
        let envelope = self.envelope_for_intent(intent)?;
        if !self.apply(&envelope)? {
            return Err(format!(
                "reference intent unexpectedly derived duplicate command id {}",
                envelope.command_id
            ));
        }
        Ok(envelope)
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

    fn envelope_for_intent(
        &self,
        intent: &ReferenceMetadataIntent,
    ) -> Result<RaftMetadataCommandEnvelope, String> {
        let command = match intent {
            ReferenceMetadataIntent::JoinMember {
                node_id,
                generation,
            } => {
                reject_candidate_generation(self, node_id, *generation)?;
                let advances_epoch = self
                    .members
                    .get(node_id)
                    .map(|current| *current < *generation)
                    .unwrap_or(true);
                let epoch = if advances_epoch {
                    ClusterEpoch::new(self.epoch.value().saturating_add(1))
                } else {
                    self.epoch
                };
                RaftMetadataCommand::MemberUpsert {
                    node_id: node_id.clone(),
                    generation: *generation,
                    epoch,
                }
            }
            ReferenceMetadataIntent::JoinClient {
                node_id,
                generation,
            } => {
                reject_candidate_generation(self, node_id, *generation)?;
                RaftMetadataCommand::ClientUpsert {
                    node_id: node_id.clone(),
                    generation: *generation,
                    epoch: self.epoch,
                }
            }
            ReferenceMetadataIntent::Leave {
                node_id,
                generation,
            } => {
                let (role, current_generation, epoch) =
                    if let Some(current) = self.members.get(node_id) {
                        (
                            ClusterRole::Member,
                            *current,
                            ClusterEpoch::new(self.epoch.value().saturating_add(1)),
                        )
                    } else if let Some(current) = self.clients.get(node_id) {
                        (ClusterRole::Client, *current, self.epoch)
                    } else {
                        return Err(format!("leave references absent node {node_id}"));
                    };
                if current_generation != *generation {
                    return Err(format!(
                        "leave generation for {node_id} was {}, expected {}",
                        generation.value(),
                        current_generation.value()
                    ));
                }
                RaftMetadataCommand::NodeLeft {
                    node_id: node_id.clone(),
                    role,
                    epoch,
                }
            }
        };
        Ok(RaftMetadataCommandEnvelope {
            command_id: expected_command_id(&command),
            command,
        })
    }
}

fn reject_candidate_generation(
    model: &ReferenceMetadataModel,
    node_id: &ClusterNodeId,
    generation: ClusterGeneration,
) -> Result<(), String> {
    let current = model
        .members
        .get(node_id)
        .or_else(|| model.clients.get(node_id));
    if current.is_some_and(|current| *current > generation) {
        Err(format!(
            "generation for {node_id} regressed to {}",
            generation.value()
        ))
    } else {
        Ok(())
    }
}

fn expected_command_id(command: &RaftMetadataCommand) -> String {
    let escaped =
        |node_id: &ClusterNodeId| node_id.as_str().replace('|', "%7C").replace(':', "%3A");
    match command {
        RaftMetadataCommand::MemberUpsert {
            node_id,
            generation,
            ..
        } => format!("member-upsert:{}:{}", escaped(node_id), generation.value()),
        RaftMetadataCommand::ClientUpsert {
            node_id,
            generation,
            ..
        } => format!("client-upsert:{}:{}", escaped(node_id), generation.value()),
        RaftMetadataCommand::NodeLeft { node_id, epoch, .. } => {
            format!("node-left:{}:{}", escaped(node_id), epoch.value())
        }
        RaftMetadataCommand::CommitTopology { epoch, members } => format!(
            "commit-topology:{}:{}",
            epoch.value(),
            members.iter().map(escaped).collect::<Vec<_>>().join(",")
        ),
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
