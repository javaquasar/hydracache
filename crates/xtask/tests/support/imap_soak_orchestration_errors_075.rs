use super::*;
fn profile() -> SoakProfile {
    SoakProfile {
        schema: "imap075-soak-profile-v1".into(),
        source_sha: "a".repeat(40),
        cohort: Cohort::Reference,
        seed: 117,
        bounds: SoakBounds::smoke(),
        runtime: "unit-not-admitted".into(),
        artifacts: vec![BoundArtifact {
            path: "unit-never-executed".into(),
            sha256: "a".repeat(64),
        }],
    }
}
fn failed(p: SoakProfile) -> SoakReport {
    SoakReport {
        schema: "imap075-soak-report-v1".into(),
        scope: "continuous_local_development_segment".into(),
        production_enabled: false,
        profile: p,
        segment: "unit-failed-segment".into(),
        parent_segment: None,
        controller: WorkerIdentity {
            pid: 17,
            platform_birth: "unit-ineligible".into(),
            incarnation: "unit".into(),
        },
        worker: None,
        terminal: Terminal::Failed,
        rounds: 0,
        active_elapsed_ms: 0,
        transcript_sha256: crate::imap_qualification_075::digest(&[]),
        transcript_bytes: 0,
        process_reaped: false,
        remaining_members: None,
        containment: "unit-ineligible".into(),
        detail: "unit failed observation".into(),
    }
}

struct Commands {
    root: std::path::PathBuf,
    fail_at: usize,
    calls: usize,
}
impl SoakCommands for Commands {
    fn root(&mut self) -> SoakResult<std::path::PathBuf> {
        Ok(self.root.clone())
    }
    fn source(&mut self, _: &Path) -> SoakResult<String> {
        Ok("a".repeat(40))
    }
    fn profile(
        &mut self,
        _: &Path,
        _: &str,
        cohort: Cohort,
        bounds: SoakBounds,
    ) -> SoakResult<SoakProfile> {
        let mut p = profile();
        p.cohort = cohort;
        p.bounds = bounds;
        Ok(p)
    }
    fn execute(
        &mut self,
        _: &Path,
        p: SoakProfile,
        _: &Path,
        _: Option<String>,
    ) -> SoakResult<SoakReport> {
        self.calls += 1;
        if self.fail_at == 1 {
            Err("unit execution refused".into())
        } else {
            Ok(failed(p))
        }
    }
    fn completed(&mut self, report: &SoakReport, context: &str) -> SoakResult<()> {
        self.calls += 1;
        assert_eq!(report.terminal, Terminal::Failed);
        if self.fail_at == 2 {
            require_completed_segment(&report.terminal, &report.detail, context)
        } else {
            Ok(())
        }
    }
    fn check(&mut self, _: &Path, _: &Path) -> SoakResult<SoakReport> {
        self.calls += 1;
        if self.fail_at == 3 {
            Err("unit saved replay refused".into())
        } else {
            Ok(failed(profile()))
        }
    }
    fn artifact(
        &mut self,
        root: &Path,
        path: &Path,
    ) -> SoakResult<crate::imap_extended_receipt_075::Artifact> {
        self.calls += 1;
        crate::imap_extended_receipt_075::artifact(root, path, 256 * 1024)
    }
}
#[test]
fn preparation_refusals_stop_at_execution_completion_saved_replay_and_real_missing_artifact() {
    for fail_at in 1..=4 {
        let root = tempfile::tempdir().unwrap();
        let mut commands = Commands {
            root: root.path().into(),
            fail_at,
            calls: 0,
        };
        let error = preparation_smoke_using(
            root.path(),
            &"a".repeat(40),
            &root.path().join("unit"),
            &mut commands,
        )
        .unwrap_err();
        assert_eq!(commands.calls, fail_at, "{error}");
        assert!(!root.path().join("unit/soak-reference/report.json").exists());
        if fail_at == 2 {
            assert!(error.to_string().contains("Failed"));
        }
        if fail_at == 4 {
            assert!(error.downcast_ref::<std::io::Error>().is_some());
        }
    }
}
#[test]
fn cli_refuses_execution_completion_and_saved_replay_without_a_successful_segment() {
    for fail_at in 1..=3 {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("target/imap-075")).unwrap();
        let mut commands = Commands {
            root: root.path().into(),
            fail_at,
            calls: 0,
        };
        let args = [
            "--release",
            "0.75",
            "--cohort",
            "reference",
            "--host-budget-seconds",
            "60",
            "--output",
            "target/imap-075/unit-refusal",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        let error = run_using(args, &mut commands).unwrap_err();
        assert_eq!(commands.calls, fail_at, "{error}");
        assert!(!root
            .path()
            .join("target/imap-075/unit-refusal/reference/report.json")
            .exists());
        if fail_at == 2 {
            assert!(error.to_string().contains("Failed"));
        }
    }
}
#[test]
fn daemon_observer_deadline_and_binding_refusals_reap_the_actual_finite_process() {
    for deadline in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let input = root.path().join("unit-input");
        fs::write(&input, b"unit binding; not executable").unwrap();
        let mut p = profile();
        p.cohort = Cohort::Daemon;
        p.runtime = current_runtime(Cohort::Daemon).unwrap();
        p.artifacts = vec![BoundArtifact {
            path: "unit-input".into(),
            sha256: crate::imap_daemon_probe_075::binary_digest(&input).unwrap(),
        }];
        let binary = if cfg!(windows) {
            std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap())
                .join("System32/where.exe")
        } else {
            "/bin/false".into()
        };
        let now = Instant::now();
        let mut clocks = 0;
        let report = execute_daemon_bound(
            root.path(),
            p,
            &binary,
            &root.path().join("control"),
            Cancellation::default(),
            None,
            DaemonBindings {
                write: write_new,
                clock: || {
                    clocks += 1;
                    if deadline && clocks == 1 {
                        now - Duration::from_secs(31)
                    } else if !deadline && clocks == 2 {
                        now - Duration::from_secs(11)
                    } else {
                        now
                    }
                },
                check: |_, _| Err("unit refresh refused".into()),
            },
        )
        .unwrap();
        assert_eq!(report.terminal, Terminal::Failed);
        assert!(report.process_reaped);
        assert_eq!(report.remaining_members, Some(0));
        assert!(!report.production_enabled);
        assert!(
            report.detail.contains(if deadline {
                "readiness deadline"
            } else {
                "unit refresh refused"
            }),
            "{}",
            report.detail
        );
        assert!(clocks >= 3);
        validate_report(&report).unwrap();
    }
}
