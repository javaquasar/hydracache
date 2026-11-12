use super::*;
fn fixture(cohort: Cohort) -> (tempfile::TempDir, SoakProfile) {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("unit-input");
    fs::write(&file, b"unit artifact; never an executable").unwrap();
    let profile = SoakProfile {
        schema: "imap075-soak-profile-v1".into(),
        source_sha: "a".repeat(40),
        cohort,
        seed: 117,
        bounds: SoakBounds::smoke(),
        runtime: current_runtime(cohort).unwrap(),
        artifacts: vec![BoundArtifact {
            path: "unit-input".into(),
            sha256: crate::imap_daemon_probe_075::binary_digest(&file).unwrap(),
        }],
    };
    (root, profile)
}
fn finite_failure() -> Command {
    if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/d", "/c", "exit 7"]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", "exit 7", "--"]);
        c
    }
}
#[test]
fn worker_checkpoint_and_post_reap_report_write_errors_propagate_without_success_reports() {
    for fail_at in [1, 2] {
        let (root, profile) = fixture(Cohort::Reference);
        let control = root.path().join("control");
        let mut calls = 0;
        let error = execute_worker_using(
            root.path(),
            profile,
            finite_failure(),
            &control,
            Cancellation::default(),
            None,
            |path, bytes| {
                calls += 1;
                if calls == fail_at {
                    if calls == 2 {
                        let report: SoakReport = serde_json::from_slice(bytes).unwrap();
                        assert_eq!(report.terminal, Terminal::Failed);
                        assert!(report.process_reaped);
                        assert_eq!(report.remaining_members, Some(0));
                    }
                    return Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied).into());
                }
                write_new(path, bytes)
            },
        )
        .unwrap_err();
        assert_eq!(
            error.downcast_ref::<std::io::Error>().unwrap().kind(),
            std::io::ErrorKind::PermissionDenied
        );
        assert_eq!(calls, fail_at);
        assert!(!control.join("report.json").exists());
        if fail_at == 1 {
            assert!(!control.join("stdout.jsonl").exists());
        }
    }
}
#[test]
fn daemon_checkpoint_and_spawn_failure_report_writes_never_publish_a_completed_segment() {
    for fail_at in [1, 2] {
        let (root, profile) = fixture(Cohort::Daemon);
        let control = root.path().join("control");
        let mut calls = 0;
        let error = execute_daemon_using(
            root.path(),
            profile,
            &root.path().join("unit-input"),
            &control,
            Cancellation::default(),
            None,
            |path, bytes| {
                calls += 1;
                if calls == fail_at {
                    let report: SoakReport = serde_json::from_slice(bytes).unwrap();
                    assert_eq!(report.terminal, Terminal::Failed);
                    assert!(!report.production_enabled);
                    return Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied).into());
                }
                write_new(path, bytes)
            },
        )
        .unwrap_err();
        assert_eq!(
            error.downcast_ref::<std::io::Error>().unwrap().kind(),
            std::io::ErrorKind::PermissionDenied
        );
        assert_eq!(calls, fail_at);
        assert!(!control.join("report.json").exists());
        if fail_at == 1 {
            assert!(!control.join("daemon-stdout.log").exists());
        }
    }
}
