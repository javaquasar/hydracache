use super::*;
use crate::diagnostic_lease::{DiagnosticIdentity, DiagnosticStage, CONFIG_HASHES};
use std::cell::Cell;

fn context() -> Context {
    Context {
        state: DiagnosticState {
            identity: DiagnosticIdentity {
                lease_id: "a".repeat(64),
                boot_id: "00000000-0000-4000-8000-000000000074".into(),
                binary_sha256: "b".repeat(64),
                build_provenance_sha256: "c".repeat(64),
            },
            revision: 2,
            stage: DiagnosticStage::Starting,
            completed_cells: 0,
            reserved_monotonic_ns: 1_000_000_000,
            last_observed_monotonic_ns: 2_000_000_001,
            controller_monotonic_ns: 2_000_000_000,
            cell_started_monotonic_ns: Some(2_000_000_001),
            cgroup_inode: None,
            reason: None,
            cleanup_confirmed: false,
            promotable: false,
            admission_allowed: false,
        },
        owner: [74, 74, 1000, 1000, 1000, 1000],
        origins: [1_000_000_000, 2_000_000_000, 2_000_000_001, 2_000_000_001],
        bounds: [
            301_000_000_000,
            62_000_000_001,
            12_000_000_000,
            12_000_000_000,
        ],
        inventory: std::array::from_fn(|i| i as i32 + 20),
        custody: [40, 41],
        addressed: [42, 43],
        names: (":1.2".into(), ":1.3".into()),
        serial: NonZeroU32::new(74).unwrap(),
        root: RootBindingClaims {
            attempt_sha256: [74; 32],
            original_context_sha256: [75; 32],
            root_device: 23,
            root_inode: 74,
            deadline_boottime_ns: 12_000_000_000,
        },
        documents_sha256: std::array::from_fn(|_| "d".repeat(64)),
        host: Image {
            identity: [1, 2, 64],
            sha256: "e".repeat(64),
        },
        launcher: Image {
            identity: [1, 3, 64],
            sha256: "f".repeat(64),
        },
        worker: Image {
            identity: [2, 4, 64],
            sha256: "b".repeat(64),
        },
        config: Image {
            identity: [2, 5, 64],
            sha256: CONFIG_HASHES[0].into(),
        },
        streams: [[4, 6], [4, 7]],
        guardians: [(75, [5, 8]), (76, [5, 9])],
    }
}

struct Probe {
    context: Context,
    calls: usize,
    fail: usize,
    refused: bool,
    trace: Vec<&'static str>,
    contexts: Cell<usize>,
    drift: usize,
}
impl Probe {
    fn new() -> Self {
        Self {
            context: context(),
            calls: 0,
            fail: 0,
            refused: false,
            trace: vec![],
            contexts: Cell::new(0),
            drift: 0,
        }
    }
    fn call(&mut self, name: &'static str) -> Result<(), HeldJoinError> {
        self.calls += 1;
        self.trace.push(name);
        if self.calls == self.fail {
            Err(HeldJoinError::Binding)
        } else {
            Ok(())
        }
    }
}
impl Checks for Probe {
    fn is_refused(&self) -> bool {
        self.refused
    }
    fn frame(&mut self) -> Result<(), HeldJoinError> {
        self.call("frame")
    }
    fn trust(&mut self) -> Result<(), HeldJoinError> {
        self.call("trust")
    }
    fn roles(&mut self) -> Result<(), HeldJoinError> {
        self.call("roles")
    }
    fn refuse(&mut self) {
        self.refused = true;
    }
}
impl Owner for Probe {
    fn context(&self) -> Result<Context, HeldJoinError> {
        self.contexts.set(self.contexts.get() + 1);
        let mut context = self.context.clone();
        if self.contexts.get() == self.drift {
            context.origins[0] += 1;
        }
        Ok(context)
    }
}

#[test]
fn held_unit_projection_exact_slots_command_placement_and_hardening() {
    let c = context();
    let p = project(c.clone()).unwrap();
    let attempt = "4a".repeat(32);
    assert_eq!(p.slice, format!("system-hc074{attempt}.slice"));
    assert_eq!(p.root_path, format!("/system.slice/{}", p.slice));
    assert_eq!(p.held_path, format!("{}/{}", p.root_path, p.unit));
    assert_eq!(p.unit, cell_intent(&c.state).unwrap().unit_name);
    assert_eq!(p.launcher_argv, ["diagnostic-held-launcher-074"]);
    assert_eq!(
        p.worker_argv,
        [
            BINARY_PATH,
            "--run",
            "/proc/self/fd/5",
            &"b".repeat(64),
            SOURCE_COMMIT
        ]
    );
    assert_eq!(
        p.slots,
        [
            Slot::DedicatedEmptyInputPipe,
            Slot::OriginalStdout([4, 6]),
            Slot::OriginalStderr([4, 7]),
            Slot::AuthenticatedReleaseEndpoint,
            Slot::OriginalWorker(c.worker.clone()),
            Slot::SealedCopyOfOriginalConfig(c.config.clone()),
            Slot::OriginalCreatorPidfd(74),
            Slot::ArmedRootGuardian(75, [5, 8]),
            Slot::ArmedRootGuardian(76, [5, 9]),
            Slot::OriginalAbsoluteAlarm(12_000_000_000),
            Slot::SealedBootstrap256
        ]
    );
    let original = crate::diagnostic_unit::build_diagnostic_unit_spec(&c.state).unwrap();
    for (name, value) in &original.properties {
        if !matches!(
            *name,
            "ExecStart"
                | "StandardInput"
                | "StandardOutputFileToAppend"
                | "StandardErrorFileToAppend"
                | "Description"
                | "Slice"
                | "Environment"
                | "ReadWritePaths"
                | "WorkingDirectory"
                | "LimitFSIZE"
        ) {
            assert!(p.properties.contains(&(*name, value.clone())), "{name}");
        }
    }
    assert!(p
        .properties
        .contains(&("Environment", UnitProperty::Strings(vec![]))));
    assert!(p
        .properties
        .contains(&("ReadWritePaths", UnitProperty::Strings(vec![]))));
    assert!(p
        .properties
        .contains(&("LimitFSIZE", UnitProperty::Unsigned(0))));
    assert!(p.properties.iter().all(|(n, _)| !matches!(
        *n,
        "ExecStart" | "StandardInput" | "StandardOutputFileToAppend" | "StandardErrorFileToAppend"
    )));
}

#[test]
fn held_unit_projection_brackets_with_two_unchanged_validations_and_stable_origins() {
    let mut p = Probe::new();
    let original = p.context.clone();
    let projection = round(&mut p, None).unwrap();
    let expected = [
        "frame", "trust", "roles", "frame", "frame", "trust", "roles", "frame",
    ];
    assert_eq!(p.trace, expected.repeat(2));
    assert_eq!(p.contexts.get(), 2);
    round(&mut p, Some(&projection)).unwrap();
    assert_eq!(p.trace, expected.repeat(4));
    assert!(p.context == original);
}

#[test]
fn held_unit_projection_every_full_validation_cut_is_sticky_and_has_no_retry() {
    for fail in 1..=16 {
        let mut p = Probe::new();
        p.fail = fail;
        assert!(round(&mut p, None).is_err(), "{fail}");
        assert!(p.refused);
        let calls = p.calls;
        p.fail = 0;
        assert!(matches!(round(&mut p, None), Err(HeldJoinError::Refused)));
        assert_eq!(p.calls, calls);
    }
}

#[test]
fn held_unit_projection_closing_drift_and_retained_body_tamper_refuse() {
    let mut p = Probe::new();
    p.drift = 2;
    assert!(round(&mut p, None).is_err());
    assert!(p.refused);
    let good = project(context()).unwrap();
    for at in 0..good.properties.len() {
        let mut p = Probe::new();
        let mut bad = project(context()).unwrap();
        bad.properties[at].1 = UnitProperty::Text("replacement".into());
        assert!(round(&mut p, Some(&bad)).is_err(), "property {at}");
        assert!(p.refused);
    }
    for at in 0..11 {
        let mut p = Probe::new();
        let mut bad = project(context()).unwrap();
        bad.slots[at] = Slot::SealedBootstrap256;
        if at == 10 {
            bad.slots[at] = Slot::DedicatedEmptyInputPipe;
        }
        assert!(round(&mut p, Some(&bad)).is_err(), "slot {at}");
        assert!(p.refused);
    }
    for at in 0..7 {
        let mut p = Probe::new();
        let mut bad = project(context()).unwrap();
        match at {
            0 => bad.unit.push('x'),
            1 => bad.slice.push('x'),
            2 => bad.root_path.push('x'),
            3 => bad.held_path.push('x'),
            4 => bad.launcher_argv[0] = BINARY_PATH.into(),
            5 => bad.worker_argv[2] = "/tmp/config".into(),
            _ => bad.worker_argv[0] = "/tmp/worker".into(),
        }
        assert!(round(&mut p, Some(&bad)).is_err());
        assert!(p.refused);
    }
}

#[test]
fn held_unit_projection_seeded_original_context_substitutions_are_not_rebound() {
    let mut seed = 0x0740_51a7_u64;
    eprintln!("held unit projection seed=0x074051a7 cases=256");
    for sample in 0..256 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut p = Probe::new();
        let retained = round(&mut p, None).unwrap();
        let v = 1 + seed % 100;
        let c = &mut p.context;
        match sample % 26 {
            0 => c.owner[0] += v as i64,
            1 => c.owner[1] += v as i64,
            2 => c.origins[sample % 4] += v,
            3 => c.bounds[3] += v,
            4 => c.inventory[sample % 10] += v as i32,
            5 => c.custody[0] += v as i32,
            6 => c.addressed[1] += v as i32,
            7 => c.names.0.push('x'),
            8 => c.serial = NonZeroU32::new(99).unwrap(),
            9 => c.root.root_inode += v,
            10 => c.root.attempt_sha256[0] ^= 1,
            11 => c.root.original_context_sha256[0] ^= 1,
            12 => c.documents_sha256[sample % 5].push('x'),
            13 => c.host.identity[1] += v,
            14 => c.launcher.identity[1] += v,
            15 => c.worker.identity[1] += v,
            16 => c.config.identity[1] += v,
            17 => c.streams[0][1] += v,
            18 => c.guardians[0].0 += v as i32,
            19 => c.guardians[1].1[1] += v,
            20 => c.state.identity.boot_id.push('x'),
            21 => c.state.revision += v,
            22 => c.launcher.sha256.push('x'),
            23 => c.worker.sha256.push('x'),
            24 => c.config.sha256.push('x'),
            _ => c.root.deadline_boottime_ns += v,
        }
        assert!(round(&mut p, Some(&retained)).is_err(), "{sample}");
        assert!(p.refused);
    }
}

#[test]
fn held_unit_projection_refuses_aliases_bad_guardians_and_renewed_bounds() {
    for at in 0..7 {
        let mut c = context();
        match at {
            0 => c.host.identity = c.launcher.identity,
            1 => c.streams[1] = c.streams[0],
            2 => c.guardians[1] = c.guardians[0],
            3 => c.guardians[0].0 = 1,
            4 => c.guardians[0].0 = c.owner[0] as i32,
            5 => c.bounds[3] += 1,
            _ => c.root.deadline_boottime_ns += 1,
        }
        assert!(project(c).is_err(), "{at}");
    }
}

#[test]
fn held_unit_projection_production_api_consumes_owner_and_returns_no_spec() {
    fn signature(owner: JoinedHeldPlacement<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
        match owner.project_held_unit() {
            Ok(mut held) => {
                let _ = held.revalidate();
            }
            Err(failure) => {
                assert!(failure.is_refused());
                let _ = failure.error();
            }
        }
    }
    let _contract = signature;
}
