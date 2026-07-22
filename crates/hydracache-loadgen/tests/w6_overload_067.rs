pub use hydracache_loadgen::{
    histogram, knee, rate, report, runner, scenario, target, targets, tiers, PERF_RELEASE,
};

#[path = "../src/overload.rs"]
#[allow(dead_code)]
mod overload;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use hydracache_loadgen::report::LoadClaim;
use overload::{
    admission_disabled_collapse_detected, deterministic_fixture_predecessor, run_overload_curve,
    AdmissionControlMode, DeterministicAdmissionFixture, EligibleOverloadSurface, OverloadRunMode,
    OverloadScenario, ReferencePredecessorRequest, OVERLOAD_FACTORS_MILLIONTHS, W6_CANARY_MARKER,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const SCENARIO: &str =
    include_str!("../../../docs/testing/perf-scenarios/0.67/overload-capacity-v1.toml");

fn scenario() -> OverloadScenario {
    OverloadScenario::parse_toml(SCENARIO).expect("committed W6 scenario must parse")
}

#[test]
fn committed_scenario_digest_seals_workload_spread_and_recovery_shape() {
    let contract = scenario();
    assert_eq!(
        contract.contract_digest().unwrap(),
        contract.reference.committed_scenario_sha256
    );
    assert_eq!(contract.work.reference_preload_operations.local, 0);
    assert_eq!(contract.work.burst_operations, 50_000);
    assert_eq!(contract.work.recovery_operations_per_window, 50_000);
    assert_eq!(
        contract.work.reference_preload_operations.client_surface,
        10_000
    );
    assert_eq!(contract.work.reference_preload_operations.node_resp, 10_000);
    let mut drifted = contract;
    drifted.work.maximum_goodput_spread_ratio = 0.30;
    assert!(drifted.validate().is_err());
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Clone, Serialize)]
struct ForgedReceipt {
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

async fn smoke_report(
    surface: EligibleOverloadSurface,
    admission_mode: AdmissionControlMode,
) -> overload::OverloadReport {
    let target = Arc::new(DeterministicAdmissionFixture::new(
        Duration::from_micros(100),
        admission_mode,
    ));
    let predecessor = deterministic_fixture_predecessor(Arc::clone(&target), surface, 500)
        .await
        .expect("fixture must establish a valid capacity knee");
    run_overload_curve(
        Arc::clone(&target),
        target.as_ref(),
        &scenario(),
        predecessor,
    )
    .await
    .expect("deterministic overload curve must execute")
}

#[tokio::test(start_paused = true)]
async fn overload_goodput_curve_1_2x_1_5x_2x_knee_per_eligible_surface() {
    let scenario_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/testing/perf-scenarios/0.67/overload-capacity-v1.toml");
    OverloadScenario::load(&scenario_path).unwrap();
    let _passthrough = overload::PassthroughWindowControl;
    for surface in [
        EligibleOverloadSurface::Local,
        EligibleOverloadSurface::ClientSurface,
        EligibleOverloadSurface::NodeResp,
    ] {
        let report = smoke_report(surface, AdmissionControlMode::Enabled).await;
        report.validate(&scenario()).unwrap();
        assert!(!report.to_pretty_json(&scenario()).unwrap().is_empty());
        assert_eq!(report.points.len(), 3);
        assert_eq!(
            report
                .points
                .iter()
                .map(|point| point.factor_millionths)
                .collect::<Vec<_>>(),
            OVERLOAD_FACTORS_MILLIONTHS
        );
        assert_eq!(
            report
                .points
                .iter()
                .map(|point| point.offered_rate_per_second)
                .collect::<Vec<_>>(),
            vec![600, 750, 1_000]
        );
        assert!(report.points.iter().all(|point| {
            point.repeats.len() == 3
                && point.aggregate.successful_goodput_per_second > 0.0
                && point.aggregate.scheduled_p99_us > 0
                && point.aggregate.robust_goodput_spread_ratio
                    <= scenario().work.maximum_goodput_spread_ratio
        }));
    }
}

#[tokio::test(start_paused = true)]
async fn rejection_ratio_latency_and_backlog_under_overload() {
    let report = smoke_report(
        EligibleOverloadSurface::Local,
        AdmissionControlMode::Enabled,
    )
    .await;
    let point_2x = report
        .points
        .iter()
        .find(|point| point.factor_millionths == 2_000_000)
        .unwrap();
    assert!(point_2x.aggregate.rejection_ratio > 0.0);
    assert_eq!(point_2x.aggregate.error_timeout_ratio, 0.0);
    assert!(point_2x.aggregate.scheduled_p99_us > 0);
    assert!(point_2x.aggregate.backlog_high_water > 0);

    let mut forged = report;
    forged.points[2].aggregate.backlog_high_water += 1;
    assert!(forged.validate(&scenario()).is_err());
}

#[tokio::test(start_paused = true)]
async fn recovery_time_to_baseline_after_burst() {
    let report = smoke_report(
        EligibleOverloadSurface::ClientSurface,
        AdmissionControlMode::Enabled,
    )
    .await;
    assert!(report.all_points_recovered());
    for point in &report.points {
        for repeat in &point.repeats {
            assert!(repeat.recovery.recovered_at_window.is_some());
            assert!(repeat.recovery.recovered_at_window.unwrap() >= 2);
            assert_eq!(repeat.recovery.consecutive_passing_windows, 2);
            assert!(repeat.recovery.time_to_baseline_ms.is_some());
            assert_eq!(
                repeat.recovery.time_to_baseline_ms,
                Some(repeat.recovery.observed_recovery_ms)
            );
            let window_ms = repeat
                .recovery
                .windows
                .iter()
                .map(|window| window.elapsed_ms)
                .sum::<u64>();
            assert_eq!(
                repeat.recovery.observed_recovery_ms,
                repeat.recovery.transition_duration_ms + window_ms
            );
            assert!(repeat.recovery.windows.len() <= scenario().work.max_recovery_windows as usize);
        }
    }
}

#[tokio::test(start_paused = true)]
async fn reset_and_preload_digests_are_equivalent_across_every_repeat_and_factor() {
    let mut report = smoke_report(
        EligibleOverloadSurface::Local,
        AdmissionControlMode::Enabled,
    )
    .await;
    let expected_reset = report.points[0].repeats[0].reset_state_digest.clone();
    let expected_preload = report.points[0].repeats[0].preloaded_state_digest.clone();
    assert!(report
        .points
        .iter()
        .flat_map(|point| &point.repeats)
        .all(|repeat| {
            repeat.reset_state_digest == expected_reset
                && repeat.preloaded_state_digest == expected_preload
        }));
    report.points[2].repeats[2].preloaded_state_digest = "drifted-preload".to_owned();
    assert!(report.validate(&scenario()).is_err());
}

#[tokio::test(start_paused = true)]
async fn reference_overload_fails_closed_without_valid_predecessor_receipt_and_profile() {
    let contract = scenario();
    let target = Arc::new(DeterministicAdmissionFixture::new(
        Duration::from_micros(100),
        AdmissionControlMode::Enabled,
    ));
    let mut predecessor = deterministic_fixture_predecessor(
        Arc::clone(&target),
        EligibleOverloadSurface::NodeResp,
        500,
    )
    .await
    .unwrap();
    predecessor.profile = "reference-v1".to_owned();
    predecessor.stable_capacity_evidence = true;
    let missing_receipt = predecessor
        .validate(&contract, OverloadRunMode::Reference)
        .unwrap_err();
    assert!(missing_receipt
        .to_string()
        .contains("missing its predecessor receipt"));

    // Even a fully sealed, syntactically valid receipt cannot promote the
    // smoke knee: validation re-opens the alleged PerfReport from disk.
    let fake_artifact = std::fs::canonicalize(std::env::current_exe().unwrap()).unwrap();
    let fake_bytes = std::fs::read(&fake_artifact).unwrap();
    let mut receipt = ForgedReceipt {
        profile: "reference-v1".to_owned(),
        predecessor_report_path: fake_artifact.clone(),
        predecessor_report_sha256: sha256(&fake_bytes),
        predecessor_measurement_id: "resp_open_loop_get_set_knee_at_slo_workload_a".to_owned(),
        predecessor_scenario_sha256: "ab".repeat(32),
        predecessor_payload_sha256: predecessor.payload_sha256().unwrap(),
        predecessor_lifecycle_path: Some(fake_artifact.clone()),
        predecessor_lifecycle_sha256: Some(sha256(&fake_bytes)),
        source_commit: "cd".repeat(20),
        cargo_lock_sha256: "bc".repeat(32),
        runner_fingerprint_sha256: "ef".repeat(32),
        prebuild_manifest_path: fake_artifact,
        prebuild_receipt_sha256: sha256(&fake_bytes),
        stable_surface_capability_sha256: predecessor.stable_surface_capability_sha256.clone(),
        workload_identity_sha256: predecessor.workload_identity_sha256.clone(),
        archived_execution_receipt_sha256: "98".repeat(32),
        archived_execution_pid: u32::MAX,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = sha256(&serde_json::to_vec(&receipt).unwrap());
    let mut value = serde_json::to_value(predecessor).unwrap();
    value["reference_receipt"] = serde_json::to_value(receipt).unwrap();
    let forged = serde_json::from_value::<overload::CapacityPredecessor>(value).unwrap();
    let forged_error = forged
        .validate(&contract, OverloadRunMode::Reference)
        .unwrap_err();
    assert!(forged_error.to_string().contains("typed PerfReport schema"));

    let missing_w1 = overload::load_reference_predecessor(
        &contract,
        ReferencePredecessorRequest {
            surface: EligibleOverloadSurface::Local,
            report_path: PathBuf::from("missing.json"),
            expected_report_sha256: "12".repeat(32),
            lifecycle_path: None,
            expected_lifecycle_sha256: None,
            prebuild_manifest_path: PathBuf::from("missing-prebuild.json"),
            expected_prebuild_manifest_sha256: "34".repeat(32),
        },
    )
    .unwrap_err();
    assert!(missing_w1.to_string().contains("unable to canonicalize"));
}

#[tokio::test(start_paused = true)]
async fn overload_rejects_removed_tiers_and_non_capacity_or_model_inputs() {
    assert!(EligibleOverloadSurface::from_cli_name("node-native").is_err());
    assert!(EligibleOverloadSurface::from_cli_name("cluster").is_err());
    assert!(EligibleOverloadSurface::from_cli_name("generic-cluster").is_err());
    assert!(EligibleOverloadSurface::from_cli_name("control-plane-read-only").is_err());

    let target = Arc::new(DeterministicAdmissionFixture::new(
        Duration::from_micros(100),
        AdmissionControlMode::Enabled,
    ));
    let mut predecessor =
        deterministic_fixture_predecessor(Arc::clone(&target), EligibleOverloadSurface::Local, 500)
            .await
            .unwrap();
    predecessor.claim = LoadClaim::ModelCost;
    assert!(run_overload_curve(
        Arc::clone(&target),
        target.as_ref(),
        &scenario(),
        predecessor.clone(),
    )
    .await
    .is_err());

    predecessor.claim = LoadClaim::OperationalCost;
    assert!(run_overload_curve(
        Arc::clone(&target),
        target.as_ref(),
        &scenario(),
        predecessor,
    )
    .await
    .is_err());
}

#[tokio::test(start_paused = true)]
async fn canary_admission_disabled_fixture_shows_goodput_collapse() {
    let enabled = smoke_report(
        EligibleOverloadSurface::Local,
        AdmissionControlMode::Enabled,
    )
    .await;
    let disabled = smoke_report(
        EligibleOverloadSurface::Local,
        AdmissionControlMode::DisabledCanary,
    )
    .await;
    let collapse = admission_disabled_collapse_detected(&enabled, &disabled).unwrap();

    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W6") {
        assert!(
            !collapse,
            "{W6_CANARY_MARKER} admission bypass collapsed 2x successful goodput instead of preserving the rejection plateau"
        );
    }
    assert!(
        collapse,
        "admission-disabled 2x fixture must collapse successful goodput with errors while the enabled path rejects and plateaus"
    );
}
