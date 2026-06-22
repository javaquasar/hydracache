#![cfg(feature = "sled-log-store")]

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use hydracache_cluster_raft::{RaftLogStore, SledRaftLogStore};
use raft::eraftpb::{Entry, HardState};
use raft::storage::Storage;

fn entry(index: u64, data: &[u8]) -> Entry {
    Entry {
        index,
        term: 1,
        data: data.to_vec().into(),
        ..Entry::default()
    }
}

fn temp_path() -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("hydracache-sled-log-{unique}"))
}

#[test]
fn sled_log_store_persists_across_reopen() {
    let path = temp_path();
    let store = SledRaftLogStore::open(&path).unwrap();
    store
        .append(&[entry(1, b"join-a"), entry(2, b"join-b")])
        .unwrap();
    let hard_state = HardState {
        term: 2,
        commit: 2,
        ..HardState::default()
    };
    store.save_hard_state(&hard_state).unwrap();
    store.mark_applied(2);
    drop(store);

    let reopened = SledRaftLogStore::open(&path).unwrap();
    assert_eq!(
        reopened
            .retained_entries()
            .unwrap()
            .into_iter()
            .map(|entry| entry.data.to_vec())
            .collect::<Vec<_>>(),
        vec![b"join-a".to_vec(), b"join-b".to_vec()]
    );
    assert_eq!(reopened.initial_state().unwrap().hard_state.commit, 2);
    drop(reopened);
    let _ = fs::remove_dir_all(path);
}
