use super::*;

fn fixture() -> SoakReport {
    SoakReport {
        schema: "imap075-soak-report-v1".into(),
        scope: "unit-classification-only".into(),
        production_enabled: false,
        profile: SoakProfile {
            schema: "imap075-soak-profile-v1".into(),
            source_sha: "a".repeat(40),
            cohort: Cohort::Reference,
            seed: 117,
            bounds: SoakBounds::smoke(),
            runtime: "unit-fixture".into(),
            artifacts: vec![],
        },
        segment: "b".repeat(32),
        parent_segment: None,
        controller: WorkerIdentity {
            pid: std::process::id(),
            platform_birth: "unit-fixture".into(),
            incarnation: "unit-fixture".into(),
        },
        worker: None,
        terminal: Terminal::Failed,
        rounds: 0,
        active_elapsed_ms: 0,
        transcript_sha256: "c".repeat(64),
        transcript_bytes: 0,
        process_reaped: false,
        remaining_members: None,
        containment: String::new(),
        detail: String::new(),
    }
}
fn exited() -> Result<process::ProcessOutput, process::ProcessFailure> {
    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;
    #[cfg(windows)]
    use std::os::windows::process::ExitStatusExt;
    Ok(process::ProcessOutput {
        stdout: vec![],
        stderr: vec![],
        status: std::process::ExitStatus::from_raw(0),
        scope: "unit-fixture-no-evidence",
        direct_child_reaped: true,
        remaining_live_members: 0,
        io_threads: 0,
    })
}
#[test]
fn process_exit_requires_complete_protocol_and_keeps_pre_admission_cancel_distinct() {
    for (protocol, expected, detail) in [
        (
            ProtocolEnd::Worker {
                line_complete: true,
                terminal: Some(Terminal::Complete),
                cancelled_before_active: false,
            },
            Terminal::Complete,
            "",
        ),
        (
            ProtocolEnd::Worker {
                line_complete: false,
                terminal: Some(Terminal::Complete),
                cancelled_before_active: false,
            },
            Terminal::Failed,
            "incomplete soak protocol line",
        ),
        (
            ProtocolEnd::Worker {
                line_complete: true,
                terminal: None,
                cancelled_before_active: true,
            },
            Terminal::Cancelled,
            "cancelled before active admission",
        ),
        (
            ProtocolEnd::Worker {
                line_complete: true,
                terminal: None,
                cancelled_before_active: false,
            },
            Terminal::Failed,
            "worker exited without drain",
        ),
        (
            ProtocolEnd::Daemon { terminal: None },
            Terminal::Failed,
            "daemon exited before drain",
        ),
        (
            ProtocolEnd::Daemon {
                terminal: Some(Terminal::Failed),
            },
            Terminal::Failed,
            "daemon exited before drain",
        ),
        (
            ProtocolEnd::Daemon {
                terminal: Some(Terminal::Complete),
            },
            Terminal::Complete,
            "",
        ),
    ] {
        let mut report = fixture();
        finalize_process(&mut report, exited(), protocol, false);
        assert_eq!(report.terminal, expected);
        assert_eq!(report.detail, detail);
        assert!(report.process_reaped);
        assert_eq!(report.remaining_members, Some(0));
        assert_eq!(report.containment, "unit-fixture-no-evidence");
        assert!(validate_report(&report).is_err());
    }
}
#[test]
fn process_failure_preserves_observed_cleanup_and_timeout_has_priority_over_cancel() {
    for (kind, cancelled, detail, expected) in [
        (
            process::FailureKind::Timeout,
            true,
            "cancellation drain deadline",
            Terminal::TimedOut,
        ),
        (
            process::FailureKind::Observation,
            true,
            "cancellation drain deadline",
            Terminal::Cancelled,
        ),
        (
            process::FailureKind::Observation,
            false,
            "cancellation drain deadline",
            Terminal::Failed,
        ),
        (
            process::FailureKind::Observation,
            true,
            "logical history mismatch",
            Terminal::Failed,
        ),
        (
            process::FailureKind::Exit,
            false,
            "unexpected exit",
            Terminal::Failed,
        ),
    ] {
        let mut report = fixture();
        let failure = process::ProcessFailure {
            kind,
            detail: detail.into(),
            direct_child_reaped: false,
            remaining_live_members: None,
        };
        finalize_process(
            &mut report,
            Err(failure),
            ProtocolEnd::Daemon {
                terminal: Some(Terminal::Complete),
            },
            cancelled,
        );
        assert_eq!(report.terminal, expected);
        assert!(report.detail.contains(detail));
        assert!(!report.process_reaped);
        assert_eq!(report.remaining_members, None);
        assert!(validate_report(&report).is_err());
    }
}
#[test]
fn artifact_and_cleanup_failures_cannot_be_promoted_by_completed_protocol_or_timeout_drain() {
    for (terminal, expected, detail) in [
        (
            Terminal::Complete,
            Terminal::TimedOut,
            "hard deadline requested bounded drain",
        ),
        (
            Terminal::Cancelled,
            Terminal::TimedOut,
            "hard deadline requested bounded drain",
        ),
        (Terminal::Failed, Terminal::Failed, "previous failure"),
        (Terminal::TimedOut, Terminal::TimedOut, "previous failure"),
    ] {
        let mut report = fixture();
        report.terminal = terminal;
        report.detail = "previous failure".into();
        finalize_segment(&mut report, true, Ok(()), true, "cleanup failure");
        assert_eq!(report.terminal, expected);
        assert_eq!(report.detail, detail);
    }
    let mut report = fixture();
    report.terminal = Terminal::Complete;
    finalize_segment(
        &mut report,
        true,
        Err("artifact changed".into()),
        false,
        "; incomplete process cleanup",
    );
    assert_eq!(report.terminal, Terminal::Failed);
    assert_eq!(
        report.detail,
        "artifact changed; incomplete process cleanup"
    );
    report.terminal = Terminal::Complete;
    report.detail.clear();
    finalize_segment(
        &mut report,
        false,
        Ok(()),
        false,
        "; daemon cleanup/listeners incomplete",
    );
    assert_eq!(report.terminal, Terminal::Failed);
    assert_eq!(report.detail, "; daemon cleanup/listeners incomplete");
    assert!(validate_report(&report).is_err());
}
