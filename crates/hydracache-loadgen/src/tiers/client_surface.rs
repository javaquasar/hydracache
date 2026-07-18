//! Release-0.67 W2 characterization of the in-process Axum client router.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

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
use crate::{PerformanceProfile, RunnerFingerprint};

const SMOKE_REPEATS: usize = 3;
const SMOKE_SPREAD_LIMIT: f64 = 1_000.0;
const SMOKE_KEY_COUNT: u64 = 32;
const SMOKE_PRELOAD_ENTRIES: u64 = 16;
const SMOKE_STEADY_OPERATIONS: u64 = 100;

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
    let mut measurements = client_surface_knee_smoke_measurements().await?;
    measurements.push(client_surface_concurrency_smoke_measurement().await?);
    measurements.push(client_surface_payload_smoke_measurement().await?);
    measurements.push(client_surface_path_cost_smoke_measurement().await?);
    Ok(measurements)
}

/// Construct all A/B/C raw knees plus the required aggregate result.
pub async fn client_surface_knee_smoke_measurements(
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
        let schedule = KeyScheduleSpec::uniform(
            binding.scenario.seed,
            SMOKE_KEY_COUNT,
            SMOKE_STEADY_OPERATIONS,
        )
        .generate()
        .map_err(ClientSurfaceTierError::Runtime)?;
        let target = Arc::new(ClientSurfaceTarget::new(target_config(
            &binding,
            Arc::new(schedule.keys.clone()),
            Duration::ZERO,
        )?)?);
        let scenario = smoke_scenario(&binding)?;
        let criteria = scenario.sustainability_criteria();
        let knee = run_scenario(target, &scenario).await?;
        let sustainable_rate = knee.sustainable_rate_per_second.ok_or_else(|| {
            ClientSurfaceTierError::Runtime(format!(
                "{} smoke knee had no sustainable point",
                binding.client.workload
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
        let scenario_digest = effective_digest(&binding.source_digest, &binding.client, &scenario);
        aggregate_scenario_inputs.push((id.clone(), scenario_digest.clone()));
        aggregate_workload_inputs.push((id.clone(), workload.digest.clone()));
        curves.push(MeasurementEvidence::LoadCurve(LoadCurveEvidence {
            id,
            scenario_digest,
            dimensions: BTreeMap::from([
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
            ]),
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
        key_count: Some(SMOKE_KEY_COUNT),
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
        max_robust_spread_ratio: SMOKE_SPREAD_LIMIT,
    }));
    Ok(curves)
}

/// Measure in-flight request scaling, never connection scaling.
pub async fn client_surface_concurrency_smoke_measurement(
) -> Result<MeasurementEvidence, ClientSurfaceTierError> {
    let input: ConcurrencyInput = parse_toml(CONCURRENCY_SCENARIO)?;
    validate_concurrency_input(&input)?;
    let operations = 1_000;
    let schedule = KeyScheduleSpec::uniform(input.seed, input.key_count, operations)
        .generate()
        .map_err(ClientSurfaceTierError::Runtime)?;
    let mut points = Vec::new();
    for inflight in &input.inflight {
        let mut samples = Vec::with_capacity(SMOKE_REPEATS);
        let mut observed_high_water = Vec::with_capacity(SMOKE_REPEATS);
        for _ in 0..SMOKE_REPEATS {
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
        scenario_digest: custom_effective_digest(
            CONCURRENCY_SCENARIO,
            &serde_json::json!({"operations": operations, "repeats": SMOKE_REPEATS}),
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
        max_robust_spread_ratio: SMOKE_SPREAD_LIMIT,
    }))
}

/// Measure accepted payloads and prove that one value beyond the frame cap is
/// rejected with HTTP 413 before dispatch or mutation.
pub async fn client_surface_payload_smoke_measurement(
) -> Result<MeasurementEvidence, ClientSurfaceTierError> {
    let input: PayloadInput = parse_toml(PAYLOAD_SCENARIO)?;
    validate_payload_input(&input)?;
    let operations = input.operations_per_repeat.min(8);
    let schedule = KeyScheduleSpec::uniform(input.seed, input.key_count, operations)
        .generate()
        .map_err(ClientSurfaceTierError::Runtime)?;
    let mut points = Vec::new();
    for payload_bytes in &input.payload_bytes {
        let mut samples = Vec::with_capacity(SMOKE_REPEATS);
        for _ in 0..SMOKE_REPEATS {
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
    let mut rejection_samples = Vec::with_capacity(SMOKE_REPEATS);
    for _ in 0..SMOKE_REPEATS {
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
        scenario_digest: custom_effective_digest(
            PAYLOAD_SCENARIO,
            &serde_json::json!({"operations_per_repeat": operations, "repeats": SMOKE_REPEATS}),
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
        max_robust_spread_ratio: SMOKE_SPREAD_LIMIT,
    }))
}

/// Price codec-only, real router dispatch, and pre-dispatch admission paths.
pub async fn client_surface_path_cost_smoke_measurement(
) -> Result<MeasurementEvidence, ClientSurfaceTierError> {
    let input: PathCostInput = parse_toml(PATH_COST_SCENARIO)?;
    validate_path_cost_input(&input)?;
    let operations = input.operations_per_repeat.min(8);
    let schedule = KeyScheduleSpec::uniform(input.seed, input.key_count, operations)
        .generate()
        .map_err(ClientSurfaceTierError::Runtime)?;
    let mut codec_samples = Vec::new();
    let mut dispatch_samples = Vec::new();
    let mut admission_samples = Vec::new();
    for _ in 0..SMOKE_REPEATS {
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
        scenario_digest: custom_effective_digest(
            PATH_COST_SCENARIO,
            &serde_json::json!({"operations_per_repeat": operations, "repeats": SMOKE_REPEATS}),
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
        max_robust_spread_ratio: SMOKE_SPREAD_LIMIT,
    }))
}

/// Canary curve with a loadgen-owned delay immediately before real dispatch.
pub async fn client_surface_dispatch_knee(
    injected_delay: Duration,
) -> Result<crate::KneeResult, ClientSurfaceTierError> {
    let binding = parse_client_scenario(WORKLOAD_C_SCENARIO)?;
    let schedule = KeyScheduleSpec::uniform(binding.scenario.seed, 16, SMOKE_STEADY_OPERATIONS)
        .generate()
        .map_err(ClientSurfaceTierError::Runtime)?;
    let target = Arc::new(ClientSurfaceTarget::new(target_config(
        &binding,
        Arc::new(schedule.keys),
        injected_delay,
    )?)?);
    let mut scenario = smoke_scenario(&binding)?;
    scenario.offered_rates_per_second = vec![1_000];
    scenario.p99_slo_us = 5_000;
    scenario.robust_spread_tolerance = SMOKE_SPREAD_LIMIT;
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
    schedule: Arc<Vec<u64>>,
    delay: Duration,
) -> Result<ClientSurfaceTargetConfig, ClientSurfaceTierError> {
    simple_target_config(
        binding.client.max_frame_bytes,
        SMOKE_PRELOAD_ENTRIES,
        SMOKE_KEY_COUNT,
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
) -> String {
    let client = serde_json::to_vec(client).expect("validated client inputs must serialize");
    let scenario = serde_json::to_vec(scenario).expect("validated scenario must serialize");
    digest_parts(&[
        source_digest.as_bytes(),
        b"hydracache-client-surface-smoke-input-v1",
        &client,
        &scenario,
    ])
}

fn custom_effective_digest(source: &[u8], effective: &serde_json::Value) -> String {
    let effective = serde_json::to_vec(effective).expect("effective inputs must serialize");
    digest_parts(&[
        digest_bytes(source).as_bytes(),
        b"hydracache-client-surface-smoke-input-v1",
        &effective,
    ])
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
}
