use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use hydracache::{cluster_grid_metric_descriptors, RegionState};
use hydracache_observability::{
    geo_metric_names, GeoStalenessSlo, GeoStatus, LinkHealth, RegionHealth,
};

#[test]
fn geo_observability_status_is_read_only_and_complete() {
    let regions = vec![
        RegionHealth::new("us", RegionState::Up, 1, 80),
        RegionHealth::new("eu", RegionState::Up, 0, 40),
    ];
    let links = vec![LinkHealth::new("eu", "us", 1, 4, 2048, false)];
    let first = GeoStatus::from_signals(
        regions.clone(),
        links.clone(),
        true,
        512,
        GeoStalenessSlo::new(100),
    );
    let second = GeoStatus::from_signals(regions, links, true, 512, GeoStalenessSlo::new(100));

    assert_eq!(first, second);
    assert!(first.is_healthy());
    assert_eq!(first.regions[0].region.as_str(), "eu");
    assert_eq!(first.worst_staleness_window_ms, 80);
    let json = serde_json::to_value(&first).unwrap();
    for field in [
        "regions",
        "links",
        "active_active_acked",
        "worst_staleness_window_ms",
        "crdt_metadata_bytes",
        "staleness_slo_target_ms",
        "staleness_slo_breached",
    ] {
        assert!(json.get(field).is_some(), "missing geo field {field}");
    }
}

#[test]
fn geo_observability_staleness_window_is_measured_and_breach_alerts() {
    let slo = GeoStalenessSlo::new(100);
    let status = GeoStatus::from_signals(
        vec![RegionHealth::new("eu", RegionState::Up, 0, 250)],
        Vec::new(),
        true,
        0,
        slo,
    );
    let evaluation = slo.evaluate(&status);

    assert_eq!(status.worst_staleness_window_ms, 250);
    assert!(status.staleness_slo_breached);
    assert!(evaluation.breached);
    assert!(!status.is_healthy());
}

#[test]
fn geo_observability_series_honor_cardinality_rule() {
    let forbidden = ["partition_id", "key", "replica_index"];
    let geo_metrics = geo_metric_names().iter().copied().collect::<BTreeSet<_>>();
    let descriptors = cluster_grid_metric_descriptors()
        .iter()
        .filter(|metric| geo_metrics.contains(metric.name))
        .collect::<Vec<_>>();

    assert_eq!(descriptors.len(), geo_metrics.len());
    for descriptor in descriptors {
        for label in descriptor.labels {
            assert!(
                !forbidden.contains(label),
                "metric {} exports forbidden high-cardinality label {label}",
                descriptor.name
            );
        }
    }
}

#[test]
fn geo_observability_alert_rules_reference_existing_metrics() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let alerts =
        fs::read_to_string(root.join("docs/cluster/dashboards/geo/prometheus-alerts.yml")).unwrap();
    let registered = cluster_grid_metric_descriptors()
        .iter()
        .map(|metric| metric.name)
        .collect::<BTreeSet<_>>();

    let referenced = alerts
        .lines()
        .filter_map(|line| line.trim().strip_prefix("expr:"))
        .filter_map(|expr| {
            expr.trim()
                .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                .find(|token| token.starts_with("hydracache_"))
        })
        .collect::<BTreeSet<_>>();

    assert!(!referenced.is_empty());
    for metric in referenced {
        assert!(
            registered.contains(metric),
            "geo alert references unregistered metric {metric}"
        );
    }
}

#[test]
fn geo_observability_dashboard_references_geo_metrics() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let dashboard =
        fs::read_to_string(root.join("docs/cluster/dashboards/geo/grafana-geo.json")).unwrap();

    for metric in geo_metric_names() {
        assert!(
            dashboard.contains(metric),
            "geo dashboard is missing metric {metric}"
        );
    }
}
