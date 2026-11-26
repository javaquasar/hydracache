use std::collections::{BTreeMap, BTreeSet, VecDeque};

use hydracache::{
    ClientAck, ClientOp, ClusterNode, ClusterNodeConfig, ClusterNodeId, ClusterNodeMessage,
    ConsistencyLevel, EffectiveReplicationMap, LogicalDuration, LogicalTime,
    OutboundClusterMessage, RegionId, Replicas, StorageOpKind,
};

use crate::replication::{
    grid_write_id, level_name, GridWrite, GridWriteStatus, RepairStats, SimHint,
    MAX_HINTS_PER_TARGET, REPAIR_INTERVAL_STEPS,
};
use crate::snapshot::SIM_REGION;
#[cfg(not(target_arch = "wasm32"))]
use crate::SimRaftCluster;
use crate::SimReplicationModel;
use crate::{
    BoundedGrowthChecker, ControlActionV1, ControlApplyError, ElectionDriver,
    ElectionDriverSnapshot, History, InvariantChecker, InvariantReport, LinkFault,
    PartitionSymmetry, ReplayScriptV1, ResourceBudget, ResourceSample, SimClock, SimNetwork,
    SimSnapshot, SimStorage, SubscriberEventView, WorkloadConfig, WorkloadGenerator, WorkloadOp,
    WorkloadResult, MAX_SUBSCRIBER_BUFFER,
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
    /// Write, invalidate and flush operations this node has applied to its
    /// storage (a catch-up copies the source's count with its state). The
    /// cluster replicates without a log, so this, not a commit index, is the
    /// node's own progress.
    applied_writes: u64,
    /// Step at which this node applied the value it now holds, per key: a
    /// write or invalidate on the node, or the catch-up that copied it in.
    applied_at: BTreeMap<String, u64>,
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
    /// Routed write this event waits for (delivered once it is acknowledged).
    write: Option<u64>,
}

/// A client process the cluster knows about, and the member it is connected to.
#[derive(Debug, Clone)]
struct ClientSession {
    /// Member the client's connection is open to. It coordinates the client's
    /// writes, the way a product member handles writes from its own clients.
    attached: Option<ClusterNodeId>,
    /// Step at which the simulated `ClientUpsert` committed. `None` for the
    /// built-in workload fleet, which the simulator treats as admitted with
    /// the cluster (disclosed in the Raft control-plane gap register).
    admitted_at_step: Option<u64>,
}

/// Product error text for a metadata proposal with no leader
/// (`hydracache-cluster-raft`, `commit_command`).
const NO_LEADER_PROPOSAL: &str = "no raft leader; retry metadata proposal after election";

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

/// Election backend used by [`SimWorld`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElectionBackend {
    /// Deterministic simulator model retained as the wasm/fallback path.
    Model,
    /// Real raft-rs driven synchronously over the simulator network.
    #[cfg(not(target_arch = "wasm32"))]
    Raft,
}

/// Deterministic whole-cluster simulation driver.
#[derive(Debug)]
pub struct SimWorld {
    seed: u64,
    cfg: SimConfig,
    clock: SimClock,
    network: SimNetwork,
    election_backend: ElectionBackend,
    election: ElectionDriver,
    #[cfg(not(target_arch = "wasm32"))]
    raft: Option<SimRaftCluster>,
    workload: WorkloadGenerator,
    history: History,
    invariant_checker: InvariantChecker,
    resource_growth_checker: BoundedGrowthChecker,
    invariant_report: InvariantReport,
    first_violation_step: Option<u64>,
    nodes: BTreeMap<ClusterNodeId, SimNode>,
    clients: BTreeMap<String, SimClientActor>,
    subscribers: BTreeMap<String, SimSubscriber>,
    sessions: BTreeMap<String, ClientSession>,
    replication: SimReplicationModel,
    grid_writes: BTreeMap<u64, GridWrite>,
    hints: Vec<SimHint>,
    /// Hint copies on their way to their target, by (write, target).
    hints_in_flight: BTreeSet<(u64, ClusterNodeId)>,
    /// Newest routed write each node applied per key: copies are versioned,
    /// so an older one never overwrites a newer value.
    applied_write: BTreeMap<(ClusterNodeId, String), u64>,
    repair_stats: RepairStats,
    next_grid_write: u64,
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
                    applied_writes: 0,
                    applied_at: BTreeMap::new(),
                },
            );
        }

        Self {
            seed,
            cfg,
            clock: SimClock::default(),
            network: SimNetwork::from_seed(seed ^ 0x44_44_44_44),
            election_backend: ElectionBackend::Model,
            election: ElectionDriver::new(seed ^ 0x53_00_00_00, node_ids.clone()),
            #[cfg(not(target_arch = "wasm32"))]
            raft: None,
            workload: WorkloadGenerator::new(
                seed ^ 0x55_55_55_55,
                WorkloadConfig {
                    key_count,
                    ..WorkloadConfig::default()
                },
            ),
            history: History::new(),
            invariant_checker: InvariantChecker,
            resource_growth_checker: BoundedGrowthChecker::default(),
            invariant_report: InvariantReport::default(),
            first_violation_step: None,
            nodes,
            clients: BTreeMap::new(),
            subscribers: BTreeMap::new(),
            sessions: BTreeMap::new(),
            replication: SimReplicationModel::default(),
            grid_writes: BTreeMap::new(),
            hints: Vec::new(),
            hints_in_flight: BTreeSet::new(),
            applied_write: BTreeMap::new(),
            repair_stats: RepairStats::default(),
            next_grid_write: 1,
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

    /// Build a world that uses real raft-rs for lab leader election.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_raft_election(seed: u64, cfg: SimConfig) -> Self {
        let mut world = Self::new(seed, cfg);
        let node_ids = world.nodes.keys().cloned().collect::<Vec<_>>();
        world.raft = Some(
            SimRaftCluster::new(seed ^ 0x53_01_00_00, node_ids)
                .expect("real raft election backend must initialize from simulator nodes"),
        );
        world.election_backend = ElectionBackend::Raft;
        world
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

        self.reattach_sessions();
        self.drive_election();
        self.deliver_network();
        self.tick_nodes();
        self.issue_smoke_workload();
        self.drain_node_effects();
        // After storage applied this step's deliveries, so a hint sees what
        // its target already holds.
        self.replay_hints();
        self.repair_pass();
        self.fail_unreachable_writes();
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

    /// Return the deterministic event trace recorded so far.
    pub fn trace(&self) -> &[String] {
        &self.trace
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
        match self.election_backend {
            ElectionBackend::Model => self.election.snapshot(),
            #[cfg(not(target_arch = "wasm32"))]
            ElectionBackend::Raft => self
                .raft
                .as_ref()
                .expect("raft backend must have a raft cluster")
                .snapshot(),
        }
    }

    /// Enable or disable the built-in smoke workload.
    ///
    /// # Panics
    ///
    /// Panics when enabling the workload under a routed replication model: the
    /// workload and its invariants run on the full-broadcast model only.
    pub fn set_workload_enabled(&mut self, enabled: bool) {
        assert!(
            !(enabled && self.replication.is_routed()),
            "the built-in workload runs only in the full_broadcast replication model"
        );
        self.workload_enabled = enabled;
    }

    /// Choose the write path manual pushes follow. Fails loud while the
    /// built-in workload runs (it and its invariants use the full-broadcast
    /// model) and on a replication factor of zero.
    pub fn set_replication(&mut self, model: SimReplicationModel) -> Result<(), ControlApplyError> {
        if model.is_routed() && self.workload_enabled {
            return Err(ControlApplyError::InvalidAction(
                "turn the built-in workload off first: it runs only in the full_broadcast model"
                    .to_owned(),
            ));
        }
        if let SimReplicationModel::Replicated {
            replication_factor, ..
        } = model
        {
            if replication_factor == 0 {
                return Err(ControlApplyError::InvalidAction(
                    "replication_factor must be at least 1".to_owned(),
                ));
            }
        }
        self.replication = model;
        self.record(format!("replication:{}", model.as_str()));
        Ok(())
    }

    /// Return whether the built-in smoke workload is enabled.
    pub fn workload_enabled(&self) -> bool {
        self.workload_enabled
    }

    /// Replace the bounded-resource invariant budget and reset its sample window.
    pub fn set_resource_budget(&mut self, budget: ResourceBudget) {
        self.resource_growth_checker.set_budget(budget);
    }

    /// Apply one shared control action.
    pub fn apply_control_action(
        &mut self,
        action: ControlActionV1,
    ) -> Result<(), ControlApplyError> {
        // A workload toggle that does not change state is not an intervention;
        // keeping it out of the log keeps default runs shareable as plain seeds.
        if let ControlActionV1::Workload { enabled, .. } = &action {
            if *enabled == self.workload_enabled {
                return Ok(());
            }
        }
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
            ControlActionV1::Subscribe { client, ns, .. } => self.subscribe(client, ns),
            ControlActionV1::AddClient { client, ns, .. } => self.add_client(client, ns),
            ControlActionV1::Unsubscribe { client, ns, .. } => self.unsubscribe(client, ns),
            ControlActionV1::RemoveClient { client, .. } => self.remove_client(client),
            ControlActionV1::SetReplication { model, .. } => self.set_replication(model),
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
            ControlActionV1::Crash { node, .. } => {
                self.crash_node(node).then_some(()).ok_or_else(|| {
                    ControlApplyError::InvalidAction("crash references an unknown node".to_owned())
                })
            }
            ControlActionV1::Restart { node, .. } => {
                self.restart_node(node).then_some(()).ok_or_else(|| {
                    ControlApplyError::InvalidAction(
                        "restart references a node that is not crashed".to_owned(),
                    )
                })
            }
            ControlActionV1::Partition { from, to, .. } => {
                self.partition_link(from, to).then_some(()).ok_or_else(|| {
                    ControlApplyError::InvalidAction(
                        "partition references an unknown link".to_owned(),
                    )
                })
            }
            ControlActionV1::Heal { from, to, .. } => {
                self.heal_link(from, to).then_some(()).ok_or_else(|| {
                    ControlApplyError::InvalidAction(
                        "heal references a link that is not partitioned".to_owned(),
                    )
                })
            }
            ControlActionV1::DropNext { from, to, .. } => self
                .drop_next_on_link(from, to)
                .then_some(())
                .ok_or_else(|| {
                    ControlApplyError::InvalidAction("drop references an unknown link".to_owned())
                }),
            ControlActionV1::DelayNext {
                from, to, millis, ..
            } => self
                .delay_next_on_link_millis(from, to, millis)
                .then_some(())
                .ok_or_else(|| {
                    ControlApplyError::InvalidAction("delay references an unknown link".to_owned())
                }),
            ControlActionV1::Workload { enabled, .. } => {
                if enabled && self.replication.is_routed() {
                    Err(ControlApplyError::InvalidAction(
                        "the built-in workload runs only in the full_broadcast replication model"
                            .to_owned(),
                    ))
                } else {
                    self.set_workload_enabled(enabled);
                    Ok(())
                }
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
        if let Some(target_step) = script.target_step {
            if target_step < self.steps {
                return Err(ControlApplyError::InvalidAction(format!(
                    "replay target_step {target_step} is before step {} reached by its actions",
                    self.steps
                )));
            }
            self.run(target_step - self.steps);
        }
        Ok(())
    }

    /// Record the curated scenario this world was built from.
    pub(crate) fn set_active_scenario(&mut self, scenario: Option<String>) {
        self.active_scenario = scenario;
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
            target_step: Some(self.steps),
        }
    }

    /// Subscribe a manual-mode client to namespace cache events.
    ///
    /// Creates only the subscriber (a reader); a writer with the same name is
    /// registered by [`Self::add_client`] or by its first push. Subscribing an
    /// existing subscriber again is a no-op. A client name the cluster has not
    /// seen is admitted first, which fails loud while there is no leader.
    pub fn subscribe(
        &mut self,
        client: impl Into<String>,
        namespace: impl Into<String>,
    ) -> Result<(), ControlApplyError> {
        let client = client.into();
        let namespace = namespace.into();
        let id = subscriber_id(&client, &namespace);
        if self.subscribers.contains_key(&id) {
            return Ok(());
        }
        self.admit_client(&client)?;
        self.subscribers.insert(
            id.clone(),
            SimSubscriber {
                id,
                client_id: client,
                namespace,
                pending: VecDeque::new(),
                last_event: None,
                dropped: 0,
            },
        );
        Ok(())
    }

    /// Register a manual-mode writer client for a namespace without writing.
    ///
    /// Fails loud on an empty name or namespace and on a client that already
    /// exists, rather than silently re-pointing it.
    pub fn add_client(
        &mut self,
        client: impl Into<String>,
        namespace: impl Into<String>,
    ) -> Result<(), ControlApplyError> {
        let client = client.into();
        let namespace = namespace.into();
        if client.trim().is_empty() || namespace.trim().is_empty() {
            return Err(ControlApplyError::InvalidAction(
                "add_client needs a client name and a namespace".to_owned(),
            ));
        }
        if self.clients.contains_key(&client) {
            return Err(ControlApplyError::InvalidAction(format!(
                "client '{client}' already exists"
            )));
        }
        self.admit_client(&client)?;
        self.ensure_client_actor(&client, &namespace);
        self.record(format!("manual_add_client:{client}:{namespace}"));
        Ok(())
    }

    /// Remove a manual-mode subscriber; its undelivered events are dropped
    /// with it. Fails loud when the client is not subscribed to `namespace`.
    pub fn unsubscribe(
        &mut self,
        client: impl Into<String>,
        namespace: impl Into<String>,
    ) -> Result<(), ControlApplyError> {
        let client = client.into();
        let namespace = namespace.into();
        let id = subscriber_id(&client, &namespace);
        if self.subscribers.remove(&id).is_none() {
            return Err(ControlApplyError::InvalidAction(format!(
                "'{client}' is not subscribed to '{namespace}'"
            )));
        }
        self.record(format!("manual_unsubscribe:{client}:{namespace}"));
        Ok(())
    }

    /// Remove a manual-mode writer client. Its operations already recorded in
    /// the history stay (they happened); a later push under the same name
    /// registers a new writer. The client process stays admitted and connected
    /// (no `NodeLeft` is modeled). Fails loud when no such writer exists.
    pub fn remove_client(&mut self, client: impl Into<String>) -> Result<(), ControlApplyError> {
        let client = client.into();
        if self.clients.remove(&client).is_none() {
            return Err(ControlApplyError::InvalidAction(format!(
                "no writer named '{client}'"
            )));
        }
        self.record(format!("manual_remove_client:{client}"));
        Ok(())
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
        // The member the client is connected to handles the write, as in the
        // product; the Raft leader is not on the data path. A client the
        // cluster has not seen is admitted first (needs a leader).
        let target = self.admit_client(&client)?;
        if self.replication.is_routed() {
            return self.push_routed(client, namespace, key, value, target);
        }
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
        self.enqueue_subscriber_events(&namespace, key, version, commit_index, None);
        self.deliver_subscriber_events();
        self.refresh_invariant_report();
        Ok(())
    }

    /// A manual write under a routed model. The coordinator is the client's
    /// member. `LocalFirst` writes there and invalidates every other member;
    /// `Replicated` refuses the write when the product level cannot be met by
    /// the live owners, otherwise forwards it to the primary, which replicates
    /// to its backups. Acknowledgements come back as product `Ack` messages.
    fn push_routed(
        &mut self,
        client: String,
        namespace: String,
        key: String,
        value: Vec<u8>,
        coordinator: ClusterNodeId,
    ) -> Result<(), ControlApplyError> {
        let (replicas, level, required_acks) = match self.replication {
            SimReplicationModel::FullBroadcast => unreachable!("routed writes only"),
            SimReplicationModel::LocalFirst => {
                (Replicas::new(coordinator.clone(), Vec::new()), None, 1)
            }
            SimReplicationModel::Replicated {
                replication_factor,
                write_level,
            } => {
                let members = self.member_ids();
                let replicas = hydracache::rank_replicas(
                    &key,
                    members.iter(),
                    usize::try_from(replication_factor).unwrap_or(usize::MAX),
                )
                .ok_or_else(|| {
                    ControlApplyError::InvalidAction("no member can own the key".to_owned())
                })?;
                // A down primary is replaced by the first live owner in rank
                // order (backup promotion, without its phases), so a level
                // the live owners can meet is not stuck behind a dead primary.
                let live = self.live_node_ids();
                let replicas = match replicas
                    .all_nodes()
                    .into_iter()
                    .position(|node| live.contains(&node))
                {
                    Some(index) if index > 0 => {
                        let mut ranked = replicas.all_nodes();
                        let acting = ranked.remove(index);
                        Replicas::new(acting, ranked)
                    }
                    _ => replicas,
                };
                let requirement =
                    self.validate_level(write_level, &replicas)
                        .map_err(|reason| {
                            ControlApplyError::InvalidAction(format!(
                                "write refused at {}: {reason}",
                                level_name(write_level)
                            ))
                        })?;
                (
                    replicas,
                    Some(write_level),
                    requirement.required_total as u64,
                )
            }
        };
        let numeric_id = self.ensure_client_actor(&client, &namespace);
        let op = WorkloadOp::Put {
            key: key.clone(),
            value: value.clone(),
        };
        let id = self.next_grid_write;
        self.next_grid_write = self.next_grid_write.saturating_add(1);
        let write = GridWrite {
            id,
            client: client.clone(),
            key: key.clone(),
            value: value.clone(),
            coordinator: coordinator.clone(),
            replicas: replicas.clone(),
            level,
            required_acks,
            acks: 0,
            backups_applied: BTreeSet::new(),
            hinted: BTreeSet::new(),
            status: GridWriteStatus::Pending,
            started_at_step: self.steps,
            acknowledged_at_step: None,
        };
        let sequence = write.sequence();
        self.grid_writes.insert(id, write);
        self.trim_grid_writes();

        let event_id = self
            .history
            .record_invocation(numeric_id, op, self.clock.now());
        self.accepted_ops = self.accepted_ops.saturating_add(1);
        self.history.record_response(
            event_id,
            self.clock.now(),
            WorkloadResult::Accepted { sequence },
        );
        let commit_index = self.history.completed().count() as u64;
        if let Some(actor) = self.clients.get_mut(&client) {
            actor.namespace = namespace.clone();
            actor.last_op = Some(format!("push {key}"));
            actor.in_flight = 0;
        }
        self.record(format!(
            "manual_push:{client}:{coordinator}:{key}:{}:write={id}:primary={}",
            self.replication.as_str(),
            replicas.primary
        ));

        let put = ClusterNodeMessage::ReplicatePut {
            key: key.clone(),
            value: value.clone(),
            sequence,
        };
        if replicas.primary == coordinator {
            self.apply_at_primary(id, &coordinator);
        } else {
            // The coordinator is not an owner: forward to the primary.
            self.send_one(coordinator.clone(), replicas.primary.clone(), put);
        }
        if self.replication == SimReplicationModel::LocalFirst {
            for member in self.member_ids() {
                if member != coordinator {
                    self.send_one(
                        coordinator.clone(),
                        member,
                        ClusterNodeMessage::ReplicateInvalidate {
                            key: key.clone(),
                            sequence,
                        },
                    );
                }
            }
        }
        self.drain_node_effects();
        let version = value_version(&value);
        self.enqueue_subscriber_events(&namespace, key, version, commit_index, Some(id));
        self.deliver_subscriber_events();
        self.refresh_invariant_report();
        Ok(())
    }

    /// The primary applies a routed write it received (or took locally as the
    /// coordinator) and sends a copy to each backup.
    fn apply_at_primary(&mut self, id: u64, primary: &ClusterNodeId) {
        let Some(write) = self.grid_writes.get(&id) else {
            return;
        };
        let coordinator = write.coordinator.clone();
        let backups = write.replicas.backups.clone();
        let put = ClusterNodeMessage::ReplicatePut {
            key: write.key.clone(),
            value: write.value.clone(),
            sequence: write.sequence(),
        };
        if *primary == coordinator {
            // Local apply: the product node acknowledges to the sender, here
            // itself; `send_outbound` counts that acknowledgement directly.
            self.applied_write
                .insert((primary.clone(), write.key.clone()), id);
            if let Some(node) = self.nodes.get_mut(primary) {
                node.node.handle_message(coordinator.clone(), put.clone());
            }
        }
        for backup in backups {
            if self.reachable(primary, &backup) {
                self.send_one(primary.clone(), backup, put.clone());
            } else {
                self.store_hint(id, primary, backup);
            }
        }
    }

    /// Whether `from` can send to `to` now: both live and the link up. This
    /// is the primary's failure detector; the simulator's is perfect.
    fn reachable(&self, from: &ClusterNodeId, to: &ClusterNodeId) -> bool {
        let live = self.live_node_ids();
        live.contains(from) && live.contains(to) && self.network.can_deliver(from, to)
    }

    /// Keep a hint for a backup the primary cannot reach. The write was
    /// validated against the live owners, so this backup's acknowledgement
    /// is not required by the level (the product's `RequiredReplicaMiss`
    /// refusal happened before the write started).
    fn store_hint(&mut self, id: u64, holder: &ClusterNodeId, target: ClusterNodeId) {
        let Some(write) = self.grid_writes.get_mut(&id) else {
            return;
        };
        let held = self
            .hints
            .iter()
            .filter(|hint| hint.holder == *holder && hint.target == target)
            .count();
        if held >= MAX_HINTS_PER_TARGET {
            self.repair_stats.hints_dropped = self.repair_stats.hints_dropped.saturating_add(1);
            self.trace
                .push(format!("hint_dropped:{id}:{target}:over_budget"));
            return;
        }
        write.hinted.insert(target.clone());
        self.hints.push(SimHint {
            write: id,
            key: write.key.clone(),
            value: write.value.clone(),
            holder: holder.clone(),
            target: target.clone(),
            created_at_step: self.steps,
        });
        self.repair_stats.hints_stored = self.repair_stats.hints_stored.saturating_add(1);
        self.trace
            .push(format!("hint_stored:{id}:{holder}->{target}"));
    }

    /// Replay hints whose target is reachable again, in the order they were
    /// stored. A hint is lost with a holder that is down (the product hint
    /// store is in memory) and suppressed when the target already applied a
    /// newer write of the same key.
    fn replay_hints(&mut self) {
        if self.hints.is_empty() {
            return;
        }
        let live = self.live_node_ids();
        let hints = std::mem::take(&mut self.hints);
        let mut kept = Vec::new();
        for hint in hints {
            let holder_down = self.crashed_nodes.contains(&hint.holder)
                || self.disabled_nodes.contains(&hint.holder);
            if holder_down {
                self.repair_stats.hints_dropped = self.repair_stats.hints_dropped.saturating_add(1);
                self.record(format!(
                    "hint_dropped:{}:{}:holder_down",
                    hint.write, hint.target
                ));
                continue;
            }
            if !(live.contains(&hint.holder)
                && live.contains(&hint.target)
                && self.network.can_deliver(&hint.holder, &hint.target))
            {
                kept.push(hint);
                continue;
            }
            // Product `apply_hint`: a hint never overwrites a newer value the
            // target already holds.
            let current = self
                .nodes
                .get(&hint.target)
                .and_then(|node| node.storage.visible_checksums().get(&hint.key).copied());
            let newer = self.grid_writes.values().any(|write| {
                write.key == hint.key
                    && write.id > hint.write
                    && Some(value_version(&write.value)) == current
            });
            if newer {
                self.repair_stats.hints_suppressed =
                    self.repair_stats.hints_suppressed.saturating_add(1);
                self.record(format!("hint_suppressed:{}:{}", hint.write, hint.target));
                continue;
            }
            self.repair_stats.hints_replayed = self.repair_stats.hints_replayed.saturating_add(1);
            self.record(format!(
                "hint_replayed:{}:{}->{}",
                hint.write, hint.holder, hint.target
            ));
            let sequence = hint.sequence();
            if let Some(write) = self.grid_writes.get_mut(&hint.write) {
                write.hinted.remove(&hint.target);
            }
            self.hints_in_flight
                .insert((hint.write, hint.target.clone()));
            self.send_one(
                hint.holder,
                hint.target,
                ClusterNodeMessage::ReplicatePut {
                    key: hint.key,
                    value: hint.value,
                    sequence,
                },
            );
        }
        self.hints.extend(kept);
    }

    /// Anti-entropy, every `REPAIR_INTERVAL_STEPS`: for each key with a
    /// finished routed write, the acting primary copies the latest value to
    /// any live, reachable owner that holds something else and has no hint
    /// pending for it. This closes copies lost in flight and dropped hints. A
    /// model of the product's repair (merkle comparison is not simulated).
    fn repair_pass(&mut self) {
        if !self.replication.is_routed()
            || self.steps == 0
            || !self.steps.is_multiple_of(REPAIR_INTERVAL_STEPS)
        {
            return;
        }
        let mut latest = BTreeMap::<String, u64>::new();
        for write in self.grid_writes.values() {
            if write.status != GridWriteStatus::Pending {
                latest.insert(write.key.clone(), write.id);
            }
        }
        let mut copies = Vec::new();
        for id in latest.into_values() {
            let write = &self.grid_writes[&id];
            let version = value_version(&write.value);
            let holds = |node: &ClusterNodeId| {
                self.nodes
                    .get(node)
                    .and_then(|sim| sim.storage.visible_checksums().get(&write.key).copied())
                    == Some(version)
            };
            let source = write.replicas.primary.clone();
            if !holds(&source) {
                continue;
            }
            for target in write.replicas.backups.iter() {
                let hinted = self
                    .hints
                    .iter()
                    .any(|hint| hint.write == id && hint.target == *target);
                if !hinted && !holds(target) && self.reachable(&source, target) {
                    copies.push((
                        source.clone(),
                        target.clone(),
                        ClusterNodeMessage::ReplicatePut {
                            key: write.key.clone(),
                            value: write.value.clone(),
                            sequence: write.sequence(),
                        },
                    ));
                }
            }
        }
        self.repair_stats.last_repair_step = Some(self.steps);
        for (from, to, message) in copies {
            self.repair_stats.repair_copies = self.repair_stats.repair_copies.saturating_add(1);
            self.record(format!("repair_copy:{from}->{to}"));
            self.send_one(from, to, message);
        }
    }

    /// A product `Ack` for a routed write arrived at `at` from `from`.
    fn on_grid_ack(&mut self, at: &ClusterNodeId, from: &ClusterNodeId, id: u64) {
        let step = self.steps;
        let Some(write) = self.grid_writes.get_mut(&id) else {
            return;
        };
        let primary = write.replicas.primary.clone();
        let coordinator = write.coordinator.clone();
        if *at == primary && write.replicas.backups.contains(from) {
            write.backups_applied.insert(from.clone());
            if primary == coordinator {
                write.acknowledge(step);
            } else {
                // The primary passes the backup's acknowledgement on.
                let sequence = write.sequence();
                self.send_one(primary, coordinator, ClusterNodeMessage::Ack { sequence });
            }
        } else if *at == coordinator && *from == primary {
            // The primary's own apply, or a backup acknowledgement it relayed.
            write.acknowledge(step);
        }
    }

    /// A pending routed write whose level the live owners can no longer meet
    /// fails loud with the product's reason.
    fn fail_unreachable_writes(&mut self) {
        let pending = self
            .grid_writes
            .values()
            .filter(|write| write.status == GridWriteStatus::Pending)
            .filter_map(|write| {
                write
                    .level
                    .map(|level| (write.id, level, write.replicas.clone()))
            })
            .collect::<Vec<_>>();
        for (id, level, replicas) in pending {
            if let Err(reason) = self.validate_level(level, &replicas) {
                if let Some(write) = self.grid_writes.get_mut(&id) {
                    write.status = GridWriteStatus::Failed(reason);
                }
                self.record(format!("write_failed:{id}"));
            }
        }
    }

    /// Product acknowledgement rule for `replicas` against the live nodes,
    /// all in one region.
    fn validate_level(
        &self,
        level: ConsistencyLevel,
        replicas: &Replicas,
    ) -> Result<hydracache::AckRequirement, String> {
        let map = EffectiveReplicationMap::new(replicas.clone());
        let region = RegionId::new(SIM_REGION);
        let topology = replicas
            .all_nodes()
            .into_iter()
            .map(|node| (node, region.clone()))
            .collect::<BTreeMap<_, _>>();
        level
            .validate(&map, &topology, &region, &self.live_node_ids())
            .map_err(|error| error.reason)
    }

    /// Cluster members that can own keys: every node not disabled. A crashed
    /// or isolated member still owns its keys; that is what acknowledgements
    /// and failures are about.
    fn member_ids(&self) -> Vec<ClusterNodeId> {
        self.nodes
            .keys()
            .filter(|node| !self.disabled_nodes.contains(*node))
            .cloned()
            .collect()
    }

    fn trim_grid_writes(&mut self) {
        let finished = self
            .grid_writes
            .values()
            .filter(|write| write.status != GridWriteStatus::Pending)
            .map(|write| write.id)
            .collect::<Vec<_>>();
        let excess = finished
            .len()
            .saturating_sub(crate::replication::MAX_WRITES_RENDERED * 4);
        for id in finished.into_iter().take(excess) {
            self.grid_writes.remove(&id);
        }
    }

    fn send_one(&mut self, from: ClusterNodeId, to: ClusterNodeId, message: ClusterNodeMessage) {
        self.send_outbound(vec![OutboundClusterMessage { from, to, message }]);
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
            self.election.restore_node(&node_id, self.steps);
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(raft) = self.raft.as_mut() {
                raft.restore_node(&node_id);
            }
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
        self.election.restore_node(&node_id, self.steps);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(raft) = self.raft.as_mut() {
            raft.restore_node(&node_id);
        }
        true
    }

    /// Disable one node without marking it crashed. Disabling is an administrative
    /// decommission: the node leaves the voting set (quorum shrinks) so the rest of
    /// the cluster can still elect a leader — unlike a crash, which is a temporary
    /// outage that keeps the node a voting member.
    pub fn disable_node(&mut self, node_id: impl Into<ClusterNodeId>) -> bool {
        let node_id = node_id.into();
        if self.nodes.contains_key(&node_id) {
            self.election.remove_node(&node_id);
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(raft) = self.raft.as_mut() {
                raft.remove_node(&node_id, self.steps)
                    .expect("raft remove-node conf change must be proposed or deferred");
            }
            self.disabled_nodes.insert(node_id);
            true
        } else {
            false
        }
    }

    /// Enable one disabled node and run deterministic catch-up. The node rejoins the
    /// voting set as a fresh member and integrates on the next heartbeat.
    pub fn enable_node(&mut self, node_id: impl Into<ClusterNodeId>) -> bool {
        let node_id = node_id.into();
        if !self.nodes.contains_key(&node_id) {
            return false;
        }
        let removed = self.disabled_nodes.remove(&node_id);
        self.catch_up_node(&node_id);
        self.election.add_node(node_id.clone(), self.steps);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(raft) = self.raft.as_mut() {
            raft.add_node(node_id.clone(), self.steps)
                .expect("raft add-node conf change must be proposed or deferred");
        }
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
                applied_writes: 0,
                applied_at: BTreeMap::new(),
            },
        );
        self.cfg.node_count = self.cfg.node_count.saturating_add(1);
        self.election.add_node(node_id.clone(), self.steps);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(raft) = self.raft.as_mut() {
            raft.add_node(node_id.clone(), self.steps)
                .expect("raft add-node conf change must be proposed or deferred");
        }
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
        let election = self.election_snapshot();
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
                    self.nodes
                        .get(node_id)
                        .map_or(0, |node| node.applied_writes),
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
                            self.network.link_drops(from, to),
                        )
                    })
            })
            .collect();
        let (in_flight, over_budget) = crate::snapshot::message_views_from_network_and_election(
            self.network.in_flight_messages(),
            election.signals.iter(),
            self.clock.now(),
        );
        let mut key_observations = BTreeMap::<String, BTreeMap<String, (u64, Option<u64>)>>::new();
        for (node_id, sim_node) in &self.nodes {
            for (key, checksum) in sim_node.storage.visible_checksums() {
                let applied_at = sim_node.applied_at.get(&key).copied();
                key_observations
                    .entry(key)
                    .or_default()
                    .insert(node_id.to_string(), (checksum, applied_at));
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
            keys: self.with_owners(crate::snapshot::key_views_from_storage(key_observations)),
            clients: self
                .clients
                .iter()
                .map(|(id, actor)| crate::snapshot::ClientView {
                    id: id.clone(),
                    namespace: actor.namespace.clone(),
                    last_op: actor.last_op.clone(),
                    in_flight: actor.in_flight,
                    connected_node: self.session_node(id),
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
                    connected_node: self.session_node(&subscriber.client_id),
                })
                .collect(),
            replication: self.replication_view(),
            hints: self.hints.iter().map(SimHint::view).collect(),
            repair: crate::snapshot::RepairView {
                interval_steps: REPAIR_INTERVAL_STEPS,
                last_run_step: self.repair_stats.last_repair_step,
                hints_stored: self.repair_stats.hints_stored,
                hints_replayed: self.repair_stats.hints_replayed,
                hints_suppressed: self.repair_stats.hints_suppressed,
                hints_dropped: self.repair_stats.hints_dropped,
                repair_copies: self.repair_stats.repair_copies,
                disclosure: "simulator model of the product hinted-handoff rules and of anti-entropy repair; the product hint store and merkle repair are not compiled into the simulator".to_owned(),
            },
            writes: {
                let mut writes = self
                    .grid_writes
                    .values()
                    .rev()
                    .take(crate::replication::MAX_WRITES_RENDERED)
                    .map(GridWrite::view)
                    .collect::<Vec<_>>();
                writes.reverse();
                writes
            },
            admitted_clients: self
                .sessions
                .iter()
                .filter_map(|(id, session)| {
                    session.admitted_at_step.map(|admitted_at_step| {
                        crate::snapshot::AdmittedClientView {
                            id: id.clone(),
                            admitted_at_step,
                            connected_node: session
                                .attached
                                .as_ref()
                                .map(|node| node.as_str().to_owned()),
                        }
                    })
                })
                .collect(),
            sync_progress,
            rebalance,
            mode: self.mode.as_str().to_owned(),
            active_scenario: self.active_scenario.clone(),
            intervention_count: self.action_log.len() as u64,
            workload_enabled: self.workload_enabled,
            checked_invariants: Self::checked_invariants(),
            first_violation_step: self.first_violation_step,
            consistency: self.consistency_views(),
            verdict: crate::snapshot::VerdictView::from_report(&self.invariant_report),
            progress: crate::snapshot::progress_from_report(
                committed_entries,
                &self.invariant_report,
            ),
        }
    }

    /// Per-level satisfiability on this world's topology, computed by the
    /// product rules (`hydracache::ConsistencyLevel`, built for every target
    /// since TD-0014): every node is a replica in one region, and only live
    /// (not crashed, disabled, or isolated) nodes can acknowledge. Partitioned
    /// individual links are not considered.
    fn consistency_views(&self) -> Vec<crate::snapshot::ConsistencyLevelView> {
        let mut nodes = self.nodes.keys().cloned();
        let primary = nodes
            .next()
            .expect("a simulated world always has at least one node");
        let map = EffectiveReplicationMap::new(Replicas::new(primary, nodes.collect()));
        let region = RegionId::new(SIM_REGION);
        let topology = self
            .nodes
            .keys()
            .map(|node| (node.clone(), region.clone()))
            .collect::<BTreeMap<_, _>>();
        let live = self
            .live_node_ids()
            .into_iter()
            .filter(|node_id| self.nodes.contains_key(node_id))
            .collect::<BTreeSet<_>>();
        crate::replication::ALL_LEVELS
            .into_iter()
            .map(|level| {
                let requirement = level.required_acks(&map, &topology, &region);
                let verdict = level.validate(&map, &topology, &region, &live);
                crate::snapshot::ConsistencyLevelView {
                    level: level_name(level).to_owned(),
                    total_replicas: requirement.total_replicas as u64,
                    required_acks: requirement.required_total as u64,
                    live_replicas: requirement
                        .replicas
                        .iter()
                        .filter(|node| live.contains(*node))
                        .count() as u64,
                    satisfiable: verdict.is_ok(),
                    reason: verdict.err().map(|error| error.reason),
                }
            })
            .collect()
    }

    /// Serialize the canonical UI snapshot as JSON.
    pub fn snapshot_json(&self) -> String {
        self.snapshot().to_json()
    }

    fn replication_view(&self) -> crate::snapshot::ReplicationView {
        let (replication_factor, write_level, disclosure) = match self.replication {
            SimReplicationModel::FullBroadcast => (
                None,
                None,
                "simulator model: the client's member broadcasts every write to every member; not the product's write path",
            ),
            SimReplicationModel::LocalFirst => (
                None,
                None,
                "product default: the client's member writes locally and the other members drop their copy",
            ),
            SimReplicationModel::Replicated {
                replication_factor,
                write_level,
            } => (
                Some(replication_factor),
                Some(level_name(write_level).to_owned()),
                "model of the opt-in replication primitives (rendezvous owners, consistency levels); the product has no runtime replicated write path yet",
            ),
        };
        crate::snapshot::ReplicationView {
            model: self.replication.as_str().to_owned(),
            replication_factor,
            write_level,
            disclosure: disclosure.to_owned(),
        }
    }

    fn with_owners(
        &self,
        mut keys: Vec<crate::snapshot::KeyView>,
    ) -> Vec<crate::snapshot::KeyView> {
        if let SimReplicationModel::Replicated {
            replication_factor, ..
        } = self.replication
        {
            let members = self.member_ids();
            let rf = usize::try_from(replication_factor).unwrap_or(usize::MAX);
            for key in &mut keys {
                key.owners =
                    hydracache::rank_replicas(&key.key, members.iter(), rf).map(|replicas| {
                        crate::snapshot::KeyOwnersView {
                            primary: replicas.primary.to_string(),
                            backups: replicas.backups.iter().map(ToString::to_string).collect(),
                        }
                    });
            }
        }
        keys
    }

    /// Member a client's connection is open to, as the snapshot reports it.
    fn session_node(&self, client: &str) -> Option<String> {
        self.sessions
            .get(client)
            .and_then(|session| session.attached.as_ref())
            .map(|node| node.as_str().to_owned())
    }

    /// Pick a live member for a client connection: a stable hash of the client
    /// name over the live members, so a seed always reproduces the same
    /// routing. `self.nodes` is a `BTreeMap`, so the candidate order is stable.
    fn pick_member(&self, client: &str) -> Option<ClusterNodeId> {
        let live = self.live_node_ids().into_iter().collect::<Vec<_>>();
        if live.is_empty() {
            return None;
        }
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in client.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        Some(live[(hash % live.len() as u64) as usize].clone())
    }

    /// The member a client writes through, reconnecting it first when its
    /// member is down. A workload client is connected on first use.
    fn attached_node(&mut self, client: &str) -> Option<ClusterNodeId> {
        let live = self.live_node_ids();
        if let Some(node) = self
            .sessions
            .get(client)
            .and_then(|session| session.attached.clone())
            .filter(|node| live.contains(node))
        {
            return Some(node);
        }
        let node = self.pick_member(client);
        let session = self
            .sessions
            .entry(client.to_owned())
            .or_insert(ClientSession {
                attached: None,
                admitted_at_step: None,
            });
        let previous = std::mem::replace(&mut session.attached, node.clone());
        if previous != node {
            self.record(format!(
                "client_connect:{client}:{}->{}",
                previous.as_ref().map_or("-", |node| node.as_str()),
                node.as_ref().map_or("-", |node| node.as_str())
            ));
        }
        node
    }

    /// Reconnect every client whose member went down (crashed, disabled or
    /// isolated) to a live one. Reconnecting is not a metadata change: the
    /// client's `ClientUpsert` is already committed.
    fn reattach_sessions(&mut self) {
        let live = self.live_node_ids();
        let stale = self
            .sessions
            .iter()
            .filter(|(_, session)| {
                session
                    .attached
                    .as_ref()
                    .is_none_or(|node| !live.contains(node))
            })
            .map(|(client, _)| client.clone())
            .collect::<Vec<_>>();
        for client in stale {
            self.attached_node(&client);
        }
    }

    /// Admit a client through the simulated Raft metadata path and connect it,
    /// returning the member that coordinates its writes. A client already
    /// admitted only reconnects if its member is down. A new client needs a
    /// live leader to commit its `ClientUpsert`; without one this fails loud
    /// with the product's no-leader error instead of falling back.
    fn admit_client(&mut self, client: &str) -> Result<ClusterNodeId, ControlApplyError> {
        if !self.sessions.contains_key(client) {
            let live = self.live_node_ids();
            let leader = self
                .election_snapshot()
                .leader
                .filter(|leader| live.contains(leader))
                .ok_or_else(|| {
                    ControlApplyError::InvalidAction(format!(
                        "cannot admit client '{client}': {NO_LEADER_PROPOSAL}"
                    ))
                })?;
            let step = self.steps;
            self.sessions.insert(
                client.to_owned(),
                ClientSession {
                    attached: None,
                    admitted_at_step: Some(step),
                },
            );
            self.record(format!(
                "client_upsert:{client}:leader={leader}:step={step}"
            ));
        }
        self.attached_node(client).ok_or_else(|| {
            ControlApplyError::InvalidAction(format!("no live member for client '{client}'"))
        })
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
                self.network.record_target_down(&from, &to);
                continue;
            }
            if self.drop_stale_copy(&from, &to, &message) {
                continue;
            }
            let routed = self.routed_delivery(&from, &to, &message);
            if let Some(target) = self.nodes.get_mut(&to) {
                target.node.handle_message(from.clone(), message);
            }
            match routed {
                Some(RoutedDelivery::Forward(id)) => self.apply_at_primary(id, &to),
                Some(RoutedDelivery::Ack(id)) => self.on_grid_ack(&to, &from, id),
                None => {}
            }
        }
    }

    /// Versioned apply for routed copies and invalidations (the product's
    /// replicated records merge by version): a copy of an older write than the
    /// one `to` already applied for the key is acknowledged but not applied. A
    /// suppressed hint replay is counted, as the product `apply_hint` does.
    fn drop_stale_copy(
        &mut self,
        from: &ClusterNodeId,
        to: &ClusterNodeId,
        message: &ClusterNodeMessage,
    ) -> bool {
        let (key, sequence) = match message {
            ClusterNodeMessage::ReplicatePut { key, sequence, .. }
            | ClusterNodeMessage::ReplicateInvalidate { key, sequence } => (key, *sequence),
            _ => return false,
        };
        let Some(id) = grid_write_id(sequence) else {
            return false;
        };
        let was_hint = self.hints_in_flight.remove(&(id, to.clone()));
        let slot = (to.clone(), key.clone());
        if self
            .applied_write
            .get(&slot)
            .is_some_and(|newest| *newest > id)
        {
            if was_hint {
                self.repair_stats.hints_suppressed =
                    self.repair_stats.hints_suppressed.saturating_add(1);
            }
            self.record(format!("stale_copy_skipped:{id}:{to}"));
            self.send_one(
                to.clone(),
                from.clone(),
                ClusterNodeMessage::Ack { sequence },
            );
            return true;
        }
        self.applied_write.insert(slot, id);
        false
    }

    /// Whether a delivered message is a step of a routed write: the forward to
    /// the primary, or an acknowledgement.
    fn routed_delivery(
        &self,
        from: &ClusterNodeId,
        to: &ClusterNodeId,
        message: &ClusterNodeMessage,
    ) -> Option<RoutedDelivery> {
        match message {
            ClusterNodeMessage::ReplicatePut { sequence, .. } => {
                let id = grid_write_id(*sequence)?;
                let write = self.grid_writes.get(&id)?;
                (*from == write.coordinator && *to == write.replicas.primary)
                    .then_some(RoutedDelivery::Forward(id))
            }
            ClusterNodeMessage::Ack { sequence } => {
                grid_write_id(*sequence).map(RoutedDelivery::Ack)
            }
            _ => None,
        }
    }

    fn drive_election(&mut self) {
        let live_nodes = self.live_node_ids();
        match self.election_backend {
            ElectionBackend::Model => {
                let previous_trace_len = self.election.trace().len();
                // The model sees the same partitions the network enforces, so
                // its signals and quorum never cross a cut link.
                let blocked = live_nodes
                    .iter()
                    .flat_map(|from| live_nodes.iter().map(move |to| (from, to)))
                    .filter(|(from, to)| from != to && !self.network.can_deliver(from, to))
                    .map(|(from, to)| (from.clone(), to.clone()))
                    .collect();
                self.election
                    .step_with_partitions(self.steps, &live_nodes, &blocked);
                let new_events = self.election.trace()[previous_trace_len..].to_vec();
                for event in new_events {
                    self.record(event);
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            ElectionBackend::Raft => {
                let raft = self
                    .raft
                    .as_mut()
                    .expect("raft backend must have a raft cluster");
                let previous_trace_len = raft.trace().len();
                raft.step(self.steps, &live_nodes, &self.network)
                    .expect("real raft election step must stay deterministic");
                let new_events = raft.trace()[previous_trace_len..].to_vec();
                for event in new_events {
                    self.record(event);
                }
            }
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
            &self.election_snapshot(),
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
        let sample = self.resource_sample();
        self.resource_growth_checker
            .observe(sample, &mut self.invariant_report);
        if self.first_violation_step.is_none() && !self.invariant_report.violations.is_empty() {
            self.first_violation_step = Some(self.steps);
        }
    }

    /// Invariant names the live verdict is computed from: workload history,
    /// election/topology, and bounded resource growth. Replica-snapshot
    /// invariants ([`InvariantChecker::REPLICA_INVARIANTS`]) are not part of it.
    pub fn checked_invariants() -> Vec<String> {
        InvariantChecker::HISTORY_INVARIANTS
            .iter()
            .chain(InvariantChecker::ELECTION_INVARIANTS)
            .chain(["resource_bounded_growth"].iter())
            .map(|name| (*name).to_owned())
            .collect()
    }

    fn resource_sample(&self) -> ResourceSample {
        ResourceSample {
            storage_bytes: self
                .nodes
                .values()
                .map(|node| node.storage.footprint().tracked_bytes())
                .sum(),
            network_in_flight: self.network.in_flight_len() as u64,
            client_in_flight: self
                .clients
                .values()
                .map(|client| u64::from(client.in_flight))
                .sum(),
            subscriber_pending: self
                .subscribers
                .values()
                .map(|subscriber| subscriber.pending.len() as u64)
                .sum(),
        }
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
        if self.live_node_ids().is_empty() {
            return;
        }
        let (client, op) = self.workload.next_invocation();
        let Some(node_id) = self.attached_node(&workload_session_id(client)) else {
            return;
        };
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
        let step = self.steps;
        for (node_id, sim_node) in self.nodes.iter_mut() {
            if self.crashed_nodes.contains(node_id)
                || self.disabled_nodes.contains(node_id)
                || self.isolated_nodes.contains(node_id)
            {
                continue;
            }
            outbound.extend(sim_node.node.take_outbound());
            for request in sim_node.node.storage_requests() {
                match &request.kind {
                    StorageOpKind::Read { .. } => {}
                    StorageOpKind::Write { key, .. } | StorageOpKind::Delete { key } => {
                        sim_node.applied_writes = sim_node.applied_writes.saturating_add(1);
                        sim_node.applied_at.insert(key.clone(), step);
                    }
                    StorageOpKind::Flush => {
                        sim_node.applied_writes = sim_node.applied_writes.saturating_add(1);
                        for applied in sim_node.applied_at.values_mut() {
                            *applied = step;
                        }
                    }
                }
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
            if outbound.from == outbound.to {
                // A node acknowledging its own local apply of a routed write.
                if let ClusterNodeMessage::Ack { sequence } = &outbound.message {
                    if let Some(id) = grid_write_id(*sequence) {
                        self.on_grid_ack(&outbound.to, &outbound.from, id);
                    }
                }
                continue;
            }
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

    /// Source for copying state to a restarted or rejoining node: the live
    /// leader, else the first live node. This models state transfer only; it
    /// is not the write path (writes go through the client's member).
    fn catch_up_source(&self) -> Option<ClusterNodeId> {
        let live = self.live_node_ids();
        self.election_snapshot()
            .leader
            .filter(|leader| live.contains(leader))
            .or_else(|| live.into_iter().next())
    }

    fn catch_up_node(&mut self, node_id: &ClusterNodeId) {
        if self.crashed_nodes.contains(node_id) || self.disabled_nodes.contains(node_id) {
            return;
        }
        let Some(source_id) = self
            .catch_up_source()
            .filter(|source| source != node_id)
            .or_else(|| self.nodes.keys().find(|source| *source != node_id).cloned())
        else {
            return;
        };
        let step = self.steps;
        let Some((source_storage, source_applied, source_keys)) =
            self.nodes.get(&source_id).map(|node| {
                (
                    node.storage.clone(),
                    node.applied_writes,
                    node.applied_at.keys().cloned().collect::<Vec<_>>(),
                )
            })
        else {
            return;
        };
        if let Some(target) = self.nodes.get_mut(node_id) {
            target
                .storage
                .replace_visible_default_zone_from(&source_storage);
            // The node now holds the source's state, so it has the source's
            // progress too; every copied value arrived now.
            target.applied_writes = source_applied;
            target.applied_at = source_keys.into_iter().map(|key| (key, step)).collect();
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
        write: Option<u64>,
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
                write,
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
            if let Some(id) = event.write {
                // A routed write reaches its subscribers once acknowledged.
                let acknowledged = self
                    .grid_writes
                    .get(&id)
                    .is_some_and(|write| write.status == GridWriteStatus::Acknowledged);
                if acknowledged {
                    let event = subscriber.pending.pop_front().expect("front event exists");
                    subscriber.last_event = Some(SubscriberEventView {
                        kind: event.kind,
                        key: event.key,
                        version: event.version,
                        commit_index: event.commit_index,
                        delivered_at_step: self.steps,
                    });
                }
                continue;
            }
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

/// A delivered message that moves a routed write on.
enum RoutedDelivery {
    /// The coordinator's forward reached the primary.
    Forward(u64),
    /// An acknowledgement for the write arrived.
    Ack(u64),
}

/// Session name of a built-in workload client.
fn workload_session_id(client: u64) -> String {
    format!("workload:{client}")
}

fn subscriber_id(client: &str, namespace: &str) -> String {
    format!("{client}@{namespace}")
}

fn value_version(value: &[u8]) -> u64 {
    crate::storage::checksum(value)
}
