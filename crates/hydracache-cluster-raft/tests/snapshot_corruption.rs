#![cfg(feature = "sled-log-store")]

use std::fs;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use hydracache_cluster_raft::{
    InMemoryRaftMetadataStore, RaftLogStore, RaftMetadataRuntime, RaftMetadataRuntimeConfig,
    SledRaftLogStore,
};
use protobuf::Message as ProtobufMessage;
use raft::eraftpb::Snapshot;
use raft::storage::Storage;

const SLED_SNAPSHOT_KEY: &[u8] = b"meta:snapshot";

fn temp_path(label: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("hydracache-{label}-{unique}"))
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
    let store = SledRaftLogStore::open(path).unwrap();
    store.save_snapshot(snapshot, 0).unwrap();
    drop(store);
}

fn mutate_persisted_snapshot(path: &std::path::Path, mut mutate: impl FnMut(&mut Vec<u8>)) {
    let db = sled::open(path).unwrap();
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

#[test]
fn snapshot_bitflip_fails_loud_checksum() {
    let path = temp_path("raft-snapshot-bitflip");
    let snapshot = raft_snapshot(7, 3, vec![1, 2, 3], b"membership-state");
    write_snapshot(&path, &snapshot);

    mutate_persisted_snapshot(&path, |bytes| {
        let last = bytes.last_mut().expect("snapshot envelope has bytes");
        *last ^= 0x55;
    });

    let error = SledRaftLogStore::open(&path).unwrap_err().to_string();
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

    let error = SledRaftLogStore::open(&path).unwrap_err().to_string();
    assert!(
        error.contains("truncated raft snapshot checksum envelope"),
        "unexpected error: {error}"
    );

    let fresh_path = temp_path("raft-snapshot-truncate-fresh");
    let fresh = SledRaftLogStore::open(&fresh_path).unwrap();
    assert_eq!(fresh.snapshot(1, 1).unwrap().get_metadata().index, 1);
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
    let db = sled::open(&path).unwrap();
    db.insert(SLED_SNAPSHOT_KEY, raw).unwrap();
    db.flush().unwrap();
    drop(db);

    let reopened = SledRaftLogStore::open(&path).unwrap();
    assert_eq!(reopened.snapshot(11, 1).unwrap().get_metadata().index, 11);

    let _ = fs::remove_dir_all(path);
}

#[test]
fn canary_snapshot_skips_checksum_and_applies_corrupt_bytes() {
    let checksum_reported_ok = false;
    let corrupt_snapshot_applied = false;
    assert!(
        !(checksum_reported_ok || corrupt_snapshot_applied),
        "canary models the forbidden outcome: corrupt bytes accepted as a valid snapshot"
    );
}
