use super::{
    validate_transition, DiagnosticIdentity, DiagnosticStage as Stage, DiagnosticState,
    DiagnosticStopReason as Reason, CONTROLLER_LOSS_SECONDS, NS, TOTAL_SECONDS,
};

fn reserved() -> DiagnosticState {
    DiagnosticState {
        identity: DiagnosticIdentity {
            lease_id: "a".repeat(64),
            boot_id: "00000000-0000-4000-8000-000000000074".into(),
            binary_sha256: "b".repeat(64),
            build_provenance_sha256: "c".repeat(64),
        },
        revision: 1,
        stage: Stage::Reserved,
        completed_cells: 0,
        reserved_monotonic_ns: 0,
        last_observed_monotonic_ns: 1,
        controller_monotonic_ns: 0,
        cell_started_monotonic_ns: None,
        cgroup_inode: None,
        reason: None,
        cleanup_confirmed: true,
        promotable: false,
        admission_allowed: false,
    }
}

fn at(stage: Stage) -> DiagnosticState {
    let mut state = reserved();
    state.stage = stage;
    if stage != Stage::Reserved {
        state.cell_started_monotonic_ns = Some(1);
        state.cgroup_inode = Some(74);
        state.cleanup_confirmed = stage == Stage::Terminal;
    }
    if matches!(stage, Stage::Stopping | Stage::Terminal) {
        state.reason = Some(Reason::OperatorCancelled);
    }
    state
}

fn candidate(previous: &DiagnosticState) -> DiagnosticState {
    let mut next = previous.clone();
    next.revision += 1;
    next.last_observed_monotonic_ns += 1;
    next
}

#[test]
fn transition_entire_stage_graph_is_explicit() {
    let stages = [
        Stage::Reserved,
        Stage::Starting,
        Stage::Running,
        Stage::Stopping,
        Stage::Terminal,
    ];
    for from in stages {
        for to in stages {
            let mut previous = at(from);
            if from == Stage::Running && to == Stage::Terminal {
                previous.completed_cells = 3;
            }
            let mut next = at(to);
            next.revision = previous.revision + 1;
            next.last_observed_monotonic_ns = 2;
            if from == Stage::Reserved && to == Stage::Starting {
                next.cell_started_monotonic_ns = Some(2);
                next.cgroup_inode = None;
            }
            if from == Stage::Reserved && to == Stage::Terminal {
                next.cell_started_monotonic_ns = None;
                next.cgroup_inode = None;
            }
            if from == Stage::Running && matches!(to, Stage::Reserved | Stage::Terminal) {
                next.completed_cells = previous.completed_cells + 1;
                if to == Stage::Terminal {
                    next.reason = Some(Reason::Completed);
                }
            }
            let allowed = matches!(
                (from, to),
                (
                    Stage::Reserved,
                    Stage::Reserved | Stage::Starting | Stage::Terminal
                ) | (
                    Stage::Starting,
                    Stage::Starting | Stage::Running | Stage::Stopping
                ) | (
                    Stage::Running,
                    Stage::Running | Stage::Stopping | Stage::Reserved | Stage::Terminal
                ) | (Stage::Stopping, Stage::Stopping | Stage::Terminal)
            );
            assert_eq!(
                validate_transition(&previous, &next).is_ok(),
                allowed,
                "{from:?}->{to:?}"
            );
        }
    }
}

#[test]
fn transition_four_cells_seeded_histories_and_mutations() {
    let seed = 0x074e_u64;
    let mut random = seed;
    let mut accepted = 0;
    let mut refused = 0;
    for _ in 0..64 {
        let mut previous = reserved();
        for cell in 0..4 {
            for step in 0..5 {
                random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                let mut next = candidate(&previous);
                next.last_observed_monotonic_ns += random % 100;
                match step {
                    0 => next.controller_monotonic_ns = next.last_observed_monotonic_ns,
                    1 => {
                        next.stage = Stage::Starting;
                        next.cleanup_confirmed = false;
                        next.cell_started_monotonic_ns = Some(next.last_observed_monotonic_ns);
                    }
                    2 => next.stage = Stage::Running,
                    3 => next.cgroup_inode = Some(100 + cell),
                    _ => {
                        next.completed_cells += 1;
                        next.cleanup_confirmed = true;
                        if cell == 3 {
                            next.stage = Stage::Terminal;
                            next.reason = Some(Reason::Completed);
                        } else {
                            next.stage = Stage::Reserved;
                            next.cell_started_monotonic_ns = None;
                            next.cgroup_inode = None;
                        }
                    }
                }
                validate_transition(&previous, &next).unwrap();
                accepted += 1;
                let mut mutated = next.clone();
                mutated.revision += 1;
                assert!(validate_transition(&previous, &mutated).is_err());
                mutated = next.clone();
                mutated.completed_cells = previous.completed_cells + 2;
                assert!(validate_transition(&previous, &mutated).is_err());
                refused += 2;
                previous = next;
            }
        }
        assert_eq!(previous.completed_cells, 4);
        assert_eq!(previous.stage, Stage::Terminal);
        assert!(validate_transition(&previous, &candidate(&previous)).is_err());
    }
    assert_eq!((accepted, refused), (1280, 2560));
    println!("transition seed=0x{seed:04x} histories=64 accepted={accepted} refused={refused}");
}

#[test]
fn transition_identity_revision_clock_and_cell_drift_refuse_without_mutation() {
    let previous = at(Stage::Running);
    let next = candidate(&previous);
    let mutations: &[fn(&mut DiagnosticState)] = &[
        |s| s.identity.lease_id = "d".repeat(64),
        |s| s.identity.boot_id = "00000000-0000-4000-8000-000000000075".into(),
        |s| s.identity.binary_sha256 = "d".repeat(64),
        |s| s.identity.build_provenance_sha256 = "d".repeat(64),
        |s| s.reserved_monotonic_ns = 1,
        |s| s.revision = 0,
        |s| s.revision = 1,
        |s| s.revision = 3,
        |s| s.last_observed_monotonic_ns = 0,
        |s| s.controller_monotonic_ns = 1,
        |s| s.completed_cells = 1,
        |s| s.completed_cells = 4,
        |s| s.cell_started_monotonic_ns = Some(2),
        |s| s.cell_started_monotonic_ns = None,
        |s| s.cgroup_inode = None,
        |s| s.cgroup_inode = Some(75),
        |s| s.reason = Some(Reason::OperatorCancelled),
        |s| s.cleanup_confirmed = true,
        |s| s.promotable = true,
        |s| s.admission_allowed = true,
    ];
    for (index, mutate) in mutations.iter().enumerate() {
        let mut bad = next.clone();
        mutate(&mut bad);
        let snapshot = bad.clone();
        assert!(
            validate_transition(&previous, &bad).is_err(),
            "mutation {index}"
        );
        assert_eq!(bad, snapshot);
        assert_eq!(previous, at(Stage::Running));
    }
    let mut max = previous.clone();
    max.revision = u64::MAX;
    assert!(validate_transition(&max, &max).is_err());
    let mut old_clock = previous.clone();
    old_clock.controller_monotonic_ns = 1;
    assert!(validate_transition(&old_clock, &next).is_err());
}

#[test]
fn transition_original_inode_and_stop_reason_are_sticky() {
    let mut running = at(Stage::Running);
    running.cgroup_inode = None;
    let mut bound = candidate(&running);
    bound.cgroup_inode = Some(74);
    validate_transition(&running, &bound).unwrap();
    for inode in [None, Some(75)] {
        let mut next = candidate(&bound);
        next.cgroup_inode = inode;
        assert!(validate_transition(&bound, &next).is_err());
    }
    let stopping = at(Stage::Stopping);
    for reason in [None, Some(Reason::CellDeadline)] {
        let mut next = candidate(&stopping);
        next.stage = Stage::Terminal;
        next.cleanup_confirmed = true;
        next.reason = reason;
        assert!(validate_transition(&stopping, &next).is_err());
    }
    let mut next = candidate(&stopping);
    next.stage = Stage::Terminal;
    next.cleanup_confirmed = true;
    validate_transition(&stopping, &next).unwrap();
}

#[test]
fn transition_start_and_heartbeat_cannot_revive_expired_budgets() {
    let previous = reserved();
    for time in [CONTROLLER_LOSS_SECONDS * NS, TOTAL_SECONDS * NS] {
        let mut starting = candidate(&previous);
        starting.last_observed_monotonic_ns = time;
        starting.cell_started_monotonic_ns = Some(time);
        starting.stage = Stage::Starting;
        starting.cleanup_confirmed = false;
        assert!(validate_transition(&previous, &starting).is_err());
        let mut heartbeat = candidate(&previous);
        heartbeat.last_observed_monotonic_ns = time;
        heartbeat.controller_monotonic_ns = time;
        assert!(validate_transition(&previous, &heartbeat).is_err());
    }
    let mut previous = reserved();
    previous.last_observed_monotonic_ns = TOTAL_SECONDS * NS - 1;
    previous.controller_monotonic_ns = previous.last_observed_monotonic_ns;
    let mut next = candidate(&previous);
    next.stage = Stage::Starting;
    next.cleanup_confirmed = false;
    next.cell_started_monotonic_ns = Some(next.last_observed_monotonic_ns);
    assert!(validate_transition(&previous, &next).is_err());
    let previous = reserved();
    let mut next = candidate(&previous);
    next.stage = Stage::Starting;
    next.cleanup_confirmed = false;
    next.cell_started_monotonic_ns = Some(1);
    assert!(validate_transition(&previous, &next).is_err());
    next.cell_started_monotonic_ns = Some(next.last_observed_monotonic_ns);
    validate_transition(&previous, &next).unwrap();
}

#[test]
fn transition_completion_and_terminal_replay_never_skip_or_restart_cells() {
    for completed in 0..4 {
        let mut previous = at(Stage::Running);
        previous.completed_cells = completed;
        for count in 0..=4 {
            let mut next = candidate(&previous);
            next.stage = if count == 4 {
                Stage::Terminal
            } else {
                Stage::Reserved
            };
            next.completed_cells = count;
            next.cleanup_confirmed = true;
            if count == 4 {
                next.reason = Some(Reason::Completed);
            } else {
                next.cell_started_monotonic_ns = None;
                next.cgroup_inode = None;
            }
            assert_eq!(
                validate_transition(&previous, &next).is_ok(),
                count == completed + 1
            );
        }
    }
    let previous = at(Stage::Terminal);
    assert!(validate_transition(&previous, &previous).is_err());
    assert!(validate_transition(&previous, &candidate(&previous)).is_err());
    for reason in [
        Reason::CellDeadline,
        Reason::InvalidReceipt,
        Reason::InterruptedStart,
    ] {
        let previous = reserved();
        let mut next = candidate(&previous);
        next.stage = Stage::Terminal;
        next.reason = Some(reason);
        assert!(validate_transition(&previous, &next).is_err());
    }
}

#[test]
fn transition_publisher_refuses_invalid_candidate_before_any_write() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("campaigns");
    std::fs::create_dir(&root).unwrap();
    super::DiagnosticCoordinator::reserve(
        &root,
        reserved().identity,
        &super::DiagnosticClock {
            boot_id: reserved().identity.boot_id,
            monotonic_ns: 1,
        },
    )
    .unwrap();
    let (_root, _fence) = super::lock_host_root(&root).unwrap();
    let original = super::read_active(&root).unwrap().unwrap();
    let path = root.join(super::ACTIVE_DIAGNOSTIC_NAME);
    let raw = std::fs::read(&path).unwrap();
    for mode in 0..3 {
        let mut next = original.clone();
        match mode {
            0 => next.completed_cells = 1,
            1 => next.identity.binary_sha256 = "d".repeat(64),
            _ => next.controller_monotonic_ns = CONTROLLER_LOSS_SECONDS * NS,
        }
        let before = next.clone();
        assert!(super::bump_publish(&root, &mut next).is_err());
        assert_eq!(next, before);
        assert_eq!(std::fs::read(&path).unwrap(), raw);
        assert!(!root.join(super::PENDING_NAME).exists());
    }
}

#[test]
fn transition_publisher_refuses_stale_revision_without_replacing_state() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("campaigns");
    std::fs::create_dir(&root).unwrap();
    super::DiagnosticCoordinator::reserve(
        &root,
        reserved().identity,
        &super::DiagnosticClock {
            boot_id: reserved().identity.boot_id,
            monotonic_ns: 1,
        },
    )
    .unwrap();
    let (_root, _fence) = super::lock_host_root(&root).unwrap();
    let mut stale = super::read_active(&root).unwrap().unwrap();
    let mut starting = stale.clone();
    starting.stage = Stage::Starting;
    starting.cell_started_monotonic_ns = Some(1);
    starting.cleanup_confirmed = false;
    super::bump_publish(&root, &mut starting).unwrap();
    assert_eq!(starting.revision, 2);
    let path = root.join(super::ACTIVE_DIAGNOSTIC_NAME);
    let raw = std::fs::read(&path).unwrap();
    assert!(super::bump_publish(&root, &mut stale).is_err());
    assert_eq!(stale.revision, 1);
    assert_eq!(std::fs::read(path).unwrap(), raw);
    assert!(!root.join(super::PENDING_NAME).exists());
}
