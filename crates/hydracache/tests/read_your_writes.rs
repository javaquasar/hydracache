use hydracache::{
    quorum_read_your_writes, ClusterEpoch, PartitionId, QuorumPosture, ReplicatedValueRecord,
    ReplicationConfig, WriteWatermark,
};

#[test]
fn read_your_writes_acked_write_is_visible_to_quorum_read() {
    let watermark = WriteWatermark::new(PartitionId::new(5), 9, ClusterEpoch::new(2));
    let replicas = vec![
        ReplicatedValueRecord::value(PartitionId::new(5), 9, ClusterEpoch::new(2), b"a".to_vec()),
        ReplicatedValueRecord::value(PartitionId::new(5), 9, ClusterEpoch::new(2), b"b".to_vec()),
        ReplicatedValueRecord::value(
            PartitionId::new(5),
            7,
            ClusterEpoch::new(1),
            b"old".to_vec(),
        ),
    ];

    let decision = quorum_read_your_writes(watermark, replicas, 2);
    assert!(!decision.requires_primary_fallback);
    assert_eq!(decision.record.unwrap().version, 9);
}

#[test]
fn read_your_writes_below_watermark_does_not_serve_stale() {
    let watermark = WriteWatermark::new(PartitionId::new(5), 9, ClusterEpoch::new(2));
    let replicas = vec![
        ReplicatedValueRecord::value(PartitionId::new(5), 8, ClusterEpoch::new(1), b"a".to_vec()),
        ReplicatedValueRecord::value(PartitionId::new(5), 7, ClusterEpoch::new(1), b"b".to_vec()),
    ];

    let decision = quorum_read_your_writes(watermark, replicas, 2);
    assert!(decision.record.is_none());
    assert!(decision.requires_primary_fallback);
}

#[test]
fn read_your_writes_quorum_overlap_validated_at_startup() {
    let strong = ReplicationConfig {
        replication_factor: 3,
        read_quorum: 2,
        write_quorum: 2,
        sync_backups: 1,
        async_backups: 1,
        max_replicated_entry_bytes: 1024,
        replicate_values: true,
    };
    let degraded = ReplicationConfig {
        read_quorum: 1,
        write_quorum: 1,
        ..strong
    };

    assert_eq!(strong.quorum_posture(), QuorumPosture::Strong);
    assert_eq!(
        degraded.quorum_posture(),
        QuorumPosture::DegradedSessionRyow
    );
}

#[test]
fn read_your_writes_holds_during_single_node_failure_when_quorum_reachable() {
    let watermark = WriteWatermark::new(PartitionId::new(2), 12, ClusterEpoch::new(4));
    let reachable = vec![
        ReplicatedValueRecord::value(PartitionId::new(2), 12, ClusterEpoch::new(4), b"a".to_vec()),
        ReplicatedValueRecord::value(PartitionId::new(2), 12, ClusterEpoch::new(4), b"b".to_vec()),
    ];

    let decision = quorum_read_your_writes(watermark, reachable, 2);
    assert!(decision.record.is_some());
    assert!(!decision.requires_primary_fallback);
}
