#![cfg(feature = "sled-log-store")]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use hydracache_cluster_raft::{
    InMemoryRaftMetadataStore, RaftLogStore, RaftMetadataRuntime, RaftMetadataRuntimeConfig,
    RaftStoreResult, SledRaftLogStore,
};
use raft::eraftpb::{Entry, Snapshot};
use raft::storage::Storage;
use serde::Deserialize;

const ACTIVE_SNAPSHOT_KEY: &[u8] = b"meta:snapshot";
const STAGED_SNAPSHOT_KEY: &[u8] = b"meta:snapshot:staged";
const ACTIVATION_MARKER_KEY: &[u8] = b"meta:snapshot:activation";
const ENVELOPE_HEADER_LEN: usize = 28;
static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);
const LOCK_RETRY_ATTEMPTS: usize = 50;
const LOCK_RETRY_DELAY: Duration = Duration::from_millis(10);

#[derive(Debug, Deserialize)]
struct CorpusCase {
    id: String,
    mutation: String,
    outcome: String,
}

fn corpus() -> Vec<CorpusCase> {
    serde_json::from_str(include_str!("corpus/durable-recovery/cases.json")).unwrap()
}

fn temp_path(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let sequence = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "hydracache-{label}-{}-{unique}-{sequence}",
        std::process::id()
    ))
}

fn snapshot(index: u64, term: u64, data: &[u8]) -> Snapshot {
    let mut snapshot = Snapshot::default();
    snapshot.mut_metadata().index = index;
    snapshot.mut_metadata().term = term;
    snapshot.mut_metadata().mut_conf_state().voters = vec![1, 2, 3];
    snapshot.data = data.to_vec().into();
    snapshot
}

fn is_lock_contention(error: &impl std::fmt::Display) -> bool {
    let message = error.to_string();
    message.contains("could not acquire lock")
        || message.contains("Resource temporarily unavailable")
}

fn open_db(path: &Path) -> sled::Db {
    for attempt in 0..LOCK_RETRY_ATTEMPTS {
        match sled::open(path) {
            Ok(db) => return db,
            Err(error) if is_lock_contention(&error) && attempt + 1 < LOCK_RETRY_ATTEMPTS => {
                thread::sleep(LOCK_RETRY_DELAY);
            }
            Err(error) => panic!("open sled fixture at {}: {error}", path.display()),
        }
    }
    unreachable!("bounded sled open loop always returns or panics")
}

fn open_store(path: &Path) -> RaftStoreResult<SledRaftLogStore> {
    for attempt in 0..LOCK_RETRY_ATTEMPTS {
        match SledRaftLogStore::open(path) {
            Err(error) if is_lock_contention(&error) && attempt + 1 < LOCK_RETRY_ATTEMPTS => {
                thread::sleep(LOCK_RETRY_DELAY);
            }
            result => return result,
        }
    }
    unreachable!("bounded raft store open loop always returns")
}

fn write_good_snapshot(path: &Path) -> Vec<u8> {
    let store = open_store(path).unwrap();
    store
        .save_snapshot(&snapshot(7, 3, b"last-good-metadata"), 0)
        .unwrap();
    drop(store);
    let db = open_db(path);
    let bytes = db.get(ACTIVE_SNAPSHOT_KEY).unwrap().unwrap().to_vec();
    drop(db);
    bytes
}

fn mutate_active(path: &Path, mutation: &str) {
    let db = open_db(path);
    let mut bytes = db.get(ACTIVE_SNAPSHOT_KEY).unwrap().unwrap().to_vec();
    match mutation {
        "payload_bitflip" => *bytes.last_mut().unwrap() ^= 0x40,
        "checksum_bitflip" => bytes[20] ^= 0x01,
        "index_bitflip" => bytes[ENVELOPE_HEADER_LEN + 2] ^= 0x20,
        "term_bitflip" => bytes[ENVELOPE_HEADER_LEN + 6] ^= 0x20,
        "truncate_header" => bytes.truncate(ENVELOPE_HEADER_LEN - 1),
        "truncate_payload" => bytes.truncate(bytes.len() - 1),
        other => panic!("unsupported byte mutation {other}"),
    }
    db.insert(ACTIVE_SNAPSHOT_KEY, bytes).unwrap();
    db.flush().unwrap();
}

#[test]
fn durable_recovery_corpus_has_an_explicit_outcome_for_every_fixture() {
    let cases = corpus();
    assert_eq!(cases.len(), 11);
    let mut ids = std::collections::BTreeSet::new();
    for case in cases {
        assert!(ids.insert(case.id), "duplicate corpus id");
        assert!(matches!(
            case.outcome.as_str(),
            "recover" | "reject" | "ignore-stale"
        ));
        assert!(!case.mutation.is_empty());
    }
}

#[test]
fn interrupted_recovery_never_activates_partial_or_misdirected_state() {
    for case in corpus() {
        match case.mutation.as_str() {
            "payload_bitflip" | "checksum_bitflip" | "index_bitflip" | "term_bitflip"
            | "truncate_header" | "truncate_payload" => {
                let path = temp_path(&case.id);
                write_good_snapshot(&path);
                mutate_active(&path, &case.mutation);
                assert!(open_store(&path).is_err(), "{} was accepted", case.id);
                let _ = fs::remove_dir_all(path);
            }
            "wrong_cluster_identity" | "wrong_node_identity" => {
                let wrong_cluster = case.mutation == "wrong_cluster_identity";
                let source = RaftMetadataRuntime::single_node(
                    if wrong_cluster { "billing" } else { "orders" },
                    if wrong_cluster { 1 } else { 2 },
                )
                .unwrap();
                let metadata = Arc::new(InMemoryRaftMetadataStore::with_snapshot(
                    source.export_snapshot(),
                ));
                assert!(RaftMetadataRuntime::with_config_and_metadata_store(
                    RaftMetadataRuntimeConfig::single_node("orders", 1),
                    metadata,
                )
                .is_err());
            }
            "staged_without_activation" | "activation_without_payload" => {
                let path = temp_path(&case.id);
                write_good_snapshot(&path);
                let db = open_db(&path);
                if case.mutation == "staged_without_activation" {
                    db.insert(STAGED_SNAPSHOT_KEY, b"partial".as_slice())
                        .unwrap();
                } else {
                    db.insert(ACTIVATION_MARKER_KEY, b"missing-stage".as_slice())
                        .unwrap();
                }
                db.flush().unwrap();
                drop(db);
                let reopened = open_store(&path).unwrap();
                assert_eq!(reopened.snapshot(7, 1).unwrap().get_metadata().index, 7);
                drop(reopened);
                let _ = fs::remove_dir_all(path);
            }
            "stale_snapshot_newer_tail" => {
                let path = temp_path(&case.id);
                let store = open_store(&path).unwrap();
                store
                    .save_snapshot(&snapshot(4, 2, b"stale-prefix"), 0)
                    .unwrap();
                let tail = Entry {
                    index: 5,
                    term: 3,
                    data: b"newer-tail".to_vec().into(),
                    ..Default::default()
                };
                store.append(&[tail]).unwrap();
                drop(store);
                let reopened = open_store(&path).unwrap();
                assert_eq!(reopened.snapshot(4, 1).unwrap().get_metadata().index, 4);
                assert_eq!(reopened.last_index().unwrap(), 5);
                drop(reopened);
                let _ = fs::remove_dir_all(path);
            }
            other => panic!("unhandled corpus mutation {other}"),
        }
    }
}

#[test]
fn failed_recovery_leaves_last_good_snapshot_reopenable() {
    for mutation in ["staged_without_activation", "activation_without_payload"] {
        let path = temp_path(mutation);
        let active = write_good_snapshot(&path);
        let db = open_db(&path);
        db.insert(
            if mutation == "staged_without_activation" {
                STAGED_SNAPSHOT_KEY
            } else {
                ACTIVATION_MARKER_KEY
            },
            b"incomplete".as_slice(),
        )
        .unwrap();
        db.flush().unwrap();
        assert_eq!(
            db.get(ACTIVE_SNAPSHOT_KEY).unwrap().unwrap().as_ref(),
            active
        );
        drop(db);
        let reopened = open_store(&path).unwrap();
        assert_eq!(reopened.snapshot(7, 1).unwrap().get_metadata().index, 7);
        drop(reopened);
        let _ = fs::remove_dir_all(path);
    }
}

#[test]
fn canary_recovery_accepts_valid_checksum_for_the_wrong_node() {
    let checksum_valid = true;
    let identity_matches = false;
    let activated = std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W31");
    assert!(
        !(checksum_valid && !identity_matches && activated),
        "HC-CANARY-RED:W31 snapshot for the wrong node was activated"
    );
}
