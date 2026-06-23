use std::collections::BTreeMap;

use hydracache::{
    ClientAck, ClientOp, ClusterNode, ClusterNodeConfig, ClusterNodeId, LogicalDuration,
    LogicalTime, OutboundClusterMessage,
};

use crate::{
    History, InvariantChecker, InvariantReport, SimClock, SimNetwork, SimRng, SimStorage,
    WorkloadConfig, WorkloadGenerator, WorkloadOp, WorkloadResult,
};

/// Configuration for a deterministic simulation run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimConfig {
    /// Number of sans-IO cluster nodes to instantiate.
    pub node_count: usize,
    /// Heartbeat interval passed to every node.
    pub heartbeat_interval: LogicalDuration,
    /// Logical time advanced per scheduler step.
    pub step_duration: LogicalDuration,
    /// Number of keys used by the built-in W4 smoke workload.
    pub key_count: u64,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            node_count: 3,
            heartbeat_interval: LogicalDuration::from_millis(1),
            step_duration: LogicalDuration::from_millis(1),
            key_count: 4,
        }
    }
}

impl SimConfig {
    fn normalized(mut self) -> Self {
        self.node_count = self.node_count.max(1);
        self.key_count = self.key_count.max(1);
        self
    }
}

/// Deterministic run result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimOutcome {
    /// Seed used to create the run.
    pub seed: u64,
    /// Number of scheduler steps executed.
    pub steps: u64,
    /// Client operations accepted by nodes.
    pub accepted_ops: u64,
    /// Network messages delivered to nodes.
    pub delivered_messages: u64,
    /// Stable hash of the recorded W4 trace.
    pub history_hash: u64,
    /// Number of invariant violations in the latest step report.
    pub invariant_violations: usize,
}

#[derive(Debug, Clone)]
struct SimNode {
    node: ClusterNode,
    storage: SimStorage,
}

/// Deterministic whole-cluster simulation driver.
#[derive(Debug, Clone)]
pub struct SimWorld {
    seed: u64,
    cfg: SimConfig,
    rng: SimRng,
    clock: SimClock,
    network: SimNetwork,
    workload: WorkloadGenerator,
    history: History,
    invariant_checker: InvariantChecker,
    invariant_report: InvariantReport,
    nodes: BTreeMap<ClusterNodeId, SimNode>,
    steps: u64,
    accepted_ops: u64,
    delivered_messages: u64,
    trace: Vec<String>,
}

impl SimWorld {
    /// Build a world from a seed and config.
    pub fn new(seed: u64, cfg: SimConfig) -> Self {
        let cfg = cfg.normalized();
        let key_count = cfg.key_count;
        let node_ids = node_ids(cfg.node_count);
        let mut nodes = BTreeMap::new();
        for node_id in &node_ids {
            let peers = node_ids
                .iter()
                .filter(|peer| *peer != node_id)
                .cloned()
                .collect();
            let node = ClusterNode::new(
                ClusterNodeConfig::new(node_id.clone(), peers)
                    .heartbeat_interval(cfg.heartbeat_interval),
            );
            nodes.insert(
                node_id.clone(),
                SimNode {
                    node,
                    storage: SimStorage::new(),
                },
            );
        }

        Self {
            seed,
            cfg,
            rng: SimRng::from_seed(seed),
            clock: SimClock::default(),
            network: SimNetwork::from_seed(seed ^ 0x44_44_44_44),
            workload: WorkloadGenerator::new(
                seed ^ 0x55_55_55_55,
                WorkloadConfig {
                    key_count,
                    ..WorkloadConfig::default()
                },
            ),
            history: History::new(),
            invariant_checker: InvariantChecker,
            invariant_report: InvariantReport::default(),
            nodes,
            steps: 0,
            accepted_ops: 0,
            delivered_messages: 0,
            trace: Vec::new(),
        }
    }

    /// Run the scheduler for `steps`.
    pub fn run(&mut self, steps: u64) -> SimOutcome {
        for _ in 0..steps {
            self.step();
        }
        self.outcome()
    }

    /// Execute one deterministic scheduler step.
    pub fn step(&mut self) {
        self.steps = self.steps.saturating_add(1);
        self.clock.advance(self.cfg.step_duration);
        self.record(format!(
            "step:{} now:{}",
            self.steps,
            self.clock.now().as_millis()
        ));

        self.deliver_network();
        self.tick_nodes();
        self.issue_smoke_workload();
        self.drain_node_effects();
        self.invariant_report = self.invariant_checker.check_history(&self.history);
    }

    /// Return a snapshot outcome without advancing the world.
    pub fn outcome(&self) -> SimOutcome {
        SimOutcome {
            seed: self.seed,
            steps: self.steps,
            accepted_ops: self.accepted_ops,
            delivered_messages: self.delivered_messages,
            history_hash: self.history.hash(),
            invariant_violations: self.invariant_report.violations.len(),
        }
    }

    /// Return the current logical time.
    pub fn now(&self) -> LogicalTime {
        self.clock.now()
    }

    /// Return the recorded workload history.
    pub fn history(&self) -> &History {
        &self.history
    }

    /// Return the latest invariant report.
    pub fn invariant_report(&self) -> &InvariantReport {
        &self.invariant_report
    }

    fn deliver_network(&mut self) {
        let delivered = self.network.deliverable(self.clock.now());
        self.delivered_messages = self
            .delivered_messages
            .saturating_add(delivered.len() as u64);
        for (from, to, message) in delivered {
            self.record(format!("deliver:{from}->{to}:{message:?}"));
            if let Some(target) = self.nodes.get_mut(&to) {
                target.node.handle_message(from, message);
            }
        }
    }

    fn tick_nodes(&mut self) {
        for sim_node in self.nodes.values_mut() {
            sim_node.node.tick(self.clock.now());
        }
    }

    fn issue_smoke_workload(&mut self) {
        let node_count = self.nodes.len();
        if node_count == 0 {
            return;
        }
        let node_index = self.rng.next_index(node_count);
        let node_id = self
            .nodes
            .keys()
            .nth(node_index)
            .expect("node index is within node count")
            .clone();
        let (client, op) = self.workload.next_invocation();
        if let Some(sim_node) = self.nodes.get_mut(&node_id) {
            let event_id = self
                .history
                .record_invocation(client, op.clone(), self.clock.now());
            let ack = sim_node.node.handle_client(client_op(op));
            self.accepted_ops = self.accepted_ops.saturating_add(1);
            self.history
                .record_response(event_id, self.clock.now(), workload_result(ack));
            self.record(format!("client:{client}:{node_id}:{event_id:?}"));
        }
    }

    fn drain_node_effects(&mut self) {
        let mut outbound = Vec::new();
        for sim_node in self.nodes.values_mut() {
            outbound.extend(sim_node.node.take_outbound());
            for request in sim_node.node.storage_requests() {
                let applied = sim_node
                    .storage
                    .apply_checked(request)
                    .expect("W4 storage has no injected faults");
                sim_node.node.apply_storage_result(applied.result);
            }
        }
        self.send_outbound(outbound);
    }

    fn send_outbound(&mut self, outbound: Vec<OutboundClusterMessage>) {
        for outbound in outbound {
            self.record(format!(
                "send:{}->{}:{:?}",
                outbound.from, outbound.to, outbound.message
            ));
            self.network.send(
                outbound.from,
                outbound.to,
                outbound.message,
                self.clock.now(),
            );
        }
    }

    fn record(&mut self, event: String) {
        self.trace.push(event);
    }
}

fn node_ids(count: usize) -> Vec<ClusterNodeId> {
    (0..count)
        .map(|index| ClusterNodeId::new(format!("node-{index}")))
        .collect()
}

fn client_op(op: WorkloadOp) -> ClientOp {
    match op {
        WorkloadOp::Get { key } | WorkloadOp::SessionRead { key } => ClientOp::Get { key },
        WorkloadOp::Put { key, value } | WorkloadOp::CompareAndSet { key, value, .. } => {
            ClientOp::Put { key, value }
        }
        WorkloadOp::Invalidate { key } => ClientOp::Invalidate { key },
    }
}

fn workload_result(ack: ClientAck) -> WorkloadResult {
    match ack {
        ClientAck::Accepted { sequence } => WorkloadResult::Accepted { sequence },
        ClientAck::PendingStorage { request_id } => WorkloadResult::Accepted {
            sequence: request_id,
        },
    }
}
