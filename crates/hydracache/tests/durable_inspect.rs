#![cfg(feature = "durable-value-store")]

use std::time::{SystemTime, UNIX_EPOCH};

use hydracache::{
    inspect_replicated_store, ClusterEpoch, DurableInspectChecksumStatus, DurableValueStore,
    PartitionId, ReplicatedValueRecord, ReplicatedValueStore,
};

#[test]
fn inspect_dumps_records_with_status() {
    let path = temp_store_path("inspect_dumps_records_with_status");
    let mut store = DurableValueStore::open(&path).expect("open durable store");
    store
        .upsert(
            "live",
            ReplicatedValueRecord::value(
                PartitionId::new(3),
                7,
                ClusterEpoch::new(11),
                b"abc".to_vec(),
            ),
        )
        .expect("write live");
    store
        .tombstone("dead", PartitionId::new(5), 9, ClusterEpoch::new(13))
        .expect("write tombstone");

    let dump = inspect_replicated_store(&store).expect("inspect durable records");

    assert_eq!(dump.len(), 2);
    assert_eq!(dump[0].key, "dead");
    assert_eq!(dump[0].partition, 5);
    assert_eq!(dump[0].version, 9);
    assert_eq!(dump[0].epoch, 13);
    assert!(dump[0].tombstone);
    assert_eq!(dump[0].approx_bytes, 1);
    assert_eq!(
        dump[0].checksum_status,
        DurableInspectChecksumStatus::Verified
    );
    assert_eq!(dump[1].key, "live");
    assert_eq!(dump[1].partition, 3);
    assert_eq!(dump[1].version, 7);
    assert_eq!(dump[1].epoch, 11);
    assert!(!dump[1].tombstone);
    assert_eq!(dump[1].approx_bytes, 3);
    assert_eq!(
        dump[1].checksum_status,
        DurableInspectChecksumStatus::Verified
    );
}

#[test]
fn corrupt_record_is_never_served() {
    let path = temp_store_path("corrupt_record_is_never_served");
    let store = DurableValueStore::open(&path).expect("open durable store");
    store
        .put_raw_record_for_test("corrupt", b"not-a-valid-record")
        .expect("inject corruption");

    assert!(store.get("corrupt").is_err());
    assert!(inspect_replicated_store(&store).is_err());
}

fn temp_store_path(test: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    std::env::temp_dir().join(format!("hydracache-0-55-durable-inspect-{test}-{unique}"))
}
