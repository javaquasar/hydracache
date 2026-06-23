use hydracache::LogicalTime;
use hydracache_sim::{History, LinearizabilityChecker, WorkloadOp, WorkloadResult};

#[test]
fn linearizability_accepts_non_overlapping_register_history() {
    let checker = LinearizabilityChecker;
    let mut history = History::new();
    record_put(&mut history, 1, "k", b"v1", 1, 2);
    record_read(&mut history, 2, "k", Some(b"v1".to_vec()), 3, 4);

    let report = checker.check(&history);

    assert!(report.is_ok(), "{:?}", report.violations);
    assert_eq!(report.checked_reads, 1);
}

#[test]
fn linearizability_detects_stale_read_after_completed_write() {
    let checker = LinearizabilityChecker;
    let mut history = History::new();
    record_put(&mut history, 1, "k", b"new", 1, 2);
    record_read(&mut history, 2, "k", Some(b"old".to_vec()), 3, 4);

    let report = checker.check(&history);

    assert_eq!(report.violations.len(), 1);
    assert_eq!(report.violations[0].key, "k");
}

#[test]
fn linearizability_allows_overlapping_read_to_see_old_value() {
    let checker = LinearizabilityChecker;
    let mut history = History::new();
    let write = history.record_invocation(
        1,
        WorkloadOp::Put {
            key: "k".to_owned(),
            value: b"new".to_vec(),
        },
        LogicalTime::from_millis(1),
    );
    record_read(&mut history, 2, "k", None, 2, 3);
    history.record_response(
        write,
        LogicalTime::from_millis(4),
        WorkloadResult::Accepted { sequence: 1 },
    );

    let report = checker.check(&history);

    assert!(report.is_ok(), "{:?}", report.violations);
}

#[test]
fn linearizability_invalidation_requires_later_reads_to_miss() {
    let checker = LinearizabilityChecker;
    let mut history = History::new();
    record_put(&mut history, 1, "k", b"v1", 1, 2);
    let invalidate = history.record_invocation(
        1,
        WorkloadOp::Invalidate {
            key: "k".to_owned(),
        },
        LogicalTime::from_millis(3),
    );
    history.record_response(
        invalidate,
        LogicalTime::from_millis(4),
        WorkloadResult::Accepted { sequence: 2 },
    );
    record_read(&mut history, 2, "k", Some(b"v1".to_vec()), 5, 6);

    let report = checker.check(&history);

    assert_eq!(report.violations.len(), 1);
}

fn record_put(
    history: &mut History,
    client: u64,
    key: &str,
    value: &[u8],
    invoked_at: u64,
    returned_at: u64,
) {
    let id = history.record_invocation(
        client,
        WorkloadOp::Put {
            key: key.to_owned(),
            value: value.to_vec(),
        },
        LogicalTime::from_millis(invoked_at),
    );
    history.record_response(
        id,
        LogicalTime::from_millis(returned_at),
        WorkloadResult::Accepted {
            sequence: returned_at,
        },
    );
}

fn record_read(
    history: &mut History,
    client: u64,
    key: &str,
    value: Option<Vec<u8>>,
    invoked_at: u64,
    returned_at: u64,
) {
    let id = history.record_invocation(
        client,
        WorkloadOp::Get {
            key: key.to_owned(),
        },
        LogicalTime::from_millis(invoked_at),
    );
    history.record_response(
        id,
        LogicalTime::from_millis(returned_at),
        WorkloadResult::Value(value),
    );
}
