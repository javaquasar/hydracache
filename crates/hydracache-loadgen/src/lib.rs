//! Coordinated-omission-safe measurement primitives for HydraCache release 0.67.
//!
//! This crate is a development tool. Product crates must never depend on it.

pub mod histogram;
pub mod knee;
pub mod profile;
pub mod rate;
pub mod report;
pub mod runner;
pub mod scenario;
pub mod target;

pub use histogram::{LatencyHistogram, LatencySummary};
pub use knee::{KneeResult, RateSample, SustainabilityCriteria, SustainabilityVerdict};
pub use profile::{PerformanceProfile, ProfileValidation, RunnerFingerprint};
pub use rate::{run_open_loop, FixedRateSchedule, OpenLoopConfig, OpenLoopObservation};
pub use report::{BuildIdentity, PerfReport, SourceIdentity, SurfaceIdentity};
pub use runner::{run_phases, PhaseConfig, PhaseRun};
pub use scenario::{ErrorBudgets, Scenario};
pub use target::{Target, TargetError, TargetOutcome, TargetRequest};

/// Schema version shared by committed scenario/report contracts.
pub const PERF_SCHEMA_VERSION: u32 = 1;

/// Release that owns this development-only measurement contract.
pub const PERF_RELEASE: &str = "0.67.0";
