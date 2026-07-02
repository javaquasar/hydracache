use hydracache::{
    cluster_grid_metric_descriptors, AdmissionSnapshot, CacheOptions, ClusterGridCounters,
    HydraCache,
};
use hydracache_observability::{
    registered_metric_names, ClusterTopologyOverview, HydraCacheOverview, HydraCacheRegistry,
    PrometheusExporter, TopologyReshardPhase, TopologyStatusSource,
};

mod exporter {
    use super::*;

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
    fn every_registered_metric_name_is_emitted() {
        let text = PrometheusExporter::render_overview(&HydraCacheOverview::default());

        for name in registered_metric_names() {
            assert_eq!(
                text.matches(&format!("# HELP {name} ")).count(),
                1,
                "registered metric {name} has missing or duplicate HELP"
            );
            assert_eq!(
                text.matches(&format!("# TYPE {name} ")).count(),
                1,
                "registered metric {name} has missing or duplicate TYPE"
            );
        }
    }

    #[test]
    fn admission_and_cluster_series_render_with_type_headers() {
        let mut counters = ClusterGridCounters::default();
        counters.replication_success_total = 4;
        counters.under_replicated_keys = 2;
        counters.hints_stored_total = 9;
        let overview = HydraCacheOverview::default()
            .with_admission_snapshot(AdmissionSnapshot {
                in_flight: 2,
                memory_bytes: 256,
                queue_depth: 3,
                rejected_total: 5,
            })
            .with_cluster_grid_counters(counters);

        let text = PrometheusExporter::render_overview(&overview);

        assert!(text.contains("# TYPE hydracache_admission_rejected_total counter"));
        assert!(text.contains("hydracache_admission_rejected_total 5"));
        assert!(text.contains("# TYPE hydracache_admission_in_flight gauge"));
        assert!(text.contains("hydracache_admission_in_flight 2"));
        assert!(text.contains("# TYPE hydracache_replication_success_total counter"));
        assert!(text.contains(
            "hydracache_replication_success_total{role=\"aggregate\",outcome=\"aggregate\"} 4"
        ));
        assert!(text.contains("# TYPE hydracache_under_replicated_keys gauge"));
        assert!(text.contains("hydracache_under_replicated_keys 2"));
        assert!(text.contains("hydracache_hints_stored_total 9"));
    }

    #[tokio::test]
    async fn exporter_labels_are_bounded() {
        let cache = HydraCache::local().build();
        cache
            .get_or_insert_with("secret-user-key:42", CacheOptions::new(), || async {
                42_u64
            })
            .await
            .unwrap();
        let exporter = PrometheusExporter::new(HydraCacheRegistry::new().with_cache("main", cache));

        let text = exporter.render().await;

        for forbidden in ["key=", "request_id=", "session_id=", "partition_id="] {
            assert!(
                !text.contains(forbidden),
                "exporter emitted forbidden high-cardinality label {forbidden}"
            );
        }
        assert!(!text.contains("secret-user-key:42"));
    }

    #[tokio::test]
    async fn empty_registry_and_zero_members_render_valid_exposition() {
        let exporter = PrometheusExporter::new(HydraCacheRegistry::new());

        let text = exporter.render().await;

        assert!(text.contains("hydracache_admission_queue_depth 0"));
        assert!(text.contains("hydracache_cluster_members{source=\"modeled\"} 0"));
        assert!(text.contains("hydracache_cluster_leader{source=\"modeled\",node=\"none\"} 0"));
        assert!(!text.contains("NaN"));
        assert!(!text.contains("+Inf"));
        assert!(!text.contains("-Inf"));
    }

    #[test]
    fn topology_gauges_carry_source_label() {
        let overview = HydraCacheOverview::default()
            .with_topology(ClusterTopologyOverview::new(
                TopologyStatusSource::Live,
                3,
                Some("node-2".to_owned()),
                42,
                TopologyReshardPhase::Moving,
            ))
            .with_backup_age_seconds(17);

        let text = PrometheusExporter::render_overview(&overview);

        assert!(text.contains("hydracache_cluster_members{source=\"live\"} 3"));
        assert!(text.contains("hydracache_cluster_leader{source=\"live\",node=\"node-2\"} 1"));
        assert!(text.contains("hydracache_cluster_epoch{source=\"live\"} 42"));
        assert!(
            text.contains("hydracache_cluster_reshard_phase{source=\"live\",phase=\"moving\"} 1")
        );
        assert!(text.contains("hydracache_backup_age_seconds{source=\"live\"} 17"));
    }

    #[test]
    fn exporter_registered_metric_names_cover_grid_and_admission_surface() {
        let names = registered_metric_names();

        assert!(names.contains("hydracache_cache_hits_total"));
        assert!(names.contains("hydracache_admission_rejected_total"));
        assert!(names.contains("hydracache_cluster_members"));
        assert!(names.contains("hydracache_backup_age_seconds"));
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
}
