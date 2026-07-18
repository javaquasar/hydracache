use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use hydracache::{CacheOptions, HydraCache};
use hydracache_cache_sim::{
    trace_digest, GeneratedKeySchedule, KeyDistribution, KeyScheduleSpec, TraceCatalogId,
    KEY_SCHEDULE_GENERATOR_VERSION,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::allocation::measure_allocations;
use crate::report::{
    BuildIdentity, DimensionValue, EvidenceRunMode, KeyDistributionIdentity, LoadClaim,
    LoadCurveEvidence, MeasurementEvidence, PerfReport, Quantity, ScalarEvidence, ScalarPoint,
    SourceIdentity, SurfaceIdentity, TraceReplayEvidence, WeightedOperation, WeightedPayload,
    WorkloadIdentity,
};
use crate::runner::run_scenario;
use crate::scenario::Scenario;
use crate::target::{Target, TargetError, TargetOutcome};
use crate::targets::local::{
    LocalCacheTarget, LocalOperation, LocalOperationMix, LocalTargetConfig,
};
use crate::{PerformanceProfile, RunnerFingerprint};

const SMOKE_REPEATS: usize = 3;
const SMOKE_OPERATIONS: u64 = 240;
const SMOKE_SPREAD_LIMIT: f64 = 1_000.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalOperationInput {
    operation: String,
    weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalScenarioInputs {
    key_count: u64,
    payload_bytes: u64,
    distribution: String,
    #[serde(default)]
    zipfian_theta: Option<f64>,
    #[serde(default)]
    worker_counts: Vec<usize>,
    #[serde(default)]
    loader_delay_us: u64,
    #[serde(default)]
    full_capacity_bytes: Option<u64>,
    #[serde(default)]
    half_capacity_bytes: Option<u64>,
    operation_mix: Vec<LocalOperationInput>,
}

#[derive(Debug, Clone)]
struct BoundLocalScenario {
    scenario: Scenario,
    local: LocalScenarioInputs,
    source_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AllocationScenarioInput {
    schema_version: u32,
    id: String,
    seed: u64,
    operations: u64,
    payload_bytes: u64,
    repeats: usize,
    features: Vec<String>,
    metric: String,
    robust_spread_tolerance: f64,
    includes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TraceScenarioInput {
    schema_version: u32,
    id: String,
    seed: u64,
    catalog: String,
    traces: Vec<String>,
    mode: String,
    require_order_preserved: bool,
    require_input_replay_digest_match: bool,
}

const SCALING_SCENARIO: &[u8] =
    include_bytes!("../../../../docs/testing/perf-scenarios/0.67/local-scaling-v1.toml");
const HOT_KEY_SCENARIO: &[u8] =
    include_bytes!("../../../../docs/testing/perf-scenarios/0.67/local-hot-key-v1.toml");
const CAPACITY_SCENARIO: &[u8] =
    include_bytes!("../../../../docs/testing/perf-scenarios/0.67/local-capacity-pressure-v1.toml");
const PATH_SCENARIO: &[u8] =
    include_bytes!("../../../../docs/testing/perf-scenarios/0.67/local-path-cost-v1.toml");
const ALLOCATION_SCENARIO: &[u8] =
    include_bytes!("../../../../docs/testing/perf-scenarios/0.67/local-allocation-v1.toml");
const TRACE_SCENARIO: &[u8] =
    include_bytes!("../../../../docs/testing/perf-scenarios/0.67/local-trace-replay-v1.toml");

/// Exact W1 measurement ids required in every local-tier report.
pub const REQUIRED_LOCAL_MEASUREMENTS: [&str; 6] = [
    "local_cache_scaling_curve_1_to_n_threads",
    "hot_key_contention_throughput_floor",
    "throughput_at_full_capacity_vs_half_capacity",
    "hit_miss_and_loader_path_cost_breakdown",
    "bytes_allocated_per_operation_by_feature",
    "w22_trace_replay_preserves_order_and_records_trace_digest",
];

#[derive(Debug, thiserror::Error)]
pub enum LocalTierError {
    #[error(transparent)]
    Target(#[from] TargetError),
    #[error("local tier runtime failed: {0}")]
    Runtime(String),
    #[error("local tier report failed: {0}")]
    Report(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Run the complete short W1 contract. These numbers only validate plumbing.
pub async fn local_smoke_measurements() -> Result<Vec<MeasurementEvidence>, LocalTierError> {
    let (scaling, scaling_efficiency) = local_scaling_smoke_measurements().await?;
    Ok(vec![
        scaling,
        scaling_efficiency,
        local_hot_key_smoke_measurement().await?,
        local_hot_key_single_flight_smoke_measurement().await?,
        local_capacity_smoke_measurement().await?,
        local_path_cost_smoke_measurement().await?,
        local_allocation_smoke_measurement().await?,
        local_trace_replay_smoke_measurement().await?,
    ])
}

/// Build a schema-valid but deliberately non-ship smoke report.
pub async fn local_smoke_report(profile_name: &str) -> Result<PerfReport, LocalTierError> {
    if profile_name != "smoke-v1" {
        return Err(LocalTierError::Report(format!(
            "profile {profile_name:?} cannot be attached to plumbing-only smoke evidence"
        )));
    }
    let measurements = local_smoke_measurements().await?;
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
        .ok_or_else(|| {
            LocalTierError::Report("local smoke run produced no state digest".to_owned())
        })?;
    let fingerprint = smoke_fingerprint();
    let profile = smoke_profile(profile_name, &fingerprint);
    Ok(PerfReport::new(
        "local-tier-smoke",
        "local-w1-suite-smoke",
        state_digest,
        EvidenceRunMode::Smoke,
        SurfaceIdentity {
            surface_kind: "embedded-cache".to_owned(),
            execution_mode: "in-process-real-hydracache".to_owned(),
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
        measurements,
        vec!["short smoke workload is not capacity evidence".to_owned()],
    ))
}

/// Both direct `tier local` and aggregate `suite core` forms call this writer.
pub async fn write_local_smoke_report(
    profile_name: &str,
    path: &Path,
) -> Result<(), LocalTierError> {
    let report = local_smoke_report(profile_name).await?;
    let bytes = report
        .to_pretty_json()
        .map_err(|error| LocalTierError::Report(error.to_string()))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Select a local execution mode without ever downgrading a requested
/// reference profile into smoke evidence. W7 installs the receipt-bound
/// reference context consumed by this dispatch point.
pub async fn write_local_report(profile_name: &str, path: &Path) -> Result<(), LocalTierError> {
    match profile_name {
        "smoke-v1" => write_local_smoke_report(profile_name, path).await,
        "reference-v1" => Err(LocalTierError::Report(
            "reference-v1 requires the W7 profile and receipt-bound prebuild context; refusing to emit smoke evidence"
                .to_owned(),
        )),
        _ => Err(LocalTierError::Report(format!(
            "unknown local performance profile {profile_name:?}"
        ))),
    }
}

/// Capacity-pressure curve used by the registered W1 defect canary.
pub async fn local_pressure_knee(
    injected_delay: Duration,
) -> Result<crate::KneeResult, LocalTierError> {
    let binding = parse_local_scenario(CAPACITY_SCENARIO)?;
    let payload_bytes = usize::try_from(binding.local.payload_bytes)
        .map_err(|_| LocalTierError::Runtime("payload size does not fit usize".to_owned()))?;
    let preload_entries = discover_full_preload_entries(2 * 1024, payload_bytes, 64).await?;
    let proof_schedule = KeyScheduleSpec::uniform(
        binding.scenario.seed,
        binding.local.key_count.clamp(1, 64),
        30,
    )
    .generate()
    .map_err(LocalTierError::Runtime)?;
    verify_each_pressure_insert_evicts(2 * 1024, payload_bytes, preload_entries, &proof_schedule)
        .await?;
    let target = Arc::new(LocalCacheTarget::new(LocalTargetConfig {
        max_capacity: 2 * 1024,
        max_entry_bytes: payload_bytes.saturating_mul(4),
        preload_entries,
        key_space: binding.local.key_count.clamp(1, 64),
        payload_bytes,
        operation_mix: LocalOperationMix {
            hit_percent: 0,
            miss_percent: 0,
            loader_percent: 0,
            put_percent: 0,
            hot_key_percent: 100,
        },
        loader_delay: Duration::ZERO,
        hot_key_expected_miss_waiters: None,
        capacity_pressure_every: Some(1),
        injected_capacity_pressure_delay: injected_delay,
    })?);
    let scenario = smoke_scenario(&binding, vec![1_000], 30, preload_entries, 5_000)?;
    Ok(run_scenario(target, &scenario).await?)
}

pub async fn local_scaling_smoke_measurements(
) -> Result<(MeasurementEvidence, MeasurementEvidence), LocalTierError> {
    let binding = parse_local_scenario(SCALING_SCENARIO)?;
    let available = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(2)
        .max(1);
    let mut worker_counts = vec![1_usize];
    for candidate in binding
        .local
        .worker_counts
        .iter()
        .copied()
        .filter(|value| *value > 1)
    {
        if candidate <= available && !worker_counts.contains(&candidate) {
            worker_counts.push(candidate);
        }
    }
    if worker_counts.len() == 1 {
        worker_counts.push(2);
    }

    let key_count = binding.local.key_count.clamp(1, 256);
    let schedule = schedule_for(
        binding.scenario.seed,
        key_count,
        SMOKE_OPERATIONS,
        &binding.local,
    )
    .generate()
    .map_err(LocalTierError::Runtime)?;
    let workload = workload_from_schedule(
        &schedule,
        operation_mix(&binding.local),
        binding.local.payload_bytes,
    );
    let target_config = LocalTargetConfig {
        preload_entries: key_count.min(64),
        key_space: key_count,
        payload_bytes: usize::try_from(binding.local.payload_bytes)
            .map_err(|_| LocalTierError::Runtime("payload size does not fit usize".to_owned()))?,
        operation_mix: local_operation_mix(&binding.local)?,
        ..LocalTargetConfig::default()
    };
    let mut raw_by_workers = Vec::new();
    for workers in worker_counts {
        let mut samples = Vec::with_capacity(SMOKE_REPEATS);
        for _ in 0..SMOKE_REPEATS {
            samples.push(
                concurrent_throughput_sample(
                    workers,
                    Arc::new(schedule.keys.clone()),
                    target_config.clone(),
                )
                .await?,
            );
        }
        raw_by_workers.push((workers, samples));
    }
    let throughput_points = raw_by_workers
        .iter()
        .map(|(workers, samples)| {
            scalar_point(
                BTreeMap::from([(
                    "worker_threads".to_owned(),
                    DimensionValue::U64(*workers as u64),
                )]),
                "operations_per_second",
                samples.clone(),
            )
        })
        .collect();
    let baseline = raw_by_workers[0].1.clone();
    let efficiency_points = raw_by_workers
        .iter()
        .map(|(workers, samples)| {
            let ratios = samples
                .iter()
                .zip(&baseline)
                .map(|(sample, base)| sample / (base * *workers as f64))
                .collect();
            scalar_point(
                BTreeMap::from([(
                    "worker_threads".to_owned(),
                    DimensionValue::U64(*workers as u64),
                )]),
                "ratio",
                ratios,
            )
        })
        .collect();
    let scenario_digest = smoke_input_digest(
        &binding,
        &serde_json::json!({
            "operations": SMOKE_OPERATIONS,
            "repeats": SMOKE_REPEATS,
            "worker_counts": raw_by_workers.iter().map(|(workers, _)| *workers).collect::<Vec<_>>(),
            "key_count": key_count,
        }),
    );
    Ok((
        MeasurementEvidence::Scalar(ScalarEvidence {
            id: "local_cache_scaling_curve_1_to_n_threads".to_owned(),
            scenario_digest: scenario_digest.clone(),
            workload: workload.clone(),
            points: throughput_points,
            derived_from: vec![],
            max_robust_spread_ratio: SMOKE_SPREAD_LIMIT,
        }),
        MeasurementEvidence::Scalar(ScalarEvidence {
            id: "local_cache_scaling_efficiency_vs_one_thread".to_owned(),
            scenario_digest,
            workload,
            points: efficiency_points,
            derived_from: vec!["local_cache_scaling_curve_1_to_n_threads".to_owned()],
            max_robust_spread_ratio: SMOKE_SPREAD_LIMIT,
        }),
    ))
}

async fn concurrent_throughput_sample(
    workers: usize,
    schedule: Arc<Vec<u64>>,
    target_config: LocalTargetConfig,
) -> Result<f64, LocalTierError> {
    let operations = schedule.len() as u64;
    tokio::task::spawn_blocking(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(workers)
            .enable_all()
            .build()
            .map_err(|error| LocalTierError::Runtime(error.to_string()))?;
        runtime.block_on(async move {
            let target = Arc::new(LocalCacheTarget::new(target_config)?);
            target.reset().await?;
            target.preload().await?;
            let next = Arc::new(AtomicU64::new(0));
            let errors = Arc::new(AtomicU64::new(0));
            let started = Instant::now();
            let mut tasks = tokio::task::JoinSet::new();
            for _ in 0..workers {
                let target = Arc::clone(&target);
                let next = Arc::clone(&next);
                let errors = Arc::clone(&errors);
                let schedule = Arc::clone(&schedule);
                tasks.spawn(async move {
                    loop {
                        let sequence = next.fetch_add(1, Ordering::Relaxed);
                        if sequence >= operations {
                            break;
                        }
                        let logical_key = schedule[sequence as usize];
                        let operation = target.operation_for(sequence);
                        if target.execute_operation(operation, logical_key).await
                            != TargetOutcome::Success
                        {
                            errors.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });
            }
            while let Some(joined) = tasks.join_next().await {
                joined.map_err(|error| LocalTierError::Runtime(error.to_string()))?;
            }
            if errors.load(Ordering::Relaxed) != 0 {
                return Err(LocalTierError::Runtime(
                    "real local target returned an unsuccessful scaling operation".to_owned(),
                ));
            }
            Ok(throughput(operations, started.elapsed()))
        })
    })
    .await
    .map_err(|error| LocalTierError::Runtime(error.to_string()))?
}

pub async fn local_hot_key_smoke_measurement() -> Result<MeasurementEvidence, LocalTierError> {
    let binding = parse_local_scenario(HOT_KEY_SCENARIO)?;
    let target = Arc::new(LocalCacheTarget::new(LocalTargetConfig {
        preload_entries: 0,
        key_space: binding.local.key_count,
        payload_bytes: usize::try_from(binding.local.payload_bytes)
            .map_err(|_| LocalTierError::Runtime("payload size does not fit usize".to_owned()))?,
        operation_mix: LocalOperationMix {
            hit_percent: 0,
            miss_percent: 0,
            loader_percent: 0,
            put_percent: 0,
            hot_key_percent: 100,
        },
        loader_delay: Duration::from_micros(binding.local.loader_delay_us),
        ..LocalTargetConfig::default()
    })?);
    let scenario = smoke_scenario(&binding, vec![500, 2_000], 60, 0, 500_000)?;
    let scenario_digest = smoke_input_digest(
        &binding,
        &serde_json::to_value(&scenario)
            .map_err(|error| LocalTierError::Runtime(error.to_string()))?,
    );
    let criteria = scenario.sustainability_criteria();
    let knee = run_scenario(target, &scenario).await?;
    let schedule = KeyScheduleSpec::uniform(binding.scenario.seed, binding.local.key_count, 60)
        .generate()
        .map_err(LocalTierError::Runtime)?;
    Ok(MeasurementEvidence::LoadCurve(LoadCurveEvidence {
        id: "hot_key_contention_throughput_floor".to_owned(),
        scenario_digest,
        dimensions: BTreeMap::from([("logical_key_count".to_owned(), DimensionValue::U64(1))]),
        workload: workload_from_schedule(
            &schedule,
            operation_mix(&binding.local),
            binding.local.payload_bytes,
        ),
        criteria: Some(criteria),
        knee: Some(knee),
        claim: LoadClaim::CapacityKnee,
    }))
}

/// Exercise a synchronized cold-miss burst and prove that one loader execution
/// serves every concurrent request for the hot key.
pub async fn local_hot_key_single_flight_smoke_measurement(
) -> Result<MeasurementEvidence, LocalTierError> {
    let binding = parse_local_scenario(HOT_KEY_SCENARIO)?;
    let workers = binding
        .local
        .worker_counts
        .iter()
        .copied()
        .filter(|workers| *workers > 1)
        .max()
        .unwrap_or(2)
        .min(32);
    let mut samples = Vec::with_capacity(SMOKE_REPEATS);
    for _ in 0..SMOKE_REPEATS {
        let target = Arc::new(LocalCacheTarget::new(LocalTargetConfig {
            preload_entries: 0,
            key_space: 1,
            payload_bytes: usize::try_from(binding.local.payload_bytes).map_err(|_| {
                LocalTierError::Runtime("payload size does not fit usize".to_owned())
            })?,
            operation_mix: LocalOperationMix {
                hit_percent: 0,
                miss_percent: 0,
                loader_percent: 0,
                put_percent: 0,
                hot_key_percent: 100,
            },
            loader_delay: Duration::from_micros(binding.local.loader_delay_us.max(1)),
            hot_key_expected_miss_waiters: Some(workers as u64),
            ..LocalTargetConfig::default()
        })?);
        target.reset().await?;
        let barrier = Arc::new(tokio::sync::Barrier::new(workers + 1));
        let mut tasks = tokio::task::JoinSet::new();
        for sequence in 0..workers {
            let target = Arc::clone(&target);
            let barrier = Arc::clone(&barrier);
            tasks.spawn(async move {
                barrier.wait().await;
                target
                    .execute_operation(LocalOperation::HotKeyLoader, sequence as u64)
                    .await
            });
        }
        let started = Instant::now();
        barrier.wait().await;
        while let Some(joined) = tasks.join_next().await {
            let outcome = joined.map_err(|error| LocalTierError::Runtime(error.to_string()))?;
            ensure_success(outcome, "single-flight hot-key miss")?;
        }
        let snapshot = target.snapshot().await;
        if snapshot.operations.loader_executions != 1
            || snapshot.operations.hot_key_loaders != workers as u64
            || snapshot.diagnostics.stats.misses != workers as u64
            || snapshot.diagnostics.stats.hits != 0
            || snapshot.diagnostics.stats.loads != 1
        {
            return Err(LocalTierError::Runtime(format!(
                "single-flight proof expected {workers} concurrent misses, zero hits, and one load; got {snapshot:?}"
            )));
        }
        samples.push(throughput(workers as u64, started.elapsed()));
    }
    let schedule = KeyScheduleSpec::uniform(binding.scenario.seed, 1, workers as u64)
        .generate()
        .map_err(LocalTierError::Runtime)?;
    Ok(MeasurementEvidence::Scalar(ScalarEvidence {
        id: "hot_key_single_flight_miss_stampede_cost".to_owned(),
        scenario_digest: smoke_input_digest(
            &binding,
            &serde_json::json!({
                "mode": "synchronized-cold-miss-burst",
                "workers": workers,
                "loader_delay_us": binding.local.loader_delay_us.max(1),
                "repeats": SMOKE_REPEATS,
            }),
        ),
        workload: workload_from_schedule(
            &schedule,
            operation_mix(&binding.local),
            binding.local.payload_bytes,
        ),
        points: vec![scalar_point(
            BTreeMap::from([
                (
                    "concurrent_requests".to_owned(),
                    DimensionValue::U64(workers as u64),
                ),
                ("loader_executions".to_owned(), DimensionValue::U64(1)),
                (
                    "cache_misses_before_loader_release".to_owned(),
                    DimensionValue::U64(workers as u64),
                ),
                ("cache_hits".to_owned(), DimensionValue::U64(0)),
            ]),
            "operations_per_second",
            samples,
        )],
        derived_from: vec![],
        max_robust_spread_ratio: SMOKE_SPREAD_LIMIT,
    }))
}

pub async fn local_capacity_smoke_measurement() -> Result<MeasurementEvidence, LocalTierError> {
    let binding = parse_local_scenario(CAPACITY_SCENARIO)?;
    let key_count = binding.local.key_count.clamp(1, 64);
    let uniform = KeyScheduleSpec::uniform(binding.scenario.seed, key_count, SMOKE_OPERATIONS)
        .generate()
        .map_err(LocalTierError::Runtime)?;
    let theta = binding.local.zipfian_theta.ok_or_else(|| {
        LocalTierError::Runtime("capacity scenario requires zipfian_theta".to_owned())
    })?;
    let zipfian =
        KeyScheduleSpec::zipfian(binding.scenario.seed, key_count, SMOKE_OPERATIONS, theta)
            .generate()
            .map_err(LocalTierError::Runtime)?;
    let payload_bytes = usize::try_from(binding.local.payload_bytes)
        .map_err(|_| LocalTierError::Runtime("payload size does not fit usize".to_owned()))?;
    let declared_full = binding.local.full_capacity_bytes.ok_or_else(|| {
        LocalTierError::Runtime("capacity scenario requires full_capacity_bytes".to_owned())
    })?;
    let declared_half = binding.local.half_capacity_bytes.ok_or_else(|| {
        LocalTierError::Runtime("capacity scenario requires half_capacity_bytes".to_owned())
    })?;
    if declared_half.saturating_mul(2) != declared_full {
        return Err(LocalTierError::Runtime(
            "capacity scenario half/full byte contracts are inconsistent".to_owned(),
        ));
    }
    // Smoke preserves the committed 2:1 matrix while bounding setup work. The
    // effective capacities are part of the smoke input digest and dimensions.
    let capacities = [("half", 2 * 1024_u64), ("full", 4 * 1024_u64)];
    let max_probe = binding.scenario.preload_operations.clamp(32, 256);
    let mut points = Vec::new();
    for (distribution, schedule) in [("uniform", &uniform), ("zipfian", &zipfian)] {
        for (capacity_profile, capacity_bytes) in capacities {
            let preload_entries =
                discover_full_preload_entries(capacity_bytes, payload_bytes, max_probe).await?;
            for _ in 0..SMOKE_REPEATS {
                verify_each_pressure_insert_evicts(
                    capacity_bytes,
                    payload_bytes,
                    preload_entries,
                    schedule,
                )
                .await?;
            }
            let mut samples = Vec::with_capacity(SMOKE_REPEATS);
            for _ in 0..SMOKE_REPEATS {
                let target = LocalCacheTarget::new(capacity_target_config(
                    capacity_bytes,
                    payload_bytes,
                    preload_entries,
                    schedule.spec.key_count,
                ))?;
                target.reset().await?;
                target.preload().await?;
                let started = Instant::now();
                for (index, logical_key) in schedule.keys.iter().enumerate() {
                    let _ = target.observe_preload_key(*logical_key).await?;
                    let sequence = index as u64;
                    ensure_success(
                        target
                            .execute_operation(LocalOperation::CapacityPressure, sequence)
                            .await,
                        "capacity pressure",
                    )?;
                }
                samples.push(throughput(schedule.keys.len() as u64, started.elapsed()));
            }
            points.push(scalar_point(
                BTreeMap::from([
                    (
                        "distribution".to_owned(),
                        DimensionValue::Text(distribution.to_owned()),
                    ),
                    (
                        "capacity_profile".to_owned(),
                        DimensionValue::Text(capacity_profile.to_owned()),
                    ),
                    (
                        "capacity_bytes".to_owned(),
                        DimensionValue::U64(capacity_bytes),
                    ),
                    (
                        "verified_full_preload_entries".to_owned(),
                        DimensionValue::U64(preload_entries),
                    ),
                    (
                        "every_insert_evicts_proof".to_owned(),
                        DimensionValue::Bool(true),
                    ),
                    (
                        "eviction_proof_operations_per_repeat".to_owned(),
                        DimensionValue::U64(schedule.keys.len() as u64),
                    ),
                    (
                        "eviction_proof_repeats".to_owned(),
                        DimensionValue::U64(SMOKE_REPEATS as u64),
                    ),
                    (
                        "eviction_proof_scope".to_owned(),
                        DimensionValue::Text("untimed-identical-config-and-schedule".to_owned()),
                    ),
                ]),
                "operations_per_second",
                samples,
            ));
        }
    }
    Ok(MeasurementEvidence::Scalar(ScalarEvidence {
        id: "throughput_at_full_capacity_vs_half_capacity".to_owned(),
        scenario_digest: smoke_input_digest(
            &binding,
            &serde_json::json!({
                "operations": SMOKE_OPERATIONS,
                "repeats": SMOKE_REPEATS,
                "key_count": key_count,
                "capacity_bytes": capacities,
                "max_fullness_probe_entries": max_probe,
            }),
        ),
        workload: matrix_workload(&uniform, &zipfian, &binding.local),
        points,
        derived_from: vec![],
        max_robust_spread_ratio: SMOKE_SPREAD_LIMIT,
    }))
}

fn capacity_target_config(
    capacity_bytes: u64,
    payload_bytes: usize,
    preload_entries: u64,
    key_space: u64,
) -> LocalTargetConfig {
    LocalTargetConfig {
        max_capacity: capacity_bytes,
        max_entry_bytes: payload_bytes.saturating_mul(4).max(1),
        preload_entries,
        key_space,
        payload_bytes,
        operation_mix: LocalOperationMix {
            hit_percent: 0,
            miss_percent: 0,
            loader_percent: 0,
            put_percent: 0,
            hot_key_percent: 100,
        },
        loader_delay: Duration::ZERO,
        hot_key_expected_miss_waiters: None,
        capacity_pressure_every: None,
        injected_capacity_pressure_delay: Duration::ZERO,
    }
}

async fn preload_fits_capacity(
    capacity_bytes: u64,
    payload_bytes: usize,
    entries: u64,
) -> Result<bool, LocalTierError> {
    let target = LocalCacheTarget::new(capacity_target_config(
        capacity_bytes,
        payload_bytes,
        entries,
        entries.max(1),
    ))?;
    target.reset().await?;
    match target.preload().await {
        Ok(_) => Ok(true),
        Err(TargetError::Preload(_)) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

async fn discover_full_preload_entries(
    capacity_bytes: u64,
    payload_bytes: usize,
    max_probe: u64,
) -> Result<u64, LocalTierError> {
    let mut fits = 0_u64;
    let mut fails = 1_u64;
    while fails <= max_probe && preload_fits_capacity(capacity_bytes, payload_bytes, fails).await? {
        fits = fails;
        fails = fails.saturating_mul(2);
    }
    if fails > max_probe {
        fails = max_probe;
        if preload_fits_capacity(capacity_bytes, payload_bytes, fails).await? {
            return Err(LocalTierError::Runtime(format!(
                "fullness probe did not reach eviction for capacity={capacity_bytes}, payload={payload_bytes}, max_probe={max_probe}"
            )));
        }
    }
    while fits.saturating_add(1) < fails {
        let midpoint = fits + (fails - fits) / 2;
        if preload_fits_capacity(capacity_bytes, payload_bytes, midpoint).await? {
            fits = midpoint;
        } else {
            fails = midpoint;
        }
    }
    if fits == 0 {
        return Err(LocalTierError::Runtime(format!(
            "capacity={capacity_bytes} cannot hold one encoded {payload_bytes}-byte value"
        )));
    }
    Ok(fits)
}

async fn verify_each_pressure_insert_evicts(
    capacity_bytes: u64,
    payload_bytes: usize,
    preload_entries: u64,
    schedule: &GeneratedKeySchedule,
) -> Result<(), LocalTierError> {
    let target = LocalCacheTarget::new(capacity_target_config(
        capacity_bytes,
        payload_bytes,
        preload_entries,
        schedule.spec.key_count,
    ))?;
    target.reset().await?;
    target.preload().await?;
    let initial_entries = target.snapshot().await.diagnostics.estimated_entries;
    if initial_entries != preload_entries {
        return Err(LocalTierError::Runtime(format!(
            "fullness proof expected {preload_entries} entries, observed {initial_entries}"
        )));
    }
    let mut previous_entries = initial_entries;
    for (index, logical_key) in schedule.keys.iter().enumerate() {
        let _ = target.observe_preload_key(*logical_key).await?;
        let sequence = index as u64;
        ensure_success(
            target
                .execute_operation(LocalOperation::CapacityPressure, sequence)
                .await,
            "capacity eviction proof",
        )?;
        if !target.capacity_pressure_key_present(sequence).await? {
            return Err(LocalTierError::Runtime(format!(
                "capacity-pressure candidate {sequence} was not admitted"
            )));
        }
        let entries = target.snapshot().await.diagnostics.estimated_entries;
        if entries > previous_entries {
            return Err(LocalTierError::Runtime(format!(
                "capacity-pressure insert {index} grew the cache from {previous_entries} to {entries} entries without evicting a prior entry"
            )));
        }
        previous_entries = entries;
    }
    Ok(())
}

pub async fn local_path_cost_smoke_measurement() -> Result<MeasurementEvidence, LocalTierError> {
    let binding = parse_local_scenario(PATH_SCENARIO)?;
    let key_count = binding.local.key_count.clamp(1, 64);
    let schedule = schedule_for(
        binding.scenario.seed,
        key_count,
        SMOKE_OPERATIONS,
        &binding.local,
    )
    .generate()
    .map_err(LocalTierError::Runtime)?;
    let payload_bytes = usize::try_from(binding.local.payload_bytes)
        .map_err(|_| LocalTierError::Runtime("payload size does not fit usize".to_owned()))?;
    let paths = [
        ("hit", LocalOperation::Hit),
        ("miss", LocalOperation::Miss),
        ("loader", LocalOperation::Loader),
    ];
    let mut points = Vec::new();
    for (name, operation) in paths {
        let mut samples = Vec::with_capacity(SMOKE_REPEATS);
        for _ in 0..SMOKE_REPEATS {
            let target = LocalCacheTarget::new(LocalTargetConfig {
                preload_entries: key_count,
                key_space: key_count,
                payload_bytes,
                ..LocalTargetConfig::default()
            })?;
            target.reset().await?;
            target.preload().await?;
            let started = Instant::now();
            for (index, logical_key) in schedule.keys.iter().enumerate() {
                let sequence = if operation == LocalOperation::Loader {
                    index as u64
                } else {
                    *logical_key
                };
                ensure_success(target.execute_operation(operation, sequence).await, name)?;
            }
            samples.push(throughput(schedule.keys.len() as u64, started.elapsed()));
        }
        points.push(scalar_point(
            BTreeMap::from([("path".to_owned(), DimensionValue::Text(name.to_owned()))]),
            "operations_per_second",
            samples,
        ));
    }
    Ok(MeasurementEvidence::Scalar(ScalarEvidence {
        id: "hit_miss_and_loader_path_cost_breakdown".to_owned(),
        scenario_digest: smoke_input_digest(
            &binding,
            &serde_json::json!({
                "operations": SMOKE_OPERATIONS,
                "repeats": SMOKE_REPEATS,
                "key_count": key_count,
            }),
        ),
        workload: workload_from_schedule(
            &schedule,
            operation_mix(&binding.local),
            binding.local.payload_bytes,
        ),
        points,
        derived_from: vec![],
        max_robust_spread_ratio: SMOKE_SPREAD_LIMIT,
    }))
}

pub async fn local_allocation_smoke_measurement() -> Result<MeasurementEvidence, LocalTierError> {
    let input: AllocationScenarioInput = parse_toml(ALLOCATION_SCENARIO)?;
    validate_allocation_input(&input)?;
    let operations = input.operations.min(100);
    let repeats = input.repeats.min(SMOKE_REPEATS);
    let input_payload_bytes = input.payload_bytes;
    let schedule = KeyScheduleSpec::uniform(input.seed, operations, operations)
        .generate()
        .map_err(LocalTierError::Runtime)?;
    let scheduled_keys = Arc::new(schedule.keys.clone());
    let mut points = Vec::new();
    for feature in &input.features {
        let mut samples = Vec::with_capacity(repeats);
        for repeat in 0..repeats {
            let cache = HydraCache::local().max_capacity(16 * 1024 * 1024).build();
            let feature_name = feature.clone();
            let scheduled_keys = Arc::clone(&scheduled_keys);
            let (result, measured) = measure_allocations(operations, async move {
                for (sequence, logical_key) in scheduled_keys.iter().copied().enumerate() {
                    let options = match feature_name.as_str() {
                        "ttl" => CacheOptions::new().ttl(Duration::from_secs(60)),
                        "tags" => CacheOptions::new().tag(format!("feature:{repeat}")),
                        _ => CacheOptions::new(),
                    };
                    cache
                        .put(
                            &format!("w1:allocation:{repeat}:{logical_key}"),
                            vec![
                                sequence as u8;
                                usize::try_from(input_payload_bytes).map_err(|_| {
                                    "allocation payload size does not fit usize".to_owned()
                                })?
                            ],
                            options,
                        )
                        .await
                        .map_err(|error| error.to_string())?;
                }
                Ok::<(), String>(())
            })
            .await;
            result.map_err(LocalTierError::Runtime)?;
            samples.push(measured.gross_allocated_bytes_per_operation);
        }
        points.push(scalar_point(
            BTreeMap::from([("feature".to_owned(), DimensionValue::Text(feature.clone()))]),
            &input.metric,
            samples,
        ));
    }
    Ok(MeasurementEvidence::Scalar(ScalarEvidence {
        id: "bytes_allocated_per_operation_by_feature".to_owned(),
        scenario_digest: custom_smoke_input_digest(
            ALLOCATION_SCENARIO,
            &serde_json::json!({
                "operations": operations,
                "repeats": repeats,
                "payload_bytes": input.payload_bytes,
                "features": input.features,
                "metric": input.metric,
                "includes": input.includes,
            }),
        ),
        workload: workload_from_schedule(
            &schedule,
            vec![WeightedOperation {
                operation: "put".to_owned(),
                weight: 1.0,
            }],
            input.payload_bytes,
        ),
        points,
        derived_from: vec![],
        max_robust_spread_ratio: SMOKE_SPREAD_LIMIT,
    }))
}

pub async fn local_trace_replay_smoke_measurement() -> Result<MeasurementEvidence, LocalTierError> {
    let input: TraceScenarioInput = parse_toml(TRACE_SCENARIO)?;
    validate_trace_input(&input)?;
    let mut events = Vec::new();
    for name in &input.traces {
        let id = trace_id_by_name(name)?;
        events.extend(id.load().map_err(LocalTierError::Runtime)?.events);
    }
    let input_digest = trace_digest(&events);
    let cache = HydraCache::local().max_capacity(16 * 1024 * 1024).build();
    let mut hits = 0_u64;
    let mut misses = 0_u64;
    let mut replayed = Vec::with_capacity(events.len());
    for event in &events {
        match cache
            .get::<u64>(&event.key)
            .await
            .map_err(|error| LocalTierError::Runtime(error.to_string()))?
        {
            Some(_) => hits = hits.saturating_add(1),
            None => {
                misses = misses.saturating_add(1);
                cache
                    .put(&event.key, event.at, CacheOptions::new())
                    .await
                    .map_err(|error| LocalTierError::Runtime(error.to_string()))?;
            }
        }
        replayed.push(event.clone());
    }
    let replayed_digest = trace_digest(&replayed);
    Ok(MeasurementEvidence::TraceReplay(TraceReplayEvidence {
        id: "w22_trace_replay_preserves_order_and_records_trace_digest".to_owned(),
        scenario_digest: digest_bytes(TRACE_SCENARIO),
        catalog_id: format!("{}:{}", input.catalog, input.traces.join(",")),
        event_count: events.len() as u64,
        input_digest,
        replayed_digest,
        order_preserved: events == replayed,
        hits,
        misses,
    }))
}

fn smoke_scenario(
    binding: &BoundLocalScenario,
    rates: Vec<u64>,
    operations: u64,
    preload_operations: u64,
    p99_slo_us: u64,
) -> Result<Scenario, LocalTierError> {
    let mut scenario = binding.scenario.clone();
    scenario.id = format!("{}-smoke", scenario.id);
    scenario.offered_rates_per_second = rates;
    scenario.preload_operations = preload_operations;
    scenario.warmup_operations = 8;
    scenario.steady_operations = operations;
    scenario.repeats = SMOKE_REPEATS as u32;
    scenario.p99_slo_us = p99_slo_us;
    scenario.p999_slo_us = None;
    scenario.p999_min_samples = 1;
    scenario.min_achieved_ratio = 0.50;
    scenario.robust_spread_tolerance = SMOKE_SPREAD_LIMIT;
    scenario.validate().map_err(LocalTierError::Runtime)?;
    Ok(scenario)
}

fn parse_local_scenario(source: &[u8]) -> Result<BoundLocalScenario, LocalTierError> {
    let text =
        std::str::from_utf8(source).map_err(|error| LocalTierError::Runtime(error.to_string()))?;
    let mut root = text
        .parse::<toml::Table>()
        .map_err(|error| LocalTierError::Runtime(error.to_string()))?;
    let local_value = root.remove("local").ok_or_else(|| {
        LocalTierError::Runtime("local performance scenario has no [local] contract".to_owned())
    })?;
    let local: LocalScenarioInputs = local_value
        .try_into()
        .map_err(|error| LocalTierError::Runtime(error.to_string()))?;
    validate_local_inputs(&local)?;
    let scenario: Scenario = toml::Value::Table(root)
        .try_into()
        .map_err(|error| LocalTierError::Runtime(error.to_string()))?;
    scenario.validate().map_err(LocalTierError::Runtime)?;
    Ok(BoundLocalScenario {
        scenario,
        local,
        source_digest: digest_bytes(source),
    })
}

fn validate_local_inputs(local: &LocalScenarioInputs) -> Result<(), LocalTierError> {
    if local.key_count == 0
        || local.payload_bytes == 0
        || local.distribution.trim().is_empty()
        || local.operation_mix.is_empty()
        || local.operation_mix.iter().any(|operation| {
            operation.operation.trim().is_empty()
                || !operation.weight.is_finite()
                || operation.weight <= 0.0
        })
        || local.worker_counts.contains(&0)
        || local
            .zipfian_theta
            .is_some_and(|theta| !theta.is_finite() || theta <= 0.0)
    {
        return Err(LocalTierError::Runtime(
            "local scenario workload contract is incomplete".to_owned(),
        ));
    }
    let total = local
        .operation_mix
        .iter()
        .map(|operation| operation.weight)
        .sum::<f64>();
    if (total - 1.0).abs() > 1e-9 {
        return Err(LocalTierError::Runtime(format!(
            "local scenario operation weights must total 1.0, got {total}"
        )));
    }
    Ok(())
}

fn parse_toml<T>(source: &[u8]) -> Result<T, LocalTierError>
where
    T: for<'de> Deserialize<'de>,
{
    let text =
        std::str::from_utf8(source).map_err(|error| LocalTierError::Runtime(error.to_string()))?;
    toml::from_str(text).map_err(|error| LocalTierError::Runtime(error.to_string()))
}

fn validate_allocation_input(input: &AllocationScenarioInput) -> Result<(), LocalTierError> {
    if input.schema_version != 1
        || input.id.trim().is_empty()
        || input.operations == 0
        || input.payload_bytes == 0
        || input.repeats < 3
        || input.features != ["baseline", "ttl", "tags"]
        || input.metric != "gross_allocated_bytes_per_operation"
        || !input.robust_spread_tolerance.is_finite()
        || input.robust_spread_tolerance < 0.0
        || !input.includes.iter().any(|value| value == "real_api")
        || !input
            .includes
            .iter()
            .any(|value| value == "runtime_overhead")
    {
        return Err(LocalTierError::Runtime(
            "allocation scenario contract is incomplete".to_owned(),
        ));
    }
    Ok(())
}

fn validate_trace_input(input: &TraceScenarioInput) -> Result<(), LocalTierError> {
    if input.schema_version != 1
        || input.id.trim().is_empty()
        || input.catalog != "w22-v1"
        || input.traces.is_empty()
        || input.mode != "quality_fixture_not_capacity"
        || !input.require_order_preserved
        || !input.require_input_replay_digest_match
    {
        return Err(LocalTierError::Runtime(
            "trace replay scenario contract is incomplete".to_owned(),
        ));
    }
    for trace in &input.traces {
        trace_id_by_name(trace)?;
    }
    Ok(())
}

fn trace_id_by_name(name: &str) -> Result<TraceCatalogId, LocalTierError> {
    TraceCatalogId::ALL
        .into_iter()
        .find(|id| id.as_str() == name)
        .ok_or_else(|| LocalTierError::Runtime(format!("unknown W22 trace id {name}")))
}

fn operation_mix(local: &LocalScenarioInputs) -> Vec<WeightedOperation> {
    local
        .operation_mix
        .iter()
        .map(|operation| WeightedOperation {
            operation: operation.operation.clone(),
            weight: operation.weight,
        })
        .collect()
}

fn local_operation_mix(local: &LocalScenarioInputs) -> Result<LocalOperationMix, LocalTierError> {
    let mut mix = LocalOperationMix {
        hit_percent: 0,
        miss_percent: 0,
        loader_percent: 0,
        put_percent: 0,
        hot_key_percent: 0,
    };
    for operation in &local.operation_mix {
        let percent = (operation.weight * 100.0).round() as u8;
        match operation.operation.as_str() {
            "get-hit" => mix.hit_percent = percent,
            "get-miss" => mix.miss_percent = percent,
            "load" => mix.loader_percent = percent,
            "put" => mix.put_percent = percent,
            "hot-key-get-or-insert" => mix.hot_key_percent = percent,
            other => {
                return Err(LocalTierError::Runtime(format!(
                    "operation {other} cannot drive LocalCacheTarget"
                )))
            }
        }
    }
    if mix.total_percent() != 100 {
        return Err(LocalTierError::Runtime(format!(
            "rounded local operation mix totals {} instead of 100",
            mix.total_percent()
        )));
    }
    Ok(mix)
}

fn schedule_for(
    seed: u64,
    key_count: u64,
    operations: u64,
    local: &LocalScenarioInputs,
) -> KeyScheduleSpec {
    match local.distribution.as_str() {
        "zipfian" => KeyScheduleSpec::zipfian(
            seed,
            key_count,
            operations,
            local.zipfian_theta.unwrap_or(0.99),
        ),
        _ => KeyScheduleSpec::uniform(seed, key_count, operations),
    }
}

fn smoke_input_digest(binding: &BoundLocalScenario, effective: &serde_json::Value) -> String {
    let local = serde_json::to_vec(&binding.local)
        .expect("validated local scenario inputs must serialize to JSON");
    let effective =
        serde_json::to_vec(effective).expect("effective smoke inputs must serialize to JSON");
    digest_parts(&[
        binding.source_digest.as_bytes(),
        b"hydracache-local-smoke-input-v1",
        &local,
        &effective,
    ])
}

fn custom_smoke_input_digest(source: &[u8], effective: &serde_json::Value) -> String {
    let effective =
        serde_json::to_vec(effective).expect("effective smoke inputs must serialize to JSON");
    digest_parts(&[
        digest_bytes(source).as_bytes(),
        b"hydracache-local-smoke-input-v1",
        &effective,
    ])
}

fn workload_from_schedule(
    schedule: &GeneratedKeySchedule,
    operation_mix: Vec<WeightedOperation>,
    payload_bytes: u64,
) -> WorkloadIdentity {
    let (kind, theta) = match schedule.spec.distribution {
        KeyDistribution::Uniform => ("uniform", None),
        KeyDistribution::Zipfian { theta } => ("zipfian", Some(theta)),
    };
    let payload_mix = vec![WeightedPayload {
        bytes: payload_bytes,
        weight: 1.0,
    }];
    let operation_bytes =
        serde_json::to_vec(&operation_mix).expect("validated operation mix must serialize to JSON");
    let payload_bytes_encoded =
        serde_json::to_vec(&payload_mix).expect("validated payload mix must serialize to JSON");
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
            b"hydracache-loadgen-workload-v1",
            &operation_bytes,
            &payload_bytes_encoded,
        ]),
    }
}

fn matrix_workload(
    uniform: &GeneratedKeySchedule,
    zipfian: &GeneratedKeySchedule,
    local: &LocalScenarioInputs,
) -> WorkloadIdentity {
    let operation_mix = operation_mix(local);
    let payload_mix = vec![WeightedPayload {
        bytes: local.payload_bytes,
        weight: 1.0,
    }];
    let operation_bytes =
        serde_json::to_vec(&operation_mix).expect("validated operation mix must serialize to JSON");
    let payload_bytes =
        serde_json::to_vec(&payload_mix).expect("validated payload mix must serialize to JSON");
    WorkloadIdentity {
        generator: "hydracache-cache-sim-key-schedule-matrix".to_owned(),
        generator_version: KEY_SCHEDULE_GENERATOR_VERSION.to_string(),
        seed: Some(uniform.spec.seed),
        key_distribution: Some(KeyDistributionIdentity {
            kind: "uniform_and_zipfian".to_owned(),
            theta: Some(0.99),
        }),
        key_count: Some(uniform.spec.key_count),
        operation_mix,
        payload_mix,
        digest: digest_parts(&[
            uniform.digest.as_bytes(),
            zipfian.digest.as_bytes(),
            b"hydracache-loadgen-workload-matrix-v1",
            &operation_bytes,
            &payload_bytes,
        ]),
    }
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

fn ensure_success(outcome: TargetOutcome, path: &str) -> Result<(), LocalTierError> {
    if outcome == TargetOutcome::Success {
        Ok(())
    } else {
        Err(LocalTierError::Runtime(format!(
            "real local-cache {path} path returned {outcome:?}"
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
        fingerprint: "smoke-local".to_owned(),
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
    fn committed_local_fields_control_scenario_and_workload_identity() {
        let original = std::str::from_utf8(SCALING_SCENARIO).unwrap();
        let changed = original
            .replace("seed = 6701", "seed = 9999")
            .replace("key_count = 10000", "key_count = 7");
        let binding = parse_local_scenario(changed.as_bytes()).unwrap();
        assert_eq!(binding.scenario.seed, 9999);
        assert_eq!(binding.local.key_count, 7);

        let schedule = schedule_for(9999, 7, 16, &binding.local)
            .generate()
            .unwrap();
        let first = workload_from_schedule(
            &schedule,
            operation_mix(&binding.local),
            binding.local.payload_bytes,
        );
        let mut changed_mix = operation_mix(&binding.local);
        changed_mix[0].weight = 0.69;
        let second = workload_from_schedule(&schedule, changed_mix, binding.local.payload_bytes);
        let third = workload_from_schedule(
            &schedule,
            operation_mix(&binding.local),
            binding.local.payload_bytes + 1,
        );
        assert_ne!(first.digest, second.digest);
        assert_ne!(first.digest, third.digest);
    }

    #[test]
    fn reference_profile_is_never_a_smoke_profile_alias() {
        let fingerprint = smoke_fingerprint();
        let profile = smoke_profile("smoke-v1", &fingerprint);
        assert_eq!(profile.name, "smoke-v1");
        assert_ne!(profile.name, "reference-v1");
    }
}
