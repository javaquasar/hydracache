use hydracache::{cluster_grid_metric_descriptors, CacheOptions, HydraCache};
use hydracache_observability::{registered_metric_names, HydraCacheRegistry, PrometheusExporter};

#[tokio::test]
async fn exporter_prometheus_text_contains_registered_cache_metrics_without_key_labels() {
    let cache = HydraCache::local().build();
    cache
        .get_or_insert_with("user:42", CacheOptions::new(), || async { 42_u64 })
        .await
        .unwrap();
    cache
        .get_or_insert_with("user:42", CacheOptions::new(), || async { 7_u64 })
        .await
        .unwrap();
    let exporter = PrometheusExporter::new(HydraCacheRegistry::new().with_cache("main", cache));

    let text = exporter.render().await;

    assert!(text.contains("# TYPE hydracache_cache_hits_total counter"));
    assert!(text.contains("hydracache_cache_hits_total{cache=\"main\"} 1"));
    assert!(text.contains("hydracache_cache_misses_total{cache=\"main\"} 1"));
    assert!(text.contains("hydracache_cache_loads_total{cache=\"main\"} 1"));
    assert!(text.contains("hydracache_cache_hit_ratio{cache=\"main\"} 0.500000"));
    assert!(!text.contains("user:42"));
}

#[test]
fn exporter_registered_metric_names_cover_grid_and_admission_surface() {
    let names = registered_metric_names();

    assert!(names.contains("hydracache_cache_hits_total"));
    assert!(names.contains("hydracache_admission_rejected_total"));
    for descriptor in cluster_grid_metric_descriptors() {
        assert!(
            names.contains(descriptor.name),
            "missing registered grid metric {}",
            descriptor.name
        );
    }
}

#[test]
fn exporter_dashboard_and_alert_rules_reference_registered_metric_names() {
    let alerts = include_str!("../../../deploy/dashboards/prometheus-alerts.yml");
    let grafana = include_str!("../../../deploy/dashboards/grafana-overview.json");
    let names = registered_metric_names();

    for required in [
        "hydracache_cache_hit_ratio",
        "hydracache_replication_backpressure_total",
        "hydracache_admission_rejected_total",
    ] {
        assert!(names.contains(required));
        assert!(alerts.contains(required), "alerts missing {required}");
        assert!(grafana.contains(required), "dashboard missing {required}");
    }
}
