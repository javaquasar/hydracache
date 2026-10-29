use super::*;
use std::cell::RefCell;
use std::rc::Rc;

struct Probe {
    refused: bool,
    calls: usize,
    fail: usize,
    frames: usize,
    leaf_fault: Option<(usize, usize)>,
    drift: usize,
    revision: u64,
    retained: Option<u64>,
    origins: [u64; 3],
    bounds: [u64; 3],
    trace: Vec<&'static str>,
}
impl Probe {
    fn new() -> Self {
        Self {
            refused: false,
            calls: 0,
            fail: 0,
            frames: 0,
            leaf_fault: None,
            drift: 0,
            revision: 7,
            retained: None,
            origins: [1, 2, 3],
            bounds: [300, 60, 10],
            trace: vec![],
        }
    }
    fn call(&mut self, label: &'static str) -> Result<(), HeldJoinError> {
        if self.refused {
            return Err(HeldJoinError::Refused);
        }
        self.calls += 1;
        self.trace.push(label);
        if self.fail == self.calls {
            return Err(HeldJoinError::Binding);
        }
        Ok(())
    }
}
impl Checks for Probe {
    fn is_refused(&self) -> bool {
        self.refused
    }
    fn frame(&mut self) -> Result<(), HeldJoinError> {
        self.call("frame")?;
        self.frames += 1;
        // Modeled cuts only; the production adapter calls unchanged FixedPlacement.
        for leaf in 0..46 {
            if self.leaf_fault == Some((self.frames, leaf)) {
                return Err(HeldJoinError::Binding);
            }
        }
        Ok(())
    }
    fn trust(&mut self) -> Result<(), HeldJoinError> {
        self.call("trust")?;
        if self.drift == self.calls {
            self.revision += 1;
        }
        if self.retained.is_some_and(|old| old != self.revision) {
            return Err(HeldJoinError::Binding);
        }
        self.retained = Some(self.revision);
        Ok(())
    }
    fn roles(&mut self) -> Result<(), HeldJoinError> {
        self.call("roles")
    }
    fn refuse(&mut self) {
        self.refused = true;
    }
}

#[test]
fn consuming_join_accepts_only_one_constant_depth_pair_of_full_frames() {
    let mut probe = Probe::new();
    validate(&mut probe).unwrap();
    assert_eq!(probe.frames, 4);
    assert_eq!(
        probe.trace,
        ["frame", "trust", "roles", "frame", "frame", "trust", "roles", "frame"]
    );
}

#[test]
fn consuming_join_each_of_eight_opening_closing_cuts_is_sticky() {
    for fail in 1..=8 {
        let mut p = Probe::new();
        p.fail = fail;
        assert!(validate(&mut p).is_err(), "{fail}");
        assert!(p.refused);
        let calls = p.calls;
        p.fail = 0;
        assert!(matches!(validate(&mut p), Err(HeldJoinError::Refused)));
        assert_eq!(p.calls, calls);
    }
}
#[test]
fn consuming_join_every_modeled_original_leaf_refusal_propagates_at_all_four_frames() {
    for frame in 1..=4 {
        for leaf in 0..46 {
            let mut p = Probe::new();
            p.leaf_fault = Some((frame, leaf));
            assert!(validate(&mut p).is_err(), "{frame}/{leaf}");
            assert!(p.refused);
            assert_eq!(p.frames, frame);
        }
    }
}
#[test]
fn consuming_join_closing_or_later_authority_drift_cannot_replace_retained_original() {
    for drift in [6, 10, 14] {
        let mut p = Probe::new();
        p.drift = drift;
        if drift > 8 {
            validate(&mut p).unwrap();
        }
        assert!(validate(&mut p).is_err());
        assert!(p.refused);
        assert_eq!(p.retained, Some(7));
    }
}
#[test]
fn consuming_join_two_full_rounds_never_change_origins_or_bounds() {
    let mut p = Probe::new();
    let origins = p.origins;
    let bounds = p.bounds;
    for _ in 0..4 {
        validate(&mut p).unwrap();
        assert_eq!(p.origins, origins);
        assert_eq!(p.bounds, bounds);
    }
    assert_eq!(p.frames, 16);
}
#[test]
fn consuming_join_budget_checks_precede_any_document_copy() {
    let good = [
        b"h".as_slice(),
        b"w".as_slice(),
        b"r".as_slice(),
        b"b".as_slice(),
    ];
    let maxima = [
        MAX_POLICY_BYTES,
        MAX_POLICY_BYTES,
        MAX_RECEIPT_BYTES as usize,
        role_binding::MAX_ENVELOPE_BYTES,
    ];
    for at in 0..4 {
        for bad in [vec![], vec![0; maxima[at] + 1]] {
            let mut values = good;
            values[at] = &bad;
            assert!(Documents::copy(JoinDocuments {
                held_policy: values[0],
                worker_policy: values[1],
                receipt: values[2],
                binding: values[3]
            })
            .is_err());
        }
    }
}
#[test]
fn consuming_join_copies_accepted_documents_and_never_accepts_replacements() {
    let mut raw = b"held".to_vec();
    let owned = Documents::copy(JoinDocuments {
        held_policy: &raw,
        worker_policy: b"worker",
        receipt: b"receipt",
        binding: b"binding",
    })
    .unwrap();
    raw[0] ^= 1;
    assert_eq!(
        owned.slices(),
        [b"held".as_slice(), b"worker", b"receipt", b"binding"]
    );
}
#[test]
fn consuming_join_seeded_cross_round_substitutions_refuse_without_new_frame_origins() {
    let mut seed = 0x0074_c05eu64;
    eprintln!("consuming_held_join seed=0x0074c05e substitutions=128");
    for _ in 0..128 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut p = Probe::new();
        validate(&mut p).unwrap();
        p.revision ^= 1 + (seed % u32::MAX as u64);
        let calls = p.calls;
        assert!(validate(&mut p).is_err());
        assert!(p.refused);
        assert_eq!(p.calls, calls + 2);
        assert_eq!(p.origins, [1, 2, 3]);
    }
}
struct Tracked(Rc<RefCell<Vec<&'static str>>>, &'static str);
impl Drop for Tracked {
    fn drop(&mut self) {
        self.0.borrow_mut().push(self.1);
    }
}
struct ModelHolder {
    original: Tracked,
    roles: Tracked,
    trust: Tracked,
    probe: Probe,
}
impl Drop for ModelHolder {
    fn drop(&mut self) {
        self.probe.refuse();
        self.original
            .0
            .borrow_mut()
            .push("retire-original-and-controls");
    }
}
#[test]
fn consuming_join_failed_holder_retains_owned_fields_until_failure_is_dropped() {
    let trace = Rc::new(RefCell::new(vec![]));
    let mut held = ModelHolder {
        original: Tracked(trace.clone(), "original-drop"),
        roles: Tracked(trace.clone(), "roles-drop"),
        trust: Tracked(trace.clone(), "trust-drop"),
        probe: Probe::new(),
    };
    held.probe.fail = 3;
    assert!(validate(&mut held.probe).is_err());
    let failure = Box::new(held);
    assert!(trace.borrow().is_empty());
    assert!(failure.probe.refused);
    let _ = (&failure.roles, &failure.trust);
    drop(failure);
    assert_eq!(
        *trace.borrow(),
        [
            "retire-original-and-controls",
            "original-drop",
            "roles-drop",
            "trust-drop"
        ]
    );
}
#[test]
fn consuming_join_production_signature_moves_all_three_owners_and_failure_has_no_retry() {
    fn signature(
        fixed: FixedPlacement<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
        roles: PinnedRoles,
        trust: HeldTrustInput,
        documents: JoinDocuments<'_>,
    ) {
        match fixed.join_held_roles(roles, trust, documents) {
            Ok(mut held) => {
                let _ = held.revalidate();
            }
            Err(failure) => {
                let _ = failure.error();
                assert!(failure.is_refused());
            }
        }
    }
    let _contract = signature;
}
