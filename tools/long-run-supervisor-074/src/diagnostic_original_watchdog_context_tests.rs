use super::*;
impl Source {
    pub(super) fn tamper(&mut self, kind: &str) {
        match kind {
            "owner" => self.owner[1] += 1,
            "identity" => self.state.identity.lease_id.push('f'),
            "state" => self.state.revision += 1,
            "origin" => self.origins[0] += 1,
            "bounds" => self.bounds[0] += 1,
            "deadline" => self.bounds[3] += 1,
            "fence" => self.descriptors[0] += 1,
            "alarm" => self.descriptors[1] += 1,
            "guardian" => self.guardians[0] += 1,
            "names" => self.names.0.push('9'),
            "serial" => self.serial = NonZeroU32::new(self.serial.get() + 1).unwrap(),
            _ => panic!("unknown context fault {kind}"),
        }
    }
}
impl Binding {
    pub(in super::super::super) fn tamper_for_test(&mut self, kind: &str) {
        if kind == "alias" {
            self.addressed[0] += 1;
        } else {
            self.source.tamper(kind);
        }
    }
}
fn source() -> Source {
    use crate::diagnostic_lease::{DiagnosticIdentity, DiagnosticStage};
    Source {
        owner: current_owner(),
        state: DiagnosticState {
            identity: DiagnosticIdentity {
                lease_id: "a".repeat(64),
                boot_id: "b".into(),
                binary_sha256: "c".repeat(64),
                build_provenance_sha256: "d".repeat(64),
            },
            revision: 2,
            stage: DiagnosticStage::Starting,
            completed_cells: 0,
            reserved_monotonic_ns: 1,
            last_observed_monotonic_ns: 4,
            controller_monotonic_ns: 3,
            cell_started_monotonic_ns: Some(4),
            cgroup_inode: None,
            reason: None,
            cleanup_confirmed: true,
            promotable: false,
            admission_allowed: false,
        },
        origins: [1, 2, 3, 4],
        bounds: [301, 64, 12, 12],
        descriptors: std::array::from_fn(|i| i as i32 + 3),
        guardians: [30, 31],
        names: (":1.74".into(), ":1.75".into()),
        serial: NonZeroU32::new(7).unwrap(),
    }
}
#[test]
fn original_watchdog_context_source_rejects_each_role_origin_bound_and_identity_mutation() {
    let original = source();
    for kind in [
        "owner", "identity", "state", "origin", "bounds", "deadline", "fence", "alarm", "guardian",
        "names", "serial",
    ] {
        let mut changed = original.clone();
        changed.tamper(kind);
        assert!(changed != original, "{kind}");
    }
    for index in 0..10 {
        let mut changed = original.clone();
        changed.descriptors[index] += 1;
        assert!(changed != original);
    }
    for index in 0..4 {
        let mut changed = original.clone();
        changed.origins[index] += 1;
        assert!(changed != original);
        let mut changed = original.clone();
        changed.bounds[index] += 1;
        assert!(changed != original);
    }
}
#[test]
fn original_watchdog_context_owner_is_actual_process_and_thread_not_a_supplied_claim() {
    let owner = current_owner();
    assert_eq!(owner[0], i64::from(std::process::id()));
    let other = std::thread::spawn(current_owner).join().unwrap();
    assert_eq!(owner[0], other[0]);
    assert_ne!(owner[1], other[1]);
    assert_eq!(&owner[2..], &other[2..]);
}
#[test]
fn original_watchdog_context_retained_alias_drift_does_not_rewrite_its_source() {
    let mut binding = Binding {
        source: source(),
        addressed: [70, 71],
    };
    let before = binding.source.clone();
    binding.tamper_for_test("alias");
    assert_eq!(binding.addressed, [71, 71]);
    assert!(binding.source == before);
}

#[test]
fn original_watchdog_context_constructor_requires_an_exclusive_original_owner() {
    fn signature(owner: &mut OriginalManagerMessage<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
        if let Ok(mut context) = owner.prepare_original_watchdog_context() {
            let _ = context.revalidate();
        }
    }
    let _contract = signature;
}
