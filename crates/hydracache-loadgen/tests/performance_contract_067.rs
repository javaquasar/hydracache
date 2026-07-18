use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use hydracache_loadgen::histogram::LatencySummary;
use hydracache_loadgen::{
    run_open_loop, run_phases, run_scenario, BuildIdentity, ComparisonEvidence, DimensionValue,
    ErrorBudgets, EvidenceRunMode, FixedRateSchedule, LatencyHistogram, LoadClaim,
    LoadCurveEvidence, MeasurementEvidence, OpenLoopConfig, OpenLoopObservation, PerfReport,
    PerformanceProfile, PhaseAccounting, PhaseConfig, Quantity, RatePointEvidence, RepeatEvidence,
    RunnerFingerprint, ScalarEvidence, ScalarPoint, Scenario, SourceIdentity, SurfaceIdentity,
    SustainabilityCriteria, Target, TargetError, TargetOutcome, TargetRequest, WeightedOperation,
    WeightedPayload, WorkloadIdentity,
};

fn latency(p99_us: u64) -> LatencySummary {
    LatencySummary {
        samples: 10_000,
        p50_us: Some(p99_us / 2),
        p90_us: Some(p99_us.saturating_sub(1)),
        p99_us: Some(p99_us),
        p999_us: Some(p99_us),
        p999_min_samples: 1_000,
        p999_reportable: true,
        max_us: Some(p99_us),
        overflow_count: 0,
    }
}

fn observation(rate: f64, achieved: f64, p99_us: u64) -> OpenLoopObservation {
    OpenLoopObservation {
        offered: 10_000,
        started: 10_000,
        completed: 10_000,
        successes: 10_000,
        errors: 0,
        timeouts: 0,
        rejections: 0,
        backlog_high_water: 1,
        backlog_drained: true,
        drain_ms: 10,
        elapsed_ms: 100_000,
        offered_rate_per_second: rate,
        achieved_rate_per_second: achieved,
        latency: latency(p99_us),
    }
}

fn repeats(rate: f64, achieved: f64, p99_us: u64) -> Vec<RepeatEvidence> {
    (0..3)
        .map(|_| RepeatEvidence {
            reset_state_digest: "reset-sha".to_owned(),
            preloaded_state_digest: "preload-sha".to_owned(),
            state_digest: "state-sha".to_owned(),
            phase: PhaseAccounting {
                reset_operations: 1,
                preload_operations: 0,
                warmup_operations: 5,
                steady_operations: 10_000,
                reset_ms: 1,
                preload_ms: 0,
                warmup_ms: 1,
                warmup_successes: 5,
                warmup_errors: 0,
                warmup_timeouts: 0,
                warmup_rejections: 0,
                steady_ms: 100_000,
                warmup_samples_in_steady_histogram: 0,
            },
            steady: observation(rate, achieved, p99_us),
        })
        .collect()
}

fn point(rate: f64, achieved: f64, p99_us: u64) -> RatePointEvidence {
    criteria().evaluate_repeats(rate, repeats(rate, achieved, p99_us))
}

fn criteria() -> SustainabilityCriteria {
    SustainabilityCriteria {
        p99_slo_us: 10,
        p999_slo_us: Some(15),
        min_achieved_ratio: 0.95,
        max_error_ratio: 0.001,
        max_timeout_ratio: 0.001,
        max_rejection_ratio: 0.01,
        max_drain_ms: 100,
        max_robust_spread_ratio: 0.10,
    }
}

#[test]
fn open_loop_scheduler_accounts_missed_ticks_as_latency_not_skips() {
    let mut schedule = FixedRateSchedule::new(1_000_000, 1_000).unwrap();
    let due = schedule.due_ticks(6_000_000);
    assert_eq!(due.len(), 6, "every delayed offer must remain represented");
    assert_eq!(due[0].scheduled_ns, 1_000_000);
    assert_eq!(due[5].scheduled_ns, 6_000_000);
    assert_eq!(schedule.interval_ns(), 1_000_000);
    assert!(schedule.due_ticks(6_999_999).is_empty());
    assert_eq!(schedule.due_ticks(7_000_000)[0].sequence, 6);
}

#[test]
fn histogram_percentiles_match_reference_values_on_known_distributions() {
    let mut histogram = LatencyHistogram::new(Duration::from_secs(1), 3).unwrap();
    for value in 1..=1_000 {
        histogram.record_us(value);
    }
    let summary = histogram.summary(1_000);
    assert!(summary.p50_us.unwrap().abs_diff(500) <= 2, "{summary:?}");
    assert!(summary.p90_us.unwrap().abs_diff(900) <= 2, "{summary:?}");
    assert!(summary.p99_us.unwrap().abs_diff(990) <= 2, "{summary:?}");
    assert!(summary.p999_reportable);
}

#[test]
fn knee_search_finds_the_stated_knee_on_a_synthetic_latency_model() {
    let result = criteria().find_knee(vec![
        point(100.0, 100.0, 5),
        point(200.0, 198.0, 8),
        point(300.0, 299.0, 20),
    ]);
    assert_eq!(result.sustainable_rate_per_second, Some(200.0));
}

#[test]
fn knee_rejects_rate_when_latency_passes_but_achieved_rate_lags() {
    let verdict = point(1_000.0, 500.0, 5).verdict;
    assert!(!verdict.sustainable);
    assert!(verdict
        .reasons
        .iter()
        .any(|reason| reason.contains("achieved/offered")));
}

#[test]
fn knee_rejects_timeout_rejection_budget_or_undrained_backlog() {
    let mut timeout = repeats(1_000.0, 1_000.0, 5);
    timeout[0].steady.timeouts = 20;
    timeout[0].steady.successes -= 20;
    assert!(
        !criteria()
            .evaluate_repeats(1_000.0, timeout)
            .verdict
            .sustainable
    );

    let mut rejected = repeats(1_000.0, 1_000.0, 5);
    rejected[0].steady.rejections = 200;
    rejected[0].steady.successes -= 200;
    assert!(
        !criteria()
            .evaluate_repeats(1_000.0, rejected)
            .verdict
            .sustainable
    );

    let mut queued = repeats(1_000.0, 1_000.0, 5);
    queued[0].steady.backlog_drained = false;
    assert!(
        !criteria()
            .evaluate_repeats(1_000.0, queued)
            .verdict
            .sustainable
    );
}

#[test]
fn knee_rejects_corrupt_counts_overflow_and_missing_required_p999() {
    let mut corrupt = repeats(1_000.0, 1_000.0, 5);
    corrupt[0].steady.completed -= 1;
    assert!(
        !criteria()
            .evaluate_repeats(1_000.0, corrupt)
            .verdict
            .sustainable
    );

    let mut overflowed = repeats(1_000.0, 1_000.0, 5);
    overflowed[0].steady.latency.overflow_count = 1;
    assert!(
        !criteria()
            .evaluate_repeats(1_000.0, overflowed)
            .verdict
            .sustainable
    );

    let mut insufficient = repeats(1_000.0, 1_000.0, 5);
    insufficient[0].steady.latency.p999_reportable = false;
    insufficient[0].steady.latency.p999_us = None;
    assert!(
        !criteria()
            .evaluate_repeats(1_000.0, insufficient)
            .verdict
            .sustainable
    );
}

#[test]
fn knee_rejects_invalid_direct_criteria_and_forged_verdicts() {
    let mut invalid = criteria();
    invalid.max_error_ratio = f64::NAN;
    assert!(
        !invalid
            .evaluate_repeats(100.0, repeats(100.0, 100.0, 5))
            .verdict
            .sustainable
    );

    let mut knee = criteria().find_knee(vec![point(100.0, 100.0, 5)]);
    knee.evaluated[0].verdict.sustainable = false;
    knee.evaluated[0].verdict.reasons = vec!["forged".to_owned()];
    assert!(!criteria().knee_validation_problems(&knee).is_empty());
}

#[test]
fn p999_is_unreportable_below_the_declared_sample_count() {
    let mut histogram = LatencyHistogram::new(Duration::from_secs(1), 3).unwrap();
    for value in 1..=99 {
        histogram.record_us(value);
    }
    let summary = histogram.summary(100);
    assert!(!summary.p999_reportable);
    assert_eq!(summary.p999_us, None);
    assert_eq!(summary.samples, 99);
}

#[derive(Default)]
struct CountingTarget {
    calls: AtomicU64,
    resets: AtomicU64,
}

#[async_trait]
impl Target for CountingTarget {
    async fn reset(&self) -> Result<String, TargetError> {
        self.calls.store(0, Ordering::SeqCst);
        self.resets.fetch_add(1, Ordering::SeqCst);
        Ok("state:empty:v1".to_owned())
    }

    async fn execute(&self, _request: TargetRequest) -> TargetOutcome {
        self.calls.fetch_add(1, Ordering::SeqCst);
        TargetOutcome::Success
    }

    async fn state_digest(&self) -> Result<String, TargetError> {
        Ok(format!(
            "state:calls:{}:v1",
            self.calls.load(Ordering::SeqCst)
        ))
    }
}

fn phase_config() -> PhaseConfig {
    PhaseConfig {
        preload_operations: 0,
        warmup_operations: 5,
        steady: OpenLoopConfig {
            offered_rate_per_second: 20_000,
            operations: 20,
            highest_trackable_latency: Duration::from_secs(1),
            significant_figures: 3,
            p999_min_samples: 100,
            drain_timeout: Duration::from_secs(1),
        },
    }
}

#[tokio::test]
async fn warmup_samples_never_enter_the_steady_histogram() {
    let target = Arc::new(CountingTarget::default());
    let run = run_phases(Arc::clone(&target), &phase_config())
        .await
        .unwrap();
    assert_eq!(run.steady.latency.samples, 20);
    assert_eq!(run.warmup_samples_in_steady_histogram, 0);
    assert_eq!(run.steady_state_digest, "state:calls:5:v1");
    assert_eq!(target.calls.load(Ordering::SeqCst), 25);
}

struct RejectingWarmupTarget;

#[async_trait]
impl Target for RejectingWarmupTarget {
    async fn reset(&self) -> Result<String, TargetError> {
        Ok("state:empty:v1".to_owned())
    }

    async fn execute(&self, _request: TargetRequest) -> TargetOutcome {
        TargetOutcome::Rejected
    }

    async fn state_digest(&self) -> Result<String, TargetError> {
        Ok("state:rejected:v1".to_owned())
    }
}

#[tokio::test]
async fn unsuccessful_warmup_never_enters_the_steady_window() {
    let error = run_phases(Arc::new(RejectingWarmupTarget), &phase_config())
        .await
        .unwrap_err();
    assert!(matches!(error, TargetError::Warmup(_)));
}

#[tokio::test]
async fn declared_preload_count_must_match_target_evidence() {
    let target = Arc::new(CountingTarget::default());
    let mut config = phase_config();
    config.preload_operations = 1;
    let error = run_phases(target, &config).await.unwrap_err();
    assert!(matches!(error, TargetError::Preload(_)));
}

#[tokio::test]
async fn repeat_reset_reproduces_the_initial_state_digest() {
    let target = Arc::new(CountingTarget::default());
    let first = run_phases(Arc::clone(&target), &phase_config())
        .await
        .unwrap();
    let second = run_phases(Arc::clone(&target), &phase_config())
        .await
        .unwrap();
    assert_eq!(first.initial_state_digest, second.initial_state_digest);
    assert_eq!(target.resets.load(Ordering::SeqCst), 2);
}

#[tokio::test(start_paused = true)]
async fn scenario_runner_executes_every_declared_rate_and_repeat() {
    let target = Arc::new(CountingTarget::default());
    let scenario = Scenario {
        schema_version: 1,
        id: "runner-fixture".to_owned(),
        seed: 67,
        offered_rates_per_second: vec![100, 200],
        preload_operations: 0,
        warmup_operations: 0,
        steady_operations: 30,
        repeats: 3,
        p99_slo_us: 10_000,
        p999_slo_us: Some(10_000),
        p999_min_samples: 1,
        highest_trackable_latency_us: 1_000_000,
        histogram_significant_figures: 3,
        min_achieved_ratio: 0.5,
        error_budgets: ErrorBudgets {
            max_error_ratio: 0.0,
            max_timeout_ratio: 0.0,
            max_rejection_ratio: 0.0,
        },
        backlog_drain_ms: 1_000,
        robust_spread_tolerance: 0.10,
    };
    let knee = run_scenario(Arc::clone(&target), &scenario).await.unwrap();
    assert_eq!(knee.evaluated.len(), 2);
    assert!(knee.evaluated.iter().all(|point| point.repeats.len() == 3));
    assert_eq!(target.resets.load(Ordering::SeqCst), 6);
}

struct ActiveGuard<'a>(&'a AtomicU64);

impl Drop for ActiveGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Default)]
struct NeverFinishingTarget {
    active: AtomicU64,
}

#[async_trait]
impl Target for NeverFinishingTarget {
    async fn reset(&self) -> Result<String, TargetError> {
        Ok("state:empty:v1".to_owned())
    }

    async fn execute(&self, _request: TargetRequest) -> TargetOutcome {
        self.active.fetch_add(1, Ordering::SeqCst);
        let _active = ActiveGuard(&self.active);
        tokio::time::sleep(Duration::from_secs(60)).await;
        TargetOutcome::Success
    }

    async fn state_digest(&self) -> Result<String, TargetError> {
        Ok("state:never-finishing:v1".to_owned())
    }
}

#[tokio::test]
async fn drain_timeout_cancels_inflight_requests_before_returning() {
    let target = Arc::new(NeverFinishingTarget::default());
    let result = run_open_loop(
        Arc::clone(&target),
        &OpenLoopConfig {
            offered_rate_per_second: 1_000,
            operations: 3,
            highest_trackable_latency: Duration::from_secs(1),
            significant_figures: 3,
            p999_min_samples: 100,
            drain_timeout: Duration::from_millis(5),
        },
    )
    .await
    .unwrap();
    assert!(!result.backlog_drained);
    assert_eq!(result.completed, 0);
    assert_eq!(target.active.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn cancelling_the_driver_cancels_every_owned_request() {
    let target = Arc::new(NeverFinishingTarget::default());
    let run_target = Arc::clone(&target);
    let driver = tokio::spawn(async move {
        run_open_loop(
            run_target,
            &OpenLoopConfig {
                offered_rate_per_second: 1_000,
                operations: 100,
                highest_trackable_latency: Duration::from_secs(1),
                significant_figures: 3,
                p999_min_samples: 100,
                drain_timeout: Duration::from_secs(60),
            },
        )
        .await
    });
    while target.active.load(Ordering::SeqCst) == 0 {
        tokio::task::yield_now().await;
    }
    driver.abort();
    let _ = driver.await;
    tokio::time::timeout(Duration::from_secs(1), async {
        while target.active.load(Ordering::SeqCst) != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("owned target futures must be cancelled with their driver");
}

fn fingerprint(shared_hardware: bool, fingerprint: &str) -> RunnerFingerprint {
    RunnerFingerprint {
        runner_class: "reference-v1".to_owned(),
        fingerprint: fingerprint.to_owned(),
        cpu_model: "fixture-cpu".to_owned(),
        logical_cores: 16,
        ram_bytes: 64 * 1024 * 1024 * 1024,
        os: "linux".to_owned(),
        kernel: "fixture".to_owned(),
        cpu_affinity: "0-15".to_owned(),
        cgroup_cpu_quota: "max".to_owned(),
        governor: "performance".to_owned(),
        turbo: "disabled".to_owned(),
        shared_hardware,
        calibration_score: 0.01,
    }
}

fn reference_profile() -> PerformanceProfile {
    PerformanceProfile {
        name: "reference-v1".to_owned(),
        required_runner_class: "reference-v1".to_owned(),
        allowed_fingerprints: vec!["approved".to_owned()],
        minimum_logical_cores: 8,
        required_cpu_affinity: "0-15".to_owned(),
        required_cgroup_cpu_quota: "max".to_owned(),
        require_dedicated: true,
        maximum_calibration_score: 0.05,
    }
}

#[test]
fn reference_profile_rejects_a_spoofed_or_shared_runner() {
    assert!(
        reference_profile()
            .validate(&fingerprint(false, "approved"))
            .eligible
    );
    let spoofed = reference_profile().validate(&fingerprint(false, "unapproved"));
    assert!(!spoofed.eligible);
    let shared = reference_profile().validate(&fingerprint(true, "approved"));
    assert!(!shared.eligible);
    assert!(shared
        .reasons
        .iter()
        .any(|reason| reason.contains("shared hardware")));
}

fn foundation_workload() -> WorkloadIdentity {
    WorkloadIdentity {
        generator: "synthetic-foundation".to_owned(),
        generator_version: "1".to_owned(),
        seed: Some(67),
        key_distribution: None,
        key_count: None,
        operation_mix: vec![WeightedOperation {
            operation: "noop".to_owned(),
            weight: 1.0,
        }],
        payload_mix: vec![WeightedPayload {
            bytes: 1,
            weight: 1.0,
        }],
        digest: "workload-sha".to_owned(),
    }
}

fn foundation_measurement() -> MeasurementEvidence {
    MeasurementEvidence::LoadCurve(LoadCurveEvidence {
        id: "foundation-open-loop".to_owned(),
        scenario_digest: "scenario-sha".to_owned(),
        dimensions: BTreeMap::new(),
        workload: foundation_workload(),
        criteria: Some(criteria()),
        knee: Some(criteria().find_knee(vec![point(100.0, 100.0, 5)])),
        claim: LoadClaim::CapacityKnee,
    })
}

fn scalar_measurement() -> MeasurementEvidence {
    MeasurementEvidence::Scalar(ScalarEvidence {
        id: "foundation-efficiency".to_owned(),
        scenario_digest: "scenario-sha".to_owned(),
        workload: foundation_workload(),
        points: vec![ScalarPoint {
            dimensions: BTreeMap::from([("workers".to_owned(), DimensionValue::U64(1))]),
            quantity: Quantity {
                value: 100.0,
                unit: "operations_per_second".to_owned(),
            },
            sample_count: 3,
            samples: vec![100.0, 100.0, 100.0],
            min: 100.0,
            max: 100.0,
            robust_spread_ratio: 0.0,
        }],
        derived_from: vec!["foundation-open-loop".to_owned()],
        max_robust_spread_ratio: 0.10,
    })
}

fn comparison_measurement(ratio: f64, same_box: bool) -> MeasurementEvidence {
    MeasurementEvidence::Comparison(ComparisonEvidence {
        id: "foundation-comparison".to_owned(),
        scenario_digest: "scenario-sha".to_owned(),
        left_measurement_id: "foundation-open-loop".to_owned(),
        right_measurement_id: "foundation-efficiency".to_owned(),
        ratio,
        unit: "ratio".to_owned(),
        same_box,
    })
}

fn fixture_report(measurements: Vec<MeasurementEvidence>) -> PerfReport {
    let observed = fingerprint(false, "approved");
    PerfReport::new(
        "foundation-fixture",
        "foundation",
        "state-sha",
        EvidenceRunMode::ReferenceEvidence,
        SurfaceIdentity {
            surface_kind: "synthetic-instrument".to_owned(),
            execution_mode: "deterministic-model".to_owned(),
            state_scope: "test-process".to_owned(),
            network_boundary: "none".to_owned(),
            claim_scope: "instrument-contract".to_owned(),
        },
        reference_profile(),
        observed,
        SourceIdentity {
            git_commit: "0123456789012345678901234567890123456789".to_owned(),
            cargo_lock_sha256: "lock-sha".to_owned(),
            toolchain: "rustc fixture".to_owned(),
            build_flags: vec!["--release".to_owned()],
        },
        BuildIdentity {
            prebuild_contract_digest: "contract-sha".to_owned(),
            prebuild_manifest_sha256: "manifest-sha".to_owned(),
            binary_sha256: vec![("hydracache-loadgen".to_owned(), "binary-sha".to_owned())],
        },
        measurements,
        vec![],
    )
}

#[test]
fn perf_report_schema_records_surface_profile_commit_workload_and_prebuild_digests() {
    let report = fixture_report(vec![foundation_measurement()]);
    assert!(report.stable, "{:?}", report.stability_reasons);
    assert!(report.validation_problems().is_empty());
    let value: serde_json::Value =
        serde_json::from_slice(&report.to_pretty_json().unwrap()).unwrap();
    assert_eq!(value["release"], "0.67.0");
    assert_eq!(value["surface"]["network_boundary"], "none");
    assert_eq!(value["runner_profile"], "reference-v1");
    assert_eq!(
        value["source"]["git_commit"],
        "0123456789012345678901234567890123456789"
    );
    assert_eq!(value["workload_digest"].as_str().unwrap().len(), 64);
    assert_eq!(
        value["measurements"][0]["evidence"]["workload"]["digest"],
        "workload-sha"
    );
    assert_eq!(
        value["measurements"][0]["evidence"]["knee"]["evaluated"][0]["repeats"][0]["state_digest"],
        "state-sha"
    );
    assert_eq!(value["build"]["prebuild_contract_digest"], "contract-sha");
    assert_eq!(value["build"]["prebuild_manifest_sha256"], "manifest-sha");
}

#[test]
fn perf_report_json_schema_accepts_valid_evidence_and_rejects_short_repeat_sets() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/testing/schemas/perf-report.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let mut instance =
        serde_json::to_value(fixture_report(vec![foundation_measurement()])).unwrap();
    assert!(validator.is_valid(&instance));

    instance["measurements"][0]["evidence"]["knee"]["evaluated"][0]["repeats"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    assert!(!validator.is_valid(&instance));
}

#[test]
fn perf_report_without_measurements_is_never_stable() {
    let report = fixture_report(vec![]);
    assert!(!report.stable);
    assert!(report
        .stability_reasons
        .iter()
        .any(|reason| reason.contains("typed measurement")));
}

#[test]
fn perf_report_revalidates_profile_and_knee_instead_of_trusting_stored_flags() {
    let mut report = fixture_report(vec![foundation_measurement()]);
    report.observed_runner.shared_hardware = true;
    report.profile_validation.eligible = true;
    report.profile_validation.reasons.clear();
    report.stable = true;
    assert!(!report.validation_problems().is_empty());
    assert!(report.to_pretty_json().is_err());

    let MeasurementEvidence::LoadCurve(curve) = &mut report.measurements[0] else {
        panic!("fixture must contain a load curve");
    };
    let knee = curve.knee.as_mut().unwrap();
    knee.evaluated[0].sample.completed -= 1;
    assert!(!report.validation_problems().is_empty());
}

#[test]
fn scalar_and_comparison_evidence_require_raw_spread_and_recomputed_dependencies() {
    let report = fixture_report(vec![
        foundation_measurement(),
        scalar_measurement(),
        comparison_measurement(1.0, true),
    ]);
    assert!(report.stable, "{:?}", report.stability_reasons);
    assert!(report.to_pretty_json().is_ok());

    let wrong_ratio = fixture_report(vec![
        foundation_measurement(),
        scalar_measurement(),
        comparison_measurement(2.0, true),
    ]);
    assert!(!wrong_ratio.stable);

    let not_same_box = fixture_report(vec![
        foundation_measurement(),
        scalar_measurement(),
        comparison_measurement(1.0, false),
    ]);
    assert!(!not_same_box.stable);

    let mut short_scalar = scalar_measurement();
    let MeasurementEvidence::Scalar(value) = &mut short_scalar else {
        unreachable!()
    };
    value.points[0].samples.truncate(1);
    value.points[0].sample_count = 1;
    let short = fixture_report(vec![foundation_measurement(), short_scalar]);
    assert!(!short.stable);
}

#[test]
fn report_writer_rejects_measurement_input_digest_mutation() {
    let mut report = fixture_report(vec![foundation_measurement()]);
    let MeasurementEvidence::LoadCurve(value) = &mut report.measurements[0] else {
        unreachable!()
    };
    value.workload.digest = "mutated-workload".to_owned();
    assert!(report.to_pretty_json().is_err());

    let mut seed_mutation = fixture_report(vec![foundation_measurement()]);
    seed_mutation.seed = seed_mutation.seed.wrapping_add(1);
    assert!(seed_mutation.to_pretty_json().is_err());

    let mut workload_seed_mutation = fixture_report(vec![foundation_measurement()]);
    let MeasurementEvidence::LoadCurve(value) = &mut workload_seed_mutation.measurements[0] else {
        unreachable!()
    };
    value.workload.seed = Some(68);
    assert!(workload_seed_mutation.to_pretty_json().is_err());
}

struct SerializedStallTarget {
    lane: tokio::sync::Mutex<()>,
    stall_at: u64,
}

#[async_trait]
impl Target for SerializedStallTarget {
    async fn reset(&self) -> Result<String, TargetError> {
        Ok("state:synthetic-stall:v1".to_owned())
    }

    async fn execute(&self, request: TargetRequest) -> TargetOutcome {
        let _guard = self.lane.lock().await;
        if request.sequence == self.stall_at {
            tokio::time::sleep(Duration::from_secs(1)).await;
        } else {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        TargetOutcome::Success
    }

    async fn state_digest(&self) -> Result<String, TargetError> {
        Ok("state:synthetic-stall:v1".to_owned())
    }
}

#[tokio::test(start_paused = true)]
async fn canary_closed_loop_measurement_hides_a_synthetic_stall() {
    let operations = 200_u64;
    let target = Arc::new(SerializedStallTarget {
        lane: tokio::sync::Mutex::new(()),
        stall_at: 50,
    });
    let open_loop = run_open_loop(
        Arc::clone(&target),
        &OpenLoopConfig {
            offered_rate_per_second: 100,
            operations,
            highest_trackable_latency: Duration::from_secs(5),
            significant_figures: 3,
            p999_min_samples: 100,
            drain_timeout: Duration::from_secs(2),
        },
    )
    .await
    .unwrap();
    let mut closed_loop = LatencyHistogram::new(Duration::from_secs(5), 3).unwrap();
    for sequence in 0..operations {
        let started = tokio::time::Instant::now();
        let _ = target.execute(TargetRequest { sequence }).await;
        closed_loop.record(started.elapsed());
    }
    let open_p99 = open_loop.latency.p99_us.unwrap();
    let closed_p99 = closed_loop.summary(100).p99_us.unwrap();

    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W0") {
        assert!(
            open_p99 <= closed_p99,
            "HC-CANARY-RED:W0 closed-loop measurement hid a synthetic stall"
        );
    }
    assert!(open_p99 > 500_000, "open-loop p99={open_p99}");
    assert!(closed_p99 < 10_000, "closed-loop p99={closed_p99}");
}

fn w1_measurement_id(measurement: &MeasurementEvidence) -> &str {
    match measurement {
        MeasurementEvidence::LoadCurve(value) => &value.id,
        MeasurementEvidence::Scalar(value) => &value.id,
        MeasurementEvidence::TraceReplay(value) => &value.id,
        MeasurementEvidence::Comparison(value) => &value.id,
    }
}

#[tokio::test]
async fn local_cache_scaling_curve_1_to_n_threads_smoke() {
    let (curve, efficiency) = hydracache_loadgen::tiers::local::local_scaling_smoke_measurements()
        .await
        .unwrap();

    let MeasurementEvidence::Scalar(curve) = curve else {
        panic!("local scaling curve must use scalar evidence");
    };
    assert_eq!(curve.id, "local_cache_scaling_curve_1_to_n_threads");
    assert!(curve.points.len() >= 2);
    assert!(curve
        .points
        .iter()
        .any(|point| { point.dimensions.get("worker_threads") == Some(&DimensionValue::U64(1)) }));
    assert!(curve.points.iter().all(|point| {
        point.sample_count == 3
            && point.samples.len() == 3
            && point.quantity.unit == "operations_per_second"
            && point.quantity.value.is_finite()
            && point.quantity.value > 0.0
    }));
    assert_eq!(curve.workload.generator_version, "1");
    assert_eq!(curve.workload.digest.len(), 64);

    let MeasurementEvidence::Scalar(efficiency) = efficiency else {
        panic!("local scaling efficiency must use scalar evidence");
    };
    assert_eq!(
        efficiency.derived_from,
        ["local_cache_scaling_curve_1_to_n_threads"]
    );
    assert_eq!(efficiency.points.len(), curve.points.len());
}

#[tokio::test]
async fn hot_key_contention_throughput_floor_smoke() {
    let measurement = hydracache_loadgen::tiers::local::local_hot_key_smoke_measurement()
        .await
        .unwrap();
    let MeasurementEvidence::LoadCurve(curve) = measurement else {
        panic!("hot-key contention must use load-curve evidence");
    };

    assert_eq!(curve.id, "hot_key_contention_throughput_floor");
    assert_eq!(
        curve.dimensions.get("logical_key_count"),
        Some(&DimensionValue::U64(1))
    );
    assert_eq!(curve.claim, LoadClaim::CapacityKnee);
    assert_eq!(curve.workload.key_count, Some(1));
    let knee = curve
        .knee
        .expect("hot-key load curve must retain raw knee data");
    assert_eq!(knee.evaluated.len(), 2);
    assert!(knee.evaluated.iter().all(|point| point.repeats.len() == 3));
    assert!(knee.sustainable_rate_per_second.is_some());

    let single_flight =
        hydracache_loadgen::tiers::local::local_hot_key_single_flight_smoke_measurement()
            .await
            .unwrap();
    let MeasurementEvidence::Scalar(single_flight) = single_flight else {
        panic!("cold hot-key burst must use scalar evidence");
    };
    assert_eq!(single_flight.id, "hot_key_single_flight_miss_stampede_cost");
    assert_eq!(
        single_flight.points[0].dimensions.get("loader_executions"),
        Some(&DimensionValue::U64(1))
    );
    let concurrent_requests = single_flight.points[0]
        .dimensions
        .get("concurrent_requests")
        .cloned();
    assert_eq!(
        single_flight.points[0]
            .dimensions
            .get("cache_misses_before_loader_release"),
        concurrent_requests.as_ref()
    );
    assert_eq!(
        single_flight.points[0].dimensions.get("cache_hits"),
        Some(&DimensionValue::U64(0))
    );
}

#[tokio::test]
async fn throughput_at_full_capacity_vs_half_capacity_smoke() {
    let measurement = hydracache_loadgen::tiers::local::local_capacity_smoke_measurement()
        .await
        .unwrap();
    let MeasurementEvidence::Scalar(capacity) = measurement else {
        panic!("capacity-pressure comparison must use scalar evidence");
    };

    assert_eq!(capacity.id, "throughput_at_full_capacity_vs_half_capacity");
    assert_eq!(capacity.points.len(), 4);
    let combinations = capacity
        .points
        .iter()
        .map(|point| {
            let distribution = match point.dimensions.get("distribution") {
                Some(DimensionValue::Text(value)) => value.as_str(),
                other => panic!("missing distribution dimension: {other:?}"),
            };
            let capacity_profile = match point.dimensions.get("capacity_profile") {
                Some(DimensionValue::Text(value)) => value.as_str(),
                other => panic!("missing capacity profile dimension: {other:?}"),
            };
            assert_eq!(
                point.dimensions.get("every_insert_evicts_proof"),
                Some(&DimensionValue::Bool(true))
            );
            assert!(matches!(
                point.dimensions.get("verified_full_preload_entries"),
                Some(DimensionValue::U64(entries)) if *entries > 0
            ));
            assert_eq!(
                point.dimensions.get("eviction_proof_operations_per_repeat"),
                Some(&DimensionValue::U64(240))
            );
            assert_eq!(
                point.dimensions.get("eviction_proof_repeats"),
                Some(&DimensionValue::U64(3))
            );
            assert_eq!(point.sample_count, 3);
            assert_eq!(point.samples.len(), 3);
            (distribution, capacity_profile)
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        combinations,
        std::collections::BTreeSet::from([
            ("uniform", "half"),
            ("uniform", "full"),
            ("zipfian", "half"),
            ("zipfian", "full"),
        ])
    );
    assert_eq!(capacity.workload.digest.len(), 64);
}

#[tokio::test]
async fn hit_miss_and_loader_path_cost_breakdown_smoke() {
    let measurement = hydracache_loadgen::tiers::local::local_path_cost_smoke_measurement()
        .await
        .unwrap();
    let MeasurementEvidence::Scalar(paths) = measurement else {
        panic!("path-cost breakdown must use scalar evidence");
    };

    assert_eq!(paths.id, "hit_miss_and_loader_path_cost_breakdown");
    let names = paths
        .points
        .iter()
        .map(|point| match point.dimensions.get("path") {
            Some(DimensionValue::Text(value)) => value.as_str(),
            other => panic!("missing path dimension: {other:?}"),
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        names,
        std::collections::BTreeSet::from(["hit", "miss", "loader"])
    );
    assert!(paths.points.iter().all(|point| {
        point.sample_count == 3
            && point.samples.len() == 3
            && point.quantity.unit == "operations_per_second"
    }));
}

#[tokio::test]
async fn bytes_allocated_per_operation_by_feature_smoke() {
    let measurement = hydracache_loadgen::tiers::local::local_allocation_smoke_measurement()
        .await
        .unwrap();
    let MeasurementEvidence::Scalar(allocations) = measurement else {
        panic!("allocation breakdown must use scalar evidence");
    };

    assert_eq!(allocations.id, "bytes_allocated_per_operation_by_feature");
    let features = allocations
        .points
        .iter()
        .map(|point| match point.dimensions.get("feature") {
            Some(DimensionValue::Text(value)) => value.as_str(),
            other => panic!("missing feature dimension: {other:?}"),
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        features,
        std::collections::BTreeSet::from(["baseline", "ttl", "tags"])
    );
    assert!(allocations.points.iter().all(|point| {
        point.sample_count == 3
            && point.samples.len() == 3
            && point.quantity.unit == "gross_allocated_bytes_per_operation"
            && point.quantity.value.is_finite()
            && point.quantity.value > 0.0
    }));
}

#[tokio::test]
async fn w22_trace_replay_preserves_order_and_records_trace_digest() {
    let measurement = hydracache_loadgen::tiers::local::local_trace_replay_smoke_measurement()
        .await
        .unwrap();
    let MeasurementEvidence::TraceReplay(replay) = measurement else {
        panic!("W22 replay must use trace-replay evidence");
    };

    assert_eq!(
        replay.id,
        "w22_trace_replay_preserves_order_and_records_trace_digest"
    );
    assert!(replay.order_preserved);
    assert_eq!(replay.input_digest, replay.replayed_digest);
    assert_eq!(replay.input_digest.len(), 64);
    assert!(replay.event_count > 0);
    assert_eq!(replay.hits + replay.misses, replay.event_count);
    assert!(replay.catalog_id.contains("w22-v1"));
}

#[tokio::test]
async fn local_report_contains_every_required_measurement_and_workload_identity() {
    let report = hydracache_loadgen::tiers::local::local_smoke_report("smoke-v1")
        .await
        .unwrap();

    assert_eq!(report.run_mode, EvidenceRunMode::Smoke);
    assert_eq!(report.surface.claim_scope, "plumbing-only");
    assert!(
        !report.stable,
        "smoke output must never become ship evidence"
    );
    assert!(report.to_pretty_json().is_ok());

    let ids = report
        .measurements
        .iter()
        .map(w1_measurement_id)
        .collect::<std::collections::BTreeSet<_>>();
    for required in hydracache_loadgen::tiers::local::REQUIRED_LOCAL_MEASUREMENTS {
        assert!(
            ids.contains(required),
            "missing required W1 measurement {required}"
        );
    }
    for measurement in &report.measurements {
        match measurement {
            MeasurementEvidence::LoadCurve(value) => {
                assert_eq!(value.workload.digest.len(), 64);
                assert!(!value.workload.generator.is_empty());
                assert!(!value.workload.generator_version.is_empty());
            }
            MeasurementEvidence::Scalar(value) => {
                assert_eq!(value.workload.digest.len(), 64);
                assert!(!value.workload.generator.is_empty());
                assert!(!value.workload.generator_version.is_empty());
            }
            MeasurementEvidence::TraceReplay(value) => {
                assert_eq!(value.input_digest.len(), 64);
                assert_eq!(value.replayed_digest.len(), 64);
            }
            MeasurementEvidence::Comparison(_) => {}
        }
    }
    let seeds = report
        .measurements
        .iter()
        .filter_map(|measurement| match measurement {
            MeasurementEvidence::LoadCurve(value) => value.workload.seed,
            MeasurementEvidence::Scalar(value) => value.workload.seed,
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        seeds,
        std::collections::BTreeSet::from([6701, 6702, 6703, 6704, 6705])
    );
}

#[test]
fn local_cli_plan_and_suite_forms_share_one_runner() {
    let direct = hydracache_loadgen::cli::parse(
        [
            "tier",
            "local",
            "--profile",
            "reference-v1",
            "--report",
            "target/test-evidence/0.67/local.json",
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap();
    let suite = hydracache_loadgen::cli::parse(
        [
            "suite",
            "core",
            "--profile",
            "reference-v1",
            "--output-dir",
            "target/test-evidence/0.67",
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap();

    assert_eq!(direct.profile(), suite.profile());
    assert_eq!(direct.local_report_path(), suite.local_report_path());
}

#[tokio::test]
async fn local_reference_profile_never_silently_downgrades_to_smoke() {
    let error = hydracache_loadgen::tiers::local::write_local_report(
        "reference-v1",
        std::path::Path::new("target/test-evidence/0.67/forbidden-reference-smoke.json"),
    )
    .await
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("refusing to emit smoke evidence"));
    assert!(
        hydracache_loadgen::tiers::local::local_smoke_report("reference-v1")
            .await
            .is_err()
    );
}

#[tokio::test]
async fn canary_injected_slow_eviction_breaches_the_local_budget() {
    let slow_knee =
        hydracache_loadgen::tiers::local::local_pressure_knee(Duration::from_millis(25))
            .await
            .unwrap();
    let slow_path_is_red = slow_knee.sustainable_rate_per_second.is_none()
        && slow_knee
            .evaluated
            .iter()
            .all(|point| !point.verdict.sustainable);

    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W1") {
        assert!(
            !slow_path_is_red,
            "HC-CANARY-RED:W1 injected capacity-pressure delay breached the local budget"
        );
    }
    assert!(
        slow_path_is_red,
        "injected capacity-pressure delay must make the local knee unsustainable: {slow_knee:?}"
    );
}

#[tokio::test]
async fn client_surface_target_routes_real_frames_without_a_daemon() {
    use hydracache_loadgen::targets::client_surface::{
        ClientSurfaceOperation, ClientSurfaceOperationMix, ClientSurfaceTarget,
        ClientSurfaceTargetConfig,
    };

    let target = ClientSurfaceTarget::new(ClientSurfaceTargetConfig {
        limits: hydracache_client_transport_axum::ClientSurfaceLimits::default(),
        preload_entries: 2,
        key_space: 4,
        payload_bytes: 32,
        batch_size: 2,
        operation_mix: ClientSurfaceOperationMix::WORKLOAD_A,
        key_schedule: Arc::new(vec![0, 1, 2, 3]),
        injected_dispatch_delay: Duration::ZERO,
    })
    .unwrap();

    let reset_digest = target.reset().await.unwrap();
    let preload = target.preload().await.unwrap();
    assert_ne!(reset_digest, preload.state_digest);
    let put = target
        .dispatch_operation(ClientSurfaceOperation::Put, 2)
        .await
        .unwrap();
    let get = target
        .dispatch_operation(ClientSurfaceOperation::Get, 2)
        .await
        .unwrap();
    assert_eq!(put.outcome, TargetOutcome::Success);
    assert_eq!(get.outcome, TargetOutcome::Success);
    assert!(target.snapshot().await.dispatch_attempts >= 2);
}

async fn w2_smoke_report() -> PerfReport {
    static REPORT: tokio::sync::OnceCell<PerfReport> = tokio::sync::OnceCell::const_new();
    REPORT
        .get_or_init(|| async {
            hydracache_loadgen::tiers::client_surface::run_client_surface_profile("smoke-v1")
                .await
                .unwrap()
        })
        .await
        .clone()
}

fn measurement_named<'a>(report: &'a PerfReport, id: &str) -> &'a MeasurementEvidence {
    report
        .measurements
        .iter()
        .find(|measurement| match measurement {
            MeasurementEvidence::LoadCurve(value) => value.id == id,
            MeasurementEvidence::Scalar(value) => value.id == id,
            MeasurementEvidence::TraceReplay(value) => value.id == id,
            MeasurementEvidence::Comparison(value) => value.id == id,
        })
        .unwrap_or_else(|| panic!("missing measurement {id}"))
}

#[tokio::test]
async fn client_surface_in_process_knee_at_slo_for_a_b_c_smoke() {
    let report = w2_smoke_report().await;
    let measurement = measurement_named(&report, "client_surface_in_process_knee_at_slo_for_a_b_c");
    assert!(matches!(measurement, MeasurementEvidence::Scalar(_)));
    for workload in ["a", "b", "c"] {
        let id = format!("client_surface_in_process_knee_at_slo_workload_{workload}");
        assert!(matches!(
            measurement_named(&report, &id),
            MeasurementEvidence::LoadCurve(_)
        ));
    }

    let mut forged = report;
    let MeasurementEvidence::Scalar(aggregate) = forged
        .measurements
        .iter_mut()
        .find(|measurement| {
            matches!(measurement, MeasurementEvidence::Scalar(value) if value.id == "client_surface_in_process_knee_at_slo_for_a_b_c")
        })
        .expect("aggregate")
    else {
        unreachable!()
    };
    aggregate.derived_from.pop();
    assert!(forged.to_pretty_json().is_err());
}

#[test]
fn client_surface_operation_schedule_executes_declared_a_b_c_mix() {
    use hydracache_loadgen::targets::client_surface::{
        ClientSurfaceOperation, ClientSurfaceOperationMix,
    };

    for (mix, expected) in [
        (
            ClientSurfaceOperationMix::WORKLOAD_A,
            BTreeMap::from([("get", 45), ("put", 45), ("batch_get", 5), ("batch_put", 5)]),
        ),
        (
            ClientSurfaceOperationMix::WORKLOAD_B,
            BTreeMap::from([("get", 90), ("put", 4), ("batch_get", 5), ("batch_put", 1)]),
        ),
        (
            ClientSurfaceOperationMix::WORKLOAD_C,
            BTreeMap::from([("get", 90), ("put", 0), ("batch_get", 10), ("batch_put", 0)]),
        ),
    ] {
        let mut observed =
            BTreeMap::from([("get", 0), ("put", 0), ("batch_get", 0), ("batch_put", 0)]);
        for sequence in 0..100 {
            let operation = match mix.operation_for(sequence) {
                ClientSurfaceOperation::Get => "get",
                ClientSurfaceOperation::Put => "put",
                ClientSurfaceOperation::BatchGet => "batch_get",
                ClientSurfaceOperation::BatchPut => "batch_put",
            };
            *observed.get_mut(operation).unwrap() += 1;
        }
        assert_eq!(observed, expected);
    }
}

#[tokio::test]
async fn concurrent_inflight_scaling_curve_1_10_100_1000_smoke() {
    let report = w2_smoke_report().await;
    let measurement = measurement_named(&report, "concurrent_inflight_scaling_curve_1_10_100_1000");
    let MeasurementEvidence::Scalar(curve) = measurement else {
        panic!("concurrent in-flight scaling must use typed scalar points");
    };
    let inflight = curve
        .points
        .iter()
        .map(|point| {
            let declared = point.dimensions.get("concurrent_inflight");
            assert_eq!(
                declared,
                point.dimensions.get("observed_inflight_high_water")
            );
            assert_eq!(
                point.dimensions.get("measurement_boundary"),
                Some(&DimensionValue::Text(
                    "framed-request-lifetime-at-router-oneshot".to_owned()
                ))
            );
            declared.expect("declared in-flight")
        })
        .collect::<Vec<_>>();
    assert_eq!(inflight.len(), 4);
}

#[tokio::test]
async fn client_surface_payload_sweep_enforces_documented_cap() {
    let report = w2_smoke_report().await;
    let measurement = measurement_named(&report, "client_surface_payload_sweep_100b_1kb_64kb_1mb");
    let MeasurementEvidence::Scalar(payloads) = measurement else {
        panic!("payload sweep must use typed scalar points");
    };
    assert_eq!(payloads.points.len(), 4);
    let accepted = payloads
        .points
        .iter()
        .filter_map(|point| match point.dimensions.get("payload_bytes") {
            Some(DimensionValue::U64(bytes)) => Some(*bytes),
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        accepted,
        std::collections::BTreeSet::from([100, 1_000, 65_536, 1_000_000])
    );
    assert!(payloads.points.iter().all(|point| {
        point.dimensions.get("beyond_cap_http_status") == Some(&DimensionValue::U64(413))
            && point.dimensions.get("beyond_cap_rejected_before_dispatch")
                == Some(&DimensionValue::Bool(true))
    }));
}

#[tokio::test]
async fn client_surface_codec_dispatch_and_admission_rejection_cost_smoke() {
    let report = w2_smoke_report().await;
    let measurement = measurement_named(
        &report,
        "client_surface_codec_dispatch_and_admission_rejection_cost",
    );
    let MeasurementEvidence::Scalar(costs) = measurement else {
        panic!("client-surface path cost must use typed scalar points");
    };
    assert!(costs.points.len() >= 3);
    assert!(costs
        .points
        .iter()
        .all(|point| point.dimensions.contains_key("path")
            && point.dimensions.contains_key("payload_bytes")));
    let payloads = costs
        .workload
        .payload_mix
        .iter()
        .map(|payload| payload.bytes)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        payloads,
        std::collections::BTreeSet::from([1_000, 1_048_576])
    );
}

#[tokio::test]
async fn client_surface_report_preserves_in_process_reality_boundary() {
    let report = w2_smoke_report().await;
    assert_eq!(report.surface.surface_kind, "client-surface");
    assert_eq!(report.surface.execution_mode, "in-process-axum-router");
    assert_eq!(report.surface.state_scope, "process-local");
    assert_eq!(report.surface.network_boundary, "none");
    assert_eq!(report.surface.claim_scope, "plumbing-only");
    assert!(report.to_pretty_json().is_ok());
    let fast_registry = include_str!("../../../docs/testing/fast-suite-registry.toml")
        .parse::<toml::Table>()
        .unwrap();
    let w2_is_declared = fast_registry
        .get("suite")
        .and_then(toml::Value::as_array)
        .and_then(|suites| {
            suites.iter().find(|suite| {
                suite.get("id").and_then(toml::Value::as_str)
                    == Some("fast.performance-contract-067")
            })
        })
        .and_then(|suite| suite.get("work_items"))
        .and_then(toml::Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some("W2")));
    assert!(w2_is_declared, "W2 fast gate must declare W2 coverage");

    let mut forged = report;
    forged.surface.execution_mode = "daemon-native-wire".to_owned();
    assert!(forged
        .validation_problems()
        .iter()
        .any(|problem| problem.contains("daemon and wire claims are forbidden")));
    assert!(forged.to_pretty_json().is_err());
}

#[test]
fn client_surface_cli_plan_and_suite_forms_share_one_runner() {
    let direct = hydracache_loadgen::cli::parse(
        [
            "tier",
            "client-surface",
            "--profile",
            "reference-v1",
            "--report",
            "target/test-evidence/0.67/client-surface.json",
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap();
    let suite = hydracache_loadgen::cli::parse(
        [
            "suite",
            "core",
            "--profile",
            "reference-v1",
            "--output-dir",
            "target/test-evidence/0.67",
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap();

    assert_eq!(direct.profile(), suite.profile());
    assert_eq!(
        direct.client_surface_report_path(),
        suite.client_surface_report_path()
    );
}

#[tokio::test]
async fn client_surface_reference_profile_never_silently_downgrades_to_smoke() {
    let error =
        hydracache_loadgen::tiers::client_surface::run_client_surface_profile("reference-v1")
            .await
            .unwrap_err();
    assert!(error.to_string().contains("receipt-bound prebuild"));
}

#[tokio::test]
async fn canary_injected_client_surface_dispatch_delay_breaches_the_in_process_budget() {
    let slow_knee = hydracache_loadgen::tiers::client_surface::client_surface_dispatch_knee(
        Duration::from_millis(25),
    )
    .await
    .unwrap();
    let slow_path_is_red = slow_knee.sustainable_rate_per_second.is_none()
        && slow_knee
            .evaluated
            .iter()
            .all(|point| !point.verdict.sustainable);

    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W2") {
        assert!(
            !slow_path_is_red,
            "HC-CANARY-RED:W2 injected client-surface dispatch delay breached the in-process budget"
        );
    }
    assert!(
        slow_path_is_red,
        "injected dispatch delay must make the client-surface knee unsustainable: {slow_knee:?}"
    );
}
