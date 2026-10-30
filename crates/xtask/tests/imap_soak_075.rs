use std::{
    path::Path,
    process::Command,
    sync::OnceLock,
    time::{Duration, Instant},
};
use xtask::{imap_build_process_075::Cancellation, imap_soak_075::*};
#[test]
fn long_cli_profiles_are_validated_before_any_worker_can_start() {
    for selected in ["hour", "six-hours", "day"] {
        let arguments = [
            "--release",
            "0.75",
            "--cohort",
            "all",
            "--profile",
            selected,
            "--host-budget-seconds",
            "100000",
            "--output",
            "target/imap-075/must-not-start",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        assert!(run(arguments)
            .unwrap_err()
            .to_string()
            .contains("one explicit cohort"));
    }
    for arguments in [
        vec!["--release"],
        vec!["--release", "0.74"],
        vec!["--release", "0.75", "--cohort", "foreign"],
        vec!["--release", "0.75", "--profile", "forever"],
        vec!["--release", "0.75", "--release", "0.75"],
        vec!["--release", "0.75", "--foreign", "value"],
        vec![
            "--release",
            "0.75",
            "--host-budget-seconds",
            "18446744073709551615",
        ],
    ] {
        assert!(run(arguments.into_iter().map(String::from).collect()).is_err());
    }
}
fn runtime() -> String {
    static RUNTIME: OnceLock<String> = OnceLock::new();
    RUNTIME
        .get_or_init(|| current_runtime(Cohort::Reference).unwrap())
        .clone()
}
fn profile() -> SoakProfile {
    SoakProfile {
        schema: "imap075-soak-profile-v1".into(),
        source_sha: "a".repeat(40),
        cohort: Cohort::Reference,
        seed: 117,
        bounds: SoakBounds {
            active_ms: 120,
            period_ms: 10,
            maximum_rounds: 32,
            warmup_rounds: 2,
            startup_ms: 10000,
            drain_ms: 100,
            hard_ms: 10220,
            host_remaining_ms: 20000,
        },
        runtime: runtime(),
        artifacts: vec![BoundArtifact {
            path: "worker".into(),
            sha256: "a".repeat(64),
        }],
    }
}
fn setup() -> (tempfile::TempDir, SoakProfile) {
    let root = tempfile::tempdir().unwrap();
    std::fs::copy(
        env!("CARGO_BIN_EXE_imap-soak-worker-075"),
        root.path().join("worker"),
    )
    .unwrap();
    let mut p = profile();
    p.artifacts[0].sha256 =
        xtask::imap_daemon_probe_075::binary_digest(&root.path().join("worker")).unwrap();
    (root, p)
}
fn worker() -> Command {
    Command::new(env!("CARGO_BIN_EXE_imap-soak-worker-075"))
}
fn frame(p: &SoakProfile, phase: Phase, round: u64) -> Frame {
    let ready = phase == Phase::Ready;
    Frame {
        schema: "imap075-soak-frame-v1".into(),
        phase,
        pid: 17,
        incarnation: "incarnation-a".into(),
        cohort: p.cohort,
        seed: p.seed,
        source_sha: p.source_sha.clone(),
        round,
        active_elapsed_ms: round * 10,
        digest: if ready {
            None
        } else {
            Some(expected_digest(p, round))
        },
        owners: Owners {
            unit: "reference_owner_bytes".into(),
            values: Some(if ready { 0 } else { 73 }),
            outcomes: Some(if ready { 0 } else { 28 }),
            pending: Some(0),
            listeners: Some(0),
            streams: Some(0),
            resync: Some(0),
            queued: Some(0),
            correlations: Some(0),
        },
        heap_bytes: None,
        rss_bytes: None,
        gc_collections: None,
        cpu_ns: None,
        threads: None,
        reason: None,
    }
}
fn active(p: &SoakProfile) -> SegmentState {
    let mut s = SegmentState::new(p.clone()).unwrap();
    for phase in [Phase::Ready, Phase::Warmup, Phase::Active] {
        s.accept(&frame(p, phase, 0), 17, "birth-a").unwrap();
    }
    s
}
#[test]
fn every_invalid_phase_cut_digest_and_owner_scope_ends_the_segment() {
    let p = profile();
    for (before, phase, expected) in [
        (vec![], Phase::Ready, "readiness"),
        (vec![Phase::Ready], Phase::Warmup, "warmup"),
        (
            vec![Phase::Ready, Phase::Warmup],
            Phase::Active,
            "active cut",
        ),
    ] {
        let mut state = SegmentState::new(p.clone()).unwrap();
        for previous in before {
            state.accept(&frame(&p, previous, 0), 17, "birth").unwrap();
        }
        assert!(state
            .accept(&frame(&p, phase, 1), 17, "birth")
            .unwrap_err()
            .contains(expected));
        assert_eq!(state.terminal, Some(Terminal::Failed));
    }
    let mut state = SegmentState::new(p.clone()).unwrap();
    assert!(state
        .accept(&frame(&p, Phase::Round, 1), 17, "birth")
        .unwrap_err()
        .contains("phase transition"));
    for mode in 0..3 {
        let mut state = active(&p);
        let mut invalid = frame(&p, Phase::Round, 1);
        let expected = match mode {
            0 => {
                invalid.digest = None;
                "logical digest"
            }
            1 => {
                invalid.reason = Some("cancelled".into());
                "premature"
            }
            _ => {
                invalid.phase = Phase::Drained;
                "changed logical state"
            }
        };
        assert!(state
            .accept(&invalid, 17, "birth-a")
            .unwrap_err()
            .contains(expected));
    }
    for cohort in [Cohort::Native, Cohort::Daemon] {
        let mut p = p.clone();
        p.cohort = cohort;
        let mut state = SegmentState::new(p.clone()).unwrap();
        let mut ready = frame(&p, Phase::Ready, 0);
        ready.owners.unit = if cohort == Cohort::Native {
            "native_fixture_counts"
        } else {
            "daemon_protocol_only"
        }
        .into();
        assert!(state.accept(&ready, 17, "birth").is_err());
    }
}
#[test]
fn bounds_and_logical_phase_protocol_do_not_stitch_workers_or_accept_leaks() {
    let p = profile();
    let mut s = active(&p);
    s.accept(&frame(&p, Phase::Round, 1), 17, "birth-a")
        .unwrap();
    let mut second = frame(&p, Phase::Round, 2);
    second.incarnation = "replacement".into();
    assert!(s.accept(&second, 17, "birth-a").is_err());
    for mode in 0..12 {
        let mut s = active(&p);
        let mut f = frame(&p, Phase::Round, 1);
        let mut pid = 17;
        let mut birth = "birth-a";
        match mode {
            0 => f.seed += 1,
            1 => f.source_sha = "b".repeat(40),
            2 => f.cohort = Cohort::Native,
            3 => f.round = 2,
            4 => f.digest = Some("b".repeat(64)),
            5 => f.owners.resync = Some(1),
            6 => f.owners.values = Some(100),
            7 => f.owners.outcomes = Some(29),
            8 => f.active_elapsed_ms = p.bounds.hard_ms + 1,
            9 => f.schema = "future".into(),
            10 => pid = 18,
            _ => birth = "birth-b",
        }
        assert!(s.accept(&f, pid, birth).is_err(), "{mode}");
    }
    let mut s = active(&p);
    for n in 1..=2 {
        s.accept(&frame(&p, Phase::Round, n), 17, "birth-a")
            .unwrap();
    }
    let mut done = frame(&p, Phase::Drained, 2);
    done.active_elapsed_ms = p.bounds.active_ms;
    done.reason = Some("duration_complete".into());
    s.accept(&done, 17, "birth-a").unwrap();
    assert_eq!(s.terminal, Some(Terminal::Complete));
    assert!(s.accept(&done, 17, "birth-a").is_err());
    let mut s = active(&p);
    let mut early = frame(&p, Phase::Drained, 0);
    early.reason = Some("duration_complete".into());
    assert!(s.accept(&early, 17, "birth-a").is_err());
    let mut s = active(&p);
    early.reason = Some("cancelled".into());
    s.accept(&early, 17, "birth-a").unwrap();
    assert_eq!(s.terminal, Some(Terminal::Cancelled));
}
#[test]
fn malformed_budgets_profiles_paths_and_markers_refuse_before_unbounded_work() {
    for mode in 0..12 {
        let mut p = profile();
        match mode {
            0 => p.bounds.period_ms = 0,
            1 => p.bounds.active_ms = u64::MAX,
            2 => p.bounds.maximum_rounds = 2,
            3 => p.bounds.warmup_rounds = 0,
            4 => p.bounds.startup_ms = 0,
            5 => p.bounds.drain_ms = 0,
            6 => p.bounds.hard_ms = 1,
            7 => p.bounds.host_remaining_ms = 1,
            8 => p.schema = "future".into(),
            9 => p.source_sha = "bad".into(),
            10 => p.runtime.clear(),
            _ => p.artifacts.clear(),
        }
        assert!(p.validate().is_err(), "{mode}");
    }
    for path in ["../foreign", "C:/foreign", "/absolute", "x\\y", ""] {
        let mut p = profile();
        p.artifacts[0].path = path.into();
        assert!(p.validate().is_err(), "{path}");
    }
    let root = tempfile::tempdir().unwrap();
    assert!(!marker(root.path(), "cancel").unwrap());
    std::fs::write(root.path().join("cancel"), b"wrong").unwrap();
    assert!(marker(root.path(), "cancel").is_err());
    std::fs::write(root.path().join("cancel"), b"cancel-v1").unwrap();
    assert!(marker(root.path(), "cancel").unwrap());
    assert!(linux_start("bad").is_err());
    assert!(linux_start(&"x".repeat(8193)).is_err());
    let stat = format!(
        "17 (name with ) parentheses) S {} 321 0",
        vec!["0"; 18].join(" ")
    );
    assert_eq!(linux_start(&stat).unwrap(), 321);
    assert!(!process_identity(std::process::id()).unwrap().is_empty());
    assert!(process_identity(u32::MAX).is_err());
    assert!(xtask::imap_soak_worker_075::WorkerConfig::parse(&[]).is_err());
    for args in [
        vec!["--unknown".into(), "x".into()],
        vec!["--release".into(), "0.74".into()],
        vec!["--release".into()],
    ] {
        assert!(run(args).is_err());
    }
}
#[test]
fn real_reference_worker_survives_multiple_rounds_and_owns_no_temporary_credits() {
    let (root, p) = setup();
    let report = execute_worker(
        root.path(),
        p.clone(),
        worker(),
        &root.path().join("segment"),
        Cancellation::default(),
        None,
    )
    .unwrap();
    assert_eq!(report.terminal, Terminal::Complete, "{}", report.detail);
    assert!(report.rounds >= 2);
    assert!(report.process_reaped);
    assert_eq!(report.remaining_members, Some(0));
    validate_report(&report).unwrap();
    let frames = std::fs::read_to_string(root.path().join("segment/stdout.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Frame>(line).unwrap())
        .collect::<Vec<_>>();
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/testing/imap/0.75/schemas/soak.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    validator
        .validate(&serde_json::to_value(&report).unwrap())
        .unwrap();
    validator
        .validate(&serde_json::to_value(&p).unwrap())
        .unwrap();
    for frame in &frames {
        validator
            .validate(&serde_json::to_value(frame).unwrap())
            .unwrap();
    }
    validator
        .validate(
            &serde_json::from_slice::<serde_json::Value>(
                &std::fs::read(root.path().join("segment/identity.json")).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
    let mut missing = serde_json::to_value(&frames[0]).unwrap();
    missing["owners"].as_object_mut().unwrap().remove("resync");
    assert!(serde_json::from_value::<Frame>(missing).is_err());
    assert!(frames
        .iter()
        .all(|f| f.pid == report.worker.as_ref().unwrap().pid
            && f.incarnation == report.worker.as_ref().unwrap().incarnation));
    assert!(frames
        .iter()
        .filter(|f| f.phase == Phase::Round)
        .all(|f| f.owners.resync == Some(0)
            && f.owners.streams == Some(0)
            && f.owners.listeners == Some(0)));
    let parent = resume_parent(&report, &p).unwrap();
    let next = execute_worker(
        root.path(),
        p,
        worker(),
        &root.path().join("next"),
        Cancellation::default(),
        Some(parent),
    )
    .unwrap();
    assert_ne!(report.segment, next.segment);
    assert_ne!(report.worker, next.worker);
    assert_eq!(next.parent_segment, Some(report.segment));
}
#[test]
fn physical_defects_leaks_replacement_exit_and_output_never_qualify() {
    for fault in [
        "leak",
        "replacement",
        "exit",
        "malformed",
        "flood",
        "profile",
    ] {
        let (root, p) = setup();
        let mut command = worker();
        command.arg(format!("--canary={fault}"));
        let report = execute_worker(
            root.path(),
            p,
            command,
            &root.path().join("segment"),
            Cancellation::default(),
            None,
        )
        .unwrap();
        assert_eq!(
            report.terminal,
            Terminal::Failed,
            "{fault}: {}",
            report.detail
        );
        assert!(report.process_reaped, "{fault}");
        assert_eq!(report.remaining_members, Some(0));
    }
    let (root, p) = setup();
    let mut command = worker();
    command.arg("--canary=hang");
    let report = execute_worker(
        root.path(),
        p,
        command,
        &root.path().join("segment"),
        Cancellation::default(),
        None,
    )
    .unwrap();
    assert_eq!(report.terminal, Terminal::TimedOut, "{}", report.detail);
    assert_eq!(report.remaining_members, Some(0));
}
fn wait_round(path: &Path) {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(30) {
        if std::fs::read_to_string(path).is_ok_and(|text| text.contains("\"phase\":\"round\"")) {
            return;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("worker never reached a logical round");
}
#[test]
fn cancellation_drains_and_artifact_mutation_without_source_change_ends_the_segment() {
    let (root, p) = setup();
    let cancel = Cancellation::default();
    let token = cancel.clone();
    let progress = root.path().join("segment/stdout.jsonl");
    let thread = std::thread::spawn(move || {
        wait_round(&progress);
        token.cancel();
    });
    let report = execute_worker(
        root.path(),
        p,
        worker(),
        &root.path().join("segment"),
        cancel,
        None,
    )
    .unwrap();
    thread.join().unwrap();
    assert_eq!(report.terminal, Terminal::Cancelled, "{}", report.detail);
    assert!(report.process_reaped);
    let (root, mut p) = setup();
    std::fs::write(root.path().join("binding"), b"original").unwrap();
    p.artifacts.push(BoundArtifact {
        path: "binding".into(),
        sha256: xtask::imap_daemon_probe_075::binary_digest(&root.path().join("binding")).unwrap(),
    });
    let progress = root.path().join("segment/stdout.jsonl");
    let binding = root.path().join("binding");
    let thread = std::thread::spawn(move || {
        wait_round(&progress);
        std::fs::write(binding, b"mutated").unwrap();
    });
    let report = execute_worker(
        root.path(),
        p,
        worker(),
        &root.path().join("segment"),
        Cancellation::default(),
        None,
    )
    .unwrap();
    thread.join().unwrap();
    assert_eq!(report.terminal, Terminal::Failed);
    assert!(report.detail.contains("artifact changed"));
    assert_eq!(report.profile.source_sha, "a".repeat(40));
}
#[test]
fn restart_context_corruption_runtime_mismatch_and_false_completion_refuse() {
    let (root, p) = setup();
    let report = execute_worker(
        root.path(),
        p.clone(),
        worker(),
        &root.path().join("segment"),
        Cancellation::default(),
        None,
    )
    .unwrap();
    assert_eq!(report.terminal, Terminal::Complete, "{}", report.detail);
    for mode in 0..4 {
        let mut changed = p.clone();
        match mode {
            0 => changed.source_sha = "b".repeat(40),
            1 => changed.runtime = "foreign".into(),
            2 => changed.cohort = Cohort::Daemon,
            _ => changed.artifacts[0].sha256 = "b".repeat(64),
        }
        assert!(resume_parent(&report, &changed).is_err());
    }
    let mut live = report.clone();
    live.terminal = Terminal::Failed;
    live.process_reaped = false;
    live.worker = Some(WorkerIdentity {
        pid: std::process::id(),
        platform_birth: process_identity(std::process::id()).unwrap(),
        incarnation: "live".into(),
    });
    assert!(resume_parent(&live, &p).is_err());
    live.worker = None;
    assert!(resume_parent(&live, &p)
        .unwrap_err()
        .to_string()
        .contains("cleanup/identity unknown"));
    live.worker = Some(WorkerIdentity {
        pid: u32::MAX,
        platform_birth: "absent-process".into(),
        incarnation: "dead".into(),
    });
    assert_eq!(resume_parent(&live, &p).unwrap(), report.segment);
    let mut incomplete = report.clone();
    incomplete.rounds = 0;
    assert!(validate_report(&incomplete).is_err());
    let mut incomplete = report.clone();
    incomplete.process_reaped = false;
    assert!(validate_report(&incomplete).is_err());
    let mut bytes = serde_json::to_value(report).unwrap();
    bytes["unknown"] = serde_json::json!(1);
    assert!(serde_json::from_value::<SoakReport>(bytes).is_err());
    assert!(check_saved(root.path(), Path::new("missing")).is_err());
    let mut wrong = p;
    wrong.runtime = "foreign".into();
    assert!(execute_worker(
        root.path(),
        wrong,
        worker(),
        &root.path().join("wrong"),
        Cancellation::default(),
        None
    )
    .is_err());
}

#[test]
#[ignore = "requires imap-pre074-tools packaged Java and current daemon artifacts"]
fn actual_prepared_native_and_daemon_segments_preserve_workers_and_verify_rounds() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let scratch = tempfile::tempdir_in(root.join("target/imap-075")).unwrap();
    for cohort in [Cohort::Native, Cohort::Daemon] {
        let p = trusted_profile(&root, &"a".repeat(40), cohort, SoakBounds::smoke()).unwrap();
        let report = run_prepared(
            &root,
            p,
            &scratch.path().join(format!("{cohort:?}")),
            Cancellation::default(),
            None,
        )
        .unwrap();
        assert_eq!(
            report.terminal,
            Terminal::Complete,
            "{cohort:?}: {}",
            report.detail
        );
        assert!(report.rounds >= 2);
        assert!(report.process_reaped);
        assert_eq!(report.remaining_members, Some(0));
        validate_report(&report).unwrap();
    }
}
#[test]
#[ignore = "requires current-source public smoke outputs; all mutations stay in an owned fixture"]
fn actual_saved_soak_and_restart_reject_substituted_and_partial_archives() {
    use serde_json::{json, Value};
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let directory =
        std::env::var("IMAP075_SOAK_FIXTURE").expect("explicit actual smoke fixture required");
    assert!(directory.starts_with("target/imap-075/"));
    let directory = root.join(directory);
    assert!(check_saved(root, &directory.join("report.json"))
        .unwrap_err()
        .to_string()
        .contains("producer/artifact profile differs"));
    let mut original: SoakReport =
        serde_json::from_slice(&std::fs::read(directory.join("report.json")).unwrap()).unwrap();
    // The real CLI already replays the original archive with its original producer.
    // This distinct test binary owns separate unqualified fixtures, just as the
    // provenance mutation test does; it never overwrites the original reports.
    original.profile.artifacts = trusted_profile(
        root,
        &original.profile.source_sha,
        original.profile.cohort,
        original.profile.bounds.clone(),
    )
    .unwrap()
    .artifacts;
    let transcript = std::fs::read(directory.join("stdout.jsonl")).unwrap();
    let mut checkpoint: Value =
        serde_json::from_slice(&std::fs::read(directory.join("checkpoint.json")).unwrap()).unwrap();
    checkpoint["profile"]["artifacts"] = serde_json::to_value(&original.profile.artifacts).unwrap();
    let checkpoint = serde_json::to_vec(&checkpoint).unwrap();
    let identity = std::fs::read(directory.join("identity.json")).unwrap();
    let scratch = tempfile::tempdir_in(root.join("target/imap-075")).unwrap();
    let report_path = scratch.path().join("report.json");
    std::fs::write(&report_path, serde_json::to_vec(&original).unwrap()).unwrap();
    std::fs::write(scratch.path().join("stdout.jsonl"), &transcript).unwrap();
    std::fs::write(scratch.path().join("checkpoint.json"), &checkpoint).unwrap();
    std::fs::write(scratch.path().join("identity.json"), &identity).unwrap();
    check_saved(root, &report_path).unwrap();
    check_checkpoint(root, &scratch.path().join("checkpoint.json")).unwrap();
    for mode in 0..6 {
        let mut report = original.clone();
        let mut bytes = transcript.clone();
        let expected = match mode {
            0 => {
                report.profile.source_sha = "0".repeat(40);
                "archived source differs"
            }
            1 => {
                report.profile.seed += 1;
                "producer/artifact profile differs"
            }
            2 => {
                report.transcript_sha256 = "0".repeat(64);
                "transcript binding differs"
            }
            3 => {
                bytes = vec![b' '; 4097];
                bytes.push(b'\n');
                "saved soak frame bound"
            }
            4 => {
                let mut rows = transcript
                    .split(|b| *b == b'\n')
                    .filter(|row| !row.is_empty())
                    .collect::<Vec<_>>();
                rows.pop();
                bytes = rows
                    .into_iter()
                    .flat_map(|row| row.iter().copied().chain([b'\n']))
                    .collect();
                "saved soak progress incomplete"
            }
            _ => {
                bytes = b"not-json\n".to_vec();
                "expected"
            }
        };
        if mode >= 3 {
            report.transcript_bytes = bytes.len() as u64;
            report.transcript_sha256 = xtask::imap_qualification_075::digest(&bytes);
        }
        std::fs::write(&report_path, serde_json::to_vec(&report).unwrap()).unwrap();
        std::fs::write(scratch.path().join("stdout.jsonl"), &bytes).unwrap();
        let error = check_saved(root, &report_path).unwrap_err().to_string();
        assert!(error.contains(expected), "saved {mode}: {error}");
    }
    let checkpoint_path = scratch.path().join("checkpoint.json");
    for mode in 0..8 {
        let mut report: Value = serde_json::from_slice(&checkpoint).unwrap();
        let mut identity: Value = serde_json::from_slice(&identity).unwrap();
        let mut bytes = transcript.clone();
        let expected = match mode {
            0 => {
                report["profile"]["source_sha"] = json!("0".repeat(40));
                "checkpoint source differs"
            }
            1 => {
                report["controller"]["pid"] = json!(std::process::id());
                report["controller"]["platform_birth"] =
                    json!(process_identity(std::process::id()).unwrap());
                "previous controller still lives"
            }
            2 => {
                report["profile"]["seed"] = json!(original.profile.seed + 1);
                "checkpoint runtime/artifact/profile differs"
            }
            3 => {
                identity["segment"] = json!("foreign");
                "checkpoint identity differs"
            }
            4 => {
                identity["worker"]["pid"] = json!(std::process::id());
                identity["worker"]["platform_birth"] =
                    json!(process_identity(std::process::id()).unwrap());
                "previous worker still lives"
            }
            5 => {
                bytes.pop();
                "corrupt/partial"
            }
            6 => {
                bytes = vec![b' '; 4097];
                bytes.push(b'\n');
                "checkpoint frame bound"
            }
            _ => {
                bytes = b"not-json\n".to_vec();
                "expected"
            }
        };
        std::fs::write(&checkpoint_path, serde_json::to_vec(&report).unwrap()).unwrap();
        std::fs::write(
            scratch.path().join("identity.json"),
            serde_json::to_vec(&identity).unwrap(),
        )
        .unwrap();
        std::fs::write(scratch.path().join("stdout.jsonl"), &bytes).unwrap();
        let error = check_checkpoint(root, &checkpoint_path)
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "checkpoint {mode}: {error}");
    }
    std::fs::write(&checkpoint_path, checkpoint).unwrap();
    assert!(request_cancel(root, &checkpoint_path)
        .unwrap_err()
        .to_string()
        .contains("controller no longer lives"));
    assert!(check_checkpoint(root, &report_path)
        .unwrap_err()
        .to_string()
        .contains("checkpoint name required"));
}

#[test]
fn manual_workflow_has_closed_trusted_source_inputs_and_no_automatic_transfer() {
    use serde_yaml::Value;
    let original = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows/imap-soak-075.yml"),
    )
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join(".github/workflows")).unwrap();
    let file = root.path().join(".github/workflows/imap-soak-075.yml");
    std::fs::write(&file, &original).unwrap();
    check_manual_workflow(root.path()).unwrap();
    for mode in 0..12 {
        let mut value: Value = serde_yaml::from_str(&original).unwrap();
        match mode {
            0 => value["on"]["pull_request"] = Value::Null,
            1 => value["permissions"]["contents"] = Value::String("write".into()),
            2 => value["jobs"]["hour"]["runs-on"] = serde_yaml::to_value(["ubuntu-24.04"]).unwrap(),
            3 => value["jobs"]["hour"]["if"] = Value::String("true".into()),
            4 => value["jobs"]["hour"]["timeout-minutes"] = serde_yaml::to_value(1560).unwrap(),
            5 => {
                value["on"]["workflow_dispatch"]["inputs"]["cohort"]["options"] =
                    serde_yaml::to_value(["all"]).unwrap()
            }
            6 => {
                value["jobs"]["hour"]["steps"][1]["with"]["fetch-depth"] =
                    serde_yaml::to_value(1).unwrap()
            }
            7 => {
                value["jobs"]["hour"]["steps"][1]["uses"] =
                    Value::String("actions/checkout@main".into())
            }
            8 => {
                value["jobs"]["hour"]["steps"][4]["continue-on-error"] =
                    serde_yaml::to_value(true).unwrap()
            }
            9 => value["jobs"]["hour"]["steps"][5]["run"] = Value::String("echo pass".into()),
            10 => value["jobs"]["hour"]["steps"]
                .as_sequence_mut()
                .unwrap()
                .push(serde_yaml::from_str("uses: actions/upload-artifact@main").unwrap()),
            _ => value["concurrency"]["cancel-in-progress"] = serde_yaml::to_value(true).unwrap(),
        }
        std::fs::write(&file, serde_yaml::to_string(&value).unwrap()).unwrap();
        assert!(check_manual_workflow(root.path()).is_err(), "{mode}");
    }
}
#[test]
fn operator_marker_and_hung_drain_both_have_bounded_physical_cleanup() {
    let (root, p) = setup();
    let control = root.path().join("target/imap-075/operator");
    std::fs::create_dir_all(control.parent().unwrap()).unwrap();
    let observer_root = root.path().to_path_buf();
    let checkpoint = control.join("checkpoint.json");
    let progress = control.join("stdout.jsonl");
    let thread = std::thread::spawn(move || {
        wait_round(&progress);
        request_cancel(&observer_root, &checkpoint).unwrap();
        request_cancel(&observer_root, &checkpoint).unwrap();
    });
    let report = execute_worker(
        root.path(),
        p,
        worker(),
        &control,
        Cancellation::default(),
        None,
    )
    .unwrap();
    thread.join().unwrap();
    assert_eq!(report.terminal, Terminal::Cancelled, "{}", report.detail);
    assert!(report.process_reaped);
    let (root, p) = setup();
    let cancel = Cancellation::default();
    let token = cancel.clone();
    let progress = root.path().join("hang/stdout.jsonl");
    let thread = std::thread::spawn(move || {
        wait_round(&progress);
        token.cancel();
    });
    let mut command = worker();
    command.arg("--canary=hang");
    let report = execute_worker(
        root.path(),
        p,
        command,
        &root.path().join("hang"),
        cancel,
        None,
    )
    .unwrap();
    thread.join().unwrap();
    assert_eq!(report.terminal, Terminal::Cancelled, "{}", report.detail);
    assert!(report.detail.contains("cancellation drain"));
    assert_eq!(report.remaining_members, Some(0));
    assert!(run_cancel(vec![]).is_err());
    assert!(request_cancel(root.path(), Path::new("foreign/checkpoint.json")).is_err());
}
