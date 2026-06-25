use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use hydracache::ClusterNodeId;

use crate::election::{ElectionDriverSnapshot, NodeFsmState};
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

/// Per-node election/topology observation used by C3 invariant checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElectionTopologyNode {
    /// Replica node id.
    pub node_id: ClusterNodeId,
    /// Whether the node is live in the modeled topology.
    pub up: bool,
    /// Current election role.
    pub state: NodeFsmState,
    /// Current modeled term.
    pub term: u64,
    /// Vote target in the current term.
    pub voted_for: Option<ClusterNodeId>,
    /// Votes received by this node when it is a candidate/leader.
    pub votes_received: usize,
    /// Historical `(commit_index, applied_index)` samples.
    pub index_history: Vec<(u64, u64)>,
    /// Applied commit sequence observed while catching up.
    pub applied_commits: Vec<u64>,
    /// Writes accepted after this node stopped being the authoritative leader.
    pub stale_leader_writes: u64,
}

impl ElectionTopologyNode {
    /// Build a live follower-like node with monotonic indices.
    pub fn new(node_id: impl Into<ClusterNodeId>) -> Self {
        Self {
            node_id: node_id.into(),
            up: true,
            state: NodeFsmState::Follower,
            term: 0,
            voted_for: None,
            votes_received: 0,
            index_history: vec![(0, 0)],
            applied_commits: Vec::new(),
            stale_leader_writes: 0,
        }
    }

    /// Set election role and term.
    pub fn role(mut self, state: NodeFsmState, term: u64) -> Self {
        self.state = state;
        self.term = term;
        self
    }

    /// Set vote metadata.
    pub fn vote(mut self, voted_for: impl Into<ClusterNodeId>, votes_received: usize) -> Self {
        self.voted_for = Some(voted_for.into());
        self.votes_received = votes_received;
        self
    }

    /// Replace index history.
    pub fn index_history(mut self, history: Vec<(u64, u64)>) -> Self {
        self.index_history = history;
        self
    }

    /// Replace catch-up applied commits.
    pub fn applied_commits(mut self, commits: Vec<u64>) -> Self {
        self.applied_commits = commits;
        self
    }

    /// Mark writes accepted after leadership was lost.
    pub fn stale_leader_writes(mut self, writes: u64) -> Self {
        self.stale_leader_writes = writes;
        self
    }
}

/// Subscriber delivery observation used by `event_after_commit`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriberDeliveryObservation {
    /// Subscriber id.
    pub subscriber_id: String,
    /// Event key.
    pub key: String,
    /// Commit index required before this event may be delivered.
    pub commit_index: u64,
    /// Commit index visible when the subscriber received the event.
    pub delivered_after_commit_index: u64,
}

/// C3 election/topology state checked every simulator step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElectionTopologyState {
    /// Current authoritative leader.
    pub leader: Option<ClusterNodeId>,
    /// Total configured node count, including unavailable nodes.
    pub total_nodes: usize,
    /// Per-node observations.
    pub nodes: Vec<ElectionTopologyNode>,
    /// Subscriber delivery observations.
    pub subscriber_deliveries: Vec<SubscriberDeliveryObservation>,
}

impl ElectionTopologyState {
    /// Build a topology state from the deterministic election driver.
    pub fn from_election_snapshot(snapshot: &ElectionDriverSnapshot) -> Self {
        Self {
            leader: snapshot.leader.clone(),
            total_nodes: snapshot.nodes.len(),
            nodes: snapshot
                .nodes
                .iter()
                .map(|node| {
                    ElectionTopologyNode::new(node.node_id.clone())
                        .role(node.state, node.term)
                        .index_history(vec![(0, 0)])
                        .vote(
                            node.voted_for
                                .clone()
                                .unwrap_or_else(|| node.node_id.clone()),
                            node.votes_received,
                        )
                })
                .collect(),
            subscriber_deliveries: Vec::new(),
        }
    }

    /// Build from explicit nodes for tests.
    pub fn new(total_nodes: usize, nodes: Vec<ElectionTopologyNode>) -> Self {
        Self {
            leader: nodes
                .iter()
                .find(|node| node.state == NodeFsmState::Leader)
                .map(|node| node.node_id.clone()),
            total_nodes,
            nodes,
            subscriber_deliveries: Vec::new(),
        }
    }

    /// Override the authoritative leader.
    pub fn leader(mut self, leader: Option<ClusterNodeId>) -> Self {
        self.leader = leader;
        self
    }

    /// Add one subscriber delivery observation.
    pub fn subscriber_delivery(mut self, delivery: SubscriberDeliveryObservation) -> Self {
        self.subscriber_deliveries.push(delivery);
        self
    }
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

    /// Check only election/topology invariants.
    pub fn check_election_topology(&self, topology: &ElectionTopologyState) -> InvariantReport {
        let mut report = InvariantReport::default();
        self.check_election_safety(topology, &mut report);
        self.check_leader_requires_quorum(topology, &mut report);
        self.check_no_stale_leader_writes(topology, &mut report);
        self.check_index_monotonicity(topology, &mut report);
        self.check_catchup_no_skip(topology, &mut report);
        self.check_event_after_commit(topology, &mut report);
        report
    }

    /// Check history and replica snapshots.
    pub fn check(&self, history: &History, replicas: &[ReplicaSnapshot]) -> InvariantReport {
        let mut report = self.check_history(history);
        report.merge(self.check_replicas(replicas));
        report
    }

    /// Check history plus election/topology invariants.
    pub fn check_history_and_election(
        &self,
        history: &History,
        topology: &ElectionTopologyState,
    ) -> InvariantReport {
        let mut report = self.check_history(history);
        report.merge(self.check_election_topology(topology));
        report
    }

    fn check_election_safety(
        &self,
        topology: &ElectionTopologyState,
        report: &mut InvariantReport,
    ) {
        report.checked();
        let mut leaders_by_term: BTreeMap<u64, Vec<&ClusterNodeId>> = BTreeMap::new();
        for node in topology
            .nodes
            .iter()
            .filter(|node| node.state == NodeFsmState::Leader)
        {
            leaders_by_term
                .entry(node.term)
                .or_default()
                .push(&node.node_id);
        }
        for (term, leaders) in leaders_by_term {
            if leaders.len() > 1 {
                report.violation(
                    "election_safety",
                    format!("term {term} has multiple leaders: {leaders:?}"),
                );
            }
        }
    }

    fn check_leader_requires_quorum(
        &self,
        topology: &ElectionTopologyState,
        report: &mut InvariantReport,
    ) {
        report.checked();
        let quorum = topology.total_nodes / 2 + 1;
        for node in topology
            .nodes
            .iter()
            .filter(|node| node.state == NodeFsmState::Leader)
        {
            if node.votes_received < quorum {
                report.violation(
                    "leader_requires_quorum",
                    format!(
                        "{} is leader with {} vote(s), quorum is {quorum}",
                        node.node_id, node.votes_received
                    ),
                );
            }
        }
    }

    fn check_no_stale_leader_writes(
        &self,
        topology: &ElectionTopologyState,
        report: &mut InvariantReport,
    ) {
        report.checked();
        for node in &topology.nodes {
            let is_authoritative = topology
                .leader
                .as_ref()
                .is_some_and(|leader| leader == &node.node_id);
            if !is_authoritative && node.stale_leader_writes > 0 {
                report.violation(
                    "no_stale_leader_writes",
                    format!(
                        "{} accepted {} write(s) after losing leadership",
                        node.node_id, node.stale_leader_writes
                    ),
                );
            }
        }
    }

    fn check_index_monotonicity(
        &self,
        topology: &ElectionTopologyState,
        report: &mut InvariantReport,
    ) {
        report.checked();
        for node in &topology.nodes {
            for window in node.index_history.windows(2) {
                let [(prev_commit, prev_applied), (commit, applied)] = window else {
                    continue;
                };
                if commit < prev_commit || applied < prev_applied {
                    report.violation(
                        "index_monotonicity",
                        format!(
                            "{} index regressed from ({prev_commit},{prev_applied}) to ({commit},{applied})",
                            node.node_id
                        ),
                    );
                }
            }
            for (commit, applied) in &node.index_history {
                if applied > commit {
                    report.violation(
                        "index_monotonicity",
                        format!(
                            "{} applied index {applied} is ahead of commit index {commit}",
                            node.node_id
                        ),
                    );
                }
            }
        }
    }

    fn check_catchup_no_skip(
        &self,
        topology: &ElectionTopologyState,
        report: &mut InvariantReport,
    ) {
        report.checked();
        for node in &topology.nodes {
            for window in node.applied_commits.windows(2) {
                let [previous, next] = window else {
                    continue;
                };
                if next != &(previous.saturating_add(1)) {
                    report.violation(
                        "catchup_no_skip",
                        format!(
                            "{} skipped commit {} before applying {next}",
                            node.node_id,
                            previous + 1
                        ),
                    );
                }
            }
        }
    }

    fn check_event_after_commit(
        &self,
        topology: &ElectionTopologyState,
        report: &mut InvariantReport,
    ) {
        report.checked();
        for delivery in &topology.subscriber_deliveries {
            if delivery.delivered_after_commit_index < delivery.commit_index {
                report.violation(
                    "event_after_commit",
                    format!(
                        "{} received '{}' at commit {}, before required commit {}",
                        delivery.subscriber_id,
                        delivery.key,
                        delivery.delivered_after_commit_index,
                        delivery.commit_index
                    ),
                );
            }
        }
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
