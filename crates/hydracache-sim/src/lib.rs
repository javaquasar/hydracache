//! Deterministic simulation primitives for HydraCache release 0.44.
//!
//! This crate is intentionally small and sans-IO. Higher-level simulator pieces
//! build on these seeded primitives so every failing run can be reproduced from
//! a seed and step count.

pub mod clock;
pub mod network;
pub mod rng;
pub mod storage;
pub mod world;

pub use clock::SimClock;
pub use network::{LinkFault, PartitionSymmetry, SimNetwork, TimedMessage};
pub use rng::SimRng;
pub use storage::{
    SimStorage, SimStorageApply, SimStorageError, StorageFault, StorageZoneId, StoredValue,
};
pub use world::{SimConfig, SimOutcome, SimWorld};
