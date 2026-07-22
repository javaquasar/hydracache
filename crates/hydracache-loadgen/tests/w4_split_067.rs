use hydracache_loadgen::targets::control_plane::{
    canary_control_plane_delay_breaches_the_w4a_event_budget, ControlPlaneCapabilityAttestation,
    ControlPlaneCapabilityOutcome, ControlPlaneScenario, ReferenceCapabilityPolicy,
};
use hydracache_loadgen::targets::grid_model::{
    canary_grid_model_short_circuit_is_rejected, run_grid_model_smoke, GridModelRunMode,
    GridModelScenario,
};

const CONTROL_SCENARIO: &str =
    include_str!("../../../docs/testing/perf-scenarios/0.67/control-plane-real-daemon-v1.toml");
const GRID_SCENARIO: &str =
    include_str!("../../../docs/testing/perf-scenarios/0.67/grid-model-primitives-v1.toml");

#[test]
fn w4_scenarios_are_strict_and_reject_surface_overclaims() {
    let control = ControlPlaneScenario::parse_toml(CONTROL_SCENARIO).unwrap();
    assert_eq!(control.read_only.node_counts, [3, 5, 7]);
    assert_eq!(control.read_only.repeats, 5);
    assert_eq!(
        control.identity.capacity_claim,
        "selected-admin-endpoint-read-only"
    );
    let mut summed = control.clone();
    summed.identity.aggregate_cluster_capacity = true;
    assert!(summed.validate().is_err());
    let mut no_knee_scope = control;
    no_knee_scope.identity.capacity_claim = "none".to_owned();
    assert!(no_knee_scope.validate().is_err());

    let grid = GridModelScenario::parse_toml(GRID_SCENARIO).unwrap();
    grid.validate_exact_reference_shape().unwrap();
    assert_eq!(
        grid.reference.runner.required_runner_class,
        "github-hosted-reference-v1"
    );
    assert_eq!(grid.reference.runner.minimum_logical_cores, 4);
    assert_eq!(
        grid.reference.runner.required_cpu_affinity,
        "github-managed-vm"
    );
    assert_eq!(
        grid.reference.runner.required_cgroup_cpu_quota,
        "github-managed-vm"
    );
    assert_eq!(
        grid.reference.runner.maximum_calibration_score_millionths,
        250_000
    );
    let mut daemon = grid.clone();
    daemon.identity.daemon_processes = true;
    assert!(daemon.validate().is_err());
    let mut product = grid;
    product.identity.product_data_plane = true;
    assert!(product.validate().is_err());
}

#[test]
fn w4a_reference_capability_fails_closed_and_local_absence_skips_loud() {
    let scenario = ControlPlaneScenario::parse_toml(CONTROL_SCENARIO).unwrap();
    let absent = ControlPlaneCapabilityAttestation::absent();
    assert!(absent
        .clone()
        .require(&scenario, ReferenceCapabilityPolicy::MandatoryFailClosed)
        .is_err());
    let skipped = absent
        .require(&scenario, ReferenceCapabilityPolicy::LocalSkipLoud)
        .unwrap();
    assert!(matches!(
        skipped,
        ControlPlaneCapabilityOutcome::SkippedLoud(_)
    ));
}

#[tokio::test]
async fn w4b_executes_exported_primitives_with_exact_copy_and_fanout_accounting() {
    let scenario = reduced_grid_scenario();
    let report = run_grid_model_smoke(&scenario).await.unwrap();
    report.validate(&scenario).unwrap();
    assert_eq!(report.run_mode, GridModelRunMode::Smoke);
    assert!(!report.daemon_processes);
    assert!(!report.product_data_plane);
    assert!(!report.end_to_end_cluster_capacity);

    for point in &report.replication_primitive_curve {
        assert_eq!(
            point.modeled_replica_copy_bytes,
            point
                .input_bytes
                .saturating_mul(u64::from(point.replica_peers))
        );
    }
    for point in &report.invalidation_fanout_cost {
        assert_eq!(
            point.deliveries_observed,
            point
                .iterations
                .saturating_mul(u64::from(point.subscriber_count))
        );
    }
    assert!(
        canary_grid_model_short_circuit_is_rejected(&scenario, &report)
            .unwrap_err()
            .contains("HC-CANARY-RED:W4")
    );
}

#[test]
fn w4b_reduced_smoke_shape_cannot_be_promoted_to_reference() {
    let reduced = reduced_grid_scenario();
    reduced.validate().unwrap();
    assert!(reduced.validate_exact_reference_shape().is_err());

    let mut relabelled = reduced;
    relabelled.reference.committed_scenario_sha256 = relabelled.contract_sha256();
    assert!(relabelled.validate_exact_reference_shape().is_err());
}

#[tokio::test]
async fn canary_w4_instruments_reject_control_plane_delay_and_grid_model_short_circuit() {
    let control = ControlPlaneScenario::parse_toml(CONTROL_SCENARIO).unwrap();
    let control_red =
        canary_control_plane_delay_breaches_the_w4a_event_budget(&control).unwrap_err();

    let grid = reduced_grid_scenario();
    let baseline = run_grid_model_smoke(&grid).await.unwrap();
    let grid_red = canary_grid_model_short_circuit_is_rejected(&grid, &baseline).unwrap_err();

    assert!(control_red.contains("HC-CANARY-RED:W4"));
    assert!(grid_red.contains("HC-CANARY-RED:W4"));
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W4") {
        panic!("HC-CANARY-RED:W4 both W4A event timing and W4B primitive accounting rejected their injected defects");
    }
}

fn reduced_grid_scenario() -> GridModelScenario {
    let mut scenario = GridModelScenario::parse_toml(GRID_SCENARIO).unwrap();
    scenario.dimensions.iterations = 64;
    scenario.dimensions.replica_shapes = vec![1, 3];
    scenario.dimensions.region_shapes = vec![1, 2];
    scenario.dimensions.replication_peer_shapes = vec![1, 2];
    scenario.dimensions.payload_bytes = vec![64, 1_024];
    scenario.dimensions.invalidation_subscribers = vec![1, 3];
    scenario.dimensions.watermark_entries = 8;
    scenario.measurement.warmup_iterations = 16;
    scenario.measurement.raw_repeats = 3;
    scenario.measurement.maximum_robust_spread_ratio_millionths = 1_000_000;
    scenario
}
