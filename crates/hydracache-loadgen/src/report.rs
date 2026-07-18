use serde::{Deserialize, Serialize};

use crate::knee::KneeResult;
use crate::profile::{ProfileValidation, RunnerFingerprint};
use crate::rate::OpenLoopObservation;
use crate::{PERF_RELEASE, PERF_SCHEMA_VERSION};

/// Honest identity of the callable boundary measured by a report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceIdentity {
    pub surface_kind: String,
    pub execution_mode: String,
    pub state_scope: String,
    pub network_boundary: String,
    pub claim_scope: String,
}

/// Exact source candidate measured by a report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceIdentity {
    pub git_commit: String,
    pub cargo_lock_sha256: String,
    pub toolchain: String,
    pub build_flags: Vec<String>,
}

/// Stable build contract plus per-run prebuild evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildIdentity {
    pub prebuild_contract_digest: String,
    pub prebuild_manifest_sha256: String,
    pub binary_sha256: Vec<(String, String)>,
}

/// Phase accounting proves warm-up samples are not steady-state samples.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhaseAccounting {
    pub reset_operations: u64,
    pub preload_operations: u64,
    pub warmup_operations: u64,
    pub steady_operations: u64,
    pub reset_ms: u64,
    pub preload_ms: u64,
    pub warmup_ms: u64,
    pub steady_ms: u64,
    pub warmup_samples_in_steady_histogram: u64,
}

/// One canonical release-0.67 performance report.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerfReport {
    pub schema_version: u32,
    pub release: String,
    pub report_id: String,
    pub scenario_id: String,
    pub scenario_digest: String,
    pub workload_digest: String,
    pub state_digest: String,
    pub seed: u64,
    pub surface: SurfaceIdentity,
    pub runner_profile: String,
    pub observed_runner: RunnerFingerprint,
    pub profile_validation: ProfileValidation,
    pub source: SourceIdentity,
    pub build: BuildIdentity,
    pub phase_accounting: PhaseAccounting,
    pub repeat_state_digests: Vec<String>,
    pub repeats: Vec<OpenLoopObservation>,
    pub knee: KneeResult,
    pub stable: bool,
    pub stability_reasons: Vec<String>,
}

impl PerfReport {
    /// Build a report with the fixed release/schema identity.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        report_id: impl Into<String>,
        scenario_id: impl Into<String>,
        scenario_digest: impl Into<String>,
        workload_digest: impl Into<String>,
        state_digest: impl Into<String>,
        seed: u64,
        surface: SurfaceIdentity,
        runner_profile: impl Into<String>,
        observed_runner: RunnerFingerprint,
        profile_validation: ProfileValidation,
        source: SourceIdentity,
        build: BuildIdentity,
        phase_accounting: PhaseAccounting,
        repeat_state_digests: Vec<String>,
        repeats: Vec<OpenLoopObservation>,
        knee: KneeResult,
        stability_reasons: Vec<String>,
    ) -> Self {
        let stable = stability_reasons.is_empty() && profile_validation.eligible;
        Self {
            schema_version: PERF_SCHEMA_VERSION,
            release: PERF_RELEASE.to_owned(),
            report_id: report_id.into(),
            scenario_id: scenario_id.into(),
            scenario_digest: scenario_digest.into(),
            workload_digest: workload_digest.into(),
            state_digest: state_digest.into(),
            seed,
            surface,
            runner_profile: runner_profile.into(),
            observed_runner,
            profile_validation,
            source,
            build,
            phase_accounting,
            repeat_state_digests,
            repeats,
            knee,
            stable,
            stability_reasons,
        }
    }

    /// Serialize stable pretty JSON for a receipt-hashed artifact.
    pub fn to_pretty_json(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec_pretty(self)
    }
}
