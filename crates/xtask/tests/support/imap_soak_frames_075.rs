use super::*;
fn profile() -> SoakProfile {
    SoakProfile {
        schema: "imap075-soak-profile-v1".into(),
        source_sha: "a".repeat(40),
        cohort: Cohort::Reference,
        seed: 117,
        bounds: SoakBounds::smoke(),
        runtime: "unit-frame-parser".into(),
        artifacts: vec![BoundArtifact {
            path: "unit-never-executed".into(),
            sha256: "a".repeat(64),
        }],
    }
}
fn parser() -> WorkerFrames {
    WorkerFrames {
        parsed: 0,
        state: SegmentState::new(profile()).unwrap(),
        birth: None,
    }
}
fn ready() -> Vec<u8> {
    let p = profile();
    let frame = Frame {
        schema: "imap075-soak-frame-v1".into(),
        phase: Phase::Ready,
        pid: 17,
        incarnation: "unit-incarnation".into(),
        cohort: p.cohort,
        seed: p.seed,
        source_sha: p.source_sha,
        round: 0,
        active_elapsed_ms: 0,
        digest: None,
        owners: Owners {
            unit: "reference_owner_bytes".into(),
            values: Some(0),
            outcomes: Some(0),
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
    };
    let mut bytes = serde_json::to_vec(&frame).unwrap();
    bytes.push(b'\n');
    bytes
}
fn observation(bytes: &[u8], root_exited: bool) -> process::ProcessObservation<'_> {
    process::ProcessObservation {
        stdout: bytes,
        stderr: &[],
        root_exited,
        pid: 17,
        elapsed: Duration::ZERO,
    }
}
#[test]
fn soak_frames_keep_partial_input_and_bind_ready_once_without_publishing_a_report() {
    let root = tempfile::tempdir().unwrap();
    let mut parser = parser();
    let bytes = ready();
    parser
        .accept(
            &observation(&bytes[..3], false),
            root.path(),
            "unit-segment",
            |_| panic!("partial frame must not bind identity"),
        )
        .unwrap();
    assert_eq!(parser.parsed, 0);
    let mut births = 0;
    parser
        .accept(
            &observation(&bytes, false),
            root.path(),
            "unit-segment",
            |pid| {
                assert_eq!(pid, 17);
                births += 1;
                Ok("unit-birth".into())
            },
        )
        .unwrap();
    assert_eq!(births, 1);
    assert_eq!(parser.parsed, bytes.len());
    parser
        .accept(
            &observation(&bytes, true),
            root.path(),
            "unit-segment",
            |_| panic!("identity must not be rebound"),
        )
        .unwrap();
    assert!(root.path().join("identity.json").is_file());
    assert!(marker(root.path(), "start").unwrap());
    assert_eq!(parser.state.phase, Some(Phase::Ready));
    assert!(parser.state.terminal.is_none());
    assert!(!root.path().join("report.json").exists());
    assert!(parser
        .accept(
            &observation(&[], false),
            root.path(),
            "unit-segment",
            |_| panic!("regressed output must not rebind identity")
        )
        .unwrap_err()
        .contains("observation regressed"));
}
#[test]
fn oversized_dead_unreadable_or_malformed_ready_frames_never_create_start_controls() {
    for mode in 0..5 {
        let root = tempfile::tempdir().unwrap();
        let mut parser = parser();
        let mut bytes = match mode {
            0 | 1 => vec![b'x'; 4097],
            2 | 3 => ready(),
            _ => b"{\n".to_vec(),
        };
        if mode == 0 {
            bytes.push(b'\n');
        }
        let error = parser
            .accept(
                &observation(&bytes, mode == 2),
                root.path(),
                "unit-segment",
                |_| {
                    if mode == 3 {
                        Err("unit identity unreadable".into())
                    } else {
                        panic!("invalid frame must not read a process identity")
                    }
                },
            )
            .unwrap_err();
        assert!(!error.is_empty());
        assert!(!root.path().join("start").exists());
        assert!(!root.path().join("report.json").exists());
    }
}
#[test]
fn real_identity_and_start_marker_io_failures_stop_readiness_publication() {
    for block_identity in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let mut parser = parser();
        if block_identity {
            fs::write(root.path().join("identity.json"), b"foreign").unwrap();
        } else {
            fs::create_dir(root.path().join("start")).unwrap();
        }
        assert!(parser
            .accept(
                &observation(&ready(), false),
                root.path(),
                "unit-segment",
                |_| Ok("unit-birth".into())
            )
            .is_err());
        assert!(!root.path().join("report.json").exists());
    }
}

fn controls<'a>(
    root: &'a Path,
    p: &'a SoakProfile,
    cancel: &'a Cancellation,
    now: Instant,
) -> WorkerControl<'a> {
    WorkerControl {
        root,
        profile: p,
        control: root,
        cancel,
        started: now,
        cancellation_at: None,
        timeout_drain: false,
        last_binding: now,
    }
}
fn no_binding_check(_: &SoakProfile, _: &Path) -> Result<(), Box<dyn Error>> {
    panic!("binding interval has not expired")
}
#[test]
fn readiness_and_drain_deadlines_use_exact_observed_clock_boundaries() {
    let root = tempfile::tempdir().unwrap();
    let p = profile();
    let cancel = Cancellation::default();
    let now = Instant::now();
    let mut startup = controls(root.path(), &p, &cancel, now);
    let observation = observation(&[], false);
    startup
        .observe(
            &observation,
            false,
            || now + Duration::from_millis(p.bounds.startup_ms),
            |_, _| Ok(()),
        )
        .unwrap();
    assert!(startup
        .observe(
            &observation,
            false,
            || now + Duration::from_millis(p.bounds.startup_ms + 1),
            |_, _| Ok(())
        )
        .unwrap_err()
        .contains("readiness deadline"));
    let mut controls = controls(root.path(), &p, &cancel, now);
    let mut budget = observation;
    budget.elapsed = Duration::from_millis(p.bounds.hard_ms - p.bounds.drain_ms);
    controls
        .observe(&budget, true, || now, no_binding_check)
        .unwrap();
    assert!(controls.timeout_drain);
    assert_eq!(controls.cancellation_at, Some(now));
    controls
        .observe(
            &budget,
            true,
            || now + Duration::from_millis(p.bounds.drain_ms),
            |_, _| Ok(()),
        )
        .unwrap();
    assert!(controls
        .observe(
            &budget,
            true,
            || now + Duration::from_millis(p.bounds.drain_ms + 1),
            no_binding_check
        )
        .unwrap_err()
        .contains("cancellation drain deadline"));
    budget.root_exited = true;
    controls
        .observe(
            &budget,
            true,
            || now + Duration::from_millis(p.bounds.drain_ms + 1),
            |_, _| Ok(()),
        )
        .unwrap();
    assert!(!root.path().join("report.json").exists());
}
#[test]
fn explicit_cancel_and_marker_io_are_not_reported_as_active_budget_expiry() {
    for operator_marker in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let p = profile();
        let cancel = Cancellation::default();
        let now = Instant::now();
        if operator_marker {
            publish_marker(root.path(), "cancel").unwrap();
        } else {
            cancel.cancel();
        }
        let mut controls = controls(root.path(), &p, &cancel, now);
        controls
            .observe(&observation(&[], false), true, || now, no_binding_check)
            .unwrap();
        assert!(controls.cancellation_at.is_some());
        assert!(!controls.timeout_drain);
    }
    let root = tempfile::tempdir().unwrap();
    let p = profile();
    let cancel = Cancellation::default();
    let now = Instant::now();
    fs::write(root.path().join("cancel"), b"foreign marker").unwrap();
    let mut controls = controls(root.path(), &p, &cancel, now);
    assert!(controls
        .observe(&observation(&[], false), true, || now, no_binding_check)
        .is_err());
}
#[test]
fn periodic_artifact_rebinding_refuses_failure_and_samples_after_success() {
    let root = tempfile::tempdir().unwrap();
    let p = profile();
    let cancel = Cancellation::default();
    let now = Instant::now();
    let mut controls = controls(root.path(), &p, &cancel, now);
    assert!(controls
        .observe(
            &observation(&[], false),
            true,
            || now + Duration::from_secs(10),
            |_, _| Err("unit artifact changed".into())
        )
        .unwrap_err()
        .contains("unit artifact changed"));
    assert_eq!(controls.last_binding, now);
    controls
        .observe(
            &observation(&[], false),
            true,
            || now + Duration::from_secs(10),
            |_, _| Ok(()),
        )
        .unwrap();
    assert_eq!(controls.last_binding, now + Duration::from_secs(10));
    controls
        .observe(
            &observation(&[], false),
            true,
            || now + Duration::from_secs(11),
            no_binding_check,
        )
        .unwrap();
    assert!(!root.path().join("report.json").exists());
}
#[test]
fn daemon_deadlines_keep_exact_startup_and_drain_boundaries_and_allow_observed_exit() {
    let b = SoakBounds::smoke();
    let startup = Duration::from_millis(b.startup_ms);
    let drain = Duration::from_millis(b.drain_ms);
    daemon_deadlines(false, false, &b, || startup, || None).unwrap();
    assert!(daemon_deadlines(
        false,
        false,
        &b,
        || startup + Duration::from_millis(1),
        || panic!("startup refusal stops before drain")
    )
    .unwrap_err()
    .contains("readiness deadline"));
    daemon_deadlines(
        true,
        false,
        &b,
        || panic!("a ready daemon skips startup clock"),
        || Some(drain),
    )
    .unwrap();
    assert!(daemon_deadlines(
        true,
        false,
        &b,
        || Duration::ZERO,
        || Some(drain + Duration::from_millis(1))
    )
    .unwrap_err()
    .contains("drain deadline"));
    daemon_deadlines(
        true,
        true,
        &b,
        || Duration::ZERO,
        || Some(drain + Duration::from_millis(1)),
    )
    .unwrap();
    assert_eq!(daemon_drain_reason(true), "cancelled");
    assert_eq!(daemon_drain_reason(false), "duration_complete");
    daemon_round_budget(b.maximum_rounds - 1, b.maximum_rounds).unwrap();
    assert!(daemon_round_budget(b.maximum_rounds, b.maximum_rounds)
        .unwrap_err()
        .contains("round limit"));
}
#[test]
fn wrong_daemon_cohort_and_marker_io_failures_refuse_before_any_worker_or_control_is_created() {
    let root = tempfile::tempdir().unwrap();
    let control = root.path().join("control");
    assert!(execute_daemon(
        root.path(),
        profile(),
        Path::new("missing-binary"),
        &control,
        Cancellation::default(),
        None
    )
    .unwrap_err()
    .to_string()
    .contains("daemon soak cohort required"));
    assert!(!control.exists());
    for kind in [
        std::io::ErrorKind::PermissionDenied,
        std::io::ErrorKind::WouldBlock,
    ] {
        let e = marker_using(root.path(), "cancel", |path| {
            assert_eq!(path, root.path().join("cancel"));
            Err(std::io::Error::from(kind))
        })
        .unwrap_err();
        assert_eq!(e.downcast_ref::<std::io::Error>().unwrap().kind(), kind);
    }
    assert!(
        !marker_using(root.path(), "cancel", |_| Err(std::io::Error::from(
            std::io::ErrorKind::NotFound
        )))
        .unwrap()
    );
    assert!(marker_using(root.path(), "foreign", |_| panic!(
        "invalid marker does not open files"
    ))
    .is_err());
}
