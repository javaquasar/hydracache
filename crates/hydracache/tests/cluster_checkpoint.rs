use std::time::Duration;

use hydracache::{
    recover_cluster_checkpoint, rescale_with_checkpoint, CheckpointCoordinator,
    ClusterCheckpointErrorKind, ClusterCheckpointManifest, ClusterEpoch,
    DurabilitySnapshotManifest, EffectiveReplicationMap, InMemoryReplicatedValueStore, MovePhase,
    NamespacePersistenceRule, NodeCheckpointManifest, PartitionId, PartitionMove,
    PersistencePolicy, PersistenceRegionPlacement, RecoveryErrorKind, RecoveryNamespace,
    RecoveryPolicy, Replicas, ReplicatedValueRecord, ReplicatedValueStore, RescaleCheckpointPhase,
    ReshardPlan, WriteWatermark, CLUSTER_CHECKPOINT_MANIFEST_FORMAT_VERSION,
};

#[test]
fn cluster_checkpoint_is_a_consistent_cut() {
    let epoch = ClusterEpoch::new(55);
    let partition_a = PartitionId::new(1);
    let partition_b = PartitionId::new(2);
    let barrier_a = WriteWatermark::new(partition_a, 10, epoch);
    let barrier_b = WriteWatermark::new(partition_b, 20, epoch);
    let mut coordinator = CheckpointCoordinator::new();

    let valid = coordinator
        .coordinate(
            "cut-1",
            epoch,
            [barrier_a, barrier_b],
            [
                NodeCheckpointManifest::new("node-a", vec![snapshot("default", barrier_a)]),
                NodeCheckpointManifest::new(
                    "node-b",
                    vec![snapshot(
                        "default",
                        WriteWatermark::new(partition_b, 21, epoch),
                    )],
                ),
            ],
            Duration::from_secs(5),
        )
        .unwrap();

    assert_eq!(
        valid.format_version,
        CLUSTER_CHECKPOINT_MANIFEST_FORMAT_VERSION
    );
    assert_eq!(valid.watermark_for_partition(partition_a), Some(barrier_a));
    assert_eq!(valid.watermark_for_partition(partition_b), Some(barrier_b));
    assert!(valid.covers(WriteWatermark::new(partition_b, 20, epoch)));
    assert!(!valid.covers(WriteWatermark::new(partition_b, 21, epoch)));
    assert_eq!(
        coordinator.latest_valid().unwrap().checkpoint_id,
        valid.checkpoint_id
    );

    let partial = coordinator
        .coordinate(
            "partial",
            epoch,
            [barrier_a, barrier_b],
            [NodeCheckpointManifest::new(
                "node-a",
                vec![snapshot("default", barrier_a)],
            )],
            Duration::from_secs(6),
        )
        .unwrap_err();
    assert_eq!(partial.kind(), ClusterCheckpointErrorKind::PartialCut);
    assert_eq!(coordinator.latest_valid().unwrap().checkpoint_id, "cut-1");

    let stale = coordinator
        .coordinate(
            "stale",
            epoch,
            [barrier_a, barrier_b],
            [
                NodeCheckpointManifest::new("node-a", vec![snapshot("default", barrier_a)]),
                NodeCheckpointManifest::new(
                    "node-b",
                    vec![snapshot(
                        "default",
                        WriteWatermark::new(partition_b, 19, epoch),
                    )],
                ),
            ],
            Duration::from_secs(7),
        )
        .unwrap_err();
    assert_eq!(stale.kind(), ClusterCheckpointErrorKind::StaleWatermark);
    assert_eq!(coordinator.rejected_total(), 2);
    assert_eq!(coordinator.latest_valid().unwrap().checkpoint_id, "cut-1");
}

#[test]
fn rescale_with_checkpoint_loses_no_committed_write() {
    let epoch = ClusterEpoch::new(55);
    let partition = PartitionId::new(7);
    let mut source = InMemoryReplicatedValueStore::default();
    let mut target = InMemoryReplicatedValueStore::default();
    for version in 1..=5 {
        source
            .upsert(
                format!("cache/item-{version}"),
                ReplicatedValueRecord::value(
                    partition,
                    version,
                    epoch,
                    format!("v{version}").into_bytes(),
                ),
            )
            .unwrap();
    }

    let barrier = WriteWatermark::new(partition, 5, epoch);
    let checkpoint = make_checkpoint("rescale-cut", epoch, [barrier], [("source", barrier)]);
    let reshard = ReshardPlan::new(
        epoch,
        vec![PartitionMove::new(
            partition,
            "source",
            "target",
            source.total_bytes(),
        )],
        1,
    );
    let mut flow = rescale_with_checkpoint(checkpoint, reshard).unwrap();
    flow.redistribute();

    let shadowed = ReplicatedValueRecord::value(partition, 6, epoch, b"shadowed".to_vec());
    for node in flow
        .reshard
        .write_targets_for_partition(partition)
        .unwrap_or_default()
    {
        if node.as_str() == "source" {
            source.upsert("cache/item-6", shadowed.clone()).unwrap();
        }
        if node.as_str() == "target" {
            target.upsert("cache/item-6", shadowed.clone()).unwrap();
        }
    }
    for (key, record) in source.scan_all().unwrap() {
        target.upsert(key, record).unwrap();
    }
    flow.reshard
        .record_backfill(partition, source.total_bytes());
    flow.reshard.moves[0].advance();
    flow.reshard.moves[0].advance();

    let resumed = hydracache::RescaleWithCheckpointPlan::resume_from(flow.snapshot()).unwrap();

    assert_eq!(resumed.phase, RescaleCheckpointPhase::Resumed);
    assert_eq!(resumed.reshard.moves[0].phase, MovePhase::Commit);
    for version in 1..=6 {
        assert!(
            target
                .get(&format!("cache/item-{version}"))
                .unwrap()
                .is_some(),
            "version {version} was lost"
        );
    }
}

#[test]
fn checkpoint_restore_reconciles_with_epoch_version_authority() {
    let epoch = ClusterEpoch::new(55);
    let partition = PartitionId::new(3);
    let mut store = InMemoryReplicatedValueStore::default();
    let before = ReplicatedValueRecord::value(partition, 9, epoch, b"before".to_vec());
    let at_barrier = ReplicatedValueRecord::value(partition, 10, epoch, b"at".to_vec());
    let after = ReplicatedValueRecord::value(partition, 11, epoch, b"after".to_vec());
    store.upsert("cache/before", before.clone()).unwrap();
    store.upsert("cache/at", at_barrier.clone()).unwrap();
    store.upsert("cache/after", after).unwrap();

    let barrier = WriteWatermark::new(partition, 10, epoch);
    let checkpoint = make_checkpoint("restore-cut", epoch, [barrier], [("node-a", barrier)]);
    let report = recover_cluster_checkpoint(
        &checkpoint,
        &store,
        &policy(),
        &"eu".into(),
        epoch,
        &RecoveryPolicy::full_recovery_only(),
        [
            RecoveryNamespace::new("default", placement(), replication_map())
                .with_key_prefix("cache/"),
        ],
    )
    .unwrap();

    assert_eq!(report.record("default", "cache/before"), Some(&before));
    assert_eq!(report.record("default", "cache/at"), Some(&at_barrier));
    assert!(report.record("default", "cache/after").is_none());
    assert_eq!(report.recovered_record_total, 2);
    assert_eq!(report.stale_fenced_total, 1);
    assert_eq!(
        report.namespaces["default"].stale_keys,
        vec!["cache/after".to_owned()]
    );

    let older = make_checkpoint(
        "older",
        ClusterEpoch::new(54),
        [WriteWatermark::new(partition, 10, ClusterEpoch::new(54))],
        [(
            "node-a",
            WriteWatermark::new(partition, 10, ClusterEpoch::new(54)),
        )],
    );
    let error = recover_cluster_checkpoint(
        &older,
        &store,
        &policy(),
        &"eu".into(),
        epoch,
        &RecoveryPolicy::full_recovery_only(),
        [
            RecoveryNamespace::new("default", placement(), replication_map())
                .with_key_prefix("cache/"),
        ],
    )
    .unwrap_err();
    assert_eq!(error.kind(), RecoveryErrorKind::Checkpoint);
    assert!(error.to_string().contains("older than authority epoch"));
}

#[test]
fn unknown_future_checkpoint_format_refuses_to_restore() {
    let epoch = ClusterEpoch::new(55);
    let partition = PartitionId::new(4);
    let barrier = WriteWatermark::new(partition, 1, epoch);
    let mut future = make_checkpoint("future", epoch, [barrier], [("node-a", barrier)]);
    future.format_version = CLUSTER_CHECKPOINT_MANIFEST_FORMAT_VERSION + 1;

    let error = recover_cluster_checkpoint(
        &future,
        &InMemoryReplicatedValueStore::default(),
        &policy(),
        &"eu".into(),
        epoch,
        &RecoveryPolicy::full_recovery_only(),
        [RecoveryNamespace::new(
            "default",
            placement(),
            replication_map(),
        )],
    )
    .unwrap_err();

    assert_eq!(error.kind(), RecoveryErrorKind::Checkpoint);
    assert!(error
        .to_string()
        .contains("unsupported cluster checkpoint format"));
}

fn make_checkpoint(
    id: &str,
    epoch: ClusterEpoch,
    watermarks: impl IntoIterator<Item = WriteWatermark>,
    node_watermarks: impl IntoIterator<Item = (&'static str, WriteWatermark)>,
) -> ClusterCheckpointManifest {
    ClusterCheckpointManifest::new(
        id,
        epoch,
        watermarks,
        node_watermarks.into_iter().map(|(node, watermark)| {
            NodeCheckpointManifest::new(node, vec![snapshot("default", watermark)])
        }),
        Duration::from_secs(1),
    )
    .unwrap()
}

fn snapshot(namespace: &str, watermark: WriteWatermark) -> DurabilitySnapshotManifest {
    DurabilitySnapshotManifest::new(
        namespace,
        watermark,
        Duration::from_millis(10),
        Duration::from_secs(1),
    )
}

fn policy() -> PersistencePolicy {
    PersistencePolicy::try_new([NamespacePersistenceRule::persistent("default").unwrap()]).unwrap()
}

fn placement() -> PersistenceRegionPlacement {
    PersistenceRegionPlacement::home_region_only("eu")
}

fn replication_map() -> EffectiveReplicationMap {
    EffectiveReplicationMap::new(Replicas::new("node-a", Vec::new()))
}
