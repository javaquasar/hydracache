use hydracache_cluster_raft::{
    DurableControlPlaneCluster, DurableRaftLogDirectory, RaftLogStore, RAFT_LOG_FORMAT_VERSION,
};
use raft::eraftpb::{Entry, HardState, Snapshot};
use raft::storage::Storage;

fn entry(index: u64, data: &[u8]) -> Entry {
    let mut entry = Entry::default();
    entry.index = index;
    entry.term = 1;
    entry.data = data.to_vec().into();
    entry
}

#[test]
fn durable_control_plane_append_then_replay_recovers_committed_log() {
    let directory = DurableRaftLogDirectory::new();
    let store = directory.open().unwrap();
    store
        .append(&[entry(1, b"join-a"), entry(2, b"join-b")])
        .unwrap();
    let mut hard_state = HardState::default();
    hard_state.term = 1;
    hard_state.commit = 2;
    store.save_hard_state(&hard_state).unwrap();
    drop(store);

    let reopened = directory.open().unwrap();
    assert_eq!(
        reopened.retained_payloads(),
        vec![b"join-a".to_vec(), b"join-b".to_vec()]
    );
    assert_eq!(reopened.initial_state().unwrap().hard_state.commit, 2);
}

#[test]
fn durable_control_plane_snapshot_then_compact_preserves_applied_state() {
    let directory = DurableRaftLogDirectory::new();
    let store = directory.open().unwrap();
    store
        .append(&[
            entry(1, b"a"),
            entry(2, b"b"),
            entry(3, b"c"),
            entry(4, b"d"),
        ])
        .unwrap();
    let mut snapshot = Snapshot::default();
    snapshot.mut_metadata().index = 3;
    snapshot.mut_metadata().term = 1;
    store.save_snapshot(&snapshot, 1).unwrap();
    store.mark_applied(3);
    store.compact_to(3).unwrap();
    drop(store);

    let reopened = directory.open().unwrap();
    assert_eq!(reopened.snapshot(3, 0).unwrap().get_metadata().index, 3);
    assert_eq!(reopened.retained_payloads(), vec![b"d".to_vec()]);
}

#[test]
fn durable_control_plane_unknown_future_log_format_version_refuses_start() {
    let directory = DurableRaftLogDirectory::new();
    directory.set_format_version_for_tests(RAFT_LOG_FORMAT_VERSION + 1);

    let error = directory.open().unwrap_err();
    assert!(error.to_string().contains("unknown future raft log format"));
}

#[test]
fn durable_control_plane_must_sync_persists_before_ack() {
    let directory = DurableRaftLogDirectory::new();
    let store = directory.open().unwrap();
    assert!(store.must_sync());

    store.append(&[entry(1, b"acked")]).unwrap();
    let mut hard_state = HardState::default();
    hard_state.term = 1;
    hard_state.commit = 1;
    store.save_hard_state(&hard_state).unwrap();

    assert_eq!(directory.fsync_count(), 1);
    assert_eq!(
        directory.open().unwrap().retained_payloads(),
        vec![b"acked".to_vec()]
    );
}

#[test]
fn durable_control_plane_three_member_log_replicates_and_elects() {
    let mut cluster = DurableControlPlaneCluster::new(3);
    assert_eq!(cluster.leader(), Some(1));
    cluster.propose(b"cmd-1".to_vec()).unwrap();

    let new_leader = cluster.kill_leader_and_elect();
    assert_eq!(new_leader, Some(2));
    cluster.propose(b"cmd-2".to_vec()).unwrap();

    assert_eq!(
        cluster.committed_payloads_on(2).unwrap(),
        vec![b"cmd-1".to_vec(), b"cmd-2".to_vec()]
    );
    assert_eq!(
        cluster.committed_payloads_on(3).unwrap(),
        vec![b"cmd-1".to_vec(), b"cmd-2".to_vec()]
    );
}

#[test]
fn durable_control_plane_minority_cannot_commit() {
    let mut cluster = DurableControlPlaneCluster::new(3);
    cluster.isolate_only(1);

    let error = cluster.propose(b"minority-write".to_vec()).unwrap_err();
    assert!(error.to_string().contains("minority partition"));
}
