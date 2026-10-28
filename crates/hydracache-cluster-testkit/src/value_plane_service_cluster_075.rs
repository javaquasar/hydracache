//! Test-only composition of the actual IMap service, owner routing and logical fault transport.
//! No production partition hash, wire schema, durable identity or distributed transaction is defined.

use crate::value_plane_collection_075::CollectionBounds;
use crate::value_plane_loopback_075::{
    LoopbackError, LoopbackFault, LoopbackFrame, LoopbackTransport,
};
use crate::value_plane_model_075::MutationIdentity;
use crate::value_plane_service_075::*;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceOperationScope {
    PointOwner,
    BoundedBulkFanout,
    LocalMetadata,
    SinglePartitionMapWide,
}

const CORE_OPERATIONS: [&str; 40] = [
    "get",
    "put",
    "delete",
    "conditional_replace",
    "conditional_remove",
    "contains_key",
    "put_if_absent",
    "replace_if_present",
    "get_and_remove",
    "get_and_put",
    "get_all",
    "put_all_detailed",
    "remove_all_detailed",
    "set_ttl",
    "remaining_ttl",
    "entry_listener",
    "size",
    "is_empty",
    "contains_value_bounded",
    "clear_detailed",
    "destroy",
    "evict",
    "evict_all_detailed",
    "key_set_page",
    "values_page",
    "entry_set_page",
    "get_or_default",
    "contains_entry",
    "set",
    "get_entry_view",
    "try_put",
    "try_remove",
    "replace_all_detailed",
    "remove_entry_listener",
    "capability_descriptor",
    "replace_if_version",
    "remove_if_version",
    "set_ttl_if_version",
    "query_operation_outcome",
    "local_statistics",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceOperationSupport {
    pub operation: &'static str,
    pub scope: ServiceOperationScope,
    pub supported: bool,
}

pub fn service_operation_scope(operation: &str) -> Option<ServiceOperationScope> {
    use ServiceOperationScope::*;
    Some(match operation {
        "get_all" | "put_all_detailed" | "remove_all_detailed" | "replace_all_detailed" => {
            BoundedBulkFanout
        }
        "local_statistics" | "capability_descriptor" | "query_operation_outcome" => LocalMetadata,
        "size"
        | "is_empty"
        | "contains_value_bounded"
        | "clear_detailed"
        | "destroy"
        | "evict_all_detailed"
        | "key_set_page"
        | "values_page"
        | "entry_set_page"
        | "entry_listener"
        | "remove_entry_listener" => SinglePartitionMapWide,
        "get"
        | "put"
        | "delete"
        | "conditional_replace"
        | "conditional_remove"
        | "contains_key"
        | "put_if_absent"
        | "replace_if_present"
        | "get_and_remove"
        | "get_and_put"
        | "set_ttl"
        | "remaining_ttl"
        | "evict"
        | "get_or_default"
        | "contains_entry"
        | "set"
        | "get_entry_view"
        | "try_put"
        | "try_remove"
        | "replace_if_version"
        | "remove_if_version"
        | "set_ttl_if_version" => PointOwner,
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy)]
pub struct ServiceClusterBounds {
    pub partitions: u32,
    pub max_nodes: usize,
    pub max_frame_bytes: usize,
    pub max_transport_ticks: usize,
    pub max_bulk_items: usize,
    pub max_trace_events: usize,
}
impl Default for ServiceClusterBounds {
    fn default() -> Self {
        Self {
            partitions: 3,
            max_nodes: 8,
            max_frame_bytes: 64 * 1_024 * 1_024,
            max_transport_ticks: 8,
            max_bulk_items: 256,
            max_trace_events: 1_024,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ServiceLinkFaults {
    pub request: LoopbackFault,
    pub replica: LoopbackFault,
    pub ack: LoopbackFault,
}
impl Default for ServiceLinkFaults {
    fn default() -> Self {
        Self {
            request: LoopbackFault::Deliver,
            replica: LoopbackFault::Deliver,
            ack: LoopbackFault::Deliver,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceClusterFaults {
    pub data: ServiceLinkFaults,
    pub coordinator: ServiceLinkFaults,
    pub partitions: BTreeMap<u32, ServiceLinkFaults>,
    pub response: LoopbackFault,
}
impl Default for ServiceClusterFaults {
    fn default() -> Self {
        Self {
            data: ServiceLinkFaults::default(),
            coordinator: ServiceLinkFaults::default(),
            partitions: BTreeMap::new(),
            response: LoopbackFault::Deliver,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ServiceClusterCall {
    pub expected_epoch: Option<u64>,
    pub proxy_hops: u8,
    pub faults: ServiceClusterFaults,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceClusterError {
    InvalidBound,
    UnknownNode,
    NodeUnavailable,
    UnsupportedScope,
    StaleEpoch,
    StaleOwner,
    ProxyHopLimit,
    ReplicaNotProved,
    TransportTimeout,
    Transport(LoopbackError),
    Model(MapServiceError),
}
impl fmt::Display for ServiceClusterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ServiceClusterError {}
impl From<MapServiceError> for ServiceClusterError {
    fn from(value: MapServiceError) -> Self {
        Self::Model(value)
    }
}
impl From<LoopbackError> for ServiceClusterError {
    fn from(value: LoopbackError) -> Self {
        Self::Transport(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServicePartitionAssignment {
    pub partition: u32,
    pub epoch: u64,
    pub owner: String,
    pub backup: String,
}

#[derive(Debug, Clone)]
pub struct ServiceClusterExecution {
    pub result: ServiceResult,
    pub replayed: bool,
    pub backup_proved: bool,
    pub routes: Vec<ServicePartitionAssignment>,
    pub failure: Option<ServiceClusterError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceClusterTrace {
    pub stage: &'static str,
    pub partition: u32,
    pub delivered: usize,
}

struct ServiceNode {
    live: bool,
    data: Vec<UnifiedIMapService>,
    journal: Vec<UnifiedIMapService>,
}
struct Assignment {
    route: ServicePartitionAssignment,
    required_data: ReplicaStamp,
    required_journal: ReplicaStamp,
    backup_data: Option<ReplicaStamp>,
    backup_journal: Option<ReplicaStamp>,
}

/// A coordinator journal is another actual service ledger, never a second copy of map values.
/// Pending roots block promotion until their data decisions and final receipt are proved.
pub struct ServiceClusterHarness {
    bounds: ServiceClusterBounds,
    generation: u64,
    nodes: BTreeMap<String, ServiceNode>,
    assignments: Vec<Assignment>,
    sequence: u64,
    trace: VecDeque<ServiceClusterTrace>,
    dropped_trace_events: u64,
    max_response_bytes: usize,
}

impl ServiceClusterHarness {
    pub fn new(
        node_ids: &[&str],
        bounds: ServiceClusterBounds,
        collection: CollectionBounds,
        limits: ServiceLimits,
        generation: u64,
    ) -> Result<Self, ServiceClusterError> {
        if node_ids.len() < 2
            || node_ids.len() > bounds.max_nodes
            || bounds.partitions == 0
            || bounds.partitions > 256
            || bounds.max_frame_bytes == 0
            || bounds.max_transport_ticks == 0
            || bounds.max_transport_ticks > 1_024
            || bounds.max_bulk_items == 0
            || bounds.max_bulk_items > 256
            || bounds.max_trace_events == 0
            || node_ids.iter().any(|id| id.is_empty() || id.len() > 128)
            || node_ids.iter().collect::<BTreeSet<_>>().len() != node_ids.len()
        {
            return Err(ServiceClusterError::InvalidBound);
        }
        let mut nodes = BTreeMap::new();
        for id in node_ids {
            let mut data = Vec::new();
            let mut journal = Vec::new();
            for _ in 0..bounds.partitions {
                data.push(UnifiedIMapService::with_limits(
                    collection, generation, limits,
                )?);
                journal.push(UnifiedIMapService::with_limits(
                    collection, generation, limits,
                )?);
            }
            nodes.insert(
                (*id).to_string(),
                ServiceNode {
                    live: true,
                    data,
                    journal,
                },
            );
        }
        let mut assignments = Vec::new();
        for partition in 0..bounds.partitions {
            let owner = node_ids[partition as usize % node_ids.len()];
            let backup = node_ids[(partition as usize + 1) % node_ids.len()];
            let data = &nodes[owner].data[partition as usize];
            let control = &nodes[owner].journal[partition as usize];
            let data_stamp = nodes[backup].data[partition as usize]
                .install_replica_image(data.export_replica_image()?)?;
            let journal_stamp = nodes[backup].journal[partition as usize]
                .install_replica_image(control.export_replica_image()?)?;
            assignments.push(Assignment {
                route: ServicePartitionAssignment {
                    partition,
                    epoch: 1,
                    owner: owner.into(),
                    backup: backup.into(),
                },
                required_data: data_stamp,
                required_journal: journal_stamp,
                backup_data: Some(data_stamp),
                backup_journal: Some(journal_stamp),
            });
        }
        Ok(Self {
            bounds,
            generation,
            nodes,
            assignments,
            sequence: 1,
            trace: VecDeque::new(),
            dropped_trace_events: 0,
            max_response_bytes: collection.max_response_bytes,
        })
    }

    /// This routing function belongs only to the test harness; it is not a product hash identity.
    pub fn partition_for(&self, key: &[u8]) -> u32 {
        partition(key, self.bounds.partitions)
    }
    pub fn coordinator_for(&self, identity: &MutationIdentity) -> u32 {
        partition(
            format!("{}:{}", identity.client, identity.sequence).as_bytes(),
            self.bounds.partitions,
        )
    }
    pub fn assignment(&self, partition: u32) -> Option<ServicePartitionAssignment> {
        self.assignments
            .get(partition as usize)
            .map(|a| a.route.clone())
    }
    pub fn supports_operation(&self, operation: &str) -> bool {
        service_operation_scope(operation).is_some_and(|scope| {
            scope != ServiceOperationScope::SinglePartitionMapWide || self.bounds.partitions == 1
        })
    }
    pub fn operation_inventory(&self) -> Vec<ServiceOperationSupport> {
        CORE_OPERATIONS
            .iter()
            .map(|operation| ServiceOperationSupport {
                operation,
                scope: service_operation_scope(operation).expect("declared operation has a scope"),
                supported: self.supports_operation(operation),
            })
            .collect()
    }
    pub fn trace(&self) -> &VecDeque<ServiceClusterTrace> {
        &self.trace
    }
    pub const fn dropped_trace_events(&self) -> u64 {
        self.dropped_trace_events
    }
    pub fn set_node_live(&mut self, node: &str, live: bool) -> Result<(), ServiceClusterError> {
        self.nodes
            .get_mut(node)
            .ok_or(ServiceClusterError::UnknownNode)?
            .live = live;
        Ok(())
    }

    pub fn diagnostic_node_snapshot(
        &self,
        node: &str,
        partition: u32,
        now: u64,
    ) -> Result<Vec<ServiceEntry>, ServiceClusterError> {
        let service = self.service(node, partition, false)?;
        Ok(service.diagnostic_snapshot(now)?)
    }

    pub fn query_operation_outcome(
        &self,
        identity: &MutationIdentity,
    ) -> Result<OperationOutcomeQuery, ServiceClusterError> {
        let route = &self.assignments[self.coordinator_for(identity) as usize].route;
        self.require_live(&route.owner)?;
        Ok(self
            .service(&route.owner, route.partition, true)?
            .query_operation_outcome(identity)?)
    }

    /// Cancellation after admission retains a terminal unknown receipt; it never makes the
    /// original identity available for a different request or silently rolls back applied keys.
    pub fn cancel_pending(
        &mut self,
        command: &ServiceCommand,
    ) -> Result<bool, ServiceClusterError> {
        let identity = command
            .identity
            .as_ref()
            .ok_or(MapServiceError::InvalidArgument)?;
        let p = self.coordinator_for(identity);
        let route = self.assignments[p as usize].route.clone();
        self.require_live(&route.owner)?;
        let journal = self.service(&route.owner, p, true)?;
        let digest = request_digest(command);
        if !matches!(
            journal.recover_external_operation(identity, digest)?,
            ServiceRecovery::Pending
        ) {
            return Ok(false);
        }
        journal.complete_external_operation(
            identity,
            digest,
            &ServiceResult::new("outcome_unknown"),
        )?;
        if let Ok(stamp) = journal.replica_stamp() {
            self.assignments[p as usize].required_journal = stamp;
        }
        if !self.replicate(p, true, ServiceLinkFaults::default())? {
            return Err(ServiceClusterError::ReplicaNotProved);
        }
        Ok(true)
    }

    pub fn backup_proved(&self, partition: u32) -> bool {
        let Some(assignment) = self.assignments.get(partition as usize) else {
            return false;
        };
        let Ok(control) = self.service(&assignment.route.owner, partition, true) else {
            return false;
        };
        if control.namespace_owners().pending != 0 {
            return false;
        }
        assignment.backup_data == Some(assignment.required_data)
            && assignment.backup_journal == Some(assignment.required_journal)
    }

    pub fn execute(
        &mut self,
        entry_node: &str,
        command: ServiceCommand,
        call: ServiceClusterCall,
    ) -> Result<ServiceClusterExecution, ServiceClusterError> {
        self.require_live(entry_node)?;
        self.service(entry_node, 0, false)?
            .validate_request(&command)?;
        let scope = service_operation_scope(&command.operation)
            .ok_or(ServiceClusterError::UnsupportedScope)?;
        if scope == ServiceOperationScope::SinglePartitionMapWide && self.bounds.partitions != 1 {
            return Err(ServiceClusterError::UnsupportedScope);
        }
        if scope == ServiceOperationScope::LocalMetadata {
            return self.metadata(entry_node, command);
        }
        let groups = self.groups(&command, scope)?;
        for (p, _, _) in &groups {
            self.route(entry_node, *p, &call)?;
        }
        if !is_mutating_operation(&command.operation) {
            let mut bytes = 0usize;
            for (p, group, _) in &groups {
                let route = &self.assignments[*p as usize].route;
                bytes = bytes
                    .checked_add(
                        self.service(&route.owner, *p, false)?
                            .encoded_response_bound(group)?,
                    )
                    .ok_or(ServiceClusterError::InvalidBound)?;
            }
            self.check_response_bytes(bytes)?;
            return self.read_groups(entry_node, command, groups, &call);
        }
        let identity = command
            .identity
            .clone()
            .ok_or(MapServiceError::InvalidArgument)?;
        let coordinator = self.coordinator_for(&identity);
        let coordinator_route = self.route(entry_node, coordinator, &call)?;
        let journal = self.service(&coordinator_route.owner, coordinator, true)?;
        let digest = request_digest(&command);
        let recovered = journal.recover_external_operation(&identity, digest)?;
        if let ServiceRecovery::Completed(result) = recovered {
            let proved = self
                .replicate(coordinator, true, call.faults.coordinator)
                .unwrap_or(false);
            return self.finish(
                execution(
                    *result,
                    true,
                    proved,
                    vec![coordinator_route],
                    if proved {
                        None
                    } else {
                        Some(ServiceClusterError::ReplicaNotProved)
                    },
                ),
                &call,
                command.lose_response,
            );
        }
        let root_pending = matches!(recovered, ServiceRecovery::Pending);
        if command.cancelled || command.now >= command.deadline {
            return Ok(execution(
                ServiceResult::new(if root_pending {
                    "outcome_unknown"
                } else {
                    "not_applied"
                }),
                root_pending,
                false,
                vec![coordinator_route],
                None,
            ));
        }
        let mut maximum_result = 64usize;
        let mut response_bytes = 0usize;
        let mut prepared = Vec::new();
        for (p, mut group, indices) in groups {
            let route = self.route(entry_node, p, &call)?;
            self.require_live(&route.backup)?;
            if !root_pending && !self.backup_proved(p) {
                return Err(ServiceClusterError::ReplicaNotProved);
            }
            group.identity = Some(data_identity(&identity, p));
            group.lose_response = false;
            let owner = self.service(&route.owner, p, false)?;
            match owner.recover_command(&group, group.identity.as_ref().expect("group identity"))? {
                ServiceRecovery::Unknown => {
                    maximum_result = maximum_result
                        .checked_add(owner.command_result_bound(&group)?.saturating_sub(64))
                        .ok_or(ServiceClusterError::InvalidBound)?;
                    response_bytes = response_bytes
                        .checked_add(owner.encoded_response_bound(&group)?)
                        .ok_or(ServiceClusterError::InvalidBound)?;
                }
                ServiceRecovery::Completed(ref result) => {
                    maximum_result = maximum_result
                        .checked_add(service_result_bytes(result).saturating_sub(64))
                        .ok_or(ServiceClusterError::InvalidBound)?;
                    response_bytes = response_bytes
                        .checked_add(service_encoded_result_bytes(result))
                        .ok_or(ServiceClusterError::InvalidBound)?;
                }
                ServiceRecovery::Pending => return Err(ServiceClusterError::ReplicaNotProved),
            }
            prepared.push((p, group, indices, route));
        }
        self.check_response_bytes(response_bytes)?;
        if maximum_result > 8 * 1_024 * 1_024 {
            return Err(MapServiceError::ResourceBudget("root_result").into());
        }
        let request_payload = command.test_transport_payload(self.bounds.max_frame_bytes)?;
        if self.transmit(
            coordinator,
            "request",
            request_payload,
            call.faults.coordinator.request,
        )? == 0
        {
            return Ok(execution(
                ServiceResult::new("not_applied"),
                false,
                false,
                vec![coordinator_route],
                None,
            ));
        }
        journal.admit_external_operation(identity.clone(), digest, maximum_result)?;
        let mut combined = ServiceResult::new("bulk");
        let mut routes = vec![coordinator_route];
        let mut failure = None;
        let mut decided = false;
        for (p, group, indices, route) in prepared {
            routes.push(route.clone());
            let faults = call
                .faults
                .partitions
                .get(&p)
                .copied()
                .unwrap_or(call.faults.data);
            let deliveries = self.transmit(
                p,
                "owner_request",
                group.test_transport_payload(self.bounds.max_frame_bytes)?,
                faults.request,
            );
            let deliveries = match deliveries {
                Ok(n) if n > 0 => n,
                Ok(_) => {
                    failure = Some(ServiceClusterError::TransportTimeout);
                    continue;
                }
                Err(error) => {
                    failure = Some(error);
                    continue;
                }
            };
            let owner = self.service(&route.owner, p, false)?;
            let mut result = match owner.execute(group.clone()) {
                Ok(result) => result,
                Err(error) => {
                    failure = Some(error.into());
                    continue;
                }
            };
            for _ in 1..deliveries {
                let replay = owner.execute(group.clone())?;
                if replay != result {
                    return Err(ServiceClusterError::ReplicaNotProved);
                }
            }
            decided = true;
            self.assignments[p as usize].required_data = owner.replica_stamp()?;
            match self.replicate(p, false, faults) {
                Ok(true) => {}
                Ok(false) => failure = Some(ServiceClusterError::ReplicaNotProved),
                Err(error) => failure = Some(error),
            }
            if scope == ServiceOperationScope::BoundedBulkFanout {
                for item in &mut result.items {
                    item.index = indices[item.index];
                }
                combined.items.extend(result.items);
            } else {
                combined = result;
            }
        }
        if let Some(error) = failure {
            if !decided && !root_pending {
                journal.cancel_reserved(&identity)?;
            }
            return Ok(execution(
                ServiceResult::new(if decided || root_pending {
                    "outcome_unknown"
                } else {
                    "not_applied"
                }),
                root_pending,
                false,
                routes,
                Some(error),
            ));
        }
        if scope == ServiceOperationScope::BoundedBulkFanout {
            combined.items.sort_by_key(|i| i.index);
            combined.number = combined.items.len() as u64;
        }
        journal.complete_external_operation(&identity, digest, &combined)?;
        if let Ok(stamp) = journal.replica_stamp() {
            self.assignments[coordinator as usize].required_journal = stamp;
        }
        let proof = self.replicate(coordinator, true, call.faults.coordinator);
        let (proved, failure) = match proof {
            Ok(true) => (true, None),
            Ok(false) => (false, Some(ServiceClusterError::ReplicaNotProved)),
            Err(error) => (false, Some(error)),
        };
        self.finish(
            execution(combined, root_pending, proved, routes, failure),
            &call,
            command.lose_response,
        )
    }

    pub fn execute_at_owner(
        &mut self,
        owner: &str,
        partition: u32,
        epoch: u64,
        command: ServiceCommand,
    ) -> Result<ServiceClusterExecution, ServiceClusterError> {
        let route = self
            .assignments
            .get(partition as usize)
            .ok_or(ServiceClusterError::InvalidBound)?
            .route
            .clone();
        if route.owner != owner {
            return Err(ServiceClusterError::StaleOwner);
        }
        if route.epoch != epoch {
            return Err(ServiceClusterError::StaleEpoch);
        }
        self.execute(
            owner,
            command,
            ServiceClusterCall {
                expected_epoch: Some(epoch),
                ..ServiceClusterCall::default()
            },
        )
    }

    pub fn promote_backup(
        &mut self,
        partition: u32,
    ) -> Result<ServicePartitionAssignment, ServiceClusterError> {
        let old = self
            .assignments
            .get(partition as usize)
            .ok_or(ServiceClusterError::InvalidBound)?
            .route
            .clone();
        self.require_live(&old.backup)?;
        if !self.backup_proved(partition) {
            return Err(ServiceClusterError::ReplicaNotProved);
        }
        let epoch = old
            .epoch
            .checked_add(1)
            .ok_or(ServiceClusterError::InvalidBound)?;
        let backup = self
            .nodes
            .iter()
            .find(|(id, node)| node.live && **id != old.backup)
            .map(|(id, _)| id.clone())
            .ok_or(ServiceClusterError::NodeUnavailable)?;
        let assignment = &mut self.assignments[partition as usize];
        assignment.route = ServicePartitionAssignment {
            partition,
            epoch,
            owner: old.backup,
            backup,
        };
        assignment.backup_data = None;
        assignment.backup_journal = None;
        Ok(assignment.route.clone())
    }

    pub fn repair_backup(
        &mut self,
        partition: u32,
        backup: &str,
        faults: ServiceLinkFaults,
    ) -> Result<(), ServiceClusterError> {
        self.require_live(backup)?;
        let route = self
            .assignments
            .get(partition as usize)
            .ok_or(ServiceClusterError::InvalidBound)?
            .route
            .clone();
        self.require_live(&route.owner)?;
        if route.owner == backup {
            return Err(ServiceClusterError::InvalidBound);
        }
        self.assignments[partition as usize].route.backup = backup.into();
        self.assignments[partition as usize].backup_data = None;
        self.assignments[partition as usize].backup_journal = None;
        let data = self.replicate(partition, false, faults)?;
        let journal = self.replicate(partition, true, faults)?;
        if !data || !journal {
            return Err(ServiceClusterError::ReplicaNotProved);
        }
        Ok(())
    }

    pub fn rebalance(
        &mut self,
        partition: u32,
        target: &str,
        faults: ServiceLinkFaults,
    ) -> Result<ServicePartitionAssignment, ServiceClusterError> {
        self.require_live(target)?;
        let old = self
            .assignments
            .get(partition as usize)
            .ok_or(ServiceClusterError::InvalidBound)?
            .route
            .clone();
        self.require_live(&old.owner)?;
        if target == old.owner {
            return Err(ServiceClusterError::InvalidBound);
        }
        if self
            .service(&old.owner, partition, true)?
            .namespace_owners()
            .pending
            != 0
        {
            return Err(ServiceClusterError::ReplicaNotProved);
        }
        for journal in [false, true] {
            let source = self.service(&old.owner, partition, journal)?;
            let image = source.export_replica_image()?;
            let expected = image.stamp();
            if self.transmit(
                partition,
                "transfer",
                image.transport_payload(self.bounds.max_frame_bytes)?,
                faults.replica,
            )? == 0
            {
                return Err(ServiceClusterError::TransportTimeout);
            }
            let installed = self
                .service(target, partition, journal)?
                .install_replica_image(image)?;
            if installed != expected {
                return Err(ServiceClusterError::ReplicaNotProved);
            }
            if self.transmit(
                partition,
                "transfer_ack",
                expected.digest.to_vec(),
                faults.ack,
            )? == 0
            {
                return Err(ServiceClusterError::TransportTimeout);
            }
        }
        let epoch = old
            .epoch
            .checked_add(1)
            .ok_or(ServiceClusterError::InvalidBound)?;
        let assignment = &mut self.assignments[partition as usize];
        assignment.route = ServicePartitionAssignment {
            partition,
            epoch,
            owner: target.into(),
            backup: old.owner,
        };
        assignment.backup_data = Some(assignment.required_data);
        assignment.backup_journal = Some(assignment.required_journal);
        Ok(assignment.route.clone())
    }

    pub fn retire_outcomes(
        &mut self,
        client: &str,
        through: u64,
    ) -> Result<(), ServiceClusterError> {
        for partition in 0..self.bounds.partitions {
            let route = self.assignments[partition as usize].route.clone();
            for journal in [false, true] {
                let identity_client = if journal {
                    client.to_string()
                } else {
                    data_identity(&MutationIdentity::new(client, through), partition).client
                };
                let service = self.service(&route.owner, partition, journal)?;
                if service
                    .pending_identities()
                    .iter()
                    .any(|id| id.client == identity_client && id.sequence <= through)
                {
                    return Err(MapServiceError::PendingOperation.into());
                }
            }
        }
        for partition in 0..self.bounds.partitions {
            let route = self.assignments[partition as usize].route.clone();
            for journal in [false, true] {
                let identity_client = if journal {
                    client.to_string()
                } else {
                    data_identity(&MutationIdentity::new(client, through), partition).client
                };
                let service = self.service(&route.owner, partition, journal)?;
                service.retire_outcomes(&identity_client, self.generation, through)?;
                let stamp = service.replica_stamp()?;
                if journal {
                    self.assignments[partition as usize].required_journal = stamp;
                } else {
                    self.assignments[partition as usize].required_data = stamp;
                }
                if !self.replicate(partition, journal, ServiceLinkFaults::default())? {
                    return Err(ServiceClusterError::ReplicaNotProved);
                }
            }
        }
        Ok(())
    }

    fn metadata(
        &self,
        entry: &str,
        command: ServiceCommand,
    ) -> Result<ServiceClusterExecution, ServiceClusterError> {
        if command.operation == "query_operation_outcome" {
            let id = command
                .identity
                .as_ref()
                .ok_or(MapServiceError::InvalidArgument)?;
            let outcome = self.query_operation_outcome(id)?;
            let result = match outcome {
                OperationOutcomeQuery::Unknown => ServiceResult::new("unknown"),
                OperationOutcomeQuery::Pending => ServiceResult::new("pending"),
                OperationOutcomeQuery::Completed(result) => {
                    let mut result = *result;
                    result.kind = "completed".into();
                    result
                }
            };
            return Ok(execution(result, false, false, Vec::new(), None));
        }
        let service = self.service(entry, 0, false)?;
        let mut command = command;
        command.now = service.logical_time();
        let capability = command.operation == "capability_descriptor";
        let mut result = service.execute(command)?;
        if capability {
            result.number = self
                .operation_inventory()
                .iter()
                .filter(|op| op.supported)
                .count() as u64;
            result.flag = false;
        }
        Ok(execution(result, false, false, Vec::new(), None))
    }

    fn groups(
        &self,
        command: &ServiceCommand,
        scope: ServiceOperationScope,
    ) -> Result<Vec<(u32, ServiceCommand, Vec<usize>)>, ServiceClusterError> {
        if scope != ServiceOperationScope::BoundedBulkFanout {
            return Ok(vec![(
                if scope == ServiceOperationScope::SinglePartitionMapWide {
                    0
                } else {
                    self.partition_for(&command.key)
                },
                command.clone(),
                Vec::new(),
            )]);
        }
        let use_entries = matches!(
            command.operation.as_str(),
            "put_all_detailed" | "replace_all_detailed"
        );
        let count = if use_entries {
            command.entries.len()
        } else {
            command.keys.len()
        };
        if count == 0 || count > self.bounds.max_bulk_items {
            return Err(MapServiceError::InvalidArgument.into());
        }
        let mut groups: BTreeMap<u32, (ServiceCommand, Vec<usize>)> = BTreeMap::new();
        let mut seen = BTreeSet::new();
        for index in 0..count {
            let key = if use_entries {
                &command.entries[index].key
            } else {
                &command.keys[index]
            };
            if !seen.insert(key) {
                return Err(MapServiceError::InvalidArgument.into());
            }
            let p = self.partition_for(key);
            let entry = groups.entry(p).or_insert_with(|| {
                let mut group = ServiceCommand::point(&command.operation, b"");
                group.ttl = command.ttl;
                group.now = command.now;
                group.deadline = command.deadline;
                group.cancelled = command.cancelled;
                (group, Vec::new())
            });
            if use_entries {
                entry.0.entries.push(command.entries[index].clone());
            } else {
                entry.0.keys.push(key.clone());
            }
            entry.1.push(index);
        }
        Ok(groups
            .into_iter()
            .map(|(p, (command, indices))| (p, command, indices))
            .collect())
    }

    fn read_groups(
        &mut self,
        entry: &str,
        command: ServiceCommand,
        groups: Vec<(u32, ServiceCommand, Vec<usize>)>,
        call: &ServiceClusterCall,
    ) -> Result<ServiceClusterExecution, ServiceClusterError> {
        let bulk = command.operation == "get_all";
        let mut combined = ServiceResult::new("bulk");
        let mut routes = Vec::new();
        let mut proved = true;
        for (p, group, indices) in groups {
            let route = self.route(entry, p, call)?;
            let faults = call
                .faults
                .partitions
                .get(&p)
                .copied()
                .unwrap_or(call.faults.data);
            if self.transmit(
                p,
                "read_request",
                group.test_transport_payload(self.bounds.max_frame_bytes)?,
                faults.request,
            )? == 0
            {
                return Err(ServiceClusterError::TransportTimeout);
            }
            let owner = self.service(&route.owner, p, false)?;
            let mut result = owner.execute(group)?;
            self.assignments[p as usize].required_data = owner.replica_stamp()?;
            proved &= self.replicate(p, false, faults).unwrap_or(false);
            if bulk {
                for item in &mut result.items {
                    item.index = indices[item.index];
                }
                combined.items.extend(result.items);
            } else {
                combined = result;
            }
            routes.push(route);
        }
        if bulk {
            combined.items.sort_by_key(|item| item.index);
            combined.number = combined.items.len() as u64;
        }
        Ok(execution(combined, false, proved, routes, None))
    }

    fn finish(
        &mut self,
        mut completed: ServiceClusterExecution,
        call: &ServiceClusterCall,
        lose_response: bool,
    ) -> Result<ServiceClusterExecution, ServiceClusterError> {
        if !completed.backup_proved {
            completed.result = ServiceResult::new("outcome_unknown");
            return Ok(completed);
        }
        let p = completed.routes.first().map_or(0, |r| r.partition);
        let delivered = self.transmit(
            p,
            "response",
            completed
                .result
                .test_transport_payload(self.bounds.max_frame_bytes)?,
            call.faults.response,
        );
        let lost = lose_response || !matches!(delivered, Ok(n) if n > 0);
        if lost {
            completed.result = ServiceResult::new("outcome_unknown");
            completed.failure = Some(ServiceClusterError::TransportTimeout);
        }
        Ok(completed)
    }

    fn route(
        &self,
        entry: &str,
        p: u32,
        call: &ServiceClusterCall,
    ) -> Result<ServicePartitionAssignment, ServiceClusterError> {
        let route = self
            .assignments
            .get(p as usize)
            .ok_or(ServiceClusterError::InvalidBound)?
            .route
            .clone();
        if call
            .expected_epoch
            .is_some_and(|epoch| epoch != route.epoch)
        {
            return Err(ServiceClusterError::StaleEpoch);
        }
        if entry != route.owner && call.proxy_hops >= 1 {
            return Err(ServiceClusterError::ProxyHopLimit);
        }
        self.require_live(&route.owner)?;
        Ok(route)
    }

    fn check_response_bytes(&self, bytes: usize) -> Result<(), ServiceClusterError> {
        if bytes > self.max_response_bytes {
            return Err(MapServiceError::Model(
                crate::value_plane_collection_075::CollectionError::BoundExceeded {
                    bound: "response_bytes",
                    limit: self.max_response_bytes,
                    actual: bytes,
                },
            )
            .into());
        }
        Ok(())
    }

    fn service(
        &self,
        node: &str,
        p: u32,
        journal: bool,
    ) -> Result<UnifiedIMapService, ServiceClusterError> {
        let node = self
            .nodes
            .get(node)
            .ok_or(ServiceClusterError::UnknownNode)?;
        let services = if journal { &node.journal } else { &node.data };
        services
            .get(p as usize)
            .cloned()
            .ok_or(ServiceClusterError::InvalidBound)
    }
    fn require_live(&self, node: &str) -> Result<(), ServiceClusterError> {
        if self
            .nodes
            .get(node)
            .ok_or(ServiceClusterError::UnknownNode)?
            .live
        {
            Ok(())
        } else {
            Err(ServiceClusterError::NodeUnavailable)
        }
    }

    fn replicate(
        &mut self,
        p: u32,
        journal: bool,
        faults: ServiceLinkFaults,
    ) -> Result<bool, ServiceClusterError> {
        let route = self.assignments[p as usize].route.clone();
        self.require_live(&route.owner)?;
        self.require_live(&route.backup)?;
        let source = self.service(&route.owner, p, journal)?;
        let image = source.export_replica_image()?;
        let stamp = image.stamp();
        if self.transmit(
            p,
            if journal {
                "journal_replica"
            } else {
                "data_replica"
            },
            image.transport_payload(self.bounds.max_frame_bytes)?,
            faults.replica,
        )? == 0
        {
            return Ok(false);
        }
        if self
            .service(&route.backup, p, journal)?
            .install_replica_image(image)?
            != stamp
        {
            return Err(ServiceClusterError::ReplicaNotProved);
        }
        if journal {
            self.assignments[p as usize].backup_journal = Some(stamp);
        } else {
            self.assignments[p as usize].backup_data = Some(stamp);
        }
        Ok(self.transmit(p, "replica_ack", stamp.digest.to_vec(), faults.ack)? > 0)
    }

    fn transmit(
        &mut self,
        p: u32,
        stage: &'static str,
        payload: Vec<u8>,
        fault: LoopbackFault,
    ) -> Result<usize, ServiceClusterError> {
        let generation = self.assignments[p as usize].route.epoch;
        let sequence = self.sequence;
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or(ServiceClusterError::InvalidBound)?;
        let expected: [u8; 32] = Sha256::digest(&payload).into();
        let mut transport = LoopbackTransport::new(self.bounds.max_frame_bytes, generation)?;
        transport.send(LoopbackFrame::new(generation, sequence, payload), fault)?;
        let mut delivered = 0;
        let mut ticks = 0;
        loop {
            match transport.receive() {
                Ok(Some(frame)) => {
                    if frame.generation != generation
                        || frame.sequence != sequence
                        || <[u8; 32]>::from(Sha256::digest(&frame.payload)) != expected
                    {
                        return Err(ServiceClusterError::ReplicaNotProved);
                    }
                    delivered += 1;
                    if delivered > 2 {
                        return Err(ServiceClusterError::InvalidBound);
                    }
                }
                Ok(None) if delivered > 0 || ticks >= self.bounds.max_transport_ticks => break,
                Ok(None) => {
                    transport.advance();
                    ticks += 1;
                }
                Err(error) => {
                    self.record_trace(ServiceClusterTrace {
                        stage,
                        partition: p,
                        delivered,
                    });
                    return Err(error.into());
                }
            }
        }
        self.record_trace(ServiceClusterTrace {
            stage,
            partition: p,
            delivered,
        });
        Ok(delivered)
    }
    fn record_trace(&mut self, event: ServiceClusterTrace) {
        if self.trace.len() == self.bounds.max_trace_events {
            self.trace.pop_front();
            self.dropped_trace_events = self.dropped_trace_events.saturating_add(1);
        }
        self.trace.push_back(event);
    }
}

fn partition(bytes: &[u8], count: u32) -> u32 {
    let digest = Sha256::digest(bytes);
    u64::from_be_bytes(digest[..8].try_into().expect("eight digest bytes")) as u32 % count
}
fn data_identity(identity: &MutationIdentity, p: u32) -> MutationIdentity {
    let digest = Sha256::digest(format!("test-service-data:{}:{p}", identity.client).as_bytes());
    let mut client = String::from("cluster:");
    for byte in digest {
        use std::fmt::Write;
        write!(&mut client, "{byte:02x}").expect("string writer");
    }
    MutationIdentity::new(client, identity.sequence)
}
fn request_digest(command: &ServiceCommand) -> [u8; 32] {
    command.test_request_digest()
}
fn service_result_bytes(result: &ServiceResult) -> usize {
    result.items.iter().fold(
        64usize.saturating_add(result.value.as_ref().map_or(0, Vec::len)),
        |sum, item| {
            sum.saturating_add(64)
                .saturating_add(item.key.len())
                .saturating_add(item.value.as_ref().map_or(0, Vec::len))
        },
    )
}
fn service_encoded_result_bytes(result: &ServiceResult) -> usize {
    result
        .items
        .iter()
        .fold(result.value.as_ref().map_or(0, Vec::len), |sum, item| {
            sum.saturating_add(17)
                .saturating_add(item.key.len())
                .saturating_add(item.value.as_ref().map_or(0, Vec::len))
        })
}
fn execution(
    result: ServiceResult,
    replayed: bool,
    backup_proved: bool,
    routes: Vec<ServicePartitionAssignment>,
    failure: Option<ServiceClusterError>,
) -> ServiceClusterExecution {
    ServiceClusterExecution {
        result,
        replayed,
        backup_proved,
        routes,
        failure,
    }
}
