use hydracache::{CacheOptions, HydraCache};

#[tokio::test]
async fn local_only_fallback_works_without_cluster_runtime() {
    let cache = HydraCache::local().build();

    cache
        .put("user:1", "Ada".to_owned(), CacheOptions::new().tag("users"))
        .await
        .unwrap();
    assert_eq!(
        cache.get::<String>("user:1").await.unwrap(),
        Some("Ada".to_owned())
    );
    assert_eq!(cache.invalidate_tag("users").await.unwrap(), 1);
    assert_eq!(cache.get::<String>("user:1").await.unwrap(), None);
    assert!(cache.cluster_diagnostics().is_none());
}

#[test]
fn disabling_read_through_stops_remote_peer_fetch() {
    let cache = HydraCache::local().read_through_enabled(false).build();

    cache.record_cluster_direct_remote_fetch().unwrap();

    assert_eq!(cache.cluster_fill_counters().remote_fetch_success, 0);
    assert_eq!(cache.cluster_fill_counters().hot_cache_hits, 0);
}

#[test]
fn health_report_shows_degraded_cluster_mode() {
    let cache = HydraCache::local()
        .transport_auth_configured(true)
        .strict_wire_compatibility(true)
        .build();

    let report = cache.cluster_pilot_report();

    assert!(!report.readiness.is_pilot_ready());
    assert!(!report.readiness.has_members);
    assert_eq!(report.epoch, 0);
}
