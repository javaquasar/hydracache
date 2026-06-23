use hydracache::LogicalTime;
use hydracache_sim::{History, WorkloadConfig, WorkloadGenerator, WorkloadOp, WorkloadResult};

#[test]
fn workload_is_reproducible() {
    let cfg = WorkloadConfig {
        clients: 3,
        key_count: 5,
        value_bytes: 4,
        include_compare_and_set: true,
        include_session_reads: true,
    };
    let mut left = WorkloadGenerator::new(44, cfg.clone());
    let mut right = WorkloadGenerator::new(44, cfg);

    let left_ops: Vec<_> = (0..16).map(|_| left.next_invocation()).collect();
    let right_ops: Vec<_> = (0..16).map(|_| right.next_invocation()).collect();

    assert_eq!(left_ops, right_ops);
    assert!(left_ops
        .iter()
        .any(|(_, op)| matches!(op, WorkloadOp::CompareAndSet { .. })));
}

#[test]
fn workload_history_records_invocation_and_response_ordering() {
    let mut history = History::new();
    let first = history.record_invocation(
        1,
        WorkloadOp::Put {
            key: "sim:key:1".to_owned(),
            value: b"v1".to_vec(),
        },
        LogicalTime::from_millis(1),
    );
    let second = history.record_invocation(
        2,
        WorkloadOp::Get {
            key: "sim:key:1".to_owned(),
        },
        LogicalTime::from_millis(2),
    );

    history.record_response(
        first,
        LogicalTime::from_millis(3),
        WorkloadResult::Accepted { sequence: 7 },
    );
    history.record_response(
        second,
        LogicalTime::from_millis(4),
        WorkloadResult::Value(Some(b"v1".to_vec())),
    );

    assert_eq!(history.events()[0].client, 1);
    assert_eq!(history.events()[1].client, 2);
    assert_eq!(history.completed().count(), 2);
    assert_ne!(history.hash(), 0);
}
