use super::*;
use crate::imap_build_process_075::{BuildBounds, BuildOutcome, BuildReport, BuildStage};
use serde_json::json;

// These bytes deliberately are not LLVM objects/profiles or JaCoCo data. The
// driver models command boundaries only; no capture or qualified report is made.
struct Fixture {
    root: tempfile::TempDir,
    directory: PathBuf,
    control: Control,
    capture: CaptureReceipt,
    raw: Vec<Artifact>,
}
fn material(root: &Path, directory: &Path, name: &str, bytes: &[u8]) -> Artifact {
    let path = directory.join(name);
    fs::write(&path, bytes).unwrap();
    artifact(root, &path, MAX).unwrap()
}
fn build(stdout_path: String) -> BuildReport {
    BuildReport {
        bounds: BuildBounds::default(),
        scope: "unit-driver-only-unqualified".into(),
        stage: BuildStage::CoverageReport,
        outcome: BuildOutcome::Completed,
        elapsed_ms: 1,
        effective_timeout_ms: 100,
        stdout_path,
        stderr_path: "unused-unit-stream".into(),
        stdout_bytes: 0,
        stderr_bytes: 0,
        stdout_sha256: "0".repeat(64),
        stderr_sha256: "0".repeat(64),
        pid: None,
        root_reaped: true,
        remaining_live_members: Some(0),
        containment: "unit-driver-only".into(),
        journal_complete: true,
        logs_complete: true,
    }
}
fn fixture() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("target/imap-075/reconstruction-unit");
    fs::create_dir_all(&directory).unwrap();
    let object = material(root.path(), &directory, "object", b"not an LLVM object");
    let mut sources = vec![Source {
        path: "crates/xtask/src/unit.rs".into(),
        sha256: "0".repeat(64),
        bytes: 10,
        added: [1].into_iter().collect(),
        has_executable: true,
    }];
    let mut java = Vec::new();
    for module in [
        "java-imap-benchmark",
        "java-imap-hazelcast-075",
        "java-imap-provenance-075",
    ] {
        let projection = serde_json::to_vec(&json!({
            "schema":"imap075-java-coverage-v1", "sessions":[{"id":module}],
            "sources":{"unit.java":{"1":0}}
        }))
        .unwrap();
        let xml = material(
            root.path(),
            &directory,
            &format!("{module}.xml"),
            &projection,
        );
        let exec = material(
            root.path(),
            &directory,
            &format!("{module}.exec"),
            &projection,
        );
        let jar = material(
            root.path(),
            &directory,
            &format!("{module}.jar"),
            b"not a JAR",
        );
        let receipt = material(
            root.path(),
            &directory,
            &format!("{module}.build.json"),
            &serde_json::to_vec(&build("unused".into())).unwrap(),
        );
        java.push(JavaSession {
            schema: "imap075-java-coverage-session-v1".into(),
            module: module.into(),
            id: module.into(),
            source_sha: "0".repeat(40),
            exec,
            xml,
            jar,
            build: receipt,
        });
        sources.push(Source {
            path: format!("tests/{module}/src/main/java/unit.java"),
            sha256: "0".repeat(64),
            bytes: 10,
            added: [1].into_iter().collect(),
            has_executable: true,
        });
    }
    let control = Control {
        schema: "imap075-coverage-control-v2".into(),
        source_sha: "0".repeat(40),
        run_id: "0".repeat(32),
        production_enabled: false,
        profile: Profile {
            schema: "imap075-coverage-profile-v2".into(),
            baseline: BASELINE.into(),
            os: "linux".into(),
            arch: "x86_64".into(),
            rust_flags: RUST_FLAGS.into(),
            cargo_config: CARGO_CONFIG.iter().map(|s| (*s).into()).collect(),
            feature: "imap-075-coverage".into(),
            target: crate::imap_sbom_075::CargoTarget {
                triple: "x86_64-unknown-linux-gnu".into(),
                cfg: ["unix".into()].into_iter().collect(),
            },
            tools: vec![],
        },
        sources,
        producer: object.clone(),
        bootstrap: CompilerRecord {
            stream: object.clone(),
            objects: vec![],
        },
    };
    let extra = material(
        root.path(),
        &directory,
        "extra-object",
        b"not an LLVM library",
    );
    let capture = CaptureReceipt {
        schema: "imap075-coverage-capture-v2".into(),
        control: object.clone(),
        controller: crate::imap_soak_075::WorkerIdentity {
            pid: 0,
            platform_birth: "unit".into(),
            incarnation: "unit".into(),
        },
        cargo_streams: vec![],
        objects: vec![object.clone(), extra],
        java,
        preparation: object,
        java_tools: vec![],
        completed: false,
    };
    let raw = vec![material(
        root.path(),
        &directory,
        "raw",
        b"not an LLVM profile",
    )];
    Fixture {
        root,
        directory,
        control,
        capture,
        raw,
    }
}
#[derive(Default)]
struct Driver {
    calls: usize,
    fail_at: usize,
    mismatch: bool,
    duplicate: bool,
    bad_json: bool,
}
impl CoverageDriver for Driver {
    fn execute(&mut self, stage: BuildStage, command: Command) -> Result<BuildReport> {
        assert_eq!(stage, BuildStage::CoverageReport);
        self.calls += 1;
        if self.calls == self.fail_at {
            return Err("unit command refusal".into());
        }
        let args = command
            .get_args()
            .map(|s| s.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let program = command.get_program().to_string_lossy();
        let output = if program == "unit-llvm-profdata" {
            assert_eq!(&args[..3], &["merge", "--sparse", "--failure-mode=any"]);
            Vec::new()
        } else if program == "unit-llvm-cov" {
            assert_eq!(args[0], "export");
            let response = fs::read_to_string(args[1].strip_prefix('@').unwrap())?;
            assert!(response.contains("--check-binary-ids"));
            let mut lcov = "SF:crates/xtask/src/unit.rs\nDA:1,0\nend_of_record\n".to_owned();
            if self.duplicate {
                lcov.push_str(
                    "SF:tests/java-imap-benchmark/src/main/java/unit.java\nDA:1,0\nend_of_record\n",
                );
            }
            lcov.into_bytes()
        } else if args
            .iter()
            .any(|s| s == "io.hydracache.imap.provenance.CoverageDataMain075")
        {
            if self.bad_json {
                b"{".to_vec()
            } else {
                fs::read(args.last().unwrap())?
            }
        } else {
            let report = args.iter().position(|s| s == "report").unwrap();
            let xml = args.iter().position(|s| s == "--xml").unwrap();
            let mut projection: Value = serde_json::from_slice(&fs::read(&args[report + 1])?)?;
            if self.mismatch {
                projection["sources"]["unit.java"]["1"] = json!(1);
            }
            fs::write(&args[xml + 1], serde_json::to_vec(&projection)?)?;
            Vec::new()
        };
        // Every report stream is a distinct retained file owned by this test.
        let directory = command
            .get_current_dir()
            .map(Path::to_path_buf)
            .unwrap_or_else(std::env::temp_dir);
        let output_file = tempfile::NamedTempFile::new_in(directory)?;
        fs::write(output_file.path(), output)?;
        let (_, path) = output_file.keep()?;
        Ok(build(path.to_string_lossy().into_owned()))
    }
}
fn reconstruct(f: &Fixture, driver: &mut Driver) -> Result<(Artifact, Vec<Artifact>, Lines)> {
    // Driver logs must be owned by the same root as the captured materials.
    // The scoped adapter below only supplies that working directory.
    struct Owned<'a> {
        root: &'a Path,
        driver: &'a mut Driver,
    }
    impl CoverageDriver for Owned<'_> {
        fn execute(&mut self, stage: BuildStage, mut command: Command) -> Result<BuildReport> {
            command.current_dir(self.root);
            self.driver.execute(stage, command)
        }
    }
    let tools = [
        ("llvm-profdata".into(), "unit-llvm-profdata".into()),
        ("llvm-cov".into(), "unit-llvm-cov".into()),
    ]
    .into_iter()
    .collect();
    let result = regenerate_with_tools(
        f.root.path(),
        &f.directory,
        &f.control,
        &f.capture,
        &f.raw,
        &mut Owned {
            root: f.root.path(),
            driver,
        },
        &tools,
    );
    assert!(!f.capture.completed);
    assert!(!f.directory.join("report.json").exists());
    assert!(!f.directory.join("capture.json").exists());
    result
}
#[test]
fn reconstruction_driver_retains_zero_counts_and_never_qualifies_a_fixture() {
    let f = fixture();
    let mut driver = Driver::default();
    let (_, java, lines) = reconstruct(&f, &mut driver).unwrap();
    assert_eq!(driver.calls, 11);
    assert_eq!(java.len(), 3);
    let summary = summarize(&f.control.sources, &lines, "linux", vec![]).unwrap();
    assert_eq!(summary.covered_added, 0);
    assert_eq!(summary.executable_added, 4);
    assert!(require_complete(&summary).is_err());
}
#[test]
fn each_reconstruction_command_failure_stops_the_remaining_pipeline() {
    for fail_at in 1..=11 {
        let f = fixture();
        let mut driver = Driver {
            fail_at,
            ..Default::default()
        };
        assert!(reconstruct(&f, &mut driver)
            .unwrap_err()
            .to_string()
            .contains("unit command refusal"));
        assert_eq!(driver.calls, fail_at);
    }
}
#[test]
fn reconstruction_refuses_bad_java_identity_incomplete_build_and_changed_raw_projection() {
    for mode in 0..10 {
        let mut f = fixture();
        let mut driver = Driver::default();
        match mode {
            0 => {
                f.capture.java.pop();
            }
            1 => f.capture.java[0].schema = "future".into(),
            2 => f.capture.java[0].source_sha = "foreign".into(),
            3 => f.capture.java[1].module = f.capture.java[0].module.clone(),
            4 => f.capture.java[1].id = f.capture.java[0].id.clone(),
            5 => {
                let mut b = build("unused".into());
                b.outcome = BuildOutcome::Failed;
                f.capture.java[0].build = material(
                    f.root.path(),
                    &f.directory,
                    "failed-build.json",
                    &serde_json::to_vec(&b).unwrap(),
                );
            }
            6 => driver.mismatch = true,
            7 => driver.duplicate = true,
            8 => driver.bad_json = true,
            _ => {
                fs::write(f.root.path().join(&f.raw[0].path), b"changed raw input").unwrap();
            }
        }
        assert!(reconstruct(&f, &mut driver).is_err(), "mode {mode}");
    }
}

#[test]
fn reconstruction_metadata_guards_refuse_substitution_without_qualifying_inputs() {
    let f = fixture();
    assert!(validate_capture_header(&f.capture).is_err());
    let mut capture = f.capture.clone();
    capture.completed = true;
    validate_capture_header(&capture).unwrap();
    capture.schema = "future".into();
    assert!(validate_capture_header(&capture).is_err());
    capture.schema = f.capture.schema.clone();
    let path = "target/imap-075/unit/capture.json";
    validate_capture_origin(path, path, &capture, &f.control, &f.control.producer, &[]).unwrap();
    for mode in 0..3 {
        let mut producer = f.control.producer.clone();
        let mut tools = vec![];
        let actual_path = if mode == 0 {
            "target/foreign/capture.json"
        } else {
            path
        };
        if mode == 1 {
            producer.sha256 = "foreign".into();
        }
        if mode == 2 {
            tools.push(producer.clone());
        }
        assert!(validate_capture_origin(
            actual_path,
            path,
            &capture,
            &f.control,
            &producer,
            &tools
        )
        .is_err());
    }
    capture.cargo_streams = vec![f.control.bootstrap.clone(); 6];
    let mut raw = f.raw.clone();
    raw[0].path = "target/imap-075/unit/raw/analysis/123.profraw".into();
    let prefix = "target/imap-075/unit/raw/analysis/";
    validate_capture_materials(&capture, &f.control, &capture.objects, &raw, prefix, true).unwrap();
    validate_capture_materials(&capture, &f.control, &capture.objects, &[], prefix, false).unwrap();
    for mode in 0..5 {
        let mut changed = capture.clone();
        let mut objects = capture.objects.clone();
        let mut raw = raw.clone();
        match mode {
            0 => {
                changed.cargo_streams.pop();
            }
            1 => changed.cargo_streams[0].stream.sha256 = "foreign".into(),
            2 => {
                objects.pop();
            }
            3 => raw.clear(),
            _ => raw[0].path = "target/imap-075/other/raw/analysis/123.profraw".into(),
        }
        assert!(
            validate_capture_materials(&changed, &f.control, &objects, &raw, prefix, true).is_err(),
            "mode {mode}"
        );
    }
    // A metadata predicate is insufficient evidence: the original incomplete
    // fixture and invalid PID still refuse, and no report has been published.
    assert!(validate_capture_header(&f.capture).is_err());
    assert!(dead(&capture.controller).is_err());
    assert!(!f.directory.join("report.json").exists());
}

#[test]
fn coverage_inputs_are_read_between_two_equal_source_observations() {
    use std::cell::{Cell, RefCell};
    let f = fixture();
    let events = RefCell::new(Vec::new());
    let input = current_inputs_using(
        f.root.path(),
        |root| {
            assert_eq!(root, f.root.path());
            events.borrow_mut().push("source");
            Ok(f.control.source_sha.clone())
        },
        || {
            events.borrow_mut().push("profile");
            Ok(f.control.profile.clone())
        },
        |_| {
            events.borrow_mut().push("snapshot");
            Ok(f.control.sources.clone())
        },
    )
    .unwrap();
    assert_eq!(
        *events.borrow(),
        vec!["source", "profile", "snapshot", "source"]
    );
    assert_eq!(
        input,
        (
            f.control.source_sha.clone(),
            f.control.profile.clone(),
            f.control.sources.clone()
        )
    );
    for fail_at in 1..=4 {
        let calls = Cell::new(0);
        let step = || -> Result<()> {
            calls.set(calls.get() + 1);
            if calls.get() == fail_at {
                Err("unit coverage input unavailable".into())
            } else {
                Ok(())
            }
        };
        assert!(current_inputs_using(
            f.root.path(),
            |_| {
                step()?;
                Ok(f.control.source_sha.clone())
            },
            || {
                step()?;
                Ok(f.control.profile.clone())
            },
            |_| {
                step()?;
                Ok(f.control.sources.clone())
            }
        )
        .unwrap_err()
        .to_string()
        .contains("unit coverage input unavailable"));
        assert_eq!(calls.get(), fail_at);
    }
    let source_reads = Cell::new(0);
    assert!(current_inputs_using(
        f.root.path(),
        |_| {
            source_reads.set(source_reads.get() + 1);
            Ok(if source_reads.get() == 1 {
                f.control.source_sha.clone()
            } else {
                "changed".into()
            })
        },
        || Ok(f.control.profile.clone()),
        |_| Ok(f.control.sources.clone())
    )
    .unwrap_err()
    .to_string()
    .contains("source changed while reading"));
}

#[test]
fn final_capture_and_analysis_inputs_must_match_every_bound_component() {
    let f = fixture();
    for phase in ["execution", "analysis"] {
        require_current_inputs(f.root.path(), &f.control, phase, |root| {
            assert_eq!(root, f.root.path());
            Ok((
                f.control.source_sha.clone(),
                f.control.profile.clone(),
                f.control.sources.clone(),
            ))
        })
        .unwrap();
        for mode in 0..3 {
            let mut source = f.control.source_sha.clone();
            let mut profile = f.control.profile.clone();
            let mut sources = f.control.sources.clone();
            match mode {
                0 => source = "changed".into(),
                1 => profile.os = "foreign".into(),
                _ => sources.clear(),
            }
            assert_eq!(
                require_current_inputs(f.root.path(), &f.control, phase, |_| Ok((
                    source, profile, sources
                )))
                .unwrap_err()
                .to_string(),
                format!("coverage inputs changed during {phase}")
            );
        }
        assert_eq!(
            require_current_inputs(f.root.path(), &f.control, phase, |_| Err(
                "unit input failure".into()
            ))
            .unwrap_err()
            .to_string(),
            "unit input failure"
        );
    }
    assert!(!f.capture.completed);
    assert!(!f.directory.join("report.json").exists());
}

#[test]
fn owned_control_loading_refuses_aliases_substituted_producers_and_missing_inputs() {
    let mut f = fixture();
    let directory = f.directory.join("controller");
    fs::create_dir(&directory).unwrap();
    f.control.producer = material(
        f.root.path(),
        &directory,
        &suffix("xtask"),
        b"unit producer bytes; never executable",
    );
    let path = f.directory.join("control.json");
    fs::write(&path, serde_json::to_vec(&f.control).unwrap()).unwrap();
    let relative = relative(f.root.path(), &path).unwrap();
    let inputs = |_: &Path| {
        Ok((
            f.control.source_sha.clone(),
            f.control.profile.clone(),
            f.control.sources.clone(),
        ))
    };
    let (_, owned) = read_control_using(f.root.path(), &relative, inputs).unwrap();
    assert_eq!(owned, f.directory);
    for path in [
        "foreign/control.json",
        "target/imap-075/unit/alias.json",
        "/absolute/control.json",
    ] {
        assert!(read_control_using(f.root.path(), path, |_| panic!(
            "invalid path must not load inputs"
        ))
        .unwrap_err()
        .to_string()
        .contains("fixed owned coverage control path"));
    }
    assert!(read_control_using(f.root.path(), &relative, |_| Err(
        "unit inputs missing".into()
    ))
    .unwrap_err()
    .to_string()
    .contains("unit inputs missing"));
    let mut changed = f.control.clone();
    changed.producer = f.capture.objects[0].clone();
    fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(read_control_using(f.root.path(), &relative, inputs)
        .unwrap_err()
        .to_string()
        .contains("foreign coverage controller"));
    changed = f.control.clone();
    changed.producer.sha256 = "1".repeat(64);
    fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(read_control_using(f.root.path(), &relative, inputs).is_err());
    fs::write(&path, b"{").unwrap();
    assert!(read_control_using(f.root.path(), &relative, |_| panic!(
        "invalid JSON must not load tools"
    ))
    .is_err());
    assert!(!f.capture.completed);
    assert!(!f.directory.join("capture.json").exists());
    assert!(!f.directory.join("report.json").exists());
}
