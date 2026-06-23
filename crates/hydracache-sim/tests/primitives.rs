use hydracache::{LogicalDuration, LogicalTime, StorageOp, StorageOpKind};
use hydracache_sim::{SimClock, SimRng, SimStorage, SimStorageError, StorageFault};

#[test]
fn primitives_sim_rng_is_reproducible_from_seed() {
    let mut left = SimRng::from_seed(44);
    let mut right = SimRng::from_seed(44);

    let left_values: Vec<_> = (0..16).map(|_| left.next_u64()).collect();
    let right_values: Vec<_> = (0..16).map(|_| right.next_u64()).collect();

    assert_eq!(left_values, right_values);
    assert_eq!(left.next_index(10), right.next_index(10));
    assert_eq!(left.chance(1, 3), right.chance(1, 3));
}

#[test]
fn primitives_sim_clock_advances_only_when_scheduler_advances_it() {
    let mut clock = SimClock::new(LogicalTime::from_millis(7));

    assert_eq!(clock.now(), LogicalTime::from_millis(7));
    clock.advance(LogicalDuration::from_millis(5));
    assert_eq!(clock.now(), LogicalTime::from_millis(12));
    clock.set(LogicalTime::from_millis(20));
    assert_eq!(clock.now(), LogicalTime::from_millis(20));
}

#[test]
fn primitives_fsynced_survives_crash_unsynced_lost() {
    let mut storage = SimStorage::new();

    storage
        .apply_checked(write_request(1, "k", b"unsynced"))
        .expect("write succeeds");
    storage.crash();
    assert_eq!(storage.read_checked("k").expect("read succeeds"), None);

    storage
        .apply_checked(write_request(2, "k", b"synced"))
        .expect("write succeeds");
    storage.fsync();
    storage.crash();
    assert_eq!(
        storage.read_checked("k").expect("read succeeds"),
        Some(b"synced".to_vec())
    );
}

#[test]
fn primitives_injected_corruption_is_detected_by_checksum() {
    let mut storage = SimStorage::new();
    storage
        .apply_checked(write_request(1, "k", b"value"))
        .expect("write succeeds");
    storage.fsync();

    storage.inject_fault("default", StorageFault::Corruption);

    let error = storage
        .read_checked("k")
        .expect_err("corruption must be detected");
    assert!(matches!(error, SimStorageError::ChecksumMismatch { .. }));
}

#[test]
fn primitives_torn_write_is_detected_by_checksum_after_fsync() {
    let mut storage = SimStorage::new();
    storage.inject_fault("default", StorageFault::TornWrite);

    storage
        .apply_checked(write_request(1, "k", b"abcdef"))
        .expect("write succeeds");
    storage.fsync();

    let error = storage
        .read_checked("k")
        .expect_err("torn write must be detected");
    assert!(matches!(error, SimStorageError::ChecksumMismatch { .. }));
}

fn write_request(request_id: u64, key: &str, value: &[u8]) -> StorageOp {
    StorageOp {
        request_id,
        kind: StorageOpKind::Write {
            key: key.to_owned(),
            value: value.to_vec(),
        },
    }
}
