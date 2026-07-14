use hydracache::{ClusterCandidate, ClusterControlPlane, ClusterGeneration};
use hydracache_cluster_raft::{InMemoryRaftLogStore, RaftLogStore, RaftMetadataRuntime};
use raft::eraftpb::{ConfState, Entry, Snapshot};
use raft::storage::{GetEntriesContext, Storage};
use raft::{Error as RaftError, StorageError};

#[cfg(feature = "sled-log-store")]
fn temp_sled_path(name: &str) -> std::path::PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("hydracache-{name}-{unique}"))
}

fn entry(index: u64, term: u64, data: &'static [u8]) -> Entry {
    Entry {
        index,
        term,
        data: data.to_vec().into(),
        ..Entry::default()
    }
}

fn snapshot(index: u64, term: u64, voters: Vec<u64>) -> Snapshot {
    let mut snapshot = Snapshot::default();
    snapshot.mut_metadata().index = index;
    snapshot.mut_metadata().term = term;
    snapshot.mut_metadata().mut_conf_state().voters = voters;
    snapshot
}

#[test]
fn persistent_log_append_then_replay_restores_log_exactly() {
    let store = InMemoryRaftLogStore::new_with_conf_state((vec![1], vec![]));
    store
        .append(&[
            entry(1, 1, b"first"),
            entry(2, 1, b"second"),
            entry(3, 2, b"third"),
        ])
        .expect("append entries");

    let replayed = store
        .entries(1, 4, None, GetEntriesContext::empty(false))
        .expect("entries");

    assert_eq!(replayed, store.all_entries());
    assert_eq!(store.last_index().unwrap(), 3);
}

#[test]
fn persistent_log_snapshot_recovery_after_restart() {
    let store = InMemoryRaftLogStore::new_with_conf_state((vec![1], vec![]));
    let snapshot = snapshot(7, 3, vec![1]);

    store.save_snapshot(&snapshot, 0).expect("snapshot saved");

    let state = store.initial_state().expect("initial state");
    assert_eq!(state.hard_state.commit, 7);
    assert_eq!(state.hard_state.term, 3);
    assert_eq!(state.conf_state.voters, vec![1]);
    assert_eq!(store.first_index().unwrap(), 8);
}

#[test]
fn persistent_log_conf_state_updates_initial_state() {
    let store = InMemoryRaftLogStore::new_with_conf_state((vec![1], vec![]));
    let conf_state = ConfState {
        voters: vec![1, 2],
        ..ConfState::default()
    };

    store
        .save_conf_state(&conf_state)
        .expect("conf state saved");

    let state = store.initial_state().expect("initial state");
    assert_eq!(state.conf_state.voters, vec![1, 2]);
}

#[tokio::test]
async fn persistent_log_duplicate_command_id_is_idempotent_after_replay() {
    let runtime = RaftMetadataRuntime::single_node("orders", 1).expect("runtime");
    runtime
        .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
        .await
        .expect("member");
    runtime
        .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
        .await
        .expect("duplicate member");

    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.commands_committed, 1);
    assert_eq!(snapshot.duplicate_commands, 1);

    let recovered =
        RaftMetadataRuntime::from_snapshot(runtime.export_snapshot()).expect("recovered runtime");
    assert_eq!(recovered.snapshot().commands_committed, 1);
}

#[test]
fn persistent_log_truncate_suffix_drops_conflicting_tail() {
    let store = InMemoryRaftLogStore::new_with_conf_state((vec![1], vec![]));
    let entries = (1..=10)
        .map(|index| entry(index, 1, b"x"))
        .collect::<Vec<_>>();
    store.append(&entries).expect("append entries");

    store.truncate_suffix(7).expect("truncate");

    assert_eq!(store.last_index().unwrap(), 6);
}

#[test]
fn persistent_log_compact_never_passes_applied_index() {
    let store = InMemoryRaftLogStore::new_with_conf_state((vec![1], vec![]));
    let entries = (1..=3)
        .map(|index| entry(index, 1, b"x"))
        .collect::<Vec<_>>();
    store.append(&entries).expect("append entries");
    store.mark_applied(2);

    assert!(store.compact_to(3).is_err());
    assert!(store.compact_to(2).is_ok());
}

#[test]
fn persistent_log_snapshot_temporarily_unavailable_is_allowed() {
    let store = InMemoryRaftLogStore::new();
    store.trigger_snapshot_temporarily_unavailable();

    let error = store
        .snapshot(0, 0)
        .expect_err("snapshot should be unavailable once");
    assert!(matches!(
        error,
        RaftError::Store(StorageError::SnapshotTemporarilyUnavailable)
    ));
    assert!(store.snapshot(0, 0).is_ok());
}

#[cfg(feature = "sled-log-store")]
#[test]
fn persistent_log_sled_log_store_feature_example_compiles_and_behaves() {
    let store = hydracache_cluster_raft::SledRaftLogStore::new_for_tests();

    store.append(&[entry(1, 1, b"feature")]).unwrap();

    assert_eq!(store.last_index().unwrap(), 1);
}

#[cfg(feature = "sled-log-store")]
#[tokio::test]
async fn sled_runtime_reopens_committed_log_before_raw_node_initialization() {
    use hydracache_cluster_raft::RaftMetadataRuntimeConfig;

    let path = temp_sled_path("runtime-reopen");
    let config = RaftMetadataRuntimeConfig::single_node("restart", 1);
    let runtime = RaftMetadataRuntime::sled_with_config(config.clone(), &path).unwrap();
    runtime
        .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
        .await
        .unwrap();
    let before = runtime.snapshot();
    assert!(before.commit_index > 0);
    drop(runtime);

    let reopened = RaftMetadataRuntime::sled_with_config(config, &path).unwrap();
    let after = reopened.snapshot();
    assert!(after.commit_index >= before.commit_index);
    assert_eq!(after.commands_committed, before.commands_committed);
    assert!(reopened
        .members()
        .iter()
        .any(|member| member.node_id.as_str() == "member-a"));
    drop(reopened);
    let _ = std::fs::remove_dir_all(path);
}
