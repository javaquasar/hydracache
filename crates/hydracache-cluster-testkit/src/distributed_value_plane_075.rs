//! Deterministic, test-only integration simulator for the provisional 0.75 value plane.
//!
//! This composes owner routing, the canonical mutation model, synchronous backup proof,
//! promotion, repair, rebalance, bounded bulk execution, and listener overflow without exposing a
//! production capability or freezing a wire/hash/durable identity.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;

use crate::value_plane_model_075::{
    CanonicalMapKey, MutationIdentity, MutationOutcome, MutationPlan, MutationStage,
    OutcomeCertainty, ReferenceValuePlane, ValuePlaneBounds, ValuePlaneError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimulatorBounds {
    pub partitions: u32,
    pub max_proxy_hops: u8,
    pub max_bulk_items: usize,
    pub max_listener_events: usize,
}

impl Default for SimulatorBounds {
    fn default() -> Self {
        Self {
            partitions: 17,
            max_proxy_hops: 1,
            max_bulk_items: 256,
            max_listener_events: 1_024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimulatorError {
    InvalidBound(&'static str),
    UnknownNode(String),
    NodeUnavailable(String),
    UnknownPartition(u32),
    StaleEpoch { expected: u64, actual: u64 },
    StaleOwner { expected: String, actual: String },
    ProxyHopLimit { limit: u8, actual: u8 },
    RequiredBackupUnavailable(String),
    NoPromotionCandidate,
    UnsafeEpoch { current: u64, proposed: u64 },
    BulkLimit { limit: usize, actual: usize },
    Model(ValuePlaneError),
}

impl fmt::Display for SimulatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SimulatorError {}

impl From<ValuePlaneError> for SimulatorError {
    fn from(value: ValuePlaneError) -> Self {
        Self::Model(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionAssignment {
    pub partition: u32,
    pub epoch: u64,
    pub owner: String,
    pub backup: String,
    pub backup_proved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionFault {
    None,
    LoseResponseAfterAcknowledgement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimMutationResult {
    pub outcome: MutationOutcome,
    pub owner: String,
    pub epoch: u64,
    pub proxied: bool,
    pub replayed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkItemResult {
    pub input_index: usize,
    pub key: CanonicalMapKey,
    pub result: Result<SimMutationResult, SimulatorError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimEventKind {
    Mutation,
    Gap,
    Promotion,
    Repair,
    Rebalance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimEvent {
    pub sequence: u64,
    pub kind: SimEventKind,
    pub partition: u32,
    pub epoch: u64,
    pub mutation: Option<MutationIdentity>,
}

#[derive(Debug, Clone)]
struct SimNode {
    live: bool,
    plane: ReferenceValuePlane,
}

/// Test-only cluster. All transitions are synchronous and use logical ordering only.
#[derive(Debug, Clone)]
pub struct DistributedValuePlaneSimulator {
    bounds: SimulatorBounds,
    epoch: u64,
    nodes: BTreeMap<String, SimNode>,
    assignments: BTreeMap<u32, PartitionAssignment>,
    events: VecDeque<SimEvent>,
    next_event_sequence: u64,
    dropped_event_count: u64,
}

impl DistributedValuePlaneSimulator {
    pub fn new(
        node_ids: impl IntoIterator<Item = impl Into<String>>,
        bounds: SimulatorBounds,
        model_bounds: ValuePlaneBounds,
    ) -> Result<Self, SimulatorError> {
        if bounds.partitions == 0 {
            return Err(SimulatorError::InvalidBound("partitions"));
        }
        if bounds.max_proxy_hops == 0 {
            return Err(SimulatorError::InvalidBound("proxy_hops"));
        }
        if bounds.max_bulk_items == 0 {
            return Err(SimulatorError::InvalidBound("bulk_items"));
        }
        if bounds.max_listener_events == 0 {
            return Err(SimulatorError::InvalidBound("listener_events"));
        }
        let node_ids = node_ids.into_iter().map(Into::into).collect::<Vec<_>>();
        if node_ids.len() < 2 {
            return Err(SimulatorError::InvalidBound("replica_members"));
        }
        if node_ids.iter().any(String::is_empty) {
            return Err(SimulatorError::InvalidBound("node_id"));
        }
        let unique = node_ids.iter().collect::<std::collections::BTreeSet<_>>();
        if unique.len() != node_ids.len() {
            return Err(SimulatorError::InvalidBound("unique_node_ids"));
        }
        let mut nodes = BTreeMap::new();
        for node_id in &node_ids {
            nodes.insert(
                node_id.clone(),
                SimNode {
                    live: true,
                    plane: ReferenceValuePlane::new(model_bounds, 1)?,
                },
            );
        }
        let mut assignments = BTreeMap::new();
        for partition in 0..bounds.partitions {
            let owner_index = partition as usize % node_ids.len();
            let backup_index = (owner_index + 1) % node_ids.len();
            assignments.insert(
                partition,
                PartitionAssignment {
                    partition,
                    epoch: 1,
                    owner: node_ids[owner_index].clone(),
                    backup: node_ids[backup_index].clone(),
                    backup_proved: true,
                },
            );
        }
        Ok(Self {
            bounds,
            epoch: 1,
            nodes,
            assignments,
            events: VecDeque::new(),
            next_event_sequence: 1,
            dropped_event_count: 0,
        })
    }

    pub const fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn assignment(&self, partition: u32) -> Option<&PartitionAssignment> {
        self.assignments.get(&partition)
    }

    pub fn execute(
        &mut self,
        entry_node: &str,
        plan: MutationPlan,
    ) -> Result<SimMutationResult, SimulatorError> {
        self.execute_with_fault(entry_node, plan, 0, ExecutionFault::None)
    }

    pub fn execute_with_fault(
        &mut self,
        entry_node: &str,
        plan: MutationPlan,
        proxy_hops: u8,
        fault: ExecutionFault,
    ) -> Result<SimMutationResult, SimulatorError> {
        self.require_live(entry_node)?;
        let assignment = self.assignment_for_key(&plan.key)?.clone();
        if plan.expected_epoch != assignment.epoch {
            return Err(SimulatorError::StaleEpoch {
                expected: plan.expected_epoch,
                actual: assignment.epoch,
            });
        }
        let proxied = entry_node != assignment.owner;
        if proxied && proxy_hops >= self.bounds.max_proxy_hops {
            return Err(SimulatorError::ProxyHopLimit {
                limit: self.bounds.max_proxy_hops,
                actual: proxy_hops,
            });
        }
        self.require_live(&assignment.owner)?;
        if !assignment.backup_proved || !self.node_live(&assignment.backup) {
            return Err(SimulatorError::RequiredBackupUnavailable(assignment.backup));
        }

        let owner_id = assignment.owner.clone();
        let backup_id = assignment.backup.clone();
        let mut owner = self
            .nodes
            .remove(&owner_id)
            .ok_or_else(|| SimulatorError::UnknownNode(owner_id.clone()))?;
        let mut backup = self
            .nodes
            .remove(&backup_id)
            .ok_or_else(|| SimulatorError::UnknownNode(backup_id.clone()))?;

        let identity = plan.identity.clone();
        let execution = (|| {
            if let Some(retained) = owner.plane.receive(plan.clone())? {
                let outcome = retained.outcome.ok_or(ValuePlaneError::UnknownMutation)?;
                return Ok((outcome, true));
            }
            advance_to_decision(&mut owner.plane, &identity)?;
            let owner_outcome = owner.plane.apply_owner(&identity)?;
            owner.plane.mark_visible_owner(&identity)?;

            if backup.plane.receive(plan)?.is_some() {
                return Err(SimulatorError::Model(
                    ValuePlaneError::MutationIdentityConflict,
                ));
            }
            advance_to_decision(&mut backup.plane, &identity)?;
            let backup_outcome = backup.plane.apply_owner(&identity)?;
            backup.plane.mark_visible_owner(&identity)?;
            if owner_outcome != backup_outcome {
                return Err(SimulatorError::Model(ValuePlaneError::ReplicaProofMismatch));
            }

            owner.plane.prove_replica(&identity, backup_id.clone())?;
            owner.plane.acknowledge(&identity)?;
            let outcome = match fault {
                ExecutionFault::None => owner.plane.respond(&identity)?,
                ExecutionFault::LoseResponseAfterAcknowledgement => {
                    owner.plane.lose_response(&identity)?
                }
            };
            Ok((outcome, false))
        })();
        self.nodes.insert(owner_id.clone(), owner);
        self.nodes.insert(backup_id, backup);

        let (outcome, replayed) = execution?;
        if outcome.applied && !replayed {
            self.publish(SimEventKind::Mutation, assignment.partition, Some(identity));
        }
        Ok(SimMutationResult {
            outcome,
            owner: owner_id,
            epoch: assignment.epoch,
            proxied,
            replayed,
        })
    }

    pub fn execute_bulk(
        &mut self,
        entry_node: &str,
        plans: Vec<MutationPlan>,
    ) -> Result<Vec<BulkItemResult>, SimulatorError> {
        if plans.len() > self.bounds.max_bulk_items {
            return Err(SimulatorError::BulkLimit {
                limit: self.bounds.max_bulk_items,
                actual: plans.len(),
            });
        }
        Ok(plans
            .into_iter()
            .enumerate()
            .map(|(input_index, plan)| {
                let key = plan.key.clone();
                let result = self.execute(entry_node, plan);
                BulkItemResult {
                    input_index,
                    key,
                    result,
                }
            })
            .collect())
    }

    pub fn read(
        &self,
        entry_node: &str,
        key: &CanonicalMapKey,
        proxy_hops: u8,
    ) -> Result<Option<Vec<u8>>, SimulatorError> {
        self.require_live(entry_node)?;
        let assignment = self.assignment_for_key(key)?;
        if entry_node != assignment.owner && proxy_hops >= self.bounds.max_proxy_hops {
            return Err(SimulatorError::ProxyHopLimit {
                limit: self.bounds.max_proxy_hops,
                actual: proxy_hops,
            });
        }
        self.require_live(&assignment.owner)?;
        Ok(self
            .nodes
            .get(&assignment.owner)
            .and_then(|node| node.plane.live_value(key))
            .map(<[u8]>::to_vec))
    }

    pub fn fail_node(&mut self, node_id: &str) -> Result<(), SimulatorError> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| SimulatorError::UnknownNode(node_id.to_owned()))?;
        node.live = false;
        Ok(())
    }

    pub fn restore_node(&mut self, node_id: &str) -> Result<(), SimulatorError> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| SimulatorError::UnknownNode(node_id.to_owned()))?;
        node.live = true;
        Ok(())
    }

    pub fn promote_backup(
        &mut self,
        partition: u32,
        new_epoch: u64,
    ) -> Result<PartitionAssignment, SimulatorError> {
        if new_epoch <= self.epoch {
            return Err(SimulatorError::UnsafeEpoch {
                current: self.epoch,
                proposed: new_epoch,
            });
        }
        let current = self
            .assignments
            .get(&partition)
            .cloned()
            .ok_or(SimulatorError::UnknownPartition(partition))?;
        if !self.node_live(&current.backup) {
            return Err(SimulatorError::NoPromotionCandidate);
        }
        let next_backup = self
            .nodes
            .iter()
            .find_map(|(node_id, node)| {
                (node.live && node_id != &current.backup && node_id != &current.owner)
                    .then_some(node_id.clone())
            })
            .unwrap_or(current.owner.clone());
        self.advance_cluster_epoch(new_epoch, &current.backup)?;
        let assignment = PartitionAssignment {
            partition,
            epoch: new_epoch,
            owner: current.backup,
            backup: next_backup,
            backup_proved: false,
        };
        self.assignments.insert(partition, assignment.clone());
        self.publish(SimEventKind::Promotion, partition, None);
        Ok(assignment)
    }

    pub fn repair_backup(&mut self, partition: u32, target: &str) -> Result<(), SimulatorError> {
        self.require_live(target)?;
        let assignment = self
            .assignments
            .get(&partition)
            .cloned()
            .ok_or(SimulatorError::UnknownPartition(partition))?;
        self.require_live(&assignment.owner)?;
        if target == assignment.owner {
            return Err(SimulatorError::StaleOwner {
                expected: assignment.owner.clone(),
                actual: target.to_owned(),
            });
        }
        let source = self
            .nodes
            .get(&assignment.owner)
            .expect("owner checked above")
            .plane
            .clone();
        self.nodes
            .get_mut(target)
            .expect("target checked above")
            .plane = source;
        let assignment = self
            .assignments
            .get_mut(&partition)
            .expect("partition checked above");
        assignment.backup = target.to_owned();
        assignment.backup_proved = true;
        self.publish(SimEventKind::Repair, partition, None);
        Ok(())
    }

    pub fn rebalance(
        &mut self,
        partition: u32,
        target: &str,
        new_epoch: u64,
    ) -> Result<PartitionAssignment, SimulatorError> {
        self.require_live(target)?;
        if new_epoch <= self.epoch {
            return Err(SimulatorError::UnsafeEpoch {
                current: self.epoch,
                proposed: new_epoch,
            });
        }
        let current = self
            .assignments
            .get(&partition)
            .cloned()
            .ok_or(SimulatorError::UnknownPartition(partition))?;
        self.require_live(&current.owner)?;
        if target == current.owner {
            return Err(SimulatorError::StaleOwner {
                expected: current.owner,
                actual: target.to_owned(),
            });
        }
        let source = self
            .nodes
            .get(&current.owner)
            .expect("owner checked above")
            .plane
            .clone();
        self.nodes
            .get_mut(target)
            .expect("target checked above")
            .plane = source;
        self.advance_cluster_epoch(new_epoch, target)?;
        let assignment = PartitionAssignment {
            partition,
            epoch: new_epoch,
            owner: target.to_owned(),
            backup: current.owner,
            backup_proved: true,
        };
        self.assignments.insert(partition, assignment.clone());
        self.publish(SimEventKind::Rebalance, partition, None);
        Ok(assignment)
    }

    pub fn drain_events(&mut self) -> Vec<SimEvent> {
        self.events.drain(..).collect()
    }

    pub const fn dropped_event_count(&self) -> u64 {
        self.dropped_event_count
    }

    pub fn node_value(
        &self,
        node_id: &str,
        key: &CanonicalMapKey,
    ) -> Result<Option<Vec<u8>>, SimulatorError> {
        let node = self
            .nodes
            .get(node_id)
            .ok_or_else(|| SimulatorError::UnknownNode(node_id.to_owned()))?;
        Ok(node.plane.live_value(key).map(<[u8]>::to_vec))
    }

    fn assignment_for_key(
        &self,
        key: &CanonicalMapKey,
    ) -> Result<&PartitionAssignment, SimulatorError> {
        self.assignments
            .get(&key.partition)
            .ok_or(SimulatorError::UnknownPartition(key.partition))
    }

    fn require_live(&self, node_id: &str) -> Result<(), SimulatorError> {
        match self.nodes.get(node_id) {
            Some(node) if node.live => Ok(()),
            Some(_) => Err(SimulatorError::NodeUnavailable(node_id.to_owned())),
            None => Err(SimulatorError::UnknownNode(node_id.to_owned())),
        }
    }

    fn node_live(&self, node_id: &str) -> bool {
        self.nodes.get(node_id).is_some_and(|node| node.live)
    }

    fn advance_cluster_epoch(
        &mut self,
        new_epoch: u64,
        authority: &str,
    ) -> Result<(), SimulatorError> {
        for node in self.nodes.values_mut() {
            if node.live {
                node.plane.promote(new_epoch, authority.to_owned())?;
            }
        }
        self.epoch = new_epoch;
        for assignment in self.assignments.values_mut() {
            assignment.epoch = new_epoch;
        }
        Ok(())
    }

    fn publish(&mut self, kind: SimEventKind, partition: u32, mutation: Option<MutationIdentity>) {
        if self.events.len() >= self.bounds.max_listener_events {
            self.dropped_event_count = self
                .dropped_event_count
                .saturating_add(self.events.len() as u64);
            self.events.clear();
            self.events.push_back(SimEvent {
                sequence: self.next_event_sequence,
                kind: SimEventKind::Gap,
                partition,
                epoch: self.epoch,
                mutation: None,
            });
            self.next_event_sequence = self.next_event_sequence.saturating_add(1);
            return;
        }
        self.events.push_back(SimEvent {
            sequence: self.next_event_sequence,
            kind,
            partition,
            epoch: self.epoch,
            mutation,
        });
        self.next_event_sequence = self.next_event_sequence.saturating_add(1);
    }
}

fn advance_to_decision(
    plane: &mut ReferenceValuePlane,
    identity: &MutationIdentity,
) -> Result<(), ValuePlaneError> {
    for stage in [
        MutationStage::Admitted,
        MutationStage::Routed,
        MutationStage::OwnerDecided,
    ] {
        plane.advance(identity, stage)?;
    }
    Ok(())
}

pub fn certainty_label(certainty: OutcomeCertainty) -> &'static str {
    match certainty {
        OutcomeCertainty::Certain => "certain",
        OutcomeCertainty::OutcomeUnknown => "outcome_unknown",
    }
}
