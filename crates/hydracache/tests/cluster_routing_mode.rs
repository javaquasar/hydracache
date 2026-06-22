use hydracache::{HydraCache, RoutingMode};

#[test]
fn direct_routes_to_computed_owner_path() {
    let cache = HydraCache::local()
        .routing_mode(RoutingMode::Direct)
        .build();

    cache.record_cluster_direct_remote_fetch().unwrap();

    assert_eq!(cache.routing_mode(), RoutingMode::Direct);
    assert_eq!(cache.cluster_fill_counters().remote_fetch_success, 1);
    assert_eq!(cache.cluster_fill_counters().hot_cache_hits, 0);
}

#[test]
fn single_endpoint_always_uses_gateway_path() {
    let cache = HydraCache::local()
        .routing_mode(RoutingMode::SingleEndpoint)
        .build();

    cache.record_cluster_direct_remote_fetch().unwrap();

    assert_eq!(cache.routing_mode(), RoutingMode::SingleEndpoint);
    assert_eq!(cache.cluster_fill_counters().remote_fetch_success, 0);
    assert_eq!(cache.cluster_fill_counters().hot_cache_hits, 1);
}

#[test]
fn direct_degrades_when_read_through_disabled() {
    let cache = HydraCache::local()
        .routing_mode(RoutingMode::Direct)
        .read_through_enabled(false)
        .build();

    cache.record_cluster_direct_remote_fetch().unwrap();

    assert!(!cache.read_through_enabled());
    assert_eq!(cache.cluster_fill_counters().remote_fetch_success, 0);
    assert_eq!(cache.cluster_fill_counters().hot_cache_hits, 0);
}
