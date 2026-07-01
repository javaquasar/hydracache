#![cfg(feature = "durable-value-store")]

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hydracache::{
    ClusterEpoch, DurableScrubConfig, DurableScrubber, DurableValueStore, PartitionId,
    ReplicatedValueRecord, ReplicatedValueStore,
};

#[test]
fn scrubber_detects_injected_corruption_and_fails_loud() {
    let path = temp_store_path("scrubber_detects_injected_corruption_and_fails_loud");
    let store = DurableValueStore::open(&path).expect("open durable store");
    store
        .put_raw_record_for_test("bad", b"not-a-valid-record")
        .expect("inject corruption");

    let mut scrubber = scrubber(16);
    let report = scrubber.scrub_cycle(&store).expect("scrub cycle");

    assert_eq!(report.records_checked, 1);
    assert_eq!(report.corruption_count, 1);
    assert_eq!(report.corruptions[0].key, "bad");
    assert!(!report.corruptions[0].error.is_empty());
    assert_eq!(scrubber.metrics().durable_scrub_records_total, 1);
    assert_eq!(scrubber.metrics().durable_scrub_corruption_total, 1);
}

#[test]
fn scrub_does_not_abort_on_one_corrupt_record() {
    let path = temp_store_path("scrub_does_not_abort_on_one_corrupt_record");
    let mut store = DurableValueStore::open(&path).expect("open durable store");
    store.upsert("good", value(1, b"ok")).expect("write good");
    store
        .put_raw_record_for_test("bad", b"not-a-valid-record")
        .expect("inject corruption");

    let mut scrubber = scrubber(16);
    let report = scrubber.scrub_cycle(&store).expect("scrub cycle");

    assert_eq!(report.records_checked, 2);
    assert_eq!(report.corruption_count, 1);
    assert_eq!(report.checked_keys, vec!["bad", "good"]);
}

#[test]
fn scrubber_is_bounded_per_cycle_via_cursor_not_a_full_store_stall() {
    let path = temp_store_path("scrubber_is_bounded_per_cycle_via_cursor_not_a_full_store_stall");
    let mut store = DurableValueStore::open(&path).expect("open durable store");
    for index in 0..5 {
        store
            .upsert(format!("key-{index}"), value(index, [index as u8]))
            .expect("write record");
    }

    let mut scrubber = scrubber(2);
    let first = scrubber.scrub_cycle(&store).expect("first scrub");
    let second = scrubber.scrub_cycle(&store).expect("second scrub");
    let third = scrubber.scrub_cycle(&store).expect("third scrub");

    assert_eq!(first.records_checked, 2);
    assert!(!first.finished_pass);
    assert_eq!(first.cursor.as_deref(), Some("key-1"));
    assert_eq!(second.records_checked, 2);
    assert!(!second.finished_pass);
    assert_eq!(second.cursor.as_deref(), Some("key-3"));
    assert_eq!(third.records_checked, 1);
    assert!(third.finished_pass);
    assert_eq!(third.cursor, None);
    assert_eq!(scrubber.metrics().durable_scrub_records_total, 5);
}

#[test]
fn scrub_over_empty_store_and_all_tombstones_reports_zero_corruption() {
    let path = temp_store_path("scrub_over_empty_store_and_all_tombstones_reports_zero_corruption");
    let mut store = DurableValueStore::open(&path).expect("open durable store");
    let mut scrubber = scrubber(4);

    let empty = scrubber.scrub_cycle(&store).expect("empty scrub");
    assert_eq!(empty.records_checked, 0);
    assert!(empty.is_clean());
    assert!(empty.finished_pass);

    for index in 0..3 {
        store
            .tombstone(
                format!("dead-{index}"),
                PartitionId::new(index),
                100 + u64::from(index),
                ClusterEpoch::new(7),
            )
            .expect("write tombstone");
    }
    let tombstones = scrubber.scrub_cycle(&store).expect("tombstone scrub");
    assert_eq!(tombstones.records_checked, 3);
    assert!(tombstones.is_clean());
}

#[test]
fn scrub_cursor_is_deterministic_when_store_grows_mid_scan() {
    let path = temp_store_path("scrub_cursor_is_deterministic_when_store_grows_mid_scan");
    let mut store = DurableValueStore::open(&path).expect("open durable store");
    store.upsert("a", value(1, b"a")).expect("write a");
    store.upsert("b", value(2, b"b")).expect("write b");
    let mut scrubber = scrubber(1);

    let first = scrubber.scrub_cycle(&store).expect("first scrub");
    store.upsert("c", value(3, b"c")).expect("write c");
    let second = scrubber.scrub_cycle(&store).expect("second scrub");
    let third = scrubber.scrub_cycle(&store).expect("third scrub");

    assert_eq!(first.checked_keys, vec!["a"]);
    assert_eq!(second.checked_keys, vec!["b"]);
    assert_eq!(third.checked_keys, vec!["c"]);
    assert!(third.finished_pass);
    assert_eq!(scrubber.metrics().durable_scrub_records_total, 3);
}

#[test]
fn torn_or_bad_checksum_record_is_counted_not_panicked() {
    let path = temp_store_path("torn_or_bad_checksum_record_is_counted_not_panicked");
    let mut store = DurableValueStore::open(&path).expect("open durable store");
    store
        .upsert("bad-checksum", value(9, b"checksum"))
        .expect("write bad-checksum seed");
    let mut raw = store
        .raw_record_for_test("bad-checksum")
        .expect("read raw")
        .expect("raw exists");
    let last = raw.last_mut().expect("encoded record has checksum byte");
    *last ^= 0xff;
    store
        .put_raw_record_for_test("bad-checksum", raw)
        .expect("inject bad checksum");
    store
        .put_raw_record_for_test("torn", [1_u8, 2])
        .expect("inject torn record");

    let mut scrubber = scrubber(16);
    let report = scrubber.scrub_cycle(&store).expect("scrub cycle");

    assert_eq!(report.records_checked, 2);
    assert_eq!(report.corruption_count, 2);
    assert_eq!(
        report
            .corruptions
            .iter()
            .map(|corruption| corruption.key.as_str())
            .collect::<Vec<_>>(),
        vec!["bad-checksum", "torn"]
    );
}

fn scrubber(records_per_cycle: usize) -> DurableScrubber {
    DurableScrubber::new(DurableScrubConfig::new(
        records_per_cycle,
        Duration::from_millis(10),
    ))
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
    std::env::temp_dir().join(format!("hydracache-0-55-durable-scrub-{test}-{unique}"))
}
