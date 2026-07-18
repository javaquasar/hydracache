//! Coordinated-omission-safe measurement primitives for HydraCache release 0.67.
//!
//! This crate is a development tool. Product crates must never depend on it.

pub mod allocation;
pub mod budget_receipt;
pub mod cli;
pub mod compare_redis;
pub mod histogram;
pub mod knee;
pub mod overload;
pub mod profile;
pub mod rate;
pub mod report;
pub mod resp_external;
pub mod runner;
pub mod scenario;
pub mod target;
pub mod targets;
pub mod tiers;

pub use histogram::{LatencyHistogram, LatencySummary};
pub use knee::{
    KneeResult, PhaseAccounting, RatePointEvidence, RateSample, RepeatEvidence,
    SustainabilityCriteria, SustainabilityVerdict,
};
pub use profile::{PerformanceProfile, ProfileValidation, RunnerFingerprint};
pub use rate::{run_open_loop, FixedRateSchedule, OpenLoopConfig, OpenLoopObservation};
pub use report::{
    BuildIdentity, ComparisonEvidence, DimensionValue, EvidenceRunMode, KeyDistributionIdentity,
    LoadClaim, LoadCurveEvidence, MeasurementEvidence, PerfReport, Quantity, ReportWriteError,
    ScalarEvidence, ScalarPoint, SourceIdentity, SurfaceIdentity, TraceReplayEvidence,
    WeightedOperation, WeightedPayload, WorkloadIdentity,
};
pub use runner::{run_phases, run_scenario, PhaseConfig, PhaseRun};
pub use scenario::{ErrorBudgets, Scenario};
pub use target::{PreloadOutcome, Target, TargetError, TargetOutcome, TargetRequest};

/// Schema version shared by committed scenario/report contracts.
pub const PERF_SCHEMA_VERSION: u32 = 1;

/// Release that owns this development-only measurement contract.
pub const PERF_RELEASE: &str = "0.67.0";
