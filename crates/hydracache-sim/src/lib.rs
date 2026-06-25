//! Deterministic simulation primitives for HydraCache release 0.44.
//!
//! This crate is intentionally small and sans-IO. Higher-level simulator pieces
//! build on these seeded primitives so every failing run can be reproduced from
//! a seed and step count.

pub mod clock;
pub mod election;
pub mod invariants;
pub mod linearizability;
pub mod lock_safety;
pub mod network;
pub mod persistence_recovery;
pub mod rng;
pub mod scenarios;
pub mod schedule;
pub mod snapshot;
pub mod storage;
#[cfg(not(target_arch = "wasm32"))]
pub mod upgrade_recovery;
pub mod workload;
pub mod world;

pub use clock::SimClock;
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
pub use lock_safety::{run_lock_safety, LockSafetyReport, LockSafetyScenario};
pub use network::{LinkFault, PartitionSymmetry, SimNetwork, TimedMessage};
pub use persistence_recovery::{
    run_persistence_recovery, PersistenceRecoveryFault, PersistenceRecoveryInvariantReport,
    PersistenceRecoveryScenario,
};
pub use rng::SimRng;
pub use scenarios::{
    run_scenario, scenario_matches_expectation, scenario_presets, ExpectedScenarioProgress,
    ExpectedScenarioVerdict, ScenarioAction, ScenarioError, ScenarioPreset, ScenarioRun,
    SIM_SCENARIO_SET_VERSION,
};
pub use schedule::{
    FailureReport, FaultSchedule, ReplayOutcome, ReplayRunner, ScheduledFault, ScheduledFaultKind,
};
pub use snapshot::{
    ConvergenceView, KeyReplicaView, KeyView, LinkStateView, LinkView, MessageView, NodeView,
    ProgressView, SimSnapshot, SimSnapshotDecodeError, SnapshotOverBudgetView, VerdictView,
    MAX_IN_FLIGHT_RENDERED, SIM_SNAPSHOT_SCHEMA_VERSION,
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
pub use world::{SimConfig, SimOutcome, SimWorld};
