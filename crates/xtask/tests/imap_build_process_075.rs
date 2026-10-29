use std::io::{self, Write};
use std::process::Command;
use std::time::Duration;
use xtask::imap_build_process_075::*;
use xtask::imap_proof_process_075::{execute, FailureKind, ProcessBounds};
fn worker(mode: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_imap-proof-worker-075"));
    command.arg(mode);
    command
}
fn limits() -> BuildBounds {
    BuildBounds {
        phase_timeout: Duration::from_secs(3),
        cleanup_reserve: Duration::from_millis(20),
        stdout_bytes: 8192,
        stderr_bytes: 8192,
        heartbeat: Duration::from_millis(20),
        ..Default::default()
    }
}
fn controller(root: &std::path::Path, bounds: BuildBounds, token: Cancellation) -> BuildController {
    BuildController::with_budget(root, Duration::from_secs(5), bounds, token).unwrap()
}
#[test]
fn builds_retain_bounded_logs_phase_events_and_verified_cleanup() {
    let root = tempfile::tempdir().unwrap();
    let mut c = controller(root.path(), limits(), Cancellation::default());
    let mut command = worker("environment");
    command
        .env("IMAP_PROOF_TEST_SENTINEL", "ordinary")
        .env("JAVA_TOOL_OPTIONS", "hostile");
    let r = c.run(BuildStage::JavaContract, command).unwrap();
    assert_eq!(r.outcome, BuildOutcome::Completed);
    assert!(r.root_reaped && r.journal_complete && r.logs_complete);
    assert_eq!(r.remaining_live_members, Some(0));
    let stdout = std::fs::read_to_string(&r.stdout_path).unwrap();
    assert!(stdout.contains("JAVA_TOOL_OPTIONS=false") && stdout.contains("sentinel=ordinary"));
    assert_eq!(r.stdout_bytes, stdout.len());
    assert_eq!(
        r.stdout_sha256,
        xtask::imap_qualification_075::digest(stdout.as_bytes())
    );
    c.run(BuildStage::NativeJava, worker("echo")).unwrap();
    let raw = std::fs::read_to_string(c.directory().join("events.jsonl")).unwrap();
    let events = raw
        .lines()
        .map(|s| serde_json::from_str::<BuildEvent>(s).unwrap())
        .collect::<Vec<_>>();
    for (i, e) in events.iter().enumerate() {
        assert_eq!(e.sequence, (i + 1) as u64);
    }
    assert_eq!(events.last().unwrap().stage, BuildStage::NativeJava);
    assert_eq!(events.last().unwrap().outcome, BuildOutcome::Completed);
}
#[test]
fn failed_phase_cannot_start_following_phase_and_retains_exit_log() {
    let root = tempfile::tempdir().unwrap();
    let mut c = controller(root.path(), limits(), Cancellation::default());
    let e = c.run(BuildStage::ServerTests, worker("error")).unwrap_err();
    assert_eq!(e.report.outcome, BuildOutcome::Failed);
    assert!(e.report.root_reaped);
    assert_eq!(e.report.remaining_live_members, Some(0));
    assert!(std::fs::read_to_string(e.report.stderr_path)
        .unwrap()
        .contains("fixture_exit"));
    let next = c.run(BuildStage::JavaContract, worker("echo")).unwrap_err();
    assert_eq!(next.report.outcome, BuildOutcome::Refused);
    assert!(next.report.pid.is_none());
}
#[test]
fn controller_deadline_is_not_extended_by_heartbeats_and_output_floods_are_refused() {
    let root = tempfile::tempdir().unwrap();
    let mut b = limits();
    b.phase_timeout = Duration::from_millis(300);
    let mut c = controller(root.path(), b, Cancellation::default());
    let e = c.run(BuildStage::ServerTests, worker("sleep")).unwrap_err();
    assert_eq!(e.report.outcome, BuildOutcome::TimedOut);
    assert!(e.report.root_reaped);
    assert_eq!(e.report.remaining_live_members, Some(0));
    assert!(c.events() > 2);
    for mode in ["stdout", "stderr"] {
        let mut c = controller(root.path(), limits(), Cancellation::default());
        let e = c.run(BuildStage::JavaContract, worker(mode)).unwrap_err();
        assert_eq!(e.report.outcome, BuildOutcome::OutputLimit);
        assert!(e.report.root_reaped);
        assert!(!e.report.logs_complete);
    }
}
#[test]
fn outer_budget_reserve_refuses_before_spawn_and_proof_cap_cannot_be_bypassed() {
    let root = tempfile::tempdir().unwrap();
    let mut c = BuildController::with_budget(
        root.path(),
        Duration::from_millis(10),
        limits(),
        Cancellation::default(),
    )
    .unwrap();
    let e = c.run(BuildStage::ServerTests, worker("echo")).unwrap_err();
    assert_eq!(e.report.outcome, BuildOutcome::Refused);
    assert!(e.report.pid.is_none());
    let proof = execute(
        worker("echo"),
        &[],
        ProcessBounds {
            timeout: Duration::from_secs(61),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(proof.kind, FailureKind::InvalidBounds);
    let mut b = limits();
    b.phase_timeout = Duration::from_secs(61);
    let mut c = BuildController::with_budget(
        root.path(),
        Duration::from_secs(65),
        b,
        Cancellation::default(),
    )
    .unwrap();
    assert!(
        c.run(BuildStage::ServerTests, worker("echo"))
            .unwrap()
            .effective_timeout_ms
            > 60_000
    );
    assert!(remaining_budget_at(Some("100"), 100).is_err());
    assert!(remaining_budget_at(Some("not-a-number"), 100).is_err());
    assert!(remaining_budget_at(Some("9999999999"), 100).is_err());
    assert_eq!(
        remaining_budget_at(Some("1100"), 100).unwrap(),
        Duration::from_secs(1)
    );
    assert_eq!(
        remaining_budget_at(None, 0).unwrap(),
        Duration::from_secs(2400)
    );
}
#[test]
fn cancellation_is_monotonic_and_reaps_an_actual_running_child() {
    let root = tempfile::tempdir().unwrap();
    let token = Cancellation::default();
    token.cancel();
    let mut c = controller(root.path(), limits(), token);
    assert!(c
        .run(BuildStage::ServerTests, worker("echo"))
        .unwrap_err()
        .report
        .pid
        .is_none());
    let token = Cancellation::default();
    let mut c = controller(root.path(), limits(), token.clone());
    let trigger = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(80));
        token.cancel();
    });
    let e = c.run(BuildStage::ServerTests, worker("sleep")).unwrap_err();
    trigger.join().unwrap();
    assert_eq!(e.report.outcome, BuildOutcome::Cancelled);
    assert!(e.report.root_reaped);
    assert_eq!(e.report.remaining_live_members, Some(0));
    let token = Cancellation::default();
    let mut c = controller(root.path(), limits(), token.clone());
    let done = c.run(BuildStage::ServerTests, worker("echo")).unwrap();
    token.cancel();
    assert_eq!(done.outcome, BuildOutcome::Completed);
}
struct BrokenJournal {
    writes: usize,
}
impl Write for BrokenJournal {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.writes += 1;
        if self.writes > 1 {
            Err(io::Error::other("journal fixture refusal"))
        } else {
            Ok(bytes.len())
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[test]
fn journal_failure_terminates_running_builder_and_event_budget_is_finite() {
    let root = tempfile::tempdir().unwrap();
    let mut c = controller(root.path(), limits(), Cancellation::default());
    c.replace_journal(Box::new(BrokenJournal { writes: 0 }));
    let e = c.run(BuildStage::ServerTests, worker("sleep")).unwrap_err();
    assert_eq!(e.report.outcome, BuildOutcome::JournalFailed);
    assert!(e.report.root_reaped);
    assert_eq!(e.report.remaining_live_members, Some(0));
    assert!(!e.report.journal_complete);
    let mut b = limits();
    b.maximum_events = 2;
    b.heartbeat = Duration::from_secs(1);
    let mut c = controller(root.path(), b, Cancellation::default());
    c.run(BuildStage::ServerTests, worker("echo")).unwrap();
    assert!(c
        .run(BuildStage::JavaContract, worker("echo"))
        .unwrap_err()
        .report
        .pid
        .is_none());
}
#[test]
fn inherited_descendant_is_reaped_after_successful_builder_root_exit() {
    let root = tempfile::tempdir().unwrap();
    let mut c = controller(root.path(), limits(), Cancellation::default());
    let r = c.run(BuildStage::ServerTests, worker("hold_pipe")).unwrap();
    assert!(std::fs::read_to_string(r.stdout_path)
        .unwrap()
        .starts_with("descendant:"));
    assert_eq!(r.remaining_live_members, Some(0));
    assert!(r.root_reaped);
}
#[test]
fn aggregate_output_and_stage_work_are_bounded() {
    let root = tempfile::tempdir().unwrap();
    let mut b = limits();
    b.aggregate_output_bytes = 12;
    let mut c = controller(root.path(), b, Cancellation::default());
    let mut cmd = worker("environment");
    cmd.env("IMAP_PROOF_TEST_SENTINEL", "ordinary");
    let e = c.run(BuildStage::ServerTests, cmd).unwrap_err();
    assert_eq!(e.report.outcome, BuildOutcome::OutputLimit);
    let mut b = limits();
    b.maximum_stages = 1;
    let mut c = controller(root.path(), b, Cancellation::default());
    c.run(BuildStage::ServerTests, worker("echo")).unwrap();
    assert!(c
        .run(BuildStage::JavaContract, worker("echo"))
        .unwrap_err()
        .report
        .pid
        .is_none());
    let mut invalid = limits();
    invalid.maximum_stages = 0;
    assert!(BuildController::with_budget(
        root.path(),
        Duration::from_secs(1),
        invalid,
        Cancellation::default()
    )
    .is_err());
}
