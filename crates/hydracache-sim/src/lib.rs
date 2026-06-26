//! Deterministic simulation primitives for HydraCache release 0.44.
//!
//! This crate is intentionally small and sans-IO. Higher-level simulator pieces
//! build on these seeded primitives so every failing run can be reproduced from
//! a seed and step count.

pub mod clock;
pub mod control;
pub mod election;
pub mod invariants;
pub mod linearizability;
#[cfg(not(target_arch = "wasm32"))]
pub mod lock_safety;
pub mod network;
#[cfg(not(target_arch = "wasm32"))]
pub mod persistence_recovery;
pub mod rng;
pub mod scenarios;
pub mod schedule;
#[cfg(not(target_arch = "wasm32"))]
pub mod sim_raft;
pub mod snapshot;
pub mod storage;
#[cfg(not(target_arch = "wasm32"))]
pub mod upgrade_recovery;
pub mod workload;
pub mod world;

pub use clock::SimClock;
pub use control::{
    ControlActionV1, ControlApplyError, ReplayScriptError, ReplayScriptV1, SimMode,
    MAX_REPLAY_ACTIONS, REPLAY_SCRIPT_VERSION,
};
pub use election::{
    cluster_transition, node_transition, ClusterFsm, ClusterFsmAction, ClusterFsmEvent,
    ElectionDriver, ElectionDriverSnapshot, ElectionNodeState, ElectionSignal, ElectionSignalKind,
    ElectionSource, FormationPhase, FsmTransition, NodeFsm, NodeFsmState, CLUSTER_TRANSITION_TABLE,
    NODE_TRANSITION_TABLE,
};
pub use invariants::{
    ElectionTopologyNode, ElectionTopologyState, InvariantChecker, InvariantReport,
    InvariantViolation, LogEntry, LogOp, ReplicaSnapshot, SubscriberDeliveryObservation,
    ValueObservation, ValueState,
};
pub use linearizability::{
    LinearizabilityChecker, LinearizabilityReport, LinearizabilityViolation,
};
#[cfg(not(target_arch = "wasm32"))]
pub use lock_safety::{run_lock_safety, LockSafetyReport, LockSafetyScenario};
pub use network::{LinkFault, PartitionSymmetry, SimNetwork, TimedMessage};
#[cfg(not(target_arch = "wasm32"))]
pub use persistence_recovery::{
    run_persistence_recovery, PersistenceRecoveryFault, PersistenceRecoveryInvariantReport,
    PersistenceRecoveryScenario,
};
pub use rng::SimRng;
pub use scenarios::{
    run_scenario, scenario_matches_expectation, scenario_presets, scripted_lab_catalog,
    ExpectedScenarioProgress, ExpectedScenarioVerdict, ScenarioAction, ScenarioError,
    ScenarioPreset, ScenarioRun, SIM_SCENARIO_SET_VERSION,
};
pub use schedule::{
    FailureReport, FaultSchedule, ReplayOutcome, ReplayRunner, ScheduledFault, ScheduledFaultKind,
};
#[cfg(not(target_arch = "wasm32"))]
pub use sim_raft::{SimRaftCluster, SimRaftError, SimRaftInFlightKey, SimRaftResult};
pub use snapshot::{
    ClientView, ConvergenceView, KeyReplicaView, KeyView, LinkStateView, LinkView, MessageView,
    NodeView, ProgressView, RebalanceView, SimSnapshot, SimSnapshotDecodeError,
    SnapshotOverBudgetView, SubscriberEventView, SubscriberView, SyncProgressView, VerdictView,
    MAX_IN_FLIGHT_RENDERED, MAX_SUBSCRIBER_BUFFER, SIM_SNAPSHOT_SCHEMA_VERSION,
};
pub use storage::{
    SimStorage, SimStorageApply, SimStorageError, StorageFault, StorageZoneId, StoredValue,
};
#[cfg(not(target_arch = "wasm32"))]
pub use upgrade_recovery::{
    run_upgrade_and_recovery, DeploymentFault, DeploymentInvariantReport,
    DeploymentRecoveryScenario,
};
pub use workload::{
    EventId, History, HistoryEvent, WorkloadConfig, WorkloadGenerator, WorkloadOp, WorkloadResult,
};
pub use world::{ElectionBackend, SimConfig, SimOutcome, SimWorld};
