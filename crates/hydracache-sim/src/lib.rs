//! Deterministic simulation primitives for HydraCache release 0.44.
//!
//! This crate is intentionally small and sans-IO. Higher-level simulator pieces
//! build on these seeded primitives so every failing run can be reproduced from
//! a seed and step count.

pub mod clock;
pub mod invariants;
pub mod network;
pub mod rng;
pub mod storage;
pub mod workload;
pub mod world;

pub use clock::SimClock;
pub use invariants::{
    InvariantChecker, InvariantReport, InvariantViolation, LogEntry, LogOp, ReplicaSnapshot,
    ValueObservation, ValueState,
};
pub use network::{LinkFault, PartitionSymmetry, SimNetwork, TimedMessage};
pub use rng::SimRng;
pub use storage::{
    SimStorage, SimStorageApply, SimStorageError, StorageFault, StorageZoneId, StoredValue,
};
pub use workload::{
    EventId, History, HistoryEvent, WorkloadConfig, WorkloadGenerator, WorkloadOp, WorkloadResult,
};
pub use world::{SimConfig, SimOutcome, SimWorld};
