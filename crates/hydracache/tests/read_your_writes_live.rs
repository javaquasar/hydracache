use hydracache::{
    live_read_your_writes, ClusterEpoch, PartitionId, QuorumPosture, ReplicatedValueRecord,
    ReplicationConfig, WriteWatermark,
};

fn config() -> ReplicationConfig {
    ReplicationConfig {
        replication_factor: 3,
        read_quorum: 2,
        write_quorum: 2,
        sync_backups: 1,
        async_backups: 1,
        max_replicated_entry_bytes: 1024,
        replicate_values: true,
    }
}

#[test]
fn read_your_writes_live_acked_write_visible_to_quorum_read_on_other_node() {
    let watermark = WriteWatermark::new(PartitionId::new(2), 12, ClusterEpoch::new(4));
    let replicas = vec![
        ReplicatedValueRecord::value(PartitionId::new(2), 12, ClusterEpoch::new(4), b"a"),
        ReplicatedValueRecord::value(PartitionId::new(2), 12, ClusterEpoch::new(4), b"b"),
    ];

    let result = live_read_your_writes(config(), watermark, replicas);

    assert_eq!(result.posture, QuorumPosture::Strong);
    assert!(!result.decision.requires_primary_fallback);
    assert_eq!(result.decision.record.unwrap().version, 12);
}

#[test]
fn read_your_writes_live_read_below_watermark_does_not_serve_stale() {
    let watermark = WriteWatermark::new(PartitionId::new(2), 12, ClusterEpoch::new(4));
    let replicas = vec![
        ReplicatedValueRecord::value(PartitionId::new(2), 11, ClusterEpoch::new(3), b"a"),
        ReplicatedValueRecord::value(PartitionId::new(2), 10, ClusterEpoch::new(3), b"b"),
    ];

    let result = live_read_your_writes(config(), watermark, replicas);

    assert!(result.decision.record.is_none());
    assert!(result.decision.requires_primary_fallback);
}

#[test]
#[ignore = "chaos gate: run with -- --ignored when exercising single-node failure"]
fn read_your_writes_live_ryw_holds_during_single_node_failure() {
    let watermark = WriteWatermark::new(PartitionId::new(2), 12, ClusterEpoch::new(4));
    let replicas = vec![
        ReplicatedValueRecord::value(PartitionId::new(2), 12, ClusterEpoch::new(4), b"a"),
        ReplicatedValueRecord::value(PartitionId::new(2), 12, ClusterEpoch::new(4), b"b"),
    ];
    assert!(
        !live_read_your_writes(config(), watermark, replicas)
            .decision
            .requires_primary_fallback
    );
}
