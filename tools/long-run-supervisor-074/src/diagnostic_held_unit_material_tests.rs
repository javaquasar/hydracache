use super::*;
use crate::diagnostic_lease::{DiagnosticIdentity, DiagnosticStage};
use std::cell::{Cell, RefCell};

fn inventory() -> Inventory {
    let image = || Image {
        identity: [1, 2, 3],
        sha256: "a".repeat(64),
    };
    Inventory {
        descriptors: vec![],
        roles: kernel::RoleBindings([(0, 0); 8]),
        context: Context {
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
                reserved_monotonic_ns: 1,
                last_observed_monotonic_ns: 2,
                controller_monotonic_ns: 2,
                cell_started_monotonic_ns: Some(2),
                cgroup_inode: None,
                reason: None,
                cleanup_confirmed: false,
                promotable: false,
                admission_allowed: false,
            },
            owner: [74, 74, 1000, 1000, 1000, 1000],
            origins: [1, 2, 3, 4],
            bounds: [300, 60, 10, 10],
            inventory: std::array::from_fn(|i| i as i32 + 20),
            custody: [40, 41],
            addressed: [42, 43],
            names: (":1.2".into(), ":1.3".into()),
            serial: NonZeroU32::new(74).unwrap(),
            root: RootBindingClaims {
                attempt_sha256: [74; 32],
                original_context_sha256: [75; 32],
                root_device: 1,
                root_inode: 2,
                deadline_boottime_ns: 10,
            },
            documents_sha256: std::array::from_fn(|_| "d".repeat(64)),
            host: image(),
            launcher: image(),
            worker: image(),
            config: image(),
            streams: [[4, 5], [4, 6]],
            guardians: [(75, [5, 6]), (76, [5, 7])],
        },
    }
}
struct Probe {
    calls: Cell<usize>,
    fail: usize,
    trace: RefCell<Vec<&'static str>>,
    refused: bool,
    retained: Option<Inventory>,
    drift: usize,
    role_drift: bool,
}
impl Probe {
    fn new() -> Self {
        Self {
            calls: Cell::new(0),
            fail: 0,
            trace: RefCell::new(vec![]),
            refused: false,
            retained: None,
            drift: 0,
            role_drift: false,
        }
    }
    fn call(&self, name: &'static str) -> Result<(), MaterialError> {
        self.calls.set(self.calls.get() + 1);
        self.trace.borrow_mut().push(name);
        if self.fail == self.calls.get() {
            Err(MaterialError::Binding)
        } else {
            Ok(())
        }
    }
}
impl MaterialOwner for Probe {
    fn refused(&self) -> bool {
        self.refused
    }
    fn full_frame(&mut self) -> Result<(), MaterialError> {
        self.call("full-frame")
    }
    fn prepare(&mut self) -> Result<(), MaterialError> {
        self.call("prepare")
    }
    fn inspect(&self) -> Result<Inventory, MaterialError> {
        self.call("expanded-inventory")?;
        let mut got = inventory();
        if self.calls.get() == self.drift {
            if self.role_drift {
                got.roles.0[0].1 += 1;
            } else {
                got.context.origins[0] += 1;
            }
        }
        Ok(got)
    }
    fn retained(&self) -> Option<&Inventory> {
        self.retained.as_ref()
    }
    fn refuse(&mut self) {
        self.refused = true;
    }
}

#[test]
fn held_material_join_brackets_effects_and_extended_inventory_with_full_unchanged_frames() {
    let mut p = Probe::new();
    let result = validate_material(&mut p).unwrap();
    assert_eq!(
        *p.trace.borrow(),
        [
            "full-frame",
            "prepare",
            "expanded-inventory",
            "full-frame",
            "expanded-inventory"
        ]
    );
    assert_eq!(result.context.origins, [1, 2, 3, 4]);
    assert_eq!(result.context.bounds, [300, 60, 10, 10]);
    p.retained = Some(result);
    validate_material(&mut p).unwrap();
    assert_eq!(p.calls.get(), 10);
}
#[test]
fn held_material_join_each_effect_and_validation_cut_is_sticky_without_retry() {
    for fail in 1..=5 {
        let mut p = Probe::new();
        p.fail = fail;
        assert!(validate_material(&mut p).is_err(), "{fail}");
        assert!(p.refused);
        let calls = p.calls.get();
        p.fail = 0;
        assert!(matches!(
            validate_material(&mut p),
            Err(MaterialError::Refused)
        ));
        assert_eq!(p.calls.get(), calls);
    }
}
#[test]
fn held_material_join_closing_and_retained_original_substitutions_refuse() {
    let mut p = Probe::new();
    p.drift = 5;
    assert!(validate_material(&mut p).is_err());
    assert!(p.refused);
    for at in 0..13 {
        let mut p = Probe::new();
        let mut old = inventory();
        match at {
            0 => old.context.origins[0] += 1,
            1 => old.context.bounds[3] += 1,
            2 => old.context.inventory[0] += 1,
            3 => old.context.addressed[0] += 1,
            4 => old.context.root.root_inode += 1,
            5 => old.context.guardians[0].0 += 1,
            6 => old.context.documents_sha256[0].push('x'),
            7 => old.context.state.identity.boot_id.push('x'),
            8 => old.context.config.identity[1] += 1,
            9 => old.context.names.0.push('0'),
            10 => old.context.names.1.push('0'),
            11 => old.context.serial = NonZeroU32::new(75).unwrap(),
            _ => old.roles.0[0].1 += 1,
        }
        p.retained = Some(old);
        assert!(validate_material(&mut p).is_err());
        assert!(p.refused);
        assert_eq!(p.calls.get(), 3);
    }
}
#[test]
fn held_material_roles_closing_substitution_retires_the_same_owner_without_retry() {
    let mut p = Probe::new();
    p.drift = 5;
    p.role_drift = true;
    assert!(validate_material(&mut p).is_err());
    assert!(p.refused);
    assert_eq!(p.calls.get(), 5);
    p.drift = 0;
    assert!(matches!(
        validate_material(&mut p),
        Err(MaterialError::Refused)
    ));
    assert_eq!(p.calls.get(), 5);
}
#[test]
fn held_material_join_api_consumes_projected_owner_without_transport_or_recovery() {
    fn signature(owner: ProjectedHeldUnit<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
        match owner.materialize_held_unit() {
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
