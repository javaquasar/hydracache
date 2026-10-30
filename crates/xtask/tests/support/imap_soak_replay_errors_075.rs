use super::*;
use serde_json::json;
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
fn bindings() -> ReplayBindings {
    ReplayBindings {
        source: |_| Ok("a".repeat(40)),
        profile: |root, _, _, _| {
            Ok(serde_json::from_slice(&fs::read(
                root.join("unit-profile.json"),
            )?)?)
        },
        artifacts: |_, _| Ok(()),
        live: |_| Ok(false),
    }
}
fn bad_ready() -> Vec<u8> {
    let value = json!({"schema":"imap075-soak-frame-v1","phase":"ready","pid":17,"incarnation":"unit","cohort":"reference","seed":117,"source_sha":"b".repeat(40),"round":0,"active_elapsed_ms":0,"digest":null,"owners":{"unit":"reference_owner_bytes","values":0,"outcomes":0,"pending":0,"listeners":0,"streams":0,"resync":0,"queued":0,"correlations":0},"heap_bytes":null,"rss_bytes":null,"gc_collections":null,"cpu_ns":null,"threads":null,"reason":null});
    let frame: Frame = serde_json::from_value(value).unwrap();
    let mut bytes = serde_json::to_vec(&frame).unwrap();
    bytes.push(b'\n');
    bytes
}
#[test]
fn runtime_command_launcher_utf8_and_hash_refusals_precede_any_trusted_profile() {
    assert!(runtime_observation(Command::new("unit-imap075-missing-runtime-program")).is_err());
    assert!(runtime_using(
        Cohort::Native,
        || Err("unit launcher refused".into()),
        |_| panic!("missing launcher cannot run"),
        |_| panic!("missing launcher cannot hash")
    )
    .is_err());
    assert!(runtime_using(
        Cohort::Reference,
        || panic!("reference has no Java launcher"),
        |command| {
            assert_eq!(command.get_program(), "rustc");
            Err("unit runtime refused".into())
        },
        |_| panic!()
    )
    .is_err());
    assert!(runtime_using(
        Cohort::Native,
        || Ok("unit-launcher".into()),
        |_| Ok((vec![255], vec![])),
        |_| panic!("malformed version cannot hash")
    )
    .is_err());
    assert!(runtime_using(
        Cohort::Native,
        || Ok("unit-launcher".into()),
        |_| Ok((b"unit JVM".to_vec(), vec![])),
        |_| Err("unit launcher hash refused".into())
    )
    .is_err());
}
#[test]
fn trusted_profile_refuses_substituted_native_pin_and_missing_tooling_before_runtime() {
    let root = tempfile::tempdir().unwrap();
    let paths = [
        "tests/java-imap-benchmark/target/java-imap-benchmark-0.75.0-SNAPSHOT.jar",
        "tests/java-imap-hazelcast-075/target/java-imap-hazelcast-075-0.75.0-SNAPSHOT.jar",
        "tests/java-imap-hazelcast-075/target/dependencies/hazelcast-5.5.0.jar",
    ];
    for relative in paths {
        let path = root.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"unit bytes; not a JAR").unwrap();
    }
    assert!(trusted_profile_using(
        root.path(),
        &"a".repeat(40),
        Cohort::Native,
        SoakBounds::smoke(),
        |_| panic!("wrong native pin stops tooling"),
        |_| panic!("wrong native pin stops runtime")
    )
    .unwrap_err()
    .to_string()
    .contains("native pin differs"));
    let worker = binary(root.path(), "imap-soak-worker-075");
    fs::create_dir_all(worker.parent().unwrap()).unwrap();
    fs::write(worker, b"unit bytes; never executed").unwrap();
    assert!(trusted_profile_using(
        root.path(),
        &"a".repeat(40),
        Cohort::Reference,
        SoakBounds::smoke(),
        |_| Err("unit tooling refused".into()),
        |_| panic!("missing tooling stops runtime")
    )
    .is_err());
}
#[test]
fn replay_failures_keep_failed_fixture_ineligible_and_never_read_fixture_pids() {
    let root = tempfile::tempdir().unwrap();
    let control = root.path().join("target/imap-075/control");
    fs::create_dir_all(&control).unwrap();
    let p = profile();
    fs::write(
        root.path().join("unit-profile.json"),
        serde_json::to_vec(&p).unwrap(),
    )
    .unwrap();
    let report = failed(p.clone());
    fs::write(
        control.join("report.json"),
        serde_json::to_vec(&report).unwrap(),
    )
    .unwrap();
    fs::write(control.join("stdout.jsonl"), b"").unwrap();
    assert!(check_saved(root.path(), &control.join("report.json")).is_err());
    let mut b = bindings();
    b.profile = |_, _, _, _| Err("unit trusted profile refused".into());
    assert!(check_saved_using(root.path(), &control.join("report.json"), &b).is_err());
    assert_eq!(
        check_saved_using(root.path(), &control.join("report.json"), &bindings())
            .unwrap()
            .terminal,
        Terminal::Failed
    );
    fs::write(control.join("checkpoint.json"), b"{").unwrap();
    assert!(check_checkpoint(root.path(), &control.join("checkpoint.json")).is_err());
    assert!(request_cancel(root.path(), &control.join("checkpoint.json")).is_err());
    for mode in 0..4 {
        fs::write(
            control.join("checkpoint.json"),
            serde_json::to_vec(&report).unwrap(),
        )
        .unwrap();
        let mut b = bindings();
        if mode == 0 {
            b.profile = |_, _, _, _| Err("unit profile refused".into());
        }
        let identity = IdentityCheckpoint {
            schema: "imap075-soak-identity-v1".into(),
            segment: report.segment.clone(),
            worker: WorkerIdentity {
                pid: 17,
                platform_birth: "unit".into(),
                incarnation: "unit".into(),
            },
        };
        fs::write(
            control.join("identity.json"),
            if mode == 1 {
                b"{".to_vec()
            } else {
                serde_json::to_vec(&identity).unwrap()
            },
        )
        .unwrap();
        if mode == 2 {
            fs::remove_file(control.join("stdout.jsonl")).unwrap();
        } else {
            fs::write(control.join("stdout.jsonl"), bad_ready()).unwrap();
        }
        assert!(check_checkpoint_using(root.path(), &control.join("checkpoint.json"), &b).is_err());
    }
    let identity = WorkerIdentity {
        pid: 17,
        platform_birth: "unit".into(),
        incarnation: "unit".into(),
    };
    assert!(replay_saved_progress(&p, &identity, &bad_ready(), 0, 0).is_err());
    assert!(replay_saved_progress(&p, &identity, b"", 0, 0).is_err());
    assert!(check_manual_workflow(root.path()).is_err());
    assert!(!root.path().join("qualified.json").exists());
}
#[cfg(windows)]
#[test]
fn process_times_failure_is_captured_before_handle_cleanup_and_success_is_stable() {
    let error = windows_process_birth(false, 0, 0, || {
        std::io::Error::from(std::io::ErrorKind::PermissionDenied)
    })
    .unwrap_err();
    assert_eq!(
        error.downcast_ref::<std::io::Error>().unwrap().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert_eq!(
        windows_process_birth(true, 1, 2, || panic!(
            "successful times cannot read stale error"
        ))
        .unwrap(),
        "windows-filetime-0000000100000002"
    );
}
