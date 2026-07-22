//! Release-0.67 W2 characterization of the in-process Axum client router.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use hydracache_cache_sim::{
    GeneratedKeySchedule, KeyDistribution, KeyScheduleSpec, KEY_SCHEDULE_GENERATOR_VERSION,
};
use hydracache_client_protocol::{
    ClientFrame, ClientRequest, ClientRequestEnvelope, ClientWireMessage, Namespace, StructuredKey,
    PROTOCOL_VERSION,
};
use hydracache_client_transport_axum::ClientSurfaceLimits;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::report::{
    derived_identity_digest, BuildIdentity, DimensionValue, EvidenceRunMode,
    KeyDistributionIdentity, LoadClaim, LoadCurveEvidence, MeasurementEvidence, PerfReport,
    Quantity, ScalarEvidence, ScalarPoint, SourceIdentity, SurfaceIdentity, WeightedOperation,
    WeightedPayload, WorkloadIdentity,
};
use crate::runner::run_scenario;
use crate::scenario::Scenario;
use crate::target::{Target, TargetError, TargetOutcome};
use crate::targets::client_surface::{
    ClientSurfaceOperation, ClientSurfaceOperationMix, ClientSurfaceTarget,
    ClientSurfaceTargetConfig,
};
use crate::tiers::resp_reference::ValidatedRespReferenceContext;
use crate::{PerformanceProfile, RunnerFingerprint};

const SMOKE_REPEATS: usize = 3;
const SMOKE_SPREAD_LIMIT: f64 = 1_000.0;
const SMOKE_KEY_COUNT: u64 = 32;
const SMOKE_PRELOAD_ENTRIES: u64 = 16;
const SMOKE_STEADY_OPERATIONS: u64 = 100;
const CLIENT_REFERENCE_CAPABILITY_VERSION: u32 = 1;
const CLIENT_REFERENCE_INSTANCE_VERSION: u32 = 1;
static CLIENT_REFERENCE_INSTANCE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub const CLIENT_W6_CAPACITY_MEASUREMENTS: [&str; 3] = [
    "client_surface_in_process_knee_at_slo_workload_a",
    "client_surface_in_process_knee_at_slo_workload_b",
    "client_surface_in_process_knee_at_slo_workload_c",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClientRunShape {
    Smoke,
    Reference,
}

impl ClientRunShape {
    fn repeats(self, committed: usize) -> usize {
        match self {
            Self::Smoke => SMOKE_REPEATS,
            Self::Reference => committed,
        }
    }

    fn spread_limit(self, committed: f64) -> f64 {
        match self {
            Self::Smoke => SMOKE_SPREAD_LIMIT,
            Self::Reference => committed,
        }
    }

    fn effective_digest(
        self,
        source_digest: &str,
        client: &ClientSurfaceInputs,
        scenario: &Scenario,
    ) -> String {
        effective_digest(source_digest, client, scenario, self)
    }

    fn custom_digest(self, source: &[u8], effective: &serde_json::Value) -> String {
        custom_effective_digest(source, effective, self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientSurfaceReferenceCapability {
    pub schema_version: u32,
    pub surface_kind: String,
    pub execution_mode: String,
    pub state_scope: String,
    pub network_boundary: String,
    pub capacity_measurements: Vec<String>,
    pub capacity_scenario_sources: Vec<(String, String)>,
    pub source_commit: String,
    pub cargo_lock_sha256: String,
    pub prebuild_contract_sha256: String,
    pub prebuild_manifest_sha256: String,
    pub loadgen_binary_sha256: String,
}

impl ClientSurfaceReferenceCapability {
    pub fn validate(&self) -> Result<(), ClientSurfaceTierError> {
        let expected_measurements = CLIENT_W6_CAPACITY_MEASUREMENTS
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let expected_sources = [
            (
                CLIENT_W6_CAPACITY_MEASUREMENTS[0].to_owned(),
                digest_bytes(WORKLOAD_A_SCENARIO),
            ),
            (
                CLIENT_W6_CAPACITY_MEASUREMENTS[1].to_owned(),
                digest_bytes(WORKLOAD_B_SCENARIO),
            ),
            (
                CLIENT_W6_CAPACITY_MEASUREMENTS[2].to_owned(),
                digest_bytes(WORKLOAD_C_SCENARIO),
            ),
        ]
        .into_iter()
        .collect::<Vec<_>>();
        if self.schema_version != CLIENT_REFERENCE_CAPABILITY_VERSION
            || self.surface_kind != "client-surface"
            || self.execution_mode != "in-process-axum-router"
            || self.state_scope != "process-local"
            || self.network_boundary != "none"
            || self.capacity_measurements != expected_measurements
            || self.capacity_scenario_sources != expected_sources
            || !is_git_commit(&self.source_commit)
            || !is_sha256(&self.cargo_lock_sha256)
            || !is_sha256(&self.prebuild_contract_sha256)
            || !is_sha256(&self.prebuild_manifest_sha256)
            || !is_sha256(&self.loadgen_binary_sha256)
        {
            return Err(ClientSurfaceTierError::Report(
                "W2 stable in-process capability is incomplete or crosses its surface boundary"
                    .to_owned(),
            ));
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String, ClientSurfaceTierError> {
        self.validate()?;
        canonical_digest(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientSurfaceReferenceInstanceReceipt {
    pub schema_version: u32,
    pub instance_sequence: u64,
    pub owning_pid: u32,
    pub created_unix_nanos: u64,
    pub direct_prebuilt_exec: bool,
    pub loadgen_binary_path: String,
    pub loadgen_binary_sha256: String,
    pub stable_capability_sha256: String,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientSurfaceReferenceEvidenceBinding {
    pub capability: ClientSurfaceReferenceCapability,
    pub instance: ClientSurfaceReferenceInstanceReceipt,
    pub capacity_measurements: Vec<(String, String, String)>,
}

impl ClientSurfaceReferenceInstanceReceipt {
    fn seal(
        context: &ValidatedRespReferenceContext,
        stable_capability_sha256: String,
    ) -> Result<Self, ClientSurfaceTierError> {
        let created_unix_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| ClientSurfaceTierError::Report(error.to_string()))?
            .as_nanos()
            .try_into()
            .map_err(|_| {
                ClientSurfaceTierError::Report(
                    "system time does not fit u64 nanoseconds".to_owned(),
                )
            })?;
        let mut receipt = Self {
            schema_version: CLIENT_REFERENCE_INSTANCE_VERSION,
            instance_sequence: CLIENT_REFERENCE_INSTANCE_SEQUENCE
                .fetch_add(1, Ordering::Relaxed)
                .saturating_add(1),
            owning_pid: std::process::id(),
            created_unix_nanos,
            direct_prebuilt_exec: true,
            loadgen_binary_path: context.loadgen.canonical_path.display().to_string(),
            loadgen_binary_sha256: context.loadgen.sha256.clone(),
            stable_capability_sha256,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt.computed_sha256()?;
        receipt.validate(context)?;
        Ok(receipt)
    }

    pub fn computed_sha256(&self) -> Result<String, ClientSurfaceTierError> {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        canonical_digest(&payload)
    }

    pub fn validate(
        &self,
        context: &ValidatedRespReferenceContext,
    ) -> Result<(), ClientSurfaceTierError> {
        self.validate_seal()?;
        if self.owning_pid != std::process::id()
            || self.loadgen_binary_path != context.loadgen.canonical_path.display().to_string()
            || self.loadgen_binary_sha256 != context.loadgen.sha256
        {
            return Err(ClientSurfaceTierError::Report(
                "W2 in-process instance receipt is not owned by the running receipt-bound loadgen"
                    .to_owned(),
            ));
        }
        Ok(())
    }

    pub fn validate_seal(&self) -> Result<(), ClientSurfaceTierError> {
        if self.schema_version != CLIENT_REFERENCE_INSTANCE_VERSION
            || self.instance_sequence == 0
            || self.owning_pid == 0
            || self.created_unix_nanos == 0
            || !self.direct_prebuilt_exec
            || self.loadgen_binary_path.trim().is_empty()
            || !is_sha256(&self.loadgen_binary_sha256)
            || !is_sha256(&self.stable_capability_sha256)
            || self.receipt_sha256 != self.computed_sha256()?
        {
            return Err(ClientSurfaceTierError::Report(
                "W2 in-process instance receipt is unsealed".to_owned(),
            ));
        }
        Ok(())
    }
}

const WORKLOAD_A_SCENARIO: &[u8] =
    include_bytes!("../../../../docs/testing/perf-scenarios/0.67/client-surface-a-v1.toml");
const WORKLOAD_B_SCENARIO: &[u8] =
    include_bytes!("../../../../docs/testing/perf-scenarios/0.67/client-surface-b-v1.toml");
const WORKLOAD_C_SCENARIO: &[u8] =
    include_bytes!("../../../../docs/testing/perf-scenarios/0.67/client-surface-c-v1.toml");
const CONCURRENCY_SCENARIO: &[u8] = include_bytes!(
    "../../../../docs/testing/perf-scenarios/0.67/client-surface-concurrency-v1.toml"
);
const PAYLOAD_SCENARIO: &[u8] =
    include_bytes!("../../../../docs/testing/perf-scenarios/0.67/client-surface-payload-v1.toml");
const PATH_COST_SCENARIO: &[u8] =
    include_bytes!("../../../../docs/testing/perf-scenarios/0.67/client-surface-path-cost-v1.toml");

/// Required W2 measurement ids carried by the client-surface report.
pub const REQUIRED_CLIENT_SURFACE_MEASUREMENTS: [&str; 4] = [
    "client_surface_in_process_knee_at_slo_for_a_b_c",
    "concurrent_inflight_scaling_curve_1_10_100_1000",
    "client_surface_payload_sweep_100b_1kb_64kb_1mb",
    "client_surface_codec_dispatch_and_admission_rejection_cost",
];

#[derive(Debug, thiserror::Error)]
pub enum ClientSurfaceTierError {
    #[error(transparent)]
    Target(#[from] TargetError),
    #[error("client-surface tier runtime failed: {0}")]
    Runtime(String),
    #[error("client-surface tier report failed: {0}")]
    Report(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
struct ClientSurfaceReferenceRunBinding {
    capability: ClientSurfaceReferenceCapability,
    capability_sha256: String,
    instance: ClientSurfaceReferenceInstanceReceipt,
}

impl ClientSurfaceReferenceRunBinding {
    fn establish(context: &ValidatedRespReferenceContext) -> Result<Self, ClientSurfaceTierError> {
        context
            .verify_binaries_unchanged()
            .map_err(|error| ClientSurfaceTierError::Report(error.to_string()))?;
        let capability = client_surface_reference_capability(context);
        let capability_sha256 = capability.digest()?;
        let instance =
            ClientSurfaceReferenceInstanceReceipt::seal(context, capability_sha256.clone())?;
        Ok(Self {
            capability,
            capability_sha256,
            instance,
        })
    }

    fn capacity_dimensions(
        &self,
        measurement_id: &str,
    ) -> Result<BTreeMap<String, DimensionValue>, ClientSurfaceTierError> {
        let scenario_source = self
            .capability
            .capacity_scenario_sources
            .iter()
            .find_map(|(id, digest)| (id == measurement_id).then_some(digest.clone()))
            .ok_or_else(|| {
                ClientSurfaceTierError::Report(format!(
                    "W2 reference capability has no scenario binding for {measurement_id}"
                ))
            })?;
        Ok(BTreeMap::from([
            (
                "reference_instance_schema_version".to_owned(),
                DimensionValue::U64(self.instance.schema_version as u64),
            ),
            (
                "surface_capability_sha256".to_owned(),
                DimensionValue::Text(self.capability_sha256.clone()),
            ),
            (
                "reference_instance_receipt_sha256".to_owned(),
                DimensionValue::Text(self.instance.receipt_sha256.clone()),
            ),
            (
                "reference_instance_sequence".to_owned(),
                DimensionValue::U64(self.instance.instance_sequence),
            ),
            (
                "reference_owning_pid".to_owned(),
                DimensionValue::U64(self.instance.owning_pid as u64),
            ),
            (
                "reference_instance_created_unix_nanos".to_owned(),
                DimensionValue::U64(self.instance.created_unix_nanos),
            ),
            (
                "direct_prebuilt_exec".to_owned(),
                DimensionValue::Bool(self.instance.direct_prebuilt_exec),
            ),
            (
                "loadgen_binary_path".to_owned(),
                DimensionValue::Text(self.instance.loadgen_binary_path.clone()),
            ),
            (
                "loadgen_binary_sha256".to_owned(),
                DimensionValue::Text(self.instance.loadgen_binary_sha256.clone()),
            ),
            (
                "capacity_scenario_source_sha256".to_owned(),
                DimensionValue::Text(scenario_source),
            ),
            (
                "w6_capacity_eligible".to_owned(),
                DimensionValue::Bool(true),
            ),
        ]))
    }
}

fn client_surface_reference_capability(
    context: &ValidatedRespReferenceContext,
) -> ClientSurfaceReferenceCapability {
    ClientSurfaceReferenceCapability {
        schema_version: CLIENT_REFERENCE_CAPABILITY_VERSION,
        surface_kind: "client-surface".to_owned(),
        execution_mode: "in-process-axum-router".to_owned(),
        state_scope: "process-local".to_owned(),
        network_boundary: "none".to_owned(),
        capacity_measurements: CLIENT_W6_CAPACITY_MEASUREMENTS
            .into_iter()
            .map(str::to_owned)
            .collect(),
        capacity_scenario_sources: [
            (
                CLIENT_W6_CAPACITY_MEASUREMENTS[0].to_owned(),
                digest_bytes(WORKLOAD_A_SCENARIO),
            ),
            (
                CLIENT_W6_CAPACITY_MEASUREMENTS[1].to_owned(),
                digest_bytes(WORKLOAD_B_SCENARIO),
            ),
            (
                CLIENT_W6_CAPACITY_MEASUREMENTS[2].to_owned(),
                digest_bytes(WORKLOAD_C_SCENARIO),
            ),
        ]
        .into_iter()
        .collect(),
        source_commit: context.source.git_commit.clone(),
        cargo_lock_sha256: context.source.cargo_lock_sha256.clone(),
        prebuild_contract_sha256: context.build.prebuild_contract_digest.clone(),
        prebuild_manifest_sha256: context.build.prebuild_manifest_sha256.clone(),
        loadgen_binary_sha256: context.loadgen.sha256.clone(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationInput {
    operation: String,
    weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClientSurfaceInputs {
    workload: String,
    key_count: u64,
    payload_bytes: u64,
    batch_size: usize,
    max_frame_bytes: usize,
    operation_mix: Vec<OperationInput>,
}

#[derive(Debug, Clone)]
struct BoundClientScenario {
    scenario: Scenario,
    client: ClientSurfaceInputs,
    source_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConcurrencyInput {
    schema_version: u32,
    id: String,
    seed: u64,
    operations: u64,
    repeats: usize,
    preload_entries: u64,
    key_count: u64,
    payload_bytes: u64,
    batch_size: usize,
    inflight: Vec<usize>,
    metric: String,
    robust_spread_tolerance: f64,
    execution_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PayloadInput {
    schema_version: u32,
    id: String,
    seed: u64,
    operations_per_repeat: u64,
    repeats: usize,
    key_count: u64,
    payload_bytes: Vec<usize>,
    max_frame_bytes: usize,
    beyond_cap_value_bytes: usize,
    metric: String,
    require_http_413_beyond_cap: bool,
    robust_spread_tolerance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PathCostInput {
    schema_version: u32,
    id: String,
    seed: u64,
    operations_per_repeat: u64,
    repeats: usize,
    key_count: u64,
    payload_bytes: usize,
    max_frame_bytes: usize,
    beyond_cap_value_bytes: usize,
    paths: Vec<String>,
    metric: String,
    require_rejection_before_dispatch: bool,
    robust_spread_tolerance: f64,
}

/// Execute every W2 smoke measurement against the real in-process router.
pub async fn client_surface_smoke_measurements(
) -> Result<Vec<MeasurementEvidence>, ClientSurfaceTierError> {
    client_surface_measurements(ClientRunShape::Smoke, None).await
}

async fn client_surface_measurements(
    shape: ClientRunShape,
    reference: Option<&ClientSurfaceReferenceRunBinding>,
) -> Result<Vec<MeasurementEvidence>, ClientSurfaceTierError> {
    if (shape == ClientRunShape::Reference) != reference.is_some() {
        return Err(ClientSurfaceTierError::Report(
            "W2 run shape and reference capability do not match".to_owned(),
        ));
    }
    let mut measurements = client_surface_knee_measurements(shape, reference).await?;
    measurements.push(client_surface_concurrency_measurement(shape).await?);
    measurements.push(client_surface_payload_measurement(shape).await?);
    measurements.push(client_surface_path_cost_measurement(shape).await?);
    Ok(measurements)
}

/// Construct all A/B/C raw knees plus the required aggregate result.
pub async fn client_surface_knee_smoke_measurements(
) -> Result<Vec<MeasurementEvidence>, ClientSurfaceTierError> {
    client_surface_knee_measurements(ClientRunShape::Smoke, None).await
}

async fn client_surface_knee_measurements(
    shape: ClientRunShape,
    reference: Option<&ClientSurfaceReferenceRunBinding>,
) -> Result<Vec<MeasurementEvidence>, ClientSurfaceTierError> {
    let mut curves = Vec::new();
    let mut aggregate_points = Vec::new();
    let mut dependencies = Vec::new();
    let mut aggregate_scenario_inputs = Vec::new();
    let mut aggregate_workload_inputs = Vec::new();
    for source in [
        WORKLOAD_A_SCENARIO,
        WORKLOAD_B_SCENARIO,
        WORKLOAD_C_SCENARIO,
    ] {
        let binding = parse_client_scenario(source)?;
        let scenario = match shape {
            ClientRunShape::Smoke => smoke_scenario(&binding)?,
            ClientRunShape::Reference => binding.scenario.clone(),
        };
        let key_count = match shape {
            ClientRunShape::Smoke => SMOKE_KEY_COUNT,
            ClientRunShape::Reference => binding.client.key_count,
        };
        let schedule_operations = match shape {
            ClientRunShape::Smoke => SMOKE_STEADY_OPERATIONS,
            ClientRunShape::Reference => scenario
                .preload_operations
                .max(scenario.warmup_operations)
                .max(scenario.steady_operations),
        };
        let schedule = KeyScheduleSpec::uniform(scenario.seed, key_count, schedule_operations)
            .generate()
            .map_err(ClientSurfaceTierError::Runtime)?;
        let target = Arc::new(ClientSurfaceTarget::new(target_config(
            &binding,
            &scenario,
            shape,
            Arc::new(schedule.keys.clone()),
            Duration::ZERO,
        )?)?);
        let criteria = scenario.sustainability_criteria();
        let knee = run_scenario(target, &scenario).await?;
        let sustainable_rate = knee.sustainable_rate_per_second.ok_or_else(|| {
            ClientSurfaceTierError::Runtime(format!(
                "{} {:?} knee had no sustainable point",
                binding.client.workload, shape
            ))
        })?;
        let point = knee
            .evaluated
            .iter()
            .find(|point| point.sample.offered_rate_per_second == sustainable_rate)
            .ok_or_else(|| {
                ClientSurfaceTierError::Runtime("selected knee point disappeared".into())
            })?;
        aggregate_points.push(scalar_point(
            BTreeMap::from([(
                "workload".to_owned(),
                DimensionValue::Text(binding.client.workload.clone()),
            )]),
            "operations_per_second_at_slo",
            point
                .repeats
                .iter()
                .map(|repeat| repeat.steady.achieved_rate_per_second)
                .collect(),
        ));
        let id = format!(
            "client_surface_in_process_knee_at_slo_workload_{}",
            binding.client.workload.to_ascii_lowercase()
        );
        dependencies.push(id.clone());
        let workload = workload_identity(&schedule, &binding.client);
        let scenario_digest =
            shape.effective_digest(&binding.source_digest, &binding.client, &scenario);
        aggregate_scenario_inputs.push((id.clone(), scenario_digest.clone()));
        aggregate_workload_inputs.push((id.clone(), workload.digest.clone()));
        let mut dimensions = BTreeMap::from([
            (
                "workload".to_owned(),
                DimensionValue::Text(binding.client.workload.clone()),
            ),
            (
                "execution_mode".to_owned(),
                DimensionValue::Text("in-process-axum-router".to_owned()),
            ),
            (
                "network_boundary".to_owned(),
                DimensionValue::Text("none".to_owned()),
            ),
            (
                "preload_operations".to_owned(),
                DimensionValue::U64(scenario.preload_operations),
            ),
            (
                "warmup_operations".to_owned(),
                DimensionValue::U64(scenario.warmup_operations),
            ),
            (
                "steady_operations".to_owned(),
                DimensionValue::U64(scenario.steady_operations),
            ),
            (
                "repeats".to_owned(),
                DimensionValue::U64(scenario.repeats as u64),
            ),
            ("key_count".to_owned(), DimensionValue::U64(key_count)),
        ]);
        if let Some(reference) = reference {
            dimensions.extend(reference.capacity_dimensions(&id)?);
        }
        curves.push(MeasurementEvidence::LoadCurve(LoadCurveEvidence {
            id,
            scenario_digest,
            dimensions,
            workload,
            criteria: Some(criteria),
            knee: Some(knee),
            claim: LoadClaim::CapacityKnee,
        }));
    }
    let aggregate_workload = WorkloadIdentity {
        generator: "hydracache-client-surface-abc-matrix".to_owned(),
        generator_version: "1".to_owned(),
        seed: None,
        key_distribution: Some(KeyDistributionIdentity {
            kind: "uniform".to_owned(),
            theta: None,
        }),
        key_count: Some(match shape {
            ClientRunShape::Smoke => SMOKE_KEY_COUNT,
            ClientRunShape::Reference => 10_000,
        }),
        operation_mix: vec![
            WeightedOperation {
                operation: "workload_a".to_owned(),
                weight: 1.0,
            },
            WeightedOperation {
                operation: "workload_b".to_owned(),
                weight: 1.0,
            },
            WeightedOperation {
                operation: "workload_c".to_owned(),
                weight: 1.0,
            },
        ],
        payload_mix: vec![WeightedPayload {
            bytes: 1_000,
            weight: 1.0,
        }],
        digest: derived_identity_digest(&aggregate_workload_inputs),
    };
    curves.push(MeasurementEvidence::Scalar(ScalarEvidence {
        id: "client_surface_in_process_knee_at_slo_for_a_b_c".to_owned(),
        scenario_digest: derived_identity_digest(&aggregate_scenario_inputs),
        workload: aggregate_workload,
        points: aggregate_points,
        derived_from: dependencies,
        max_robust_spread_ratio: shape.spread_limit(
            parse_client_scenario(WORKLOAD_A_SCENARIO)?
                .scenario
                .robust_spread_tolerance,
        ),
    }));
    Ok(curves)
}

/// Measure in-flight request scaling, never connection scaling.
pub async fn client_surface_concurrency_smoke_measurement(
) -> Result<MeasurementEvidence, ClientSurfaceTierError> {
    client_surface_concurrency_measurement(ClientRunShape::Smoke).await
}

async fn client_surface_concurrency_measurement(
    shape: ClientRunShape,
) -> Result<MeasurementEvidence, ClientSurfaceTierError> {
    let input: ConcurrencyInput = parse_toml(CONCURRENCY_SCENARIO)?;
    validate_concurrency_input(&input)?;
    let operations = match shape {
        ClientRunShape::Smoke => 1_000,
        ClientRunShape::Reference => input.operations,
    };
    let repeats = shape.repeats(input.repeats);
    let schedule = KeyScheduleSpec::uniform(input.seed, input.key_count, operations)
        .generate()
        .map_err(ClientSurfaceTierError::Runtime)?;
    let mut points = Vec::new();
    for inflight in &input.inflight {
        let mut samples = Vec::with_capacity(repeats);
        let mut observed_high_water = Vec::with_capacity(repeats);
        for _ in 0..repeats {
            let target = Arc::new(ClientSurfaceTarget::new(simple_target_config(
                input.max_frame_bytes_or_default(),
                input.preload_entries,
                input.key_count,
                input.payload_bytes as usize,
                input.batch_size,
                ClientSurfaceOperationMix::WORKLOAD_C,
                Arc::new(schedule.keys.clone()),
                Duration::ZERO,
            )?)?);
            target.reset().await?;
            target.preload().await?;
            let (sample, observed) = concurrent_sample(target, *inflight, operations).await?;
            if observed != *inflight as u64 {
                return Err(ClientSurfaceTierError::Runtime(format!(
                    "declared concurrent in-flight {} but observed high-water {observed}",
                    inflight
                )));
            }
            samples.push(sample);
            observed_high_water.push(observed);
        }
        let observed = observed_high_water
            .first()
            .copied()
            .ok_or_else(|| ClientSurfaceTierError::Runtime("missing in-flight proof".into()))?;
        if observed_high_water.iter().any(|value| *value != observed) {
            return Err(ClientSurfaceTierError::Runtime(
                "in-flight high-water changed across repeats".into(),
            ));
        }
        points.push(scalar_point(
            BTreeMap::from([
                (
                    "concurrent_inflight".to_owned(),
                    DimensionValue::U64(*inflight as u64),
                ),
                (
                    "observed_inflight_high_water".to_owned(),
                    DimensionValue::U64(observed),
                ),
                (
                    "measurement_boundary".to_owned(),
                    DimensionValue::Text("framed-request-lifetime-at-router-oneshot".to_owned()),
                ),
                ("not_connections".to_owned(), DimensionValue::Bool(true)),
            ]),
            &input.metric,
            samples,
        ));
    }
    Ok(MeasurementEvidence::Scalar(ScalarEvidence {
        id: "concurrent_inflight_scaling_curve_1_10_100_1000".to_owned(),
        scenario_digest: shape.custom_digest(
            CONCURRENCY_SCENARIO,
            &serde_json::json!({"operations": operations, "repeats": repeats}),
        ),
        workload: workload_identity_from_parts(
            &schedule,
            vec![WeightedOperation {
                operation: "get".to_owned(),
                weight: 1.0,
            }],
            vec![WeightedPayload {
                bytes: input.payload_bytes,
                weight: 1.0,
            }],
        ),
        points,
        derived_from: Vec::new(),
        max_robust_spread_ratio: shape.spread_limit(input.robust_spread_tolerance),
    }))
}

/// Measure accepted payloads and prove that one value beyond the frame cap is
/// rejected with HTTP 413 before dispatch or mutation.
pub async fn client_surface_payload_smoke_measurement(
) -> Result<MeasurementEvidence, ClientSurfaceTierError> {
    client_surface_payload_measurement(ClientRunShape::Smoke).await
}

async fn client_surface_payload_measurement(
    shape: ClientRunShape,
) -> Result<MeasurementEvidence, ClientSurfaceTierError> {
    let input: PayloadInput = parse_toml(PAYLOAD_SCENARIO)?;
    validate_payload_input(&input)?;
    let operations = match shape {
        ClientRunShape::Smoke => input.operations_per_repeat.min(8),
        ClientRunShape::Reference => input.operations_per_repeat,
    };
    let repeats = shape.repeats(input.repeats);
    let schedule = KeyScheduleSpec::uniform(input.seed, input.key_count, operations)
        .generate()
        .map_err(ClientSurfaceTierError::Runtime)?;
    let mut points = Vec::new();
    for payload_bytes in &input.payload_bytes {
        let mut samples = Vec::with_capacity(repeats);
        for _ in 0..repeats {
            let target = ClientSurfaceTarget::new(simple_target_config(
                input.max_frame_bytes,
                0,
                input.key_count,
                100,
                1,
                ClientSurfaceOperationMix::WORKLOAD_A,
                Arc::new(schedule.keys.clone()),
                Duration::ZERO,
            )?)?;
            target.reset().await?;
            let started = Instant::now();
            for sequence in 0..operations {
                let dispatch = target
                    .dispatch_payload_put(*payload_bytes, sequence)
                    .await?;
                if dispatch.outcome != TargetOutcome::Success {
                    return Err(ClientSurfaceTierError::Runtime(format!(
                        "accepted payload {payload_bytes} returned {dispatch:?}"
                    )));
                }
            }
            samples.push(throughput(operations, started.elapsed()));
        }
        points.push(scalar_point(
            BTreeMap::from([
                (
                    "payload_bytes".to_owned(),
                    DimensionValue::U64(*payload_bytes as u64),
                ),
                ("expected_rejection".to_owned(), DimensionValue::Bool(false)),
            ]),
            &input.metric,
            samples,
        ));
    }
    let mut rejection_samples = Vec::with_capacity(repeats);
    for _ in 0..repeats {
        let target = ClientSurfaceTarget::new(simple_target_config(
            input.max_frame_bytes,
            0,
            input.key_count,
            100,
            1,
            ClientSurfaceOperationMix::WORKLOAD_A,
            Arc::new(schedule.keys.clone()),
            Duration::ZERO,
        )?)?;
        target.reset().await?;
        let before = target.snapshot().await;
        let started = Instant::now();
        let dispatch = target
            .dispatch_payload_put(input.beyond_cap_value_bytes, 0)
            .await?;
        let elapsed = started.elapsed();
        let after = target.snapshot().await;
        if dispatch.status != axum::http::StatusCode::PAYLOAD_TOO_LARGE
            || dispatch.outcome != TargetOutcome::Rejected
            || after.dispatch_attempts != before.dispatch_attempts
            || after.state_mutations != before.state_mutations
            || after.rejected_oversized != before.rejected_oversized.saturating_add(1)
        {
            return Err(ClientSurfaceTierError::Runtime(format!(
                "beyond-cap request did not fail before dispatch and mutation: dispatch={dispatch:?}, before={before:?}, after={after:?}"
            )));
        }
        rejection_samples.push(throughput(1, elapsed));
    }
    let rejection_rate = rejection_samples
        .iter()
        .copied()
        .min_by(f64::total_cmp)
        .unwrap_or_default();
    for point in &mut points {
        point.dimensions.insert(
            "frame_cap_bytes".to_owned(),
            DimensionValue::U64(input.max_frame_bytes as u64),
        );
        point.dimensions.insert(
            "beyond_cap_value_bytes".to_owned(),
            DimensionValue::U64(input.beyond_cap_value_bytes as u64),
        );
        point.dimensions.insert(
            "beyond_cap_http_status".to_owned(),
            DimensionValue::U64(413),
        );
        point.dimensions.insert(
            "beyond_cap_rejected_before_dispatch".to_owned(),
            DimensionValue::Bool(true),
        );
        point.dimensions.insert(
            "beyond_cap_min_rejections_per_second".to_owned(),
            DimensionValue::F64(rejection_rate),
        );
    }
    Ok(MeasurementEvidence::Scalar(ScalarEvidence {
        id: "client_surface_payload_sweep_100b_1kb_64kb_1mb".to_owned(),
        scenario_digest: shape.custom_digest(
            PAYLOAD_SCENARIO,
            &serde_json::json!({"operations_per_repeat": operations, "repeats": repeats}),
        ),
        workload: workload_identity_from_parts(
            &schedule,
            vec![WeightedOperation {
                operation: "put".to_owned(),
                weight: 1.0,
            }],
            input
                .payload_bytes
                .iter()
                .map(|bytes| WeightedPayload {
                    bytes: *bytes as u64,
                    weight: 1.0,
                })
                .collect(),
        ),
        points,
        derived_from: Vec::new(),
        max_robust_spread_ratio: shape.spread_limit(input.robust_spread_tolerance),
    }))
}

/// Price codec-only, real router dispatch, and pre-dispatch admission paths.
pub async fn client_surface_path_cost_smoke_measurement(
) -> Result<MeasurementEvidence, ClientSurfaceTierError> {
    client_surface_path_cost_measurement(ClientRunShape::Smoke).await
}

async fn client_surface_path_cost_measurement(
    shape: ClientRunShape,
) -> Result<MeasurementEvidence, ClientSurfaceTierError> {
    let input: PathCostInput = parse_toml(PATH_COST_SCENARIO)?;
    validate_path_cost_input(&input)?;
    let operations = match shape {
        ClientRunShape::Smoke => input.operations_per_repeat.min(8),
        ClientRunShape::Reference => input.operations_per_repeat,
    };
    let repeats = shape.repeats(input.repeats);
    let schedule = KeyScheduleSpec::uniform(input.seed, input.key_count, operations)
        .generate()
        .map_err(ClientSurfaceTierError::Runtime)?;
    let mut codec_samples = Vec::new();
    let mut dispatch_samples = Vec::new();
    let mut admission_samples = Vec::new();
    for _ in 0..repeats {
        codec_samples.push(codec_cost_sample(operations, input.payload_bytes)?);
        let target = ClientSurfaceTarget::new(simple_target_config(
            input.max_frame_bytes,
            input.key_count,
            input.key_count,
            input.payload_bytes,
            1,
            ClientSurfaceOperationMix::WORKLOAD_C,
            Arc::new(schedule.keys.clone()),
            Duration::ZERO,
        )?)?;
        target.reset().await?;
        target.preload().await?;
        let started = Instant::now();
        for sequence in 0..operations {
            ensure_success(
                target
                    .execute_operation(ClientSurfaceOperation::Get, sequence)
                    .await,
                "router dispatch",
            )?;
        }
        dispatch_samples.push(nanos_per_operation(operations, started.elapsed()));

        let before = target.snapshot().await;
        let started = Instant::now();
        for sequence in 0..operations {
            let dispatch = target
                .dispatch_payload_put(input.beyond_cap_value_bytes, sequence)
                .await?;
            if dispatch.outcome != TargetOutcome::Rejected
                || dispatch.status != axum::http::StatusCode::PAYLOAD_TOO_LARGE
            {
                return Err(ClientSurfaceTierError::Runtime(format!(
                    "oversized admission path returned {dispatch:?}"
                )));
            }
        }
        admission_samples.push(nanos_per_operation(operations, started.elapsed()));
        let after = target.snapshot().await;
        if after.dispatch_attempts != before.dispatch_attempts
            || after.state_mutations != before.state_mutations
            || after.rejected_oversized != before.rejected_oversized.saturating_add(operations)
        {
            return Err(ClientSurfaceTierError::Runtime(
                "oversized cost path reached dispatch or mutation".into(),
            ));
        }
    }
    let points = vec![
        scalar_point(
            BTreeMap::from([
                (
                    "path".to_owned(),
                    DimensionValue::Text("codec_encode_decode".to_owned()),
                ),
                (
                    "payload_bytes".to_owned(),
                    DimensionValue::U64(input.payload_bytes as u64),
                ),
            ]),
            &input.metric,
            codec_samples,
        ),
        scalar_point(
            BTreeMap::from([
                (
                    "path".to_owned(),
                    DimensionValue::Text("router_dispatch".to_owned()),
                ),
                (
                    "payload_bytes".to_owned(),
                    DimensionValue::U64(input.payload_bytes as u64),
                ),
            ]),
            &input.metric,
            dispatch_samples,
        ),
        scalar_point(
            BTreeMap::from([
                (
                    "path".to_owned(),
                    DimensionValue::Text("oversized_admission_rejection".to_owned()),
                ),
                (
                    "rejected_before_dispatch".to_owned(),
                    DimensionValue::Bool(true),
                ),
                (
                    "payload_bytes".to_owned(),
                    DimensionValue::U64(input.beyond_cap_value_bytes as u64),
                ),
            ]),
            &input.metric,
            admission_samples,
        ),
    ];
    Ok(MeasurementEvidence::Scalar(ScalarEvidence {
        id: "client_surface_codec_dispatch_and_admission_rejection_cost".to_owned(),
        scenario_digest: shape.custom_digest(
            PATH_COST_SCENARIO,
            &serde_json::json!({"operations_per_repeat": operations, "repeats": repeats}),
        ),
        workload: workload_identity_from_parts(
            &schedule,
            vec![
                WeightedOperation {
                    operation: "codec_encode_decode".to_owned(),
                    weight: 1.0,
                },
                WeightedOperation {
                    operation: "router_dispatch".to_owned(),
                    weight: 1.0,
                },
                WeightedOperation {
                    operation: "oversized_admission_rejection".to_owned(),
                    weight: 1.0,
                },
            ],
            vec![
                WeightedPayload {
                    bytes: input.payload_bytes as u64,
                    weight: 2.0 / 3.0,
                },
                WeightedPayload {
                    bytes: input.beyond_cap_value_bytes as u64,
                    weight: 1.0 / 3.0,
                },
            ],
        ),
        points,
        derived_from: Vec::new(),
        max_robust_spread_ratio: shape.spread_limit(input.robust_spread_tolerance),
    }))
}

/// Canary curve with a loadgen-owned delay immediately before real dispatch.
pub async fn client_surface_dispatch_knee(
    injected_delay: Duration,
) -> Result<crate::KneeResult, ClientSurfaceTierError> {
    let binding = parse_client_scenario(WORKLOAD_C_SCENARIO)?;
    let mut scenario = smoke_scenario(&binding)?;
    scenario.offered_rates_per_second = vec![1_000];
    scenario.p99_slo_us = 5_000;
    scenario.robust_spread_tolerance = SMOKE_SPREAD_LIMIT;
    let schedule = KeyScheduleSpec::uniform(binding.scenario.seed, 16, SMOKE_STEADY_OPERATIONS)
        .generate()
        .map_err(ClientSurfaceTierError::Runtime)?;
    let target = Arc::new(ClientSurfaceTarget::new(target_config(
        &binding,
        &scenario,
        ClientRunShape::Smoke,
        Arc::new(schedule.keys),
        injected_delay,
    )?)?);
    Ok(run_scenario(target, &scenario).await?)
}

/// Build a complete schema-valid but explicitly non-ship W2 report.
pub async fn client_surface_smoke_report(
    profile_name: &str,
) -> Result<PerfReport, ClientSurfaceTierError> {
    if profile_name != "smoke-v1" {
        return Err(ClientSurfaceTierError::Report(format!(
            "profile {profile_name:?} cannot be attached to plumbing-only client-surface smoke evidence"
        )));
    }
    let measurements = client_surface_smoke_measurements().await?;
    let state_digest = measurements
        .iter()
        .find_map(|measurement| match measurement {
            MeasurementEvidence::LoadCurve(curve) => curve
                .knee
                .as_ref()
                .and_then(|knee| knee.evaluated.first())
                .and_then(|point| point.repeats.first())
                .map(|repeat| repeat.state_digest.clone()),
            _ => None,
        })
        .ok_or_else(|| ClientSurfaceTierError::Report("W2 report has no state digest".into()))?;
    let fingerprint = smoke_fingerprint();
    let profile = smoke_profile(profile_name, &fingerprint);
    Ok(PerfReport::new(
        "client-surface-tier-smoke",
        "client-surface-w2-suite-smoke",
        state_digest,
        EvidenceRunMode::Smoke,
        SurfaceIdentity {
            surface_kind: "client-surface".to_owned(),
            execution_mode: "in-process-axum-router".to_owned(),
            state_scope: "process-local".to_owned(),
            network_boundary: "none".to_owned(),
            claim_scope: "plumbing-only".to_owned(),
        },
        profile,
        fingerprint,
        SourceIdentity {
            git_commit: "smoke-unclaimed-working-tree".to_owned(),
            cargo_lock_sha256: digest_bytes(include_bytes!("../../../../Cargo.lock")),
            toolchain: "smoke-current-toolchain".to_owned(),
            build_flags: vec!["smoke-debug".to_owned()],
        },
        BuildIdentity {
            prebuild_contract_digest: "smoke-no-prebuild-contract".to_owned(),
            prebuild_manifest_sha256: "smoke-no-prebuild-manifest".to_owned(),
            binary_sha256: vec![(
                "hydracache-loadgen".to_owned(),
                "smoke-unclaimed-binary".to_owned(),
            )],
        },
        None,
        measurements,
        vec!["short in-process client-surface smoke is not capacity evidence".to_owned()],
    ))
}

/// Execute the exact committed W2 A/B/C, in-flight, payload, and path-cost
/// shapes inside the receipt-bound prebuilt loadgen. This is explicitly an
/// in-process library/client surface and makes no daemon or network claim.
pub async fn client_surface_reference_report(
    context: &ValidatedRespReferenceContext,
) -> Result<PerfReport, ClientSurfaceTierError> {
    let binding = ClientSurfaceReferenceRunBinding::establish(context)?;
    let measurements =
        client_surface_measurements(ClientRunShape::Reference, Some(&binding)).await?;
    context
        .verify_binaries_unchanged()
        .map_err(|error| ClientSurfaceTierError::Report(error.to_string()))?;
    binding.instance.validate(context)?;
    let state_digest = first_state_digest(&measurements)?;
    let report = PerfReport::new(
        "client-surface-tier-reference-v1",
        "client-surface-w2-suite-reference-v1",
        state_digest,
        EvidenceRunMode::ReferenceEvidence,
        SurfaceIdentity {
            surface_kind: "client-surface".to_owned(),
            execution_mode: "in-process-axum-router".to_owned(),
            state_scope: "process-local".to_owned(),
            network_boundary: "none".to_owned(),
            claim_scope: "in-process-client-surface-capacity".to_owned(),
        },
        context.profile.clone(),
        context.runner.clone(),
        context.source.clone(),
        context.build.clone(),
        None,
        measurements,
        Vec::new(),
    );
    let validated = validate_client_surface_reference_report(&report)?;
    if validated.capability != binding.capability || validated.instance != binding.instance {
        return Err(ClientSurfaceTierError::Report(
            "W2 report binding differs from the live in-process capability".to_owned(),
        ));
    }
    Ok(report)
}

/// Strict disk-consumer validation for W2. This recomputes the stable
/// capability and instance seal, and accepts only the complete committed A/B/C
/// plus concurrency/payload/path shape.
pub fn validate_client_surface_reference_report(
    report: &PerfReport,
) -> Result<ClientSurfaceReferenceEvidenceBinding, ClientSurfaceTierError> {
    let problems = report.validation_problems();
    if !problems.is_empty() || report.to_pretty_json().is_err() {
        return Err(ClientSurfaceTierError::Report(format!(
            "W2 reference report failed canonical validation: {problems:?}"
        )));
    }
    let expected_surface = SurfaceIdentity {
        surface_kind: "client-surface".to_owned(),
        execution_mode: "in-process-axum-router".to_owned(),
        state_scope: "process-local".to_owned(),
        network_boundary: "none".to_owned(),
        claim_scope: "in-process-client-surface-capacity".to_owned(),
    };
    if report.report_id != "client-surface-tier-reference-v1"
        || report.scenario_id != "client-surface-w2-suite-reference-v1"
        || report.run_mode != EvidenceRunMode::ReferenceEvidence
        || report.surface != expected_surface
        || report.runner_profile != "reference-v1"
        || !report.stable
        || !report.stability_reasons.is_empty()
        || report.resp_endpoint_capability.is_some()
    {
        return Err(ClientSurfaceTierError::Report(
            "W2 reference report identity, stability, or in-process boundary is incorrect"
                .to_owned(),
        ));
    }
    let loadgen_binary_sha256 = exact_loadgen_digest(report)?;
    let capability = ClientSurfaceReferenceCapability {
        schema_version: CLIENT_REFERENCE_CAPABILITY_VERSION,
        surface_kind: report.surface.surface_kind.clone(),
        execution_mode: report.surface.execution_mode.clone(),
        state_scope: report.surface.state_scope.clone(),
        network_boundary: report.surface.network_boundary.clone(),
        capacity_measurements: CLIENT_W6_CAPACITY_MEASUREMENTS
            .into_iter()
            .map(str::to_owned)
            .collect(),
        capacity_scenario_sources: [
            (
                CLIENT_W6_CAPACITY_MEASUREMENTS[0].to_owned(),
                digest_bytes(WORKLOAD_A_SCENARIO),
            ),
            (
                CLIENT_W6_CAPACITY_MEASUREMENTS[1].to_owned(),
                digest_bytes(WORKLOAD_B_SCENARIO),
            ),
            (
                CLIENT_W6_CAPACITY_MEASUREMENTS[2].to_owned(),
                digest_bytes(WORKLOAD_C_SCENARIO),
            ),
        ]
        .into_iter()
        .collect(),
        source_commit: report.source.git_commit.clone(),
        cargo_lock_sha256: report.source.cargo_lock_sha256.clone(),
        prebuild_contract_sha256: report.build.prebuild_contract_digest.clone(),
        prebuild_manifest_sha256: report.build.prebuild_manifest_sha256.clone(),
        loadgen_binary_sha256,
    };
    let capability_sha256 = capability.digest()?;
    let expected_rates = [1_000_f64, 5_000_f64, 10_000_f64, 25_000_f64];
    let mut capacity_measurements = Vec::new();
    let mut observed_instance = None;
    for (source, id) in [
        (WORKLOAD_A_SCENARIO, CLIENT_W6_CAPACITY_MEASUREMENTS[0]),
        (WORKLOAD_B_SCENARIO, CLIENT_W6_CAPACITY_MEASUREMENTS[1]),
        (WORKLOAD_C_SCENARIO, CLIENT_W6_CAPACITY_MEASUREMENTS[2]),
    ] {
        let curve = report
            .measurements
            .iter()
            .find_map(|measurement| match measurement {
                MeasurementEvidence::LoadCurve(curve) if curve.id == id => Some(curve),
                _ => None,
            })
            .ok_or_else(|| {
                ClientSurfaceTierError::Report(format!("W2 capacity curve {id} is absent"))
            })?;
        let binding = parse_client_scenario(source)?;
        let expected_source_digest = digest_bytes(source);
        let expected_digest = ClientRunShape::Reference.effective_digest(
            &binding.source_digest,
            &binding.client,
            &binding.scenario,
        );
        let schedule = KeyScheduleSpec::uniform(
            binding.scenario.seed,
            binding.client.key_count,
            binding
                .scenario
                .preload_operations
                .max(binding.scenario.warmup_operations)
                .max(binding.scenario.steady_operations),
        )
        .generate()
        .map_err(ClientSurfaceTierError::Runtime)?;
        let expected_workload = workload_identity(&schedule, &binding.client);
        let knee = curve.knee.as_ref().ok_or_else(|| {
            ClientSurfaceTierError::Report(format!("W2 capacity curve {id} has no knee"))
        })?;
        let exact_knee = knee.evaluated.len() == expected_rates.len()
            && knee
                .evaluated
                .iter()
                .zip(expected_rates)
                .all(|(point, rate)| {
                    point.sample.offered_rate_per_second == rate
                        && point.repeats.len() == binding.scenario.repeats as usize
                        && point.repeats.iter().all(|repeat| {
                            repeat.phase.reset_operations == 1
                                && repeat.phase.preload_operations
                                    == binding.scenario.preload_operations
                                && repeat.phase.warmup_operations
                                    == binding.scenario.warmup_operations
                                && repeat.phase.steady_operations
                                    == binding.scenario.steady_operations
                                && repeat.phase.warmup_samples_in_steady_histogram == 0
                        })
                });
        if curve.claim != LoadClaim::CapacityKnee
            || curve.scenario_digest != expected_digest
            || curve.workload != expected_workload
            || curve.criteria.as_ref() != Some(&binding.scenario.sustainability_criteria())
            || !exact_knee
            || knee.sustainable_rate_per_second.is_none()
            || text_dimension(&curve.dimensions, "surface_capability_sha256")
                != Some(capability_sha256.as_str())
            || text_dimension(&curve.dimensions, "capacity_scenario_source_sha256")
                != Some(expected_source_digest.as_str())
            || bool_dimension(&curve.dimensions, "w6_capacity_eligible") != Some(true)
        {
            return Err(ClientSurfaceTierError::Report(format!(
                "W2 curve {id} does not retain its exact committed open-loop shape or stable capability"
            )));
        }
        let instance = instance_from_dimensions(&curve.dimensions, capability_sha256.clone())?;
        instance.validate_seal()?;
        if observed_instance
            .as_ref()
            .is_some_and(|observed| observed != &instance)
        {
            return Err(ClientSurfaceTierError::Report(
                "W2 A/B/C curves do not share one exact in-process instance receipt".to_owned(),
            ));
        }
        observed_instance = Some(instance);
        capacity_measurements.push((
            id.to_owned(),
            curve.scenario_digest.clone(),
            curve.workload.digest.clone(),
        ));
    }
    validate_client_surface_reference_scalar_shapes(report)?;
    Ok(ClientSurfaceReferenceEvidenceBinding {
        capability,
        instance: observed_instance.ok_or_else(|| {
            ClientSurfaceTierError::Report("W2 has no instance receipt".to_owned())
        })?,
        capacity_measurements,
    })
}

/// Stable profile entry point shared by direct and aggregate CLI dispatch.
pub async fn run_client_surface_profile(
    profile: &str,
) -> Result<PerfReport, ClientSurfaceTierError> {
    match profile {
        "smoke-v1" => client_surface_smoke_report(profile).await,
        "reference-v1" => Err(ClientSurfaceTierError::Report(
            "reference-v1 requires the W7 profile and receipt-bound prebuild context; refusing to emit smoke evidence"
                .to_owned(),
        )),
        _ => Err(ClientSurfaceTierError::Report(format!(
            "unknown client-surface performance profile {profile:?}"
        ))),
    }
}

pub async fn run_client_surface_profile_with_context(
    profile: &str,
    context: Option<&ValidatedRespReferenceContext>,
) -> Result<PerfReport, ClientSurfaceTierError> {
    match profile {
        "smoke-v1" if context.is_none() => client_surface_smoke_report(profile).await,
        "reference-v1" => {
            let context = context.ok_or_else(|| {
                ClientSurfaceTierError::Report(
                    "reference-v1 requires a validated W7 reference context".to_owned(),
                )
            })?;
            client_surface_reference_report(context).await
        }
        "smoke-v1" => Err(ClientSurfaceTierError::Report(
            "smoke-v1 must not consume a reference capability".to_owned(),
        )),
        _ => Err(ClientSurfaceTierError::Report(format!(
            "unknown client-surface performance profile {profile:?}"
        ))),
    }
}

/// Convenience writer for integration by the direct and suite CLI forms.
pub async fn write_client_surface_report(
    profile: &str,
    path: &Path,
) -> Result<(), ClientSurfaceTierError> {
    let report = run_client_surface_profile(profile).await?;
    let bytes = report
        .to_pretty_json()
        .map_err(|error| ClientSurfaceTierError::Report(error.to_string()))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)?;
    Ok(())
}

pub async fn write_client_surface_report_with_context(
    profile: &str,
    path: &Path,
    context: Option<&ValidatedRespReferenceContext>,
) -> Result<(), ClientSurfaceTierError> {
    let report = run_client_surface_profile_with_context(profile, context).await?;
    write_report(&report, path)
}

async fn concurrent_sample(
    target: Arc<ClientSurfaceTarget>,
    inflight: usize,
    operations: u64,
) -> Result<(f64, u64), ClientSurfaceTierError> {
    if inflight == 0 || operations < inflight as u64 || !operations.is_multiple_of(inflight as u64)
    {
        return Err(ClientSurfaceTierError::Runtime(
            "concurrency proof requires complete non-zero request waves".into(),
        ));
    }
    target.configure_dispatch_rendezvous(Some(inflight)).await?;
    let next = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));
    let start = Arc::new(tokio::sync::Barrier::new(inflight + 1));
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..inflight {
        let target = Arc::clone(&target);
        let next = Arc::clone(&next);
        let errors = Arc::clone(&errors);
        let start = Arc::clone(&start);
        tasks.spawn(async move {
            start.wait().await;
            loop {
                let sequence = next.fetch_add(1, Ordering::Relaxed);
                if sequence >= operations {
                    break;
                }
                if target
                    .execute_operation(ClientSurfaceOperation::Get, sequence)
                    .await
                    != TargetOutcome::Success
                {
                    errors.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
    }
    let started = Instant::now();
    start.wait().await;
    while let Some(joined) = tasks.join_next().await {
        joined.map_err(|error| ClientSurfaceTierError::Runtime(error.to_string()))?;
    }
    let snapshot = target.snapshot().await;
    target.configure_dispatch_rendezvous(None).await?;
    if errors.load(Ordering::Relaxed) != 0 {
        return Err(ClientSurfaceTierError::Runtime(
            "in-process concurrency sample returned an unsuccessful request".into(),
        ));
    }
    if snapshot.active_router_requests != 0 {
        return Err(ClientSurfaceTierError::Runtime(
            "router request-lifetime accounting did not return to zero".into(),
        ));
    }
    Ok((
        throughput(operations, started.elapsed()),
        snapshot.router_request_high_water,
    ))
}

fn codec_cost_sample(operations: u64, payload_bytes: usize) -> Result<f64, ClientSurfaceTierError> {
    let namespace = Namespace::new("performance")
        .map_err(|error| ClientSurfaceTierError::Runtime(error.to_string()))?;
    let key = StructuredKey::new(vec!["w2".to_owned(), "codec".to_owned()])
        .map_err(|error| ClientSurfaceTierError::Runtime(error.to_string()))?;
    let started = Instant::now();
    for sequence in 0..operations {
        let message = ClientWireMessage::Request(ClientRequestEnvelope::new(
            format!("w2-codec-{sequence}"),
            ClientRequest::Put {
                ns: namespace.clone(),
                key: key.clone(),
                value: vec![sequence as u8; payload_bytes],
                ttl_ms: None,
                dimensions: Vec::new(),
            },
        ));
        let bytes = ClientFrame::from_message_with_version(PROTOCOL_VERSION, &message)
            .and_then(|frame| frame.encode())
            .map_err(|error| ClientSurfaceTierError::Runtime(error.to_string()))?;
        let decoded = ClientFrame::decode(&bytes, 1024 * 1024)
            .and_then(|frame| frame.decode_message())
            .map_err(|error| ClientSurfaceTierError::Runtime(error.to_string()))?;
        if !matches!(decoded, ClientWireMessage::Request(_)) {
            return Err(ClientSurfaceTierError::Runtime(
                "codec sample did not round-trip a request".into(),
            ));
        }
    }
    Ok(nanos_per_operation(operations, started.elapsed()))
}

fn target_config(
    binding: &BoundClientScenario,
    scenario: &Scenario,
    shape: ClientRunShape,
    schedule: Arc<Vec<u64>>,
    delay: Duration,
) -> Result<ClientSurfaceTargetConfig, ClientSurfaceTierError> {
    let (preload_entries, key_space) = match shape {
        ClientRunShape::Smoke => (SMOKE_PRELOAD_ENTRIES, SMOKE_KEY_COUNT),
        ClientRunShape::Reference => (scenario.preload_operations, binding.client.key_count),
    };
    simple_target_config(
        binding.client.max_frame_bytes,
        preload_entries,
        key_space,
        usize::try_from(binding.client.payload_bytes)
            .map_err(|_| ClientSurfaceTierError::Runtime("payload does not fit usize".into()))?,
        binding.client.batch_size,
        parsed_operation_mix(&binding.client)?,
        schedule,
        delay,
    )
}

#[allow(clippy::too_many_arguments)]
fn simple_target_config(
    max_frame_bytes: usize,
    preload_entries: u64,
    key_space: u64,
    payload_bytes: usize,
    batch_size: usize,
    operation_mix: ClientSurfaceOperationMix,
    key_schedule: Arc<Vec<u64>>,
    injected_dispatch_delay: Duration,
) -> Result<ClientSurfaceTargetConfig, ClientSurfaceTierError> {
    let limits = ClientSurfaceLimits {
        max_frame_bytes,
        ..ClientSurfaceLimits::default()
    };
    let config = ClientSurfaceTargetConfig {
        limits,
        preload_entries,
        key_space,
        payload_bytes,
        batch_size,
        operation_mix,
        key_schedule,
        injected_dispatch_delay,
    };
    config.validate().map_err(ClientSurfaceTierError::Runtime)?;
    Ok(config)
}

fn smoke_scenario(binding: &BoundClientScenario) -> Result<Scenario, ClientSurfaceTierError> {
    let mut scenario = binding.scenario.clone();
    scenario.id = format!("{}-smoke", scenario.id);
    scenario.offered_rates_per_second = vec![500, 2_000];
    scenario.preload_operations = SMOKE_PRELOAD_ENTRIES;
    // Keep the pre-steady state identical across A/B/C. Warm-up behavior is
    // already exercised by W0; this W2 smoke proves target/report plumbing.
    scenario.warmup_operations = 0;
    scenario.steady_operations = SMOKE_STEADY_OPERATIONS;
    scenario.repeats = SMOKE_REPEATS as u32;
    scenario.p99_slo_us = 500_000;
    scenario.p999_slo_us = None;
    scenario.p999_min_samples = 1;
    scenario.min_achieved_ratio = 0.50;
    scenario.robust_spread_tolerance = SMOKE_SPREAD_LIMIT;
    scenario
        .validate()
        .map_err(ClientSurfaceTierError::Runtime)?;
    Ok(scenario)
}

fn parse_client_scenario(source: &[u8]) -> Result<BoundClientScenario, ClientSurfaceTierError> {
    let text = std::str::from_utf8(source)
        .map_err(|error| ClientSurfaceTierError::Runtime(error.to_string()))?;
    let mut root = text
        .parse::<toml::Table>()
        .map_err(|error| ClientSurfaceTierError::Runtime(error.to_string()))?;
    let client: ClientSurfaceInputs = root
        .remove("client_surface")
        .ok_or_else(|| ClientSurfaceTierError::Runtime("missing [client_surface]".into()))?
        .try_into()
        .map_err(|error| ClientSurfaceTierError::Runtime(error.to_string()))?;
    validate_client_inputs(&client)?;
    let scenario: Scenario = toml::Value::Table(root)
        .try_into()
        .map_err(|error| ClientSurfaceTierError::Runtime(error.to_string()))?;
    scenario
        .validate()
        .map_err(ClientSurfaceTierError::Runtime)?;
    Ok(BoundClientScenario {
        scenario,
        client,
        source_digest: digest_bytes(source),
    })
}

fn validate_client_inputs(client: &ClientSurfaceInputs) -> Result<(), ClientSurfaceTierError> {
    if !matches!(client.workload.as_str(), "A" | "B" | "C")
        || client.key_count == 0
        || client.payload_bytes == 0
        || client.batch_size == 0
        || client.max_frame_bytes == 0
        || client.operation_mix.is_empty()
        || client.operation_mix.iter().any(|entry| {
            !matches!(
                entry.operation.as_str(),
                "get" | "put" | "batch_get" | "batch_put"
            ) || !entry.weight.is_finite()
                || entry.weight <= 0.0
        })
    {
        return Err(ClientSurfaceTierError::Runtime(
            "client-surface scenario contract is incomplete".into(),
        ));
    }
    let total = client
        .operation_mix
        .iter()
        .map(|entry| entry.weight)
        .sum::<f64>();
    if (total - 1.0).abs() > 1e-9 {
        return Err(ClientSurfaceTierError::Runtime(format!(
            "client-surface operation weights must total 1.0, got {total}"
        )));
    }
    let expected = match client.workload.as_str() {
        "A" => ClientSurfaceOperationMix::WORKLOAD_A,
        "B" => ClientSurfaceOperationMix::WORKLOAD_B,
        "C" => ClientSurfaceOperationMix::WORKLOAD_C,
        _ => unreachable!(),
    };
    if parsed_operation_mix(client)? != expected {
        return Err(ClientSurfaceTierError::Runtime(format!(
            "workload {} does not match its committed A/B/C taxonomy",
            client.workload
        )));
    }
    Ok(())
}

fn parsed_operation_mix(
    client: &ClientSurfaceInputs,
) -> Result<ClientSurfaceOperationMix, ClientSurfaceTierError> {
    let mut mix = ClientSurfaceOperationMix {
        get_percent: 0,
        put_percent: 0,
        batch_get_percent: 0,
        batch_put_percent: 0,
    };
    for entry in &client.operation_mix {
        let percentage = (entry.weight * 100.0).round() as u8;
        match entry.operation.as_str() {
            "get" => mix.get_percent = percentage,
            "put" => mix.put_percent = percentage,
            "batch_get" => mix.batch_get_percent = percentage,
            "batch_put" => mix.batch_put_percent = percentage,
            operation => {
                return Err(ClientSurfaceTierError::Runtime(format!(
                    "unsupported client-surface operation {operation}"
                )))
            }
        }
    }
    Ok(mix)
}

fn validate_concurrency_input(input: &ConcurrencyInput) -> Result<(), ClientSurfaceTierError> {
    if input.schema_version != 1
        || input.id.is_empty()
        || input.operations == 0
        || input.repeats < 3
        || input.preload_entries == 0
        || input.preload_entries > input.key_count
        || input.payload_bytes == 0
        || input.batch_size == 0
        || input.inflight != [1, 10, 100, 1000]
        || input.metric != "operations_per_second"
        || input.execution_mode != "in-process-axum-router"
        || !input.robust_spread_tolerance.is_finite()
        || input.robust_spread_tolerance < 0.0
    {
        return Err(ClientSurfaceTierError::Runtime(
            "concurrency scenario contract is incomplete".into(),
        ));
    }
    Ok(())
}

impl ConcurrencyInput {
    fn max_frame_bytes_or_default(&self) -> usize {
        ClientSurfaceLimits::default().max_frame_bytes
    }
}

fn validate_payload_input(input: &PayloadInput) -> Result<(), ClientSurfaceTierError> {
    if input.schema_version != 1
        || input.id.is_empty()
        || input.operations_per_repeat == 0
        || input.repeats < 3
        || input.key_count == 0
        || input.payload_bytes != [100, 1_000, 65_536, 1_000_000]
        || input.max_frame_bytes != 1_048_576
        || input.beyond_cap_value_bytes < input.max_frame_bytes
        || input.metric != "operations_per_second"
        || !input.require_http_413_beyond_cap
        || !input.robust_spread_tolerance.is_finite()
        || input.robust_spread_tolerance < 0.0
    {
        return Err(ClientSurfaceTierError::Runtime(
            "payload scenario contract is incomplete".into(),
        ));
    }
    Ok(())
}

fn validate_path_cost_input(input: &PathCostInput) -> Result<(), ClientSurfaceTierError> {
    if input.schema_version != 1
        || input.id.is_empty()
        || input.operations_per_repeat == 0
        || input.repeats < 3
        || input.key_count == 0
        || input.payload_bytes == 0
        || input.max_frame_bytes != 1_048_576
        || input.beyond_cap_value_bytes < input.max_frame_bytes
        || input.paths
            != [
                "codec_encode_decode",
                "router_dispatch",
                "oversized_admission_rejection",
            ]
        || input.metric != "nanoseconds_per_operation"
        || !input.require_rejection_before_dispatch
        || !input.robust_spread_tolerance.is_finite()
        || input.robust_spread_tolerance < 0.0
    {
        return Err(ClientSurfaceTierError::Runtime(
            "path-cost scenario contract is incomplete".into(),
        ));
    }
    Ok(())
}

fn parse_toml<T>(source: &[u8]) -> Result<T, ClientSurfaceTierError>
where
    T: for<'de> Deserialize<'de>,
{
    let text = std::str::from_utf8(source)
        .map_err(|error| ClientSurfaceTierError::Runtime(error.to_string()))?;
    toml::from_str(text).map_err(|error| ClientSurfaceTierError::Runtime(error.to_string()))
}

fn workload_identity(
    schedule: &GeneratedKeySchedule,
    input: &ClientSurfaceInputs,
) -> WorkloadIdentity {
    workload_identity_from_parts(
        schedule,
        input
            .operation_mix
            .iter()
            .map(|entry| WeightedOperation {
                operation: entry.operation.clone(),
                weight: entry.weight,
            })
            .collect(),
        vec![WeightedPayload {
            bytes: input.payload_bytes,
            weight: 1.0,
        }],
    )
}

fn workload_identity_from_parts(
    schedule: &GeneratedKeySchedule,
    operation_mix: Vec<WeightedOperation>,
    payload_mix: Vec<WeightedPayload>,
) -> WorkloadIdentity {
    let (kind, theta) = match schedule.spec.distribution {
        KeyDistribution::Uniform => ("uniform", None),
        KeyDistribution::Zipfian { theta } => ("zipfian", Some(theta)),
    };
    let operation_bytes =
        serde_json::to_vec(&operation_mix).expect("validated client operation mix must serialize");
    let payload_bytes =
        serde_json::to_vec(&payload_mix).expect("validated client payload mix must serialize");
    WorkloadIdentity {
        generator: "hydracache-cache-sim-key-schedule".to_owned(),
        generator_version: KEY_SCHEDULE_GENERATOR_VERSION.to_string(),
        seed: Some(schedule.spec.seed),
        key_distribution: Some(KeyDistributionIdentity {
            kind: kind.to_owned(),
            theta,
        }),
        key_count: Some(schedule.spec.key_count),
        operation_mix,
        payload_mix,
        digest: digest_parts(&[
            schedule.digest.as_bytes(),
            b"hydracache-client-surface-workload-v1",
            &operation_bytes,
            &payload_bytes,
        ]),
    }
}

fn effective_digest(
    source_digest: &str,
    client: &ClientSurfaceInputs,
    scenario: &Scenario,
    shape: ClientRunShape,
) -> String {
    let client = serde_json::to_vec(client).expect("validated client inputs must serialize");
    let scenario = serde_json::to_vec(scenario).expect("validated scenario must serialize");
    let domain = match shape {
        ClientRunShape::Smoke => b"hydracache-client-surface-smoke-input-v1".as_slice(),
        ClientRunShape::Reference => b"hydracache-client-surface-reference-input-v1".as_slice(),
    };
    digest_parts(&[source_digest.as_bytes(), domain, &client, &scenario])
}

fn custom_effective_digest(
    source: &[u8],
    effective: &serde_json::Value,
    shape: ClientRunShape,
) -> String {
    let effective = serde_json::to_vec(effective).expect("effective inputs must serialize");
    let domain = match shape {
        ClientRunShape::Smoke => b"hydracache-client-surface-smoke-input-v1".as_slice(),
        ClientRunShape::Reference => b"hydracache-client-surface-reference-input-v1".as_slice(),
    };
    digest_parts(&[digest_bytes(source).as_bytes(), domain, &effective])
}

fn scalar_point(
    dimensions: BTreeMap<String, DimensionValue>,
    unit: &str,
    samples: Vec<f64>,
) -> ScalarPoint {
    let mut ordered = samples.clone();
    ordered.sort_by(f64::total_cmp);
    let value = ordered[ordered.len() / 2];
    let min = ordered[0];
    let max = ordered[ordered.len() - 1];
    let robust_spread_ratio = if value > 0.0 {
        (max - min) / value
    } else if max == min {
        0.0
    } else {
        f64::INFINITY
    };
    ScalarPoint {
        dimensions,
        quantity: Quantity {
            value,
            unit: unit.to_owned(),
        },
        sample_count: samples.len() as u64,
        samples,
        min,
        max,
        robust_spread_ratio,
    }
}

fn throughput(operations: u64, elapsed: Duration) -> f64 {
    operations as f64 / elapsed.as_secs_f64().max(f64::EPSILON)
}

fn nanos_per_operation(operations: u64, elapsed: Duration) -> f64 {
    elapsed.as_nanos() as f64 / operations.max(1) as f64
}

fn ensure_success(outcome: TargetOutcome, path: &str) -> Result<(), ClientSurfaceTierError> {
    if outcome == TargetOutcome::Success {
        Ok(())
    } else {
        Err(ClientSurfaceTierError::Runtime(format!(
            "client-surface {path} returned {outcome:?}"
        )))
    }
}

fn validate_client_surface_reference_scalar_shapes(
    report: &PerfReport,
) -> Result<(), ClientSurfaceTierError> {
    let ids = report
        .measurements
        .iter()
        .map(|measurement| match measurement {
            MeasurementEvidence::LoadCurve(value) => value.id.as_str(),
            MeasurementEvidence::Scalar(value) => value.id.as_str(),
            MeasurementEvidence::TraceReplay(value) => value.id.as_str(),
            MeasurementEvidence::Comparison(value) => value.id.as_str(),
        })
        .collect::<BTreeSet<_>>();
    let expected_ids = BTreeSet::from([
        "client_surface_in_process_knee_at_slo_workload_a",
        "client_surface_in_process_knee_at_slo_workload_b",
        "client_surface_in_process_knee_at_slo_workload_c",
        "client_surface_in_process_knee_at_slo_for_a_b_c",
        "concurrent_inflight_scaling_curve_1_10_100_1000",
        "client_surface_payload_sweep_100b_1kb_64kb_1mb",
        "client_surface_codec_dispatch_and_admission_rejection_cost",
    ]);
    if ids != expected_ids {
        return Err(ClientSurfaceTierError::Report(format!(
            "W2 reference report measurement set differs from the exact contract: {ids:?}"
        )));
    }
    let aggregate = scalar_measurement(report, "client_surface_in_process_knee_at_slo_for_a_b_c")?;
    if aggregate.points.len() != 3
        || aggregate.max_robust_spread_ratio != 0.15
        || aggregate
            .points
            .iter()
            .any(|point| point.sample_count != 3 || point.samples.len() != 3)
    {
        return Err(ClientSurfaceTierError::Report(
            "W2 aggregate A/B/C shape lost its exact repeat/spread contract".to_owned(),
        ));
    }

    let concurrency_input: ConcurrencyInput = parse_toml(CONCURRENCY_SCENARIO)?;
    let concurrency =
        scalar_measurement(report, "concurrent_inflight_scaling_curve_1_10_100_1000")?;
    let concurrency_digest = ClientRunShape::Reference.custom_digest(
        CONCURRENCY_SCENARIO,
        &serde_json::json!({
            "operations": concurrency_input.operations,
            "repeats": concurrency_input.repeats,
        }),
    );
    let inflight = concurrency
        .points
        .iter()
        .filter_map(|point| match point.dimensions.get("concurrent_inflight") {
            Some(DimensionValue::U64(value)) => Some(*value),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    if concurrency.scenario_digest != concurrency_digest
        || concurrency.max_robust_spread_ratio != concurrency_input.robust_spread_tolerance
        || concurrency.points.len() != 4
        || inflight != BTreeSet::from([1, 10, 100, 1_000])
        || concurrency
            .points
            .iter()
            .any(|point| point.sample_count != 3 || point.samples.len() != 3)
    {
        return Err(ClientSurfaceTierError::Report(
            "W2 reference concurrency evidence differs from the exact 4000-op 1/10/100/1000 contract"
                .to_owned(),
        ));
    }

    let payload_input: PayloadInput = parse_toml(PAYLOAD_SCENARIO)?;
    let payload = scalar_measurement(report, "client_surface_payload_sweep_100b_1kb_64kb_1mb")?;
    let payload_digest = ClientRunShape::Reference.custom_digest(
        PAYLOAD_SCENARIO,
        &serde_json::json!({
            "operations_per_repeat": payload_input.operations_per_repeat,
            "repeats": payload_input.repeats,
        }),
    );
    let payloads = payload
        .points
        .iter()
        .filter_map(|point| match point.dimensions.get("payload_bytes") {
            Some(DimensionValue::U64(value)) => Some(*value),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    if payload.scenario_digest != payload_digest
        || payload.max_robust_spread_ratio != payload_input.robust_spread_tolerance
        || payload.points.len() != 4
        || payloads != BTreeSet::from([100, 1_000, 65_536, 1_000_000])
        || payload
            .points
            .iter()
            .any(|point| point.sample_count != 3 || point.samples.len() != 3)
    {
        return Err(ClientSurfaceTierError::Report(
            "W2 reference payload evidence differs from the exact committed sweep".to_owned(),
        ));
    }

    let path_input: PathCostInput = parse_toml(PATH_COST_SCENARIO)?;
    let path = scalar_measurement(
        report,
        "client_surface_codec_dispatch_and_admission_rejection_cost",
    )?;
    let path_digest = ClientRunShape::Reference.custom_digest(
        PATH_COST_SCENARIO,
        &serde_json::json!({
            "operations_per_repeat": path_input.operations_per_repeat,
            "repeats": path_input.repeats,
        }),
    );
    let paths = path
        .points
        .iter()
        .filter_map(|point| match point.dimensions.get("path") {
            Some(DimensionValue::Text(value)) => Some(value.as_str()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    if path.scenario_digest != path_digest
        || path.max_robust_spread_ratio != path_input.robust_spread_tolerance
        || path.points.len() != 3
        || paths
            != BTreeSet::from([
                "codec_encode_decode",
                "router_dispatch",
                "oversized_admission_rejection",
            ])
        || path
            .points
            .iter()
            .any(|point| point.sample_count != 3 || point.samples.len() != 3)
    {
        return Err(ClientSurfaceTierError::Report(
            "W2 reference path-cost evidence differs from the exact committed 64-op shape"
                .to_owned(),
        ));
    }
    Ok(())
}

fn scalar_measurement<'a>(
    report: &'a PerfReport,
    id: &str,
) -> Result<&'a ScalarEvidence, ClientSurfaceTierError> {
    report
        .measurements
        .iter()
        .find_map(|measurement| match measurement {
            MeasurementEvidence::Scalar(value) if value.id == id => Some(value),
            _ => None,
        })
        .ok_or_else(|| ClientSurfaceTierError::Report(format!("W2 scalar {id} is absent")))
}

fn exact_loadgen_digest(report: &PerfReport) -> Result<String, ClientSurfaceTierError> {
    let matches = report
        .build
        .binary_sha256
        .iter()
        .filter(|(id, digest)| id == "hydracache-loadgen" && is_sha256(digest))
        .map(|(_, digest)| digest.clone())
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(ClientSurfaceTierError::Report(
            "W2 reference report does not bind exactly one loadgen binary".to_owned(),
        ));
    }
    Ok(matches[0].clone())
}

fn instance_from_dimensions(
    dimensions: &BTreeMap<String, DimensionValue>,
    stable_capability_sha256: String,
) -> Result<ClientSurfaceReferenceInstanceReceipt, ClientSurfaceTierError> {
    Ok(ClientSurfaceReferenceInstanceReceipt {
        schema_version: u32_dimension(dimensions, "reference_instance_schema_version")?,
        instance_sequence: required_u64_dimension(dimensions, "reference_instance_sequence")?,
        owning_pid: u32_dimension(dimensions, "reference_owning_pid")?,
        created_unix_nanos: required_u64_dimension(
            dimensions,
            "reference_instance_created_unix_nanos",
        )?,
        direct_prebuilt_exec: bool_dimension(dimensions, "direct_prebuilt_exec").ok_or_else(
            || ClientSurfaceTierError::Report("missing direct-prebuild proof".to_owned()),
        )?,
        loadgen_binary_path: required_text_dimension(dimensions, "loadgen_binary_path")?,
        loadgen_binary_sha256: required_text_dimension(dimensions, "loadgen_binary_sha256")?,
        stable_capability_sha256,
        receipt_sha256: required_text_dimension(dimensions, "reference_instance_receipt_sha256")?,
    })
}

fn text_dimension<'a>(
    dimensions: &'a BTreeMap<String, DimensionValue>,
    name: &str,
) -> Option<&'a str> {
    match dimensions.get(name) {
        Some(DimensionValue::Text(value)) => Some(value),
        _ => None,
    }
}

fn required_text_dimension(
    dimensions: &BTreeMap<String, DimensionValue>,
    name: &str,
) -> Result<String, ClientSurfaceTierError> {
    text_dimension(dimensions, name)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ClientSurfaceTierError::Report(format!("missing W2 text dimension {name}")))
}

fn bool_dimension(dimensions: &BTreeMap<String, DimensionValue>, name: &str) -> Option<bool> {
    match dimensions.get(name) {
        Some(DimensionValue::Bool(value)) => Some(*value),
        _ => None,
    }
}

fn required_u64_dimension(
    dimensions: &BTreeMap<String, DimensionValue>,
    name: &str,
) -> Result<u64, ClientSurfaceTierError> {
    match dimensions.get(name) {
        Some(DimensionValue::U64(value)) => Ok(*value),
        _ => Err(ClientSurfaceTierError::Report(format!(
            "missing W2 integer dimension {name}"
        ))),
    }
}

fn u32_dimension(
    dimensions: &BTreeMap<String, DimensionValue>,
    name: &str,
) -> Result<u32, ClientSurfaceTierError> {
    required_u64_dimension(dimensions, name)?
        .try_into()
        .map_err(|_| {
            ClientSurfaceTierError::Report(format!("W2 dimension {name} does not fit u32"))
        })
}

fn first_state_digest(
    measurements: &[MeasurementEvidence],
) -> Result<String, ClientSurfaceTierError> {
    measurements
        .iter()
        .find_map(|measurement| match measurement {
            MeasurementEvidence::LoadCurve(curve) => curve
                .knee
                .as_ref()
                .and_then(|knee| knee.evaluated.first())
                .and_then(|point| point.repeats.first())
                .map(|repeat| repeat.state_digest.clone()),
            _ => None,
        })
        .ok_or_else(|| ClientSurfaceTierError::Report("W2 report has no state digest".to_owned()))
}

fn write_report(report: &PerfReport, path: &Path) -> Result<(), ClientSurfaceTierError> {
    let bytes = report
        .to_pretty_json()
        .map_err(|error| ClientSurfaceTierError::Report(error.to_string()))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)?;
    Ok(())
}

fn canonical_digest<T: Serialize>(value: &T) -> Result<String, ClientSurfaceTierError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| ClientSurfaceTierError::Report(error.to_string()))?;
    Ok(digest_bytes(&bytes))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn is_git_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn digest_bytes(bytes: &[u8]) -> String {
    hex_digest(Sha256::digest(bytes).as_ref())
}

fn digest_parts(parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part);
    }
    hex_digest(hasher.finalize().as_ref())
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn smoke_fingerprint() -> RunnerFingerprint {
    RunnerFingerprint {
        runner_class: "smoke-unclaimed".to_owned(),
        fingerprint: "smoke-client-surface".to_owned(),
        cpu_model: "unclaimed-local-host".to_owned(),
        logical_cores: std::thread::available_parallelism()
            .map(|value| value.get() as u32)
            .unwrap_or(1),
        ram_bytes: 1,
        os: std::env::consts::OS.to_owned(),
        kernel: "unclaimed".to_owned(),
        cpu_affinity: "unclaimed".to_owned(),
        cgroup_cpu_quota: "unclaimed".to_owned(),
        governor: "unclaimed".to_owned(),
        turbo: "unclaimed".to_owned(),
        shared_hardware: true,
        calibration_score: 0.0,
    }
}

fn smoke_profile(name: &str, fingerprint: &RunnerFingerprint) -> PerformanceProfile {
    PerformanceProfile {
        name: name.to_owned(),
        required_runner_class: fingerprint.runner_class.clone(),
        allowed_fingerprints: vec![fingerprint.fingerprint.clone()],
        minimum_logical_cores: 1,
        required_cpu_affinity: fingerprint.cpu_affinity.clone(),
        required_cgroup_cpu_quota: fingerprint.cgroup_cpu_quota.clone(),
        require_dedicated: true,
        maximum_calibration_score: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mismatched_reference_context() -> ValidatedRespReferenceContext {
        let missing = Path::new(env!("CARGO_MANIFEST_DIR")).join("missing-reference-binary");
        let runner = RunnerFingerprint {
            runner_class: "reference-v1".to_owned(),
            fingerprint: "fixture".to_owned(),
            cpu_model: "fixture".to_owned(),
            logical_cores: 8,
            ram_bytes: 1,
            os: "fixture".to_owned(),
            kernel: "fixture".to_owned(),
            cpu_affinity: "dedicated-cpuset".to_owned(),
            cgroup_cpu_quota: "unlimited".to_owned(),
            governor: "fixture".to_owned(),
            turbo: "fixture".to_owned(),
            shared_hardware: false,
            calibration_score: 0.0,
        };
        ValidatedRespReferenceContext {
            repo_root: Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
            manifest_path: missing.clone(),
            manifest_sha256: "1".repeat(64),
            source: SourceIdentity {
                git_commit: "a".repeat(40),
                cargo_lock_sha256: "b".repeat(64),
                toolchain: "rustc-fixture".to_owned(),
                build_flags: vec!["--release".to_owned()],
            },
            build: BuildIdentity {
                prebuild_contract_digest: "c".repeat(64),
                prebuild_manifest_sha256: "1".repeat(64),
                binary_sha256: vec![("hydracache-loadgen".to_owned(), "d".repeat(64))],
            },
            profile: PerformanceProfile {
                name: "reference-v1".to_owned(),
                required_runner_class: "reference-v1".to_owned(),
                allowed_fingerprints: vec!["fixture".to_owned()],
                minimum_logical_cores: 8,
                required_cpu_affinity: "dedicated-cpuset".to_owned(),
                required_cgroup_cpu_quota: "unlimited".to_owned(),
                require_dedicated: true,
                maximum_calibration_score: 0.05,
            },
            runner,
            surface: SurfaceIdentity {
                surface_kind: "node-resp".to_owned(),
                execution_mode: "unused".to_owned(),
                state_scope: "node-local".to_owned(),
                network_boundary: "loopback-tcp".to_owned(),
                claim_scope: "unused".to_owned(),
            },
            server: crate::tiers::resp_reference::VerifiedBinary {
                id: "hydracache-server".to_owned(),
                canonical_path: missing.clone(),
                sha256: "e".repeat(64),
            },
            loadgen: crate::tiers::resp_reference::VerifiedBinary {
                id: "hydracache-loadgen".to_owned(),
                canonical_path: missing,
                sha256: "d".repeat(64),
            },
        }
    }

    #[test]
    fn committed_abc_taxonomy_is_exact_and_digest_bound() {
        for (source, expected) in [
            (WORKLOAD_A_SCENARIO, ClientSurfaceOperationMix::WORKLOAD_A),
            (WORKLOAD_B_SCENARIO, ClientSurfaceOperationMix::WORKLOAD_B),
            (WORKLOAD_C_SCENARIO, ClientSurfaceOperationMix::WORKLOAD_C),
        ] {
            let binding = parse_client_scenario(source).unwrap();
            assert_eq!(parsed_operation_mix(&binding.client).unwrap(), expected);
            assert_eq!(binding.source_digest.len(), 64);
        }
    }

    #[test]
    fn reference_profile_is_not_a_smoke_alias() {
        let fingerprint = smoke_fingerprint();
        assert_ne!(smoke_profile("smoke-v1", &fingerprint).name, "reference-v1");
    }

    #[tokio::test]
    async fn reference_dispatch_rejects_missing_and_mismatched_context_without_downgrade() {
        let missing = run_client_surface_profile_with_context("reference-v1", None)
            .await
            .unwrap_err();
        assert!(missing
            .to_string()
            .contains("validated W7 reference context"));

        let mismatched = client_surface_reference_report(&mismatched_reference_context())
            .await
            .unwrap_err();
        assert!(mismatched.to_string().contains("receipt-bound binary"));
    }

    #[tokio::test]
    async fn strict_reference_validator_rejects_smoke_evidence() {
        let smoke = client_surface_smoke_report("smoke-v1").await.unwrap();
        assert!(validate_client_surface_reference_report(&smoke).is_err());
    }
}
