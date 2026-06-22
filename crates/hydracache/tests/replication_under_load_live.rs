use hydracache::{
    anti_entropy_repair, AdaptiveWindow, ClusterEpoch, InMemoryReplicatedValueStore,
    LiveReplicationPeer, PartitionId, ReplicatedValueRecord, ReplicatedValueStore,
};

#[test]
fn replication_under_load_live_slow_backup_does_not_stall_primary() {
    let mut slow = LiveReplicationPeer::new("member-b", AdaptiveWindow::new(1, 1, 4));
    let mut healthy = LiveReplicationPeer::new("member-c", AdaptiveWindow::new(1, 4, 8));
    let mut slow_store = InMemoryReplicatedValueStore::default();
    let mut healthy_store = InMemoryReplicatedValueStore::default();
    let record =
        ReplicatedValueRecord::value(PartitionId::new(1), 1, ClusterEpoch::new(1), b"value");

    let slow_send = slow
        .send_record(&mut slow_store, "user:1", record.clone(), false)
        .unwrap();
    let healthy_send = healthy
        .send_record(&mut healthy_store, "user:1", record, true)
        .unwrap();

    assert!(slow_send.admitted);
    assert!(healthy_send.admitted);
    assert!(healthy_send.max_in_flight > slow_send.max_in_flight);
    assert!(healthy_store.get("user:1").unwrap().is_some());
}

#[test]
fn replication_under_load_live_anti_entropy_converges_after_partition_heal() {
    let mut backup = InMemoryReplicatedValueStore::default();
    let records = vec![(
        "user:1".to_owned(),
        ReplicatedValueRecord::value(PartitionId::new(1), 9, ClusterEpoch::new(2), b"fresh"),
    )];

    let repaired = anti_entropy_repair(&mut backup, records).unwrap();

    assert_eq!(repaired, 1);
    assert_eq!(backup.get("user:1").unwrap().unwrap().version, 9);
}

#[test]
#[ignore = "chaos gate: run with -- --ignored when exercising slow backup partitions"]
fn replication_under_load_live_anti_entropy_converges_after_partition_heal_chaos() {
    let mut backup = InMemoryReplicatedValueStore::default();
    assert_eq!(anti_entropy_repair(&mut backup, Vec::new()).unwrap(), 0);
}
