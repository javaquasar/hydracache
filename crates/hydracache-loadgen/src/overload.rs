//! W6 overload goodput and recovery characterization.
//!
//! Overload evidence is derived from a previously measured capacity knee.  It
//! cannot manufacture a knee from an operational event, a library/model cost,
//! or one of the removed `node-native`/generic `cluster` tier names.  Every
//! aggregate remains recomputable from the raw common open-loop observations.

use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use hydracache_cache_sim::KeyScheduleSpec;
use hydracache_client_transport_axum::ClientSurfaceLimits;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::knee::{KneeResult, RatePointEvidence, SustainabilityCriteria};
use crate::rate::{run_open_loop, OpenLoopConfig, OpenLoopObservation};
use crate::report::{
    EvidenceRunMode, LoadClaim, MeasurementEvidence, PerfReport, RespEndpointCapability,
    SurfaceIdentity, WorkloadIdentity,
};
use crate::runner::run_scenario;
use crate::scenario::{ErrorBudgets, Scenario};
use crate::target::{PreloadOutcome, Target, TargetError, TargetOutcome, TargetRequest};
use crate::targets::client_surface::{
    ClientSurfaceOperationMix, ClientSurfaceTarget, ClientSurfaceTargetConfig,
};
use crate::targets::local::{LocalCacheTarget, LocalOperationMix, LocalTargetConfig};
use crate::targets::resp::{Resp2Limits, RespOperationMix, RespTargetConfig, RespTcpTarget};
use crate::tiers::client_surface::{
    validate_client_surface_reference_report, CLIENT_W6_CAPACITY_MEASUREMENTS,
};
use crate::tiers::local::validate_local_reference_report;
use crate::tiers::resp_reference::{
    start_reference_daemon, RespDaemonEvidence, RespDaemonLaunch, RespReferencePorts,
    ValidatedRespReferenceContext, RESP_PING_FRAME, RESP_PONG_DISPLAY, RESP_PONG_FRAME,
};
use crate::PERF_RELEASE;

pub const OVERLOAD_SCHEMA_VERSION: u32 = 1;
pub const OVERLOAD_REPORT_VERSION: u32 = 1;
pub const W6_EVIDENCE_CLASS: &str = "w6-capacity-bound-overload-goodput-recovery";
pub const W6_CLAIM_SCOPE: &str = "capacity-bound-overload-goodput-recovery";
pub const W6_CANARY_MARKER: &str = "HC-CANARY-RED:W6";
pub const OVERLOAD_FACTORS_MILLIONTHS: [u32; 3] = [1_200_000, 1_500_000, 2_000_000];
pub const W6_CORE_REFERENCE_ENV: &str = "HYDRACACHE_RUN_PERF_CORE";
pub const W6_RESP_REFERENCE_ENV: &str = "HYDRACACHE_RUN_PERF_RESP";

const DETERMINISTIC_SMOKE_WINDOW_OPERATIONS: u64 = 48;
const LOCAL_PREDECESSOR: &str = "w1-local-cache-capacity";
const CLIENT_SURFACE_PREDECESSOR: &str = "w2-client-surface-in-process-capacity";
const RESP_PREDECESSOR: &str = "w3-node-local-resp-open-loop";
const FRESH_EXECUTION_RECEIPT_VERSION: u32 = 1;
const NODE_RESP_STABLE_CAPABILITY_VERSION: u32 = 1;
const LOCAL_CAPACITY_MEASUREMENT: &str = "hot_key_contention_throughput_floor";
const RESP_CAPACITY_MEASUREMENTS: [&str; 3] = [
    "resp_open_loop_get_set_knee_at_slo_workload_a",
    "resp_open_loop_get_set_knee_at_slo_workload_b",
    "resp_open_loop_get_set_knee_at_slo_workload_c",
];
const LOCAL_HOT_KEY_SCENARIO: &[u8] =
    include_bytes!("../../../docs/testing/perf-scenarios/0.67/local-hot-key-v1.toml");
const CLIENT_A_SCENARIO: &[u8] =
    include_bytes!("../../../docs/testing/perf-scenarios/0.67/client-surface-a-v1.toml");
const CLIENT_B_SCENARIO: &[u8] =
    include_bytes!("../../../docs/testing/perf-scenarios/0.67/client-surface-b-v1.toml");
const CLIENT_C_SCENARIO: &[u8] =
    include_bytes!("../../../docs/testing/perf-scenarios/0.67/client-surface-c-v1.toml");
const RESP_A_SCENARIO: &[u8] =
    include_bytes!("../../../docs/testing/perf-scenarios/0.67/resp-open-loop-a-v1.toml");
const RESP_B_SCENARIO: &[u8] =
    include_bytes!("../../../docs/testing/perf-scenarios/0.67/resp-open-loop-b-v1.toml");
const RESP_C_SCENARIO: &[u8] =
    include_bytes!("../../../docs/testing/perf-scenarios/0.67/resp-open-loop-c-v1.toml");

static FRESH_EXECUTION_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Error)]
pub enum OverloadError {
    #[error("W6 contract rejected: {0}")]
    Contract(String),
    #[error("W6 predecessor rejected: {0}")]
    Predecessor(String),
    #[error("W6 claim boundary rejected: {0}")]
    Boundary(String),
    #[error("W6 measurement failed: {0}")]
    Measurement(String),
    #[error("W6 evidence rejected: {0}")]
    Evidence(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<TargetError> for OverloadError {
    fn from(error: TargetError) -> Self {
        Self::Measurement(error.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EligibleOverloadSurface {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "client-surface")]
    ClientSurface,
    #[serde(rename = "node-resp")]
    NodeResp,
}

impl EligibleOverloadSurface {
    pub fn from_cli_name(value: &str) -> Result<Self, OverloadError> {
        match value {
            "local" => Ok(Self::Local),
            "client-surface" => Ok(Self::ClientSurface),
            "node-resp" => Ok(Self::NodeResp),
            "node-native" | "cluster" | "generic-cluster" => Err(OverloadError::Boundary(format!(
                "removed or generic tier name {value:?} cannot produce W6 evidence"
            ))),
            _ => Err(OverloadError::Boundary(format!(
                "surface {value:?} has no capacity-bound W6 adapter"
            ))),
        }
    }

    fn predecessor_class(self) -> &'static str {
        match self {
            Self::Local => LOCAL_PREDECESSOR,
            Self::ClientSurface => CLIENT_SURFACE_PREDECESSOR,
            Self::NodeResp => RESP_PREDECESSOR,
        }
    }

    fn capacity_surface(self) -> SurfaceIdentity {
        match self {
            Self::Local => SurfaceIdentity {
                surface_kind: "embedded-cache".to_owned(),
                execution_mode: "in-process-real-hydracache".to_owned(),
                state_scope: "process-local".to_owned(),
                network_boundary: "none".to_owned(),
                claim_scope: "embedded-cache-capacity".to_owned(),
            },
            Self::ClientSurface => SurfaceIdentity {
                surface_kind: "client-surface".to_owned(),
                execution_mode: "in-process-axum-router".to_owned(),
                state_scope: "process-local".to_owned(),
                network_boundary: "none".to_owned(),
                claim_scope: "in-process-client-surface-capacity".to_owned(),
            },
            Self::NodeResp => SurfaceIdentity {
                surface_kind: "node-resp".to_owned(),
                execution_mode: "real-daemon-tcp-resp-open-loop".to_owned(),
                state_scope: "node-local".to_owned(),
                network_boundary: "loopback-tcp".to_owned(),
                claim_scope: "selected-endpoint-capacity".to_owned(),
            },
        }
    }

    fn overload_surface(self) -> SurfaceIdentity {
        let mut identity = self.capacity_surface();
        identity.claim_scope = W6_CLAIM_SCOPE.to_owned();
        identity
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverloadRunMode {
    DeterministicSmoke,
    Reference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverloadIdentityContract {
    pub evidence_class: String,
    pub claim_scope: String,
    pub eligible_surfaces: Vec<EligibleOverloadSurface>,
    pub forbidden_tier_names: Vec<String>,
    pub require_capacity_knee: bool,
    pub reject_model_cost_as_capacity: bool,
    pub reject_operational_event_as_capacity: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferencePreloadContract {
    pub local: u64,
    pub client_surface: u64,
    pub node_resp: u64,
}

impl ReferencePreloadContract {
    fn for_surface(&self, surface: EligibleOverloadSurface) -> u64 {
        match surface {
            EligibleOverloadSurface::Local => self.local,
            EligibleOverloadSurface::ClientSurface => self.client_surface,
            EligibleOverloadSurface::NodeResp => self.node_resp,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverloadWorkContract {
    pub factors_millionths: Vec<u32>,
    pub repeats: u32,
    pub reference_preload_operations: ReferencePreloadContract,
    pub warmup_operations: u64,
    pub burst_operations: u64,
    pub recovery_operations_per_window: u64,
    pub max_recovery_windows: u32,
    pub required_consecutive_recovery_windows: u32,
    pub p999_min_samples: u64,
    pub highest_trackable_latency_us: u64,
    pub histogram_significant_figures: u8,
    pub drain_timeout_ms: u64,
    pub recovery_goodput_floor_ratio: f64,
    pub recovery_p99_ceiling_ratio: f64,
    pub maximum_goodput_spread_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverloadReferenceContract {
    pub required_profile: String,
    pub require_stable_predecessor: bool,
    pub require_predecessor_report_receipt: bool,
    pub require_runner_fingerprint: bool,
    pub require_prebuild_receipt: bool,
    pub require_surface_capability_receipt: bool,
    pub require_target_workload_binding: bool,
    pub committed_scenario_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverloadScenario {
    pub schema_version: u32,
    pub scenario_id: String,
    pub identity: OverloadIdentityContract,
    pub work: OverloadWorkContract,
    pub reference: OverloadReferenceContract,
}

impl OverloadScenario {
    fn expected_preload_operations(
        &self,
        surface: EligibleOverloadSurface,
        run_mode: OverloadRunMode,
    ) -> u64 {
        match run_mode {
            OverloadRunMode::DeterministicSmoke => 0,
            OverloadRunMode::Reference => {
                self.work.reference_preload_operations.for_surface(surface)
            }
        }
    }

    pub fn parse_toml(text: &str) -> Result<Self, OverloadError> {
        let scenario = toml::from_str::<Self>(text)
            .map_err(|error| OverloadError::Contract(format!("invalid W6 TOML: {error}")))?;
        scenario.validate()?;
        Ok(scenario)
    }

    pub fn load(path: &Path) -> Result<Self, OverloadError> {
        Self::parse_toml(&fs::read_to_string(path)?)
    }

    pub fn validate(&self) -> Result<(), OverloadError> {
        let expected_surfaces = BTreeSet::from([
            EligibleOverloadSurface::Local,
            EligibleOverloadSurface::ClientSurface,
            EligibleOverloadSurface::NodeResp,
        ]);
        let observed_surfaces = self
            .identity
            .eligible_surfaces
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let expected_forbidden = ["cluster", "generic-cluster", "node-native"]
            .into_iter()
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        let observed_forbidden = self
            .identity
            .forbidden_tier_names
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        if self.schema_version != OVERLOAD_SCHEMA_VERSION
            || self.scenario_id != "overload-capacity-v1"
            || self.identity.evidence_class != W6_EVIDENCE_CLASS
            || self.identity.claim_scope != W6_CLAIM_SCOPE
            || observed_surfaces != expected_surfaces
            || self.identity.eligible_surfaces
                != [
                    EligibleOverloadSurface::Local,
                    EligibleOverloadSurface::ClientSurface,
                    EligibleOverloadSurface::NodeResp,
                ]
            || observed_forbidden != expected_forbidden
            || self.identity.forbidden_tier_names != ["cluster", "generic-cluster", "node-native"]
            || !self.identity.require_capacity_knee
            || !self.identity.reject_model_cost_as_capacity
            || !self.identity.reject_operational_event_as_capacity
        {
            return Err(OverloadError::Boundary(
                "W6 must retain exactly the three W1/W2/W3 capacity-eligible surfaces and reject node-native, generic cluster, model-cost, and operational-event inputs"
                    .to_owned(),
            ));
        }
        if self.work.factors_millionths != OVERLOAD_FACTORS_MILLIONTHS
            || self.work.repeats != 3
            || self.work.reference_preload_operations.local != 0
            || self.work.reference_preload_operations.client_surface != 10_000
            || self.work.reference_preload_operations.node_resp != 10_000
            || self.work.warmup_operations != 4
            || self.work.burst_operations != 50_000
            || self.work.recovery_operations_per_window != 50_000
            || self.work.max_recovery_windows != 3
            || self.work.required_consecutive_recovery_windows != 2
            || self.work.p999_min_samples != 1
            || self.work.highest_trackable_latency_us != 1_000_000
            || self.work.histogram_significant_figures != 3
            || self.work.drain_timeout_ms != 1_000
            || self.work.recovery_goodput_floor_ratio != 0.85
            || self.work.recovery_p99_ceiling_ratio != 1.50
            || self.work.maximum_goodput_spread_ratio != 0.25
        {
            return Err(OverloadError::Contract(
                "W6 requires the exact committed 1.2x/1.5x/2x workload, spread tolerance, and two-window recovery confirmation"
                    .to_owned(),
            ));
        }
        if self.reference.required_profile != "reference-v1"
            || !self.reference.require_stable_predecessor
            || !self.reference.require_predecessor_report_receipt
            || !self.reference.require_runner_fingerprint
            || !self.reference.require_prebuild_receipt
            || !self.reference.require_surface_capability_receipt
            || !self.reference.require_target_workload_binding
        {
            return Err(OverloadError::Contract(
                "reference W6 runs must require the stable predecessor, runner, prebuild, and surface capability receipts"
                    .to_owned(),
            ));
        }
        let observed_digest = self.contract_digest()?;
        if !is_sha256(&self.reference.committed_scenario_sha256)
            || self.reference.committed_scenario_sha256 != observed_digest
        {
            return Err(OverloadError::Contract(format!(
                "W6 scenario shape differs from committed digest: expected {}, observed {observed_digest}",
                self.reference.committed_scenario_sha256
            )));
        }
        Ok(())
    }

    pub fn contract_digest(&self) -> Result<String, OverloadError> {
        let mut payload = self.clone();
        payload.reference.committed_scenario_sha256.clear();
        canonical_digest(&payload)
    }

    fn window_operations(&self, run_mode: OverloadRunMode) -> (u64, u64) {
        match run_mode {
            OverloadRunMode::DeterministicSmoke => (
                DETERMINISTIC_SMOKE_WINDOW_OPERATIONS,
                DETERMINISTIC_SMOKE_WINDOW_OPERATIONS,
            ),
            OverloadRunMode::Reference => (
                self.work.burst_operations,
                self.work.recovery_operations_per_window,
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReferencePredecessorRequest {
    pub surface: EligibleOverloadSurface,
    pub report_path: PathBuf,
    pub expected_report_sha256: String,
    /// Mandatory only for the W3 daemon predecessor. W1/W2 carry their
    /// archived in-process instance receipt inside the report itself.
    pub lifecycle_path: Option<PathBuf>,
    pub expected_lifecycle_sha256: Option<String>,
    pub prebuild_manifest_path: PathBuf,
    pub expected_prebuild_manifest_sha256: String,
}

/// Disk-backed receipt. All fields are deliberately private: reference
/// predecessors are produced only by [`load_reference_predecessor`], and every
/// later validation re-reads both artifacts instead of trusting serialized
/// booleans or caller-supplied identity strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferencePredecessorReceipt {
    profile: String,
    predecessor_report_path: PathBuf,
    predecessor_report_sha256: String,
    predecessor_measurement_id: String,
    predecessor_scenario_sha256: String,
    predecessor_payload_sha256: String,
    predecessor_lifecycle_path: Option<PathBuf>,
    predecessor_lifecycle_sha256: Option<String>,
    source_commit: String,
    cargo_lock_sha256: String,
    runner_fingerprint_sha256: String,
    prebuild_manifest_path: PathBuf,
    prebuild_receipt_sha256: String,
    stable_surface_capability_sha256: String,
    workload_identity_sha256: String,
    archived_execution_receipt_sha256: String,
    archived_execution_pid: u32,
    receipt_sha256: String,
}

impl ReferencePredecessorReceipt {
    fn computed_receipt_sha256(&self) -> Result<String, OverloadError> {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        canonical_digest(&payload)
    }

    fn validate(
        &self,
        scenario: &OverloadScenario,
        predecessor: &CapacityPredecessor,
    ) -> Result<(), OverloadError> {
        if self.profile != scenario.reference.required_profile
            || !self.predecessor_report_path.is_absolute()
            || !is_sha256(&self.predecessor_report_sha256)
            || !portable_identifier(&self.predecessor_measurement_id)
            || !is_sha256(&self.predecessor_scenario_sha256)
            || !is_sha256(&self.predecessor_payload_sha256)
            || !valid_optional_lifecycle(
                predecessor.surface,
                self.predecessor_lifecycle_path.as_deref(),
                self.predecessor_lifecycle_sha256.as_deref(),
            )
            || !is_git_commit(&self.source_commit)
            || !is_sha256(&self.cargo_lock_sha256)
            || !is_sha256(&self.runner_fingerprint_sha256)
            || !self.prebuild_manifest_path.is_absolute()
            || !is_sha256(&self.prebuild_receipt_sha256)
            || !is_sha256(&self.stable_surface_capability_sha256)
            || !is_sha256(&self.workload_identity_sha256)
            || !is_sha256(&self.archived_execution_receipt_sha256)
            || self.archived_execution_pid == 0
            || self.receipt_sha256 != self.computed_receipt_sha256()?
            || self.stable_surface_capability_sha256 != predecessor.stable_surface_capability_sha256
            || self.workload_identity_sha256 != predecessor.workload_identity_sha256
            || self.predecessor_payload_sha256 != capacity_payload_digest(predecessor)?
        {
            return Err(OverloadError::Predecessor(
                "reference predecessor receipt is unsealed or not bound to the exact capacity payload"
                    .to_owned(),
            ));
        }
        let snapshot = load_reference_snapshot(&ReferencePredecessorRequest {
            surface: predecessor.surface,
            report_path: self.predecessor_report_path.clone(),
            expected_report_sha256: self.predecessor_report_sha256.clone(),
            lifecycle_path: self.predecessor_lifecycle_path.clone(),
            expected_lifecycle_sha256: self.predecessor_lifecycle_sha256.clone(),
            prebuild_manifest_path: self.prebuild_manifest_path.clone(),
            expected_prebuild_manifest_sha256: self.prebuild_receipt_sha256.clone(),
        })?;
        if snapshot.report_path != self.predecessor_report_path
            || snapshot.report_sha256 != self.predecessor_report_sha256
            || snapshot.measurement_id != self.predecessor_measurement_id
            || snapshot.scenario_sha256 != self.predecessor_scenario_sha256
            || snapshot.payload_sha256 != self.predecessor_payload_sha256
            || snapshot.source_commit != self.source_commit
            || snapshot.cargo_lock_sha256 != self.cargo_lock_sha256
            || snapshot.runner_fingerprint_sha256 != self.runner_fingerprint_sha256
            || snapshot.prebuild_manifest_path != self.prebuild_manifest_path
            || snapshot.prebuild_manifest_sha256 != self.prebuild_receipt_sha256
            || snapshot.lifecycle_path != self.predecessor_lifecycle_path
            || snapshot.lifecycle_sha256 != self.predecessor_lifecycle_sha256
            || snapshot.stable_surface_capability_sha256 != self.stable_surface_capability_sha256
            || snapshot.workload_identity_sha256 != self.workload_identity_sha256
            || snapshot.archived_execution_receipt_sha256 != self.archived_execution_receipt_sha256
            || snapshot.archived_execution_pid != self.archived_execution_pid
        {
            return Err(OverloadError::Predecessor(
                "serialized predecessor receipt differs from the typed artifacts re-read from disk"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityPredecessor {
    pub evidence_class: String,
    pub surface: EligibleOverloadSurface,
    pub surface_identity: SurfaceIdentity,
    pub claim: LoadClaim,
    pub profile: String,
    pub stable_capacity_evidence: bool,
    /// Process-independent surface/build/scenario contract. Runtime endpoint
    /// capabilities and instance receipts are deliberately not stored here.
    pub stable_surface_capability_sha256: String,
    pub workload_identity_sha256: String,
    pub criteria: SustainabilityCriteria,
    pub knee: KneeResult,
    reference_receipt: Option<ReferencePredecessorReceipt>,
}

impl CapacityPredecessor {
    pub fn validate(
        &self,
        scenario: &OverloadScenario,
        run_mode: OverloadRunMode,
    ) -> Result<&RatePointEvidence, OverloadError> {
        scenario.validate()?;
        if self.evidence_class != self.surface.predecessor_class()
            || self.surface_identity != self.surface.capacity_surface()
            || self.claim != LoadClaim::CapacityKnee
            || !is_sha256(&self.stable_surface_capability_sha256)
            || !is_sha256(&self.workload_identity_sha256)
        {
            return Err(OverloadError::Predecessor(
                "W6 input is not the exact capacity-knee evidence class and surface identity"
                    .to_owned(),
            ));
        }
        reject_forbidden_identity(&self.surface_identity)?;
        let knee_problems = self.criteria.knee_validation_problems(&self.knee);
        if !knee_problems.is_empty() {
            return Err(OverloadError::Predecessor(format!(
                "capacity knee failed semantic validation: {knee_problems:?}"
            )));
        }
        let selected_rate = self.knee.sustainable_rate_per_second.ok_or_else(|| {
            OverloadError::Predecessor("predecessor has no sustainable capacity knee".to_owned())
        })?;
        if !selected_rate.is_finite()
            || selected_rate <= 0.0
            || selected_rate.fract() != 0.0
            || selected_rate > u64::MAX as f64
        {
            return Err(OverloadError::Predecessor(
                "capacity knee must be a positive integral rate supported by the fixed-rate runner"
                    .to_owned(),
            ));
        }
        let selected = self
            .knee
            .evaluated
            .iter()
            .find(|point| point.sample.offered_rate_per_second == selected_rate)
            .ok_or_else(|| {
                OverloadError::Predecessor(
                    "selected knee has no matching raw rate point".to_owned(),
                )
            })?;
        if !selected.verdict.sustainable
            || selected.sample.successes == 0
            || selected.sample.completed == 0
            || selected.sample.latency.p99_us.is_none()
        {
            return Err(OverloadError::Predecessor(
                "selected capacity knee lacks successful goodput or scheduled p99 evidence"
                    .to_owned(),
            ));
        }
        match run_mode {
            OverloadRunMode::DeterministicSmoke => {
                if self.profile != "smoke-v1"
                    || self.stable_capacity_evidence
                    || self.reference_receipt.is_some()
                {
                    return Err(OverloadError::Predecessor(
                        "deterministic smoke must remain an unclaimed smoke-v1 predecessor without a reference receipt"
                            .to_owned(),
                    ));
                }
            }
            OverloadRunMode::Reference => {
                if self.profile != scenario.reference.required_profile
                    || !self.stable_capacity_evidence
                {
                    return Err(OverloadError::Predecessor(
                        "reference overload requires stable reference-v1 predecessor evidence"
                            .to_owned(),
                    ));
                }
                let receipt = self.reference_receipt.as_ref().ok_or_else(|| {
                    OverloadError::Predecessor(
                        "reference overload is missing its predecessor receipt".to_owned(),
                    )
                })?;
                receipt.validate(scenario, self)?;
                if receipt.stable_surface_capability_sha256 != self.stable_surface_capability_sha256
                {
                    return Err(OverloadError::Predecessor(
                        "predecessor receipt does not bind the selected surface capability"
                            .to_owned(),
                    ));
                }
            }
        }
        Ok(selected)
    }

    pub fn knee_rate_per_second(&self) -> Result<u64, OverloadError> {
        let rate = self.knee.sustainable_rate_per_second.ok_or_else(|| {
            OverloadError::Predecessor("predecessor has no sustainable capacity knee".to_owned())
        })?;
        if !rate.is_finite() || rate <= 0.0 || rate.fract() != 0.0 || rate > u64::MAX as f64 {
            return Err(OverloadError::Predecessor(
                "capacity knee is not a positive integral rate".to_owned(),
            ));
        }
        Ok(rate as u64)
    }

    pub fn reference_target_contract(&self) -> Result<ReferenceTargetContract, OverloadError> {
        let receipt = self.reference_receipt.as_ref().ok_or_else(|| {
            OverloadError::Predecessor(
                "smoke predecessor has no reference target binding".to_owned(),
            )
        })?;
        Ok(ReferenceTargetContract {
            surface: self.surface,
            surface_identity: self.surface_identity.clone(),
            stable_surface_capability_sha256: self.stable_surface_capability_sha256.clone(),
            workload_identity_sha256: self.workload_identity_sha256.clone(),
            source_commit: receipt.source_commit.clone(),
            cargo_lock_sha256: receipt.cargo_lock_sha256.clone(),
            prebuild_manifest_sha256: receipt.prebuild_receipt_sha256.clone(),
        })
    }

    fn reference_receipt(&self) -> Result<&ReferencePredecessorReceipt, OverloadError> {
        self.reference_receipt.as_ref().ok_or_else(|| {
            OverloadError::Predecessor("reference predecessor receipt is absent".to_owned())
        })
    }

    pub fn payload_sha256(&self) -> Result<String, OverloadError> {
        capacity_payload_digest(self)
    }
}

#[derive(Debug, Clone)]
struct ReferenceSnapshot {
    report_path: PathBuf,
    report_sha256: String,
    measurement_id: String,
    scenario_sha256: String,
    source_commit: String,
    cargo_lock_sha256: String,
    runner_fingerprint_sha256: String,
    prebuild_manifest_path: PathBuf,
    prebuild_manifest_sha256: String,
    lifecycle_path: Option<PathBuf>,
    lifecycle_sha256: Option<String>,
    stable_surface_capability_sha256: String,
    workload_identity_sha256: String,
    archived_execution_receipt_sha256: String,
    archived_execution_pid: u32,
    criteria: SustainabilityCriteria,
    knee: KneeResult,
    payload_sha256: String,
}

/// Load one of the three exact W6 predecessor producers. W2/W3 select the
/// minimum valid A/B/C knee deterministically; callers cannot cherry-pick a
/// prettier workload. W4/model/event evidence has no route into this loader.
pub fn load_reference_predecessor(
    scenario: &OverloadScenario,
    request: ReferencePredecessorRequest,
) -> Result<CapacityPredecessor, OverloadError> {
    scenario.validate()?;
    let snapshot = load_reference_snapshot(&request)?;
    let mut predecessor = CapacityPredecessor {
        evidence_class: request.surface.predecessor_class().to_owned(),
        surface: request.surface,
        surface_identity: request.surface.capacity_surface(),
        claim: LoadClaim::CapacityKnee,
        profile: scenario.reference.required_profile.clone(),
        stable_capacity_evidence: true,
        stable_surface_capability_sha256: snapshot.stable_surface_capability_sha256.clone(),
        workload_identity_sha256: snapshot.workload_identity_sha256.clone(),
        criteria: snapshot.criteria,
        knee: snapshot.knee,
        reference_receipt: None,
    };
    let mut receipt = ReferencePredecessorReceipt {
        profile: scenario.reference.required_profile.clone(),
        predecessor_report_path: snapshot.report_path,
        predecessor_report_sha256: snapshot.report_sha256,
        predecessor_measurement_id: snapshot.measurement_id,
        predecessor_scenario_sha256: snapshot.scenario_sha256,
        predecessor_payload_sha256: snapshot.payload_sha256,
        predecessor_lifecycle_path: snapshot.lifecycle_path,
        predecessor_lifecycle_sha256: snapshot.lifecycle_sha256,
        source_commit: snapshot.source_commit,
        cargo_lock_sha256: snapshot.cargo_lock_sha256,
        runner_fingerprint_sha256: snapshot.runner_fingerprint_sha256,
        prebuild_manifest_path: snapshot.prebuild_manifest_path,
        prebuild_receipt_sha256: snapshot.prebuild_manifest_sha256,
        stable_surface_capability_sha256: predecessor.stable_surface_capability_sha256.clone(),
        workload_identity_sha256: predecessor.workload_identity_sha256.clone(),
        archived_execution_receipt_sha256: snapshot.archived_execution_receipt_sha256,
        archived_execution_pid: snapshot.archived_execution_pid,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = receipt.computed_receipt_sha256()?;
    predecessor.reference_receipt = Some(receipt);
    predecessor.validate(scenario, OverloadRunMode::Reference)?;
    Ok(predecessor)
}

fn capacity_payload_digest(predecessor: &CapacityPredecessor) -> Result<String, OverloadError> {
    canonical_digest(&(
        &predecessor.evidence_class,
        predecessor.surface,
        &predecessor.surface_identity,
        predecessor.claim,
        &predecessor.stable_surface_capability_sha256,
        &predecessor.workload_identity_sha256,
        &predecessor.criteria,
        &predecessor.knee,
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct W7PrebuildManifest {
    schema_version: u32,
    source: W7PrebuildSource,
    toolchain_identity: String,
    target_set: Vec<String>,
    features: Vec<String>,
    cargo_profile: String,
    flags: Vec<String>,
    build_recipe: Vec<String>,
    build_contract_digest: String,
    runner_profile: String,
    runner_fingerprint: String,
    platform_key: String,
    binaries: Vec<W7PrebuiltBinary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct W7PrebuildSource {
    git_commit: String,
    cargo_lock_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct W7PrebuiltBinary {
    id: String,
    canonical_path: PathBuf,
    sha256: String,
}

#[derive(Debug, Serialize)]
struct W7BuildContract<'a> {
    schema_version: u32,
    toolchain_identity: &'a str,
    target_set: &'a [String],
    features: &'a [String],
    cargo_profile: &'a str,
    flags: &'a [String],
    build_recipe: &'a [String],
}

fn load_reference_snapshot(
    request: &ReferencePredecessorRequest,
) -> Result<ReferenceSnapshot, OverloadError> {
    match request.surface {
        EligibleOverloadSurface::Local => load_local_reference_snapshot(request),
        EligibleOverloadSurface::ClientSurface => load_client_reference_snapshot(request),
        EligibleOverloadSurface::NodeResp => load_resp_reference_snapshot(request),
    }
}

fn load_local_reference_snapshot(
    request: &ReferencePredecessorRequest,
) -> Result<ReferenceSnapshot, OverloadError> {
    require_no_lifecycle(request, "W1")?;
    let (report_path, report) = read_reference_report(request, "W1")?;
    validate_common_reference_report(&report, request, "W1")?;
    let binding = validate_local_reference_report(&report)
        .map_err(|error| OverloadError::Predecessor(error.to_string()))?;
    if binding.measurement_id != LOCAL_CAPACITY_MEASUREMENT {
        return Err(OverloadError::Predecessor(
            "W1 W6 predecessor is not the committed hot-key capacity curve".to_owned(),
        ));
    }
    let curve = exact_capacity_curve(&report, &binding.measurement_id, "W1")?;
    if curve.scenario_digest != binding.scenario_sha256
        || curve.workload.digest != binding.workload_sha256
    {
        return Err(OverloadError::Predecessor(
            "W1 typed binding differs from its selected raw curve".to_owned(),
        ));
    }
    let stable_surface_capability_sha256 = binding
        .capability
        .digest()
        .map_err(|error| OverloadError::Predecessor(error.to_string()))?;
    snapshot_from_curve(
        request,
        report_path,
        &report,
        curve,
        None,
        None,
        stable_surface_capability_sha256,
        binding.instance.receipt_sha256,
        binding.instance.pid,
    )
}

fn load_client_reference_snapshot(
    request: &ReferencePredecessorRequest,
) -> Result<ReferenceSnapshot, OverloadError> {
    require_no_lifecycle(request, "W2")?;
    let (report_path, report) = read_reference_report(request, "W2")?;
    validate_common_reference_report(&report, request, "W2")?;
    let binding = validate_client_surface_reference_report(&report)
        .map_err(|error| OverloadError::Predecessor(error.to_string()))?;
    let ids = binding
        .capacity_measurements
        .iter()
        .map(|(id, _, _)| id.as_str())
        .collect::<Vec<_>>();
    if ids.as_slice() != CLIENT_W6_CAPACITY_MEASUREMENTS.as_slice() {
        return Err(OverloadError::Predecessor(
            "W2 typed binding lost its exact A/B/C capacity set".to_owned(),
        ));
    }
    let curve = minimum_valid_capacity_curve(&report, &ids, "W2")?;
    let stable_surface_capability_sha256 = binding
        .capability
        .digest()
        .map_err(|error| OverloadError::Predecessor(error.to_string()))?;
    snapshot_from_curve(
        request,
        report_path,
        &report,
        curve,
        None,
        None,
        stable_surface_capability_sha256,
        binding.instance.receipt_sha256,
        binding.instance.owning_pid,
    )
}

fn load_resp_reference_snapshot(
    request: &ReferencePredecessorRequest,
) -> Result<ReferenceSnapshot, OverloadError> {
    let (report_path, report) = read_reference_report(request, "W3")?;
    validate_common_reference_report(&report, request, "W3")?;
    let capability = report.resp_endpoint_capability.as_ref().ok_or_else(|| {
        OverloadError::Predecessor(
            "W3 reference report has no typed runtime endpoint capability".to_owned(),
        )
    })?;
    let runtime_capability_sha256 = capability
        .digest()
        .map_err(|error| OverloadError::Predecessor(error.to_string()))?;
    let (lifecycle_path, lifecycle_sha256, lifecycle) = read_archived_resp_lifecycle(request)?;
    validate_archived_resp_lifecycle(capability, &runtime_capability_sha256, &lifecycle)?;
    let curve = minimum_valid_capacity_curve(&report, &RESP_CAPACITY_MEASUREMENTS, "W3")?;
    let stable_surface_capability_sha256 =
        NodeRespStableCapability::from_runtime(capability, report.source.cargo_lock_sha256.clone())
            .digest()?;
    snapshot_from_curve(
        request,
        report_path,
        &report,
        curve,
        Some(lifecycle_path),
        Some(lifecycle_sha256),
        stable_surface_capability_sha256,
        runtime_capability_sha256,
        lifecycle.pid,
    )
}

fn read_reference_report(
    request: &ReferencePredecessorRequest,
    work_item: &str,
) -> Result<(PathBuf, PerfReport), OverloadError> {
    let (report_path, report_bytes) = read_bounded_regular_file(
        &request.report_path,
        64 * 1024 * 1024,
        &format!("{work_item} predecessor report"),
    )?;
    if !is_sha256(&request.expected_report_sha256)
        || sha256(&report_bytes) != request.expected_report_sha256
    {
        return Err(OverloadError::Predecessor(format!(
            "{work_item} predecessor report bytes do not match the expected SHA-256"
        )));
    }
    let report = serde_json::from_slice::<PerfReport>(&report_bytes).map_err(|error| {
        OverloadError::Predecessor(format!(
            "{work_item} predecessor report does not match the typed PerfReport schema: {error}"
        ))
    })?;
    Ok((report_path, report))
}

fn validate_common_reference_report(
    report: &PerfReport,
    request: &ReferencePredecessorRequest,
    work_item: &str,
) -> Result<(), OverloadError> {
    let problems = report.validation_problems();
    if !problems.is_empty()
        || report.to_pretty_json().is_err()
        || report.run_mode != EvidenceRunMode::ReferenceEvidence
        || !report.stable
        || !report.stability_reasons.is_empty()
        || report.runner_profile != "reference-v1"
        || report.runner_contract.name != "reference-v1"
        || !report.runner_contract.require_dedicated
        || report.observed_runner.shared_hardware
        || !report.profile_validation.eligible
        || !report.profile_validation.reasons.is_empty()
        || report.surface != request.surface.capacity_surface()
        || !is_git_commit(&report.source.git_commit)
        || !is_sha256(&report.source.cargo_lock_sha256)
        || !is_sha256(&report.build.prebuild_contract_digest)
        || !is_sha256(&report.build.prebuild_manifest_sha256)
        || report.build.prebuild_manifest_sha256 != request.expected_prebuild_manifest_sha256
    {
        return Err(OverloadError::Predecessor(format!(
            "{work_item} predecessor is not stable exact-candidate reference evidence: {problems:?}"
        )));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn snapshot_from_curve(
    request: &ReferencePredecessorRequest,
    report_path: PathBuf,
    report: &PerfReport,
    curve: &crate::report::LoadCurveEvidence,
    lifecycle_path: Option<PathBuf>,
    lifecycle_sha256: Option<String>,
    stable_surface_capability_sha256: String,
    archived_execution_receipt_sha256: String,
    archived_execution_pid: u32,
) -> Result<ReferenceSnapshot, OverloadError> {
    let (prebuild_manifest_path, _) = validate_w7_manifest(
        &request.prebuild_manifest_path,
        &request.expected_prebuild_manifest_sha256,
        report,
    )?;
    let criteria = curve.criteria.clone().ok_or_else(|| {
        OverloadError::Predecessor("selected predecessor has no sustainability criteria".to_owned())
    })?;
    let knee = curve.knee.clone().ok_or_else(|| {
        OverloadError::Predecessor("selected predecessor has no capacity knee".to_owned())
    })?;
    let workload_identity_sha256 = workload_identity_digest(&curve.workload)?;
    let temporary = CapacityPredecessor {
        evidence_class: request.surface.predecessor_class().to_owned(),
        surface: request.surface,
        surface_identity: request.surface.capacity_surface(),
        claim: LoadClaim::CapacityKnee,
        profile: "reference-v1".to_owned(),
        stable_capacity_evidence: true,
        stable_surface_capability_sha256: stable_surface_capability_sha256.clone(),
        workload_identity_sha256: workload_identity_sha256.clone(),
        criteria: criteria.clone(),
        knee: knee.clone(),
        reference_receipt: None,
    };
    Ok(ReferenceSnapshot {
        report_path,
        report_sha256: request.expected_report_sha256.clone(),
        measurement_id: curve.id.clone(),
        scenario_sha256: curve.scenario_digest.clone(),
        source_commit: report.source.git_commit.clone(),
        cargo_lock_sha256: report.source.cargo_lock_sha256.clone(),
        runner_fingerprint_sha256: canonical_digest(&report.observed_runner)?,
        prebuild_manifest_path,
        prebuild_manifest_sha256: request.expected_prebuild_manifest_sha256.clone(),
        lifecycle_path,
        lifecycle_sha256,
        stable_surface_capability_sha256,
        workload_identity_sha256,
        archived_execution_receipt_sha256,
        archived_execution_pid,
        criteria,
        knee,
        payload_sha256: capacity_payload_digest(&temporary)?,
    })
}

fn exact_capacity_curve<'a>(
    report: &'a PerfReport,
    id: &str,
    work_item: &str,
) -> Result<&'a crate::report::LoadCurveEvidence, OverloadError> {
    let mut matches = report
        .measurements
        .iter()
        .filter_map(|measurement| match measurement {
            MeasurementEvidence::LoadCurve(curve) if curve.id == id => Some(curve),
            _ => None,
        });
    let curve = matches.next().ok_or_else(|| {
        OverloadError::Predecessor(format!("{work_item} capacity curve {id} is absent"))
    })?;
    if matches.next().is_some()
        || curve.claim != LoadClaim::CapacityKnee
        || !is_sha256(&curve.scenario_digest)
        || !is_sha256(&curve.workload.digest)
        || curve.knee.as_ref().is_none_or(|knee| {
            knee.sustainable_rate_per_second
                .is_none_or(|rate| !rate.is_finite() || rate <= 0.0 || rate.fract() != 0.0)
        })
    {
        return Err(OverloadError::Predecessor(format!(
            "{work_item} capacity curve {id} is duplicated, unsealed, or has no valid knee"
        )));
    }
    Ok(curve)
}

fn minimum_valid_capacity_curve<'a>(
    report: &'a PerfReport,
    ids: &[&str],
    work_item: &str,
) -> Result<&'a crate::report::LoadCurveEvidence, OverloadError> {
    let mut curves = ids
        .iter()
        .map(|id| exact_capacity_curve(report, id, work_item))
        .collect::<Result<Vec<_>, _>>()?;
    curves.sort_by(|left, right| {
        let left_rate = left
            .knee
            .as_ref()
            .and_then(|knee| knee.sustainable_rate_per_second)
            .unwrap_or(f64::INFINITY);
        let right_rate = right
            .knee
            .as_ref()
            .and_then(|knee| knee.sustainable_rate_per_second)
            .unwrap_or(f64::INFINITY);
        left_rate
            .total_cmp(&right_rate)
            .then_with(|| left.id.cmp(&right.id))
    });
    curves.into_iter().next().ok_or_else(|| {
        OverloadError::Predecessor(format!("{work_item} has no valid A/B/C capacity curve"))
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeRespStableCapability {
    pub schema_version: u32,
    pub surface: SurfaceIdentity,
    pub direct_prebuilt_exec: bool,
    pub fresh_data_dir_per_run: bool,
    pub role: String,
    pub listen_addr: SocketAddr,
    pub cluster_addr: SocketAddr,
    pub admin_enabled: bool,
    pub redis_enabled: bool,
    pub redis_auth_required: bool,
    pub rediss_enabled: bool,
    pub server_binary_sha256: String,
    pub loadgen_binary_sha256: String,
    pub prebuild_manifest_sha256: String,
    pub prebuild_contract_sha256: String,
    pub source_commit: String,
    pub cargo_lock_sha256: String,
}

impl NodeRespStableCapability {
    fn from_runtime(capability: &RespEndpointCapability, cargo_lock_sha256: String) -> Self {
        Self {
            schema_version: NODE_RESP_STABLE_CAPABILITY_VERSION,
            surface: EligibleOverloadSurface::NodeResp.capacity_surface(),
            direct_prebuilt_exec: capability.direct_prebuilt_exec,
            fresh_data_dir_per_run: capability.fresh_data_dir,
            role: capability.config.role.clone(),
            listen_addr: capability.config.listen_addr,
            cluster_addr: capability.config.cluster_addr,
            admin_enabled: capability.config.admin_enabled,
            redis_enabled: capability.config.redis_enabled,
            redis_auth_required: capability.config.redis_auth_required,
            rediss_enabled: capability.config.rediss_enabled,
            server_binary_sha256: capability.server_binary_sha256.clone(),
            loadgen_binary_sha256: capability.loadgen_binary_sha256.clone(),
            prebuild_manifest_sha256: capability.prebuild_manifest_sha256.clone(),
            prebuild_contract_sha256: capability.prebuild_contract_digest.clone(),
            source_commit: capability.source_commit.clone(),
            cargo_lock_sha256,
        }
    }

    fn validate(&self) -> Result<(), OverloadError> {
        if self.schema_version != NODE_RESP_STABLE_CAPABILITY_VERSION
            || self.surface != EligibleOverloadSurface::NodeResp.capacity_surface()
            || !self.direct_prebuilt_exec
            || !self.fresh_data_dir_per_run
            || self.role != "local"
            || self.listen_addr != SocketAddr::from((Ipv4Addr::LOCALHOST, 0))
            || self.cluster_addr != SocketAddr::from((Ipv4Addr::LOCALHOST, 0))
            || !self.admin_enabled
            || !self.redis_enabled
            || self.redis_auth_required
            || self.rediss_enabled
            || !is_sha256(&self.server_binary_sha256)
            || !is_sha256(&self.loadgen_binary_sha256)
            || !is_sha256(&self.prebuild_manifest_sha256)
            || !is_sha256(&self.prebuild_contract_sha256)
            || !is_git_commit(&self.source_commit)
            || !is_sha256(&self.cargo_lock_sha256)
        {
            return Err(OverloadError::Predecessor(
                "W3 stable surface capability is incomplete or contains a runtime identity"
                    .to_owned(),
            ));
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String, OverloadError> {
        self.validate()?;
        canonical_digest(self)
    }
}

fn require_no_lifecycle(
    request: &ReferencePredecessorRequest,
    work_item: &str,
) -> Result<(), OverloadError> {
    if request.lifecycle_path.is_some() || request.expected_lifecycle_sha256.is_some() {
        return Err(OverloadError::Predecessor(format!(
            "{work_item} is in-process and must not be promoted with a daemon lifecycle"
        )));
    }
    Ok(())
}

fn valid_optional_lifecycle(
    surface: EligibleOverloadSurface,
    path: Option<&Path>,
    digest: Option<&str>,
) -> bool {
    match surface {
        EligibleOverloadSurface::Local | EligibleOverloadSurface::ClientSurface => {
            path.is_none() && digest.is_none()
        }
        EligibleOverloadSurface::NodeResp => {
            path.is_some_and(|value| value.is_absolute()) && digest.is_some_and(is_sha256)
        }
    }
}

fn read_archived_resp_lifecycle(
    request: &ReferencePredecessorRequest,
) -> Result<(PathBuf, String, RespDaemonEvidence), OverloadError> {
    let path = request.lifecycle_path.as_deref().ok_or_else(|| {
        OverloadError::Predecessor(
            "W3 predecessor requires its typed killed-and-waited daemon lifecycle".to_owned(),
        )
    })?;
    let expected_sha256 = request
        .expected_lifecycle_sha256
        .as_deref()
        .filter(|digest| is_sha256(digest))
        .ok_or_else(|| {
            OverloadError::Predecessor(
                "W3 predecessor lifecycle requires an expected SHA-256".to_owned(),
            )
        })?;
    let (canonical, bytes) =
        read_bounded_regular_file(path, 16 * 1024 * 1024, "W3 daemon lifecycle")?;
    if sha256(&bytes) != expected_sha256 {
        return Err(OverloadError::Predecessor(
            "W3 daemon lifecycle bytes differ from the expected SHA-256".to_owned(),
        ));
    }
    let lifecycle = serde_json::from_slice::<RespDaemonEvidence>(&bytes).map_err(|error| {
        OverloadError::Predecessor(format!(
            "W3 daemon lifecycle does not match RespDaemonEvidence: {error}"
        ))
    })?;
    Ok((canonical, expected_sha256.to_owned(), lifecycle))
}

fn validate_archived_resp_lifecycle(
    capability: &RespEndpointCapability,
    runtime_capability_sha256: &str,
    lifecycle: &RespDaemonEvidence,
) -> Result<(), OverloadError> {
    if lifecycle.endpoint_capability_digest != runtime_capability_sha256
        || lifecycle.selected_endpoint != capability.selected_endpoint
        || !lifecycle.direct_prebuilt_exec
        || lifecycle.pid != capability.pid
        || lifecycle.repeat_index != capability.repeat_index
        || lifecycle.resp_endpoint != capability.config.redis_addr
        || lifecycle.admin_endpoint != capability.config.admin_addr
        || lifecycle.data_dir != capability.config.storage_dir
        || lifecycle.server_binary_sha256 != capability.server_binary_sha256
        || lifecycle.loadgen_binary_sha256 != capability.loadgen_binary_sha256
        || lifecycle.readiness.selected_endpoint != lifecycle.resp_endpoint
        || lifecycle.readiness.attempts == 0
        || lifecycle.readiness.exact_response != RESP_PONG_DISPLAY
        || lifecycle.readiness.request_sha256 != sha256(RESP_PING_FRAME)
        || lifecycle.readiness.response_sha256 != sha256(RESP_PONG_FRAME)
        || !lifecycle.binaries_verified_after_measurement
        || !lifecycle.killed_and_waited
        || !lifecycle.server_binary_path.is_absolute()
        || !lifecycle.loadgen_binary_path.is_absolute()
        || process_is_alive(lifecycle.pid)
    {
        return Err(OverloadError::Predecessor(
            "W3 archived runtime capability and killed-and-waited lifecycle do not cross-bind"
                .to_owned(),
        ));
    }
    validate_log_receipt(&lifecycle.stdout_log, "W3 stdout")?;
    validate_log_receipt(&lifecycle.stderr_log, "W3 stderr")?;
    Ok(())
}

fn validate_fresh_resp_lifecycle(
    binding: &ReferenceTargetBinding,
    lifecycle: &RespDaemonEvidence,
) -> Result<(), OverloadError> {
    let execution = &binding.execution;
    execution.validate_seal()?;
    let capability = binding.resp_runtime_capability.as_ref().ok_or_else(|| {
        OverloadError::Boundary("fresh RESP lifecycle has no runtime capability".to_owned())
    })?;
    let runtime_capability_sha256 = capability
        .digest()
        .map_err(|error| OverloadError::Boundary(error.to_string()))?;
    if execution.surface != EligibleOverloadSurface::NodeResp
        || execution.kind != ReferenceExecutionKind::DirectDaemon
        || execution.owning_pid != lifecycle.pid
        || execution.runtime_capability_sha256.as_deref()
            != Some(lifecycle.endpoint_capability_digest.as_str())
        || lifecycle.endpoint_capability_digest != runtime_capability_sha256
        || execution.selected_endpoint.as_deref() != Some(lifecycle.selected_endpoint.as_str())
        || lifecycle.pid != capability.pid
        || lifecycle.repeat_index != capability.repeat_index
        || lifecycle.resp_endpoint != capability.config.redis_addr
        || lifecycle.admin_endpoint != capability.config.admin_addr
        || lifecycle.data_dir != capability.config.storage_dir
        || lifecycle.selected_endpoint != capability.selected_endpoint
        || lifecycle.server_binary_sha256 != capability.server_binary_sha256
        || lifecycle.loadgen_binary_sha256 != capability.loadgen_binary_sha256
        || !lifecycle.direct_prebuilt_exec
        || lifecycle.readiness.selected_endpoint != lifecycle.resp_endpoint
        || lifecycle.readiness.attempts == 0
        || lifecycle.readiness.exact_response != RESP_PONG_DISPLAY
        || lifecycle.readiness.request_sha256 != sha256(RESP_PING_FRAME)
        || lifecycle.readiness.response_sha256 != sha256(RESP_PONG_FRAME)
        || !lifecycle.binaries_verified_after_measurement
        || !lifecycle.killed_and_waited
        || process_is_alive(lifecycle.pid)
    {
        return Err(OverloadError::Boundary(
            "fresh node-resp execution receipt is not closed by its exact kill+wait lifecycle"
                .to_owned(),
        ));
    }
    validate_log_receipt(&lifecycle.stdout_log, "W6 node-resp stdout")?;
    validate_log_receipt(&lifecycle.stderr_log, "W6 node-resp stderr")?;
    Ok(())
}

fn validate_log_receipt(
    receipt: &crate::tiers::resp_reference::LogEvidence,
    label: &str,
) -> Result<(), OverloadError> {
    let path = fs::canonicalize(&receipt.canonical_path).map_err(|error| {
        OverloadError::Predecessor(format!(
            "unable to canonicalize {label} {}: {error}",
            receipt.canonical_path.display()
        ))
    })?;
    let metadata = fs::metadata(&path)?;
    if path != receipt.canonical_path
        || !metadata.is_file()
        || metadata.len() != receipt.bytes
        || metadata.len() > 64 * 1024 * 1024
        || !is_sha256(&receipt.sha256)
        || sha256(&fs::read(path)?) != receipt.sha256
    {
        return Err(OverloadError::Predecessor(format!(
            "{label} path/length/SHA receipt does not match its archived file"
        )));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn process_is_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(target_os = "windows")]
fn process_is_alive(pid: u32) -> bool {
    std::process::Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!(
                "if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ exit 0 }} else {{ exit 1 }}"
            ),
        ])
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn process_is_alive(_pid: u32) -> bool {
    true
}

fn workload_identity_digest(workload: &WorkloadIdentity) -> Result<String, OverloadError> {
    if !is_sha256(&workload.digest)
        || workload.generator.trim().is_empty()
        || workload.generator_version.trim().is_empty()
        || workload.operation_mix.is_empty()
    {
        return Err(OverloadError::Predecessor(
            "predecessor workload identity is incomplete or lacks a canonical digest".to_owned(),
        ));
    }
    canonical_digest(workload)
}

fn validate_w7_manifest(
    path: &Path,
    expected_sha256: &str,
    report: &PerfReport,
) -> Result<(PathBuf, W7PrebuildManifest), OverloadError> {
    let (canonical, bytes) = read_bounded_regular_file(path, 1024 * 1024, "W7 prebuild manifest")?;
    if canonical.file_name().and_then(|value| value.to_str()) != Some("prebuild-manifest.json")
        || !is_sha256(expected_sha256)
        || sha256(&bytes) != expected_sha256
    {
        return Err(OverloadError::Predecessor(
            "predecessor does not use the exact hashed W7 prebuild-manifest.json".to_owned(),
        ));
    }
    let manifest = serde_json::from_slice::<W7PrebuildManifest>(&bytes).map_err(|error| {
        OverloadError::Predecessor(format!(
            "W7 prebuild manifest does not match its typed schema: {error}"
        ))
    })?;
    let build_contract = W7BuildContract {
        schema_version: manifest.schema_version,
        toolchain_identity: &manifest.toolchain_identity,
        target_set: &manifest.target_set,
        features: &manifest.features,
        cargo_profile: &manifest.cargo_profile,
        flags: &manifest.flags,
        build_recipe: &manifest.build_recipe,
    };
    let expected_targets = ["hydracache-loadgen", "hydracache-server"];
    let binary_ids = manifest
        .binaries
        .iter()
        .map(|binary| binary.id.as_str())
        .collect::<BTreeSet<_>>();
    let report_binaries = report
        .build
        .binary_sha256
        .iter()
        .map(|(id, digest)| (id.as_str(), digest.as_str()))
        .collect::<BTreeSet<_>>();
    let manifest_binaries = manifest
        .binaries
        .iter()
        .map(|binary| (binary.id.as_str(), binary.sha256.as_str()))
        .collect::<BTreeSet<_>>();
    if manifest.schema_version != 1
        || manifest.source.git_commit != report.source.git_commit
        || manifest.source.cargo_lock_sha256 != report.source.cargo_lock_sha256
        || manifest.toolchain_identity != report.source.toolchain
        || manifest.flags != report.source.build_flags
        || manifest.cargo_profile != "release"
        || manifest
            .target_set
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            != expected_targets
        || manifest.flags.iter().collect::<BTreeSet<_>>().len() != manifest.flags.len()
        || !manifest.flags.iter().any(|flag| flag == "--release")
        || !manifest.flags.iter().any(|flag| flag == "--locked")
        || manifest.build_recipe.is_empty()
        || manifest
            .build_recipe
            .iter()
            .any(|step| step.trim().is_empty())
        || manifest.build_contract_digest != canonical_digest(&build_contract)?
        || manifest.build_contract_digest != report.build.prebuild_contract_digest
        || manifest.runner_profile != report.runner_profile
        || manifest.runner_fingerprint != report.observed_runner.fingerprint
        || manifest.platform_key.trim().is_empty()
        || binary_ids != expected_targets.into_iter().collect::<BTreeSet<_>>()
        || report_binaries != manifest_binaries
    {
        return Err(OverloadError::Predecessor(
            "W7 prebuild manifest does not cross-bind the predecessor source, runner, build contract, and exact binary set"
                .to_owned(),
        ));
    }
    for binary in &manifest.binaries {
        if !portable_identifier(&binary.id)
            || !binary.canonical_path.is_absolute()
            || !is_sha256(&binary.sha256)
        {
            return Err(OverloadError::Predecessor(
                "W7 binary receipt is incomplete".to_owned(),
            ));
        }
        let (observed_path, observed_bytes) =
            read_bounded_regular_file(&binary.canonical_path, 1024 * 1024 * 1024, "W7 binary")?;
        if observed_path != binary.canonical_path || sha256(&observed_bytes) != binary.sha256 {
            return Err(OverloadError::Predecessor(format!(
                "W7 binary {} changed after prebuild",
                binary.id
            )));
        }
    }
    Ok((canonical, manifest))
}

fn read_bounded_regular_file(
    path: &Path,
    maximum_bytes: u64,
    label: &str,
) -> Result<(PathBuf, Vec<u8>), OverloadError> {
    let canonical = fs::canonicalize(path).map_err(|error| {
        OverloadError::Predecessor(format!(
            "unable to canonicalize {label} {}: {error}",
            path.display()
        ))
    })?;
    let metadata = fs::metadata(&canonical)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > maximum_bytes {
        return Err(OverloadError::Predecessor(format!(
            "{label} must be a regular 1..={maximum_bytes}-byte file"
        )));
    }
    Ok((canonical.clone(), fs::read(canonical)?))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionControlMode {
    Enabled,
    DisabledCanary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdmissionControlEvidence {
    pub mode: AdmissionControlMode,
    pub factor_millionths: u32,
    pub authority: String,
    pub configuration_sha256: String,
    pub receipt_sha256: String,
}

impl AdmissionControlEvidence {
    pub fn sealed(
        mode: AdmissionControlMode,
        factor_millionths: u32,
        authority: impl Into<String>,
        configuration_sha256: String,
    ) -> Result<Self, OverloadError> {
        let mut evidence = Self {
            mode,
            factor_millionths,
            authority: authority.into(),
            configuration_sha256,
            receipt_sha256: String::new(),
        };
        evidence.receipt_sha256 = evidence.computed_receipt_sha256()?;
        evidence.validate(factor_millionths)?;
        Ok(evidence)
    }

    fn computed_receipt_sha256(&self) -> Result<String, OverloadError> {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        canonical_digest(&payload)
    }

    fn validate(&self, expected_factor: u32) -> Result<(), OverloadError> {
        if self.factor_millionths != expected_factor
            || !OVERLOAD_FACTORS_MILLIONTHS.contains(&self.factor_millionths)
            || !portable_identifier(&self.authority)
            || !is_sha256(&self.configuration_sha256)
            || self.receipt_sha256 != self.computed_receipt_sha256()?
        {
            return Err(OverloadError::Evidence(
                "admission-control evidence is unsealed or bound to the wrong overload factor"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceTargetContract {
    pub surface: EligibleOverloadSurface,
    pub surface_identity: SurfaceIdentity,
    pub stable_surface_capability_sha256: String,
    pub workload_identity_sha256: String,
    pub source_commit: String,
    pub cargo_lock_sha256: String,
    pub prebuild_manifest_sha256: String,
}

impl ReferenceTargetContract {
    fn validate(&self) -> Result<(), OverloadError> {
        if self.surface_identity != self.surface.capacity_surface()
            || !is_sha256(&self.stable_surface_capability_sha256)
            || !is_sha256(&self.workload_identity_sha256)
            || !is_git_commit(&self.source_commit)
            || !is_sha256(&self.cargo_lock_sha256)
            || !is_sha256(&self.prebuild_manifest_sha256)
        {
            return Err(OverloadError::Boundary(
                "reference target binding is incomplete or differs from its eligible surface"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceExecutionKind {
    InProcess,
    DirectDaemon,
}

/// Identity of the W6 execution itself. It is intentionally distinct from
/// the archived W1/W2 instance or W3 endpoint capability used to derive the
/// knee. In-process runs may share a PID, but never sequence/time/receipt;
/// daemon runs must own a new PID and runtime capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FreshExecutionReceipt {
    pub schema_version: u32,
    pub surface: EligibleOverloadSurface,
    pub kind: ReferenceExecutionKind,
    pub instance_sequence: u64,
    pub owning_pid: u32,
    pub started_unix_nanos: u64,
    pub direct_prebuilt_exec: bool,
    pub stable_surface_capability_sha256: String,
    pub runtime_capability_sha256: Option<String>,
    pub selected_endpoint: Option<String>,
    pub receipt_sha256: String,
}

impl FreshExecutionReceipt {
    fn seal(
        contract: &ReferenceTargetContract,
        kind: ReferenceExecutionKind,
        owning_pid: u32,
        runtime_capability_sha256: Option<String>,
        selected_endpoint: Option<String>,
    ) -> Result<Self, OverloadError> {
        let mut receipt = Self {
            schema_version: FRESH_EXECUTION_RECEIPT_VERSION,
            surface: contract.surface,
            kind,
            instance_sequence: FRESH_EXECUTION_SEQUENCE
                .fetch_add(1, Ordering::Relaxed)
                .saturating_add(1),
            owning_pid,
            started_unix_nanos: unix_nanos_now()?,
            direct_prebuilt_exec: true,
            stable_surface_capability_sha256: contract.stable_surface_capability_sha256.clone(),
            runtime_capability_sha256,
            selected_endpoint,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt.computed_sha256()?;
        receipt.validate_seal()?;
        Ok(receipt)
    }

    fn computed_sha256(&self) -> Result<String, OverloadError> {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        canonical_digest(&payload)
    }

    fn validate_seal(&self) -> Result<(), OverloadError> {
        let runtime_shape_is_valid = match (self.surface, self.kind) {
            (
                EligibleOverloadSurface::Local | EligibleOverloadSurface::ClientSurface,
                ReferenceExecutionKind::InProcess,
            ) => {
                self.owning_pid == std::process::id()
                    && self.runtime_capability_sha256.is_none()
                    && self.selected_endpoint.is_none()
            }
            (EligibleOverloadSurface::NodeResp, ReferenceExecutionKind::DirectDaemon) => {
                self.runtime_capability_sha256
                    .as_deref()
                    .is_some_and(is_sha256)
                    && self
                        .selected_endpoint
                        .as_deref()
                        .is_some_and(|endpoint| !endpoint.trim().is_empty())
            }
            _ => false,
        };
        if self.schema_version != FRESH_EXECUTION_RECEIPT_VERSION
            || self.instance_sequence == 0
            || self.owning_pid == 0
            || self.started_unix_nanos == 0
            || !self.direct_prebuilt_exec
            || !is_sha256(&self.stable_surface_capability_sha256)
            || !runtime_shape_is_valid
            || self.receipt_sha256 != self.computed_sha256()?
        {
            return Err(OverloadError::Boundary(
                "fresh W6 execution receipt is unsealed or crosses its surface boundary".to_owned(),
            ));
        }
        Ok(())
    }

    fn validate_fresh_against(
        &self,
        contract: &ReferenceTargetContract,
        archived: &ReferencePredecessorReceipt,
    ) -> Result<(), OverloadError> {
        self.validate_seal()?;
        if self.surface != contract.surface
            || self.stable_surface_capability_sha256 != contract.stable_surface_capability_sha256
            || self.receipt_sha256 == archived.archived_execution_receipt_sha256
            || self.surface == EligibleOverloadSurface::NodeResp
                && self.owning_pid == archived.archived_execution_pid
        {
            return Err(OverloadError::Boundary(
                "W6 execution reuses its archived predecessor runtime identity".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceTargetBinding {
    pub contract: ReferenceTargetContract,
    pub execution: FreshExecutionReceipt,
    pub resp_runtime_capability: Option<RespEndpointCapability>,
}

impl ReferenceTargetBinding {
    fn validate(&self, predecessor: &CapacityPredecessor) -> Result<(), OverloadError> {
        self.contract.validate()?;
        if self.contract != predecessor.reference_target_contract()? {
            return Err(OverloadError::Boundary(
                "reference target stable surface/workload contract differs from its predecessor"
                    .to_owned(),
            ));
        }
        self.execution
            .validate_fresh_against(&self.contract, predecessor.reference_receipt()?)?;
        match self.contract.surface {
            EligibleOverloadSurface::Local | EligibleOverloadSurface::ClientSurface => {
                if self.resp_runtime_capability.is_some() {
                    return Err(OverloadError::Boundary(
                        "in-process W6 binding cannot carry a RESP runtime capability".to_owned(),
                    ));
                }
            }
            EligibleOverloadSurface::NodeResp => {
                let capability = self.resp_runtime_capability.as_ref().ok_or_else(|| {
                    OverloadError::Boundary(
                        "node-resp W6 binding has no typed fresh runtime capability".to_owned(),
                    )
                })?;
                let runtime_digest = capability
                    .digest()
                    .map_err(|error| OverloadError::Boundary(error.to_string()))?;
                let stable_digest = NodeRespStableCapability::from_runtime(
                    capability,
                    self.contract.cargo_lock_sha256.clone(),
                )
                .digest()?;
                if self.execution.owning_pid != capability.pid
                    || self.execution.runtime_capability_sha256.as_deref()
                        != Some(runtime_digest.as_str())
                    || self.execution.selected_endpoint.as_deref()
                        != Some(capability.selected_endpoint.as_str())
                    || stable_digest != self.contract.stable_surface_capability_sha256
                {
                    return Err(OverloadError::Boundary(
                        "fresh RESP runtime capability differs from execution/stable contract"
                            .to_owned(),
                    ));
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
pub trait OverloadWindowControl: Send + Sync {
    async fn prepare_overload_window(
        &self,
        factor_millionths: u32,
    ) -> Result<AdmissionControlEvidence, OverloadError>;

    async fn prepare_recovery_window(&self) -> Result<(), OverloadError>;
}

/// Explicit producer seam for real process/library adapters. Generic
/// [`Target`] implementations are accepted only by the smoke runner.
#[allow(dead_code)] // Used once the receipt-bound W6 tier adapter is wired into the shared CLI.
pub trait ReferenceOverloadAdapter: Target + OverloadWindowControl {
    fn reference_target_binding(&self) -> Result<ReferenceTargetBinding, OverloadError>;
}

/// Loadgen-owned capacity gate around a real product target. It rejects only
/// the offered excess above the predecessor knee; accepted requests still
/// traverse the exact local/router/RESP surface. The receipt names this
/// authority explicitly, so it cannot be mistaken for a product admission
/// implementation.
#[derive(Debug)]
pub struct CapacityBoundReferenceAdapter<T> {
    target: Arc<T>,
    binding: ReferenceTargetBinding,
    factor_millionths: AtomicU32,
    knee_rate_per_second: u64,
}

impl<T> CapacityBoundReferenceAdapter<T>
where
    T: Target,
{
    fn new(
        target: Arc<T>,
        binding: ReferenceTargetBinding,
        knee_rate_per_second: u64,
    ) -> Result<Self, OverloadError> {
        binding.contract.validate()?;
        binding.execution.validate_seal()?;
        if knee_rate_per_second == 0 {
            return Err(OverloadError::Predecessor(
                "reference overload adapter requires a positive capacity knee".to_owned(),
            ));
        }
        Ok(Self {
            target,
            binding,
            factor_millionths: AtomicU32::new(1_000_000),
            knee_rate_per_second,
        })
    }

    fn accepts(&self, sequence: u64) -> bool {
        match self.factor_millionths.load(Ordering::SeqCst) {
            0..=1_000_000 => true,
            1_200_000 => sequence % 6 != 5,
            1_500_000 => sequence % 3 != 2,
            2_000_000 => sequence.is_multiple_of(2),
            _ => false,
        }
    }
}

#[async_trait]
impl<T> Target for CapacityBoundReferenceAdapter<T>
where
    T: Target,
{
    async fn reset(&self) -> Result<String, TargetError> {
        self.factor_millionths.store(1_000_000, Ordering::SeqCst);
        self.target.reset().await
    }

    async fn preload(&self) -> Result<PreloadOutcome, TargetError> {
        self.target.preload().await
    }

    async fn state_digest(&self) -> Result<String, TargetError> {
        self.target.state_digest().await
    }

    async fn execute(&self, request: TargetRequest) -> TargetOutcome {
        if self.accepts(request.sequence) {
            self.target.execute(request).await
        } else {
            TargetOutcome::Rejected
        }
    }
}

#[async_trait]
impl<T> OverloadWindowControl for CapacityBoundReferenceAdapter<T>
where
    T: Target,
{
    async fn prepare_overload_window(
        &self,
        factor_millionths: u32,
    ) -> Result<AdmissionControlEvidence, OverloadError> {
        if !OVERLOAD_FACTORS_MILLIONTHS.contains(&factor_millionths) {
            return Err(OverloadError::Contract(
                "reference capacity gate received an unsupported overload factor".to_owned(),
            ));
        }
        self.factor_millionths
            .store(factor_millionths, Ordering::SeqCst);
        AdmissionControlEvidence::sealed(
            AdmissionControlMode::Enabled,
            factor_millionths,
            "loadgen-capacity-bound-admission",
            canonical_digest(&(
                "w6-loadgen-capacity-bound-admission-v1",
                &self.binding.contract,
                self.knee_rate_per_second,
                ["five-of-six", "two-of-three", "one-of-two"],
            ))?,
        )
    }

    async fn prepare_recovery_window(&self) -> Result<(), OverloadError> {
        self.factor_millionths.store(1_000_000, Ordering::SeqCst);
        Ok(())
    }
}

impl<T> ReferenceOverloadAdapter for CapacityBoundReferenceAdapter<T>
where
    T: Target,
{
    fn reference_target_binding(&self) -> Result<ReferenceTargetBinding, OverloadError> {
        Ok(self.binding.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceOverloadPublication {
    pub report_path: PathBuf,
    pub report_sha256: String,
    pub fresh_execution_receipt_sha256: String,
    pub daemon_lifecycle_sha256: Option<String>,
}

/// Execute one exact W6 DoD surface with explicit repository, context,
/// predecessor, daemon-evidence, scenario, and output paths. No Cargo command
/// is spawned during measurement; the W7 context is rehashed before and after.
#[allow(clippy::too_many_arguments)]
pub async fn write_reference_overload_report(
    repo_root: &Path,
    context: &ValidatedRespReferenceContext,
    scenario_path: &Path,
    predecessor_request: ReferencePredecessorRequest,
    evidence_root: &Path,
    report_path: &Path,
) -> Result<ReferenceOverloadPublication, OverloadError> {
    require_reference_surface_gate(predecessor_request.surface)?;
    let repo_root = canonical_existing_directory(repo_root, "repository root")?;
    if repo_root != context.repo_root {
        return Err(OverloadError::Boundary(
            "W6 repository root differs from the validated W7 context".to_owned(),
        ));
    }
    context
        .verify_binaries_unchanged()
        .map_err(|error| OverloadError::Predecessor(error.to_string()))?;
    if fs::canonicalize(&predecessor_request.prebuild_manifest_path)
        .ok()
        .as_ref()
        != Some(&context.manifest_path)
        || predecessor_request.expected_prebuild_manifest_sha256 != context.manifest_sha256
    {
        return Err(OverloadError::Predecessor(
            "W6 predecessor does not use the active validated prebuild context".to_owned(),
        ));
    }
    let scenario_path = fs::canonicalize(scenario_path).map_err(|error| {
        OverloadError::Contract(format!(
            "unable to canonicalize W6 scenario {}: {error}",
            scenario_path.display()
        ))
    })?;
    let expected_scenario_path =
        repo_root.join("docs/testing/perf-scenarios/0.67/overload-capacity-v1.toml");
    if scenario_path != fs::canonicalize(expected_scenario_path)? {
        return Err(OverloadError::Contract(
            "W6 reference producer requires the committed overload-capacity-v1 scenario".to_owned(),
        ));
    }
    let scenario = OverloadScenario::load(&scenario_path)?;
    let evidence_root = ensure_absolute_directory(evidence_root, "W6 evidence root")?;
    let report_path = absolute_new_output_path(report_path, "W6 report")?;
    let predecessor = load_reference_predecessor(&scenario, predecessor_request)?;
    validate_context_contract(context, &predecessor.reference_target_contract()?)?;

    let report = match predecessor.surface {
        EligibleOverloadSurface::Local => {
            run_local_reference_overload(context, &scenario, predecessor).await?
        }
        EligibleOverloadSurface::ClientSurface => {
            run_client_reference_overload(context, &scenario, predecessor).await?
        }
        EligibleOverloadSurface::NodeResp => {
            run_resp_reference_overload(context, &scenario, predecessor, &evidence_root).await?
        }
    };
    context
        .verify_binaries_unchanged()
        .map_err(|error| OverloadError::Evidence(error.to_string()))?;
    report.validate(&scenario)?;
    let report_bytes = report.to_pretty_json(&scenario)?;
    let report_sha256 = sha256(&report_bytes);
    let execution_sha256 = report
        .target_binding
        .as_ref()
        .ok_or_else(|| OverloadError::Evidence("W6 report lost its execution binding".to_owned()))?
        .execution
        .receipt_sha256
        .clone();
    let daemon_lifecycle_sha256 = report
        .daemon_lifecycle
        .as_ref()
        .map(canonical_digest)
        .transpose()?;
    write_new_bytes_atomic(&report_path, &report_bytes)?;
    Ok(ReferenceOverloadPublication {
        report_path,
        report_sha256,
        fresh_execution_receipt_sha256: execution_sha256,
        daemon_lifecycle_sha256,
    })
}

fn required_reference_gate(surface: EligibleOverloadSurface) -> &'static str {
    match surface {
        EligibleOverloadSurface::Local | EligibleOverloadSurface::ClientSurface => {
            W6_CORE_REFERENCE_ENV
        }
        EligibleOverloadSurface::NodeResp => W6_RESP_REFERENCE_ENV,
    }
}

fn validate_reference_surface_gate_with<F>(
    surface: EligibleOverloadSurface,
    lookup: F,
) -> Result<(), OverloadError>
where
    F: FnOnce(&str) -> Option<String>,
{
    let variable = required_reference_gate(surface);
    if lookup(variable).as_deref() != Some("1") {
        return Err(OverloadError::Boundary(format!(
            "W6 reference surface {} requires exact {variable}=1",
            surface_cli_name(surface)
        )));
    }
    Ok(())
}

fn require_reference_surface_gate(surface: EligibleOverloadSurface) -> Result<(), OverloadError> {
    validate_reference_surface_gate_with(surface, |variable| std::env::var(variable).ok())
}

async fn run_local_reference_overload(
    _context: &ValidatedRespReferenceContext,
    scenario: &OverloadScenario,
    predecessor: CapacityPredecessor,
) -> Result<OverloadReport, OverloadError> {
    let receipt = predecessor.reference_receipt()?;
    if receipt.predecessor_measurement_id != LOCAL_CAPACITY_MEASUREMENT {
        return Err(OverloadError::Predecessor(
            "local W6 adapter requires the W1 hot-key capacity predecessor".to_owned(),
        ));
    }
    let (capacity_scenario, input) =
        parse_adapter_scenario::<LocalAdapterInputs>(LOCAL_HOT_KEY_SCENARIO, "local")?;
    validate_local_adapter_input(&capacity_scenario, &input)?;
    let expected_preload = scenario
        .expected_preload_operations(EligibleOverloadSurface::Local, OverloadRunMode::Reference);
    if capacity_scenario.preload_operations != expected_preload {
        return Err(OverloadError::Boundary(
            "W6 local preload differs from the exact selected W1 workload".to_owned(),
        ));
    }
    let target = Arc::new(LocalCacheTarget::new(LocalTargetConfig {
        preload_entries: capacity_scenario.preload_operations,
        key_space: input.key_count,
        payload_bytes: usize::try_from(input.payload_bytes).map_err(|_| {
            OverloadError::Boundary("W1 payload size does not fit usize".to_owned())
        })?,
        operation_mix: LocalOperationMix {
            hit_percent: 0,
            miss_percent: 0,
            loader_percent: 0,
            put_percent: 0,
            hot_key_percent: 100,
        },
        loader_delay: Duration::from_micros(input.loader_delay_us),
        ..LocalTargetConfig::default()
    })?);
    let contract = predecessor.reference_target_contract()?;
    let execution = FreshExecutionReceipt::seal(
        &contract,
        ReferenceExecutionKind::InProcess,
        std::process::id(),
        None,
        None,
    )?;
    let adapter = Arc::new(CapacityBoundReferenceAdapter::new(
        target,
        ReferenceTargetBinding {
            contract,
            execution,
            resp_runtime_capability: None,
        },
        predecessor.knee_rate_per_second()?,
    )?);
    run_reference_overload_curve(adapter, scenario, predecessor).await
}

async fn run_client_reference_overload(
    _context: &ValidatedRespReferenceContext,
    scenario: &OverloadScenario,
    predecessor: CapacityPredecessor,
) -> Result<OverloadReport, OverloadError> {
    let measurement_id = predecessor
        .reference_receipt()?
        .predecessor_measurement_id
        .clone();
    let source = client_scenario_source(&measurement_id)?;
    let (capacity_scenario, input) =
        parse_adapter_scenario::<ClientAdapterInputs>(source, "client_surface")?;
    let operation_mix = validate_client_adapter_input(&capacity_scenario, &input)?;
    let expected_preload = scenario.expected_preload_operations(
        EligibleOverloadSurface::ClientSurface,
        OverloadRunMode::Reference,
    );
    if capacity_scenario.preload_operations != expected_preload {
        return Err(OverloadError::Boundary(
            "W6 client preload differs from the exact selected W2 workload".to_owned(),
        ));
    }
    let schedule = KeyScheduleSpec::uniform(
        capacity_scenario.seed,
        input.key_count,
        capacity_scenario
            .preload_operations
            .max(capacity_scenario.warmup_operations)
            .max(capacity_scenario.steady_operations),
    )
    .generate()
    .map_err(OverloadError::Boundary)?;
    let target = Arc::new(ClientSurfaceTarget::new(ClientSurfaceTargetConfig {
        limits: ClientSurfaceLimits {
            max_frame_bytes: input.max_frame_bytes,
            ..ClientSurfaceLimits::default()
        },
        preload_entries: capacity_scenario.preload_operations,
        key_space: input.key_count,
        payload_bytes: usize::try_from(input.payload_bytes).map_err(|_| {
            OverloadError::Boundary("W2 payload size does not fit usize".to_owned())
        })?,
        batch_size: input.batch_size,
        operation_mix,
        key_schedule: Arc::new(schedule.keys),
        injected_dispatch_delay: Duration::ZERO,
    })?);
    let contract = predecessor.reference_target_contract()?;
    let execution = FreshExecutionReceipt::seal(
        &contract,
        ReferenceExecutionKind::InProcess,
        std::process::id(),
        None,
        None,
    )?;
    let adapter = Arc::new(CapacityBoundReferenceAdapter::new(
        target,
        ReferenceTargetBinding {
            contract,
            execution,
            resp_runtime_capability: None,
        },
        predecessor.knee_rate_per_second()?,
    )?);
    run_reference_overload_curve(adapter, scenario, predecessor).await
}

async fn run_resp_reference_overload(
    context: &ValidatedRespReferenceContext,
    scenario: &OverloadScenario,
    predecessor: CapacityPredecessor,
    evidence_root: &Path,
) -> Result<OverloadReport, OverloadError> {
    let measurement_id = predecessor
        .reference_receipt()?
        .predecessor_measurement_id
        .clone();
    let source = resp_scenario_source(&measurement_id)?;
    let (capacity_scenario, input) = parse_adapter_scenario::<RespAdapterInputs>(source, "resp")?;
    let operation_mix = validate_resp_adapter_input(&capacity_scenario, &input)?;
    let expected_preload = scenario.expected_preload_operations(
        EligibleOverloadSurface::NodeResp,
        OverloadRunMode::Reference,
    );
    if capacity_scenario.preload_operations != expected_preload {
        return Err(OverloadError::Boundary(
            "W6 RESP preload differs from the exact selected W3 workload".to_owned(),
        ));
    }
    let schedule = KeyScheduleSpec::uniform(
        capacity_scenario.seed,
        input.key_count,
        capacity_scenario.steady_operations,
    )
    .generate()
    .map_err(OverloadError::Boundary)?;
    let ports = RespReferencePorts::select_available()
        .map_err(|error| OverloadError::Measurement(error.to_string()))?;
    let launch = RespDaemonLaunch {
        repeat_index: 6_700_006,
        ports,
        evidence_root: evidence_root.to_path_buf(),
        startup_timeout: Duration::from_secs(20),
        ping_interval: Duration::from_millis(25),
    };
    let fixture = start_reference_daemon(context, &launch)
        .await
        .map_err(|error| OverloadError::Measurement(error.to_string()))?;
    let runtime_capability = fixture.endpoint_capability().clone();
    let runtime_capability_sha256 = runtime_capability
        .digest()
        .map_err(|error| OverloadError::Boundary(error.to_string()))?;
    let contract = predecessor.reference_target_contract()?;
    let fresh_stable_capability = NodeRespStableCapability::from_runtime(
        &runtime_capability,
        context.source.cargo_lock_sha256.clone(),
    )
    .digest()?;
    if fresh_stable_capability != contract.stable_surface_capability_sha256 {
        return Err(OverloadError::Boundary(
            "fresh W6 RESP daemon differs from the stable W3 surface/build contract".to_owned(),
        ));
    }
    let execution = FreshExecutionReceipt::seal(
        &contract,
        ReferenceExecutionKind::DirectDaemon,
        runtime_capability.pid,
        Some(runtime_capability_sha256.clone()),
        Some(runtime_capability.selected_endpoint.clone()),
    )?;
    execution.validate_fresh_against(&contract, predecessor.reference_receipt()?)?;
    let target = Arc::new(
        RespTcpTarget::new(RespTargetConfig {
            endpoint: fixture.endpoint_identity(),
            require_loopback: true,
            connections: input.connections,
            pipeline_depth: input.pipeline,
            preload_entries: capacity_scenario.preload_operations,
            key_space: input.key_count,
            payload_bytes: usize::try_from(input.payload_bytes).map_err(|_| {
                OverloadError::Boundary("W3 payload size does not fit usize".to_owned())
            })?,
            batch_size: input.batch_size,
            reset_batch_entries: 128,
            operation_mix,
            key_schedule: Arc::new(schedule.keys),
            connect_timeout: Duration::from_secs(2),
            io_timeout: Duration::from_secs(2),
            parser_limits: Resp2Limits::default(),
            injected_dispatch_delay: Duration::ZERO,
        })
        .map_err(|error| OverloadError::Measurement(error.to_string()))?,
    );
    let binding = ReferenceTargetBinding {
        contract,
        execution,
        resp_runtime_capability: Some(runtime_capability.clone()),
    };
    let adapter = Arc::new(CapacityBoundReferenceAdapter::new(
        target,
        binding.clone(),
        predecessor.knee_rate_per_second()?,
    )?);
    let measured = run_overload_curve_inner(
        Arc::clone(&adapter),
        adapter.as_ref(),
        scenario,
        predecessor,
        OverloadRunMode::Reference,
        Some(binding),
    )
    .await;
    drop(adapter);
    let lifecycle = fixture.stop().await;
    let (mut report, lifecycle) = match (measured, lifecycle) {
        (Ok(report), Ok(lifecycle)) => {
            if lifecycle.endpoint_capability_digest != runtime_capability_sha256 {
                return Err(OverloadError::Evidence(
                    "fresh W6 daemon lifecycle lost its runtime capability binding".to_owned(),
                ));
            }
            (report, lifecycle)
        }
        (Err(measurement), Ok(_)) => return Err(measurement),
        (Ok(_), Err(cleanup)) => {
            return Err(OverloadError::Evidence(format!(
                "W6 RESP measurement completed but daemon cleanup failed: {cleanup}"
            )))
        }
        (Err(measurement), Err(cleanup)) => {
            return Err(OverloadError::Evidence(format!(
                "{measurement}; fresh W6 daemon cleanup also failed: {cleanup}"
            )))
        }
    };
    match report.target_binding.as_ref() {
        Some(binding)
            if binding.execution.runtime_capability_sha256.as_deref()
                == Some(runtime_capability_sha256.as_str()) => {}
        _ => {
            return Err(OverloadError::Evidence(
                "W6 report lost the fresh RESP execution receipt".to_owned(),
            ))
        }
    }
    report.daemon_lifecycle = Some(lifecycle);
    report.validate(scenario)?;
    Ok(report)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdapterOperationInput {
    operation: String,
    weight: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalAdapterInputs {
    key_count: u64,
    payload_bytes: u64,
    distribution: String,
    worker_counts: Vec<usize>,
    loader_delay_us: u64,
    operation_mix: Vec<AdapterOperationInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClientAdapterInputs {
    workload: String,
    key_count: u64,
    payload_bytes: u64,
    batch_size: usize,
    max_frame_bytes: usize,
    operation_mix: Vec<AdapterOperationInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RespAdapterInputs {
    workload: String,
    key_count: u64,
    payload_bytes: u64,
    batch_size: usize,
    connections: usize,
    pipeline: usize,
    repeat_isolation: String,
    daemon_reused_across_repeats: bool,
    operation_mix: Vec<AdapterOperationInput>,
}

fn parse_adapter_scenario<T>(source: &[u8], section: &str) -> Result<(Scenario, T), OverloadError>
where
    T: for<'de> Deserialize<'de>,
{
    let text =
        std::str::from_utf8(source).map_err(|error| OverloadError::Boundary(error.to_string()))?;
    let mut root = text
        .parse::<toml::Table>()
        .map_err(|error| OverloadError::Boundary(error.to_string()))?;
    let input: T = root
        .remove(section)
        .ok_or_else(|| OverloadError::Boundary(format!("missing [{section}] adapter input")))?
        .try_into()
        .map_err(|error| OverloadError::Boundary(error.to_string()))?;
    let scenario: Scenario = toml::Value::Table(root)
        .try_into()
        .map_err(|error| OverloadError::Boundary(error.to_string()))?;
    scenario.validate().map_err(OverloadError::Boundary)?;
    Ok((scenario, input))
}

fn validate_local_adapter_input(
    scenario: &Scenario,
    input: &LocalAdapterInputs,
) -> Result<(), OverloadError> {
    if scenario.id != "local-hot-key-v1"
        || input.key_count != 1
        || input.payload_bytes != 128
        || input.distribution != "single-key"
        || input.worker_counts.as_slice() != [1, 2, 4].as_slice()
        || input.loader_delay_us != 1_000
        || input.operation_mix.len() != 1
        || input.operation_mix[0].operation != "hot-key-get-or-insert"
        || input.operation_mix[0].weight != 1.0
    {
        return Err(OverloadError::Boundary(
            "W6 local adapter differs from the committed W1 hot-key workload".to_owned(),
        ));
    }
    Ok(())
}

fn validate_client_adapter_input(
    scenario: &Scenario,
    input: &ClientAdapterInputs,
) -> Result<ClientSurfaceOperationMix, OverloadError> {
    validate_operation_set(
        &input.operation_mix,
        &["get", "put", "batch_get", "batch_put"],
    )?;
    if !matches!(input.workload.as_str(), "A" | "B" | "C")
        || scenario.id != format!("client-surface-{}-v1", input.workload.to_ascii_lowercase())
        || input.key_count != 10_000
        || input.payload_bytes != 1_000
        || input.batch_size != 8
        || input.max_frame_bytes != 1_048_576
    {
        return Err(OverloadError::Boundary(
            "W6 client adapter differs from its committed W2 A/B/C workload".to_owned(),
        ));
    }
    let mix = ClientSurfaceOperationMix {
        get_percent: operation_percent(&input.operation_mix, "get")?,
        put_percent: operation_percent(&input.operation_mix, "put")?,
        batch_get_percent: operation_percent(&input.operation_mix, "batch_get")?,
        batch_put_percent: operation_percent(&input.operation_mix, "batch_put")?,
    };
    let expected = match input.workload.as_str() {
        "A" => ClientSurfaceOperationMix::WORKLOAD_A,
        "B" => ClientSurfaceOperationMix::WORKLOAD_B,
        "C" => ClientSurfaceOperationMix::WORKLOAD_C,
        _ => unreachable!(),
    };
    if mix != expected {
        return Err(OverloadError::Boundary(
            "W6 client operation mix differs from the selected W2 predecessor".to_owned(),
        ));
    }
    Ok(mix)
}

fn validate_resp_adapter_input(
    scenario: &Scenario,
    input: &RespAdapterInputs,
) -> Result<RespOperationMix, OverloadError> {
    validate_operation_set(&input.operation_mix, &["get", "set", "mget", "mset"])?;
    if !matches!(input.workload.as_str(), "A" | "B" | "C")
        || scenario.id
            != format!(
                "resp-open-loop-workload-{}-v1",
                input.workload.to_ascii_lowercase()
            )
        || input.key_count != 10_000
        || input.payload_bytes != 256
        || input.batch_size != 10
        || input.connections != 10
        || input.pipeline != 1
        || input.repeat_isolation != "logical-keyspace-reset-and-counter-zero"
        || !input.daemon_reused_across_repeats
    {
        return Err(OverloadError::Boundary(
            "W6 RESP adapter differs from its committed W3 A/B/C workload".to_owned(),
        ));
    }
    let mix = RespOperationMix {
        get_percent: operation_percent(&input.operation_mix, "get")?,
        set_percent: operation_percent(&input.operation_mix, "set")?,
        mget_percent: operation_percent(&input.operation_mix, "mget")?,
        mset_percent: operation_percent(&input.operation_mix, "mset")?,
    };
    let expected = match input.workload.as_str() {
        "A" => RespOperationMix::WORKLOAD_A,
        "B" => RespOperationMix::WORKLOAD_B,
        "C" => RespOperationMix::WORKLOAD_C,
        _ => unreachable!(),
    };
    if mix != expected {
        return Err(OverloadError::Boundary(
            "W6 RESP operation mix differs from the selected W3 predecessor".to_owned(),
        ));
    }
    Ok(mix)
}

fn operation_percent(
    operations: &[AdapterOperationInput],
    name: &str,
) -> Result<u8, OverloadError> {
    let matches = operations
        .iter()
        .filter(|operation| operation.operation == name)
        .collect::<Vec<_>>();
    if matches.len() > 1
        || operations.iter().any(|operation| {
            !operation.weight.is_finite() || operation.weight <= 0.0 || operation.weight > 1.0
        })
    {
        return Err(OverloadError::Boundary(
            "adapter operation mix is duplicated or outside 0..=1".to_owned(),
        ));
    }
    Ok(matches
        .first()
        .map_or(0, |operation| (operation.weight * 100.0).round() as u8))
}

fn validate_operation_set(
    operations: &[AdapterOperationInput],
    allowed: &[&str],
) -> Result<(), OverloadError> {
    let names = operations
        .iter()
        .map(|operation| operation.operation.as_str())
        .collect::<BTreeSet<_>>();
    let total = operations
        .iter()
        .map(|operation| operation.weight)
        .sum::<f64>();
    if operations.is_empty()
        || names.len() != operations.len()
        || !names.iter().all(|name| allowed.contains(name))
        || operations.iter().any(|operation| {
            !operation.weight.is_finite() || operation.weight <= 0.0 || operation.weight > 1.0
        })
        || (total - 1.0).abs() > 1e-9
    {
        return Err(OverloadError::Boundary(
            "adapter operation set is unknown, duplicated, or does not total 1.0".to_owned(),
        ));
    }
    Ok(())
}

fn client_scenario_source(measurement_id: &str) -> Result<&'static [u8], OverloadError> {
    match measurement_id {
        "client_surface_in_process_knee_at_slo_workload_a" => Ok(CLIENT_A_SCENARIO),
        "client_surface_in_process_knee_at_slo_workload_b" => Ok(CLIENT_B_SCENARIO),
        "client_surface_in_process_knee_at_slo_workload_c" => Ok(CLIENT_C_SCENARIO),
        _ => Err(OverloadError::Predecessor(
            "W2 predecessor selection is not one of the exact A/B/C curves".to_owned(),
        )),
    }
}

fn resp_scenario_source(measurement_id: &str) -> Result<&'static [u8], OverloadError> {
    match measurement_id {
        "resp_open_loop_get_set_knee_at_slo_workload_a" => Ok(RESP_A_SCENARIO),
        "resp_open_loop_get_set_knee_at_slo_workload_b" => Ok(RESP_B_SCENARIO),
        "resp_open_loop_get_set_knee_at_slo_workload_c" => Ok(RESP_C_SCENARIO),
        _ => Err(OverloadError::Predecessor(
            "W3 predecessor selection is not one of the exact A/B/C curves".to_owned(),
        )),
    }
}

fn validate_context_contract(
    context: &ValidatedRespReferenceContext,
    contract: &ReferenceTargetContract,
) -> Result<(), OverloadError> {
    contract.validate()?;
    if context.source.git_commit != contract.source_commit
        || context.source.cargo_lock_sha256 != contract.cargo_lock_sha256
        || context.manifest_sha256 != contract.prebuild_manifest_sha256
        || context.build.prebuild_manifest_sha256 != contract.prebuild_manifest_sha256
        || context.profile.name != "reference-v1"
        || context.runner.shared_hardware
    {
        return Err(OverloadError::Boundary(
            "fresh W6 adapter context differs from the predecessor candidate/profile".to_owned(),
        ));
    }
    Ok(())
}

fn canonical_existing_directory(path: &Path, label: &str) -> Result<PathBuf, OverloadError> {
    let canonical = fs::canonicalize(path).map_err(|error| {
        OverloadError::Contract(format!(
            "unable to canonicalize {label} {}: {error}",
            path.display()
        ))
    })?;
    if !fs::metadata(&canonical)?.is_dir() {
        return Err(OverloadError::Contract(format!(
            "{label} is not a directory"
        )));
    }
    Ok(canonical)
}

fn ensure_absolute_directory(path: &Path, label: &str) -> Result<PathBuf, OverloadError> {
    if !path.is_absolute() {
        return Err(OverloadError::Contract(format!("{label} must be absolute")));
    }
    fs::create_dir_all(path)?;
    canonical_existing_directory(path, label)
}

fn absolute_new_output_path(path: &Path, label: &str) -> Result<PathBuf, OverloadError> {
    if !path.is_absolute() || path.file_name().is_none() {
        return Err(OverloadError::Contract(format!(
            "{label} must be an absolute file path"
        )));
    }
    let parent = path
        .parent()
        .ok_or_else(|| OverloadError::Contract(format!("{label} has no parent directory")))?;
    fs::create_dir_all(parent)?;
    let canonical_parent = fs::canonicalize(parent)?;
    let output = canonical_parent.join(path.file_name().expect("checked above"));
    if output.exists() {
        return Err(OverloadError::Evidence(format!(
            "{label} {} already exists; reference evidence is create-new",
            output.display()
        )));
    }
    Ok(output)
}

fn write_new_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), OverloadError> {
    let parent = path
        .parent()
        .ok_or_else(|| OverloadError::Evidence("atomic W6 report path has no parent".to_owned()))?;
    let nonce = unix_nanos_now()?;
    let temporary = parent.join(format!(
        ".{}.pid-{}.nanos-{nonce}.part",
        path.file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| OverloadError::Evidence("W6 report name is not UTF-8".to_owned()))?,
        std::process::id()
    ));
    let result = (|| -> Result<(), OverloadError> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        if path.exists() {
            return Err(OverloadError::Evidence(
                "W6 report destination appeared before atomic publication".to_owned(),
            ));
        }
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() && temporary.exists() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn unix_nanos_now() -> Result<u64, OverloadError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| OverloadError::Evidence(error.to_string()))?
        .as_nanos()
        .try_into()
        .map_err(|_| OverloadError::Evidence("system time does not fit u64 nanos".to_owned()))
}

#[derive(Debug, Default)]
pub struct PassthroughWindowControl;

#[async_trait]
impl OverloadWindowControl for PassthroughWindowControl {
    async fn prepare_overload_window(
        &self,
        factor_millionths: u32,
    ) -> Result<AdmissionControlEvidence, OverloadError> {
        AdmissionControlEvidence::sealed(
            AdmissionControlMode::Enabled,
            factor_millionths,
            "passthrough-smoke-control",
            sha256(b"passthrough-smoke-control-v1"),
        )
    }

    async fn prepare_recovery_window(&self) -> Result<(), OverloadError> {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverloadMetrics {
    pub successful_goodput_per_second: f64,
    pub scheduled_p99_us: u64,
    pub rejection_ratio: f64,
    pub error_timeout_ratio: f64,
    pub backlog_high_water: u64,
    pub backlog_drained: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryEvidence {
    pub transition_duration_ms: u64,
    pub windows: Vec<OpenLoopObservation>,
    pub recovered_at_window: Option<u32>,
    pub consecutive_passing_windows: u32,
    pub observed_recovery_ms: u64,
    pub time_to_baseline_ms: Option<u64>,
    pub final_state_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverloadRepeatEvidence {
    pub reset_state_digest: String,
    pub preloaded_state_digest: String,
    pub steady_state_digest: String,
    pub preload_operations: u64,
    pub warmup_operations: u64,
    pub admission_control: AdmissionControlEvidence,
    pub overload: OpenLoopObservation,
    pub metrics: OverloadMetrics,
    pub recovery: RecoveryEvidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverloadAggregate {
    pub representative_repeat_index: u32,
    pub successful_goodput_per_second: f64,
    pub goodput_min_per_second: f64,
    pub goodput_max_per_second: f64,
    pub robust_goodput_spread_ratio: f64,
    pub scheduled_p99_us: u64,
    pub rejection_ratio: f64,
    pub error_timeout_ratio: f64,
    pub backlog_high_water: u64,
    pub backlog_drained: bool,
    pub recovered_at_window: Option<u32>,
    pub time_to_baseline_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverloadPointEvidence {
    pub factor_millionths: u32,
    pub offered_rate_per_second: u64,
    pub repeats: Vec<OverloadRepeatEvidence>,
    pub aggregate: OverloadAggregate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverloadReport {
    pub schema_version: u32,
    pub release: String,
    pub report_id: String,
    pub scenario_id: String,
    pub scenario_digest_sha256: String,
    pub evidence_class: String,
    pub claim_scope: String,
    pub run_mode: OverloadRunMode,
    pub surface: EligibleOverloadSurface,
    pub surface_identity: SurfaceIdentity,
    pub predecessor: CapacityPredecessor,
    pub target_binding: Option<ReferenceTargetBinding>,
    /// Present only for the fresh W3-derived direct daemon. The report is
    /// published only after this child was killed and waited/reaped.
    pub daemon_lifecycle: Option<RespDaemonEvidence>,
    pub admission_control_mode: AdmissionControlMode,
    pub baseline_goodput_per_second: f64,
    pub baseline_scheduled_p99_us: u64,
    pub points: Vec<OverloadPointEvidence>,
    pub generic_cluster_capacity_claim: bool,
    pub node_native_wire_claim: bool,
    pub library_model_capacity_claim: bool,
    pub deferred_claims: Vec<String>,
}

impl OverloadReport {
    pub fn validate(&self, scenario: &OverloadScenario) -> Result<(), OverloadError> {
        scenario.validate()?;
        let selected = self.predecessor.validate(scenario, self.run_mode)?;
        let (expected_goodput, expected_p99) = predecessor_baseline(selected)?;
        if self.schema_version != OVERLOAD_REPORT_VERSION
            || self.release != PERF_RELEASE
            || !portable_identifier(&self.report_id)
            || self.scenario_id != scenario.scenario_id
            || self.scenario_digest_sha256 != scenario.contract_digest()?
            || self.evidence_class != W6_EVIDENCE_CLASS
            || self.claim_scope != W6_CLAIM_SCOPE
            || self.surface != self.predecessor.surface
            || self.surface_identity != self.surface.overload_surface()
            || !nearly_equal(self.baseline_goodput_per_second, expected_goodput)
            || self.baseline_scheduled_p99_us != expected_p99
            || self.generic_cluster_capacity_claim
            || self.node_native_wire_claim
            || self.library_model_capacity_claim
            || !exact_deferred_claims(&self.deferred_claims)
        {
            return Err(OverloadError::Boundary(
                "W6 report changed its scenario/predecessor/surface identity or crossed the node-native, generic-cluster, or library/model boundary"
                    .to_owned(),
            ));
        }
        reject_forbidden_identity(&self.surface_identity)?;
        match self.run_mode {
            OverloadRunMode::DeterministicSmoke => {
                if self.target_binding.is_some() || self.daemon_lifecycle.is_some() {
                    return Err(OverloadError::Boundary(
                        "generic smoke targets cannot carry reference execution evidence"
                            .to_owned(),
                    ));
                }
            }
            OverloadRunMode::Reference => {
                let binding = self.target_binding.as_ref().ok_or_else(|| {
                    OverloadError::Boundary(
                        "reference overload report has no producer target binding".to_owned(),
                    )
                })?;
                binding.validate(&self.predecessor)?;
                if self.admission_control_mode != AdmissionControlMode::Enabled {
                    return Err(OverloadError::Boundary(
                        "reference target identity/workload does not match its predecessor, or admission is disabled"
                            .to_owned(),
                    ));
                }
                match self.surface {
                    EligibleOverloadSurface::Local | EligibleOverloadSurface::ClientSurface => {
                        if self.daemon_lifecycle.is_some() {
                            return Err(OverloadError::Boundary(
                                "in-process W6 evidence cannot carry a daemon lifecycle".to_owned(),
                            ));
                        }
                    }
                    EligibleOverloadSurface::NodeResp => {
                        let lifecycle = self.daemon_lifecycle.as_ref().ok_or_else(|| {
                            OverloadError::Boundary(
                                "node-resp W6 evidence is missing fresh kill+wait lifecycle"
                                    .to_owned(),
                            )
                        })?;
                        validate_fresh_resp_lifecycle(binding, lifecycle)?;
                    }
                }
            }
        }
        if self.points.len() != OVERLOAD_FACTORS_MILLIONTHS.len() {
            return Err(OverloadError::Evidence(
                "W6 report must contain exactly the 1.2x, 1.5x, and 2x points".to_owned(),
            ));
        }
        let knee_rate = self.predecessor.knee_rate_per_second()?;
        let expected_preload_operations =
            scenario.expected_preload_operations(self.surface, self.run_mode);
        let (overload_operations, recovery_operations) = scenario.window_operations(self.run_mode);
        for (point, factor) in self.points.iter().zip(OVERLOAD_FACTORS_MILLIONTHS) {
            validate_point(
                point,
                factor,
                knee_rate,
                expected_goodput,
                expected_p99,
                expected_preload_operations,
                overload_operations,
                recovery_operations,
                scenario,
            )?;
            if point
                .repeats
                .iter()
                .any(|repeat| repeat.admission_control.mode != self.admission_control_mode)
            {
                return Err(OverloadError::Evidence(
                    "report admission mode differs from raw window-control evidence".to_owned(),
                ));
            }
        }
        validate_reset_preload_equivalence(&self.points, expected_preload_operations)?;
        Ok(())
    }

    pub fn to_pretty_json(&self, scenario: &OverloadScenario) -> Result<Vec<u8>, OverloadError> {
        self.validate(scenario)?;
        serde_json::to_vec_pretty(self).map_err(|error| OverloadError::Evidence(error.to_string()))
    }

    pub fn all_points_recovered(&self) -> bool {
        self.points
            .iter()
            .all(|point| point.aggregate.recovered_at_window.is_some())
    }
}

pub async fn run_overload_curve<T, C>(
    target: Arc<T>,
    control: &C,
    scenario: &OverloadScenario,
    predecessor: CapacityPredecessor,
) -> Result<OverloadReport, OverloadError>
where
    T: Target,
    C: OverloadWindowControl,
{
    let report = run_overload_curve_inner(
        target,
        control,
        scenario,
        predecessor,
        OverloadRunMode::DeterministicSmoke,
        None,
    )
    .await?;
    report.validate(scenario)?;
    Ok(report)
}

#[allow(dead_code)] // Producer API intentionally lands before its shared CLI integration.
pub async fn run_reference_overload_curve<A>(
    adapter: Arc<A>,
    scenario: &OverloadScenario,
    predecessor: CapacityPredecessor,
) -> Result<OverloadReport, OverloadError>
where
    A: ReferenceOverloadAdapter,
{
    scenario.validate()?;
    predecessor.validate(scenario, OverloadRunMode::Reference)?;
    let expected = predecessor.reference_target_contract()?;
    let observed = adapter.reference_target_binding()?;
    observed.validate(&predecessor)?;
    if observed.contract != expected {
        return Err(OverloadError::Boundary(
            "reference adapter identity/capability/workload differs from the disk-verified predecessor"
                .to_owned(),
        ));
    }
    let report = run_overload_curve_inner(
        Arc::clone(&adapter),
        adapter.as_ref(),
        scenario,
        predecessor,
        OverloadRunMode::Reference,
        Some(observed),
    )
    .await?;
    report.validate(scenario)?;
    Ok(report)
}

async fn run_overload_curve_inner<T, C>(
    target: Arc<T>,
    control: &C,
    scenario: &OverloadScenario,
    predecessor: CapacityPredecessor,
    run_mode: OverloadRunMode,
    target_binding: Option<ReferenceTargetBinding>,
) -> Result<OverloadReport, OverloadError>
where
    T: Target,
    C: OverloadWindowControl,
{
    scenario.validate()?;
    let selected = predecessor.validate(scenario, run_mode)?;
    let (baseline_goodput_per_second, baseline_scheduled_p99_us) = predecessor_baseline(selected)?;
    let knee_rate = predecessor.knee_rate_per_second()?;
    let expected_preload_operations =
        scenario.expected_preload_operations(predecessor.surface, run_mode);
    let (overload_operations, recovery_operations) = scenario.window_operations(run_mode);
    let mut points = Vec::with_capacity(OVERLOAD_FACTORS_MILLIONTHS.len());
    for factor_millionths in OVERLOAD_FACTORS_MILLIONTHS {
        let offered_rate_per_second = factored_rate(knee_rate, factor_millionths)?;
        let mut repeats = Vec::with_capacity(scenario.work.repeats as usize);
        for _ in 0..scenario.work.repeats {
            repeats.push(
                run_overload_repeat(
                    Arc::clone(&target),
                    control,
                    scenario,
                    factor_millionths,
                    offered_rate_per_second,
                    knee_rate,
                    baseline_goodput_per_second,
                    baseline_scheduled_p99_us,
                    expected_preload_operations,
                    overload_operations,
                    recovery_operations,
                )
                .await?,
            );
        }
        let aggregate = aggregate_repeats(&repeats)?;
        if aggregate.robust_goodput_spread_ratio > scenario.work.maximum_goodput_spread_ratio {
            return Err(OverloadError::Evidence(format!(
                "overload repeat spread {} exceeds committed tolerance {}",
                aggregate.robust_goodput_spread_ratio, scenario.work.maximum_goodput_spread_ratio
            )));
        }
        points.push(OverloadPointEvidence {
            factor_millionths,
            offered_rate_per_second,
            repeats,
            aggregate,
        });
    }
    validate_reset_preload_equivalence(&points, expected_preload_operations)?;
    let admission_control_mode = points
        .first()
        .and_then(|point| point.repeats.first())
        .map(|repeat| repeat.admission_control.mode)
        .ok_or_else(|| {
            OverloadError::Evidence("overload curve produced no raw repeats".to_owned())
        })?;
    let report = OverloadReport {
        schema_version: OVERLOAD_REPORT_VERSION,
        release: PERF_RELEASE.to_owned(),
        report_id: format!("overload-{}-v1", surface_cli_name(predecessor.surface)),
        scenario_id: scenario.scenario_id.clone(),
        scenario_digest_sha256: scenario.contract_digest()?,
        evidence_class: W6_EVIDENCE_CLASS.to_owned(),
        claim_scope: W6_CLAIM_SCOPE.to_owned(),
        run_mode,
        surface: predecessor.surface,
        surface_identity: predecessor.surface.overload_surface(),
        predecessor,
        target_binding,
        daemon_lifecycle: None,
        admission_control_mode,
        baseline_goodput_per_second,
        baseline_scheduled_p99_us,
        points,
        generic_cluster_capacity_claim: false,
        node_native_wire_claim: false,
        library_model_capacity_claim: false,
        deferred_claims: vec![
            "generic-cluster-capacity".to_owned(),
            "library-model-overload".to_owned(),
            "node-native-wire".to_owned(),
        ],
    };
    report.validate(scenario)?;
    Ok(report)
}

#[allow(clippy::too_many_arguments)]
async fn run_overload_repeat<T, C>(
    target: Arc<T>,
    control: &C,
    scenario: &OverloadScenario,
    factor_millionths: u32,
    offered_rate_per_second: u64,
    knee_rate_per_second: u64,
    baseline_goodput_per_second: f64,
    baseline_scheduled_p99_us: u64,
    expected_preload_operations: u64,
    overload_operations: u64,
    recovery_operations: u64,
) -> Result<OverloadRepeatEvidence, OverloadError>
where
    T: Target,
    C: OverloadWindowControl,
{
    let reset_state_digest = target.reset().await?;
    let observed_reset_state_digest = target.state_digest().await?;
    let preload = target.preload().await?;
    let observed_preloaded_state_digest = target.state_digest().await?;
    if reset_state_digest.is_empty()
        || observed_reset_state_digest != reset_state_digest
        || preload.state_digest.is_empty()
        || observed_preloaded_state_digest != preload.state_digest
        || preload.operations != expected_preload_operations
        || expected_preload_operations == 0 && reset_state_digest != preload.state_digest
    {
        return Err(OverloadError::Measurement(
            "reset/preload did not establish the committed W6 state".to_owned(),
        ));
    }
    control.prepare_recovery_window().await?;
    for sequence in 0..scenario.work.warmup_operations {
        if target.execute(TargetRequest { sequence }).await != TargetOutcome::Success {
            return Err(OverloadError::Measurement(
                "W6 warm-up must complete successfully outside the overload histogram".to_owned(),
            ));
        }
    }
    let steady_state_digest = target.state_digest().await?;
    if steady_state_digest.is_empty() {
        return Err(OverloadError::Measurement(
            "W6 post-warm-up state digest is empty".to_owned(),
        ));
    }
    let admission_control = control.prepare_overload_window(factor_millionths).await?;
    admission_control.validate(factor_millionths)?;
    let overload = run_open_loop(
        Arc::clone(&target),
        &open_loop_config(offered_rate_per_second, overload_operations, scenario),
    )
    .await
    .map_err(OverloadError::Measurement)?;
    let metrics = metrics_from_observation(&overload)?;
    let recovery_started = tokio::time::Instant::now();
    control.prepare_recovery_window().await?;
    let transition_duration_ms = duration_millis(recovery_started.elapsed());
    let mut windows = Vec::new();
    let mut recovered_at_window = None;
    let mut consecutive_passing_windows = 0_u32;
    for index in 0..scenario.work.max_recovery_windows {
        let window = run_open_loop(
            Arc::clone(&target),
            &open_loop_config(knee_rate_per_second, recovery_operations, scenario),
        )
        .await
        .map_err(OverloadError::Measurement)?;
        let recovered = recovery_window_passes(
            &window,
            baseline_goodput_per_second,
            baseline_scheduled_p99_us,
            scenario,
        )?;
        windows.push(window);
        if recovered {
            consecutive_passing_windows = consecutive_passing_windows.saturating_add(1);
            if consecutive_passing_windows == scenario.work.required_consecutive_recovery_windows {
                recovered_at_window = Some(index + 1);
                break;
            }
        } else {
            consecutive_passing_windows = 0;
        }
    }
    let observed_recovery_ms = windows
        .iter()
        .try_fold(transition_duration_ms, |total, window| {
            total.checked_add(window.elapsed_ms)
        })
        .ok_or_else(|| OverloadError::Measurement("recovery duration overflow".to_owned()))?;
    let time_to_baseline_ms = recovered_at_window.map(|_| observed_recovery_ms);
    let final_state_digest = target.state_digest().await?;
    if final_state_digest.is_empty() {
        return Err(OverloadError::Measurement(
            "W6 recovery state digest is empty".to_owned(),
        ));
    }
    Ok(OverloadRepeatEvidence {
        reset_state_digest,
        preloaded_state_digest: preload.state_digest,
        steady_state_digest,
        preload_operations: preload.operations,
        warmup_operations: scenario.work.warmup_operations,
        admission_control,
        overload,
        metrics,
        recovery: RecoveryEvidence {
            transition_duration_ms,
            windows,
            recovered_at_window,
            consecutive_passing_windows,
            observed_recovery_ms,
            time_to_baseline_ms,
            final_state_digest,
        },
    })
}

fn open_loop_config(
    offered_rate_per_second: u64,
    operations: u64,
    scenario: &OverloadScenario,
) -> OpenLoopConfig {
    OpenLoopConfig {
        offered_rate_per_second,
        operations,
        highest_trackable_latency: Duration::from_micros(
            scenario.work.highest_trackable_latency_us,
        ),
        significant_figures: scenario.work.histogram_significant_figures,
        p999_min_samples: scenario.work.p999_min_samples,
        drain_timeout: Duration::from_millis(scenario.work.drain_timeout_ms),
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_point(
    point: &OverloadPointEvidence,
    expected_factor: u32,
    knee_rate: u64,
    baseline_goodput: f64,
    baseline_p99: u64,
    expected_preload_operations: u64,
    overload_operations: u64,
    recovery_operations: u64,
    scenario: &OverloadScenario,
) -> Result<(), OverloadError> {
    if point.factor_millionths != expected_factor
        || point.offered_rate_per_second != factored_rate(knee_rate, expected_factor)?
        || point.repeats.len() != scenario.work.repeats as usize
    {
        return Err(OverloadError::Evidence(
            "overload point factor/rate/repeat count does not match the committed contract"
                .to_owned(),
        ));
    }
    for repeat in &point.repeats {
        repeat.admission_control.validate(expected_factor)?;
        validate_repeat(
            repeat,
            point.offered_rate_per_second,
            knee_rate,
            baseline_goodput,
            baseline_p99,
            expected_preload_operations,
            overload_operations,
            recovery_operations,
            scenario,
        )?;
    }
    let expected = aggregate_repeats(&point.repeats)?;
    if expected != point.aggregate {
        return Err(OverloadError::Evidence(
            "overload aggregate does not match its raw repeat evidence".to_owned(),
        ));
    }
    if point.aggregate.robust_goodput_spread_ratio > scenario.work.maximum_goodput_spread_ratio {
        return Err(OverloadError::Evidence(
            "overload goodput spread exceeds the committed scenario tolerance".to_owned(),
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_repeat(
    repeat: &OverloadRepeatEvidence,
    offered_rate: u64,
    knee_rate: u64,
    baseline_goodput: f64,
    baseline_p99: u64,
    expected_preload_operations: u64,
    overload_operations: u64,
    recovery_operations: u64,
    scenario: &OverloadScenario,
) -> Result<(), OverloadError> {
    if repeat.reset_state_digest.is_empty()
        || repeat.preloaded_state_digest.is_empty()
        || repeat.steady_state_digest.is_empty()
        || repeat.recovery.final_state_digest.is_empty()
        || repeat.preload_operations != expected_preload_operations
        || repeat.warmup_operations != scenario.work.warmup_operations
        || expected_preload_operations == 0
            && repeat.reset_state_digest != repeat.preloaded_state_digest
    {
        return Err(OverloadError::Evidence(
            "overload repeat has incomplete lifecycle/state evidence".to_owned(),
        ));
    }
    validate_observation(&repeat.overload, offered_rate, overload_operations)?;
    let expected_metrics = metrics_from_observation(&repeat.overload)?;
    if expected_metrics != repeat.metrics {
        return Err(OverloadError::Evidence(
            "goodput/p99/rejection/error/backlog metrics do not match the raw overload window"
                .to_owned(),
        ));
    }
    if repeat.recovery.windows.is_empty()
        || repeat.recovery.windows.len() > scenario.work.max_recovery_windows as usize
    {
        return Err(OverloadError::Evidence(
            "recovery evidence has no window or exceeded the committed search bound".to_owned(),
        ));
    }
    let mut expected_recovered = None;
    let mut expected_consecutive = 0_u32;
    let mut expected_elapsed = repeat.recovery.transition_duration_ms;
    for (index, window) in repeat.recovery.windows.iter().enumerate() {
        validate_observation(window, knee_rate, recovery_operations)?;
        expected_elapsed = expected_elapsed
            .checked_add(window.elapsed_ms)
            .ok_or_else(|| OverloadError::Evidence("recovery duration overflow".to_owned()))?;
        if recovery_window_passes(window, baseline_goodput, baseline_p99, scenario)? {
            expected_consecutive = expected_consecutive.saturating_add(1);
            if expected_consecutive == scenario.work.required_consecutive_recovery_windows {
                expected_recovered = Some(u32::try_from(index + 1).map_err(|_| {
                    OverloadError::Evidence("recovery window index overflow".to_owned())
                })?);
                if index + 1 != repeat.recovery.windows.len() {
                    return Err(OverloadError::Evidence(
                        "recovery search retained trailing windows after consecutive confirmation"
                            .to_owned(),
                    ));
                }
                break;
            }
        } else {
            expected_consecutive = 0;
        }
    }
    let expected_time = expected_recovered.map(|_| expected_elapsed);
    if repeat.recovery.recovered_at_window != expected_recovered
        || repeat.recovery.consecutive_passing_windows != expected_consecutive
        || repeat.recovery.observed_recovery_ms != expected_elapsed
        || repeat.recovery.time_to_baseline_ms != expected_time
        || expected_recovered.is_none()
            && repeat.recovery.windows.len() != scenario.work.max_recovery_windows as usize
    {
        return Err(OverloadError::Evidence(
            "transition-inclusive time-to-baseline does not identify the committed consecutive recovery confirmation"
                .to_owned(),
        ));
    }
    Ok(())
}

fn validate_observation(
    observation: &OpenLoopObservation,
    expected_rate: u64,
    expected_operations: u64,
) -> Result<(), OverloadError> {
    let classified = observation
        .successes
        .checked_add(observation.errors)
        .and_then(|value| value.checked_add(observation.timeouts))
        .and_then(|value| value.checked_add(observation.rejections));
    if observation.offered != expected_operations
        || observation.started != observation.offered
        || observation.completed > observation.started
        || classified != Some(observation.completed)
        || observation.latency.samples != observation.completed
        || !nearly_equal(observation.offered_rate_per_second, expected_rate as f64)
        || !observation.achieved_rate_per_second.is_finite()
        || observation.achieved_rate_per_second < 0.0
        || observation.latency.p99_us.is_none()
        || observation.latency.overflow_count > 0
        || observation.backlog_high_water > observation.started
    {
        return Err(OverloadError::Evidence(
            "raw open-loop observation is empty, unbalanced, overflowed, or bound to the wrong scheduled rate"
                .to_owned(),
        ));
    }
    Ok(())
}

fn metrics_from_observation(
    observation: &OpenLoopObservation,
) -> Result<OverloadMetrics, OverloadError> {
    let denominator = observation.started.max(1) as f64;
    let successful_goodput_per_second = observation.achieved_rate_per_second
        * observation.successes as f64
        / observation.completed.max(1) as f64;
    let error_timeout = observation
        .errors
        .checked_add(observation.timeouts)
        .ok_or_else(|| OverloadError::Evidence("error/timeout accounting overflow".to_owned()))?;
    let metrics = OverloadMetrics {
        successful_goodput_per_second,
        scheduled_p99_us: observation
            .latency
            .p99_us
            .ok_or_else(|| OverloadError::Evidence("scheduled p99 is unavailable".to_owned()))?,
        rejection_ratio: observation.rejections as f64 / denominator,
        error_timeout_ratio: error_timeout as f64 / denominator,
        backlog_high_water: observation.backlog_high_water,
        backlog_drained: observation.backlog_drained,
    };
    if !metrics.successful_goodput_per_second.is_finite()
        || !valid_ratio(metrics.rejection_ratio)
        || !valid_ratio(metrics.error_timeout_ratio)
    {
        return Err(OverloadError::Evidence(
            "derived overload metric is non-finite or outside 0..=1".to_owned(),
        ));
    }
    Ok(metrics)
}

fn recovery_window_passes(
    observation: &OpenLoopObservation,
    baseline_goodput: f64,
    baseline_p99: u64,
    scenario: &OverloadScenario,
) -> Result<bool, OverloadError> {
    let metrics = metrics_from_observation(observation)?;
    Ok(metrics.successful_goodput_per_second
        >= baseline_goodput * scenario.work.recovery_goodput_floor_ratio
        && metrics.scheduled_p99_us as f64
            <= baseline_p99 as f64 * scenario.work.recovery_p99_ceiling_ratio
        && metrics.error_timeout_ratio == 0.0
        && metrics.rejection_ratio == 0.0
        && metrics.backlog_drained)
}

fn aggregate_repeats(
    repeats: &[OverloadRepeatEvidence],
) -> Result<OverloadAggregate, OverloadError> {
    if repeats.len() < 3 {
        return Err(OverloadError::Evidence(
            "overload aggregation requires at least three raw repeats".to_owned(),
        ));
    }
    let mut order = (0..repeats.len()).collect::<Vec<_>>();
    order.sort_by(|left, right| {
        repeats[*left]
            .metrics
            .successful_goodput_per_second
            .total_cmp(&repeats[*right].metrics.successful_goodput_per_second)
            .then_with(|| left.cmp(right))
    });
    let representative_repeat_index = order[order.len() / 2];
    let representative = &repeats[representative_repeat_index];
    let minimum = repeats
        .iter()
        .map(|repeat| repeat.metrics.successful_goodput_per_second)
        .min_by(f64::total_cmp)
        .ok_or_else(|| OverloadError::Evidence("no overload goodput samples".to_owned()))?;
    let maximum = repeats
        .iter()
        .map(|repeat| repeat.metrics.successful_goodput_per_second)
        .max_by(f64::total_cmp)
        .ok_or_else(|| OverloadError::Evidence("no overload goodput samples".to_owned()))?;
    let median = representative.metrics.successful_goodput_per_second;
    let spread = if median > 0.0 {
        (maximum - minimum) / median
    } else if maximum == minimum {
        0.0
    } else {
        f64::INFINITY
    };
    if !minimum.is_finite() || !maximum.is_finite() || !spread.is_finite() {
        return Err(OverloadError::Evidence(
            "overload goodput aggregation is non-finite".to_owned(),
        ));
    }
    Ok(OverloadAggregate {
        representative_repeat_index: u32::try_from(representative_repeat_index).map_err(|_| {
            OverloadError::Evidence("representative repeat index overflow".to_owned())
        })?,
        successful_goodput_per_second: median,
        goodput_min_per_second: minimum,
        goodput_max_per_second: maximum,
        robust_goodput_spread_ratio: spread,
        scheduled_p99_us: representative.metrics.scheduled_p99_us,
        rejection_ratio: representative.metrics.rejection_ratio,
        error_timeout_ratio: representative.metrics.error_timeout_ratio,
        backlog_high_water: representative.metrics.backlog_high_water,
        backlog_drained: representative.metrics.backlog_drained,
        recovered_at_window: representative.recovery.recovered_at_window,
        time_to_baseline_ms: representative.recovery.time_to_baseline_ms,
    })
}

fn validate_reset_preload_equivalence(
    points: &[OverloadPointEvidence],
    expected_preload_operations: u64,
) -> Result<(), OverloadError> {
    let mut repeats = points.iter().flat_map(|point| point.repeats.iter());
    let first = repeats.next().ok_or_else(|| {
        OverloadError::Evidence("W6 has no reset/preload lifecycle evidence".to_owned())
    })?;
    if first.reset_state_digest.is_empty()
        || first.preloaded_state_digest.is_empty()
        || first.preload_operations != expected_preload_operations
        || expected_preload_operations == 0
            && first.reset_state_digest != first.preloaded_state_digest
        || repeats.any(|repeat| {
            repeat.reset_state_digest != first.reset_state_digest
                || repeat.preloaded_state_digest != first.preloaded_state_digest
                || repeat.preload_operations != first.preload_operations
        })
    {
        return Err(OverloadError::Evidence(
            "reset/preload state digest and operation count must be identical across every factor and repeat"
                .to_owned(),
        ));
    }
    Ok(())
}

fn predecessor_baseline(point: &RatePointEvidence) -> Result<(f64, u64), OverloadError> {
    if point.sample.completed == 0 || point.sample.successes == 0 {
        return Err(OverloadError::Predecessor(
            "selected knee cannot derive successful goodput".to_owned(),
        ));
    }
    let goodput = point.sample.achieved_rate_per_second * point.sample.successes as f64
        / point.sample.completed as f64;
    let p99 = point.sample.latency.p99_us.ok_or_else(|| {
        OverloadError::Predecessor("selected knee scheduled p99 is unavailable".to_owned())
    })?;
    if !goodput.is_finite() || goodput <= 0.0 || p99 == 0 {
        return Err(OverloadError::Predecessor(
            "selected knee baseline goodput/p99 is invalid".to_owned(),
        ));
    }
    Ok((goodput, p99))
}

fn factored_rate(knee_rate: u64, factor_millionths: u32) -> Result<u64, OverloadError> {
    if !OVERLOAD_FACTORS_MILLIONTHS.contains(&factor_millionths) {
        return Err(OverloadError::Contract(
            "overload factor is not exactly 1.2x, 1.5x, or 2x".to_owned(),
        ));
    }
    let scaled = knee_rate
        .checked_mul(u64::from(factor_millionths))
        .ok_or_else(|| OverloadError::Predecessor("factored knee rate overflow".to_owned()))?;
    if scaled % 1_000_000 != 0 {
        return Err(OverloadError::Predecessor(
            "capacity knee cannot express every exact W6 factor as an integral fixed rate"
                .to_owned(),
        ));
    }
    let rate = scaled / 1_000_000;
    if rate == 0 {
        return Err(OverloadError::Predecessor(
            "factored capacity knee produced a zero rate".to_owned(),
        ));
    }
    Ok(rate)
}

fn reject_forbidden_identity(identity: &SurfaceIdentity) -> Result<(), OverloadError> {
    let joined = [
        identity.surface_kind.as_str(),
        identity.execution_mode.as_str(),
        identity.state_scope.as_str(),
        identity.network_boundary.as_str(),
        identity.claim_scope.as_str(),
    ]
    .join(" ")
    .to_ascii_lowercase();
    if joined.contains("node-native")
        || joined.contains("generic-cluster")
        || joined.contains("cluster-capacity")
        || joined.contains("distributed-value")
        || joined.contains("library-model")
    {
        return Err(OverloadError::Boundary(
            "node-native, generic/distributed cluster, and library/model identities cannot satisfy W6 capacity overload"
                .to_owned(),
        ));
    }
    Ok(())
}

fn exact_deferred_claims(claims: &[String]) -> bool {
    let expected = [
        "generic-cluster-capacity",
        "library-model-overload",
        "node-native-wire",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    claims.len() == expected.len()
        && claims.iter().map(String::as_str).collect::<BTreeSet<_>>() == expected
}

fn surface_cli_name(surface: EligibleOverloadSurface) -> &'static str {
    match surface {
        EligibleOverloadSurface::Local => "local",
        EligibleOverloadSurface::ClientSurface => "client-surface",
        EligibleOverloadSurface::NodeResp => "node-resp",
    }
}

fn canonical_digest<T: Serialize>(value: &T) -> Result<String, OverloadError> {
    let bytes =
        serde_json::to_vec(value).map_err(|error| OverloadError::Contract(error.to_string()))?;
    Ok(sha256(&bytes))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_git_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn portable_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
}

fn valid_ratio(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn nearly_equal(left: f64, right: f64) -> bool {
    left.is_finite()
        && right.is_finite()
        && (left - right).abs() <= f64::EPSILON * 32.0 * left.abs().max(right.abs()).max(1.0)
}

fn duration_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// Deterministic fast target used only to prove the W6 measurement plumbing.
/// Admission-enabled overload rejects excess work and plateaus successful
/// goodput.  The test-only bypass accepts excess work into a deliberately
/// collapsing path, making the W6 canary observably red.
#[derive(Debug)]
pub struct DeterministicAdmissionFixture {
    factor_millionths: AtomicU32,
    admission_gate_active: AtomicBool,
    mode: AdmissionControlMode,
    base_delay: Duration,
}

impl DeterministicAdmissionFixture {
    pub fn new(base_delay: Duration, mode: AdmissionControlMode) -> Self {
        Self {
            factor_millionths: AtomicU32::new(1_000_000),
            admission_gate_active: AtomicBool::new(true),
            mode,
            base_delay,
        }
    }

    fn accepted(&self, sequence: u64, factor: u32, admission_enabled: bool) -> bool {
        if factor <= 1_000_000 {
            return true;
        }
        let bucket = sequence % 120;
        if admission_enabled {
            match factor {
                1_200_000 => bucket % 6 != 5,
                1_500_000 => bucket % 3 != 2,
                2_000_000 => bucket.is_multiple_of(2),
                _ => false,
            }
        } else {
            match factor {
                1_200_000 => bucket % 3 != 2,
                1_500_000 => bucket.is_multiple_of(3),
                2_000_000 => bucket.is_multiple_of(8),
                _ => false,
            }
        }
    }
}

#[async_trait]
impl Target for DeterministicAdmissionFixture {
    async fn reset(&self) -> Result<String, TargetError> {
        self.factor_millionths.store(1_000_000, Ordering::SeqCst);
        self.admission_gate_active.store(true, Ordering::SeqCst);
        Ok("w6-fixture-logical-state-v1".to_owned())
    }

    async fn preload(&self) -> Result<PreloadOutcome, TargetError> {
        Ok(PreloadOutcome {
            operations: 0,
            state_digest: "w6-fixture-logical-state-v1".to_owned(),
        })
    }

    async fn state_digest(&self) -> Result<String, TargetError> {
        Ok("w6-fixture-logical-state-v1".to_owned())
    }

    async fn execute(&self, request: TargetRequest) -> TargetOutcome {
        let factor = self.factor_millionths.load(Ordering::SeqCst);
        let admission_enabled = self.admission_gate_active.load(Ordering::SeqCst);
        let accepted = self.accepted(request.sequence, factor, admission_enabled);
        if admission_enabled && !accepted {
            return TargetOutcome::Rejected;
        }
        let collapse_multiplier = if !admission_enabled && factor == 2_000_000 {
            12
        } else if !admission_enabled && factor > 1_000_000 {
            4
        } else {
            1
        };
        tokio::time::sleep(self.base_delay.saturating_mul(collapse_multiplier)).await;
        if accepted {
            TargetOutcome::Success
        } else {
            TargetOutcome::Error
        }
    }
}

#[async_trait]
impl OverloadWindowControl for DeterministicAdmissionFixture {
    async fn prepare_overload_window(
        &self,
        factor_millionths: u32,
    ) -> Result<AdmissionControlEvidence, OverloadError> {
        if !OVERLOAD_FACTORS_MILLIONTHS.contains(&factor_millionths) {
            return Err(OverloadError::Contract(
                "fixture received an unsupported overload factor".to_owned(),
            ));
        }
        self.factor_millionths
            .store(factor_millionths, Ordering::SeqCst);
        let gate_active = self.mode == AdmissionControlMode::Enabled;
        self.admission_gate_active
            .store(gate_active, Ordering::SeqCst);
        AdmissionControlEvidence::sealed(
            self.mode,
            factor_millionths,
            "deterministic-admission-fixture",
            canonical_digest(&(
                "deterministic-admission-fixture-v1",
                self.base_delay.as_nanos(),
                self.mode,
            ))?,
        )
    }

    async fn prepare_recovery_window(&self) -> Result<(), OverloadError> {
        self.factor_millionths.store(1_000_000, Ordering::SeqCst);
        self.admission_gate_active.store(true, Ordering::SeqCst);
        Ok(())
    }
}

/// Build a real common-runner capacity knee for the deterministic smoke
/// fixture.  This helper cannot create reference evidence or a receipt.
pub async fn deterministic_fixture_predecessor(
    target: Arc<DeterministicAdmissionFixture>,
    surface: EligibleOverloadSurface,
    knee_rate_per_second: u64,
) -> Result<CapacityPredecessor, OverloadError> {
    if knee_rate_per_second == 0 || !knee_rate_per_second.is_multiple_of(10) {
        return Err(OverloadError::Predecessor(
            "fixture knee must be positive and divisible by ten for exact W6 factors".to_owned(),
        ));
    }
    target.prepare_recovery_window().await?;
    let capacity_scenario = Scenario {
        schema_version: 1,
        id: format!("w6-fixture-{}-capacity", surface_cli_name(surface)),
        seed: 67_006,
        offered_rates_per_second: vec![knee_rate_per_second],
        preload_operations: 0,
        warmup_operations: 4,
        steady_operations: 48,
        repeats: 3,
        p99_slo_us: 100_000,
        p999_slo_us: None,
        p999_min_samples: 1,
        highest_trackable_latency_us: 1_000_000,
        histogram_significant_figures: 3,
        min_achieved_ratio: 0.40,
        error_budgets: ErrorBudgets {
            max_error_ratio: 0.0,
            max_timeout_ratio: 0.0,
            max_rejection_ratio: 0.0,
        },
        backlog_drain_ms: 1_000,
        robust_spread_tolerance: 1.0,
    };
    let knee = run_scenario(target, &capacity_scenario).await?;
    if knee.sustainable_rate_per_second != Some(knee_rate_per_second as f64) {
        return Err(OverloadError::Measurement(format!(
            "fixture failed to establish the requested capacity knee: {knee:?}"
        )));
    }
    Ok(CapacityPredecessor {
        evidence_class: surface.predecessor_class().to_owned(),
        surface,
        surface_identity: surface.capacity_surface(),
        claim: LoadClaim::CapacityKnee,
        profile: "smoke-v1".to_owned(),
        stable_capacity_evidence: false,
        stable_surface_capability_sha256: sha256(
            format!("w6-deterministic-fixture:{}", surface_cli_name(surface)).as_bytes(),
        ),
        workload_identity_sha256: sha256(
            format!("w6-deterministic-workload:{}", surface_cli_name(surface)).as_bytes(),
        ),
        criteria: capacity_scenario.sustainability_criteria(),
        knee,
        reference_receipt: None,
    })
}

/// Canary predicate: the bypassed 2x point must deliver materially less
/// successful goodput than the admission-enabled plateau while surfacing
/// errors rather than rejections.
pub fn admission_disabled_collapse_detected(
    admission_enabled: &OverloadReport,
    admission_disabled: &OverloadReport,
) -> Result<bool, OverloadError> {
    let enabled = admission_enabled
        .points
        .iter()
        .find(|point| point.factor_millionths == 2_000_000)
        .ok_or_else(|| OverloadError::Evidence("enabled report has no 2x point".to_owned()))?;
    let disabled = admission_disabled
        .points
        .iter()
        .find(|point| point.factor_millionths == 2_000_000)
        .ok_or_else(|| OverloadError::Evidence("disabled report has no 2x point".to_owned()))?;
    if admission_enabled.admission_control_mode != AdmissionControlMode::Enabled
        || admission_disabled.admission_control_mode != AdmissionControlMode::DisabledCanary
        || admission_enabled.surface != admission_disabled.surface
        || admission_enabled
            .predecessor
            .stable_surface_capability_sha256
            != admission_disabled
                .predecessor
                .stable_surface_capability_sha256
    {
        return Err(OverloadError::Evidence(
            "canary reports do not form a same-surface admission A/B pair".to_owned(),
        ));
    }
    Ok(disabled.aggregate.successful_goodput_per_second
        < enabled.aggregate.successful_goodput_per_second * 0.60
        && disabled.aggregate.error_timeout_ratio > 0.0
        && disabled.aggregate.rejection_ratio == 0.0
        && enabled.aggregate.rejection_ratio > 0.0)
}

#[cfg(test)]
mod reference_identity_tests {
    use super::*;
    use crate::report::RespDaemonConfigIdentity;

    fn in_process_contract() -> ReferenceTargetContract {
        ReferenceTargetContract {
            surface: EligibleOverloadSurface::Local,
            surface_identity: EligibleOverloadSurface::Local.capacity_surface(),
            stable_surface_capability_sha256: "11".repeat(32),
            workload_identity_sha256: "22".repeat(32),
            source_commit: "33".repeat(20),
            cargo_lock_sha256: "44".repeat(32),
            prebuild_manifest_sha256: "55".repeat(32),
        }
    }

    fn short_window(elapsed_ms: u64, achieved_rate_per_second: f64) -> OpenLoopObservation {
        OpenLoopObservation {
            offered: 48,
            started: 48,
            completed: 48,
            successes: 24,
            errors: 0,
            timeouts: 0,
            rejections: 24,
            backlog_high_water: 1,
            backlog_drained: true,
            drain_ms: 0,
            elapsed_ms,
            offered_rate_per_second: 48_000.0,
            achieved_rate_per_second,
            latency: crate::histogram::LatencySummary {
                samples: 48,
                p50_us: Some(1),
                p90_us: Some(1),
                p99_us: Some(1),
                p999_us: Some(1),
                p999_min_samples: 1,
                p999_reportable: true,
                max_us: Some(1),
                overflow_count: 0,
            },
        }
    }

    #[test]
    fn w6_goodput_retains_sub_millisecond_precision_across_short_windows() {
        let before_boundary = metrics_from_observation(&short_window(1, 24_120.6)).unwrap();
        let after_boundary = metrics_from_observation(&short_window(2, 23_880.6)).unwrap();

        assert!((before_boundary.successful_goodput_per_second - 12_060.3).abs() < 0.001);
        assert!((after_boundary.successful_goodput_per_second - 11_940.3).abs() < 0.001);
        let spread = (before_boundary.successful_goodput_per_second
            - after_boundary.successful_goodput_per_second)
            / after_boundary.successful_goodput_per_second;
        assert!(
            spread < 0.02,
            "integer-millisecond quantization reappeared: {spread}"
        );
    }

    #[test]
    fn same_pid_in_process_executions_still_receive_distinct_fresh_receipts() {
        let contract = in_process_contract();
        let first = FreshExecutionReceipt::seal(
            &contract,
            ReferenceExecutionKind::InProcess,
            std::process::id(),
            None,
            None,
        )
        .unwrap();
        let second = FreshExecutionReceipt::seal(
            &contract,
            ReferenceExecutionKind::InProcess,
            std::process::id(),
            None,
            None,
        )
        .unwrap();
        assert_eq!(first.owning_pid, second.owning_pid);
        assert_ne!(first.instance_sequence, second.instance_sequence);
        assert_ne!(first.receipt_sha256, second.receipt_sha256);
    }

    #[test]
    fn node_stable_capability_excludes_pid_ports_time_and_data_directory() {
        let root = std::fs::canonicalize(std::env::current_dir().unwrap()).unwrap();
        let runtime =
            |pid, started, repeat, resp_port, admin_port, data: &str| RespEndpointCapability {
                schema_version: 1,
                pid,
                started_unix_nanos: started,
                repeat_index: repeat,
                direct_prebuilt_exec: true,
                fresh_data_dir: true,
                config: RespDaemonConfigIdentity {
                    role: "local".to_owned(),
                    listen_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
                    cluster_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
                    storage_dir: root.join(data),
                    admin_enabled: true,
                    admin_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, admin_port)),
                    redis_enabled: true,
                    redis_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, resp_port)),
                    redis_auth_required: false,
                    rediss_enabled: false,
                },
                selected_endpoint: format!("hydracache-server@127.0.0.1:{resp_port}"),
                server_binary_sha256: "66".repeat(32),
                loadgen_binary_sha256: "77".repeat(32),
                prebuild_manifest_sha256: "55".repeat(32),
                prebuild_contract_digest: "88".repeat(32),
                source_commit: "33".repeat(20),
            };
        let archived = runtime(101, 1, 1, 16_301, 16_302, "archived");
        let fresh = runtime(202, 2, 2, 26_301, 26_302, "fresh");
        let archived_stable = NodeRespStableCapability::from_runtime(&archived, "44".repeat(32))
            .digest()
            .unwrap();
        let fresh_stable = NodeRespStableCapability::from_runtime(&fresh, "44".repeat(32))
            .digest()
            .unwrap();
        assert_eq!(archived_stable, fresh_stable);
        assert_ne!(
            canonical_digest(&archived).unwrap(),
            canonical_digest(&fresh).unwrap()
        );
    }

    #[test]
    fn reference_writer_gate_rejects_missing_and_non_exact_values() {
        for surface in [
            EligibleOverloadSurface::Local,
            EligibleOverloadSurface::ClientSurface,
            EligibleOverloadSurface::NodeResp,
        ] {
            let missing = validate_reference_surface_gate_with(surface, |_| None).unwrap_err();
            assert!(missing
                .to_string()
                .contains(required_reference_gate(surface)));
            for value in ["", "true", "01", " 1", "1 "] {
                assert!(validate_reference_surface_gate_with(surface, |_| {
                    Some(value.to_owned())
                })
                .is_err());
            }
        }
    }

    #[test]
    fn reference_writer_gate_is_surface_specific_and_ignores_generic_w6_env() {
        let local_with_only_resp =
            validate_reference_surface_gate_with(EligibleOverloadSurface::Local, |variable| {
                (variable == W6_RESP_REFERENCE_ENV).then(|| "1".to_owned())
            });
        assert!(local_with_only_resp.is_err());

        let node_with_only_core =
            validate_reference_surface_gate_with(EligibleOverloadSurface::NodeResp, |variable| {
                (variable == W6_CORE_REFERENCE_ENV).then(|| "1".to_owned())
            });
        assert!(node_with_only_core.is_err());

        let legacy_generic_gate = concat!("HYDRACACHE_", "RUN_PERF_W6");
        let generic_only = validate_reference_surface_gate_with(
            EligibleOverloadSurface::ClientSurface,
            |variable| (variable == legacy_generic_gate).then(|| "1".to_owned()),
        );
        assert!(generic_only.is_err());

        for surface in [
            EligibleOverloadSurface::Local,
            EligibleOverloadSurface::ClientSurface,
            EligibleOverloadSurface::NodeResp,
        ] {
            let required = required_reference_gate(surface);
            assert!(validate_reference_surface_gate_with(surface, |variable| {
                (variable == required).then(|| "1".to_owned())
            })
            .is_ok());
        }
    }
}
