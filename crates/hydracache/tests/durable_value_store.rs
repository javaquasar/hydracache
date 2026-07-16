#![cfg(feature = "durable-value-store")]

use std::path::PathBuf;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hydracache::{
    ClusterEpoch, DurableValueStore, PartitionId, ReplicatedValueRecord, ReplicatedValueStore,
    DURABLE_VALUE_FORMAT_VERSION,
};

#[test]
fn durable_value_store_reopen_recovers_records_and_tombstones() {
    let path = temp_store_path("reopen");
    let mut store = DurableValueStore::open_with_budget(&path, 1024).unwrap();
    let record = ReplicatedValueRecord::value(
        PartitionId::new(3),
        7,
        ClusterEpoch::new(2),
        b"sealed-user".to_vec(),
    );
    store.upsert("user:42", record.clone()).unwrap();
    store
        .tombstone("user:43", PartitionId::new(3), 8, ClusterEpoch::new(2))
        .unwrap();
    store.flush().unwrap();
    drop(store);

    // sled releases its process-level lock from a background worker. On a
    // heavily loaded CI runner that release can lag the drop above by a few
    // milliseconds, so tolerate only that transient WouldBlock condition.
    let reopened = (0..100)
        .find_map(|_| match DurableValueStore::open_with_budget(&path, 1024) {
            Ok(store) => Some(store),
            Err(error) if error.to_string().contains("could not acquire lock") => {
                thread::sleep(Duration::from_millis(10));
                None
            }
            Err(error) => panic!("reopening durable value store failed: {error}"),
        })
        .expect("durable value store lock was not released within one second");

    assert_eq!(reopened.get("user:42").unwrap(), Some(record));
    assert!(reopened
        .get("user:43")
        .unwrap()
        .expect("tombstone")
        .is_tombstone());
}

#[test]
fn durable_value_store_unknown_future_format_refuses_to_open() {
    let path = temp_store_path("future-format");
    DurableValueStore::write_format_marker_for_test(&path, DURABLE_VALUE_FORMAT_VERSION + 1)
        .unwrap();

    let error = DurableValueStore::open(&path).unwrap_err();

    assert!(error
        .to_string()
        .contains("unsupported durable value-store format"));
}

#[test]
fn durable_value_store_over_budget_upsert_is_rejected_and_counted() {
    let path = temp_store_path("budget");
    let mut store = DurableValueStore::open_with_budget(&path, 4).unwrap();

    let error = store
        .upsert(
            "big",
            ReplicatedValueRecord::value(
                PartitionId::new(1),
                1,
                ClusterEpoch::new(1),
                b"too-large".to_vec(),
            ),
        )
        .unwrap_err();

    assert!(error.to_string().contains("budget exceeded"));
    assert_eq!(store.rejected_total(), 1);
    assert!(store.get("big").unwrap().is_none());
}

#[test]
fn durable_value_store_corrupt_record_is_detected_not_served() {
    let path = temp_store_path("corrupt");
    let mut store = DurableValueStore::open_with_budget(&path, 1024).unwrap();
    store
        .upsert(
            "user:42",
            ReplicatedValueRecord::value(
                PartitionId::new(1),
                1,
                ClusterEpoch::new(1),
                b"valid".to_vec(),
            ),
        )
        .unwrap();
    store
        .put_raw_record_for_test("user:42", b"not-a-valid-envelope")
        .unwrap();

    let error = store.get("user:42").unwrap_err();

    assert!(error.to_string().contains("durable value"));
}

fn temp_store_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "hydracache-durable-value-store-{name}-{}-{nanos}",
        std::process::id()
    ))
}
