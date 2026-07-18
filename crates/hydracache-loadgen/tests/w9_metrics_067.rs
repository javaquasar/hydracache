use std::collections::BTreeMap;

use hydracache_loadgen::metrics_honesty::{
    validate_daemon_exporter_surface, verify_counter_agreement, w9_counter_undercount_canary_red,
    MetricField, MetricSelector, MetricsHonestyScenario, W9_CANARY_MARKER,
};
use hydracache_loadgen::SurfaceIdentity;

const SCENARIO: &str =
    include_str!("../../../docs/testing/perf-scenarios/0.67/metrics-honesty-v1.toml");

#[test]
fn server_reported_ops_and_rejects_match_loadgen_within_tolerance() {
    let scenario = MetricsHonestyScenario::parse_toml(SCENARIO).unwrap();
    let operations = MetricSelector {
        name: "hydracache_cache_hits_total".to_owned(),
        labels: BTreeMap::from([("cache".to_owned(), "typed-fixture".to_owned())]),
    };
    let exact = verify_counter_agreement(
        MetricField::Operations,
        operations,
        1_000,
        1_100,
        100,
        scenario.counter_absolute_tolerance,
        scenario.counter_relative_tolerance_millionths,
    )
    .unwrap();
    assert!(exact.agrees);
    assert_eq!(exact.reported_delta, exact.observer_delta);

    let rejections = MetricSelector {
        name: "hydracache_admission_rejected_total".to_owned(),
        labels: BTreeMap::new(),
    };
    let within_tolerance = verify_counter_agreement(
        MetricField::Rejections,
        rejections.clone(),
        10,
        111,
        100,
        scenario.counter_absolute_tolerance,
        scenario.counter_relative_tolerance_millionths,
    )
    .unwrap();
    assert_eq!(within_tolerance.absolute_error, 1);
    assert!(verify_counter_agreement(
        MetricField::Rejections,
        rejections,
        10,
        108,
        100,
        scenario.counter_absolute_tolerance,
        scenario.counter_relative_tolerance_millionths,
    )
    .is_err());
}

#[test]
fn server_latency_and_open_loop_scheduled_latency_have_explicit_non_conflated_boundaries() {
    let scenario = MetricsHonestyScenario::parse_toml(SCENARIO).unwrap();
    assert_eq!(
        scenario.latency_boundary.observer_metric,
        "scheduled-send-to-completion-latency"
    );
    assert!(
        scenario
            .latency_boundary
            .observer_includes_scheduler_queue_delay
    );
    assert_eq!(scenario.latency_boundary.server_metric, "not_available");
    assert_eq!(
        scenario.latency_boundary.server_metric_scope,
        "internal-service-time-if-ever-exported"
    );
    assert!(!scenario.latency_boundary.equality_claim);

    let mut conflated = scenario;
    conflated.latency_boundary.equality_claim = true;
    conflated.latency_boundary.server_metric = "scheduled-send-to-completion-latency".to_owned();
    assert!(conflated.validate().is_err());
}

#[test]
fn metrics_cross_check_rejects_in_process_reports_labeled_as_daemon_exporter_evidence() {
    let w2 = SurfaceIdentity {
        surface_kind: "client-surface".to_owned(),
        execution_mode: "in-process-axum-router".to_owned(),
        state_scope: "process-local".to_owned(),
        network_boundary: "none".to_owned(),
        claim_scope: "client-surface-capacity".to_owned(),
    };
    let w4b = SurfaceIdentity {
        surface_kind: "grid-model".to_owned(),
        execution_mode: "in-process-library-model".to_owned(),
        state_scope: "modeled-library-state".to_owned(),
        network_boundary: "none".to_owned(),
        claim_scope: "model-cost-only".to_owned(),
    };
    assert!(validate_daemon_exporter_surface(&w2).is_err());
    assert!(validate_daemon_exporter_surface(&w4b).is_err());

    let w3 = SurfaceIdentity {
        surface_kind: "node-resp".to_owned(),
        execution_mode: "real-daemon-tcp-resp-open-loop".to_owned(),
        state_scope: "node-local".to_owned(),
        network_boundary: "loopback-tcp".to_owned(),
        claim_scope: "selected-endpoint-capacity".to_owned(),
    };
    validate_daemon_exporter_surface(&w3).unwrap();
}

#[test]
fn canary_metrics_undercount_fixture_is_detected() {
    let error = w9_counter_undercount_canary_red().unwrap_err();
    assert!(error.contains(W9_CANARY_MARKER));
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W9") {
        panic!("{error}");
    }
}

#[test]
fn metrics_honesty_scenario_freezes_surface_reality_and_exporter_boundary() {
    let scenario = MetricsHonestyScenario::parse_toml(SCENARIO).unwrap();
    assert_eq!(scenario.exporter_path, "/metrics");
    assert!(scenario
        .coverage
        .w3_node_resp
        .operations
        .starts_with("not_available:"));
    assert!(scenario
        .coverage
        .w4a_control_plane_admin
        .topology
        .contains("hydracache_cluster_epoch"));
    assert_eq!(
        scenario.observer_probe.probe_id,
        "w9-w0-open-loop-observer-v1"
    );
    assert_eq!(scenario.observer_probe.offered_rate_per_second, 1_000);
    assert_eq!(scenario.observer_probe.operations, 64);

    let mut invented = scenario;
    invented.coverage.w3_node_resp.operations = "available:hydracache_cache_hits_total".to_owned();
    assert!(invented.validate().is_err());
}

#[test]
fn metrics_honesty_scenario_rejects_a_weakened_observer_probe() {
    let mut scenario = MetricsHonestyScenario::parse_toml(SCENARIO).unwrap();
    scenario.observer_probe.operations = 1;
    assert!(scenario.validate().is_err());
}
