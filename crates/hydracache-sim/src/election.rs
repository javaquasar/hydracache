use std::fmt;

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
