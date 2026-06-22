use hydracache::{
    partition_for_key, validate_replica_config, ClusterReplicaConfigError, HydraCache,
    InMemoryCluster,
};

#[test]
fn owner_load_counter_increments_only_owner_path() {
    let cache = HydraCache::local().build();

    cache.record_cluster_owner_load_success();

    let counters = cache.cluster_fill_counters();
    assert_eq!(counters.owner_load_success, 1);
    assert_eq!(counters.remote_fetch_success, 0);
    assert_eq!(counters.hot_cache_hits, 0);
}

#[test]
fn remote_fetch_counter_increments_only_remote_path() {
    let cache = HydraCache::local().build();

    cache.record_cluster_remote_fetch_success();

    let counters = cache.cluster_fill_counters();
    assert_eq!(counters.owner_load_success, 0);
    assert_eq!(counters.remote_fetch_success, 1);
    assert_eq!(counters.hot_cache_hits, 0);
}

#[test]
fn hot_cache_hit_counter_increments_only_hot_path() {
    let cache = HydraCache::local().build();

    cache.record_cluster_hot_cache_hit();

    let counters = cache.cluster_fill_counters();
    assert_eq!(counters.owner_load_success, 0);
    assert_eq!(counters.remote_fetch_success, 0);
    assert_eq!(counters.hot_cache_hits, 1);
}

#[test]
fn partition_indirection_is_deterministic() {
    let partition = partition_for_key("tenant:1:user:42", 271);
    let same_partition = partition_for_key("tenant:1:user:42", 271);
    let single_partition = partition_for_key("tenant:1:user:42", 0);

    assert_eq!(partition, same_partition);
    assert!(partition.value() < 271);
    assert_eq!(single_partition.value(), 0);
}

#[test]
fn owner_resolution_is_deterministic_for_same_topology() {
    let cluster = InMemoryCluster::new("partition");
    cluster
        .join_member(hydracache::ClusterCandidate::member("member-a"))
        .unwrap();
    cluster
        .join_member(hydracache::ClusterCandidate::member("member-b"))
        .unwrap();

    let first = cluster.owner_for_key("tenant:1:user:42");
    let second = cluster.owner_for_key("tenant:1:user:42");

    assert_eq!(first.owner_node_id(), second.owner_node_id());
    assert_eq!(first.member_count, 2);
}

#[test]
fn validate_replica_rejects_quorum_zero() {
    assert_eq!(
        validate_replica_config(1, 1, 0),
        Err(ClusterReplicaConfigError::QuorumZero)
    );
}

#[test]
fn validate_replica_rejects_quorum_above_rf() {
    assert_eq!(
        validate_replica_config(1, 1, 2),
        Err(ClusterReplicaConfigError::QuorumExceedsReplication)
    );
}

#[test]
fn validate_replica_rejects_min_replica_zero() {
    assert_eq!(
        validate_replica_config(0, 1, 1),
        Err(ClusterReplicaConfigError::MinReplicaZero)
    );
}

#[test]
fn validate_replica_accepts_pilot_single_owner_shape() {
    assert_eq!(validate_replica_config(1, 1, 1), Ok(()));
}
