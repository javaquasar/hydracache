use hydracache_long_run_supervisor_074::diagnostic_lease::{
    CellIntent, CellOutcome, DiagnosticBackend, DiagnosticClock, DiagnosticCoordinator,
    DiagnosticIdentity, DiagnosticStage, DiagnosticStopReason, TreeObservation,
    ACTIVE_DIAGNOSTIC_NAME,
};
use hydracache_long_run_supervisor_074::host_execution::{
    active_campaign_absent, HostExecutionClaim, HostExecutionError,
};
use std::fs;

fn identity() -> DiagnosticIdentity {
    DiagnosticIdentity {
        lease_id: "a".repeat(64),
        boot_id: "00000000-0000-4000-8000-000000000074".into(),
        binary_sha256: "b".repeat(64),
        build_provenance_sha256: "c".repeat(64),
    }
}

fn clock(seconds: u64) -> DiagnosticClock {
    DiagnosticClock {
        boot_id: identity().boot_id,
        monotonic_ns: seconds * 1_000_000_000,
    }
}

#[derive(Default)]
struct Backend {
    starts: Vec<CellIntent>,
    stops: usize,
    finished: bool,
    failed_receipt: bool,
    overflow: bool,
    counter_overflow: bool,
    leader_exited: bool,
    populated_after_stop: bool,
    drift: bool,
    start_error: bool,
    stop_error: bool,
    retained: bool,
}

impl DiagnosticBackend for Backend {
    fn start_once(&mut self, intent: &CellIntent) -> Result<(), String> {
        self.starts.push(intent.clone());
        if self.start_error {
            Err("injected after intent".into())
        } else {
            Ok(())
        }
    }
    fn observe_tree(&mut self, intent: &CellIntent) -> Result<TreeObservation, String> {
        Ok(TreeObservation {
            unit_name: intent.unit_name.clone(),
            boot_id: identity().boot_id,
            cgroup_path: intent.cgroup_path.clone(),
            cgroup_inode: if self.drift { 501 } else { 500 },
            populated: !self.finished,
            stdout_bytes: if self.counter_overflow {
                u64::MAX
            } else if self.overflow {
                16_777_217
            } else {
                80
            },
            stderr_bytes: u64::from(self.counter_overflow),
            receipts_retained: self.retained,
            outcome: (self.finished || self.leader_exited).then_some(CellOutcome {
                successful: true,
                valid: !self.failed_receipt,
            }),
        })
    }
    fn stop_tree(&mut self, _intent: &CellIntent) -> Result<(), String> {
        self.stops += 1;
        if self.stop_error {
            return Err("injected cleanup failure".into());
        }
        self.finished = !self.populated_after_stop;
        Ok(())
    }
}

fn fixture() -> (tempfile::TempDir, DiagnosticCoordinator, Backend) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("campaigns");
    fs::create_dir(&root).unwrap();
    let coordinator = DiagnosticCoordinator::reserve(&root, identity(), &clock(0)).unwrap();
    (
        dir,
        coordinator,
        Backend {
            retained: true,
            ..Backend::default()
        },
    )
}

#[test]
fn diagnostic_and_campaign_reservations_share_one_short_transaction_lock() {
    let (dir, coordinator, _backend) = fixture();
    let root = dir.path().join("campaigns");
    assert!(!active_campaign_absent(&root).unwrap());
    assert!(matches!(
        HostExecutionClaim::acquire(&root, &"d".repeat(64)),
        Err(HostExecutionError::DiagnosticConflict)
    ));
    assert!(HostExecutionClaim::recover_active(&root).unwrap().is_none());
    assert_eq!(
        coordinator.state().unwrap().stage,
        DiagnosticStage::Reserved
    );
    assert!(DiagnosticCoordinator::reserve(&root, identity(), &clock(1)).is_err());
}

#[test]
fn existing_campaign_and_each_fixture_context_refuse_reservation() {
    for context in [
        None,
        Some("campaign-lifecycle-smoke-v1.json"),
        Some("controller-loss-smoke-v1.json"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("campaigns");
        fs::create_dir(&root).unwrap();
        if let Some(context) = context {
            fs::write(dir.path().join(context), b"fixture").unwrap();
        } else {
            drop(HostExecutionClaim::acquire(&root, &"d".repeat(64)).unwrap());
        }
        assert!(DiagnosticCoordinator::reserve(&root, identity(), &clock(0)).is_err());
        assert!(!root.join(ACTIVE_DIAGNOSTIC_NAME).exists());
    }
}

#[test]
fn four_fixed_cells_finish_once_and_terminal_evidence_precedes_release() {
    let (dir, coordinator, mut backend) = fixture();
    for (index, surface) in ["embedded", "direct", "resp2", "resp3"].iter().enumerate() {
        let now = clock(index as u64 + 1);
        coordinator.heartbeat(&now).unwrap();
        backend.finished = false;
        assert_eq!(
            coordinator.advance(&now, &mut backend).unwrap().stage,
            DiagnosticStage::Running
        );
        assert_eq!(backend.starts[index].surface, *surface);
        backend.finished = true;
        let state = coordinator.advance(&now, &mut backend).unwrap();
        assert_eq!(state.completed_cells, index + 1);
    }
    let root = dir.path().join("campaigns");
    assert!(!root.join(ACTIVE_DIAGNOSTIC_NAME).exists());
    assert!(root
        .join(format!("diagnostic-{}.terminal.json", identity().lease_id))
        .is_file());
    assert_eq!(backend.starts.len(), 4);
    assert_eq!(backend.stops, 0);
    assert!(DiagnosticCoordinator::reserve(&root, identity(), &clock(5)).is_err());
    assert!(HostExecutionClaim::acquire(&root, &"d".repeat(64)).is_ok());
}

#[test]
fn controller_loss_and_deadlines_stop_tree_before_release() {
    for (seconds, refresh, expected) in [
        (10, false, DiagnosticStopReason::ControllerLost),
        (60, true, DiagnosticStopReason::CellDeadline),
        (300, true, DiagnosticStopReason::TotalDeadline),
    ] {
        let (dir, coordinator, mut backend) = fixture();
        coordinator.advance(&clock(0), &mut backend).unwrap();
        if refresh {
            for tick in (5..=seconds).step_by(5) {
                coordinator.heartbeat(&clock(tick)).unwrap();
            }
        }
        let state = coordinator.advance(&clock(seconds), &mut backend).unwrap();
        assert_eq!(state.reason, Some(expected));
        assert_eq!(state.stage, DiagnosticStage::Terminal);
        assert_eq!(backend.stops, 1);
        assert_eq!(backend.starts.len(), 1);
        assert!(!dir
            .path()
            .join("campaigns")
            .join(ACTIVE_DIAGNOSTIC_NAME)
            .exists());
    }
}

#[test]
fn durable_start_intent_is_never_replayed_as_a_second_spawn() {
    let (dir, coordinator, mut backend) = fixture();
    backend.start_error = true;
    assert!(coordinator.advance(&clock(0), &mut backend).is_err());
    assert_eq!(
        coordinator.state().unwrap().stage,
        DiagnosticStage::Starting
    );
    let recovered =
        DiagnosticCoordinator::recover(&dir.path().join("campaigns"), &identity().lease_id)
            .unwrap();
    let state = recovered.advance(&clock(1), &mut backend).unwrap();
    assert_eq!(state.reason, Some(DiagnosticStopReason::InterruptedStart));
    assert_eq!(backend.starts.len(), 1);
    assert_eq!(backend.stops, 1);
}

#[test]
fn cleanup_failure_and_identity_drift_keep_host_claimed() {
    for mode in 0..4 {
        let (dir, coordinator, mut backend) = fixture();
        coordinator.advance(&clock(0), &mut backend).unwrap();
        coordinator.advance(&clock(1), &mut backend).unwrap(); // bind original cgroup inode
        match mode {
            0 => backend.stop_error = true,
            1 => backend.populated_after_stop = true,
            2 => backend.drift = true,
            _ => backend.retained = false,
        }
        assert!(coordinator.advance(&clock(10), &mut backend).is_err());
        let root = dir.path().join("campaigns");
        assert!(root.join(ACTIVE_DIAGNOSTIC_NAME).is_file());
        assert!(matches!(
            HostExecutionClaim::acquire(&root, &"d".repeat(64)),
            Err(HostExecutionError::DiagnosticConflict)
        ));
        if mode == 2 {
            assert_eq!(backend.stops, 0);
        }
    }
}

#[test]
fn overflow_and_invalid_receipts_are_retained_without_retry() {
    for overflow in [true, false] {
        let (_dir, coordinator, mut backend) = fixture();
        coordinator.advance(&clock(0), &mut backend).unwrap();
        backend.overflow = overflow;
        backend.finished = !overflow;
        backend.failed_receipt = !overflow;
        let state = coordinator.advance(&clock(1), &mut backend).unwrap();
        assert_eq!(
            state.reason,
            Some(if overflow {
                DiagnosticStopReason::ReceiptOverflow
            } else {
                DiagnosticStopReason::InvalidReceipt
            })
        );
        assert_eq!(state.stage, DiagnosticStage::Terminal);
        assert_eq!(backend.starts.len(), 1);
        assert_eq!(state.completed_cells, 0);
    }
}

#[test]
fn boot_clock_and_lease_identity_drift_fail_without_backend_calls() {
    let (_dir, coordinator, mut backend) = fixture();
    let mut changed = clock(1);
    changed.boot_id = "different-boot".into();
    assert!(coordinator.advance(&changed, &mut backend).is_err());
    coordinator.heartbeat(&clock(2)).unwrap();
    assert!(coordinator.advance(&clock(1), &mut backend).is_err());
    assert!(backend.starts.is_empty());
    assert_eq!(backend.stops, 0);
    assert!(DiagnosticCoordinator::recover(coordinator.root(), &"d".repeat(64)).is_err());
}

#[test]
fn malformed_or_ambiguous_diagnostic_markers_fail_closed() {
    let (dir, _coordinator, _backend) = fixture();
    let root = dir.path().join("campaigns");
    fs::write(
        root.join("active-campaign"),
        format!("{}\n", "d".repeat(64)),
    )
    .unwrap();
    assert!(HostExecutionClaim::recover_active(&root).is_err());
    fs::remove_file(root.join("active-campaign")).unwrap();
    fs::write(root.join(ACTIVE_DIAGNOSTIC_NAME), b"{}\n").unwrap();
    assert!(HostExecutionClaim::recover_active(&root).is_err());
    assert!(DiagnosticCoordinator::recover(&root, &identity().lease_id).is_err());
}

#[test]
fn cancel_before_start_never_needs_a_backend_or_spawns_work() {
    let (_dir, coordinator, mut backend) = fixture();
    let state = coordinator.cancel(&clock(0), &mut backend).unwrap();
    assert_eq!(state.reason, Some(DiagnosticStopReason::OperatorCancelled));
    assert_eq!(state.stage, DiagnosticStage::Terminal);
    assert!(backend.starts.is_empty());
    assert_eq!(backend.stops, 0);
}

#[cfg(unix)]
#[test]
fn linked_marker_and_pending_crash_file_are_not_silently_recovered() {
    let (dir, _coordinator, _backend) = fixture();
    let root = dir.path().join("campaigns");
    fs::hard_link(root.join(ACTIVE_DIAGNOSTIC_NAME), root.join("alias")).unwrap();
    assert!(HostExecutionClaim::recover_active(&root).is_err());
    fs::remove_file(root.join("alias")).unwrap();
    fs::write(root.join(".active-diagnostic.pending"), b"torn").unwrap();
    assert!(DiagnosticCoordinator::recover(&root, &identity().lease_id).is_err());
}

#[test]
fn expired_controller_cannot_revive_or_extend_a_cell_deadline() {
    let (_dir, coordinator, mut backend) = fixture();
    coordinator.advance(&clock(0), &mut backend).unwrap();
    assert!(coordinator.heartbeat(&clock(10)).is_err());
    assert_eq!(coordinator.state().unwrap().controller_monotonic_ns, 0);
    let state = coordinator.advance(&clock(10), &mut backend).unwrap();
    assert_eq!(state.reason, Some(DiagnosticStopReason::ControllerLost));
}

#[test]
fn terminal_publication_gap_recovers_identical_receipt_without_spawn() {
    let (dir, coordinator, mut backend) = fixture();
    coordinator.cancel(&clock(0), &mut backend).unwrap();
    let root = dir.path().join("campaigns");
    let receipt = root.join(format!("diagnostic-{}.terminal.json", identity().lease_id));
    let sealed = fs::read(&receipt).unwrap();
    // Model crash after durable archive, before active marker removal.
    fs::write(root.join(ACTIVE_DIAGNOSTIC_NAME), &sealed).unwrap();
    let recovered = DiagnosticCoordinator::recover(&root, &identity().lease_id).unwrap();
    assert_eq!(
        recovered.advance(&clock(1), &mut backend).unwrap().stage,
        DiagnosticStage::Terminal
    );
    assert_eq!(fs::read(receipt).unwrap(), sealed);
    assert!(!root.join(ACTIVE_DIAGNOSTIC_NAME).exists());
    assert!(backend.starts.is_empty());
}

#[test]
fn terminal_receipt_conflict_does_not_release_the_marker() {
    let (dir, coordinator, mut backend) = fixture();
    let root = dir.path().join("campaigns");
    fs::write(
        root.join(format!("diagnostic-{}.terminal.json", identity().lease_id)),
        b"conflict",
    )
    .unwrap();
    assert!(coordinator.cancel(&clock(0), &mut backend).is_err());
    assert!(root.join(ACTIVE_DIAGNOSTIC_NAME).exists());
    assert!(backend.starts.is_empty());
}

#[test]
fn concurrent_campaign_and_diagnostic_admission_has_exactly_one_winner() {
    for _ in 0..8 {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("campaigns");
        fs::create_dir(&root).unwrap();
        let barrier = std::sync::Barrier::new(2);
        std::thread::scope(|scope| {
            let campaign = scope.spawn(|| {
                barrier.wait();
                HostExecutionClaim::acquire(&root, &"d".repeat(64)).is_ok()
            });
            let diagnostic = scope.spawn(|| {
                barrier.wait();
                DiagnosticCoordinator::reserve(&root, identity(), &clock(0)).is_ok()
            });
            assert_ne!(campaign.join().unwrap(), diagnostic.join().unwrap());
        });
    }
}

#[test]
fn remaining_total_budget_caps_the_fixed_unit_without_padding() {
    let (_dir, coordinator, mut backend) = fixture();
    for tick in (5..=290).step_by(5) {
        coordinator.heartbeat(&clock(tick)).unwrap();
    }
    coordinator.advance(&clock(290), &mut backend).unwrap();
    assert_eq!(backend.starts[0].maximum_runtime_seconds, 10);
    for tick in [295, 300, 305] {
        coordinator.heartbeat(&clock(tick)).unwrap();
    }
    assert_eq!(
        coordinator
            .advance(&clock(305), &mut backend)
            .unwrap()
            .reason,
        Some(DiagnosticStopReason::TotalDeadline)
    );
}

#[test]
fn exited_leader_never_certifies_a_populated_child_tree() {
    let (_dir, coordinator, mut backend) = fixture();
    coordinator.advance(&clock(0), &mut backend).unwrap();
    backend.leader_exited = true;
    let state = coordinator.advance(&clock(1), &mut backend).unwrap();
    assert_eq!(state.stage, DiagnosticStage::Running);
    assert_eq!(state.completed_cells, 0);
    assert!(!state.cleanup_confirmed);
    assert!(coordinator.root().join(ACTIVE_DIAGNOSTIC_NAME).exists());
}

#[test]
fn overflowing_receipt_counters_stop_instead_of_wrapping_or_retrying() {
    let (_dir, coordinator, mut backend) = fixture();
    coordinator.advance(&clock(0), &mut backend).unwrap();
    backend.counter_overflow = true;
    let state = coordinator.advance(&clock(1), &mut backend).unwrap();
    assert_eq!(state.reason, Some(DiagnosticStopReason::ReceiptOverflow));
    assert_eq!(backend.stops, 1);
    assert_eq!(backend.starts.len(), 1);
}

#[test]
fn unknown_schema_fields_and_noncanonical_digest_drift_fail_closed() {
    for mode in 0..5 {
        let (_dir, coordinator, mut backend) = fixture();
        let path = coordinator.root().join(ACTIVE_DIAGNOSTIC_NAME);
        let mut envelope: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        match mode {
            0 => envelope["schema_version"] = serde_json::json!(2),
            1 => envelope["unknown"] = serde_json::json!(true),
            2 => envelope["state"]["revision"] = serde_json::json!(0),
            3 => envelope["state_sha256"] = serde_json::json!("0".repeat(64)),
            _ => envelope["state"]["promotable"] = serde_json::json!(true),
        }
        fs::write(&path, serde_json::to_vec(&envelope).unwrap()).unwrap();
        assert!(coordinator.advance(&clock(1), &mut backend).is_err());
        assert!(path.is_file());
        assert!(backend.starts.is_empty());
        assert!(HostExecutionClaim::recover_active(coordinator.root()).is_err());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn diagnostic_unit_properties_are_fixed_bounded_and_never_dispatch_a_process() {
    use hydracache_long_run_supervisor_074::diagnostic_unit::build_diagnostic_unit_spec;
    use hydracache_long_run_supervisor_074::systemd_unit::UnitProperty;
    let (_dir, coordinator, mut backend) = fixture();
    let spec = build_diagnostic_unit_spec(&coordinator.state().unwrap()).unwrap();
    let find = |key| {
        &spec
            .properties
            .iter()
            .find(|(name, _)| *name == key)
            .unwrap()
            .1
    };
    assert_eq!(
        find("KillMode"),
        &UnitProperty::Text("control-group".into())
    );
    assert_eq!(find("Restart"), &UnitProperty::Text("no".into()));
    assert_eq!(find("Delegate"), &UnitProperty::Boolean(false));
    assert_eq!(find("CPUAffinity"), &UnitProperty::Bytes(vec![2]));
    assert_eq!(find("RuntimeMaxUSec"), &UnitProperty::Unsigned(60_000_000));
    assert_eq!(find("LimitFSIZE"), &UnitProperty::Unsigned(8_388_608));
    let UnitProperty::Commands(commands) = find("ExecStart") else {
        panic!("fixed argv required")
    };
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].argv[1], "--run");
    assert_eq!(commands[0].argv[3], identity().binary_sha256);
    assert_eq!(
        commands[0].argv[4],
        "62114be0f5da3218706e30d7424acfb5d0579d07"
    );
    assert_eq!(commands[0].path, commands[0].argv[0]);
    assert!(!commands[0].ignore_failure);
    coordinator.advance(&clock(0), &mut backend).unwrap();
    assert!(build_diagnostic_unit_spec(&coordinator.state().unwrap()).is_err());
    assert_eq!(backend.starts.len(), 1);
}
