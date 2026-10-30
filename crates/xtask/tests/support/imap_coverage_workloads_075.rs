use super::*;
use crate::imap_build_process_075::{BuildBounds, BuildOutcome, BuildReport, BuildStage};
use serde_json::json;

// Command protocol only: no compiler, executable, JAR, profile or qualified
// report is supplied by these fixtures. The production wrapper is never called.
struct Fixture {
    root: tempfile::TempDir,
    directory: PathBuf,
    control: Control,
    inventory: Artifact,
}
fn fixture() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("target/imap-075/workload-unit");
    fs::create_dir_all(directory.join("objects")).unwrap();
    fs::create_dir_all(root.path().join(RUNNER_TARGET)).unwrap();
    fs::write(
        root.path().join("Cargo.lock"),
        include_str!("../../../../Cargo.lock"),
    )
    .unwrap();
    let object = root.path().join(RUNNER_TARGET).join("unit-object");
    fs::write(&object, b"unit bytes; never an executable").unwrap();
    let bootstrap = directory.join("bootstrap.stdout");
    fs::write(
        &bootstrap,
        serde_json::to_vec(&json!({"reason":"compiler-artifact", "executable":object})).unwrap(),
    )
    .unwrap();
    let bootstrap = snapshot_compiler_record(
        root.path(),
        &directory,
        artifact(root.path(), &bootstrap, MAX).unwrap(),
    )
    .unwrap();
    let control = Control {
        schema: "imap075-coverage-control-v2".into(),
        source_sha: "0".repeat(40),
        run_id: "0".repeat(32),
        production_enabled: false,
        profile: Profile {
            schema: "imap075-coverage-profile-v2".into(),
            baseline: BASELINE.into(),
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            rust_flags: RUST_FLAGS.into(),
            cargo_config: CARGO_CONFIG.iter().map(|s| (*s).into()).collect(),
            feature: "imap-075-coverage".into(),
            target: crate::imap_sbom_075::CargoTarget {
                triple: "unit-unqualified".into(),
                cfg: Default::default(),
            },
            tools: vec![],
        },
        sources: vec![],
        producer: artifact(root.path(), &object, MAX).unwrap(),
        bootstrap,
    };
    store_new(root.path(), &directory.join("control.json"), &control).unwrap();
    fs::create_dir_all(directory.join("preparation")).unwrap();
    fs::write(
        directory.join("preparation/receipt.json"),
        b"unit bytes; no preparation receipt",
    )
    .unwrap();
    let inventory = artifact(
        root.path(),
        &directory.join("preparation/receipt.json"),
        MAX,
    )
    .unwrap();
    fs::create_dir_all(directory.join("java")).unwrap();
    fs::write(directory.join("java/coverage-sessions.json"), b"[]").unwrap();
    for path in [
        crate::imap_sbom_capture_075::SPY.to_owned(),
        format!(
            "{}/org/jacoco/org.jacoco.cli/0.8.13/org.jacoco.cli-0.8.13-nodeps.jar",
            crate::imap_sbom_capture_075::REPOSITORY
        ),
    ] {
        let path = root.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"unit bytes; not a JAR").unwrap();
    }
    Fixture {
        root,
        directory,
        control,
        inventory,
    }
}
#[derive(Default)]
struct Driver {
    calls: usize,
    fail_at: usize,
    missing_at: usize,
    lock_drift: bool,
}
impl CoverageDriver for Driver {
    fn execute(&mut self, stage: BuildStage, command: Command) -> Result<BuildReport> {
        self.calls += 1;
        let expected = match self.calls {
            1 | 13 => BuildStage::ToolingTests,
            2 => BuildStage::HelperContractTests,
            _ => BuildStage::CoverageExecution,
        };
        assert_eq!(stage, expected);
        if self.calls == self.fail_at {
            return Err("unit workload command refused".into());
        }
        let root = if self.calls == 3 || self.calls == 4 {
            command
                .get_current_dir()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf()
        } else {
            command
                .get_current_dir()
                .unwrap_or(Path::new("."))
                .to_path_buf()
        };
        // The standalone consumer invocation has no current_dir, and no output
        // is read for that command. It is never executed by this driver.
        let stdout = if command.get_program() == "cargo" {
            let workspace = command.get_current_dir().unwrap();
            if self.lock_drift && self.calls == 3 {
                fs::write(workspace.join("Cargo.lock"), b"version = 4\n")?;
            }
            let stdout = root.join(format!(
                "target/imap-075/workload-unit/command-{}.stdout",
                self.calls
            ));
            if self.calls != self.missing_at {
                fs::write(
                    &stdout,
                    serde_json::to_vec(
                        &json!({"reason":"compiler-artifact","executable":root.join(RUNNER_TARGET).join("unit-object")}),
                    )?,
                )?;
            }
            stdout
        } else {
            root.join("unused.stdout")
        };
        Ok(BuildReport {
            bounds: BuildBounds::default(),
            scope: "unit-workload-unqualified".into(),
            stage,
            outcome: BuildOutcome::Completed,
            elapsed_ms: 1,
            effective_timeout_ms: 100,
            stdout_path: stdout.to_string_lossy().into_owned(),
            stderr_path: "unused".into(),
            stdout_bytes: 0,
            stderr_bytes: 0,
            stdout_sha256: "0".repeat(64),
            stderr_sha256: "0".repeat(64),
            pid: None,
            root_reaped: true,
            remaining_live_members: Some(0),
            containment: "unit-only".into(),
            journal_complete: true,
            logs_complete: true,
        })
    }
}
fn run(f: &Fixture, driver: &mut Driver, drift: bool) -> Result<CaptureReceipt> {
    capture_workloads(
        f.root.path(),
        &relative(f.root.path(), &f.directory.join("control.json"))?,
        &f.control,
        &f.directory,
        driver,
        |_, _| {
            Ok(PreparedWorkloadInputs {
                inventory: f.inventory.clone(),
                directory: relative(f.root.path(), &f.directory.join("java"))?,
            })
        },
        |_| {
            Ok((
                if drift {
                    "1".repeat(40)
                } else {
                    f.control.source_sha.clone()
                },
                f.control.profile.clone(),
                f.control.sources.clone(),
            ))
        },
    )
}
fn unpublished(f: &Fixture) {
    assert!(!f.directory.join("capture.json").exists());
    assert!(!f.directory.join("report.json").exists());
    assert!(!f
        .root
        .path()
        .join("target/imap-075/coverage-consumer-workspace")
        .exists());
}
#[test]
fn every_workload_command_failure_stops_later_commands_and_removes_the_consumer_workspace() {
    for fail_at in 1..=14 {
        let f = fixture();
        let mut driver = Driver {
            fail_at,
            ..Default::default()
        };
        assert!(run(&f, &mut driver, false)
            .unwrap_err()
            .to_string()
            .contains("unit workload command refused"));
        assert_eq!(driver.calls, fail_at);
        unpublished(&f);
    }
}
#[test]
fn missing_compiler_outputs_and_consumer_lock_drift_never_advance_or_publish() {
    for missing_at in [1, 2, 3, 4, 13] {
        let f = fixture();
        let mut driver = Driver {
            missing_at,
            ..Default::default()
        };
        assert!(run(&f, &mut driver, false).is_err());
        assert_eq!(driver.calls, missing_at);
        unpublished(&f);
    }
    let f = fixture();
    let mut driver = Driver {
        lock_drift: true,
        ..Default::default()
    };
    assert!(run(&f, &mut driver, false).is_err());
    assert_eq!(driver.calls, 3);
    unpublished(&f);
}
#[test]
fn final_source_drift_refuses_and_modelled_completion_is_ineligible_for_real_admission() {
    for drift in [false, true] {
        let f = fixture();
        let mut driver = Driver::default();
        let result = run(&f, &mut driver, drift);
        assert_eq!(driver.calls, 14);
        if drift {
            assert!(result.unwrap_err().to_string().contains("execution"));
        } else {
            let receipt = result.unwrap();
            assert_eq!(receipt.cargo_streams.len(), 6);
            assert!(dead(&receipt.controller)
                .unwrap_err()
                .to_string()
                .contains("still lives"));
            assert!(validate_capture_materials(
                &receipt,
                &f.control,
                &receipt.objects,
                &[],
                "raw/analysis/",
                true
            )
            .is_err());
        }
        unpublished(&f);
    }
}
#[test]
fn preparation_loader_refuses_missing_and_malformed_real_files() {
    let f = fixture();
    assert!(prepared_workload_inputs(f.root.path(), &f.inventory).is_err());
    fs::remove_file(f.root.path().join(&f.inventory.path)).unwrap();
    assert!(prepared_workload_inputs(f.root.path(), &f.inventory).is_err());
    unpublished(&f);
}
#[test]
fn workload_producer_requires_exact_path_hash_and_length_before_workloads() {
    let f = fixture();
    validate_workload_producer(&f.control.producer, &f.control.producer).unwrap();
    for field in 0..3 {
        let mut changed = f.control.producer.clone();
        match field {
            0 => changed.path.push('x'),
            1 => changed.sha256 = "1".repeat(64),
            _ => changed.bytes += 1,
        }
        assert!(validate_workload_producer(&changed, &f.control.producer).is_err());
    }
    unpublished(&f);
}
