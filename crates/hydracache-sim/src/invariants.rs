use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use hydracache::ClusterNodeId;

use crate::{History, WorkloadOp, WorkloadResult};

/// One committed log entry observed in a simulated replica.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    /// One-based log index.
    pub index: u64,
    /// Logical consensus term/epoch.
    pub term: u64,
    /// Affected key.
    pub key: String,
    /// Operation committed at this index.
    pub op: LogOp,
}

/// Committed operation shape used by invariant checkers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogOp {
    /// Store a value.
    Put(Vec<u8>),
    /// Tombstone/delete a value.
    Tombstone,
}

/// Per-key value observation in a replica snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueObservation {
    /// Monotonic value version.
    pub version: u64,
    /// Observed state at this version.
    pub state: ValueState,
}

/// Value or tombstone state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueState {
    /// Live value bytes.
    Value(Vec<u8>),
    /// Delete marker.
    Tombstone,
}

/// Snapshot used by invariant checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaSnapshot {
    /// Replica node id.
    pub node_id: ClusterNodeId,
    /// Committed log prefix.
    pub committed_log: Vec<LogEntry>,
    /// Number of committed entries known durable after recovery.
    pub durable_log_len: usize,
    /// Per-key value observations.
    pub values: BTreeMap<String, ValueObservation>,
}

impl ReplicaSnapshot {
    /// Build an empty snapshot.
    pub fn new(node_id: impl Into<ClusterNodeId>) -> Self {
        Self {
            node_id: node_id.into(),
            committed_log: Vec::new(),
            durable_log_len: 0,
            values: BTreeMap::new(),
        }
    }

    /// Replace the committed log.
    pub fn committed_log(mut self, committed_log: Vec<LogEntry>) -> Self {
        self.durable_log_len = committed_log.len();
        self.committed_log = committed_log;
        self
    }

    /// Override durable log length.
    pub fn durable_log_len(mut self, durable_log_len: usize) -> Self {
        self.durable_log_len = durable_log_len;
        self
    }

    /// Add a value observation.
    pub fn value(mut self, key: impl Into<String>, version: u64, state: ValueState) -> Self {
        self.values
            .insert(key.into(), ValueObservation { version, state });
        self
    }
}

/// One invariant violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvariantViolation {
    /// Stable invariant name.
    pub name: &'static str,
    /// Human-readable explanation.
    pub message: String,
}

impl InvariantViolation {
    fn new(name: &'static str, message: impl Into<String>) -> Self {
        Self {
            name,
            message: message.into(),
        }
    }
}

impl fmt::Display for InvariantViolation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.name, self.message)
    }
}

/// Invariant check report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InvariantReport {
    /// Number of checks executed.
    pub checked: usize,
    /// Violations found by the checks.
    pub violations: Vec<InvariantViolation>,
}

impl InvariantReport {
    /// Return whether the report has no violations.
    pub fn is_ok(&self) -> bool {
        self.violations.is_empty()
    }

    fn checked(&mut self) {
        self.checked = self.checked.saturating_add(1);
    }

    /// Record that one invariant was checked successfully.
    pub fn record_check(&mut self) {
        self.checked();
    }

    fn violation(&mut self, name: &'static str, message: impl Into<String>) {
        self.violations.push(InvariantViolation::new(name, message));
    }

    /// Record one invariant violation.
    pub fn record_violation(&mut self, name: &'static str, message: impl Into<String>) {
        self.violation(name, message);
    }

    fn merge(&mut self, other: InvariantReport) {
        self.checked = self.checked.saturating_add(other.checked);
        self.violations.extend(other.violations);
    }
}

/// Composable invariant checker.
#[derive(Debug, Clone, Default)]
pub struct InvariantChecker;

impl InvariantChecker {
    /// Check only workload history invariants.
    pub fn check_history(&self, history: &History) -> InvariantReport {
        let mut report = InvariantReport::default();
        self.check_read_your_writes(history, &mut report);
        self.check_no_read_after_invalidation_without_rewrite(history, &mut report);
        report
    }

    /// Check only replica snapshot invariants.
    pub fn check_replicas(&self, replicas: &[ReplicaSnapshot]) -> InvariantReport {
        let mut report = InvariantReport::default();
        self.check_consensus_prefix(replicas, &mut report);
        self.check_durability(replicas, &mut report);
        self.check_no_tombstone_resurrection(replicas, &mut report);
        self.check_convergence(replicas, &mut report);
        report
    }

    /// Check history and replica snapshots.
    pub fn check(&self, history: &History, replicas: &[ReplicaSnapshot]) -> InvariantReport {
        let mut report = self.check_history(history);
        report.merge(self.check_replicas(replicas));
        report
    }

    fn check_consensus_prefix(&self, replicas: &[ReplicaSnapshot], report: &mut InvariantReport) {
        report.checked();
        for (left_index, left) in replicas.iter().enumerate() {
            for right in replicas.iter().skip(left_index + 1) {
                let shared = left.committed_log.len().min(right.committed_log.len());
                for index in 0..shared {
                    if left.committed_log[index] != right.committed_log[index] {
                        report.violation(
                            "consensus-prefix",
                            format!(
                                "{} and {} diverge at committed index {}",
                                left.node_id,
                                right.node_id,
                                index + 1
                            ),
                        );
                        break;
                    }
                }
            }
        }
    }

    fn check_durability(&self, replicas: &[ReplicaSnapshot], report: &mut InvariantReport) {
        report.checked();
        for replica in replicas {
            if replica.durable_log_len < replica.committed_log.len() {
                report.violation(
                    "durability",
                    format!(
                        "{} durable log length {} is behind committed length {}",
                        replica.node_id,
                        replica.durable_log_len,
                        replica.committed_log.len()
                    ),
                );
            }
        }
    }

    fn check_no_tombstone_resurrection(
        &self,
        replicas: &[ReplicaSnapshot],
        report: &mut InvariantReport,
    ) {
        report.checked();
        let mut max_tombstones: BTreeMap<&str, u64> = BTreeMap::new();
        for replica in replicas {
            for (key, value) in &replica.values {
                if value.state == ValueState::Tombstone {
                    let version = max_tombstones.entry(key.as_str()).or_default();
                    *version = (*version).max(value.version);
                }
            }
        }
        for replica in replicas {
            for (key, value) in &replica.values {
                if matches!(value.state, ValueState::Value(_))
                    && max_tombstones
                        .get(key.as_str())
                        .is_some_and(|tombstone_version| value.version <= *tombstone_version)
                {
                    report.violation(
                        "tombstone-resurrection",
                        format!(
                            "{} has value for key '{key}' at version {} not newer than tombstone",
                            replica.node_id, value.version
                        ),
                    );
                }
            }
        }
    }

    fn check_convergence(&self, replicas: &[ReplicaSnapshot], report: &mut InvariantReport) {
        report.checked();
        let keys = replicas
            .iter()
            .flat_map(|replica| replica.values.keys().cloned())
            .collect::<BTreeSet<_>>();
        for key in keys {
            let mut observations = replicas
                .iter()
                .filter_map(|replica| {
                    replica
                        .values
                        .get(&key)
                        .map(|value| (&replica.node_id, value))
                })
                .collect::<Vec<_>>();
            observations.sort_by(|left, right| left.0.cmp(right.0));
            if let Some((_, first)) = observations.first() {
                for (node, value) in observations.iter().skip(1) {
                    if *value != *first {
                        report.violation("convergence", format!("{node} disagrees on key '{key}'"));
                    }
                }
            }
        }
    }

    fn check_read_your_writes(&self, history: &History, report: &mut InvariantReport) {
        report.checked();
        let mut writes: BTreeMap<(u64, String), Vec<u8>> = BTreeMap::new();
        for event in history.completed() {
            match (&event.op, &event.result) {
                (
                    WorkloadOp::Put { key, value } | WorkloadOp::CompareAndSet { key, value, .. },
                    Some(WorkloadResult::Accepted { .. }),
                ) => {
                    writes.insert((event.client, key.clone()), value.clone());
                }
                (
                    WorkloadOp::Get { key } | WorkloadOp::SessionRead { key },
                    Some(WorkloadResult::Value(value)),
                ) => {
                    if let Some(expected) = writes.get(&(event.client, key.clone())) {
                        if value.as_ref() != Some(expected) {
                            report.violation(
                                "read-your-writes",
                                format!("client {} read stale value for key '{key}'", event.client),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn check_no_read_after_invalidation_without_rewrite(
        &self,
        history: &History,
        report: &mut InvariantReport,
    ) {
        report.checked();
        let mut invalidated = BTreeSet::new();
        for event in history.completed() {
            match (&event.op, &event.result) {
                (WorkloadOp::Invalidate { key }, Some(WorkloadResult::Accepted { .. })) => {
                    invalidated.insert(key.clone());
                }
                (
                    WorkloadOp::Put { key, .. } | WorkloadOp::CompareAndSet { key, .. },
                    Some(WorkloadResult::Accepted { .. }),
                ) => {
                    invalidated.remove(key);
                }
                (
                    WorkloadOp::Get { key } | WorkloadOp::SessionRead { key },
                    Some(WorkloadResult::Value(Some(_))),
                ) if invalidated.contains(key) => {
                    report.violation(
                        "invalidate-read",
                        format!("key '{key}' was read after invalidation without rewrite"),
                    );
                }
                _ => {}
            }
        }
    }
}
