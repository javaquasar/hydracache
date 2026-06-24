//! Deterministic simulation primitives for HydraCache release 0.44.
//!
//! This crate is intentionally small and sans-IO. Higher-level simulator pieces
//! build on these seeded primitives so every failing run can be reproduced from
//! a seed and step count.

pub mod clock;
pub mod invariants;
pub mod linearizability;
pub mod network;
pub mod rng;
pub mod schedule;
pub mod snapshot;
pub mod storage;
#[cfg(not(target_arch = "wasm32"))]
pub mod upgrade_recovery;
pub mod workload;
pub mod world;

pub use clock::SimClock;
pub use invariants::{
    InvariantChecker, InvariantReport, InvariantViolation, LogEntry, LogOp, ReplicaSnapshot,
    ValueObservation, ValueState,
};
pub use linearizability::{
    LinearizabilityChecker, LinearizabilityReport, LinearizabilityViolation,
};
pub use network::{LinkFault, PartitionSymmetry, SimNetwork, TimedMessage};
pub use rng::SimRng;
pub use schedule::{
    FailureReport, FaultSchedule, ReplayOutcome, ReplayRunner, ScheduledFault, ScheduledFaultKind,
};
pub use snapshot::{
    ConvergenceView, KeyReplicaView, KeyView, LinkStateView, LinkView, NodeView, ProgressView,
    SimSnapshot, SimSnapshotDecodeError, VerdictView, SIM_SNAPSHOT_SCHEMA_VERSION,
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
