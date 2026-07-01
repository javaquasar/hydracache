#![cfg(feature = "durable-value-store")]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hydracache::{
    ClusterEpoch, DurableValueStore, PartitionId, PersistenceConfig, PersistenceMaintenanceConfig,
    PersistenceNamespaceConfig, ReplicatedValueRecord, ReplicatedValueStore, TombstoneBudget,
    TombstoneTracker,
};

#[test]
fn tombstone_gc_reclaims_only_after_repair_gate() {
    let path = temp_store_path("tombstone_gc_reclaims_only_after_repair_gate");
    let mut store = DurableValueStore::open(&path).expect("open durable store");
    let mut tracker = TombstoneTracker::new(TombstoneBudget::new(8, 1024));
    store
        .tombstone("dead", PartitionId::new(1), 10, ClusterEpoch::new(1))
        .expect("write tombstone");
    tracker.admit("dead", 10, 1, None);
    store.upsert("live", value(11, b"x")).expect("write live");

    let pending = store
        .collect_tombstone_garbage(&mut tracker, ClusterEpoch::new(99), 16)
        .expect("pending gc");
    assert_eq!(pending.removed, 0);
    assert_eq!(pending.skipped_repair_pending, 1);
    assert!(store.get("dead").unwrap().unwrap().is_tombstone());

    tracker.confirm_repair("dead", ClusterEpoch::new(7));
    let too_early = store
        .collect_tombstone_garbage(&mut tracker, ClusterEpoch::new(6), 16)
        .expect("early gc");
    assert_eq!(too_early.removed, 0);
    assert_eq!(too_early.skipped_repair_pending, 1);

    let reclaimed = store
        .collect_tombstone_garbage(&mut tracker, ClusterEpoch::new(7), 16)
        .expect("reclaim gc");
    assert_eq!(reclaimed.removed, 1);
    assert_eq!(reclaimed.reclaimed_bytes, 1);
    assert_eq!(reclaimed.durable_gc_reclaimed_total, 1);
    assert_eq!(reclaimed.durable_gc_skipped_repair_pending_total, 0);
    assert!(store.get("dead").unwrap().is_none());
    assert!(!tracker.contains_key("dead"));
    assert!(store.get("live").unwrap().is_some());
}

#[test]
fn gc_never_resurrects_deleted_data() {
    let primary_path = temp_store_path("gc_never_resurrects_deleted_data_primary");
    let replica_path = temp_store_path("gc_never_resurrects_deleted_data_replica");
    let mut primary = DurableValueStore::open(&primary_path).expect("open primary");
    let mut replica = DurableValueStore::open(&replica_path).expect("open replica");
    let mut tracker = TombstoneTracker::new(TombstoneBudget::new(8, 1024));

    primary
        .tombstone("user:1", PartitionId::new(1), 20, ClusterEpoch::new(2))
        .expect("write tombstone");
    tracker.admit("user:1", 20, 1, None);
    replica
        .upsert("user:1", value(10, b"stale"))
        .expect("write stale replica");

    let pending = primary
        .collect_tombstone_garbage(&mut tracker, ClusterEpoch::new(99), 16)
        .expect("pending gc");
    assert_eq!(pending.removed, 0);
    let winner = primary
        .get("user:1")
        .unwrap()
        .unwrap()
        .merge(replica.get("user:1").unwrap().unwrap());
    assert!(winner.is_tombstone());

    let repaired = primary.get("user:1").unwrap().unwrap();
    replica.upsert("user:1", repaired).expect("repair replica");
    assert!(replica.get("user:1").unwrap().unwrap().is_tombstone());
    tracker.confirm_repair("user:1", ClusterEpoch::new(3));
    let reclaimed = primary
        .collect_tombstone_garbage(&mut tracker, ClusterEpoch::new(3), 16)
        .expect("confirmed gc");

    assert_eq!(reclaimed.removed, 1);
    assert!(primary.get("user:1").unwrap().is_none());
    assert!(replica.get("user:1").unwrap().unwrap().is_tombstone());
}

#[test]
fn compaction_reclaims_bytes_and_reports() {
    let path = temp_store_path("compaction_reclaims_bytes_and_reports");
    let mut store = DurableValueStore::open(&path).expect("open durable store");

    assert_eq!(store.compact().expect("compact empty"), 0);
    store.upsert("temp", value(1, b"x")).expect("write temp");
    store.remove("temp").expect("remove temp");
    let reclaimed = store.compact().expect("compact after churn");

    assert_eq!(reclaimed, 0);
    assert_eq!(store.total_bytes().expect("total bytes"), 0);
}

#[test]
fn budget_is_exact_under_concurrent_gc() {
    let path = temp_store_path("budget_is_exact_under_concurrent_gc");
    let mut store = DurableValueStore::open_with_budget(&path, 2).expect("open durable store");
    let mut tracker = TombstoneTracker::new(TombstoneBudget::new(8, 1024));
    store
        .tombstone("dead", PartitionId::new(1), 10, ClusterEpoch::new(1))
        .expect("write tombstone");
    tracker.admit("dead", 10, 1, None);
    store.upsert("live", value(11, b"x")).expect("write live");
    assert!(store.upsert("blocked", value(12, b"y")).is_err());
    assert_eq!(store.rejected_total(), 1);
    tracker.confirm_repair("dead", ClusterEpoch::new(2));

    let store = Arc::new(Mutex::new(store));
    let tracker = Arc::new(Mutex::new(tracker));
    let gc_store = Arc::clone(&store);
    let gc_tracker = Arc::clone(&tracker);
    let gc = std::thread::spawn(move || {
        gc_store
            .lock()
            .unwrap()
            .collect_tombstone_garbage(&mut gc_tracker.lock().unwrap(), ClusterEpoch::new(2), 16)
            .unwrap()
    });
    let report = gc.join().expect("gc thread");
    assert_eq!(report.removed, 1);

    let mut store = store.lock().unwrap();
    store
        .upsert("accepted", value(12, b"y"))
        .expect("post-gc upsert fits");
    assert_eq!(store.total_bytes().expect("total bytes"), 2);
    assert_eq!(store.rejected_total(), 1);
}

#[test]
fn maintenance_config_maps_to_policy_settings() {
    let mut namespaces = BTreeMap::new();
    namespaces.insert(
        "cache.*".to_owned(),
        PersistenceNamespaceConfig {
            persist: true,
            maintenance: PersistenceMaintenanceConfig {
                tombstone_gc_interval_secs: Some(5),
                compaction_interval_secs: Some(30),
                gc_records_per_cycle: 0,
            },
            ..PersistenceNamespaceConfig::default()
        },
    );
    let policy = PersistenceConfig {
        namespaces,
        ..PersistenceConfig::default()
    }
    .to_policy()
    .expect("policy");
    let resolved = policy.resolve("cache.users");

    assert_eq!(
        resolved.settings.maintenance.tombstone_gc_interval,
        Some(Duration::from_secs(5))
    );
    assert_eq!(
        resolved.settings.maintenance.compaction_interval,
        Some(Duration::from_secs(30))
    );
    assert_eq!(resolved.settings.maintenance.gc_records_per_cycle, 1);
}

fn value(version: u64, bytes: impl Into<Vec<u8>>) -> ReplicatedValueRecord {
    ReplicatedValueRecord::value(
        PartitionId::new(version as u32),
        version,
        ClusterEpoch::new(version + 1),
        bytes,
    )
}

fn temp_store_path(test: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "hydracache-0-55-durable-maintenance-{test}-{unique}"
    ))
}
