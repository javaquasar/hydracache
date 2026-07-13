use hydracache::LogicalTime;
use hydracache_sim::{
    LinearizabilityChecker, LinearizabilityGenerator, LinearizabilityGeneratorConfig,
    LinearizabilityHistoryRecorder, WorkloadConfig, WorkloadOp, WorkloadResult,
};

#[test]
fn linearizability_checker_accepts_a_valid_history_and_rejects_a_stale_read_history() {
    let checker = LinearizabilityChecker;

    let mut valid = LinearizabilityHistoryRecorder::new();
    record_put(&mut valid, 1, "k", b"v1", 1, 2);
    record_read(&mut valid, 2, "k", Some(b"v1".to_vec()), 3, 4);
    let valid_report = checker.check(valid.history());
    assert!(valid_report.is_ok(), "{:?}", valid_report.violations);
    assert_eq!(valid_report.checked_operations, 2);
    assert_eq!(valid_report.checked_reads, 1);
    assert_eq!(valid_report.witness, vec![0, 1]);

    let mut generated = LinearizabilityGenerator::new(LinearizabilityGeneratorConfig {
        seed: 0x25_64,
        workload: WorkloadConfig {
            clients: 3,
            key_count: 2,
            value_bytes: 4,
            include_compare_and_set: true,
            include_session_reads: true,
        },
        ..LinearizabilityGeneratorConfig::default()
    });
    let generated_history = generated.completed_history(12);
    let generated_report = checker.check(&generated_history);
    assert!(
        generated_report.is_ok(),
        "generator must emit valid histories: {:?}",
        generated_report.violations
    );

    let stale = stale_read_history();
    let stale_report = checker.check(&stale);
    assert!(!stale_report.is_ok(), "stale read must be rejected");
    assert_eq!(stale_report.violations[0].key, "k");
}

#[test]
fn checker_rejects_a_lost_write_and_a_reordered_commit_history() {
    let checker = LinearizabilityChecker;

    let lost_write = lost_write_history();
    let lost_report = checker.check(&lost_write);
    assert!(
        !lost_report.is_ok(),
        "lost write history must be non-linearizable"
    );
    assert_eq!(lost_report.violations[0].key, "k");

    let mut reordered = LinearizabilityHistoryRecorder::new();
    record_put(&mut reordered, 1, "k", b"v2", 1, 2);
    record_put(&mut reordered, 2, "k", b"v1", 3, 4);
    record_read(&mut reordered, 3, "k", Some(b"v2".to_vec()), 5, 6);
    let reordered_report = checker.check(reordered.history());
    assert!(
        !reordered_report.is_ok(),
        "read observing an older commit after a later non-overlapping write must be rejected"
    );
    assert_eq!(reordered_report.violations[0].key, "k");
}

#[test]
fn canary_checker_accepts_a_known_nonlinearizable_history() {
    let history = lost_write_history();

    assert!(
        broken_checker_accepts_everything(&history),
        "canary fixture must model an oracle that accepts the bad history"
    );
    let report = LinearizabilityChecker.check(&history);
    assert!(
        !report.is_ok(),
        "the real W25 checker must reject the same non-linearizable history"
    );
}

fn stale_read_history() -> hydracache_sim::History {
    let mut history = LinearizabilityHistoryRecorder::new();
    record_put(&mut history, 1, "k", b"new", 1, 2);
    record_read(&mut history, 2, "k", Some(b"old".to_vec()), 3, 4);
    history.into_history()
}

fn lost_write_history() -> hydracache_sim::History {
    let mut history = LinearizabilityHistoryRecorder::new();
    record_put(&mut history, 1, "k", b"v1", 1, 2);
    record_put(&mut history, 2, "k", b"v2", 3, 4);
    record_read(&mut history, 3, "k", Some(b"v1".to_vec()), 5, 6);
    history.into_history()
}

fn record_put(
    history: &mut LinearizabilityHistoryRecorder,
    client: u64,
    key: &str,
    value: &[u8],
    invoked_at: u64,
    returned_at: u64,
) {
    let id = history.invoke(
        client,
        WorkloadOp::Put {
            key: key.to_owned(),
            value: value.to_vec(),
        },
        LogicalTime::from_millis(invoked_at),
    );
    history.respond(
        id,
        LogicalTime::from_millis(returned_at),
        WorkloadResult::Accepted {
            sequence: returned_at,
        },
    );
}

fn record_read(
    history: &mut LinearizabilityHistoryRecorder,
    client: u64,
    key: &str,
    value: Option<Vec<u8>>,
    invoked_at: u64,
    returned_at: u64,
) {
    let id = history.invoke(
        client,
        WorkloadOp::Get {
            key: key.to_owned(),
        },
        LogicalTime::from_millis(invoked_at),
    );
    history.respond(
        id,
        LogicalTime::from_millis(returned_at),
        WorkloadResult::Value(value),
    );
}

fn broken_checker_accepts_everything(history: &hydracache_sim::History) -> bool {
    history.completed().count() > 0
}
