use std::time::Duration;

use hydracache::{
    ClusterEpoch, DurabilityErrorKind, DurabilitySnapshotManifest, DurabilityWritePath,
    DurableWriteOutcome, InMemoryReplicatedValueStore, NamespacePersistenceRule,
    NamespacePersistenceSettings, PartitionId, PersistenceDurability, PersistencePolicy,
    PersistenceRegionPlacement, ReplicatedValueRecord, ReplicatedValueStore, WriteWatermark,
    DURABILITY_SNAPSHOT_FORMAT_VERSION,
};

fn record(version: u64) -> ReplicatedValueRecord {
    ReplicatedValueRecord::value(
        PartitionId::new(1),
        version,
        ClusterEpoch::new(3),
        format!("value-{version}").into_bytes(),
    )
}

fn policy(settings: NamespacePersistenceSettings) -> PersistencePolicy {
    PersistencePolicy::try_new([NamespacePersistenceRule::new("durable", settings).unwrap()])
        .unwrap()
}

#[test]
fn default_store_durability_state_machine_preserves_ack_and_queue_invariants() {
    let placement = PersistenceRegionPlacement::home_region_only("eu");
    let mut sync = DurabilityWritePath::new(
        InMemoryReplicatedValueStore::default(),
        policy(
            NamespacePersistenceSettings::persistent().with_durability(PersistenceDurability::Sync),
        ),
        "eu",
        placement.clone(),
    );
    assert_eq!(
        sync.write("durable", "key", record(1)).unwrap(),
        DurableWriteOutcome::SyncAcked {
            watermark: WriteWatermark::new(PartitionId::new(1), 1, ClusterEpoch::new(3)),
            fsync_before_ack: true,
        }
    );
    assert_eq!(sync.metrics().sync_write_total, 1);
    assert!(sync.store().get("key").unwrap().is_some());
    sync.store_mut().remove("key").unwrap();
    assert!(sync.into_store().get("key").unwrap().is_none());

    let settings = NamespacePersistenceSettings::persistent()
        .with_durability(PersistenceDurability::AsyncBounded { max_lag: 2 })
        .with_snapshot_interval(Duration::from_secs(10));
    let mut asynchronous = DurabilityWritePath::new(
        InMemoryReplicatedValueStore::default(),
        policy(settings),
        "eu",
        placement,
    );
    assert_eq!(
        asynchronous.write("durable", "key-1", record(1)).unwrap(),
        DurableWriteOutcome::Queued {
            watermark: WriteWatermark::new(PartitionId::new(1), 1, ClusterEpoch::new(3)),
            lag: 1,
        }
    );
    asynchronous
        .tombstone(
            "durable",
            "key-2",
            PartitionId::new(1),
            2,
            ClusterEpoch::new(3),
        )
        .unwrap();
    let error = asynchronous
        .write("durable", "key-3", record(3))
        .unwrap_err();
    assert_eq!(error.kind(), DurabilityErrorKind::Backpressure);
    assert_eq!(asynchronous.pending_lag(), 2);
    assert_eq!(asynchronous.drain_async().unwrap(), 2);
    assert_eq!(asynchronous.drain_async().unwrap(), 0);
    assert!(asynchronous
        .store()
        .get("key-2")
        .unwrap()
        .unwrap()
        .is_tombstone());
    assert_eq!(asynchronous.metrics().async_drained_total, 2);

    let watermark = WriteWatermark::new(PartitionId::new(1), 2, ClusterEpoch::new(3));
    let first = asynchronous
        .maybe_snapshot("durable", Duration::from_secs(10), watermark)
        .unwrap()
        .unwrap();
    assert!(asynchronous
        .maybe_snapshot("durable", Duration::from_secs(19), watermark)
        .unwrap()
        .is_none());
    assert!(asynchronous
        .maybe_snapshot("durable", Duration::from_secs(20), watermark)
        .unwrap()
        .is_some());
    assert_eq!(asynchronous.snapshots().first(), Some(&first));
    assert_eq!(
        asynchronous.snapshot_age_ms("durable", Duration::from_secs(25)),
        Some(5_000)
    );
    assert_eq!(
        asynchronous.snapshot_age_ms("missing", Duration::from_secs(25)),
        None
    );
}

#[test]
fn durability_snapshot_and_policy_failures_are_fail_closed() {
    let watermark = WriteWatermark::new(PartitionId::new(2), 8, ClusterEpoch::new(4));
    let manifest = DurabilitySnapshotManifest::new(
        "durable",
        watermark,
        Duration::MAX,
        Duration::from_secs(1),
    );
    assert_eq!(manifest.created_after_ms, u64::MAX);
    manifest.verify().unwrap();

    let mut future = manifest.clone();
    future.format_version = DURABILITY_SNAPSHOT_FORMAT_VERSION + 1;
    assert_eq!(
        future.verify().unwrap_err().kind(),
        DurabilityErrorKind::Snapshot
    );
    let mut corrupt = manifest;
    corrupt.checksum ^= 1;
    assert_eq!(
        corrupt.verify().unwrap_err().kind(),
        DurabilityErrorKind::Snapshot
    );

    let mut ram_only = DurabilityWritePath::new(
        InMemoryReplicatedValueStore::default(),
        PersistencePolicy::ram_only(),
        "eu",
        PersistenceRegionPlacement::home_region_only("eu"),
    );
    assert_eq!(
        ram_only.write("ephemeral", "key", record(1)).unwrap(),
        DurableWriteOutcome::SkippedRamOnly
    );
    assert!(ram_only
        .maybe_snapshot("ephemeral", Duration::from_secs(1), watermark)
        .unwrap()
        .is_none());

    let mut no_interval = DurabilityWritePath::new(
        InMemoryReplicatedValueStore::default(),
        policy(NamespacePersistenceSettings::persistent()),
        "eu",
        PersistenceRegionPlacement::home_region_only("eu"),
    );
    assert!(no_interval
        .maybe_snapshot("durable", Duration::from_secs(1), watermark)
        .unwrap()
        .is_none());

    let mut too_small = DurabilityWritePath::new(
        InMemoryReplicatedValueStore::with_budget(1),
        policy(
            NamespacePersistenceSettings::persistent().with_durability(PersistenceDurability::Sync),
        ),
        "eu",
        PersistenceRegionPlacement::home_region_only("eu"),
    );
    assert_eq!(
        too_small
            .write("durable", "oversized", record(1))
            .unwrap_err()
            .kind(),
        DurabilityErrorKind::Store
    );
}
