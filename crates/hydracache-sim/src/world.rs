use std::collections::{BTreeMap, BTreeSet, VecDeque};

use hydracache::{
    ClientAck, ClientOp, ClusterNode, ClusterNodeConfig, ClusterNodeId, LogicalDuration,
    LogicalTime, OutboundClusterMessage,
};

use crate::{
    ControlActionV1, ControlApplyError, ElectionDriver, ElectionDriverSnapshot, History,
    InvariantChecker, InvariantReport, LinkFault, PartitionSymmetry, ReplayScriptV1, SimClock,
    SimNetwork, SimRng, SimSnapshot, SimStorage, SubscriberEventView, WorkloadConfig,
    WorkloadGenerator, WorkloadOp, WorkloadResult, MAX_SUBSCRIBER_BUFFER,
};

const BUS_EVENT_UPSERTED: &str = "upserted";

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

#[derive(Debug, Clone)]
struct SimClientActor {
    numeric_id: u64,
    namespace: String,
    last_op: Option<String>,
    in_flight: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingSubscriberEvent {
    kind: String,
    key: String,
    version: u64,
    commit_index: u64,
}

#[derive(Debug, Clone)]
struct SimSubscriber {
    id: String,
    client_id: String,
    namespace: String,
    pending: VecDeque<PendingSubscriberEvent>,
    last_event: Option<SubscriberEventView>,
    dropped: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RebalanceState {
    phase: String,
    moved_partitions: u64,
    total_partitions: u64,
}

/// Deterministic whole-cluster simulation driver.
#[derive(Debug, Clone)]
pub struct SimWorld {
    seed: u64,
    cfg: SimConfig,
    rng: SimRng,
    clock: SimClock,
    network: SimNetwork,
    election: ElectionDriver,
    workload: WorkloadGenerator,
    history: History,
    invariant_checker: InvariantChecker,
    invariant_report: InvariantReport,
    nodes: BTreeMap<ClusterNodeId, SimNode>,
    clients: BTreeMap<String, SimClientActor>,
    subscribers: BTreeMap<String, SimSubscriber>,
    crashed_nodes: BTreeSet<ClusterNodeId>,
    isolated_nodes: BTreeSet<ClusterNodeId>,
    disabled_nodes: BTreeSet<ClusterNodeId>,
    rebalance: Option<RebalanceState>,
    mode: crate::SimMode,
    active_scenario: Option<String>,
    action_log: Vec<ControlActionV1>,
    workload_enabled: bool,
    next_client_numeric_id: u64,
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
            election: ElectionDriver::new(seed ^ 0x53_00_00_00, node_ids.clone()),
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
            clients: BTreeMap::new(),
            subscribers: BTreeMap::new(),
            crashed_nodes: BTreeSet::new(),
            isolated_nodes: BTreeSet::new(),
            disabled_nodes: BTreeSet::new(),
            rebalance: None,
            mode: crate::SimMode::Manual,
            active_scenario: None,
            action_log: Vec::new(),
            workload_enabled: true,
            next_client_numeric_id: 1,
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

        self.drive_election();
        self.deliver_network();
        self.tick_nodes();
        self.issue_smoke_workload();
        self.drain_node_effects();
        self.deliver_subscriber_events();
        self.refresh_invariant_report();
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

    /// Return the current deterministic election-driver snapshot.
    pub fn election_snapshot(&self) -> ElectionDriverSnapshot {
        self.election.snapshot()
    }

    /// Enable or disable the built-in smoke workload.
    pub fn set_workload_enabled(&mut self, enabled: bool) {
        self.workload_enabled = enabled;
    }

    /// Return whether the built-in smoke workload is enabled.
    pub fn workload_enabled(&self) -> bool {
        self.workload_enabled
    }

    /// Apply one shared control action.
    pub fn apply_control_action(
        &mut self,
        action: ControlActionV1,
    ) -> Result<(), ControlApplyError> {
        let result = match action.clone() {
            ControlActionV1::Step { n, .. } => {
                self.run(n);
                Ok(())
            }
            ControlActionV1::PushEvent {
                client,
                ns,
                key,
                value,
                ..
            } => self.push_event(client, ns, key, value),
            ControlActionV1::Subscribe { client, ns, .. } => {
                self.subscribe(client, ns);
                Ok(())
            }
            ControlActionV1::ModeChange { mode, .. } => {
                self.mode = mode;
                Ok(())
            }
            ControlActionV1::Isolate { node, .. } => {
                self.isolate_node(node).then_some(()).ok_or_else(|| {
                    ControlApplyError::InvalidAction(
                        "isolate references an unknown node".to_owned(),
                    )
                })
            }
            ControlActionV1::Rejoin { node, .. } => {
                self.rejoin_node(node).then_some(()).ok_or_else(|| {
                    ControlApplyError::InvalidAction("rejoin references an unknown node".to_owned())
                })
            }
            ControlActionV1::Disable { node, .. } => {
                self.disable_node(node).then_some(()).ok_or_else(|| {
                    ControlApplyError::InvalidAction(
                        "disable references an unknown node".to_owned(),
                    )
                })
            }
            ControlActionV1::Enable { node, .. } => {
                self.enable_node(node).then_some(()).ok_or_else(|| {
                    ControlApplyError::InvalidAction("enable references an unknown node".to_owned())
                })
            }
            ControlActionV1::AddNode { .. } => {
                self.add_node();
                Ok(())
            }
        };
        if result.is_ok() {
            self.action_log.push(action);
        }
        result
    }

    /// Apply a replay script through the same control surface used by WASM and sandbox.
    pub fn apply_replay_script(
        &mut self,
        script: &ReplayScriptV1,
    ) -> Result<(), ControlApplyError> {
        script.validate()?;
        self.mode = script.mode;
        self.active_scenario = script.scenario.clone();
        for action in script.actions.iter().cloned() {
            if action.at_step() > self.steps {
                self.run(action.at_step() - self.steps);
            }
            self.apply_control_action(action)?;
        }
        Ok(())
    }

    /// Return the replay-visible action log accumulated by the shared control surface.
    pub fn action_log(&self) -> &[ControlActionV1] {
        &self.action_log
    }

    /// Build a replay script from the current world metadata and action log.
    pub fn replay_script(&self) -> ReplayScriptV1 {
        ReplayScriptV1 {
            version: crate::REPLAY_SCRIPT_VERSION,
            seed: self.seed,
            mode: self.mode,
            scenario: self.active_scenario.clone(),
            actions: self.action_log.clone(),
        }
    }

    /// Subscribe a manual-mode client to namespace cache events.
    pub fn subscribe(&mut self, client: impl Into<String>, namespace: impl Into<String>) {
        let client = client.into();
        let namespace = namespace.into();
        self.ensure_client_actor(&client, &namespace);
        let id = subscriber_id(&client, &namespace);
        self.subscribers
            .entry(id.clone())
            .or_insert_with(|| SimSubscriber {
                id,
                client_id: client,
                namespace,
                pending: VecDeque::new(),
                last_event: None,
                dropped: 0,
            });
    }

    /// Push a manual-mode cache event into the simulated cluster.
    pub fn push_event(
        &mut self,
        client: impl Into<String>,
        namespace: impl Into<String>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), ControlApplyError> {
        let client = client.into();
        let namespace = namespace.into();
        let key = namespaced_key(&namespace, &key.into());
        let value = value.into().into_bytes();
        let target = self.preferred_write_node().ok_or_else(|| {
            ControlApplyError::InvalidAction("no live node for push_event".to_owned())
        })?;
        let numeric_id = self.ensure_client_actor(&client, &namespace);
        let op = WorkloadOp::Put {
            key: key.clone(),
            value: value.clone(),
        };
        let event_id = self
            .history
            .record_invocation(numeric_id, op.clone(), self.clock.now());
        let ack = self
            .nodes
            .get_mut(&target)
            .expect("preferred write node must exist")
            .node
            .handle_client(client_op(op));
        self.accepted_ops = self.accepted_ops.saturating_add(1);
        self.history
            .record_response(event_id, self.clock.now(), workload_result(ack.clone()));
        let commit_index = self.history.completed().count() as u64;
        if let Some(actor) = self.clients.get_mut(&client) {
            actor.namespace = namespace.clone();
            actor.last_op = Some(format!("push {key}"));
            actor.in_flight = 0;
        }
        self.record(format!("manual_push:{client}:{target}:{key}:{event_id:?}"));
        self.drain_node_effects();
        let version = value_version(&value);
        self.enqueue_subscriber_events(&namespace, key, version, commit_index);
        self.deliver_subscriber_events();
        self.refresh_invariant_report();
        Ok(())
    }

    /// Crash a node if it exists.
    pub fn crash_node(&mut self, node_id: impl Into<ClusterNodeId>) -> bool {
        let node_id = node_id.into();
        if self.nodes.contains_key(&node_id) {
            self.crashed_nodes.insert(node_id);
            true
        } else {
            false
        }
    }

    /// Restart a crashed node if it exists.
    pub fn restart_node(&mut self, node_id: impl Into<ClusterNodeId>) -> bool {
        let node_id = node_id.into();
        let restarted = self.crashed_nodes.remove(&node_id);
        if restarted {
            self.catch_up_node(&node_id);
        }
        restarted
    }

    /// Isolate one node from all peers with symmetric partitions.
    pub fn isolate_node(&mut self, node_id: impl Into<ClusterNodeId>) -> bool {
        let node_id = node_id.into();
        if !self.nodes.contains_key(&node_id) {
            return false;
        }
        let others = self
            .nodes
            .keys()
            .filter(|peer| *peer != &node_id)
            .cloned()
            .collect::<Vec<_>>();
        self.network.partition(
            (std::slice::from_ref(&node_id), &others),
            PartitionSymmetry::Symmetric,
        );
        self.isolated_nodes.insert(node_id);
        true
    }

    /// Rejoin one isolated node and run deterministic catch-up.
    pub fn rejoin_node(&mut self, node_id: impl Into<ClusterNodeId>) -> bool {
        let node_id = node_id.into();
        if !self.nodes.contains_key(&node_id) {
            return false;
        }
        let others = self.nodes.keys().cloned().collect::<Vec<_>>();
        for other in others {
            if other != node_id {
                self.network.heal_link(&node_id, &other);
                self.network.heal_link(&other, &node_id);
            }
        }
        self.isolated_nodes.remove(&node_id);
        self.catch_up_node(&node_id);
        true
    }

    /// Disable one node without marking it crashed.
    pub fn disable_node(&mut self, node_id: impl Into<ClusterNodeId>) -> bool {
        let node_id = node_id.into();
        if self.nodes.contains_key(&node_id) {
            self.disabled_nodes.insert(node_id);
            true
        } else {
            false
        }
    }

    /// Enable one disabled node and run deterministic catch-up.
    pub fn enable_node(&mut self, node_id: impl Into<ClusterNodeId>) -> bool {
        let node_id = node_id.into();
        if !self.nodes.contains_key(&node_id) {
            return false;
        }
        let removed = self.disabled_nodes.remove(&node_id);
        self.catch_up_node(&node_id);
        removed
    }

    /// Add a deterministic simulator node.
    pub fn add_node(&mut self) -> ClusterNodeId {
        let node_id = ClusterNodeId::new(format!("node-{}", self.nodes.len()));
        let peers = self.nodes.keys().cloned().collect::<Vec<_>>();
        let node = ClusterNode::new(
            ClusterNodeConfig::new(node_id.clone(), peers)
                .heartbeat_interval(self.cfg.heartbeat_interval),
        );
        self.nodes.insert(
            node_id.clone(),
            SimNode {
                node,
                storage: SimStorage::new(),
            },
        );
        self.cfg.node_count = self.cfg.node_count.saturating_add(1);
        self.catch_up_node(&node_id);
        self.rebalance = Some(RebalanceState {
            phase: "complete".to_owned(),
            moved_partitions: self.history.completed().count() as u64,
            total_partitions: self.cfg.key_count,
        });
        node_id
    }

    /// Partition one directed link.
    pub fn partition_link(
        &mut self,
        from: impl Into<ClusterNodeId>,
        to: impl Into<ClusterNodeId>,
    ) -> bool {
        let from = from.into();
        let to = to.into();
        if self.nodes.contains_key(&from) && self.nodes.contains_key(&to) && from != to {
            self.network
                .partition((&[from], &[to]), PartitionSymmetry::LeftToRight);
            true
        } else {
            false
        }
    }

    /// Heal one directed link.
    pub fn heal_link(
        &mut self,
        from: impl Into<ClusterNodeId>,
        to: impl Into<ClusterNodeId>,
    ) -> bool {
        self.network.heal_link(&from.into(), &to.into())
    }

    /// Drop the next packet on one directed link.
    pub fn drop_next_on_link(
        &mut self,
        from: impl Into<ClusterNodeId>,
        to: impl Into<ClusterNodeId>,
    ) -> bool {
        self.inject_link_fault(from, to, LinkFault::Drop)
    }

    /// Delay the next packet on one directed link.
    pub fn delay_next_on_link(
        &mut self,
        from: impl Into<ClusterNodeId>,
        to: impl Into<ClusterNodeId>,
        delay: LogicalDuration,
    ) -> bool {
        self.inject_link_fault(from, to, LinkFault::Delay(delay))
    }

    /// Delay the next packet on one directed link by milliseconds.
    pub fn delay_next_on_link_millis(
        &mut self,
        from: impl Into<ClusterNodeId>,
        to: impl Into<ClusterNodeId>,
        delay_millis: u64,
    ) -> bool {
        self.delay_next_on_link(from, to, LogicalDuration::from_millis(delay_millis))
    }

    /// Return the canonical UI snapshot for the current simulator state.
    pub fn snapshot(&self) -> SimSnapshot {
        let node_ids = self.nodes.keys().cloned().collect::<Vec<_>>();
        let committed_entries = self.history.completed().count() as u64;
        let election = self.election.snapshot();
        let election_nodes = election
            .nodes
            .iter()
            .map(|node| (node.node_id.to_string(), node))
            .collect::<BTreeMap<_, _>>();
        let nodes = node_ids
            .iter()
            .map(|node_id| {
                crate::snapshot::node_view(
                    node_id.to_string(),
                    committed_entries,
                    committed_entries,
                    self.crashed_nodes.contains(node_id),
                    self.disabled_nodes.contains(node_id),
                    election_nodes.get(node_id.as_str()).copied(),
                )
            })
            .collect();
        let links = node_ids
            .iter()
            .flat_map(|from| {
                node_ids
                    .iter()
                    .filter(move |to| *to != from)
                    .map(move |to| {
                        crate::snapshot::link_view(
                            from.to_string(),
                            to.to_string(),
                            self.network.can_deliver(from, to),
                            self.network
                                .max_pending_delay(from, to, self.clock.now())
                                .map(|duration| duration.as_millis()),
                            self.network.in_flight_between(from, to),
                        )
                    })
            })
            .collect();
        let (in_flight, over_budget) = crate::snapshot::message_views_from_network_and_election(
            self.network.in_flight_messages(),
            election.signals.iter(),
            self.clock.now(),
        );
        let mut key_observations = BTreeMap::<String, BTreeMap<String, u64>>::new();
        for (node_id, sim_node) in &self.nodes {
            for (key, checksum) in sim_node.storage.visible_checksums() {
                key_observations
                    .entry(key)
                    .or_default()
                    .insert(node_id.to_string(), checksum);
            }
        }
        for replicas in key_observations.values_mut() {
            for node_id in &node_ids {
                replicas.entry(node_id.to_string()).or_default();
            }
        }
        let sync_progress = node_ids
            .iter()
            .map(|node_id| crate::snapshot::SyncProgressView {
                node_id: node_id.to_string(),
                applied_index: self
                    .nodes
                    .get(node_id)
                    .map(|node| node.storage.visible_checksums().len() as u64)
                    .unwrap_or_default(),
                leader_commit_index: committed_entries,
            })
            .collect();
        let rebalance = self
            .rebalance
            .as_ref()
            .map(|rebalance| crate::snapshot::RebalanceView {
                phase: rebalance.phase.clone(),
                moved_partitions: rebalance.moved_partitions,
                total_partitions: rebalance.total_partitions,
            });

        SimSnapshot {
            schema_version: crate::snapshot::SIM_SNAPSHOT_SCHEMA_VERSION,
            seed: self.seed,
            step: self.steps,
            logical_time_millis: crate::snapshot::logical_millis(self.clock.now()),
            formation_phase: election.phase.to_string(),
            election_source: election.source.as_str().to_owned(),
            election_disclosure: election.source.disclosure().to_owned(),
            nodes,
            links,
            in_flight,
            over_budget,
            keys: crate::snapshot::key_views_from_storage(key_observations),
            clients: self
                .clients
                .iter()
                .map(|(id, actor)| crate::snapshot::ClientView {
                    id: id.clone(),
                    namespace: actor.namespace.clone(),
                    last_op: actor.last_op.clone(),
                    in_flight: actor.in_flight,
                })
                .collect(),
            subscribers: self
                .subscribers
                .values()
                .map(|subscriber| crate::snapshot::SubscriberView {
                    id: subscriber.id.clone(),
                    client_id: subscriber.client_id.clone(),
                    namespace: subscriber.namespace.clone(),
                    last_event: subscriber.last_event.clone(),
                    lag: subscriber.pending.len() as u64,
                    dropped: subscriber.dropped,
                })
                .collect(),
            sync_progress,
            rebalance,
            mode: self.mode.as_str().to_owned(),
            active_scenario: self.active_scenario.clone(),
            intervention_count: self.action_log.len() as u64,
            verdict: crate::snapshot::VerdictView::from_report(&self.invariant_report),
            progress: crate::snapshot::progress_from_report(
                committed_entries,
                &self.invariant_report,
            ),
        }
    }

    /// Serialize the canonical UI snapshot as JSON.
    pub fn snapshot_json(&self) -> String {
        self.snapshot().to_json()
    }

    /// Serialize the canonical invariant verdict as JSON.
    pub fn verdict_json(&self) -> String {
        self.snapshot().verdict_json()
    }

    fn deliver_network(&mut self) {
        let delivered = self.network.deliverable(self.clock.now());
        self.delivered_messages = self
            .delivered_messages
            .saturating_add(delivered.len() as u64);
        for (from, to, message) in delivered {
            self.record(format!("deliver:{from}->{to}:{message:?}"));
            if self.crashed_nodes.contains(&to)
                || self.disabled_nodes.contains(&to)
                || self.isolated_nodes.contains(&to)
            {
                continue;
            }
            if let Some(target) = self.nodes.get_mut(&to) {
                target.node.handle_message(from, message);
            }
        }
    }

    fn drive_election(&mut self) {
        let live_nodes = self.live_node_ids();
        let previous_trace_len = self.election.trace().len();
        self.election.step(self.steps, &live_nodes);
        let new_events = self.election.trace()[previous_trace_len..].to_vec();
        for event in new_events {
            self.record(event);
        }
    }

    fn live_node_ids(&self) -> BTreeSet<ClusterNodeId> {
        self.nodes
            .keys()
            .filter(|node_id| {
                !self.crashed_nodes.contains(*node_id)
                    && !self.disabled_nodes.contains(*node_id)
                    && !self.isolated_nodes.contains(*node_id)
            })
            .cloned()
            .collect()
    }

    fn refresh_invariant_report(&mut self) {
        let mut election_state = crate::invariants::ElectionTopologyState::from_election_snapshot(
            &self.election.snapshot(),
        );
        for subscriber in self.subscribers.values() {
            if let Some(event) = &subscriber.last_event {
                election_state = election_state.subscriber_delivery(
                    crate::invariants::SubscriberDeliveryObservation {
                        subscriber_id: subscriber.id.clone(),
                        key: event.key.clone(),
                        commit_index: event.commit_index,
                        delivered_after_commit_index: self.history.completed().count() as u64,
                    },
                );
            }
        }
        self.invariant_report = self
            .invariant_checker
            .check_history_and_election(&self.history, &election_state);
    }

    fn tick_nodes(&mut self) {
        for (node_id, sim_node) in self.nodes.iter_mut() {
            if self.crashed_nodes.contains(node_id)
                || self.disabled_nodes.contains(node_id)
                || self.isolated_nodes.contains(node_id)
            {
                continue;
            }
            sim_node.node.tick(self.clock.now());
        }
    }

    fn issue_smoke_workload(&mut self) {
        if !self.workload_enabled {
            return;
        }
        let live_nodes = self.live_node_ids().into_iter().collect::<Vec<_>>();
        let node_count = live_nodes.len();
        if node_count == 0 {
            return;
        }
        let node_index = self.rng.next_index(node_count);
        let node_id = live_nodes[node_index].clone();
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
        for (node_id, sim_node) in self.nodes.iter_mut() {
            if self.crashed_nodes.contains(node_id)
                || self.disabled_nodes.contains(node_id)
                || self.isolated_nodes.contains(node_id)
            {
                continue;
            }
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

    fn preferred_write_node(&self) -> Option<ClusterNodeId> {
        let live = self.live_node_ids();
        self.election
            .snapshot()
            .leader
            .filter(|leader| live.contains(leader))
            .or_else(|| live.into_iter().next())
    }

    fn catch_up_node(&mut self, node_id: &ClusterNodeId) {
        if self.crashed_nodes.contains(node_id) || self.disabled_nodes.contains(node_id) {
            return;
        }
        let Some(source_id) = self
            .preferred_write_node()
            .filter(|source| source != node_id)
            .or_else(|| self.nodes.keys().find(|source| *source != node_id).cloned())
        else {
            return;
        };
        let Some(source_storage) = self.nodes.get(&source_id).map(|node| node.storage.clone())
        else {
            return;
        };
        if let Some(target) = self.nodes.get_mut(node_id) {
            target
                .storage
                .replace_visible_default_zone_from(&source_storage);
        }
        self.rebalance = Some(RebalanceState {
            phase: "complete".to_owned(),
            moved_partitions: source_storage.visible_checksums().len() as u64,
            total_partitions: self.cfg.key_count,
        });
    }

    fn ensure_client_actor(&mut self, client: &str, namespace: &str) -> u64 {
        if let Some(actor) = self.clients.get_mut(client) {
            actor.namespace = namespace.to_owned();
            return actor.numeric_id;
        }
        let numeric_id = self.next_client_numeric_id;
        self.next_client_numeric_id = self.next_client_numeric_id.saturating_add(1);
        self.clients.insert(
            client.to_owned(),
            SimClientActor {
                numeric_id,
                namespace: namespace.to_owned(),
                last_op: None,
                in_flight: 0,
            },
        );
        numeric_id
    }

    fn enqueue_subscriber_events(
        &mut self,
        namespace: &str,
        key: String,
        version: u64,
        commit_index: u64,
    ) {
        for subscriber in self
            .subscribers
            .values_mut()
            .filter(|subscriber| subscriber.namespace == namespace)
        {
            if subscriber.pending.len() >= MAX_SUBSCRIBER_BUFFER {
                subscriber.dropped = subscriber.dropped.saturating_add(1);
                continue;
            }
            subscriber.pending.push_back(PendingSubscriberEvent {
                kind: BUS_EVENT_UPSERTED.to_owned(),
                key: key.clone(),
                version,
                commit_index,
            });
        }
    }

    fn deliver_subscriber_events(&mut self) {
        let live_count = self.live_node_ids().len();
        let completed = self.history.completed().count() as u64;
        let storage_versions = self
            .nodes
            .iter()
            .filter(|(node_id, _)| {
                !self.crashed_nodes.contains(*node_id)
                    && !self.disabled_nodes.contains(*node_id)
                    && !self.isolated_nodes.contains(*node_id)
            })
            .map(|(node_id, sim_node)| (node_id.clone(), sim_node.storage.visible_checksums()))
            .collect::<Vec<_>>();
        for subscriber in self.subscribers.values_mut() {
            let Some(event) = subscriber.pending.front() else {
                continue;
            };
            if completed < event.commit_index {
                continue;
            }
            let matching = storage_versions
                .iter()
                .filter(|(_, values)| values.get(&event.key).copied() == Some(event.version))
                .count();
            if live_count > 0 && matching == live_count {
                let event = subscriber.pending.pop_front().expect("front event exists");
                subscriber.last_event = Some(SubscriberEventView {
                    kind: event.kind,
                    key: event.key,
                    version: event.version,
                    commit_index: event.commit_index,
                    delivered_at_step: self.steps,
                });
            }
        }
    }

    fn record(&mut self, event: String) {
        self.trace.push(event);
    }

    fn inject_link_fault(
        &mut self,
        from: impl Into<ClusterNodeId>,
        to: impl Into<ClusterNodeId>,
        fault: LinkFault,
    ) -> bool {
        let from = from.into();
        let to = to.into();
        if self.nodes.contains_key(&from) && self.nodes.contains_key(&to) && from != to {
            self.network.inject_link_fault(from, to, fault);
            true
        } else {
            false
        }
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

fn namespaced_key(namespace: &str, key: &str) -> String {
    format!("{namespace}:{key}")
}

fn subscriber_id(client: &str, namespace: &str) -> String {
    format!("{client}@{namespace}")
}

fn value_version(value: &[u8]) -> u64 {
    crate::storage::checksum(value)
}
