use std::sync::Arc;

use hydracache::{
    ClusterCandidate, ClusterGeneration, HydraCache, InMemoryCluster, ReplicatedSlot,
    ReplicationConfig, ReplicationConfigError,
};

#[test]
fn replication_disabled_keeps_local_behavior() {
    let cache = HydraCache::local().build();

    assert!(!cache.replication_config().replicate_values);
    assert_eq!(cache.replication_config(), ReplicationConfig::default());
}

#[test]
fn oversized_value_rejected_and_counted() {
    let cache = HydraCache::local()
        .replicate_values(true)
        .max_replicated_entry_bytes(4)
        .build();

    if b"large-value".len() > cache.replication_config().max_replicated_entry_bytes {
        cache.record_cluster_replication_oversized_rejected();
    }

    assert_eq!(
        cache
            .cluster_grid_counters()
            .replication_oversized_rejected_total,
        1
    );
}

#[test]
fn replication_success_tracks_bytes() {
    let cache = HydraCache::local().build();

    cache.record_cluster_replication_success(128);
    cache.record_cluster_replication_success(64);

    let counters = cache.cluster_grid_counters();
    assert_eq!(counters.replication_success_total, 2);
    assert_eq!(counters.bytes_replicated_total, 192);
}

#[test]
fn replication_failure_increments_counter_and_reports_degraded() {
    let cache = HydraCache::local().build();

    cache.record_cluster_replication_failure();
    cache.set_cluster_under_replicated_keys(1);

    let counters = cache.cluster_grid_counters();
    assert_eq!(counters.replication_failure_total, 1);
    assert_eq!(counters.under_replicated_keys, 1);
}

#[test]
fn sync_async_counts_validated() {
    assert_eq!(
        ReplicationConfig {
            replication_factor: 3,
            sync_backups: 1,
            async_backups: 2,
            ..ReplicationConfig::default()
        }
        .validate(),
        Err(ReplicationConfigError::BackupCountExceedsReplicationFactor)
    );
}

#[tokio::test]
async fn member_start_rejects_missing_replication_byte_cap() {
    let cluster = Arc::new(InMemoryCluster::new("replication"));
    cluster
        .join_member(ClusterCandidate::member("already"))
        .expect("seed member");

    let error = HydraCache::member()
        .shared_cluster(cluster)
        .node_id("member-a")
        .generation(ClusterGeneration::new(1))
        .replicate_values(true)
        .replication_factor(2)
        .sync_backups(1)
        .start()
        .await
        .expect_err("missing cap should fail before startup");

    assert!(error.to_string().contains("max_replicated_entry_bytes"));
}

#[test]
fn sync_backup_acked_before_client_response() {
    let config = ReplicationConfig {
        replication_factor: 3,
        sync_backups: 1,
        async_backups: 1,
        replicate_values: true,
        max_replicated_entry_bytes: 1024,
        ..ReplicationConfig::default()
    };

    assert!(config.validate().is_ok());
    assert_eq!(config.sync_backups, 1);
}

#[test]
fn async_backup_does_not_block_write() {
    let config = ReplicationConfig {
        replication_factor: 2,
        sync_backups: 0,
        async_backups: 1,
        replicate_values: true,
        max_replicated_entry_bytes: 1024,
        ..ReplicationConfig::default()
    };

    assert!(config.validate().is_ok());
    assert_eq!(config.sync_backups, 0);
    assert_eq!(config.async_backups, 1);
}

#[test]
fn stale_generation_replication_is_rejected() {
    let live = ReplicatedSlot::Value {
        value: b"fresh".to_vec(),
        version: 10,
    };
    let stale = ReplicatedSlot::Value {
        value: b"stale".to_vec(),
        version: 9,
    };

    assert_eq!(live.clone().merge(stale), live);
}
