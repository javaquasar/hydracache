use super::super::pending::PendingError;
use super::super::root_guardians::{derive, join, RootControls, RootOwner};
use super::*;
use std::os::fd::BorrowedFd;

#[test]
fn original_root_guardian_binding_covers_every_original_role_and_keeps_deadline() {
    let binding = || Binding {
        source: source(),
        addressed: [70, 71],
    };
    let original = binding();
    let expected = derive(&original, [40, 41]).unwrap();
    assert_eq!(expected.deadline_boottime_ns, original.source.bounds[3]);
    for kind in [
        "owner", "identity", "state", "origin", "bounds", "deadline", "fence", "alarm", "guardian",
        "names", "serial", "alias",
    ] {
        let mut changed = binding();
        changed.tamper_for_test(kind);
        assert_ne!(
            derive(&changed, [40, 41]).unwrap().original_context_sha256,
            expected.original_context_sha256,
            "{kind}"
        );
    }
    for index in 0..10 {
        let mut changed = binding();
        changed.source.descriptors[index] += 1;
        assert_ne!(derive(&changed, [40, 41]).unwrap(), expected);
    }
    for index in 0..4 {
        let mut changed = binding();
        changed.source.origins[index] += 1;
        assert_ne!(derive(&changed, [40, 41]).unwrap(), expected);
        let mut changed = binding();
        changed.source.bounds[index] += 1;
        assert_ne!(derive(&changed, [40, 41]).unwrap(), expected);
    }
    assert_ne!(derive(&original, [40, 42]).unwrap(), expected);
    assert!(derive(&original, [0, 41]).is_err());
    assert!(derive(&original, [40, 0]).is_err());
}
struct Owner {
    refused: bool,
    rounds: u8,
    fail: u8,
}
impl RootOwner for Owner {
    fn full_round(&mut self) -> Result<(), PendingError> {
        self.rounds += 1;
        if self.refused || self.rounds == self.fail {
            Err(MessageError::Refused.into())
        } else {
            Ok(())
        }
    }
    fn descriptors(&self) -> Result<[BorrowedFd<'_>; 2], PendingError> {
        panic!("join exports no descriptors")
    }
    fn binding(
        &self,
        _: [u64; 2],
    ) -> Result<crate::diagnostic_two_phase_launch::root_enrollment::RootBindingClaims, PendingError>
    {
        panic!("join never replaces retained binding")
    }
    fn refuse(&mut self) {
        self.refused = true;
    }
}
struct Controls {
    refused: bool,
    consumed: bool,
    observes: u8,
    fail: u8,
}
impl RootControls for Controls {
    fn observe(&mut self) -> Result<(), PendingError> {
        self.observes += 1;
        if self.refused || self.observes == self.fail {
            Err(MessageError::Refused.into())
        } else {
            Ok(())
        }
    }
    fn enroll_once(&mut self) -> Result<(), PendingError> {
        if self.consumed {
            return Err(MessageError::Refused.into());
        }
        self.consumed = true;
        Ok(())
    }
    fn refuse(&mut self) {
        self.refused = true;
    }
}
#[test]
fn original_root_guardian_join_poisoning_is_sticky_on_every_boundary() {
    for (owner_fault, kernel_fault) in [(0, 0), (1, 0), (2, 0), (0, 1), (0, 2), (0, 3)] {
        let mut owner = Owner {
            refused: false,
            rounds: 0,
            fail: owner_fault,
        };
        let mut controls = Controls {
            refused: false,
            consumed: false,
            observes: 0,
            fail: kernel_fault,
        };
        let result = join(&mut owner, &mut controls, true);
        let healthy = owner_fault == 0 && kernel_fault == 0;
        assert_eq!(result.is_ok(), healthy);
        if healthy {
            assert_eq!(owner.rounds, 2);
            assert!(controls.consumed);
            assert!(join(&mut owner, &mut controls, true).is_err());
        }
        assert!(owner.refused && controls.refused);
        let before = owner.rounds;
        assert!(join(&mut owner, &mut controls, true).is_err());
        assert_eq!(owner.rounds, before);
    }
}
#[test]
fn original_root_guardian_constructor_requires_owned_original_context_and_root_borrow() {
    fn signature(
        context: OriginalWatchdogContext<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
        root: &mut crate::diagnostic_two_phase_launch::root_enrollment::RootSource,
    ) {
        if let Ok(mut controls) = context.prepare_root_guardians(root) {
            let _ = controls.revalidate();
            let _ = controls.enroll_once();
        }
    }
    let _contract = signature;
}
