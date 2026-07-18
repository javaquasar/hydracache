use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

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
}

/// Non-knee numeric evidence, including allocation or scaling efficiency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScalarEvidence {
    pub id: String,
    pub workload: WorkloadIdentity,
    pub points: Vec<ScalarPoint>,
    pub derived_from: Vec<String>,
}

/// Deterministic trace replay proof that preserves input ordering and identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceReplayEvidence {
    pub id: String,
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
    pub observed_runner: RunnerFingerprint,
    pub profile_validation: ProfileValidation,
    pub source: SourceIdentity,
    pub build: BuildIdentity,
    pub measurements: Vec<MeasurementEvidence>,
    pub stable: bool,
    pub stability_reasons: Vec<String>,
}

impl PerfReport {
    /// Build a report, deriving runner eligibility and every semantic stability verdict.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        report_id: impl Into<String>,
        scenario_id: impl Into<String>,
        scenario_digest: impl Into<String>,
        workload_digest: impl Into<String>,
        state_digest: impl Into<String>,
        seed: u64,
        run_mode: EvidenceRunMode,
        surface: SurfaceIdentity,
        runner_contract: PerformanceProfile,
        observed_runner: RunnerFingerprint,
        source: SourceIdentity,
        build: BuildIdentity,
        measurements: Vec<MeasurementEvidence>,
        mut stability_reasons: Vec<String>,
    ) -> Self {
        let profile_validation = runner_contract.validate(&observed_runner);
        let mut report = Self {
            schema_version: PERF_SCHEMA_VERSION,
            release: PERF_RELEASE.to_owned(),
            report_id: report_id.into(),
            scenario_id: scenario_id.into(),
            scenario_digest: scenario_digest.into(),
            workload_digest: workload_digest.into(),
            state_digest: state_digest.into(),
            seed,
            run_mode,
            surface,
            runner_profile: runner_contract.name.clone(),
            runner_contract,
            observed_runner,
            profile_validation,
            source,
            build,
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

    /// Serialize stable pretty JSON for a receipt-hashed artifact.
    pub fn to_pretty_json(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec_pretty(self)
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
            || self.profile_validation != derived_validation
            || !derived_validation.eligible
            || derived_validation.eligible != derived_validation.reasons.is_empty()
            || !self.runner_contract.require_dedicated
            || self.runner_contract.allowed_fingerprints.is_empty()
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
            problems.extend(measurement.validation_problems(&self.state_digest));
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
                }
                _ => {}
            }
        }
        problems
    }
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

    fn validation_problems(&self, state_digest: &str) -> Vec<String> {
        let mut problems = Vec::new();
        if self.id().is_empty() {
            problems.push("measurement id must be non-empty".to_owned());
        }
        match self {
            Self::LoadCurve(value) => {
                problems.extend(workload_problems(&value.workload));
                if value.claim == LoadClaim::CapacityKnee {
                    match (&value.criteria, &value.knee) {
                        (Some(criteria), Some(knee)) => {
                            problems.extend(criteria.knee_validation_problems(knee));
                            if knee.sustainable_rate_per_second.is_none() {
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
                                    repeat.state_digest != state_digest
                                        || Some(&repeat.reset_state_digest) != reset_digest
                                        || Some((
                                            repeat.phase.reset_operations,
                                            repeat.phase.preload_operations,
                                            repeat.phase.warmup_operations,
                                            repeat.phase.steady_operations,
                                        )) != phase_shape
                                }) {
                                    problems.push(format!(
                                        "measurement {} repeat state differs from report state",
                                        value.id
                                    ));
                                }
                            }
                        }
                        _ => problems.push(format!(
                            "capacity measurement {} requires criteria and knee",
                            value.id
                        )),
                    }
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
                problems.extend(workload_problems(&value.workload));
                if value.points.is_empty()
                    || value.points.iter().any(|point| {
                        point.sample_count == 0
                            || !point.quantity.value.is_finite()
                            || point.quantity.unit.is_empty()
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
                if value.catalog_id.is_empty()
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
                if value.left_measurement_id.is_empty()
                    || value.right_measurement_id.is_empty()
                    || value.left_measurement_id == value.right_measurement_id
                    || !value.ratio.is_finite()
                    || value.ratio < 0.0
                    || value.unit.is_empty()
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
