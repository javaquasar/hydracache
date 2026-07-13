use std::collections::BTreeMap;

use hydracache::{LogicalDuration, LogicalTime};

use crate::{History, WorkloadConfig, WorkloadGenerator, WorkloadOp, WorkloadResult};

/// Public history type used by the linearizability oracle.
pub type LinearizabilityHistory = History;

/// Append-only helper for tests and future external drivers that build histories
/// without depending on the simulator world.
#[derive(Debug, Clone, Default)]
pub struct LinearizabilityHistoryRecorder {
    history: History,
}

impl LinearizabilityHistoryRecorder {
    /// Create an empty recorder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an invocation and return its event id.
    pub fn invoke(
        &mut self,
        client: u64,
        op: WorkloadOp,
        invoked_at: LogicalTime,
    ) -> crate::EventId {
        self.history.record_invocation(client, op, invoked_at)
    }

    /// Record a response for a previous invocation.
    pub fn respond(
        &mut self,
        id: crate::EventId,
        returned_at: LogicalTime,
        result: WorkloadResult,
    ) {
        self.history.record_response(id, returned_at, result);
    }

    /// Return the current immutable history.
    pub fn history(&self) -> &History {
        &self.history
    }

    /// Finish recording and return the history.
    pub fn into_history(self) -> History {
        self.history
    }
}

/// Seeded generator configuration for in-process oracle histories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinearizabilityGeneratorConfig {
    /// Seed passed to the deterministic workload generator.
    pub seed: u64,
    /// Workload shape.
    pub workload: WorkloadConfig,
    /// Logical time advanced per invocation/response pair.
    pub step: LogicalDuration,
}

impl Default for LinearizabilityGeneratorConfig {
    fn default() -> Self {
        Self {
            seed: 0x64_25,
            workload: WorkloadConfig::default(),
            step: LogicalDuration::from_millis(1),
        }
    }
}

/// Deterministic generator that emits histories with a built-in register model.
#[derive(Debug, Clone)]
pub struct LinearizabilityGenerator {
    workload: WorkloadGenerator,
    now: LogicalTime,
    step: LogicalDuration,
    sequence: u64,
    model: RegisterState,
}

impl LinearizabilityGenerator {
    /// Create a generator from config.
    pub fn new(config: LinearizabilityGeneratorConfig) -> Self {
        Self {
            workload: WorkloadGenerator::new(config.seed, config.workload),
            now: LogicalTime::from_millis(0),
            step: config.step,
            sequence: 0,
            model: RegisterState::default(),
        }
    }

    /// Generate a completed, linearizable history of `operations` operations.
    pub fn completed_history(&mut self, operations: usize) -> History {
        let mut history = History::new();
        for _ in 0..operations {
            let (client, op) = self.workload.next_invocation();
            let invoked_at = self.tick();
            let result = self.apply_generated(&op);
            let id = history.record_invocation(client, op, invoked_at);
            let returned_at = self.tick();
            history.record_response(id, returned_at, result);
        }
        history
    }

    fn apply_generated(&mut self, op: &WorkloadOp) -> WorkloadResult {
        match op {
            WorkloadOp::Put { key, value } => {
                self.model.put(key.clone(), value.clone());
                self.accepted()
            }
            WorkloadOp::Invalidate { key } => {
                self.model.delete(key);
                self.accepted()
            }
            WorkloadOp::CompareAndSet {
                key,
                expected,
                value,
            } => {
                if self.model.get(key) == *expected {
                    self.model.put(key.clone(), value.clone());
                    self.accepted()
                } else {
                    WorkloadResult::Rejected
                }
            }
            WorkloadOp::Get { key } | WorkloadOp::SessionRead { key } => {
                WorkloadResult::Value(self.model.get(key))
            }
        }
    }

    fn accepted(&mut self) -> WorkloadResult {
        self.sequence = self.sequence.saturating_add(1);
        WorkloadResult::Accepted {
            sequence: self.sequence,
        }
    }

    fn tick(&mut self) -> LogicalTime {
        self.now = self.now.saturating_add(self.step);
        self.now
    }
}

/// One linearizability violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinearizabilityViolation {
    /// Key whose register semantics were violated.
    pub key: String,
    /// Human-readable explanation.
    pub message: String,
}

/// Linearizability check report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LinearizabilityReport {
    /// Completed reads checked.
    pub checked_reads: usize,
    /// Completed operations considered by the oracle.
    pub checked_operations: usize,
    /// One valid linearization witness as history event indexes, if found.
    pub witness: Vec<usize>,
    /// Violations found.
    pub violations: Vec<LinearizabilityViolation>,
}

impl LinearizabilityReport {
    /// Return whether the report has no violations.
    pub fn is_ok(&self) -> bool {
        self.violations.is_empty()
    }

    fn violation(&mut self, key: impl Into<String>, message: impl Into<String>) {
        self.violations.push(LinearizabilityViolation {
            key: key.into(),
            message: message.into(),
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Operation {
    event_index: usize,
    invoked_at: LogicalTime,
    returned_at: LogicalTime,
    kind: OperationKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OperationKind {
    Read {
        key: String,
        observed: Option<Vec<u8>>,
    },
    Write {
        key: String,
        value: Vec<u8>,
    },
    Delete {
        key: String,
    },
    Cas {
        key: String,
        expected: Option<Vec<u8>>,
        value: Vec<u8>,
        accepted: bool,
    },
    Noop,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct RegisterState {
    values: BTreeMap<String, Vec<u8>>,
}

impl RegisterState {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.values.get(key).cloned()
    }

    fn put(&mut self, key: String, value: Vec<u8>) {
        self.values.insert(key, value);
    }

    fn delete(&mut self, key: &str) {
        self.values.remove(key);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SearchState {
    placed: Vec<bool>,
    model: RegisterState,
    witness: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompletedWrite {
    key: String,
    returned_at: LogicalTime,
    value: Option<Vec<u8>>,
}

/// Register-style linearizability checker for completed history events.
#[derive(Debug, Clone, Default)]
pub struct LinearizabilityChecker;

impl LinearizabilityChecker {
    /// Check register linearizability for all completed history events.
    ///
    /// The checker searches for a sequential witness that respects real-time
    /// order: if operation A completed before operation B was invoked, A must
    /// appear before B. Overlapping operations may be ordered either way.
    pub fn check(&self, history: &History) -> LinearizabilityReport {
        let operations = completed_operations(history);
        let mut report = LinearizabilityReport::default();
        report.checked_operations = operations.len();
        report.checked_reads = operations
            .iter()
            .filter(|op| matches!(op.kind, OperationKind::Read { .. }))
            .count();

        if let Some(witness) = find_linearization(&operations) {
            report.witness = witness;
        } else {
            let key = diagnostic_key(&operations);
            report.violation(
                key,
                format!(
                    "no sequential witness for {} completed operation(s)",
                    operations.len()
                ),
            );
        }
        report
    }
}

fn completed_operations(history: &History) -> Vec<Operation> {
    history
        .events()
        .iter()
        .enumerate()
        .filter_map(|(event_index, event)| {
            let returned_at = event.returned_at?;
            Some(Operation {
                event_index,
                invoked_at: event.invoked_at,
                returned_at,
                kind: operation_kind(&event.op, &event.result)?,
            })
        })
        .collect()
}

fn operation_kind(op: &WorkloadOp, result: &Option<WorkloadResult>) -> Option<OperationKind> {
    match (op, result) {
        (WorkloadOp::Put { key, value }, Some(WorkloadResult::Accepted { .. })) => {
            Some(OperationKind::Write {
                key: key.clone(),
                value: value.clone(),
            })
        }
        (WorkloadOp::Invalidate { key }, Some(WorkloadResult::Accepted { .. })) => {
            Some(OperationKind::Delete { key: key.clone() })
        }
        (
            WorkloadOp::CompareAndSet {
                key,
                expected,
                value,
            },
            Some(WorkloadResult::Accepted { .. }),
        ) => Some(OperationKind::Cas {
            key: key.clone(),
            expected: expected.clone(),
            value: value.clone(),
            accepted: true,
        }),
        (
            WorkloadOp::CompareAndSet {
                key,
                expected,
                value,
            },
            Some(WorkloadResult::Rejected),
        ) => Some(OperationKind::Cas {
            key: key.clone(),
            expected: expected.clone(),
            value: value.clone(),
            accepted: false,
        }),
        (
            WorkloadOp::Get { key } | WorkloadOp::SessionRead { key },
            Some(WorkloadResult::Value(observed)),
        ) => Some(OperationKind::Read {
            key: key.clone(),
            observed: observed.clone(),
        }),
        (_, Some(WorkloadResult::Error(_))) => Some(OperationKind::Noop),
        (_, Some(WorkloadResult::Rejected)) => Some(OperationKind::Noop),
        (_, None) => None,
        _ => None,
    }
}

fn find_linearization(operations: &[Operation]) -> Option<Vec<usize>> {
    let state = SearchState {
        placed: vec![false; operations.len()],
        model: RegisterState::default(),
        witness: Vec::with_capacity(operations.len()),
    };
    search(operations, state)
}

fn search(operations: &[Operation], state: SearchState) -> Option<Vec<usize>> {
    if state.witness.len() == operations.len() {
        return Some(state.witness);
    }

    for index in 0..operations.len() {
        if state.placed[index] || !predecessors_placed(operations, &state.placed, index) {
            continue;
        }
        let Some(next_model) = apply_operation(&state.model, &operations[index].kind) else {
            continue;
        };
        let mut next = state.clone();
        next.placed[index] = true;
        next.model = next_model;
        next.witness.push(operations[index].event_index);
        if let Some(witness) = search(operations, next) {
            return Some(witness);
        }
    }
    None
}

fn predecessors_placed(operations: &[Operation], placed: &[bool], index: usize) -> bool {
    operations.iter().enumerate().all(|(other_index, other)| {
        other_index == index
            || placed[other_index]
            || other.returned_at > operations[index].invoked_at
    })
}

fn apply_operation(model: &RegisterState, operation: &OperationKind) -> Option<RegisterState> {
    let mut next = model.clone();
    match operation {
        OperationKind::Read { key, observed } => (next.get(key) == *observed).then_some(next),
        OperationKind::Write { key, value } => {
            next.put(key.clone(), value.clone());
            Some(next)
        }
        OperationKind::Delete { key } => {
            next.delete(key);
            Some(next)
        }
        OperationKind::Cas {
            key,
            expected,
            value,
            accepted,
        } => {
            let current = next.get(key);
            if *accepted {
                if current == *expected {
                    next.put(key.clone(), value.clone());
                    Some(next)
                } else {
                    None
                }
            } else if current != *expected {
                Some(next)
            } else {
                None
            }
        }
        OperationKind::Noop => Some(next),
    }
}

fn diagnostic_key(operations: &[Operation]) -> String {
    operations
        .iter()
        .find_map(|operation| match &operation.kind {
            OperationKind::Read { key, .. }
            | OperationKind::Write { key, .. }
            | OperationKind::Delete { key }
            | OperationKind::Cas { key, .. } => Some(key.clone()),
            OperationKind::Noop => None,
        })
        .unwrap_or_else(|| "<history>".to_owned())
}

#[allow(dead_code)]
fn completed_write(event: &crate::HistoryEvent) -> Option<CompletedWrite> {
    let returned_at = event.returned_at?;
    match operation_kind(&event.op, &event.result)? {
        OperationKind::Write { key, value } | OperationKind::Cas { key, value, .. } => {
            Some(CompletedWrite {
                key,
                returned_at,
                value: Some(value),
            })
        }
        OperationKind::Delete { key } => Some(CompletedWrite {
            key,
            returned_at,
            value: None,
        }),
        OperationKind::Read { .. } | OperationKind::Noop => None,
    }
}
