use super::*;
use std::cell::RefCell;
use std::rc::Rc;

struct Owner {
    root: RootBindingClaims,
    refused: bool,
    fail: bool,
    drift: bool,
    rounds: usize,
    trace: Rc<RefCell<Vec<&'static str>>>,
}
impl RootOwner for Owner {
    fn full_round(&mut self) -> Result<(), PendingError> {
        self.trace.borrow_mut().push("full-frame");
        self.rounds += 1;
        if self.drift {
            self.root.original_context_sha256[0] ^= 1;
        }
        if self.fail || self.refused {
            return Err(MessageError::Refused.into());
        }
        Ok(())
    }
    fn descriptors(&self) -> Result<[BorrowedFd<'_>; 2], PendingError> {
        panic!("no FD export")
    }
    fn binding(&self, identity: [u64; 2]) -> Result<RootBindingClaims, PendingError> {
        if self.refused || identity != [self.root.root_device, self.root.root_inode] {
            return Err(MessageError::Binding.into());
        }
        Ok(self.root)
    }
    fn refuse(&mut self) {
        self.refused = true;
    }
}
impl PlacementOwner for Owner {
    fn inspect(&self, raw: &[u8], root: RootBindingClaims) -> Result<(), PendingError> {
        self.trace.borrow_mut().push("signed-constraints");
        if raw != b"already-authenticated-model" || root != self.root || self.refused {
            return Err(MessageError::Binding.into());
        }
        Ok(())
    }
}
struct Controls {
    root: RootBindingClaims,
    refused: bool,
    fail: usize,
    calls: usize,
    trace: Rc<RefCell<Vec<&'static str>>>,
}
impl Controls {
    fn check(&mut self, label: &'static str) -> Result<(), PendingError> {
        self.calls += 1;
        self.trace.borrow_mut().push(label);
        if self.refused || self.calls == self.fail {
            return Err(MessageError::Refused.into());
        }
        Ok(())
    }
}
impl RootControls for Controls {
    fn observe(&mut self) -> Result<(), PendingError> {
        self.check("root")
    }
    fn enroll_once(&mut self) -> Result<(), PendingError> {
        panic!("no reenrollment")
    }
    fn refuse(&mut self) {
        self.refused = true;
    }
}
impl PlacementControls for Controls {
    fn binding_for_placement(&mut self) -> Result<RootBindingClaims, PendingError> {
        self.check("enrollment")?;
        Ok(self.root)
    }
}
impl SliceControls for Controls {
    type Slice = [u8; 32];
    fn pin(&mut self, attempt: [u8; 32]) -> Result<Self::Slice, PendingError> {
        self.check("pin-path")?;
        if attempt != self.root.attempt_sha256 {
            return Err(MessageError::Binding.into());
        }
        Ok(attempt)
    }
    fn observe_slice(
        &mut self,
        slice: &Self::Slice,
        attempt: [u8; 32],
    ) -> Result<(), PendingError> {
        self.check("path")?;
        if slice != &attempt || attempt != self.root.attempt_sha256 {
            return Err(MessageError::Binding.into());
        }
        Ok(())
    }
}
fn fixture() -> (Owner, Controls, Vec<u8>, Snapshot) {
    let root = RootBindingClaims {
        attempt_sha256: [74; 32],
        original_context_sha256: [75; 32],
        root_device: 3,
        root_inode: 6,
        deadline_boottime_ns: 10_000_000_002,
    };
    let raw = b"already-authenticated-model".to_vec();
    let snapshot = Snapshot {
        root,
        envelope_sha256: crate::sha256_hex(&raw),
    };
    let trace = Rc::default();
    (
        Owner {
            root,
            refused: false,
            fail: false,
            drift: false,
            rounds: 0,
            trace: Rc::clone(&trace),
        },
        Controls {
            root,
            refused: false,
            fail: 0,
            calls: 0,
            trace,
        },
        raw,
        snapshot,
    )
}
#[test]
fn fixed_placement_surrounds_exact_unchanged_join_with_path_checks() {
    let (mut owner, mut controls, raw, snapshot) = fixture();
    let slice = pin(&mut owner, &mut controls, &raw, &snapshot).unwrap();
    assert_eq!(owner.rounds, 1);
    assert_eq!(
        *owner.trace.borrow(),
        [
            "pin-path",
            "path",
            "enrollment",
            "root",
            "signed-constraints",
            "full-frame",
            "signed-constraints",
            "root",
            "enrollment",
            "path"
        ]
    );
    round(&mut owner, &mut controls, &raw, &snapshot, &slice).unwrap();
    assert_eq!(owner.rounds, 2);
}
#[test]
fn fixed_placement_every_opening_closing_failure_is_sticky_and_no_retry_owner() {
    for fail in 1..=7 {
        let (mut owner, mut controls, raw, snapshot) = fixture();
        controls.fail = fail;
        assert!(
            pin(&mut owner, &mut controls, &raw, &snapshot).is_err(),
            "{fail}"
        );
        assert!(owner.refused && controls.refused);
        let rounds = owner.rounds;
        assert!(pin(&mut owner, &mut controls, &raw, &snapshot).is_err());
        assert_eq!(owner.rounds, rounds);
    }
}
#[test]
fn fixed_placement_frame_refusal_or_closing_original_context_drift_poison_both() {
    for drift in [false, true] {
        let (mut owner, mut controls, raw, snapshot) = fixture();
        owner.fail = !drift;
        owner.drift = drift;
        assert!(pin(&mut owner, &mut controls, &raw, &snapshot).is_err());
        assert!(owner.refused && controls.refused);
        assert_eq!(owner.rounds, 1);
    }
}
#[test]
fn fixed_placement_retained_attempt_envelope_and_root_cannot_be_substituted() {
    for case in 0..4 {
        let (mut owner, mut controls, mut raw, mut snapshot) = fixture();
        match case {
            0 => snapshot.root.attempt_sha256[0] ^= 1,
            1 => snapshot.root.root_inode += 1,
            2 => raw.push(b'x'),
            _ => snapshot.root.deadline_boottime_ns += 1,
        }
        assert!(
            pin(&mut owner, &mut controls, &raw, &snapshot).is_err(),
            "{case}"
        );
        assert!(owner.refused && controls.refused);
    }
}
#[test]
fn fixed_placement_constructor_consumes_whole_original_and_only_revalidates() {
    fn signature(value: OwnedPlacement<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
        if let Ok(mut joined) = value.bind_fixed_slice() {
            let _ = joined.revalidate();
        }
    }
    let _contract = signature;
}
