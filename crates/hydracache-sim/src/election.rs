use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use hydracache::ClusterNodeId;

/// Cluster-wide formation phase rendered and driven by the simulator FSM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum FormationPhase {
    /// No membership or leader formation has started.
    Unformed,
    /// Nodes are discovering peers and preparing to vote.
    Bootstrapping,
    /// At least one node is trying to form a quorum.
    Electing,
    /// The cluster has a quorum-backed leader.
    Formed,
    /// The cluster lost capacity or connectivity but can still recover.
    Degraded,
    /// A node is rejoining and replaying missed state.
    CatchingUp,
    /// Membership is changing after a scale-out action.
    Rebalancing,
}

impl FormationPhase {
    /// Stable iteration order used by table-driven tests.
    pub const ALL: [Self; 7] = [
        Self::Unformed,
        Self::Bootstrapping,
        Self::Electing,
        Self::Formed,
        Self::Degraded,
        Self::CatchingUp,
        Self::Rebalancing,
    ];

    const fn as_index(self) -> usize {
        self as usize
    }
}

impl fmt::Display for FormationPhase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unformed => "unformed",
            Self::Bootstrapping => "bootstrapping",
            Self::Electing => "electing",
            Self::Formed => "formed",
            Self::Degraded => "degraded",
            Self::CatchingUp => "catching_up",
            Self::Rebalancing => "rebalancing",
        })
    }
}

/// Per-node election role used by the simulator FSM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum NodeFsmState {
    /// Node is not connected to the modeled formation.
    Disconnected,
    /// Node is joining and discovering peers.
    Joining,
    /// Node follows a quorum-backed leader.
    Follower,
    /// Node is requesting votes for a new term.
    Candidate,
    /// Node is the quorum-backed leader.
    Leader,
    /// Node is replaying missed commits before becoming a follower.
    CatchingUp,
    /// Node is administratively disabled.
    Disabled,
}

impl NodeFsmState {
    /// Stable iteration order used by table-driven tests.
    pub const ALL: [Self; 7] = [
        Self::Disconnected,
        Self::Joining,
        Self::Follower,
        Self::Candidate,
        Self::Leader,
        Self::CatchingUp,
        Self::Disabled,
    ];

    const fn as_index(self) -> usize {
        self as usize
    }
}

impl fmt::Display for NodeFsmState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Disconnected => "disconnected",
            Self::Joining => "joining",
            Self::Follower => "follower",
            Self::Candidate => "candidate",
            Self::Leader => "leader",
            Self::CatchingUp => "catching_up",
            Self::Disabled => "disabled",
        })
    }
}

/// Closed set of inputs accepted by the formation/election FSM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ClusterFsmEvent {
    /// Start a fresh simulator formation.
    Boot,
    /// A node joins an existing formation.
    Join,
    /// A logical election timeout elapsed.
    ElectionTimeout,
    /// A candidate observed enough votes for quorum.
    VoteQuorum,
    /// A leader heartbeat was observed.
    LeaderHeartbeat,
    /// The current leader became unavailable.
    LeaderLost,
    /// Connectivity to a node or quorum was removed.
    Isolate,
    /// Connectivity was restored.
    Rejoin,
    /// A node was administratively disabled.
    Disable,
    /// A disabled node was administratively enabled.
    Enable,
    /// A rejoining node finished catch-up.
    CatchUpComplete,
    /// A new node was added to the topology.
    AddNode,
    /// Rebalance work completed.
    RebalanceComplete,
}

impl ClusterFsmEvent {
    /// Stable iteration order used by table-driven tests.
    pub const ALL: [Self; 13] = [
        Self::Boot,
        Self::Join,
        Self::ElectionTimeout,
        Self::VoteQuorum,
        Self::LeaderHeartbeat,
        Self::LeaderLost,
        Self::Isolate,
        Self::Rejoin,
        Self::Disable,
        Self::Enable,
        Self::CatchUpComplete,
        Self::AddNode,
        Self::RebalanceComplete,
    ];

    const fn as_index(self) -> usize {
        self as usize
    }
}

/// Single side effect emitted by a table transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterFsmAction {
    /// No external effect is needed.
    None,
    /// Discover peers before voting.
    DiscoverPeers,
    /// Start or restart a deterministic election round.
    StartElection,
    /// Record that a quorum-backed leader exists.
    BecomeLeader,
    /// Step down after observing a legitimate leader.
    ObserveLeader,
    /// Start catch-up for a rejoining node.
    StartCatchUp,
    /// Record that catch-up finished.
    FinishCatchUp,
    /// Mark quorum/capacity as degraded.
    MarkDegraded,
    /// Start deterministic rebalance work.
    StartRebalance,
    /// Finish deterministic rebalance work.
    FinishRebalance,
}

/// One explicit table cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FsmTransition<S> {
    /// Next state after applying the event.
    pub next: S,
    /// Side effect emitted by the transition.
    pub action: ClusterFsmAction,
}

impl<S> FsmTransition<S> {
    const fn new(next: S, action: ClusterFsmAction) -> Self {
        Self { next, action }
    }
}

type NodeTable =
    [[FsmTransition<NodeFsmState>; ClusterFsmEvent::ALL.len()]; NodeFsmState::ALL.len()];
type ClusterTable =
    [[FsmTransition<FormationPhase>; ClusterFsmEvent::ALL.len()]; FormationPhase::ALL.len()];

const N: ClusterFsmAction = ClusterFsmAction::None;
const DISCOVER: ClusterFsmAction = ClusterFsmAction::DiscoverPeers;
const ELECT: ClusterFsmAction = ClusterFsmAction::StartElection;
const LEAD: ClusterFsmAction = ClusterFsmAction::BecomeLeader;
const FOLLOW: ClusterFsmAction = ClusterFsmAction::ObserveLeader;
const CATCH_UP: ClusterFsmAction = ClusterFsmAction::StartCatchUp;
const CAUGHT_UP: ClusterFsmAction = ClusterFsmAction::FinishCatchUp;
const DEGRADED: ClusterFsmAction = ClusterFsmAction::MarkDegraded;
const REBALANCE: ClusterFsmAction = ClusterFsmAction::StartRebalance;
const REBALANCED: ClusterFsmAction = ClusterFsmAction::FinishRebalance;

const fn tx<S>(next: S, action: ClusterFsmAction) -> FsmTransition<S> {
    FsmTransition::new(next, action)
}

/// Total node FSM table: row = [`NodeFsmState`], column = [`ClusterFsmEvent`].
pub const NODE_TRANSITION_TABLE: NodeTable = [
    // Disconnected
    [
        tx(NodeFsmState::Joining, DISCOVER),
        tx(NodeFsmState::Joining, DISCOVER),
        tx(NodeFsmState::Disconnected, N),
        tx(NodeFsmState::Disconnected, N),
        tx(NodeFsmState::Disconnected, N),
        tx(NodeFsmState::Disconnected, N),
        tx(NodeFsmState::Disconnected, N),
        tx(NodeFsmState::Joining, DISCOVER),
        tx(NodeFsmState::Disabled, N),
        tx(NodeFsmState::Joining, DISCOVER),
        tx(NodeFsmState::Disconnected, N),
        tx(NodeFsmState::Joining, DISCOVER),
        tx(NodeFsmState::Disconnected, N),
    ],
    // Joining
    [
        tx(NodeFsmState::Joining, DISCOVER),
        tx(NodeFsmState::Joining, DISCOVER),
        tx(NodeFsmState::Candidate, ELECT),
        tx(NodeFsmState::Leader, LEAD),
        tx(NodeFsmState::Follower, FOLLOW),
        tx(NodeFsmState::Candidate, ELECT),
        tx(NodeFsmState::Disconnected, DEGRADED),
        tx(NodeFsmState::Joining, DISCOVER),
        tx(NodeFsmState::Disabled, N),
        tx(NodeFsmState::Joining, DISCOVER),
        tx(NodeFsmState::Follower, CAUGHT_UP),
        tx(NodeFsmState::Joining, DISCOVER),
        tx(NodeFsmState::Joining, N),
    ],
    // Follower
    [
        tx(NodeFsmState::Follower, N),
        tx(NodeFsmState::Follower, N),
        tx(NodeFsmState::Candidate, ELECT),
        tx(NodeFsmState::Leader, LEAD),
        tx(NodeFsmState::Follower, FOLLOW),
        tx(NodeFsmState::Candidate, ELECT),
        tx(NodeFsmState::Disconnected, DEGRADED),
        tx(NodeFsmState::CatchingUp, CATCH_UP),
        tx(NodeFsmState::Disabled, N),
        tx(NodeFsmState::Follower, N),
        tx(NodeFsmState::Follower, N),
        tx(NodeFsmState::CatchingUp, CATCH_UP),
        tx(NodeFsmState::Follower, N),
    ],
    // Candidate
    [
        tx(NodeFsmState::Candidate, N),
        tx(NodeFsmState::Candidate, N),
        tx(NodeFsmState::Candidate, ELECT),
        tx(NodeFsmState::Leader, LEAD),
        tx(NodeFsmState::Follower, FOLLOW),
        tx(NodeFsmState::Candidate, ELECT),
        tx(NodeFsmState::Disconnected, DEGRADED),
        tx(NodeFsmState::CatchingUp, CATCH_UP),
        tx(NodeFsmState::Disabled, N),
        tx(NodeFsmState::Candidate, N),
        tx(NodeFsmState::Follower, CAUGHT_UP),
        tx(NodeFsmState::Candidate, N),
        tx(NodeFsmState::Candidate, N),
    ],
    // Leader
    [
        tx(NodeFsmState::Leader, N),
        tx(NodeFsmState::Leader, N),
        tx(NodeFsmState::Leader, N),
        tx(NodeFsmState::Leader, N),
        tx(NodeFsmState::Leader, N),
        tx(NodeFsmState::Candidate, ELECT),
        tx(NodeFsmState::Disconnected, DEGRADED),
        tx(NodeFsmState::CatchingUp, CATCH_UP),
        tx(NodeFsmState::Disabled, N),
        tx(NodeFsmState::Leader, N),
        tx(NodeFsmState::Leader, N),
        tx(NodeFsmState::Leader, REBALANCE),
        tx(NodeFsmState::Leader, REBALANCED),
    ],
    // CatchingUp
    [
        tx(NodeFsmState::CatchingUp, CATCH_UP),
        tx(NodeFsmState::CatchingUp, CATCH_UP),
        tx(NodeFsmState::CatchingUp, N),
        tx(NodeFsmState::CatchingUp, N),
        tx(NodeFsmState::CatchingUp, FOLLOW),
        tx(NodeFsmState::Candidate, ELECT),
        tx(NodeFsmState::Disconnected, DEGRADED),
        tx(NodeFsmState::CatchingUp, CATCH_UP),
        tx(NodeFsmState::Disabled, N),
        tx(NodeFsmState::CatchingUp, CATCH_UP),
        tx(NodeFsmState::Follower, CAUGHT_UP),
        tx(NodeFsmState::CatchingUp, REBALANCE),
        tx(NodeFsmState::Follower, CAUGHT_UP),
    ],
    // Disabled
    [
        tx(NodeFsmState::Disabled, N),
        tx(NodeFsmState::Disabled, N),
        tx(NodeFsmState::Disabled, N),
        tx(NodeFsmState::Disabled, N),
        tx(NodeFsmState::Disabled, N),
        tx(NodeFsmState::Disabled, N),
        tx(NodeFsmState::Disabled, N),
        tx(NodeFsmState::CatchingUp, CATCH_UP),
        tx(NodeFsmState::Disabled, N),
        tx(NodeFsmState::CatchingUp, CATCH_UP),
        tx(NodeFsmState::Follower, CAUGHT_UP),
        tx(NodeFsmState::Disabled, N),
        tx(NodeFsmState::Disabled, N),
    ],
];

/// Total cluster formation FSM table: row = [`FormationPhase`], column = [`ClusterFsmEvent`].
pub const CLUSTER_TRANSITION_TABLE: ClusterTable = [
    // Unformed
    [
        tx(FormationPhase::Bootstrapping, DISCOVER),
        tx(FormationPhase::Bootstrapping, DISCOVER),
        tx(FormationPhase::Unformed, N),
        tx(FormationPhase::Unformed, N),
        tx(FormationPhase::Unformed, N),
        tx(FormationPhase::Unformed, N),
        tx(FormationPhase::Degraded, DEGRADED),
        tx(FormationPhase::Bootstrapping, DISCOVER),
        tx(FormationPhase::Degraded, DEGRADED),
        tx(FormationPhase::Bootstrapping, DISCOVER),
        tx(FormationPhase::Unformed, N),
        tx(FormationPhase::Rebalancing, REBALANCE),
        tx(FormationPhase::Unformed, N),
    ],
    // Bootstrapping
    [
        tx(FormationPhase::Bootstrapping, DISCOVER),
        tx(FormationPhase::Bootstrapping, DISCOVER),
        tx(FormationPhase::Electing, ELECT),
        tx(FormationPhase::Formed, LEAD),
        tx(FormationPhase::Formed, FOLLOW),
        tx(FormationPhase::Electing, ELECT),
        tx(FormationPhase::Degraded, DEGRADED),
        tx(FormationPhase::Bootstrapping, DISCOVER),
        tx(FormationPhase::Degraded, DEGRADED),
        tx(FormationPhase::Bootstrapping, DISCOVER),
        tx(FormationPhase::Formed, CAUGHT_UP),
        tx(FormationPhase::Rebalancing, REBALANCE),
        tx(FormationPhase::Bootstrapping, N),
    ],
    // Electing
    [
        tx(FormationPhase::Electing, N),
        tx(FormationPhase::Electing, N),
        tx(FormationPhase::Electing, ELECT),
        tx(FormationPhase::Formed, LEAD),
        tx(FormationPhase::Formed, FOLLOW),
        tx(FormationPhase::Electing, ELECT),
        tx(FormationPhase::Degraded, DEGRADED),
        tx(FormationPhase::CatchingUp, CATCH_UP),
        tx(FormationPhase::Degraded, DEGRADED),
        tx(FormationPhase::Electing, ELECT),
        tx(FormationPhase::Formed, CAUGHT_UP),
        tx(FormationPhase::Rebalancing, REBALANCE),
        tx(FormationPhase::Electing, N),
    ],
    // Formed
    [
        tx(FormationPhase::Formed, N),
        tx(FormationPhase::Formed, N),
        tx(FormationPhase::Electing, ELECT),
        tx(FormationPhase::Formed, N),
        tx(FormationPhase::Formed, FOLLOW),
        tx(FormationPhase::Electing, ELECT),
        tx(FormationPhase::Degraded, DEGRADED),
        tx(FormationPhase::CatchingUp, CATCH_UP),
        tx(FormationPhase::Degraded, DEGRADED),
        tx(FormationPhase::CatchingUp, CATCH_UP),
        tx(FormationPhase::Formed, N),
        tx(FormationPhase::Rebalancing, REBALANCE),
        tx(FormationPhase::Formed, REBALANCED),
    ],
    // Degraded
    [
        tx(FormationPhase::Degraded, N),
        tx(FormationPhase::Degraded, N),
        tx(FormationPhase::Electing, ELECT),
        tx(FormationPhase::Formed, LEAD),
        tx(FormationPhase::Formed, FOLLOW),
        tx(FormationPhase::Electing, ELECT),
        tx(FormationPhase::Degraded, DEGRADED),
        tx(FormationPhase::CatchingUp, CATCH_UP),
        tx(FormationPhase::Degraded, DEGRADED),
        tx(FormationPhase::CatchingUp, CATCH_UP),
        tx(FormationPhase::Formed, CAUGHT_UP),
        tx(FormationPhase::Rebalancing, REBALANCE),
        tx(FormationPhase::Degraded, N),
    ],
    // CatchingUp
    [
        tx(FormationPhase::CatchingUp, CATCH_UP),
        tx(FormationPhase::CatchingUp, CATCH_UP),
        tx(FormationPhase::Electing, ELECT),
        tx(FormationPhase::Formed, LEAD),
        tx(FormationPhase::CatchingUp, FOLLOW),
        tx(FormationPhase::Electing, ELECT),
        tx(FormationPhase::Degraded, DEGRADED),
        tx(FormationPhase::CatchingUp, CATCH_UP),
        tx(FormationPhase::Degraded, DEGRADED),
        tx(FormationPhase::CatchingUp, CATCH_UP),
        tx(FormationPhase::Formed, CAUGHT_UP),
        tx(FormationPhase::Rebalancing, REBALANCE),
        tx(FormationPhase::Formed, CAUGHT_UP),
    ],
    // Rebalancing
    [
        tx(FormationPhase::Rebalancing, REBALANCE),
        tx(FormationPhase::Rebalancing, REBALANCE),
        tx(FormationPhase::Electing, ELECT),
        tx(FormationPhase::Formed, LEAD),
        tx(FormationPhase::Rebalancing, FOLLOW),
        tx(FormationPhase::Electing, ELECT),
        tx(FormationPhase::Degraded, DEGRADED),
        tx(FormationPhase::CatchingUp, CATCH_UP),
        tx(FormationPhase::Degraded, DEGRADED),
        tx(FormationPhase::CatchingUp, CATCH_UP),
        tx(FormationPhase::Formed, CAUGHT_UP),
        tx(FormationPhase::Rebalancing, REBALANCE),
        tx(FormationPhase::Formed, REBALANCED),
    ],
];

/// Return the explicit node transition table cell for `state + event`.
pub fn node_transition(state: NodeFsmState, event: ClusterFsmEvent) -> FsmTransition<NodeFsmState> {
    NODE_TRANSITION_TABLE
        .get(state.as_index())
        .and_then(|row| row.get(event.as_index()))
        .copied()
        .expect("node FSM transition table must be total")
}

/// Return the explicit cluster transition table cell for `phase + event`.
pub fn cluster_transition(
    phase: FormationPhase,
    event: ClusterFsmEvent,
) -> FsmTransition<FormationPhase> {
    CLUSTER_TRANSITION_TABLE
        .get(phase.as_index())
        .and_then(|row| row.get(event.as_index()))
        .copied()
        .expect("cluster FSM transition table must be total")
}

/// Minimal state holder that applies the explicit formation table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterFsm {
    phase: FormationPhase,
    current_term: u64,
}

impl Default for ClusterFsm {
    fn default() -> Self {
        Self {
            phase: FormationPhase::Unformed,
            current_term: 0,
        }
    }
}

impl ClusterFsm {
    /// Build a fresh formation FSM.
    pub fn new() -> Self {
        Self::default()
    }

    /// Current cluster formation phase.
    pub fn phase(&self) -> FormationPhase {
        self.phase
    }

    /// Current modeled election term.
    pub fn current_term(&self) -> u64 {
        self.current_term
    }

    /// Apply one event through the explicit transition table.
    pub fn apply(&mut self, event: ClusterFsmEvent) -> FsmTransition<FormationPhase> {
        let transition = cluster_transition(self.phase, event);
        if matches!(transition.action, ClusterFsmAction::StartElection) {
            self.current_term = self.current_term.saturating_add(1);
        }
        self.phase = transition.next;
        transition
    }
}

/// Minimal per-node state holder that applies the explicit node table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeFsm {
    state: NodeFsmState,
}

impl Default for NodeFsm {
    fn default() -> Self {
        Self {
            state: NodeFsmState::Disconnected,
        }
    }
}

impl NodeFsm {
    /// Build a fresh node FSM.
    pub fn new() -> Self {
        Self::default()
    }

    /// Current node state.
    pub fn state(&self) -> NodeFsmState {
        self.state
    }

    /// Apply one event through the explicit transition table.
    pub fn apply(&mut self, event: ClusterFsmEvent) -> FsmTransition<NodeFsmState> {
        let transition = node_transition(self.state, event);
        self.state = transition.next;
        transition
    }
}

/// Source of election behavior used by the simulator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElectionSource {
    /// Deterministic simulator model used when raft-rs multi-node election
    /// cannot expose a seedable timeout seam.
    SimModel,
}

impl ElectionSource {
    /// Stable machine-readable source label.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SimModel => "sim-model",
        }
    }

    /// Whether this source may be presented as a production consensus claim.
    pub fn carries_product_consensus_claim(self) -> bool {
        match self {
            Self::SimModel => false,
        }
    }

    /// Human-facing disclosure for demo surfaces.
    pub fn disclosure(self) -> &'static str {
        match self {
            Self::SimModel => {
                "deterministic simulator election model for the lab; not a product consensus claim"
            }
        }
    }
}

impl fmt::Display for ElectionSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Per-node modeled election state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElectionNodeState {
    /// Node id.
    pub node_id: ClusterNodeId,
    /// FSM role.
    pub state: NodeFsmState,
    /// Current modeled term.
    pub term: u64,
    /// Candidate this node voted for in the current term.
    pub voted_for: Option<ClusterNodeId>,
    /// Votes observed by this node when it is the winning candidate.
    pub votes_received: usize,
}

/// Snapshot of the deterministic election driver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElectionDriverSnapshot {
    /// Election source label.
    pub source: ElectionSource,
    /// Current cluster formation phase.
    pub phase: FormationPhase,
    /// Current modeled term.
    pub term: u64,
    /// Current leader, if a quorum elected one.
    pub leader: Option<ClusterNodeId>,
    /// Per-node state ordered by node id.
    pub nodes: Vec<ElectionNodeState>,
    /// Stable trace emitted by the driver.
    pub trace: Vec<String>,
}

impl ElectionDriverSnapshot {
    /// Return nodes currently in the leader state.
    pub fn leaders(&self) -> Vec<&ElectionNodeState> {
        self.nodes
            .iter()
            .filter(|node| node.state == NodeFsmState::Leader)
            .collect()
    }
}

#[derive(Debug, Clone)]
struct ElectionNode {
    fsm: NodeFsm,
    term: u64,
    voted_for: Option<ClusterNodeId>,
    votes_received: usize,
}

impl ElectionNode {
    fn new() -> Self {
        Self {
            fsm: NodeFsm::new(),
            term: 0,
            voted_for: None,
            votes_received: 0,
        }
    }

    fn apply(&mut self, event: ClusterFsmEvent) -> FsmTransition<NodeFsmState> {
        self.fsm.apply(event)
    }

    fn set_term(&mut self, term: u64) {
        self.term = term;
    }

    fn observe_vote(&mut self, voted_for: ClusterNodeId, votes_received: usize) {
        self.voted_for = Some(voted_for);
        self.votes_received = votes_received;
    }

    fn clear_vote(&mut self) {
        self.voted_for = None;
        self.votes_received = 0;
    }
}

/// Deterministic election driver for the simulator lab.
#[derive(Debug, Clone)]
pub struct ElectionDriver {
    seed: u64,
    source: ElectionSource,
    cluster: ClusterFsm,
    nodes: BTreeMap<ClusterNodeId, ElectionNode>,
    leader: Option<ClusterNodeId>,
    trace: Vec<String>,
}

impl ElectionDriver {
    /// Build a deterministic election driver over a stable node set.
    pub fn new(seed: u64, node_ids: impl IntoIterator<Item = ClusterNodeId>) -> Self {
        let nodes = node_ids
            .into_iter()
            .map(|node_id| (node_id, ElectionNode::new()))
            .collect();
        Self {
            seed,
            source: ElectionSource::SimModel,
            cluster: ClusterFsm::new(),
            nodes,
            leader: None,
            trace: Vec::new(),
        }
    }

    /// Election behavior source.
    pub fn source(&self) -> ElectionSource {
        self.source
    }

    /// Current leader, if one exists.
    pub fn leader(&self) -> Option<&ClusterNodeId> {
        self.leader.as_ref()
    }

    /// Current modeled term.
    pub fn term(&self) -> u64 {
        self.cluster.current_term()
    }

    /// Current cluster formation phase.
    pub fn phase(&self) -> FormationPhase {
        self.cluster.phase()
    }

    /// Stable election trace.
    pub fn trace(&self) -> &[String] {
        &self.trace
    }

    /// Return a stable snapshot of the modeled election state.
    pub fn snapshot(&self) -> ElectionDriverSnapshot {
        ElectionDriverSnapshot {
            source: self.source,
            phase: self.phase(),
            term: self.term(),
            leader: self.leader.clone(),
            nodes: self
                .nodes
                .iter()
                .map(|(node_id, node)| ElectionNodeState {
                    node_id: node_id.clone(),
                    state: node.fsm.state(),
                    term: node.term,
                    voted_for: node.voted_for.clone(),
                    votes_received: node.votes_received,
                })
                .collect(),
            trace: self.trace.clone(),
        }
    }

    /// Advance election state by one logical simulator step.
    pub fn step(&mut self, logical_step: u64, live_nodes: &BTreeSet<ClusterNodeId>) {
        self.mark_unavailable_nodes(live_nodes, logical_step);

        if self.cluster.phase() == FormationPhase::Unformed && !live_nodes.is_empty() {
            self.apply_cluster(ClusterFsmEvent::Boot, logical_step);
            for node_id in live_nodes {
                self.apply_node(node_id, ClusterFsmEvent::Boot, logical_step);
            }
        }

        if let Some(leader) = self.leader.clone() {
            if live_nodes.contains(&leader) {
                self.heartbeat_from_leader(&leader, live_nodes, logical_step);
                return;
            }
            self.leader = None;
            self.apply_cluster(ClusterFsmEvent::LeaderLost, logical_step);
            self.trace
                .push(format!("election:{logical_step}:leader-lost:{leader}"));
        }

        let quorum = quorum(self.nodes.len());
        if live_nodes.len() < quorum {
            self.apply_cluster(ClusterFsmEvent::Isolate, logical_step);
            self.trace.push(format!(
                "election:{logical_step}:no-quorum:live={} quorum={quorum}",
                live_nodes.len()
            ));
            return;
        }

        if let Some(candidate) = self.due_candidate(logical_step, live_nodes) {
            self.elect(candidate, live_nodes, logical_step);
        }
    }

    fn due_candidate(
        &self,
        logical_step: u64,
        live_nodes: &BTreeSet<ClusterNodeId>,
    ) -> Option<ClusterNodeId> {
        live_nodes
            .iter()
            .filter(|node_id| {
                self.nodes
                    .get(*node_id)
                    .is_some_and(|node| node.fsm.state() != NodeFsmState::Disabled)
            })
            .filter_map(|node_id| {
                let timeout = deterministic_timeout(self.seed, node_id, self.term());
                (logical_step >= timeout).then_some((timeout, node_id.clone()))
            })
            .min_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)))
            .map(|(_, node_id)| node_id)
    }

    fn elect(
        &mut self,
        candidate: ClusterNodeId,
        live_nodes: &BTreeSet<ClusterNodeId>,
        logical_step: u64,
    ) {
        self.apply_cluster(ClusterFsmEvent::ElectionTimeout, logical_step);
        let term = self.term();
        for node_id in live_nodes {
            self.apply_node(node_id, ClusterFsmEvent::ElectionTimeout, logical_step);
        }
        self.apply_node(&candidate, ClusterFsmEvent::VoteQuorum, logical_step);
        for node_id in live_nodes {
            if let Some(node) = self.nodes.get_mut(node_id) {
                node.set_term(term);
                node.observe_vote(candidate.clone(), usize::from(*node_id == candidate));
            }
        }
        if let Some(winner) = self.nodes.get_mut(&candidate) {
            winner.observe_vote(candidate.clone(), live_nodes.len());
        }
        for node_id in live_nodes {
            if *node_id != candidate {
                self.apply_node(node_id, ClusterFsmEvent::LeaderHeartbeat, logical_step);
            }
        }
        self.apply_cluster(ClusterFsmEvent::VoteQuorum, logical_step);
        self.leader = Some(candidate.clone());
        self.trace.push(format!(
            "election:{logical_step}:leader:{candidate}:term:{term}:votes:{}:source:{}",
            live_nodes.len(),
            self.source
        ));
    }

    fn heartbeat_from_leader(
        &mut self,
        leader: &ClusterNodeId,
        live_nodes: &BTreeSet<ClusterNodeId>,
        logical_step: u64,
    ) {
        for node_id in live_nodes {
            if node_id != leader {
                self.apply_node(node_id, ClusterFsmEvent::LeaderHeartbeat, logical_step);
            }
        }
        self.apply_cluster(ClusterFsmEvent::LeaderHeartbeat, logical_step);
    }

    fn mark_unavailable_nodes(&mut self, live_nodes: &BTreeSet<ClusterNodeId>, logical_step: u64) {
        let unavailable = self
            .nodes
            .keys()
            .filter(|node_id| !live_nodes.contains(*node_id))
            .cloned()
            .collect::<Vec<_>>();
        for node_id in unavailable {
            let current = self
                .nodes
                .get(&node_id)
                .map(|node| node.fsm.state())
                .unwrap_or(NodeFsmState::Disconnected);
            if current != NodeFsmState::Disconnected && current != NodeFsmState::Disabled {
                self.apply_node(&node_id, ClusterFsmEvent::Isolate, logical_step);
                if let Some(node) = self.nodes.get_mut(&node_id) {
                    node.clear_vote();
                }
            }
        }
    }

    fn apply_cluster(
        &mut self,
        event: ClusterFsmEvent,
        logical_step: u64,
    ) -> FsmTransition<FormationPhase> {
        let before = self.cluster.phase();
        let transition = self.cluster.apply(event);
        if before != transition.next || transition.action != ClusterFsmAction::None {
            self.trace.push(format!(
                "cluster:{logical_step}:{before}->{next}:{event:?}:{action:?}",
                next = transition.next,
                action = transition.action
            ));
        }
        transition
    }

    fn apply_node(
        &mut self,
        node_id: &ClusterNodeId,
        event: ClusterFsmEvent,
        logical_step: u64,
    ) -> Option<FsmTransition<NodeFsmState>> {
        let node = self.nodes.get_mut(node_id)?;
        let before = node.fsm.state();
        let transition = node.apply(event);
        if before != transition.next || transition.action != ClusterFsmAction::None {
            self.trace.push(format!(
                "node:{logical_step}:{node_id}:{before}->{next}:{event:?}:{action:?}",
                next = transition.next,
                action = transition.action
            ));
        }
        Some(transition)
    }
}

fn quorum(node_count: usize) -> usize {
    node_count / 2 + 1
}

fn deterministic_timeout(seed: u64, node_id: &ClusterNodeId, term: u64) -> u64 {
    let mut hash = seed ^ term.rotate_left(17) ^ 0x53_53_53_53_53_53_53_53;
    for byte in node_id.as_str().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    2 + (hash % 5)
}
