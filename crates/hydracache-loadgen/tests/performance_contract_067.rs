use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use hydracache_loadgen::histogram::LatencySummary;
use hydracache_loadgen::report::PhaseAccounting;
use hydracache_loadgen::{
    run_phases, BuildIdentity, FixedRateSchedule, KneeResult, LatencyHistogram, OpenLoopConfig,
    PerfReport, PerformanceProfile, PhaseConfig, ProfileValidation, RateSample, RunnerFingerprint,
    SourceIdentity, SurfaceIdentity, SustainabilityCriteria, Target, TargetError, TargetOutcome,
    TargetRequest,
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

fn sample(rate: f64, achieved: f64, p99_us: u64) -> RateSample {
    RateSample {
        offered_rate_per_second: rate,
        achieved_rate_per_second: achieved,
        started: 10_000,
        completed: 10_000,
        errors: 0,
        timeouts: 0,
        rejections: 0,
        backlog_drained: true,
        drain_ms: 10,
        robust_spread_ratio: 0.01,
        latency: latency(p99_us),
    }
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
    let result = criteria().find_knee(&[
        sample(100.0, 100.0, 5),
        sample(200.0, 198.0, 8),
        sample(300.0, 299.0, 20),
    ]);
    assert_eq!(result.sustainable_rate_per_second, Some(200.0));
}

#[test]
fn knee_rejects_rate_when_latency_passes_but_achieved_rate_lags() {
    let verdict = criteria().evaluate(&sample(1_000.0, 500.0, 5));
    assert!(!verdict.sustainable);
    assert!(verdict
        .reasons
        .iter()
        .any(|reason| reason.contains("achieved/offered")));
}

#[test]
fn knee_rejects_timeout_rejection_budget_or_undrained_backlog() {
    let mut timeout = sample(1_000.0, 1_000.0, 5);
    timeout.timeouts = 20;
    assert!(!criteria().evaluate(&timeout).sustainable);

    let mut rejected = sample(1_000.0, 1_000.0, 5);
    rejected.rejections = 200;
    assert!(!criteria().evaluate(&rejected).sustainable);

    let mut queued = sample(1_000.0, 1_000.0, 5);
    queued.backlog_drained = false;
    assert!(!criteria().evaluate(&queued).sustainable);
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
    assert_eq!(target.calls.load(Ordering::SeqCst), 25);
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

#[test]
fn perf_report_schema_records_surface_profile_commit_workload_and_prebuild_digests() {
    let observed = fingerprint(false, "approved");
    let report = PerfReport::new(
        "foundation-fixture",
        "foundation",
        "scenario-sha",
        "workload-sha",
        "state-sha",
        67,
        SurfaceIdentity {
            surface_kind: "synthetic-instrument".to_owned(),
            execution_mode: "deterministic-model".to_owned(),
            state_scope: "test-process".to_owned(),
            network_boundary: "none".to_owned(),
            claim_scope: "instrument-contract".to_owned(),
        },
        "reference-v1",
        observed,
        ProfileValidation {
            eligible: true,
            reasons: vec![],
        },
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
        PhaseAccounting {
            reset_operations: 1,
            preload_operations: 0,
            warmup_operations: 5,
            steady_operations: 20,
            reset_ms: 1,
            preload_ms: 0,
            warmup_ms: 1,
            steady_ms: 1,
            warmup_samples_in_steady_histogram: 0,
        },
        vec!["state-sha".to_owned()],
        vec![],
        KneeResult {
            sustainable_rate_per_second: Some(100.0),
            evaluated: vec![],
        },
        vec![],
    );
    let value: serde_json::Value =
        serde_json::from_slice(&report.to_pretty_json().unwrap()).unwrap();
    assert_eq!(value["release"], "0.67.0");
    assert_eq!(value["surface"]["network_boundary"], "none");
    assert_eq!(value["runner_profile"], "reference-v1");
    assert_eq!(
        value["source"]["git_commit"],
        "0123456789012345678901234567890123456789"
    );
    assert_eq!(value["workload_digest"], "workload-sha");
    assert_eq!(value["repeat_state_digests"][0], "state-sha");
    assert_eq!(value["build"]["prebuild_contract_digest"], "contract-sha");
    assert_eq!(value["build"]["prebuild_manifest_sha256"], "manifest-sha");
}

#[test]
fn canary_closed_loop_measurement_hides_a_synthetic_stall() {
    let operations = 200_u64;
    let interval_us = 10_000_u64;
    let stall_at = 50_u64;
    let stall_us = 1_000_000_u64;
    let normal_us = 1_000_u64;

    let mut server_available_us = 0_u64;
    let mut open_loop = LatencyHistogram::new(Duration::from_secs(5), 3).unwrap();
    for sequence in 0..operations {
        let scheduled_us = sequence * interval_us;
        let service_us = if sequence == stall_at {
            stall_us
        } else {
            normal_us
        };
        let started_us = scheduled_us.max(server_available_us);
        let finished_us = started_us + service_us;
        server_available_us = finished_us;
        open_loop.record_us(finished_us - scheduled_us);
    }

    let mut closed_loop = LatencyHistogram::new(Duration::from_secs(5), 3).unwrap();
    for sequence in 0..operations {
        let service_us = if sequence == stall_at {
            stall_us
        } else {
            normal_us
        };
        closed_loop.record_us(service_us);
    }
    let open_p99 = open_loop.summary(100).p99_us.unwrap();
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
