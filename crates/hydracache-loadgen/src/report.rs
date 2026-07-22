use std::collections::{BTreeMap, BTreeSet};
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::knee::{KneeResult, SustainabilityCriteria};
use crate::profile::{PerformanceProfile, ProfileValidation, RunnerFingerprint};
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

/// Typed W3 proof of the exact directly launched daemon configuration. Its
/// canonical digest is copied into every W3 measurement and cross-bound to the
/// report's source/build identities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespDaemonConfigIdentity {
    pub role: String,
    pub listen_addr: SocketAddr,
    pub cluster_addr: SocketAddr,
    pub storage_dir: PathBuf,
    pub admin_enabled: bool,
    pub admin_addr: SocketAddr,
    pub redis_enabled: bool,
    pub redis_addr: SocketAddr,
    pub redis_auth_required: bool,
    pub rediss_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespEndpointCapability {
    pub schema_version: u32,
    pub pid: u32,
    pub started_unix_nanos: u64,
    pub repeat_index: u32,
    pub direct_prebuilt_exec: bool,
    pub fresh_data_dir: bool,
    pub config: RespDaemonConfigIdentity,
    pub selected_endpoint: String,
    pub server_binary_sha256: String,
    pub loadgen_binary_sha256: String,
    pub prebuild_manifest_sha256: String,
    pub prebuild_contract_digest: String,
    pub source_commit: String,
}

/// Whether a report may be used as ship evidence or is only plumbing/noise feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRunMode {
    ReferenceEvidence,
    CiTripwire,
    Smoke,
}

/// Semantic claim made by a load curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadClaim {
    CapacityKnee,
    OverloadCurve,
    SupplementalClosedLoop,
    OperationalCost,
    ModelCost,
}

/// Typed dimension values prevent accidental string-only numeric evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DimensionValue {
    Text(String),
    U64(u64),
    I64(i64),
    F64(f64),
    Bool(bool),
}

/// Declared deterministic key distribution, when a workload has keys.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyDistributionIdentity {
    pub kind: String,
    pub theta: Option<f64>,
}

/// Weighted operation in a deterministic workload mix.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeightedOperation {
    pub operation: String,
    pub weight: f64,
}

/// Weighted payload size in a deterministic workload mix.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeightedPayload {
    pub bytes: u64,
    pub weight: f64,
}

/// Versioned and digest-bound identity for a generated or replayed workload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadIdentity {
    pub generator: String,
    pub generator_version: String,
    pub seed: Option<u64>,
    pub key_distribution: Option<KeyDistributionIdentity>,
    pub key_count: Option<u64>,
    pub operation_mix: Vec<WeightedOperation>,
    pub payload_mix: Vec<WeightedPayload>,
    pub digest: String,
}

/// A capacity/overload curve whose knee remains bound to raw repeat evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadCurveEvidence {
    pub id: String,
    pub scenario_digest: String,
    pub dimensions: BTreeMap<String, DimensionValue>,
    pub workload: WorkloadIdentity,
    pub criteria: Option<SustainabilityCriteria>,
    pub knee: Option<KneeResult>,
    pub claim: LoadClaim,
}

/// One named numeric result with an explicit unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Quantity {
    pub value: f64,
    pub unit: String,
}

/// One point in a scalar measurement series.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScalarPoint {
    pub dimensions: BTreeMap<String, DimensionValue>,
    pub quantity: Quantity,
    pub sample_count: u64,
    pub samples: Vec<f64>,
    pub min: f64,
    pub max: f64,
    pub robust_spread_ratio: f64,
}

/// Non-knee numeric evidence, including allocation or scaling efficiency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScalarEvidence {
    pub id: String,
    pub scenario_digest: String,
    pub workload: WorkloadIdentity,
    pub points: Vec<ScalarPoint>,
    pub derived_from: Vec<String>,
    pub max_robust_spread_ratio: f64,
}

/// Deterministic trace replay proof that preserves input ordering and identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceReplayEvidence {
    pub id: String,
    pub scenario_digest: String,
    pub catalog_id: String,
    pub event_count: u64,
    pub input_digest: String,
    pub replayed_digest: String,
    pub order_preserved: bool,
    pub hits: u64,
    pub misses: u64,
}

/// Derived comparison between two named measurements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonEvidence {
    pub id: String,
    pub scenario_digest: String,
    pub left_measurement_id: String,
    pub right_measurement_id: String,
    pub ratio: f64,
    pub unit: String,
    pub same_box: bool,
}

/// Typed evidence variants carried by one tier/suite report.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "evidence", rename_all = "snake_case")]
pub enum MeasurementEvidence {
    LoadCurve(LoadCurveEvidence),
    Scalar(ScalarEvidence),
    TraceReplay(TraceReplayEvidence),
    Comparison(ComparisonEvidence),
}

/// One canonical release-0.67 multi-measurement performance report.
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
    pub run_mode: EvidenceRunMode,
    pub surface: SurfaceIdentity,
    pub runner_profile: String,
    pub runner_contract: PerformanceProfile,
    pub runner_contract_digest: String,
    pub observed_runner: RunnerFingerprint,
    pub profile_validation: ProfileValidation,
    pub source: SourceIdentity,
    pub build: BuildIdentity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resp_endpoint_capability: Option<RespEndpointCapability>,
    pub measurements: Vec<MeasurementEvidence>,
    pub stable: bool,
    pub stability_reasons: Vec<String>,
}

/// Fail-closed error returned by canonical report emission.
#[derive(Debug, thiserror::Error)]
pub enum ReportWriteError {
    #[error("performance report failed semantic validation: {0:?}")]
    Semantic(Vec<String>),
    #[error("performance report JSON schema rejected the artifact: {0}")]
    Schema(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl PerfReport {
    /// Build a report, deriving runner eligibility and every semantic stability verdict.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        report_id: impl Into<String>,
        scenario_id: impl Into<String>,
        state_digest: impl Into<String>,
        run_mode: EvidenceRunMode,
        surface: SurfaceIdentity,
        runner_contract: PerformanceProfile,
        observed_runner: RunnerFingerprint,
        source: SourceIdentity,
        build: BuildIdentity,
        resp_endpoint_capability: Option<RespEndpointCapability>,
        measurements: Vec<MeasurementEvidence>,
        mut stability_reasons: Vec<String>,
    ) -> Self {
        let profile_validation = runner_contract.validate(&observed_runner);
        let runner_contract_digest = digest_json(&runner_contract);
        let scenario_digest = suite_scenario_digest(&measurements);
        let workload_digest = suite_workload_digest(&measurements);
        let seed = suite_seed(&measurements);
        let requested_state_digest = state_digest.into();
        let state_digest =
            canonical_suite_state_digest(&measurements).unwrap_or(requested_state_digest);
        let mut report = Self {
            schema_version: PERF_SCHEMA_VERSION,
            release: PERF_RELEASE.to_owned(),
            report_id: report_id.into(),
            scenario_id: scenario_id.into(),
            scenario_digest,
            workload_digest,
            state_digest,
            seed,
            run_mode,
            surface,
            runner_profile: runner_contract.name.clone(),
            runner_contract,
            runner_contract_digest,
            observed_runner,
            profile_validation,
            source,
            build,
            resp_endpoint_capability,
            measurements,
            stable: false,
            stability_reasons: Vec::new(),
        };
        stability_reasons.extend(report.structural_problems());
        stability_reasons.sort();
        stability_reasons.dedup();
        report.stable = stability_reasons.is_empty();
        report.stability_reasons = stability_reasons;
        report
    }

    /// Revalidate a deserialized report without trusting stored booleans or verdicts.
    pub fn validation_problems(&self) -> Vec<String> {
        let mut problems = self.structural_problems();
        problems.extend(self.stability_reasons.iter().cloned());
        let expected_stable = problems.is_empty();
        if self.stable != expected_stable {
            problems.push("stored stable flag does not match semantic validation".to_owned());
        }
        problems.sort();
        problems.dedup();
        problems
    }

    /// Serialize canonical JSON only after semantic and Draft 2020-12 validation.
    pub fn to_pretty_json(&self) -> Result<Vec<u8>, ReportWriteError> {
        let structural = self.structural_problems();
        let mut integrity = structural
            .iter()
            .filter(|problem| !self.stability_reasons.contains(problem))
            .map(|problem| format!("unreported semantic problem: {problem}"))
            .collect::<Vec<_>>();
        let expected_stable = structural.is_empty() && self.stability_reasons.is_empty();
        if self.stable != expected_stable {
            integrity.push("stored stable flag does not match semantic validation".to_owned());
        }
        if !integrity.is_empty() {
            return Err(ReportWriteError::Semantic(integrity));
        }
        let value = serde_json::to_value(self)?;
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/testing/schemas/perf-report.schema.json"
        ))?;
        let validator = jsonschema::validator_for(&schema)
            .map_err(|error| ReportWriteError::Schema(error.to_string()))?;
        validator
            .validate(&value)
            .map_err(|error| ReportWriteError::Schema(error.to_string()))?;
        Ok(serde_json::to_vec_pretty(&value)?)
    }

    fn structural_problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.schema_version != PERF_SCHEMA_VERSION || self.release != PERF_RELEASE {
            problems.push("report schema or release identity is incorrect".to_owned());
        }
        if [
            &self.report_id,
            &self.scenario_id,
            &self.scenario_digest,
            &self.workload_digest,
            &self.state_digest,
        ]
        .iter()
        .any(|value| value.is_empty())
        {
            problems.push("report identities and digests must be non-empty".to_owned());
        }
        if self.scenario_digest != suite_scenario_digest(&self.measurements)
            || self.workload_digest != suite_workload_digest(&self.measurements)
            || self.seed != suite_seed(&self.measurements)
        {
            problems.push("suite seed or digests do not match typed measurement inputs".to_owned());
        }
        if canonical_suite_state_digest(&self.measurements)
            .is_some_and(|expected| expected != self.state_digest)
        {
            problems.push(
                "report state digest does not bind the per-measurement steady-state digests"
                    .to_owned(),
            );
        }
        if [
            &self.surface.surface_kind,
            &self.surface.execution_mode,
            &self.surface.state_scope,
            &self.surface.network_boundary,
            &self.surface.claim_scope,
        ]
        .iter()
        .any(|value| value.is_empty())
        {
            problems.push("surface identity must be complete".to_owned());
        }
        if self.run_mode != EvidenceRunMode::ReferenceEvidence {
            problems.push("non-reference run modes are never stable ship evidence".to_owned());
        }
        if self.run_mode == EvidenceRunMode::Smoke && self.surface.claim_scope != "plumbing-only" {
            problems.push("smoke reports must use plumbing-only claim scope".to_owned());
        }
        let derived_validation = self.runner_contract.validate(&self.observed_runner);
        if self.runner_profile != self.runner_contract.name
            || self.runner_contract_digest != digest_json(&self.runner_contract)
            || self.profile_validation != derived_validation
            || !derived_validation.eligible
            || derived_validation.eligible != derived_validation.reasons.is_empty()
            || !self.runner_contract.require_dedicated
        {
            problems.push(
                "runner eligibility does not match the committed profile contract".to_owned(),
            );
        }
        if self.source.git_commit.is_empty()
            || self.source.cargo_lock_sha256.is_empty()
            || self.source.toolchain.is_empty()
            || self.source.build_flags.is_empty()
            || self.source.build_flags.iter().any(String::is_empty)
        {
            problems.push("source identity must be complete".to_owned());
        }
        if self.build.prebuild_contract_digest.is_empty()
            || self.build.prebuild_manifest_sha256.is_empty()
            || self.build.binary_sha256.is_empty()
            || self
                .build
                .binary_sha256
                .iter()
                .any(|(name, digest)| name.is_empty() || digest.is_empty())
        {
            problems.push("prebuild identity must be complete".to_owned());
        }
        if self.measurements.is_empty() {
            problems.push("at least one typed measurement is required".to_owned());
        }
        let mut ids = BTreeSet::new();
        for measurement in &self.measurements {
            let id = measurement.id();
            if !ids.insert(id.to_owned()) {
                problems.push(format!("duplicate measurement id: {id}"));
            }
            problems.extend(measurement.validation_problems());
        }
        let client_surface_measurements = [
            "client_surface_in_process_knee_at_slo_for_a_b_c",
            "concurrent_inflight_scaling_curve_1_10_100_1000",
            "client_surface_payload_sweep_100b_1kb_64kb_1mb",
            "client_surface_codec_dispatch_and_admission_rejection_cost",
        ];
        let is_client_surface_report = self.report_id.contains("client-surface")
            || client_surface_measurements
                .iter()
                .any(|measurement| ids.contains(*measurement));
        if is_client_surface_report {
            if self.surface.surface_kind != "client-surface"
                || self.surface.execution_mode != "in-process-axum-router"
                || self.surface.state_scope != "process-local"
                || self.surface.network_boundary != "none"
            {
                problems.push(
                    "W2 evidence must be labeled client-surface/in-process-axum-router/process-local/none; daemon and wire claims are forbidden"
                        .to_owned(),
                );
            }
            for required in client_surface_measurements {
                if !ids.contains(required) {
                    problems.push(format!(
                        "client-surface report is missing required W2 measurement {required}"
                    ));
                }
            }
            problems.extend(client_surface_validation_problems(self));
        }
        let resp_open_loop_measurements = [
            "resp_open_loop_get_set_knee_at_slo",
            "resp_open_loop_connection_and_pipeline_sweeps",
            "resp_open_loop_stall_is_visible_in_scheduled_latency",
        ];
        let is_resp_open_loop_report = self.report_id.contains("resp-loopback-fixture")
            || self.report_id.contains("node-resp-open-loop")
            || resp_open_loop_measurements
                .iter()
                .any(|measurement| ids.contains(*measurement));
        if is_resp_open_loop_report {
            let fixture_identity = self.run_mode == EvidenceRunMode::Smoke
                && self.surface.surface_kind == "resp-loopback-fixture"
                && self.surface.execution_mode == "in-process-product-resp-listener"
                && self.surface.state_scope == "fixture-process-local"
                && self.surface.network_boundary == "loopback-tcp"
                && self.surface.claim_scope == "plumbing-only";
            let reference_identity = self.run_mode == EvidenceRunMode::ReferenceEvidence
                && self.surface.surface_kind == "node-resp"
                && self.surface.execution_mode == "real-daemon-tcp-resp-open-loop"
                && self.surface.state_scope == "node-local"
                && self.surface.network_boundary == "loopback-tcp"
                && self.surface.claim_scope == "selected-endpoint-capacity";
            if !fixture_identity && !reference_identity {
                problems.push(
                    "W3 open-loop evidence must be either an explicit loopback fixture smoke or receipt-bound selected-daemon RESP evidence"
                        .to_owned(),
                );
            }
            match self.run_mode {
                EvidenceRunMode::Smoke if self.resp_endpoint_capability.is_some() => problems.push(
                    "W3 fixture smoke must not carry a daemon endpoint capability".to_owned(),
                ),
                EvidenceRunMode::ReferenceEvidence => {
                    problems.extend(resp_endpoint_capability_problems(self));
                }
                EvidenceRunMode::CiTripwire => {}
                EvidenceRunMode::Smoke => {}
            }
            for required in resp_open_loop_measurements {
                if !ids.contains(required) {
                    problems.push(format!(
                        "RESP open-loop report is missing required W3 measurement {required}"
                    ));
                }
            }
            if ids.contains("redis_benchmark_get_set_mset_throughput_and_interop")
                || self.measurements.iter().any(|measurement| {
                    matches!(
                        measurement,
                        MeasurementEvidence::Comparison(_)
                            | MeasurementEvidence::LoadCurve(LoadCurveEvidence {
                                claim: LoadClaim::SupplementalClosedLoop,
                                ..
                            })
                    )
                })
            {
                problems.push(
                    "W3 closed-loop external-tool data and comparisons must not appear in the open-loop RESP artifact"
                        .to_owned(),
                );
            }
            let forbidden_identity = [
                &self.surface.surface_kind,
                &self.surface.execution_mode,
                &self.surface.state_scope,
                &self.surface.claim_scope,
            ]
            .iter()
            .any(|value| {
                let value = value.to_ascii_lowercase();
                value.contains("native")
                    || value.contains("client-surface")
                    || value.contains("cluster-capacity")
                    || value.contains("distributed-value")
            });
            if forbidden_identity {
                problems.push(
                    "W3 RESP evidence cannot claim native wire, client-surface, cluster capacity, or distributed values"
                        .to_owned(),
                );
            }
            problems.extend(resp_open_loop_validation_problems(self));
        }
        for measurement in &self.measurements {
            match measurement {
                MeasurementEvidence::Scalar(value) => {
                    for dependency in &value.derived_from {
                        if !ids.contains(dependency) {
                            problems.push(format!(
                                "scalar {} references unknown measurement {dependency}",
                                value.id
                            ));
                        }
                    }
                }
                MeasurementEvidence::Comparison(value) => {
                    for dependency in [&value.left_measurement_id, &value.right_measurement_id] {
                        if !ids.contains(dependency) {
                            problems.push(format!(
                                "comparison {} references unknown measurement {dependency}",
                                value.id
                            ));
                        }
                    }
                    let left = self
                        .measurements
                        .iter()
                        .find(|measurement| measurement.id() == value.left_measurement_id)
                        .and_then(MeasurementEvidence::headline_value);
                    let right = self
                        .measurements
                        .iter()
                        .find(|measurement| measurement.id() == value.right_measurement_id)
                        .and_then(MeasurementEvidence::headline_value);
                    match (left, right) {
                        (Some(left), Some(right)) if right != 0.0 => {
                            let expected = left / right;
                            if relative_difference(expected, value.ratio) > f64::EPSILON * 8.0 {
                                problems.push(format!(
                                    "comparison {} ratio does not match its dependencies",
                                    value.id
                                ));
                            }
                        }
                        _ => problems.push(format!(
                            "comparison {} dependencies have no comparable headline",
                            value.id
                        )),
                    }
                }
                _ => {}
            }
        }
        if dependency_cycle(&self.measurements) {
            problems.push("measurement dependency graph contains a cycle".to_owned());
        }
        problems
    }
}

fn resp_endpoint_capability_problems(report: &PerfReport) -> Vec<String> {
    let Some(capability) = &report.resp_endpoint_capability else {
        return vec![
            "W3 reference evidence is missing its typed daemon endpoint capability".to_owned(),
        ];
    };
    let loopback_zero = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let server_matches = report.build.binary_sha256.iter().any(|(id, digest)| {
        id == "hydracache-server" && digest == &capability.server_binary_sha256
    });
    let loadgen_matches = report.build.binary_sha256.iter().any(|(id, digest)| {
        id == "hydracache-loadgen" && digest == &capability.loadgen_binary_sha256
    });
    let valid = capability.schema_version == 1
        && capability.pid != 0
        && capability.started_unix_nanos != 0
        && capability.direct_prebuilt_exec
        && capability.fresh_data_dir
        && capability.config.role == "local"
        && capability.config.listen_addr == loopback_zero
        && capability.config.cluster_addr == loopback_zero
        && capability.config.storage_dir.is_absolute()
        && capability.config.admin_enabled
        && capability.config.redis_enabled
        && !capability.config.redis_auth_required
        && !capability.config.rediss_enabled
        && capability.config.redis_addr.ip().is_loopback()
        && capability.config.redis_addr.port() != 0
        && capability.config.admin_addr.ip().is_loopback()
        && capability.config.admin_addr.port() != 0
        && capability.config.redis_addr != capability.config.admin_addr
        && capability.selected_endpoint
            == format!("hydracache-server@{}", capability.config.redis_addr)
        && is_lower_sha256(&capability.server_binary_sha256)
        && is_lower_sha256(&capability.loadgen_binary_sha256)
        && capability.prebuild_manifest_sha256 == report.build.prebuild_manifest_sha256
        && capability.prebuild_contract_digest == report.build.prebuild_contract_digest
        && capability.source_commit == report.source.git_commit
        && server_matches
        && loadgen_matches;
    if valid {
        Vec::new()
    } else {
        vec![
            "W3 typed daemon capability is incomplete or does not cross-bind the exact source, prebuild, binaries, and direct listener configuration"
                .to_owned(),
        ]
    }
}

fn resp_open_loop_validation_problems(report: &PerfReport) -> Vec<String> {
    const RAW: [(&str, &str); 3] = [
        ("resp_open_loop_get_set_knee_at_slo_workload_a", "A"),
        ("resp_open_loop_get_set_knee_at_slo_workload_b", "B"),
        ("resp_open_loop_get_set_knee_at_slo_workload_c", "C"),
    ];
    const REFERENCE_RATES: [u64; 5] = [1_000, 5_000, 10_000, 25_000, 50_000];
    let mut problems = Vec::new();
    let (expected_endpoint_kind, expected_state_scope, expected_selected_prefix) =
        match report.run_mode {
            EvidenceRunMode::Smoke => (
                "resp-loopback-fixture",
                "fixture-process-local",
                "resp-loopback-fixture@",
            ),
            EvidenceRunMode::ReferenceEvidence => ("node-resp", "node-local", "hydracache-server@"),
            EvidenceRunMode::CiTripwire => ("ci-resp-tripwire", "node-local", "ci-resp-tripwire@"),
        };
    if report.run_mode == EvidenceRunMode::ReferenceEvidence
        && (!report
            .build
            .binary_sha256
            .iter()
            .any(|(name, digest)| name == "hydracache-server" && is_lower_sha256(digest))
            || !report
                .build
                .binary_sha256
                .iter()
                .any(|(name, digest)| name == "hydracache-loadgen" && is_lower_sha256(digest))
            || !is_lower_sha256(&report.build.prebuild_manifest_sha256)
            || report.source.git_commit.starts_with("smoke-")
            || report.build.prebuild_contract_digest.starts_with("smoke-"))
    {
        problems.push(
            "W3 reference evidence must bind receipt-grade server/loadgen binaries, prebuild manifest, and source commit"
                .to_owned(),
        );
    }
    let Some(MeasurementEvidence::Scalar(aggregate)) = report
        .measurements
        .iter()
        .find(|measurement| measurement.id() == "resp_open_loop_get_set_knee_at_slo")
    else {
        return vec!["W3 aggregate A/B/C RESP knee is missing or has the wrong type".to_owned()];
    };
    let expected_dependencies = RAW
        .iter()
        .map(|(id, _)| (*id).to_owned())
        .collect::<Vec<_>>();
    if aggregate.derived_from != expected_dependencies || aggregate.points.len() != RAW.len() {
        problems.push("W3 aggregate must depend on exactly the raw A/B/C RESP knees".to_owned());
    }
    let expected_aggregate_contract = match report.run_mode {
        EvidenceRunMode::ReferenceEvidence => (10_000, 0.15),
        EvidenceRunMode::Smoke => (32, 1_000.0),
        EvidenceRunMode::CiTripwire => (32, 1_000.0),
    };
    if aggregate.workload.key_count != Some(expected_aggregate_contract.0)
        || aggregate.max_robust_spread_ratio != expected_aggregate_contract.1
    {
        problems.push(
            "W3 aggregate key-count/spread contract does not match its evidence run mode"
                .to_owned(),
        );
    }
    let mut scenario_inputs = Vec::new();
    let mut workload_inputs = Vec::new();
    let mut selected_endpoints = BTreeSet::new();
    let mut endpoint_capability_digests = BTreeSet::new();
    for (raw_id, workload_name) in RAW {
        let Some(MeasurementEvidence::LoadCurve(raw)) = report
            .measurements
            .iter()
            .find(|measurement| measurement.id() == raw_id)
        else {
            problems.push(format!(
                "W3 raw RESP knee {raw_id} is missing or has the wrong type"
            ));
            continue;
        };
        workload_inputs.push((raw_id.to_owned(), raw.workload.digest.clone()));
        if let Some(DimensionValue::Text(endpoint)) = raw.dimensions.get("selected_endpoint") {
            selected_endpoints.insert(endpoint.clone());
        }
        if let Some(DimensionValue::Text(digest)) = raw.dimensions.get("endpoint_capability_digest")
        {
            endpoint_capability_digests.insert(digest.clone());
            scenario_inputs.push((raw_id.to_owned(), raw.scenario_digest.clone()));
        }
        if raw.claim != LoadClaim::CapacityKnee
            || raw.dimensions.get("workload")
                != Some(&DimensionValue::Text(workload_name.to_owned()))
            || raw.dimensions.get("methodology")
                != Some(&DimensionValue::Text("open-loop-scheduled-send".to_owned()))
            || raw.dimensions.get("real_tcp") != Some(&DimensionValue::Bool(true))
            || !matches!(
                raw.dimensions.get("selected_endpoint"),
                Some(DimensionValue::Text(endpoint)) if endpoint.starts_with(expected_selected_prefix)
            )
            || raw.dimensions.get("endpoint_kind")
                != Some(&DimensionValue::Text(expected_endpoint_kind.to_owned()))
            || raw.dimensions.get("state_scope")
                != Some(&DimensionValue::Text(expected_state_scope.to_owned()))
            || !matches!(
                raw.dimensions.get("endpoint_capability_digest"),
                Some(DimensionValue::Text(digest)) if is_lower_sha256(digest)
            )
        {
            problems.push(format!(
                "W3 raw RESP knee {raw_id} lost its selected-endpoint open-loop TCP identity"
            ));
        }
        if report.run_mode == EvidenceRunMode::ReferenceEvidence {
            let exact_dimensions = raw.dimensions.get("preload_operations")
                == Some(&DimensionValue::U64(10_000))
                && raw.dimensions.get("warmup_operations") == Some(&DimensionValue::U64(10_000))
                && raw.dimensions.get("steady_operations") == Some(&DimensionValue::U64(100_000))
                && raw.dimensions.get("repeats") == Some(&DimensionValue::U64(5))
                && raw.dimensions.get("key_count") == Some(&DimensionValue::U64(10_000))
                && raw.dimensions.get("repeat_isolation")
                    == Some(&DimensionValue::Text(
                        "logical-keyspace-reset-and-counter-zero".to_owned(),
                    ))
                && raw.dimensions.get("daemon_reused_across_repeats")
                    == Some(&DimensionValue::Bool(true));
            let exact_criteria = raw.criteria.as_ref().is_some_and(|criteria| {
                criteria.p99_slo_us == 5_000
                    && criteria.p999_slo_us == Some(20_000)
                    && criteria.min_achieved_ratio == 0.95
                    && criteria.max_error_ratio == 0.0
                    && criteria.max_timeout_ratio == 0.0
                    && criteria.max_rejection_ratio == 0.0
                    && criteria.max_drain_ms == 10_000
                    && criteria.max_robust_spread_ratio == 0.15
            });
            let exact_knee = raw.knee.as_ref().is_some_and(|knee| {
                knee.evaluated.len() == REFERENCE_RATES.len()
                    && knee
                        .evaluated
                        .iter()
                        .zip(REFERENCE_RATES)
                        .all(|(point, rate)| {
                            point.sample.offered_rate_per_second == rate as f64
                                && point.repeats.len() == 5
                                && point.repeats.iter().all(|repeat| {
                                    repeat.phase.reset_operations == 1
                                        && repeat.phase.preload_operations == 10_000
                                        && repeat.phase.warmup_operations == 10_000
                                        && repeat.phase.steady_operations == 100_000
                                        && repeat.phase.warmup_samples_in_steady_histogram == 0
                                })
                        })
            });
            if !exact_dimensions || !exact_criteria || !exact_knee {
                problems.push(format!(
                    "W3 reference knee {raw_id} does not retain the exact committed 10k preload/warmup, 100k steady, five-repeat, five-rate contract"
                ));
            }
        }
        let expected_samples = raw.knee.as_ref().and_then(|knee| {
            let selected = knee.sustainable_rate_per_second?;
            knee.evaluated
                .iter()
                .find(|point| point.sample.offered_rate_per_second == selected)
                .map(|point| {
                    point
                        .repeats
                        .iter()
                        .map(|repeat| repeat.steady.achieved_rate_per_second)
                        .collect::<Vec<_>>()
                })
        });
        let aggregate_points = aggregate
            .points
            .iter()
            .filter(|point| {
                point.dimensions.get("workload")
                    == Some(&DimensionValue::Text(workload_name.to_owned()))
            })
            .collect::<Vec<_>>();
        if aggregate_points.len() != 1
            || expected_samples.as_ref() != aggregate_points.first().map(|point| &point.samples)
        {
            problems.push(format!(
                "W3 aggregate point {workload_name} does not recompute from raw RESP knee {raw_id}"
            ));
        }
    }
    if aggregate.scenario_digest != derived_identity_digest(&scenario_inputs)
        || aggregate.workload.digest != derived_identity_digest(&workload_inputs)
    {
        problems
            .push("W3 aggregate digests do not bind the effective A/B/C raw RESP knees".to_owned());
    }
    if selected_endpoints.len() != 1 {
        problems
            .push("W3 raw A/B/C curves must bind one identical selected RESP endpoint".to_owned());
    }
    if endpoint_capability_digests.len() != 1 {
        problems.push(
            "W3 raw A/B/C curves must bind one identical endpoint capability digest".to_owned(),
        );
    }
    if report.run_mode == EvidenceRunMode::ReferenceEvidence {
        let typed_capability_digest = report.resp_endpoint_capability.as_ref().map(digest_json);
        let typed_selected_endpoint = report
            .resp_endpoint_capability
            .as_ref()
            .map(|capability| capability.selected_endpoint.clone());
        if typed_capability_digest
            .as_ref()
            .is_none_or(|digest| !endpoint_capability_digests.contains(digest))
            || typed_selected_endpoint
                .as_ref()
                .is_none_or(|endpoint| !selected_endpoints.contains(endpoint))
        {
            problems.push(
                "W3 measurement run binding does not match the typed daemon endpoint capability"
                    .to_owned(),
            );
        }
    }

    let Some(MeasurementEvidence::Scalar(matrix)) = report
        .measurements
        .iter()
        .find(|measurement| measurement.id() == "resp_open_loop_connection_and_pipeline_sweeps")
    else {
        problems.push("W3 RESP connection/pipeline sweep has the wrong type".to_owned());
        return problems;
    };
    let matrix_pairs = matrix
        .points
        .iter()
        .filter_map(|point| {
            let Some(DimensionValue::U64(connections)) = point.dimensions.get("connections") else {
                return None;
            };
            let Some(DimensionValue::U64(pipeline)) = point.dimensions.get("pipeline") else {
                return None;
            };
            (point.dimensions.get("methodology")
                == Some(&DimensionValue::Text("open-loop-scheduled-send".to_owned()))
                && point.dimensions.get("real_tcp") == Some(&DimensionValue::Bool(true))
                && point.dimensions.get("endpoint_kind")
                    == Some(&DimensionValue::Text(expected_endpoint_kind.to_owned()))
                && point.dimensions.get("state_scope")
                    == Some(&DimensionValue::Text(expected_state_scope.to_owned()))
                && matches!(
                    point.dimensions.get("selected_endpoint"),
                    Some(DimensionValue::Text(endpoint))
                        if selected_endpoints.contains(endpoint)
                            && endpoint.starts_with(expected_selected_prefix)
                )
                && matches!(
                    point.dimensions.get("endpoint_capability_digest"),
                    Some(DimensionValue::Text(digest))
                        if endpoint_capability_digests.contains(digest) && is_lower_sha256(digest)
                )
                && matches!(
                    point.dimensions.get("logical_operations_per_second"),
                    Some(DimensionValue::F64(value)) if value.is_finite() && *value > 0.0
                )
                && point.quantity.unit == "scheduled_send_p99_microseconds")
                .then_some((*connections, *pipeline))
        })
        .collect::<BTreeSet<_>>();
    let expected_matrix = BTreeSet::from([(1, 1), (1, 10), (10, 1), (10, 10), (100, 1), (100, 10)]);
    if matrix.points.len() != 6 || matrix_pairs != expected_matrix {
        problems.push(
            "W3 RESP sweep must contain the exact real-TCP 1/10/100 connection by 1/10 pipeline matrix"
                .to_owned(),
        );
    }
    if report.run_mode == EvidenceRunMode::ReferenceEvidence
        && (matrix.max_robust_spread_ratio != 0.15
            || matrix.points.iter().any(|point| {
                point.dimensions.get("key_count") != Some(&DimensionValue::U64(10_000))
                    || point.dimensions.get("preload_entries") != Some(&DimensionValue::U64(10_000))
                    || point.dimensions.get("repeats") != Some(&DimensionValue::U64(5))
                    || point.dimensions.get("repeat_isolation")
                        != Some(&DimensionValue::Text(
                            "logical-keyspace-reset-and-counter-zero".to_owned(),
                        ))
                    || point.dimensions.get("daemon_reused_across_repeats")
                        != Some(&DimensionValue::Bool(true))
                    || point
                        .dimensions
                        .get("verified_pipeline_exchanges_per_repeat")
                        != Some(&DimensionValue::U64(10_000))
                    || point.samples.len() != 5
            }))
    {
        problems.push(
            "W3 reference RESP sweep does not retain the committed 10k-operation, 10k-key, five-repeat matrix contract"
                .to_owned(),
        );
    }

    let Some(MeasurementEvidence::Scalar(stall)) = report.measurements.iter().find(|measurement| {
        measurement.id() == "resp_open_loop_stall_is_visible_in_scheduled_latency"
    }) else {
        problems.push("W3 RESP stall-visibility proof has the wrong type".to_owned());
        return problems;
    };
    let baseline = stall.points.iter().find(|point| {
        point.dimensions.get("injected_loadgen_delay_us") == Some(&DimensionValue::U64(0))
    });
    let delayed = stall.points.iter().find(|point| {
        matches!(
            point.dimensions.get("injected_loadgen_delay_us"),
            Some(DimensionValue::U64(value)) if *value > 0
        )
    });
    if stall.points.len() != 2
        || baseline.is_none()
        || delayed.is_none()
        || delayed
            .zip(baseline)
            .is_none_or(|(delayed, baseline)| delayed.quantity.value <= baseline.quantity.value)
        || stall.points.iter().any(|point| {
            point.dimensions.get("methodology")
                != Some(&DimensionValue::Text("open-loop-scheduled-send".to_owned()))
                || point.dimensions.get("real_tcp") != Some(&DimensionValue::Bool(true))
                || point.dimensions.get("endpoint_kind")
                    != Some(&DimensionValue::Text(expected_endpoint_kind.to_owned()))
                || point.dimensions.get("state_scope")
                    != Some(&DimensionValue::Text(expected_state_scope.to_owned()))
                || !matches!(
                    point.dimensions.get("selected_endpoint"),
                    Some(DimensionValue::Text(endpoint))
                        if selected_endpoints.contains(endpoint)
                            && endpoint.starts_with(expected_selected_prefix)
                )
                || !matches!(
                    point.dimensions.get("endpoint_capability_digest"),
                    Some(DimensionValue::Text(digest))
                        if endpoint_capability_digests.contains(digest) && is_lower_sha256(digest)
                )
                || point.quantity.unit != "scheduled_send_p99_microseconds"
                || point.dimensions.get("instrument_scope")
                    != Some(&DimensionValue::Text(
                        "bounded-falsifiability-probe-not-capacity".to_owned(),
                    ))
        })
    {
        problems.push(
            "W3 stall proof must show a loadgen-injected delay increasing scheduled-send latency over real TCP"
                .to_owned(),
        );
    }
    problems
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn client_surface_validation_problems(report: &PerfReport) -> Vec<String> {
    const RAW: [(&str, &str); 3] = [
        ("client_surface_in_process_knee_at_slo_workload_a", "A"),
        ("client_surface_in_process_knee_at_slo_workload_b", "B"),
        ("client_surface_in_process_knee_at_slo_workload_c", "C"),
    ];
    let mut problems = Vec::new();
    let Some(MeasurementEvidence::Scalar(aggregate)) = report
        .measurements
        .iter()
        .find(|measurement| measurement.id() == "client_surface_in_process_knee_at_slo_for_a_b_c")
    else {
        return vec!["W2 aggregate A/B/C measurement is missing or has the wrong type".to_owned()];
    };

    let expected_dependencies = RAW
        .iter()
        .map(|(id, _)| (*id).to_owned())
        .collect::<Vec<_>>();
    if aggregate.derived_from != expected_dependencies || aggregate.points.len() != RAW.len() {
        problems.push("W2 aggregate must depend on exactly the A/B/C raw knees".to_owned());
    }
    let mut scenario_inputs = Vec::new();
    let mut workload_inputs = Vec::new();
    for (raw_id, workload_name) in RAW {
        let Some(MeasurementEvidence::LoadCurve(raw)) = report
            .measurements
            .iter()
            .find(|measurement| measurement.id() == raw_id)
        else {
            problems.push(format!(
                "W2 raw knee {raw_id} is missing or has the wrong type"
            ));
            continue;
        };
        scenario_inputs.push((raw_id.to_owned(), raw.scenario_digest.clone()));
        workload_inputs.push((raw_id.to_owned(), raw.workload.digest.clone()));
        if raw.claim != LoadClaim::CapacityKnee
            || raw.dimensions.get("workload")
                != Some(&DimensionValue::Text(workload_name.to_owned()))
        {
            problems.push(format!(
                "W2 raw knee {raw_id} has the wrong workload identity"
            ));
        }
        let expected_samples = raw.knee.as_ref().and_then(|knee| {
            let selected = knee.sustainable_rate_per_second?;
            knee.evaluated
                .iter()
                .find(|point| point.sample.offered_rate_per_second == selected)
                .map(|point| {
                    point
                        .repeats
                        .iter()
                        .map(|repeat| repeat.steady.achieved_rate_per_second)
                        .collect::<Vec<_>>()
                })
        });
        let aggregate_points = aggregate
            .points
            .iter()
            .filter(|point| {
                point.dimensions.get("workload")
                    == Some(&DimensionValue::Text(workload_name.to_owned()))
            })
            .collect::<Vec<_>>();
        if aggregate_points.len() != 1
            || expected_samples.as_ref() != aggregate_points.first().map(|point| &point.samples)
        {
            problems.push(format!(
                "W2 aggregate point {workload_name} does not recompute from raw knee {raw_id}"
            ));
        }
    }
    if aggregate.scenario_digest != derived_identity_digest(&scenario_inputs)
        || aggregate.workload.digest != derived_identity_digest(&workload_inputs)
    {
        problems.push("W2 aggregate digests do not bind the effective A/B/C raw knees".to_owned());
    }

    let Some(MeasurementEvidence::Scalar(concurrency)) = report
        .measurements
        .iter()
        .find(|measurement| measurement.id() == "concurrent_inflight_scaling_curve_1_10_100_1000")
    else {
        problems.push("W2 concurrent in-flight measurement has the wrong type".to_owned());
        return problems;
    };
    let observed_inflight = concurrency
        .points
        .iter()
        .filter_map(|point| {
            let Some(DimensionValue::U64(declared)) = point.dimensions.get("concurrent_inflight")
            else {
                return None;
            };
            let Some(DimensionValue::U64(observed)) =
                point.dimensions.get("observed_inflight_high_water")
            else {
                return None;
            };
            (*declared == *observed
                && point.dimensions.get("measurement_boundary")
                    == Some(&DimensionValue::Text(
                        "framed-request-lifetime-at-router-oneshot".to_owned(),
                    ))
                && point.dimensions.get("not_connections") == Some(&DimensionValue::Bool(true)))
            .then_some(*declared)
        })
        .collect::<BTreeSet<_>>();
    if concurrency.points.len() != 4 || observed_inflight != BTreeSet::from([1, 10, 100, 1_000]) {
        problems.push(
            "W2 concurrency points must prove observed in-flight high-water 1/10/100/1000"
                .to_owned(),
        );
    }

    let Some(MeasurementEvidence::Scalar(path_cost)) =
        report.measurements.iter().find(|measurement| {
            measurement.id() == "client_surface_codec_dispatch_and_admission_rejection_cost"
        })
    else {
        problems.push("W2 path-cost measurement has the wrong type".to_owned());
        return problems;
    };
    let path_payloads = path_cost
        .points
        .iter()
        .filter_map(|point| {
            let Some(DimensionValue::Text(path)) = point.dimensions.get("path") else {
                return None;
            };
            let Some(DimensionValue::U64(bytes)) = point.dimensions.get("payload_bytes") else {
                return None;
            };
            Some((path.clone(), *bytes))
        })
        .collect::<BTreeMap<_, _>>();
    let normal_payload = path_payloads.get("codec_encode_decode").copied();
    let dispatch_payload = path_payloads.get("router_dispatch").copied();
    let oversized_payload = path_payloads.get("oversized_admission_rejection").copied();
    let workload_payloads = path_cost
        .workload
        .payload_mix
        .iter()
        .map(|payload| payload.bytes)
        .collect::<BTreeSet<_>>();
    if path_cost.points.len() != 3
        || path_payloads.len() != 3
        || normal_payload.is_none()
        || normal_payload != dispatch_payload
        || !matches!((normal_payload, oversized_payload), (Some(normal), Some(oversized)) if oversized > normal)
        || workload_payloads
            != normal_payload
                .into_iter()
                .chain(oversized_payload)
                .collect::<BTreeSet<_>>()
    {
        problems.push(
            "W2 path-cost dimensions and workload identity must bind normal and oversized payloads"
                .to_owned(),
        );
    }
    problems
}

impl MeasurementEvidence {
    fn id(&self) -> &str {
        match self {
            Self::LoadCurve(value) => &value.id,
            Self::Scalar(value) => &value.id,
            Self::TraceReplay(value) => &value.id,
            Self::Comparison(value) => &value.id,
        }
    }

    fn headline_value(&self) -> Option<f64> {
        match self {
            Self::LoadCurve(value) => value
                .knee
                .as_ref()
                .and_then(|knee| knee.sustainable_rate_per_second),
            Self::Scalar(value) if value.points.len() == 1 => Some(value.points[0].quantity.value),
            Self::Comparison(value) => Some(value.ratio),
            _ => None,
        }
    }

    fn validation_problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.id().is_empty() {
            problems.push("measurement id must be non-empty".to_owned());
        }
        match self {
            Self::LoadCurve(value) => {
                if value.scenario_digest.is_empty() {
                    problems.push(format!("measurement {} has no scenario digest", value.id));
                }
                problems.extend(workload_problems(&value.workload));
                match (&value.criteria, &value.knee) {
                    (Some(criteria), Some(knee)) => {
                        problems.extend(criteria.knee_validation_problems(knee));
                        if value.claim == LoadClaim::CapacityKnee
                            && knee.sustainable_rate_per_second.is_none()
                        {
                            problems.push(format!(
                                "capacity measurement {} has no sustainable rate",
                                value.id
                            ));
                        }
                        let reset_digest = knee
                            .evaluated
                            .first()
                            .and_then(|point| point.repeats.first())
                            .map(|repeat| &repeat.reset_state_digest);
                        let preloaded_digest = knee
                            .evaluated
                            .first()
                            .and_then(|point| point.repeats.first())
                            .map(|repeat| &repeat.preloaded_state_digest);
                        let steady_state_digest = knee
                            .evaluated
                            .first()
                            .and_then(|point| point.repeats.first())
                            .map(|repeat| &repeat.state_digest);
                        let phase_shape = knee
                            .evaluated
                            .first()
                            .and_then(|point| point.repeats.first())
                            .map(|repeat| {
                                (
                                    repeat.phase.reset_operations,
                                    repeat.phase.preload_operations,
                                    repeat.phase.warmup_operations,
                                    repeat.phase.steady_operations,
                                )
                            });
                        for point in &knee.evaluated {
                            if point.repeats.iter().any(|repeat| {
                                Some(&repeat.state_digest) != steady_state_digest
                                    || Some(&repeat.reset_state_digest) != reset_digest
                                    || Some(&repeat.preloaded_state_digest) != preloaded_digest
                                    || Some((
                                        repeat.phase.reset_operations,
                                        repeat.phase.preload_operations,
                                        repeat.phase.warmup_operations,
                                        repeat.phase.steady_operations,
                                    )) != phase_shape
                            }) {
                                problems.push(format!(
                                    "measurement {} repeat state or phase is not reproducible within the curve",
                                    value.id
                                ));
                            }
                        }
                    }
                    _ => problems.push(format!(
                        "load-curve measurement {} requires criteria and knee",
                        value.id
                    )),
                }
                if value
                    .dimensions
                    .iter()
                    .any(|(name, dimension)| name.is_empty() || !valid_dimension(dimension))
                {
                    problems.push(format!("measurement {} has an invalid dimension", value.id));
                }
            }
            Self::Scalar(value) => {
                if value.scenario_digest.is_empty()
                    || !value.max_robust_spread_ratio.is_finite()
                    || value.max_robust_spread_ratio < 0.0
                {
                    problems.push(format!(
                        "scalar measurement {} contract is incomplete",
                        value.id
                    ));
                }
                problems.extend(workload_problems(&value.workload));
                for point in &value.points {
                    if point.robust_spread_ratio.is_finite()
                        && point.robust_spread_ratio > value.max_robust_spread_ratio
                    {
                        problems.push(format!(
                            "scalar measurement {} robust spread {} exceeds {} at {:?}",
                            value.id,
                            point.robust_spread_ratio,
                            value.max_robust_spread_ratio,
                            point.dimensions
                        ));
                    }
                }
                if value.points.is_empty()
                    || value.points.iter().any(|point| {
                        point.sample_count < 3
                            || point.sample_count as usize != point.samples.len()
                            || point.samples.iter().any(|sample| !sample.is_finite())
                            || !point.quantity.value.is_finite()
                            || point.quantity.unit.is_empty()
                            || point.quantity.value != median(&point.samples)
                            || point.min
                                != point
                                    .samples
                                    .iter()
                                    .copied()
                                    .min_by(f64::total_cmp)
                                    .unwrap_or(f64::NAN)
                            || point.max
                                != point
                                    .samples
                                    .iter()
                                    .copied()
                                    .max_by(f64::total_cmp)
                                    .unwrap_or(f64::NAN)
                            || point.robust_spread_ratio != relative_range(&point.samples)
                            || point.robust_spread_ratio > value.max_robust_spread_ratio
                            || point.dimensions.iter().any(|(name, dimension)| {
                                name.is_empty() || !valid_dimension(dimension)
                            })
                    })
                {
                    problems.push(format!("scalar measurement {} is incomplete", value.id));
                }
            }
            Self::TraceReplay(value) => {
                let accounted = value.hits.checked_add(value.misses);
                if value.scenario_digest.is_empty()
                    || value.catalog_id.is_empty()
                    || value.input_digest.is_empty()
                    || value.replayed_digest.is_empty()
                    || value.event_count == 0
                    || accounted != Some(value.event_count)
                    || !value.order_preserved
                    || value.input_digest != value.replayed_digest
                {
                    problems.push(format!("trace replay {} is incomplete", value.id));
                }
            }
            Self::Comparison(value) => {
                if value.scenario_digest.is_empty()
                    || value.left_measurement_id.is_empty()
                    || value.right_measurement_id.is_empty()
                    || value.left_measurement_id == value.right_measurement_id
                    || !value.ratio.is_finite()
                    || value.ratio < 0.0
                    || value.unit.is_empty()
                    || !value.same_box
                {
                    problems.push(format!("comparison {} is incomplete", value.id));
                }
            }
        }
        problems
    }
}

fn workload_problems(workload: &WorkloadIdentity) -> Vec<String> {
    let mut problems = Vec::new();
    if workload.generator.is_empty()
        || workload.generator_version.is_empty()
        || workload.digest.is_empty()
        || workload.key_count == Some(0)
        || workload
            .key_distribution
            .as_ref()
            .is_some_and(|distribution| {
                distribution.kind.is_empty()
                    || distribution
                        .theta
                        .is_some_and(|theta| !theta.is_finite() || theta < 0.0)
            })
        || workload.operation_mix.is_empty()
        || workload.operation_mix.iter().any(|operation| {
            operation.operation.is_empty()
                || !operation.weight.is_finite()
                || operation.weight <= 0.0
        })
        || workload.payload_mix.iter().any(|payload| {
            payload.bytes == 0 || !payload.weight.is_finite() || payload.weight <= 0.0
        })
    {
        problems.push("workload identity is incomplete".to_owned());
    }
    problems
}

fn valid_dimension(value: &DimensionValue) -> bool {
    match value {
        DimensionValue::Text(value) => !value.is_empty(),
        DimensionValue::F64(value) => value.is_finite(),
        _ => true,
    }
}

fn digest_json(value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_else(|_| b"invalid-json".to_vec());
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Canonical digest for a derived measurement's exact ordered inputs.
/// Kept crate-visible so producers and the report validator use one algorithm.
pub(crate) fn derived_identity_digest(inputs: &[(String, String)]) -> String {
    digest_json(&inputs)
}

fn suite_scenario_digest(measurements: &[MeasurementEvidence]) -> String {
    let inputs = measurements
        .iter()
        .map(|measurement| {
            let digest = match measurement {
                MeasurementEvidence::LoadCurve(value) => &value.scenario_digest,
                MeasurementEvidence::Scalar(value) => &value.scenario_digest,
                MeasurementEvidence::TraceReplay(value) => &value.scenario_digest,
                MeasurementEvidence::Comparison(value) => &value.scenario_digest,
            };
            (measurement.id(), digest)
        })
        .collect::<Vec<_>>();
    digest_json(&inputs)
}

fn suite_workload_digest(measurements: &[MeasurementEvidence]) -> String {
    let inputs = measurements
        .iter()
        .map(|measurement| {
            let digest = match measurement {
                MeasurementEvidence::LoadCurve(value) => value.workload.digest.as_str(),
                MeasurementEvidence::Scalar(value) => value.workload.digest.as_str(),
                MeasurementEvidence::TraceReplay(value) => value.input_digest.as_str(),
                MeasurementEvidence::Comparison(value) => value.scenario_digest.as_str(),
            };
            (measurement.id(), digest)
        })
        .collect::<Vec<_>>();
    digest_json(&inputs)
}

fn canonical_suite_state_digest(measurements: &[MeasurementEvidence]) -> Option<String> {
    let inputs = measurements
        .iter()
        .filter_map(|measurement| match measurement {
            MeasurementEvidence::LoadCurve(value) => value
                .knee
                .as_ref()
                .and_then(|knee| knee.evaluated.first())
                .and_then(|point| point.repeats.first())
                .map(|repeat| (value.id.clone(), repeat.state_digest.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();
    let distinct = inputs
        .iter()
        .map(|(_, digest)| digest)
        .collect::<BTreeSet<_>>();
    match distinct.len() {
        0 => None,
        1 => distinct.first().map(|digest| (*digest).clone()),
        _ => Some(derived_identity_digest(&inputs)),
    }
}

fn suite_seed(measurements: &[MeasurementEvidence]) -> u64 {
    let inputs = measurements
        .iter()
        .filter_map(|measurement| match measurement {
            MeasurementEvidence::LoadCurve(value) => value
                .workload
                .seed
                .map(|seed| (measurement.id().to_owned(), seed)),
            MeasurementEvidence::Scalar(value) => value
                .workload
                .seed
                .map(|seed| (measurement.id().to_owned(), seed)),
            _ => None,
        })
        .collect::<Vec<_>>();
    let distinct = inputs
        .iter()
        .map(|(_, seed)| *seed)
        .collect::<BTreeSet<_>>();
    if distinct.len() == 1 {
        return *distinct.first().expect("one distinct seed exists");
    }
    let digest = Sha256::digest(
        serde_json::to_vec(&inputs).unwrap_or_else(|_| b"invalid-suite-seed-input".to_vec()),
    );
    u64::from_le_bytes(
        digest[..8]
            .try_into()
            .expect("SHA-256 always contains eight seed bytes"),
    )
}

fn median(samples: &[f64]) -> f64 {
    let mut values = samples.to_vec();
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn relative_range(samples: &[f64]) -> f64 {
    if samples.is_empty() {
        return f64::INFINITY;
    }
    let center = median(samples);
    let minimum = samples.iter().copied().min_by(f64::total_cmp).unwrap();
    let maximum = samples.iter().copied().max_by(f64::total_cmp).unwrap();
    if center > 0.0 {
        (maximum - minimum) / center
    } else if maximum == minimum {
        0.0
    } else {
        f64::INFINITY
    }
}

fn relative_difference(left: f64, right: f64) -> f64 {
    (left - right).abs() / left.abs().max(right.abs()).max(f64::EPSILON)
}

fn dependency_cycle(measurements: &[MeasurementEvidence]) -> bool {
    let edges = measurements
        .iter()
        .map(|measurement| {
            let dependencies = match measurement {
                MeasurementEvidence::Scalar(value) => value.derived_from.clone(),
                MeasurementEvidence::Comparison(value) => vec![
                    value.left_measurement_id.clone(),
                    value.right_measurement_id.clone(),
                ],
                _ => Vec::new(),
            };
            (measurement.id().to_owned(), dependencies)
        })
        .collect::<BTreeMap<_, _>>();
    let mut visited = BTreeSet::new();
    let mut active = BTreeSet::new();
    edges
        .keys()
        .any(|id| visit_dependency(id, &edges, &mut active, &mut visited))
}

fn visit_dependency(
    id: &str,
    edges: &BTreeMap<String, Vec<String>>,
    active: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) -> bool {
    if active.contains(id) {
        return true;
    }
    if !visited.insert(id.to_owned()) {
        return false;
    }
    active.insert(id.to_owned());
    let cyclic = edges
        .get(id)
        .into_iter()
        .flatten()
        .any(|dependency| visit_dependency(dependency, edges, active, visited));
    active.remove(id);
    cyclic
}
