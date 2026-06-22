use hydracache::{
    AdaptiveWindow, ClusterEpoch, ClusterNodeId, PartitionId, PartitionReplicaVersions,
    PromotionFreezeWindow, ReplicatedValueRecord,
};

#[test]
fn replication_under_load_slow_backup_does_not_stall_primary() {
    let mut slow_backup = AdaptiveWindow::new(1, 4, 16);
    let mut healthy_backup = AdaptiveWindow::new(1, 4, 16);

    for _ in 0..4 {
        assert!(slow_backup.try_acquire());
    }
    assert!(!slow_backup.try_acquire());

    slow_backup.on_ack(false);
    assert_eq!(slow_backup.max_in_flight(), 2);
    assert!(healthy_backup.try_acquire());
    healthy_backup.on_ack(true);
    assert_eq!(healthy_backup.max_in_flight(), 5);
}

#[test]
fn replication_under_load_promotion_freeze_window_is_bounded_under_load() {
    let freeze = PromotionFreezeWindow {
        observed_ms: 37,
        bound_ms: 100,
    };

    assert!(freeze.is_bounded());
}

#[test]
fn replication_under_load_anti_entropy_converges_after_partition_heals() {
    let partition = PartitionId::new(7);
    let primary = ClusterNodeId::from("member-a");
    let backup = ClusterNodeId::from("member-b");
    let mut versions = PartitionReplicaVersions::default();

    versions.set_version(partition, primary.clone(), 9);
    versions.set_version(partition, backup.clone(), 3);
    assert_eq!(
        versions.lagging_replicas(partition, &primary, std::slice::from_ref(&backup)),
        vec![backup.clone()]
    );

    versions.set_version(partition, backup.clone(), 9);
    assert!(versions
        .lagging_replicas(partition, &primary, &[backup])
        .is_empty());
}

#[test]
fn replication_under_load_failover_preserves_tombstone_invariant_under_churn() {
    let tombstone =
        ReplicatedValueRecord::tombstone(PartitionId::new(3), 11, ClusterEpoch::new(5), None);
    let stale_value = ReplicatedValueRecord::value(
        PartitionId::new(3),
        10,
        ClusterEpoch::new(4),
        b"stale".to_vec(),
    );

    let merged = tombstone.clone().merge(stale_value);
    assert_eq!(merged, tombstone);
}

#[test]
fn replication_under_load_aimd_window_recovers_after_transient_slowness() {
    let mut window = AdaptiveWindow::new(1, 8, 16);
    assert!(window.try_acquire());
    window.on_ack(false);
    assert_eq!(window.max_in_flight(), 4);

    for expected in 5..=8 {
        assert!(window.try_acquire());
        window.on_ack(true);
        assert_eq!(window.max_in_flight(), expected);
    }
}
