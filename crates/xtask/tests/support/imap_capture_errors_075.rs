use super::*;
use crate::imap_build_process_075::{BuildBounds, BuildOutcome};
struct Commands {
    directory: PathBuf,
    calls: usize,
    fail_at: usize,
    coverage: bool,
    retain_calls: usize,
}
impl CaptureCommands for Commands {
    fn directory(&self) -> &Path {
        &self.directory
    }
    fn maven(&mut self, _: &Path) -> CaptureResult<Command> {
        Ok(Command::new("unit-never-executed"))
    }
    fn coverage_enabled(&self) -> bool {
        self.coverage
    }
    fn retain(&mut self, _: &Path, _: &str, _: &str, _: &BuildReport) -> CaptureResult<()> {
        self.retain_calls += 1;
        Err("unit retain refusal".into())
    }
    fn run(&mut self, stage: BuildStage, command: Command) -> CaptureResult<BuildReport> {
        self.calls += 1;
        if self.calls == self.fail_at {
            return Err("unit build refusal".into());
        }
        let stdout = self.directory.join(format!("{}.stdout", self.calls));
        if command.get_args().any(|a| a == "--version") {
            fs::write(
                &stdout,
                format!("Maven home: {}\n", self.directory.display()),
            )?;
        } else {
            fs::write(&stdout, b"unit observations; no command executed")?;
        }
        Ok(BuildReport {
            bounds: BuildBounds::default(),
            scope: "unit-ineligible".into(),
            stage,
            outcome: BuildOutcome::Refused,
            elapsed_ms: 0,
            effective_timeout_ms: 1,
            stdout_path: stdout.to_string_lossy().into_owned(),
            stderr_path: String::new(),
            stdout_bytes: 0,
            stderr_bytes: 0,
            stdout_sha256: "0".repeat(64),
            stderr_sha256: "0".repeat(64),
            pid: None,
            root_reaped: false,
            remaining_live_members: None,
            containment: "unit".into(),
            journal_complete: false,
            logs_complete: false,
        })
    }
}
fn capture() -> Capture {
    Capture {
        source_sha: "0".repeat(40),
        stages: vec![],
        host: "unit-host".into(),
        rust_version: "unit".into(),
        rustc_sha256: "0".repeat(64),
        cargo_sha256: "0".repeat(64),
        rust_inputs: vec![],
        maven_home: String::new(),
        directory: "capture".into(),
    }
}
fn commands(root: &Path, fail_at: usize, coverage: bool) -> Commands {
    let directory = root.join("capture");
    fs::create_dir_all(&directory).unwrap();
    Commands {
        directory,
        calls: 0,
        fail_at,
        coverage,
        retain_calls: 0,
    }
}
#[test]
fn bootstrap_missing_resource_stops_before_jar_and_copy_requires_real_bytes() {
    let root = tempfile::tempdir().unwrap();
    let mut host = commands(root.path(), usize::MAX, false);
    let error = capture().bootstrap(root.path(), &mut host).unwrap_err();
    assert_eq!(
        error.downcast_ref::<std::io::Error>().unwrap().kind(),
        std::io::ErrorKind::NotFound
    );
    assert_eq!(host.calls, 2);
    assert!(!root.path().join(SPY).exists());
    let source = root
        .path()
        .join("tests/java-imap-provenance-075/src/main/resources/META-INF/plexus/components.xml");
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    fs::write(&source, b"unit-resource").unwrap();
    let destination = host.directory.join("provenance-classes/META-INF/plexus");
    copy_provenance_resource(root.path(), &destination).unwrap();
    assert_eq!(
        fs::read(destination.join("components.xml")).unwrap(),
        b"unit-resource"
    );
}
#[test]
fn finish_build_and_capture_write_refusals_never_generate_inventory() {
    for fail_at in [2, 3, 4, usize::MAX] {
        let root = tempfile::tempdir().unwrap();
        let mut host = commands(root.path(), fail_at, false);
        if fail_at == usize::MAX {
            fs::create_dir(host.directory.join("capture.json")).unwrap();
        }
        let error = capture().finish_using(root.path(), &mut host).unwrap_err();
        if fail_at != usize::MAX {
            assert_eq!(error.to_string(), "unit build refusal");
            assert_eq!(host.calls, fail_at);
        } else {
            assert!(error.downcast_ref::<std::io::Error>().is_some());
            assert_eq!(host.calls, 4);
        }
        assert_eq!(host.retain_calls, 0);
        assert!(!host.directory.join("inventory.json").exists());
        assert!(!host.directory.join("provenance.json").exists());
    }
}
#[test]
fn java_session_refusal_does_not_append_an_unretained_stage() {
    let root = tempfile::tempdir().unwrap();
    let mut host = commands(root.path(), usize::MAX, true);
    let mut capture = capture();
    let error = capture
        .maven_using(
            root.path(),
            &mut host,
            "JavaContract",
            "artifact-build",
            BuildStage::JavaContract,
            &["verify"],
        )
        .unwrap_err();
    assert_eq!(error.to_string(), "unit retain refusal");
    assert_eq!(host.calls, 1);
    assert_eq!(host.retain_calls, 1);
    assert!(capture.stages.is_empty());
    assert!(!host.directory.join("capture.json").exists());
}
