//! Bounded development schedules over actual service owners. This fixture format is
//! not an owner/replica protocol. Stages retry the SAME identity with transport gates;
//! the actual service owns every decision and receipt, including partial bulk work.
use crate::value_plane_collection_075::{CollectionBounds, ReplacementEntry};
use crate::value_plane_history_075::{
    HistoryCall, HistoryOperation, HistoryOutcome, HistoryResult, ValuePlaneHistory,
    ValuePlaneHistoryOracle,
};
use crate::value_plane_loopback_075::LoopbackFault;
use crate::value_plane_model_075::{CanonicalMapKey, MutationIdentity, OutcomeCertainty};
use crate::value_plane_service_075::{
    OperationOutcomeQuery, ServiceCommand, ServiceLimits, ServiceResult, ServiceTtl,
};
use crate::value_plane_service_cluster_075::{
    ServiceClusterBounds, ServiceClusterCall, ServiceClusterExecution, ServiceClusterHarness,
    ServiceLinkFaults,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterPhase {
    Apply,
    Replicate,
    Acknowledge,
    Respond,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterChannel {
    Request,
    Replica,
    Ack,
    Response,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterFault {
    Deliver,
    Drop,
    Duplicate,
    Delay,
    Corrupt,
    Partial,
}
impl ClusterFault {
    fn transport(self) -> LoopbackFault {
        match self {
            Self::Deliver => LoopbackFault::Deliver,
            Self::Drop => LoopbackFault::Drop,
            Self::Duplicate => LoopbackFault::Duplicate,
            Self::Delay => LoopbackFault::Delay(1),
            Self::Corrupt => LoopbackFault::Corrupt,
            Self::Partial => LoopbackFault::Partial,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterScheduleStep {
    Invoke {
        id: u64,
        key: Vec<u8>,
        operation: HistoryOperation,
    },
    Advance {
        id: u64,
        phase: ClusterPhase,
    },
    Fault {
        id: u64,
        channel: ClusterChannel,
        fault: ClusterFault,
    },
    SetLive {
        node: u8,
        live: bool,
    },
    Promote {
        partition: u32,
    },
    Repair {
        partition: u32,
    },
    Rebalance {
        partition: u32,
    },
    BulkProbe {
        id: u64,
    },
    ExpiryProbe {
        id: u64,
    },
}
#[derive(Debug, Clone, Copy)]
pub struct ClusterScheduleBounds {
    pub max_steps: usize,
    pub max_calls: usize,
    pub max_pending: usize,
    pub max_payload_bytes: usize,
    pub max_search_states: usize,
}
impl Default for ClusterScheduleBounds {
    fn default() -> Self {
        Self {
            max_steps: 256,
            max_calls: 32,
            max_pending: 8,
            max_payload_bytes: 4096,
            max_search_states: 50_000,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterScheduleError {
    InvalidSchedule(&'static str),
    Input(String),
    Runtime(String),
    SearchBudget,
    Violation(String),
}
impl fmt::Display for ClusterScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ClusterScheduleError {}
fn invalid(reason: &'static str) -> ClusterScheduleError {
    ClusterScheduleError::InvalidSchedule(reason)
}
fn runtime(error: impl fmt::Display) -> ClusterScheduleError {
    ClusterScheduleError::Runtime(error.to_string())
}
#[derive(Debug, Clone)]
pub struct ClusterScheduleReport {
    pub history: ValuePlaneHistory,
    pub observations: Vec<String>,
    pub fingerprint: String,
    pub linearizable: bool,
    pub replica_checks: usize,
    pub receipt_checks: usize,
    pub bulk_checks: usize,
    pub expiry_checks: usize,
}
struct Invocation {
    key: Vec<u8>,
    operation: HistoryOperation,
    command: ServiceCommand,
    invoked: u64,
    next: ClusterPhase,
    call: ServiceClusterCall,
    held: Option<ServiceClusterExecution>,
}
struct Runner {
    cluster: ServiceClusterHarness,
    bounds: ClusterScheduleBounds,
    pending: BTreeMap<u64, Invocation>,
    history: ValuePlaneHistory,
    receipts: BTreeMap<u64, ServiceResult>,
    seen: Vec<u64>,
    probes: Vec<(bool, u64)>,
    observations: Vec<String>,
    tick: u64,
    now: u64,
    live: [bool; 3],
    replica_checks: usize,
    receipt_checks: usize,
    bulk_checks: usize,
    expiry_checks: usize,
}
const NODES: [&str; 3] = ["a", "b", "c"];
impl Runner {
    fn new(bounds: ClusterScheduleBounds) -> Result<Self, ClusterScheduleError> {
        if bounds.max_steps == 0
            || bounds.max_steps > 256
            || bounds.max_calls == 0
            || bounds.max_calls > 32
            || bounds.max_pending == 0
            || bounds.max_pending > 8
            || bounds.max_payload_bytes == 0
            || bounds.max_payload_bytes > 4096
            || bounds.max_search_states == 0
            || bounds.max_search_states > 100_000
        {
            return Err(invalid("bounds"));
        }
        let collection = CollectionBounds {
            max_key_bytes: 4096,
            max_value_bytes: 4096,
            ..Default::default()
        };
        let cluster = ServiceClusterHarness::new(
            &NODES,
            ServiceClusterBounds {
                max_frame_bytes: 1024 * 1024,
                ..Default::default()
            },
            collection,
            ServiceLimits::default(),
            1,
        )
        .map_err(runtime)?;
        Ok(Self {
            cluster,
            bounds,
            pending: BTreeMap::new(),
            history: ValuePlaneHistory::new(bounds.max_calls).map_err(runtime)?,
            receipts: BTreeMap::new(),
            seen: Vec::new(),
            probes: Vec::new(),
            observations: Vec::new(),
            tick: 0,
            now: 0,
            live: [true; 3],
            replica_checks: 0,
            receipt_checks: 0,
            bulk_checks: 0,
            expiry_checks: 0,
        })
    }
    fn entry(&self) -> Result<&'static str, ClusterScheduleError> {
        self.live
            .iter()
            .position(|live| *live)
            .map(|i| NODES[i])
            .ok_or_else(|| invalid("no live ingress"))
    }
    fn step(&mut self, step: &ClusterScheduleStep) -> Result<(), ClusterScheduleError> {
        self.tick += 1;
        let observation = match step {
            ClusterScheduleStep::Invoke { id, key, operation } => {
                if *id == 0
                    || key.starts_with(b"__")
                    || self.seen.contains(id)
                    || self.seen.len() >= self.bounds.max_calls
                    || self.pending.len() >= self.bounds.max_pending
                {
                    return Err(invalid("invocation identity/capacity"));
                }
                let payload =
                    operation_payload(key, operation).ok_or_else(|| invalid("payload"))?;
                if payload > self.bounds.max_payload_bytes {
                    return Err(invalid("payload"));
                }
                let command = command(*id, key, operation, self.now)?;
                self.seen.push(*id);
                self.pending.insert(
                    *id,
                    Invocation {
                        key: key.clone(),
                        operation: operation.clone(),
                        command,
                        invoked: self.tick,
                        next: ClusterPhase::Apply,
                        call: Default::default(),
                        held: None,
                    },
                );
                "invoked".into()
            }
            ClusterScheduleStep::Fault { id, channel, fault } => {
                let invocation = self
                    .pending
                    .get_mut(id)
                    .ok_or_else(|| invalid("unknown invocation"))?;
                let fault = fault.transport();
                match channel {
                    ClusterChannel::Request => invocation.call.faults.data.request = fault,
                    ClusterChannel::Replica => invocation.call.faults.data.replica = fault,
                    ClusterChannel::Ack => invocation.call.faults.data.ack = fault,
                    ClusterChannel::Response => invocation.call.faults.response = fault,
                };
                "fault_queued".into()
            }
            ClusterScheduleStep::Advance { id, phase } => self.advance(*id, *phase)?,
            ClusterScheduleStep::SetLive { node, live } => {
                let index = usize::from(*node);
                if index >= 3 {
                    return Err(invalid("node"));
                }
                self.cluster
                    .set_node_live(NODES[index], *live)
                    .map_err(runtime)?;
                self.live[index] = *live;
                "reachability_changed".into()
            }
            ClusterScheduleStep::Promote { partition } => {
                self.partition(*partition)?;
                match self.cluster.promote_backup(*partition) {
                    Ok(_) => "promoted".into(),
                    Err(error) => format!("promotion_refused:{error}"),
                }
            }
            ClusterScheduleStep::Repair { partition } => {
                let route = self.partition(*partition)?;
                match self.cluster.repair_backup(
                    *partition,
                    &route.backup,
                    ServiceLinkFaults::default(),
                ) {
                    Ok(()) => "repaired".into(),
                    Err(error) => format!("repair_refused:{error}"),
                }
            }
            ClusterScheduleStep::Rebalance { partition } => {
                let route = self.partition(*partition)?;
                let owner = NODES
                    .iter()
                    .find(|node| **node != route.owner && **node != route.backup)
                    .unwrap();
                match self
                    .cluster
                    .rebalance(*partition, owner, Default::default())
                {
                    Ok(_) => "rebalanced".into(),
                    Err(error) => format!("rebalance_refused:{error}"),
                }
            }
            ClusterScheduleStep::BulkProbe { id } => {
                if *id == 0 || self.probes.contains(&(false, *id)) {
                    return Err(invalid("probe identity"));
                }
                self.probes.push((false, *id));
                self.bulk_probe(*id)?;
                "bulk_verified".into()
            }
            ClusterScheduleStep::ExpiryProbe { id } => {
                if *id == 0 || self.probes.contains(&(true, *id)) {
                    return Err(invalid("probe identity"));
                }
                self.probes.push((true, *id));
                self.expiry_probe(*id)?;
                "expiry_verified".into()
            }
        };
        self.observations
            .push(format!("{}:{observation}", self.tick));
        self.check_cluster()?;
        Ok(())
    }
    fn partition(
        &self,
        p: u32,
    ) -> Result<
        crate::value_plane_service_cluster_075::ServicePartitionAssignment,
        ClusterScheduleError,
    > {
        self.cluster
            .assignment(p)
            .ok_or_else(|| invalid("partition"))
    }
    fn advance(&mut self, id: u64, phase: ClusterPhase) -> Result<String, ClusterScheduleError> {
        let entry = self.entry()?;
        let invocation = self
            .pending
            .get_mut(&id)
            .ok_or_else(|| invalid("unknown invocation"))?;
        if invocation.next != phase {
            return Err(invalid("phase order"));
        }
        let read = matches!(invocation.operation, HistoryOperation::Read);
        if phase == ClusterPhase::Respond {
            let held = invocation
                .held
                .as_ref()
                .ok_or_else(|| invalid("response without decision"))?;
            if !read && !held.backup_proved {
                return Err(ClusterScheduleError::Violation(
                    "response_before_replica_proof".into(),
                ));
            }
            let call = self.pending.remove(&id).unwrap();
            let held = call.held.unwrap();
            let result = history_result(&call.operation, &held.result)?;
            self.history
                .push(HistoryCall::completed(
                    id,
                    CanonicalMapKey::new(
                        "test",
                        "schedule",
                        1,
                        call.key.clone(),
                        self.cluster.partition_for(&call.key),
                    ),
                    call.operation,
                    call.invoked,
                    self.tick,
                    result,
                    OutcomeCertainty::Certain,
                ))
                .map_err(runtime)?;
            if !read {
                self.receipts.insert(id, held.result);
            }
            return Ok("responded".into());
        }
        let mut call = invocation.call.clone();
        if !read {
            match phase {
                ClusterPhase::Apply => {
                    call.faults.data.replica = LoopbackFault::Drop;
                    call.faults.coordinator.replica = LoopbackFault::Drop;
                }
                ClusterPhase::Replicate => {
                    call.faults.data.ack = LoopbackFault::Drop;
                    call.faults.coordinator.replica = LoopbackFault::Drop;
                }
                _ => {}
            }
        }
        match self
            .cluster
            .execute(entry, invocation.command.clone(), call)
        {
            Err(error) => Ok(format!("blocked:{error}")),
            Ok(execution) => {
                if read {
                    invocation.next = ClusterPhase::Respond;
                } else {
                    match phase {
                        ClusterPhase::Apply if execution.result.kind != "not_applied" => {
                            invocation.next = ClusterPhase::Replicate
                        }
                        ClusterPhase::Replicate => invocation.next = ClusterPhase::Acknowledge,
                        ClusterPhase::Acknowledge
                            if execution.backup_proved
                                && execution.result.kind != "outcome_unknown" =>
                        {
                            invocation.next = ClusterPhase::Respond
                        }
                        _ => {}
                    }
                }
                let kind = execution.result.kind.clone();
                invocation.held = Some(execution);
                Ok(format!("{phase:?}:{kind}"))
            }
        }
    }
    fn check_cluster(&mut self) -> Result<(), ClusterScheduleError> {
        for p in 0..3 {
            if self.cluster.backup_proved(p) {
                let route = self.partition(p)?;
                let owner = self
                    .cluster
                    .diagnostic_node_snapshot(&route.owner, p, self.now);
                let backup = self
                    .cluster
                    .diagnostic_node_snapshot(&route.backup, p, self.now);
                if let (Ok(owner), Ok(backup)) = (owner, backup) {
                    if owner != backup {
                        return Err(ClusterScheduleError::Violation(
                            "proved_replica_diverged".into(),
                        ));
                    }
                    self.replica_checks += 1;
                }
            }
        }
        for (id, result) in &self.receipts {
            match self
                .cluster
                .query_operation_outcome(&MutationIdentity::new("schedule", *id))
            {
                Ok(OperationOutcomeQuery::Completed(actual)) if actual.as_ref() == result => {
                    self.receipt_checks += 1
                }
                Err(_) => {}
                _ => {
                    return Err(ClusterScheduleError::Violation(
                        "acknowledged_receipt_lost".into(),
                    ))
                }
            }
        }
        Ok(())
    }
    fn bulk_probe(&mut self, id: u64) -> Result<(), ClusterScheduleError> {
        if !self.pending.is_empty() {
            return Err(invalid("probe requires quiescent invocation history"));
        }
        let entry = self.entry()?;
        let keys = vec![
            b"__bulk0".to_vec(),
            b"__bulk1".to_vec(),
            b"__bulk2".to_vec(),
        ];
        let mut command = ServiceCommand::point("put_all_detailed", b"");
        command.now = self.now;
        command.identity = Some(MutationIdentity::new("bulk-schedule", id));
        command.entries = keys
            .iter()
            .map(|key| ReplacementEntry {
                key: key.clone(),
                value: vec![id as u8],
            })
            .collect();
        let mut fault = ServiceClusterCall::default();
        fault.faults.data.ack = LoopbackFault::Drop;
        let first = self
            .cluster
            .execute(entry, command.clone(), fault)
            .map_err(runtime)?;
        if first.result.kind != "outcome_unknown" {
            return Err(ClusterScheduleError::Violation(
                "partial_bulk_certainty".into(),
            ));
        }
        let mut versions = BTreeMap::new();
        for key in &keys {
            let p = self.cluster.partition_for(key);
            let route = self.partition(p)?;
            let entries = self
                .cluster
                .diagnostic_node_snapshot(&route.owner, p, self.now)
                .map_err(runtime)?;
            let item = entries
                .into_iter()
                .find(|item| &item.key == key)
                .ok_or_else(|| ClusterScheduleError::Violation("bulk_item_missing".into()))?;
            versions.insert(key.clone(), item.version);
        }
        let second = self
            .cluster
            .execute(entry, command, Default::default())
            .map_err(runtime)?;
        if !second.backup_proved || second.result.items.len() != keys.len() {
            return Err(ClusterScheduleError::Violation(
                "bulk_receipt_incomplete".into(),
            ));
        }
        for (i, item) in second.result.items.iter().enumerate() {
            if item.index != i || item.key != keys[i] || item.version != Some(versions[&item.key]) {
                return Err(ClusterScheduleError::Violation(
                    "bulk_order_or_duplicate_apply".into(),
                ));
            }
        }
        self.bulk_checks += 1;
        Ok(())
    }
    fn expiry_probe(&mut self, id: u64) -> Result<(), ClusterScheduleError> {
        if !self.pending.is_empty() {
            return Err(invalid("probe requires quiescent invocation history"));
        }
        let entry = self.entry()?;
        let key = format!("__ttl{id}").into_bytes();
        let mut put = ServiceCommand::point("put", &key);
        put.now = self.now;
        put.value = Some(vec![1]);
        put.ttl = ServiceTtl::ExpireAfter(1);
        put.identity = Some(MutationIdentity::new("expiry-schedule", id));
        let write = self
            .cluster
            .execute(entry, put, Default::default())
            .map_err(runtime)?;
        if !write.backup_proved {
            return Err(ClusterScheduleError::Violation(
                "expiry_write_unproved".into(),
            ));
        }
        self.now = self.now.checked_add(1).ok_or_else(|| invalid("clock"))?;
        let mut get = ServiceCommand::point("get", &key);
        get.now = self.now;
        let read = self
            .cluster
            .execute(entry, get.clone(), Default::default())
            .map_err(runtime)?;
        if read.result.kind != "absent" || !read.backup_proved {
            return Err(ClusterScheduleError::Violation(
                "expiry_not_replicated".into(),
            ));
        }
        let p = self.cluster.partition_for(&key);
        self.cluster.promote_backup(p).map_err(runtime)?;
        let route = self.partition(p)?;
        self.cluster
            .repair_backup(p, &route.backup, Default::default())
            .map_err(runtime)?;
        if self
            .cluster
            .execute(entry, get, Default::default())
            .map_err(runtime)?
            .result
            .kind
            != "absent"
        {
            return Err(ClusterScheduleError::Violation(
                "expired_value_resurrected".into(),
            ));
        }
        self.expiry_checks += 1;
        Ok(())
    }
}
fn command(
    id: u64,
    key: &[u8],
    operation: &HistoryOperation,
    now: u64,
) -> Result<ServiceCommand, ClusterScheduleError> {
    let name = match operation {
        HistoryOperation::Read => "get",
        HistoryOperation::Put(_) | HistoryOperation::GetAndPut(_) => "get_and_put",
        HistoryOperation::PutIfAbsent(_) => "put_if_absent",
        HistoryOperation::ReplaceIfPresent(_) => "replace_if_present",
        HistoryOperation::ReplaceIfValue { .. } => "conditional_replace",
        HistoryOperation::RemoveIfValue { .. } => "conditional_remove",
        HistoryOperation::GetAndRemove | HistoryOperation::Delete => "get_and_remove",
    };
    let mut command = ServiceCommand::point(name, key);
    command.now = now;
    if !matches!(operation, HistoryOperation::Read) {
        command.identity = Some(MutationIdentity::new("schedule", id));
    }
    match operation {
        HistoryOperation::Put(value)
        | HistoryOperation::GetAndPut(value)
        | HistoryOperation::PutIfAbsent(value)
        | HistoryOperation::ReplaceIfPresent(value) => command.value = Some(value.clone()),
        HistoryOperation::ReplaceIfValue { expected, value } => {
            command.expected = Some(expected.clone());
            command.value = Some(value.clone());
        }
        HistoryOperation::RemoveIfValue { expected } => command.expected = Some(expected.clone()),
        _ => {}
    }
    Ok(command)
}
fn history_result(
    operation: &HistoryOperation,
    result: &ServiceResult,
) -> Result<HistoryResult, ClusterScheduleError> {
    if matches!(operation, HistoryOperation::Read) {
        return Ok(HistoryResult::Value(result.value.clone()));
    }
    if matches!(result.kind.as_str(), "outcome_unknown" | "not_applied") {
        return Err(ClusterScheduleError::Violation(
            "unproved_result_responded".into(),
        ));
    }
    let applied = matches!(result.kind.as_str(), "inserted" | "replaced" | "removed");
    let previous = result.value.clone();
    let current = if !applied {
        previous.clone()
    } else {
        match operation {
            HistoryOperation::Put(value)
            | HistoryOperation::GetAndPut(value)
            | HistoryOperation::PutIfAbsent(value)
            | HistoryOperation::ReplaceIfPresent(value)
            | HistoryOperation::ReplaceIfValue { value, .. } => Some(value.clone()),
            _ => None,
        }
    };
    Ok(HistoryResult::Mutation(HistoryOutcome {
        applied,
        previous,
        current,
    }))
}
pub fn replay_cluster_schedule(
    steps: &[ClusterScheduleStep],
    bounds: ClusterScheduleBounds,
) -> Result<ClusterScheduleReport, ClusterScheduleError> {
    if steps.len() > bounds.max_steps {
        return Err(invalid("steps"));
    }
    let mut runner = Runner::new(bounds)?;
    for step in steps {
        runner.step(step)?;
    }
    for (id, pending) in &runner.pending {
        if !matches!(pending.operation, HistoryOperation::Read) {
            runner
                .history
                .push(HistoryCall::completed(
                    *id,
                    CanonicalMapKey::new(
                        "test",
                        "schedule",
                        1,
                        pending.key.clone(),
                        runner.cluster.partition_for(&pending.key),
                    ),
                    pending.operation.clone(),
                    pending.invoked,
                    runner.tick + 1,
                    HistoryResult::Unknown,
                    OutcomeCertainty::OutcomeUnknown,
                ))
                .map_err(runtime)?;
        }
    }
    verify_cluster_history(&runner.history, bounds.max_search_states)?;
    let mut digest = Sha256::new();
    digest.update(encode_cluster_schedule(steps)?);
    for observation in &runner.observations {
        digest.update(observation.as_bytes());
    }
    Ok(ClusterScheduleReport {
        history: runner.history,
        observations: runner.observations,
        fingerprint: hex(&digest.finalize()),
        linearizable: true,
        replica_checks: runner.replica_checks,
        receipt_checks: runner.receipt_checks,
        bulk_checks: runner.bulk_checks,
        expiry_checks: runner.expiry_checks,
    })
}
pub fn generated_cluster_schedule(
    seed: u64,
    rounds: usize,
) -> Result<Vec<ClusterScheduleStep>, ClusterScheduleError> {
    if rounds == 0 || rounds > 6 {
        return Err(invalid("rounds"));
    }
    let mut random = seed;
    let mut steps = Vec::new();
    for round in 0..rounds {
        random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
        let first = (round * 3 + 1) as u64;
        let key = format!("key{}", random % 3).into_bytes();
        for id in [first, first + 1] {
            steps.push(ClusterScheduleStep::Invoke {
                id,
                key: key.clone(),
                operation: if id == first {
                    HistoryOperation::GetAndPut(vec![random as u8])
                } else {
                    HistoryOperation::PutIfAbsent(vec![random.wrapping_add(1) as u8])
                },
            });
        }
        steps.push(ClusterScheduleStep::Fault {
            id: first,
            channel: ClusterChannel::Request,
            fault: ClusterFault::Duplicate,
        });
        for id in [first, first + 1] {
            steps.push(ClusterScheduleStep::Advance {
                id,
                phase: ClusterPhase::Apply,
            });
        }
        steps.push(ClusterScheduleStep::Advance {
            id: first,
            phase: ClusterPhase::Replicate,
        });
        steps.push(ClusterScheduleStep::Fault {
            id: first,
            channel: ClusterChannel::Ack,
            fault: ClusterFault::Drop,
        });
        steps.push(ClusterScheduleStep::Advance {
            id: first,
            phase: ClusterPhase::Acknowledge,
        });
        steps.push(ClusterScheduleStep::Fault {
            id: first,
            channel: ClusterChannel::Ack,
            fault: ClusterFault::Deliver,
        });
        steps.push(ClusterScheduleStep::Advance {
            id: first,
            phase: ClusterPhase::Acknowledge,
        });
        for phase in [
            ClusterPhase::Apply,
            ClusterPhase::Replicate,
            ClusterPhase::Acknowledge,
        ] {
            steps.push(ClusterScheduleStep::Advance {
                id: first + 1,
                phase,
            });
        }
        for id in [first + 1, first] {
            steps.push(ClusterScheduleStep::Advance {
                id,
                phase: ClusterPhase::Respond,
            });
        }
        let id = first + 2;
        steps.extend([
            ClusterScheduleStep::Invoke {
                id,
                key,
                operation: HistoryOperation::Read,
            },
            ClusterScheduleStep::Advance {
                id,
                phase: ClusterPhase::Apply,
            },
            ClusterScheduleStep::Advance {
                id,
                phase: ClusterPhase::Respond,
            },
            ClusterScheduleStep::BulkProbe { id: first },
            ClusterScheduleStep::ExpiryProbe { id: first },
            ClusterScheduleStep::Rebalance {
                partition: round as u32 % 3,
            },
            ClusterScheduleStep::Repair {
                partition: round as u32 % 3,
            },
        ]);
    }
    Ok(steps)
}
#[derive(Debug)]
pub struct ClusterShrinkReport {
    pub steps: Vec<ClusterScheduleStep>,
    pub replays: usize,
    pub budget_exhausted: bool,
}
pub fn shrink_cluster_schedule(
    steps: &[ClusterScheduleStep],
    budget: usize,
    mut still_fails: impl FnMut(&[ClusterScheduleStep]) -> bool,
) -> Result<ClusterShrinkReport, ClusterScheduleError> {
    if steps.len() > 256 || budget == 0 || budget > 1024 || !still_fails(steps) {
        return Err(invalid("counterexample/budget"));
    }
    let mut current = steps.to_vec();
    let mut replays = 1;
    let mut index = 0;
    while index < current.len() && replays < budget {
        let mut candidate = current.clone();
        candidate.remove(index);
        replays += 1;
        if still_fails(&candidate) {
            current = candidate;
            index = 0;
        } else {
            index += 1;
        }
    }
    Ok(ClusterShrinkReport {
        steps: current,
        replays,
        budget_exhausted: replays == budget,
    })
}

// Strict bounded development JSON codec. Text operation names allocate no wire IDs.
pub fn encode_cluster_schedule(
    steps: &[ClusterScheduleStep],
) -> Result<Vec<u8>, ClusterScheduleError> {
    if steps.len() > 256 {
        return Err(invalid("steps"));
    }
    let mut maximum = 32usize;
    for step in steps {
        let bytes = if let ClusterScheduleStep::Invoke { key, operation, .. } = step {
            let bytes = operation_payload(key, operation).ok_or_else(|| invalid("payload"))?;
            if bytes > 4096 {
                return Err(invalid("payload"));
            }
            bytes
        } else {
            0
        };
        maximum = maximum
            .checked_add(200 + 2 * bytes)
            .ok_or_else(|| invalid("encoded bytes"))?;
        if maximum > 65536 {
            return Err(invalid("encoded bytes"));
        }
    }
    let steps = steps.iter().map(encode_step).collect::<Vec<_>>();
    let bytes = serde_json::to_vec(&json!({"version":1,"steps":steps}))
        .map_err(|error| ClusterScheduleError::Input(error.to_string()))?;
    if bytes.len() > 65536 {
        return Err(invalid("encoded bytes"));
    }
    Ok(bytes)
}

fn encode_step(step: &ClusterScheduleStep) -> Value {
    match step {
        ClusterScheduleStep::Invoke { id, key, operation } => {
            let mut encoded = json!({"kind":"invoke","id":id,"key":hex(key)});
            let object = encoded.as_object_mut().unwrap();
            let (name, value, expected) = match operation {
                HistoryOperation::Read => ("read", None, None),
                HistoryOperation::Put(value) => ("put", Some(value.as_slice()), None),
                HistoryOperation::GetAndPut(value) => ("get_and_put", Some(value.as_slice()), None),
                HistoryOperation::PutIfAbsent(value) => {
                    ("put_if_absent", Some(value.as_slice()), None)
                }
                HistoryOperation::ReplaceIfPresent(value) => {
                    ("replace_if_present", Some(value.as_slice()), None)
                }
                HistoryOperation::ReplaceIfValue { expected, value } => (
                    "replace_if_value",
                    Some(value.as_slice()),
                    Some(expected.as_slice()),
                ),
                HistoryOperation::RemoveIfValue { expected } => {
                    ("remove_if_value", None, Some(expected.as_slice()))
                }
                HistoryOperation::GetAndRemove => ("get_and_remove", None, None),
                HistoryOperation::Delete => ("delete", None, None),
            };
            object.insert("operation".into(), json!(name));
            if let Some(value) = value {
                object.insert("value".into(), json!(hex(value)));
            }
            if let Some(expected) = expected {
                object.insert("expected".into(), json!(hex(expected)));
            }
            encoded
        }
        ClusterScheduleStep::Advance { id, phase } => {
            json!({"kind":"advance","id":id,"phase":format!("{phase:?}")})
        }
        ClusterScheduleStep::Fault { id, channel, fault } => {
            json!({"kind":"fault","id":id,"channel":format!("{channel:?}"),"fault":format!("{fault:?}")})
        }
        ClusterScheduleStep::SetLive { node, live } => {
            json!({"kind":"live","node":node,"live":live})
        }
        ClusterScheduleStep::Promote { partition } => {
            json!({"kind":"promote","partition":partition})
        }
        ClusterScheduleStep::Repair { partition } => json!({"kind":"repair","partition":partition}),
        ClusterScheduleStep::Rebalance { partition } => {
            json!({"kind":"rebalance","partition":partition})
        }
        ClusterScheduleStep::BulkProbe { id } => json!({"kind":"bulk","id":id}),
        ClusterScheduleStep::ExpiryProbe { id } => json!({"kind":"expiry","id":id}),
    }
}
pub fn decode_cluster_schedule(
    bytes: &[u8],
) -> Result<Vec<ClusterScheduleStep>, ClusterScheduleError> {
    if bytes.len() > 65536 {
        return Err(invalid("input bytes"));
    }
    let root: Value = serde_json::from_slice(bytes)
        .map_err(|error| ClusterScheduleError::Input(error.to_string()))?;
    fields(&root, &["version", "steps"])?;
    if root["version"] != 1 {
        return Err(invalid("fixture version"));
    }
    let steps = root["steps"].as_array().ok_or_else(|| invalid("steps"))?;
    if steps.len() > 256 {
        return Err(invalid("steps"));
    }
    steps
        .iter()
        .map(|step| {
            let kind = text(step, "kind")?;
            let id = || number(step, "id");
            let partition = || {
                number(step, "partition")
                    .and_then(|n| u32::try_from(n).map_err(|_| invalid("partition")))
            };
            Ok(match kind {
                "invoke" => {
                    fields(
                        step,
                        &["kind", "id", "key", "operation", "value", "expected"],
                    )?;
                    let value = || decode_hex(text(step, "value")?);
                    let expected = || decode_hex(text(step, "expected")?);
                    let operation = match text(step, "operation")? {
                        "read" => HistoryOperation::Read,
                        "put" => HistoryOperation::Put(value()?),
                        "get_and_put" => HistoryOperation::GetAndPut(value()?),
                        "put_if_absent" => HistoryOperation::PutIfAbsent(value()?),
                        "replace_if_present" => HistoryOperation::ReplaceIfPresent(value()?),
                        "replace_if_value" => HistoryOperation::ReplaceIfValue {
                            value: value()?,
                            expected: expected()?,
                        },
                        "remove_if_value" => HistoryOperation::RemoveIfValue {
                            expected: expected()?,
                        },
                        "get_and_remove" => HistoryOperation::GetAndRemove,
                        "delete" => HistoryOperation::Delete,
                        _ => return Err(invalid("operation")),
                    };
                    ClusterScheduleStep::Invoke {
                        id: id()?,
                        key: decode_hex(text(step, "key")?)?,
                        operation,
                    }
                }
                "advance" => {
                    fields(step, &["kind", "id", "phase"])?;
                    ClusterScheduleStep::Advance {
                        id: id()?,
                        phase: match text(step, "phase")? {
                            "Apply" => ClusterPhase::Apply,
                            "Replicate" => ClusterPhase::Replicate,
                            "Acknowledge" => ClusterPhase::Acknowledge,
                            "Respond" => ClusterPhase::Respond,
                            _ => return Err(invalid("phase")),
                        },
                    }
                }
                "fault" => {
                    fields(step, &["kind", "id", "channel", "fault"])?;
                    ClusterScheduleStep::Fault {
                        id: id()?,
                        channel: match text(step, "channel")? {
                            "Request" => ClusterChannel::Request,
                            "Replica" => ClusterChannel::Replica,
                            "Ack" => ClusterChannel::Ack,
                            "Response" => ClusterChannel::Response,
                            _ => return Err(invalid("channel")),
                        },
                        fault: match text(step, "fault")? {
                            "Deliver" => ClusterFault::Deliver,
                            "Drop" => ClusterFault::Drop,
                            "Duplicate" => ClusterFault::Duplicate,
                            "Delay" => ClusterFault::Delay,
                            "Corrupt" => ClusterFault::Corrupt,
                            "Partial" => ClusterFault::Partial,
                            _ => return Err(invalid("fault")),
                        },
                    }
                }
                "live" => {
                    fields(step, &["kind", "node", "live"])?;
                    ClusterScheduleStep::SetLive {
                        node: u8::try_from(number(step, "node")?).map_err(|_| invalid("node"))?,
                        live: step["live"].as_bool().ok_or_else(|| invalid("live"))?,
                    }
                }
                "promote" | "repair" | "rebalance" => {
                    fields(step, &["kind", "partition"])?;
                    match kind {
                        "promote" => ClusterScheduleStep::Promote {
                            partition: partition()?,
                        },
                        "repair" => ClusterScheduleStep::Repair {
                            partition: partition()?,
                        },
                        _ => ClusterScheduleStep::Rebalance {
                            partition: partition()?,
                        },
                    }
                }
                "bulk" | "expiry" => {
                    fields(step, &["kind", "id"])?;
                    if kind == "bulk" {
                        ClusterScheduleStep::BulkProbe { id: id()? }
                    } else {
                        ClusterScheduleStep::ExpiryProbe { id: id()? }
                    }
                }
                _ => return Err(invalid("step")),
            })
        })
        .collect()
}
pub fn fuzz_cluster_schedule(bytes: &[u8]) -> Result<ClusterScheduleReport, ClusterScheduleError> {
    replay_cluster_schedule(&decode_cluster_schedule(bytes)?, Default::default())
}
pub fn verify_cluster_history(
    history: &ValuePlaneHistory,
    budget: usize,
) -> Result<(), ClusterScheduleError> {
    if budget == 0 || budget > 100_000 {
        return Err(invalid("search budget"));
    }
    let checked = ValuePlaneHistoryOracle::new(budget).check(history);
    if checked.is_linearizable() {
        Ok(())
    } else if checked.budget_exhausted {
        Err(ClusterScheduleError::SearchBudget)
    } else {
        Err(ClusterScheduleError::Violation(
            checked.violation.unwrap_or_default(),
        ))
    }
}
/// Negative control: an acknowledged write followed by a fabricated absent read must
/// be rejected independently of the actual cluster that produced the original history.
pub fn history_canary_detected(history: &ValuePlaneHistory) -> bool {
    let mut calls = history.calls().to_vec();
    let Some(read) = calls
        .iter_mut()
        .rev()
        .find(|call| matches!(call.result, HistoryResult::Value(Some(_))))
    else {
        return false;
    };
    read.result = HistoryResult::Value(None);
    let Ok(mut changed) = ValuePlaneHistory::new(32) else {
        return false;
    };
    for call in calls {
        if changed.push(call).is_err() {
            return false;
        }
    }
    !ValuePlaneHistoryOracle::new(50_000)
        .check(&changed)
        .is_linearizable()
}
fn fields(value: &Value, allowed: &[&str]) -> Result<(), ClusterScheduleError> {
    let object = value.as_object().ok_or_else(|| invalid("object"))?;
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(invalid("unknown field"));
    }
    Ok(())
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, ClusterScheduleError> {
    value[key].as_str().ok_or_else(|| invalid("text"))
}
fn number(value: &Value, key: &str) -> Result<u64, ClusterScheduleError> {
    value[key].as_u64().ok_or_else(|| invalid("number"))
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn decode_hex(text: &str) -> Result<Vec<u8>, ClusterScheduleError> {
    if text.len() > 8192
        || !text.len().is_multiple_of(2)
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid("hex"));
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).map_err(|_| invalid("hex")))
        .collect()
}
fn operation_payload(key: &[u8], operation: &HistoryOperation) -> Option<usize> {
    let extra = match operation {
        HistoryOperation::Put(value)
        | HistoryOperation::GetAndPut(value)
        | HistoryOperation::PutIfAbsent(value)
        | HistoryOperation::ReplaceIfPresent(value) => value.len(),
        HistoryOperation::ReplaceIfValue { expected, value } => {
            expected.len().checked_add(value.len())?
        }
        HistoryOperation::RemoveIfValue { expected } => expected.len(),
        _ => 0,
    };
    key.len().checked_add(extra)
}
