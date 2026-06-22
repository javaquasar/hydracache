use hydracache::{ClusterNodeId, HotCacheDirectory, HydraCache};

#[test]
fn three_counters_increment_independently() {
    let cache = HydraCache::local().build();

    cache.record_cluster_owner_load_success();
    cache.record_cluster_remote_fetch_success();
    cache.record_cluster_hot_cache_hit();

    let counters = cache.cluster_fill_counters();
    assert_eq!(counters.owner_load_success, 1);
    assert_eq!(counters.remote_fetch_success, 1);
    assert_eq!(counters.hot_cache_hits, 1);
}

#[test]
fn hot_copy_invalidated_before_ttl() {
    let mut directory = HotCacheDirectory::default();
    directory.record_holder("user:42", "member-b");

    let holders = directory.invalidate("user:42");

    assert_eq!(holders, vec![ClusterNodeId::from("member-b")]);
    assert!(directory.holders("user:42").is_empty());
}

#[test]
fn full_fanout_reaches_all_holders() {
    let mut directory = HotCacheDirectory::default();
    directory.record_holder("user:42", "member-c");
    directory.record_holder("user:42", "member-b");
    directory.record_holder("user:42", "member-b");

    let holders = directory.invalidate("user:42");

    assert_eq!(
        holders,
        vec![
            ClusterNodeId::from("member-b"),
            ClusterNodeId::from("member-c")
        ]
    );
}
