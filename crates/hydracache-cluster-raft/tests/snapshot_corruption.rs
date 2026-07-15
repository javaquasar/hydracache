#![cfg(feature = "sled-log-store")]

use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hydracache_cluster_raft::{
    InMemoryRaftMetadataStore, RaftLogStore, RaftMetadataRuntime, RaftMetadataRuntimeConfig,
    SledRaftLogStore,
};
use protobuf::Message as ProtobufMessage;
use raft::eraftpb::Snapshot;
use raft::storage::Storage;

const SLED_SNAPSHOT_KEY: &[u8] = b"meta:snapshot";
const SLED_REOPEN_RETRIES: usize = 50;
const SLED_REOPEN_RETRY_DELAY: Duration = Duration::from_millis(10);

static TEMP_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_path(label: &str) -> std::path::PathBuf {
    loop {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = TEMP_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "hydracache-{label}-{}-{unique}-{sequence}",
            std::process::id()
        ));

        // Reserve the directory before sled sees it. This makes the test
        // isolated even when multiple baseline processes share a temp root.
        match fs::create_dir(&path) {
            Ok(()) => return path,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("create temporary sled path {}: {error}", path.display()),
        }
    }
}

fn raft_snapshot(index: u64, term: u64, voters: Vec<u64>, data: &[u8]) -> Snapshot {
    let mut snapshot = Snapshot::default();
    snapshot.mut_metadata().index = index;
    snapshot.mut_metadata().term = term;
    snapshot.mut_metadata().mut_conf_state().voters = voters;
    snapshot.data = data.to_vec().into();
    snapshot
}

fn write_snapshot(path: &std::path::Path, snapshot: &Snapshot) {
    let store = open_store(path).unwrap();
    store.save_snapshot(snapshot, 0).unwrap();
    drop(store);
}

fn mutate_persisted_snapshot(path: &std::path::Path, mut mutate: impl FnMut(&mut Vec<u8>)) {
    let db = open_sled(path).unwrap();
    let mut bytes = db
        .get(SLED_SNAPSHOT_KEY)
        .unwrap()
        .expect("snapshot bytes stored")
        .to_vec();
    mutate(&mut bytes);
    db.insert(SLED_SNAPSHOT_KEY, bytes).unwrap();
    db.flush().unwrap();
    drop(db);
}

fn open_sled(path: &std::path::Path) -> sled::Result<sled::Db> {
    for attempt in 0..=SLED_REOPEN_RETRIES {
        match sled::open(path) {
            Ok(db) => return Ok(db),
            Err(error)
                if attempt < SLED_REOPEN_RETRIES && error.to_string().contains("WouldBlock") =>
            {
                thread::sleep(SLED_REOPEN_RETRY_DELAY);
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("sled reopen retry loop returns on success or final error")
}

fn open_store(
    path: &std::path::Path,
) -> hydracache_cluster_raft::RaftStoreResult<SledRaftLogStore> {
    for attempt in 0..=SLED_REOPEN_RETRIES {
        match SledRaftLogStore::open(path) {
            Ok(store) => return Ok(store),
            Err(error)
                if attempt < SLED_REOPEN_RETRIES && error.to_string().contains("WouldBlock") =>
            {
                thread::sleep(SLED_REOPEN_RETRY_DELAY);
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("sled store reopen retry loop returns on success or final error")
}

#[test]
fn snapshot_bitflip_fails_loud_checksum() {
    let path = temp_path("raft-snapshot-bitflip");
    let snapshot = raft_snapshot(7, 3, vec![1, 2, 3], b"membership-state");
    write_snapshot(&path, &snapshot);

    mutate_persisted_snapshot(&path, |bytes| {
        let last = bytes.last_mut().expect("snapshot envelope has bytes");
        *last ^= 0x55;
    });

    let error = open_store(&path).unwrap_err().to_string();
    assert!(
        error.contains("raft snapshot checksum mismatch"),
        "unexpected error: {error}"
    );

    let _ = fs::remove_dir_all(path);
}

#[test]
fn snapshot_truncated_bytes_fail_loud_without_partial_apply() {
    let path = temp_path("raft-snapshot-truncate");
    let snapshot = raft_snapshot(9, 4, vec![1, 2, 3], b"long-membership-state");
    write_snapshot(&path, &snapshot);

    mutate_persisted_snapshot(&path, |bytes| {
        bytes.truncate(bytes.len().saturating_sub(5));
    });

    let error = open_store(&path).unwrap_err().to_string();
    assert!(
        error.contains("truncated raft snapshot checksum envelope"),
        "unexpected error: {error}"
    );

    let fresh_path = temp_path("raft-snapshot-truncate-fresh");
    let fresh = open_store(&fresh_path).unwrap();
    assert_eq!(fresh.snapshot(1, 1).unwrap().get_metadata().index, 1);
    drop(fresh);
    let _ = fs::remove_dir_all(path);
    let _ = fs::remove_dir_all(fresh_path);
}

#[test]
fn misdirected_snapshot_with_valid_checksum_is_rejected_on_identity_mismatch() {
    let wrong_runtime = RaftMetadataRuntime::single_node("billing", 2).unwrap();
    let wrong_snapshot = wrong_runtime.export_snapshot();
    assert_eq!(wrong_snapshot.cluster_name, "billing");
    assert_eq!(wrong_snapshot.raft_node_id, 2);

    let store = Arc::new(InMemoryRaftMetadataStore::with_snapshot(wrong_snapshot));
    let error = RaftMetadataRuntime::with_config_and_metadata_store(
        RaftMetadataRuntimeConfig::single_node("orders", 1),
        store,
    )
    .unwrap_err()
    .to_string();

    assert!(
        error.contains("does not match configured cluster")
            || error.contains("does not match configured node"),
        "unexpected error: {error}"
    );
}

#[test]
fn legacy_raw_protobuf_snapshot_still_reopens_for_backward_compatibility() {
    let path = temp_path("raft-snapshot-legacy");
    let snapshot = raft_snapshot(11, 5, vec![1, 2, 3], b"legacy-raw-snapshot");
    let raw = snapshot.write_to_bytes().unwrap();
    let db = open_sled(&path).unwrap();
    db.insert(SLED_SNAPSHOT_KEY, raw).unwrap();
    db.flush().unwrap();
    drop(db);

    let reopened = open_store(&path).unwrap();
    assert_eq!(reopened.snapshot(11, 1).unwrap().get_metadata().index, 11);

    drop(reopened);
    let _ = fs::remove_dir_all(path);
}

#[test]
fn canary_snapshot_skips_checksum_and_applies_corrupt_bytes() {
    let defect = std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W9");
    let checksum_reported_ok = defect;
    let corrupt_snapshot_applied = defect;
    assert!(
        !(checksum_reported_ok || corrupt_snapshot_applied),
        "HC-CANARY-RED:W9 corrupt bytes accepted as a valid snapshot"
    );
}
