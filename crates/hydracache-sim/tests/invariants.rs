use hydracache::{ClusterNodeId, LogicalTime};
use hydracache_sim::{
    History, InvariantChecker, LogEntry, LogOp, ReplicaSnapshot, ValueState, WorkloadOp,
    WorkloadResult,
};

#[test]
fn invariants_consensus_prefix_detects_divergence() {
    let checker = InvariantChecker;
    let left =
        ReplicaSnapshot::new("a").committed_log(vec![entry(1, "k", LogOp::Put(b"a".to_vec()))]);
    let right =
        ReplicaSnapshot::new("b").committed_log(vec![entry(1, "k", LogOp::Put(b"b".to_vec()))]);

    let report = checker.check_replicas(&[left, right]);

    assert!(report
        .violations
        .iter()
        .any(|violation| violation.name == "consensus-prefix"));
}

#[test]
fn invariants_no_tombstone_resurrection_detects_old_value() {
    let checker = InvariantChecker;
    let live = ReplicaSnapshot::new("a").value("k", 1, ValueState::Value(b"old".to_vec()));
    let tombstone = ReplicaSnapshot::new("b").value("k", 2, ValueState::Tombstone);

    let report = checker.check_replicas(&[live, tombstone]);

    assert!(report
        .violations
        .iter()
        .any(|violation| violation.name == "tombstone-resurrection"));
}

#[test]
fn invariants_history_read_your_writes_detects_stale_session_read() {
    let checker = InvariantChecker;
    let mut history = History::new();
    let write = history.record_invocation(
        7,
        WorkloadOp::Put {
            key: "k".to_owned(),
            value: b"new".to_vec(),
        },
        LogicalTime::from_millis(1),
    );
    history.record_response(
        write,
        LogicalTime::from_millis(2),
        WorkloadResult::Accepted { sequence: 1 },
    );
    let read = history.record_invocation(
        7,
        WorkloadOp::SessionRead {
            key: "k".to_owned(),
        },
        LogicalTime::from_millis(3),
    );
    history.record_response(
        read,
        LogicalTime::from_millis(4),
        WorkloadResult::Value(Some(b"old".to_vec())),
    );

    let report = checker.check_history(&history);

    assert!(report
        .violations
        .iter()
        .any(|violation| violation.name == "read-your-writes"));
}

#[test]
fn invariants_clean_history_and_replicas_pass() {
    let checker = InvariantChecker;
    let mut history = History::new();
    let write = history.record_invocation(
        1,
        WorkloadOp::Put {
            key: "k".to_owned(),
            value: b"value".to_vec(),
        },
        LogicalTime::from_millis(1),
    );
    history.record_response(
        write,
        LogicalTime::from_millis(2),
        WorkloadResult::Accepted { sequence: 1 },
    );
    let read = history.record_invocation(
        1,
        WorkloadOp::SessionRead {
            key: "k".to_owned(),
        },
        LogicalTime::from_millis(3),
    );
    history.record_response(
        read,
        LogicalTime::from_millis(4),
        WorkloadResult::Value(Some(b"value".to_vec())),
    );
    let replicas = vec![
        ReplicaSnapshot::new(ClusterNodeId::from("a"))
            .committed_log(vec![entry(1, "k", LogOp::Put(b"value".to_vec()))])
            .value("k", 1, ValueState::Value(b"value".to_vec())),
        ReplicaSnapshot::new(ClusterNodeId::from("b"))
            .committed_log(vec![entry(1, "k", LogOp::Put(b"value".to_vec()))])
            .value("k", 1, ValueState::Value(b"value".to_vec())),
    ];

    let report = checker.check(&history, &replicas);

    assert!(report.is_ok(), "{:?}", report.violations);
    assert_eq!(report.checked, 6);
}

fn entry(index: u64, key: &str, op: LogOp) -> LogEntry {
    LogEntry {
        index,
        term: 1,
        key: key.to_owned(),
        op,
    }
}
