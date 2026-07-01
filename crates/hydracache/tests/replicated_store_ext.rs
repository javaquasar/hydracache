#![cfg(feature = "durable-value-store")]

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use hydracache::{
    ClusterEpoch, DurableValueStore, EffectiveReplicationMap, InMemoryReplicatedValueStore,
    PartitionId, Replicas, ReplicatedValueRecord, ReplicatedValueStore,
    DURABLE_VALUE_FORMAT_VERSION,
};

#[test]
fn scan_all_returns_every_record_both_impls() {
    let mut memory = InMemoryReplicatedValueStore::with_budget(1024);
    seed_records(&mut memory);

    let memory_all = memory.scan_all().unwrap();
    assert_eq!(memory_all.len(), 2);
    assert!(memory_all.iter().any(|(key, _)| key == "live"));
    assert!(memory_all
        .iter()
        .any(|(key, record)| key == "deleted" && record.is_tombstone()));

    let path = temp_store_path("scan-all");
    let mut durable = DurableValueStore::open_with_budget(&path, 1024).unwrap();
    seed_records(&mut durable);

    assert_eq!(durable.scan_all().unwrap(), memory_all);
}

#[test]
fn remove_deletes_only_the_targeted_key() {
    let mut memory = InMemoryReplicatedValueStore::with_budget(1024);
    seed_records(&mut memory);

    memory.remove("deleted").unwrap();
    assert!(memory.get("deleted").unwrap().is_none());
    assert!(memory.get("live").unwrap().is_some());

    let path = temp_store_path("remove");
    let mut durable = DurableValueStore::open_with_budget(&path, 1024).unwrap();
    seed_records(&mut durable);

    durable.remove("deleted").unwrap();
    assert!(durable.get("deleted").unwrap().is_none());
    assert!(durable.get("live").unwrap().is_some());
    assert_eq!(durable.scan_all().unwrap().len(), 1);
}

#[test]
fn compact_returns_reclaimed_bytes_or_zero() {
    let mut memory = InMemoryReplicatedValueStore::with_budget(1024);
    seed_records(&mut memory);
    assert_eq!(memory.compact().unwrap(), 0);

    let path = temp_store_path("compact");
    let mut durable = DurableValueStore::open_with_budget(&path, 1024).unwrap();
    seed_records(&mut durable);
    durable.remove("deleted").unwrap();

    let before = durable.total_bytes().unwrap();
    let reclaimed = durable.compact().unwrap();
    let after = durable.total_bytes().unwrap();

    assert_eq!(reclaimed, 0);
    assert_eq!(after, before);
}

#[test]
fn total_bytes_and_rejected_total_match_inherent() {
    let mut memory = InMemoryReplicatedValueStore::with_budget(4);
    let error = memory
        .upsert("too-big", record("too-large", 1))
        .unwrap_err();
    assert!(error.to_string().contains("budget exceeded"));
    assert_eq!(
        <InMemoryReplicatedValueStore as ReplicatedValueStore>::total_bytes(&memory).unwrap(),
        InMemoryReplicatedValueStore::total_bytes(&memory)
    );
    assert_eq!(
        <InMemoryReplicatedValueStore as ReplicatedValueStore>::rejected_total(&memory),
        InMemoryReplicatedValueStore::rejected_total(&memory)
    );

    let path = temp_store_path("budget-hoist");
    let mut durable = DurableValueStore::open_with_budget(&path, 4).unwrap();
    let error = durable
        .upsert("too-big", record("too-large", 1))
        .unwrap_err();
    assert!(error.to_string().contains("budget exceeded"));
    assert_eq!(
        <DurableValueStore as ReplicatedValueStore>::total_bytes(&durable).unwrap(),
        DurableValueStore::total_bytes(&durable).unwrap()
    );
    assert_eq!(
        <DurableValueStore as ReplicatedValueStore>::rejected_total(&durable),
        DurableValueStore::rejected_total(&durable)
    );
}

#[test]
fn existing_upsert_get_tombstone_scan_owned_unchanged() {
    let mut memory = InMemoryReplicatedValueStore::with_budget(1024);
    assert_existing_semantics(&mut memory);

    let path = temp_store_path("golden");
    let mut durable = DurableValueStore::open_with_budget(&path, 1024).unwrap();
    assert_existing_semantics(&mut durable);
}

#[test]
fn unknown_future_format_still_refuses_to_open() {
    let path = temp_store_path("future-format");
    DurableValueStore::write_format_marker_for_test(&path, DURABLE_VALUE_FORMAT_VERSION + 1)
        .unwrap();

    let error = DurableValueStore::open(&path).unwrap_err();

    assert!(error
        .to_string()
        .contains("unsupported durable value-store format"));
}

fn assert_existing_semantics<S>(store: &mut S)
where
    S: ReplicatedValueStore,
{
    store.upsert("user:1", record("old", 1)).unwrap();
    store.upsert("user:1", record("new", 2)).unwrap();

    let stored = store.get("user:1").unwrap().expect("stored");
    assert_eq!(stored.version, 2);

    store
        .tombstone("user:1", PartitionId::new(7), 3, ClusterEpoch::new(2))
        .unwrap();
    store.upsert("user:1", record("stale", 2)).unwrap();
    assert!(store
        .get("user:1")
        .unwrap()
        .expect("tombstone")
        .is_tombstone());

    assert!(store
        .scan_owned(&EffectiveReplicationMap::new(Replicas::new(
            "primary",
            Vec::new()
        )))
        .unwrap()
        .iter()
        .any(|(key, record)| key == "user:1" && record.is_tombstone()));
    assert!(store
        .scan_owned(&EffectiveReplicationMap {
            natural: Replicas::new("primary", Vec::new()),
            reading: Vec::new(),
            pending: None,
        })
        .unwrap()
        .is_empty());
}

fn seed_records<S>(store: &mut S)
where
    S: ReplicatedValueStore,
{
    store.upsert("live", record("value", 1)).unwrap();
    store
        .tombstone("deleted", PartitionId::new(7), 2, ClusterEpoch::new(1))
        .unwrap();
}

fn record(value: &str, version: u64) -> ReplicatedValueRecord {
    ReplicatedValueRecord::value(
        PartitionId::new(7),
        version,
        ClusterEpoch::new(1),
        value.as_bytes().to_vec(),
    )
}

fn temp_store_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "hydracache-replicated-store-ext-{name}-{}-{nanos}",
        std::process::id()
    ))
}
