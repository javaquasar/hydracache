#![cfg(feature = "durable-value-store")]

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hydracache::{
    ClusterEpoch, DurabilityErrorKind, DurabilitySnapshotManifest, DurabilityWritePath,
    DurableValueStore, DurableWriteOutcome, NamespacePersistenceRule, NamespacePersistenceSettings,
    PartitionId, PersistenceDurability, PersistencePolicy, PersistenceRegionPlacement,
    ReplicatedValueRecord, ReplicatedValueStore, WriteWatermark,
    DURABILITY_SNAPSHOT_FORMAT_VERSION,
};

#[test]
fn durability_write_path_sync_durability_acks_after_fsync() {
    let path = temp_store_path("sync");
    let store = DurableValueStore::open_with_budget(&path, 1024).unwrap();
    let policy = policy_with(
        "cache.jwt.pem",
        NamespacePersistenceSettings::persistent().with_durability(PersistenceDurability::Sync),
    );
    let placement = PersistenceRegionPlacement::home_region_only("eu");
    let record = record(1, 7, 2, b"sealed-jwt");

    let mut write_path = DurabilityWritePath::new(store, policy, "eu", placement);
    let outcome = write_path
        .write("cache.jwt.pem", "jwt:42", record.clone())
        .unwrap();

    assert_eq!(
        outcome,
        DurableWriteOutcome::SyncAcked {
            watermark: WriteWatermark::new(PartitionId::new(1), 7, ClusterEpoch::new(2)),
            fsync_before_ack: true,
        }
    );
    assert_eq!(write_path.metrics().sync_write_total, 1);
    drop(write_path);

    let reopened = DurableValueStore::open_with_budget(&path, 1024).unwrap();
    assert_eq!(reopened.get("jwt:42").unwrap(), Some(record));
}

#[test]
fn durability_write_path_async_bounded_backpressures_when_lag_exceeded() {
    let path = temp_store_path("async-bound");
    let store = DurableValueStore::open_with_budget(&path, 1024).unwrap();
    let policy = policy_with(
        "cache.session",
        NamespacePersistenceSettings::persistent()
            .with_durability(PersistenceDurability::AsyncBounded { max_lag: 1 }),
    );
    let placement = PersistenceRegionPlacement::home_region_only("eu");

    let mut write_path = DurabilityWritePath::new(store, policy, "eu", placement);
    let first = write_path
        .write("cache.session", "session:1", record(1, 1, 1, b"one"))
        .unwrap();
    assert_eq!(
        first,
        DurableWriteOutcome::Queued {
            watermark: WriteWatermark::new(PartitionId::new(1), 1, ClusterEpoch::new(1)),
            lag: 1,
        }
    );

    let error = write_path
        .write("cache.session", "session:2", record(1, 2, 1, b"two"))
        .unwrap_err();

    assert_eq!(error.kind(), DurabilityErrorKind::Backpressure);
    assert!(error.to_string().contains("lag bound exceeded"));
    assert_eq!(write_path.pending_lag(), 1);
    assert_eq!(write_path.metrics().backpressure_total, 1);

    assert_eq!(write_path.drain_async().unwrap(), 1);
    assert_eq!(write_path.pending_lag(), 0);
    assert!(write_path.store().get("session:1").unwrap().is_some());
}

#[test]
fn durability_write_path_ram_only_namespace_has_no_durable_writes() {
    let path = temp_store_path("ram-only");
    let store = DurableValueStore::open_with_budget(&path, 1024).unwrap();
    let policy = PersistencePolicy::ram_only();
    let placement = PersistenceRegionPlacement::home_region_only("eu");

    let mut write_path = DurabilityWritePath::new(store, policy, "eu", placement);
    let outcome = write_path
        .write("cache.ephemeral", "ephemeral:1", record(1, 1, 1, b"tmp"))
        .unwrap();

    assert_eq!(outcome, DurableWriteOutcome::SkippedRamOnly);
    assert_eq!(write_path.metrics().ram_only_skipped_total, 1);
    assert_eq!(write_path.metrics().sync_write_total, 0);
    assert_eq!(write_path.metrics().async_queued_total, 0);
    assert!(write_path.store().get("ephemeral:1").unwrap().is_none());
}

#[test]
fn durability_write_path_scheduled_snapshot_records_epoch_watermark() {
    let path = temp_store_path("snapshot");
    let store = DurableValueStore::open_with_budget(&path, 1024).unwrap();
    let policy = policy_with(
        "cache.jwt.pem",
        NamespacePersistenceSettings::persistent()
            .with_durability(PersistenceDurability::AsyncBounded { max_lag: 8 })
            .with_snapshot_interval(Duration::from_secs(30)),
    );
    let placement = PersistenceRegionPlacement::home_region_only("eu");

    let mut write_path = DurabilityWritePath::new(store, policy, "eu", placement);
    write_path
        .write("cache.jwt.pem", "jwt:42", record(2, 9, 3, b"sealed"))
        .unwrap();

    let watermark = WriteWatermark::new(PartitionId::new(2), 9, ClusterEpoch::new(3));
    let manifest = write_path
        .maybe_snapshot("cache.jwt.pem", Duration::from_secs(30), watermark)
        .unwrap()
        .expect("snapshot due");

    assert_eq!(manifest.format_version, DURABILITY_SNAPSHOT_FORMAT_VERSION);
    assert_eq!(manifest.namespace, "cache.jwt.pem");
    assert_eq!(manifest.watermark, watermark);
    assert_eq!(manifest.interval_ms, 30_000);
    manifest.verify().unwrap();
    assert_eq!(write_path.metrics().snapshot_total, 1);
    assert_eq!(
        write_path.snapshot_age_ms("cache.jwt.pem", Duration::from_secs(45)),
        Some(15_000)
    );
    assert!(write_path.store().get("jwt:42").unwrap().is_some());

    let mut tampered = DurabilitySnapshotManifest {
        format_version: DURABILITY_SNAPSHOT_FORMAT_VERSION + 1,
        ..manifest
    };
    assert_eq!(
        tampered.verify().unwrap_err().kind(),
        DurabilityErrorKind::Snapshot
    );
    tampered.format_version = DURABILITY_SNAPSHOT_FORMAT_VERSION;
    tampered.interval_ms += 1;
    assert_eq!(
        tampered.verify().unwrap_err().kind(),
        DurabilityErrorKind::Snapshot
    );
}

fn policy_with(pattern: &str, settings: NamespacePersistenceSettings) -> PersistencePolicy {
    PersistencePolicy::try_new([NamespacePersistenceRule::new(pattern, settings).unwrap()]).unwrap()
}

fn record(partition: u32, version: u64, epoch: u64, bytes: &[u8]) -> ReplicatedValueRecord {
    ReplicatedValueRecord::value(
        PartitionId::new(partition),
        version,
        ClusterEpoch::new(epoch),
        bytes.to_vec(),
    )
}

fn temp_store_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "hydracache-durability-write-path-{name}-{}-{nanos}",
        std::process::id()
    ))
}
