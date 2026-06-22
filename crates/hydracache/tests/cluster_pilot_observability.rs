use std::sync::Arc;

use hydracache::{ClusterGeneration, HydraCache, InMemoryCluster};

#[tokio::test]
async fn metrics_increment_on_success_and_failure_paths() {
    let cluster = Arc::new(InMemoryCluster::new("pilot-observability"));
    let _other = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .transport_auth_configured(true)
        .strict_wire_compatibility(true)
        .start()
        .await
        .unwrap();
    let cache = HydraCache::member()
        .shared_cluster(cluster)
        .node_id("member-b")
        .generation(ClusterGeneration::new(1))
        .transport_auth_configured(true)
        .strict_wire_compatibility(true)
        .start()
        .await
        .unwrap();

    cache.record_cluster_owner_load_success();
    cache.record_cluster_owner_load_error();
    cache.record_cluster_remote_fetch_success();
    cache.record_cluster_remote_fetch_error();
    cache.record_cluster_hot_cache_hit();
    cache.record_cluster_peer_fetch_auth_failure();
    cache.record_cluster_wire_version_rejection();
    cache.record_cluster_stale_generation_rejected();
    cache.record_cluster_barrier_timeout();
    cache.record_cluster_near_cache_conservative_invalidation();
    cache.record_cluster_lifecycle_restart();
    cache.record_cluster_lifecycle_stop();

    let report = cache.cluster_pilot_report();

    assert_eq!(report.counters.owner_load_total, 1);
    assert_eq!(report.counters.remote_fetch_total, 1);
    assert_eq!(report.counters.hot_cache_hit_total, 1);
    assert_eq!(report.owner_load_errors, 1);
    assert_eq!(report.remote_fetch_errors, 1);
    assert_eq!(report.auth_failures, 1);
    assert_eq!(report.wire_version_failures, 1);
    assert_eq!(report.stale_generation_rejections, 1);
    assert_eq!(report.barrier_timeouts, 1);
    assert_eq!(report.near_cache_conservative_invalidations, 1);
    assert_eq!(report.lifecycle_restart_count, 1);
    assert_eq!(report.lifecycle_stop_count, 1);
    assert!(report.readiness.is_pilot_ready());
}

#[test]
fn local_only_report_is_not_pilot_ready_but_is_serializable() {
    let cache = HydraCache::local().build();

    let report = serde_json::to_value(cache.cluster_pilot_report()).unwrap();

    assert_eq!(report["readiness"]["has_members"], false);
    assert_eq!(report["epoch"], 0);
    assert_eq!(report["highlights"][0], "AUTH MISSING");
}
