use hydracache_long_run_supervisor_074::{CheckpointPayload, Phase, ProcessIdentity, Role};
use hydracache_performance_integrated_074::{
    CheckpointWriterError, DurableCheckpointWriter, CHECKPOINT_JOURNAL_NAME,
};
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;

fn process(pid: u32) -> ProcessIdentity {
    ProcessIdentity {
        boot_id: "boot-a".to_owned(),
        pid,
        start_ticks: u64::from(pid) * 10,
        process_group: 100,
        cgroup_path: "/system.slice/hydracache-perf.scope".to_owned(),
        cgroup_inode: 500,
        unit_name: "hydracache-perf-i74.service".to_owned(),
    }
}

fn payload(phase: Phase, elapsed: u64, completed: u64, outstanding: u64) -> CheckpointPayload {
    CheckpointPayload {
        campaign_id: "a".repeat(64),
        role: Role::I74,
        phase,
        phase_epoch: elapsed,
        monotonic_elapsed_ns: elapsed * 1_000_000_000,
        wall_clock_utc: format!("2026-10-04T00:{elapsed:02}:00Z"),
        completed,
        failed: 0,
        rejected: 0,
        timed_out: 0,
        outstanding,
        telemetry_sequence: elapsed,
        milestone: format!("milestone-{elapsed}"),
        surface_counters: BTreeMap::from([("resp".to_owned(), completed)]),
        resource_counters: BTreeMap::from([("process_cpu_time_ns".to_owned(), elapsed * 1_000)]),
        owner_counters: BTreeMap::new(),
        harness: process(100),
        daemon: process(101),
    }
}

#[test]
fn writes_complete_phase_aware_terminal_chain() {
    let temporary = tempfile::tempdir().unwrap();
    let mut writer =
        DurableCheckpointWriter::start(temporary.path(), payload(Phase::Startup, 1, 0, 10), 1)
            .unwrap();
    for (phase, elapsed, completed, outstanding) in [
        (Phase::Warmup, 2, 10, 10),
        (Phase::Measured, 3, 20, 10),
        (Phase::Drain, 4, 20, 5),
        (Phase::DurableCompanion, 5, 20, 5),
        (Phase::PostWorkIdle, 6, 20, 5),
        (Phase::Reconciliation, 7, 20, 0),
        (Phase::Terminal, 8, 20, 0),
    ] {
        writer
            .append(payload(phase, elapsed, completed, outstanding), elapsed)
            .unwrap();
    }
    let report = writer.finish().unwrap();
    assert_eq!(report.records, 8);
    assert_eq!(report.last_phase, Phase::Terminal);
    assert_eq!(report.recovered_incomplete_trailing_bytes, 0);
}

#[test]
fn rejected_progress_does_not_advance_or_append() {
    let temporary = tempfile::tempdir().unwrap();
    let mut writer =
        DurableCheckpointWriter::start(temporary.path(), payload(Phase::Startup, 1, 0, 10), 1)
            .unwrap();
    let mut drift = payload(Phase::Warmup, 2, 10, 10);
    drift.harness.start_ticks += 1;
    assert!(matches!(
        writer.append(drift, 2),
        Err(CheckpointWriterError::Progress(_))
    ));
    assert_eq!(writer.sequence(), 1);
    assert_eq!(writer.report().unwrap().records, 1);
}

#[test]
fn existing_or_torn_journal_cannot_be_reused_as_a_new_process() {
    let temporary = tempfile::tempdir().unwrap();
    let writer =
        DurableCheckpointWriter::start(temporary.path(), payload(Phase::Startup, 1, 0, 10), 1)
            .unwrap();
    assert!(matches!(
        DurableCheckpointWriter::start(temporary.path(), payload(Phase::Startup, 1, 0, 10), 1),
        Err(CheckpointWriterError::Initialization)
    ));
    let mut journal = OpenOptions::new()
        .append(true)
        .open(temporary.path().join(CHECKPOINT_JOURNAL_NAME))
        .unwrap();
    journal.write_all(b"{\"torn\":").unwrap();
    journal.sync_all().unwrap();
    assert!(writer.finish().is_err());
}
