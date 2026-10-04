//! Bounded single-key state-machine history oracle for the provisional 0.75 value plane.
//!
//! The checker models real-time precedence, conditional return values, and ambiguous mutation
//! completions. It deliberately excludes transports, protocol generations, durable formats, and
//! physical time.

use std::collections::{BTreeMap, BTreeSet};

use crate::value_plane_model_075::{CanonicalMapKey, OutcomeCertainty, ValuePlaneError};

/// Canonical operations understood by the history oracle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryOperation {
    Read,
    Put(Vec<u8>),
    PutIfAbsent(Vec<u8>),
    ReplaceIfPresent(Vec<u8>),
    ReplaceIfValue { expected: Vec<u8>, value: Vec<u8> },
    RemoveIfValue { expected: Vec<u8> },
    GetAndPut(Vec<u8>),
    GetAndRemove,
    Delete,
}

/// Mutation result at the canonical domain boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryOutcome {
    pub applied: bool,
    pub previous: Option<Vec<u8>>,
    pub current: Option<Vec<u8>>,
}

impl HistoryOutcome {
    pub fn applied(previous: Option<Vec<u8>>, current: Option<Vec<u8>>) -> Self {
        Self {
            applied: true,
            previous,
            current,
        }
    }

    fn rejected(current: Option<Vec<u8>>) -> Self {
        Self {
            applied: false,
            previous: current.clone(),
            current,
        }
    }
}

/// Observable completion. `Unknown` is valid only with `OutcomeUnknown` certainty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryResult {
    Value(Option<Vec<u8>>),
    Mutation(HistoryOutcome),
    Unknown,
}

/// One completed invocation interval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryCall {
    pub id: u64,
    pub key: CanonicalMapKey,
    pub operation: HistoryOperation,
    pub invoked_at: u64,
    pub completed_at: u64,
    pub result: HistoryResult,
    pub certainty: OutcomeCertainty,
}

impl HistoryCall {
    #[allow(clippy::too_many_arguments)]
    pub fn completed(
        id: u64,
        key: CanonicalMapKey,
        operation: HistoryOperation,
        invoked_at: u64,
        completed_at: u64,
        result: HistoryResult,
        certainty: OutcomeCertainty,
    ) -> Self {
        Self {
            id,
            key,
            operation,
            invoked_at,
            completed_at,
            result,
            certainty,
        }
    }
}

/// Bounded history input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuePlaneHistory {
    max_operations: usize,
    calls: Vec<HistoryCall>,
}

impl ValuePlaneHistory {
    pub fn new(max_operations: usize) -> Result<Self, ValuePlaneError> {
        if max_operations == 0 {
            return Err(ValuePlaneError::InvalidBound("linearizability_operations"));
        }
        Ok(Self {
            max_operations,
            calls: Vec::new(),
        })
    }

    pub fn push(&mut self, call: HistoryCall) -> Result<(), ValuePlaneError> {
        if self.calls.len() >= self.max_operations {
            return Err(ValuePlaneError::BoundExceeded {
                bound: "linearizability_operations",
                limit: self.max_operations,
                actual: self.calls.len().saturating_add(1),
            });
        }
        if call.completed_at < call.invoked_at || self.calls.iter().any(|seen| seen.id == call.id) {
            return Err(ValuePlaneError::MutationIdentityConflict);
        }
        self.calls.push(call);
        Ok(())
    }

    pub fn calls(&self) -> &[HistoryCall] {
        &self.calls
    }
}

/// Oracle verdict with a concrete witness or a bounded failure reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryOracleReport {
    pub witness: Option<Vec<u64>>,
    pub explored_states: usize,
    pub violation: Option<String>,
}

impl HistoryOracleReport {
    pub const fn is_linearizable(&self) -> bool {
        self.witness.is_some()
    }
}

/// Exhaustive bounded state-machine checker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValuePlaneHistoryOracle {
    max_search_states: usize,
}

impl ValuePlaneHistoryOracle {
    pub fn new(max_search_states: usize) -> Self {
        Self {
            max_search_states: max_search_states.max(1),
        }
    }

    pub fn check(&self, history: &ValuePlaneHistory) -> HistoryOracleReport {
        let predecessors = predecessor_sets(history.calls());
        let mut search = Search {
            calls: history.calls(),
            predecessors: &predecessors,
            max_states: self.max_search_states,
            explored: 0,
            bound_exhausted: false,
        };
        let initial = SearchState {
            values: BTreeMap::new(),
            placed: vec![false; history.calls().len()],
            witness: Vec::new(),
        };
        let witness = search.visit(initial);
        let violation = if witness.is_some() {
            None
        } else if search.bound_exhausted {
            Some(format!(
                "linearizability search exceeded bounded state budget {}",
                self.max_search_states
            ))
        } else {
            Some(
                "no legal linearization satisfies real-time order and observed outcomes".to_owned(),
            )
        };
        HistoryOracleReport {
            witness,
            explored_states: search.explored,
            violation,
        }
    }
}

fn predecessor_sets(calls: &[HistoryCall]) -> Vec<BTreeSet<usize>> {
    calls
        .iter()
        .map(|call| {
            calls
                .iter()
                .enumerate()
                .filter_map(|(index, other)| {
                    (other.id != call.id && other.completed_at <= call.invoked_at).then_some(index)
                })
                .collect()
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SearchState {
    values: BTreeMap<CanonicalMapKey, Vec<u8>>,
    placed: Vec<bool>,
    witness: Vec<u64>,
}

struct Search<'a> {
    calls: &'a [HistoryCall],
    predecessors: &'a [BTreeSet<usize>],
    max_states: usize,
    explored: usize,
    bound_exhausted: bool,
}

impl Search<'_> {
    fn visit(&mut self, state: SearchState) -> Option<Vec<u64>> {
        if self.explored >= self.max_states {
            self.bound_exhausted = true;
            return None;
        }
        self.explored += 1;
        if state.placed.iter().all(|placed| *placed) {
            return Some(state.witness);
        }

        for index in 0..self.calls.len() {
            if state.placed[index]
                || !self.predecessors[index]
                    .iter()
                    .all(|predecessor| state.placed[*predecessor])
            {
                continue;
            }
            for values in apply_call(&state.values, &self.calls[index]) {
                let mut next = state.clone();
                next.values = values;
                next.placed[index] = true;
                next.witness.push(self.calls[index].id);
                if let Some(witness) = self.visit(next) {
                    return Some(witness);
                }
            }
        }
        None
    }
}

fn apply_call(
    values: &BTreeMap<CanonicalMapKey, Vec<u8>>,
    call: &HistoryCall,
) -> Vec<BTreeMap<CanonicalMapKey, Vec<u8>>> {
    if call.certainty == OutcomeCertainty::OutcomeUnknown {
        if call.result != HistoryResult::Unknown {
            return Vec::new();
        }
        let mut candidates = vec![values.clone()];
        if !matches!(call.operation, HistoryOperation::Read) {
            let (applied, _) = apply_operation(values, &call.key, &call.operation);
            if applied != *values {
                candidates.push(applied);
            }
        }
        return candidates;
    }

    let (candidate, expected) = apply_operation(values, &call.key, &call.operation);
    (expected == call.result)
        .then_some(candidate)
        .into_iter()
        .collect()
}

fn apply_operation(
    values: &BTreeMap<CanonicalMapKey, Vec<u8>>,
    key: &CanonicalMapKey,
    operation: &HistoryOperation,
) -> (BTreeMap<CanonicalMapKey, Vec<u8>>, HistoryResult) {
    let mut next = values.clone();
    let previous = values.get(key).cloned();
    match operation {
        HistoryOperation::Read => (next, HistoryResult::Value(previous)),
        HistoryOperation::Put(value) | HistoryOperation::GetAndPut(value) => {
            next.insert(key.clone(), value.clone());
            (
                next,
                HistoryResult::Mutation(HistoryOutcome::applied(previous, Some(value.clone()))),
            )
        }
        HistoryOperation::PutIfAbsent(value) => {
            if previous.is_none() {
                next.insert(key.clone(), value.clone());
                (
                    next,
                    HistoryResult::Mutation(HistoryOutcome::applied(None, Some(value.clone()))),
                )
            } else {
                (
                    next,
                    HistoryResult::Mutation(HistoryOutcome::rejected(previous)),
                )
            }
        }
        HistoryOperation::ReplaceIfPresent(value) => {
            if previous.is_some() {
                next.insert(key.clone(), value.clone());
                (
                    next,
                    HistoryResult::Mutation(HistoryOutcome::applied(previous, Some(value.clone()))),
                )
            } else {
                (
                    next,
                    HistoryResult::Mutation(HistoryOutcome::rejected(None)),
                )
            }
        }
        HistoryOperation::ReplaceIfValue { expected, value } => {
            if previous.as_ref() == Some(expected) {
                next.insert(key.clone(), value.clone());
                (
                    next,
                    HistoryResult::Mutation(HistoryOutcome::applied(previous, Some(value.clone()))),
                )
            } else {
                (
                    next,
                    HistoryResult::Mutation(HistoryOutcome::rejected(previous)),
                )
            }
        }
        HistoryOperation::RemoveIfValue { expected } => {
            if previous.as_ref() == Some(expected) {
                next.remove(key);
                (
                    next,
                    HistoryResult::Mutation(HistoryOutcome::applied(previous, None)),
                )
            } else {
                (
                    next,
                    HistoryResult::Mutation(HistoryOutcome::rejected(previous)),
                )
            }
        }
        HistoryOperation::GetAndRemove | HistoryOperation::Delete => {
            next.remove(key);
            let result = if previous.is_some() {
                HistoryOutcome::applied(previous, None)
            } else {
                HistoryOutcome::rejected(None)
            };
            (next, HistoryResult::Mutation(result))
        }
    }
}
