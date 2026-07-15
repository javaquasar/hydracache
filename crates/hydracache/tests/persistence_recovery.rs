#![cfg(feature = "durable-value-store")]

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hydracache::{
    open_durable_value_store_for_recovery, recover_namespaces, ClusterEpoch, DurableValueStore,
    EffectiveReplicationMap, NamespacePersistenceRule, NamespacePersistenceSettings, PartitionId,
    PersistencePolicy, PersistenceRegionPlacement, RecoveryErrorKind, RecoveryMode,
    RecoveryNamespace, RecoveryPolicy, Replicas, ReplicatedValueRecord, ReplicatedValueStore,
    DURABLE_VALUE_FORMAT_VERSION,
};

#[test]
fn persistence_recovery_persistent_namespace_survives_full_restart() {
    let path = temp_store_path("restart");
    let mut store = DurableValueStore::open_with_budget(&path, 1024).unwrap();
    let durable_record = record(1, 7, 2, b"sealed");
    store
        .upsert("cache.jwt.pem/jwt:42", durable_record.clone())
        .unwrap();
    store
        .upsert("cache.ephemeral/tmp:1", record(1, 1, 2, b"tmp"))
        .unwrap();
    store.flush().unwrap();
    drop(store);

    let reopened = reopen_after_store_drop(&path);
    let policy = PersistencePolicy::try_new([
        NamespacePersistenceRule::persistent("cache.jwt.pem").unwrap(),
        NamespacePersistenceRule::new("cache.ephemeral", NamespacePersistenceSettings::ram_only())
            .unwrap(),
    ])
    .unwrap();
    let report = recover_namespaces(
        &reopened,
        &policy,
        &"eu".into(),
        ClusterEpoch::new(2),
        &RecoveryPolicy::full_recovery_only(),
        [
            RecoveryNamespace::new("cache.jwt.pem", placement(), replication_map())
                .with_key_prefix("cache.jwt.pem/"),
            RecoveryNamespace::new("cache.ephemeral", placement(), replication_map())
                .with_key_prefix("cache.ephemeral/"),
        ],
    )
    .unwrap();

    assert_eq!(
        report.record("cache.jwt.pem", "cache.jwt.pem/jwt:42"),
        Some(&durable_record)
    );
    assert!(report.namespace_persistent("cache.jwt.pem"));
    assert!(!report.namespace_persistent("cache.ephemeral"));
    assert!(report
        .record("cache.ephemeral", "cache.ephemeral/tmp:1")
        .is_none());
    assert_eq!(report.non_persistent_skipped_total, 1);
}

#[test]
fn persistence_recovery_stale_epoch_record_is_fenced_not_served() {
    let path = temp_store_path("stale");
    let mut store = DurableValueStore::open_with_budget(&path, 1024).unwrap();
    store
        .upsert("cache.jwt.pem/stale", record(1, 10, 1, b"old"))
        .unwrap();
    let fresh = record(1, 11, 3, b"fresh");
    store.upsert("cache.jwt.pem/fresh", fresh.clone()).unwrap();
    store.flush().unwrap();

    let policy = PersistencePolicy::try_new([NamespacePersistenceRule::persistent(
        "cache.jwt.pem",
    )
    .unwrap()])
    .unwrap();
    let report = recover_namespaces(
        &store,
        &policy,
        &"eu".into(),
        ClusterEpoch::new(2),
        &RecoveryPolicy::full_recovery_only().with_auto_remove_stale_data(true),
        [
            RecoveryNamespace::new("cache.jwt.pem", placement(), replication_map())
                .with_key_prefix("cache.jwt.pem/"),
        ],
    )
    .unwrap();

    assert!(report
        .record("cache.jwt.pem", "cache.jwt.pem/stale")
        .is_none());
    assert_eq!(
        report.record("cache.jwt.pem", "cache.jwt.pem/fresh"),
        Some(&fresh)
    );
    assert_eq!(report.stale_fenced_total, 1);
    assert!(report.auto_remove_stale_data);
    assert_eq!(
        report.namespaces.get("cache.jwt.pem").unwrap().stale_keys,
        vec!["cache.jwt.pem/stale".to_owned()]
    );
}

#[test]
fn persistence_recovery_full_recovery_only_fails_loud_on_timeout() {
    let path = temp_store_path("timeout");
    let mut store = DurableValueStore::open_with_budget(&path, 1024).unwrap();
    store
        .upsert("cache.jwt.pem/jwt:42", record(1, 1, 1, b"sealed"))
        .unwrap();
    let policy = PersistencePolicy::try_new([NamespacePersistenceRule::persistent(
        "cache.jwt.pem",
    )
    .unwrap()])
    .unwrap();

    let strict_error = recover_namespaces(
        &store,
        &policy,
        &"eu".into(),
        ClusterEpoch::new(1),
        &RecoveryPolicy::full_recovery_only().with_data_load_timeout(Duration::ZERO),
        [RecoveryNamespace::new(
            "cache.jwt.pem",
            placement(),
            replication_map(),
        )],
    )
    .unwrap_err();
    assert_eq!(strict_error.kind(), RecoveryErrorKind::Timeout);
    assert!(strict_error.to_string().contains("full recovery timed out"));

    let partial_report = recover_namespaces(
        &store,
        &policy,
        &"eu".into(),
        ClusterEpoch::new(1),
        &RecoveryPolicy {
            mode: RecoveryMode::PartialAllowed,
            ..RecoveryPolicy::partial_allowed().with_data_load_timeout(Duration::ZERO)
        },
        [RecoveryNamespace::new(
            "cache.jwt.pem",
            placement(),
            replication_map(),
        )],
    )
    .unwrap();
    assert_eq!(partial_report.timeout_total, 1);
    assert_eq!(partial_report.partial_recovery_total, 1);
}

#[test]
fn persistence_recovery_corrupt_or_future_format_refuses_recovery() {
    let corrupt_path = temp_store_path("corrupt");
    let corrupt_store = DurableValueStore::open_with_budget(&corrupt_path, 1024).unwrap();
    corrupt_store
        .put_raw_record_for_test("cache.jwt.pem/jwt:42", b"not-a-valid-record")
        .unwrap();
    let policy = PersistencePolicy::try_new([NamespacePersistenceRule::persistent(
        "cache.jwt.pem",
    )
    .unwrap()])
    .unwrap();

    let corrupt_error = recover_namespaces(
        &corrupt_store,
        &policy,
        &"eu".into(),
        ClusterEpoch::new(1),
        &RecoveryPolicy::full_recovery_only(),
        [RecoveryNamespace::new(
            "cache.jwt.pem",
            placement(),
            replication_map(),
        )],
    )
    .unwrap_err();
    assert_eq!(corrupt_error.kind(), RecoveryErrorKind::Store);
    assert!(corrupt_error.to_string().contains("durable value"));

    let future_path = temp_store_path("future");
    DurableValueStore::write_format_marker_for_test(&future_path, DURABLE_VALUE_FORMAT_VERSION + 1)
        .unwrap();
    let future_error = open_durable_value_store_for_recovery(&future_path, 1024).unwrap_err();
    assert_eq!(future_error.kind(), RecoveryErrorKind::Store);
    assert!(future_error
        .to_string()
        .contains("unsupported durable value-store format"));
}

fn record(partition: u32, version: u64, epoch: u64, bytes: &[u8]) -> ReplicatedValueRecord {
    ReplicatedValueRecord::value(
        PartitionId::new(partition),
        version,
        ClusterEpoch::new(epoch),
        bytes.to_vec(),
    )
}

fn placement() -> PersistenceRegionPlacement {
    PersistenceRegionPlacement::home_region_only("eu")
}

fn replication_map() -> EffectiveReplicationMap {
    EffectiveReplicationMap::new(Replicas::new("node-a", Vec::new()))
}

fn temp_store_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "hydracache-persistence-recovery-{name}-{}-{nanos}",
        std::process::id()
    ))
}

fn reopen_after_store_drop(path: &std::path::Path) -> DurableValueStore {
    const MAX_ATTEMPTS: usize = 20;

    for attempt in 0..MAX_ATTEMPTS {
        match open_durable_value_store_for_recovery(path, 1024) {
            Ok(store) => return store,
            Err(error) if error.to_string().contains("WouldBlock") => {
                if attempt + 1 < MAX_ATTEMPTS {
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
            Err(error) => panic!("durable store reopen failed unexpectedly: {error}"),
        }
    }

    panic!(
        "durable store lock was not released after {MAX_ATTEMPTS} reopen attempts: {}",
        path.display()
    );
}
