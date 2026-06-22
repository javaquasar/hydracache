use hydracache::{
    AntiEntropyTask, ClusterEpoch, ClusterNodeId, PartitionId, PartitionReplicaVersions,
    TombstoneBudget, TombstoneTracker,
};

#[test]
fn lagging_backup_is_caught_up_by_task() {
    let partition = PartitionId::new(4);
    let primary = ClusterNodeId::from("member-a");
    let backup = ClusterNodeId::from("member-b");
    let mut versions = PartitionReplicaVersions::default();
    versions.set_version(partition, primary.clone(), 10);
    versions.set_version(partition, backup.clone(), 7);

    let lagging = versions.lagging_replicas(partition, &primary, std::slice::from_ref(&backup));
    assert_eq!(lagging, vec![backup.clone()]);

    versions.set_version(partition, backup.clone(), 10);
    assert!(versions
        .lagging_replicas(partition, &primary, &[backup])
        .is_empty());
}

#[test]
fn repair_confirmation_unblocks_tombstone_gc() {
    let mut tracker = TombstoneTracker::new(TombstoneBudget::new(1, 128));
    tracker.admit("user:1", 1, 64, None);
    tracker.admit("user:2", 2, 64, None);
    assert!(tracker.repair_debt());

    tracker.confirm_repair("user:1", ClusterEpoch::new(5));
    tracker.admit("user:3", 3, 64, None);

    assert!(!tracker.contains_key("user:1"));
    assert!(tracker.contains_key("user:2"));
    assert!(tracker.contains_key("user:3"));
}

#[test]
fn under_replicated_key_is_reported() {
    let cache = hydracache::HydraCache::local().build();
    cache.set_cluster_under_replicated_keys(2);

    assert_eq!(cache.cluster_grid_counters().under_replicated_keys, 2);
}

#[test]
fn anti_entropy_interval_is_normalized() {
    let task = AntiEntropyTask::new(std::time::Duration::ZERO);

    assert_eq!(task.interval, std::time::Duration::from_secs(1));
}
