//! Same retirement callback as real wrapper Drop; no constructor/host claim.
use super::*;
use std::cell::RefCell;
use std::rc::Rc;
type Trace = Rc<RefCell<Vec<&'static str>>>;
struct Owner {
    refused: bool,
    rounds: u8,
    trace: Trace,
}
impl RootOwner for Owner {
    fn full_round(&mut self) -> Result<(), PendingError> {
        if self.refused {
            return Err(MessageError::Refused.into());
        }
        self.rounds += 1;
        Ok(())
    }
    fn descriptors(&self) -> Result<[BorrowedFd<'_>; 2], PendingError> {
        panic!("retirement exports no FD")
    }
    fn binding(&self, _: [u64; 2]) -> Result<RootBindingClaims, PendingError> {
        panic!("no replacement binding")
    }
    fn refuse(&mut self) {
        self.trace.borrow_mut().push("owner");
        self.refused = true;
    }
}
struct Controls {
    refused: bool,
    consumed: bool,
    trace: Trace,
}
impl RootControls for Controls {
    fn observe(&mut self) -> Result<(), PendingError> {
        if self.refused {
            Err(MessageError::Refused.into())
        } else {
            Ok(())
        }
    }
    fn enroll_once(&mut self) -> Result<(), PendingError> {
        panic!("retirement must never enroll")
    }
    fn refuse(&mut self) {
        self.trace.borrow_mut().push("kernel");
        self.refused = true;
    }
}
#[test]
fn original_root_retirement_refuses_owner_first_and_cannot_retry_pending_or_consumed() {
    for consumed in [false, true] {
        let trace = Trace::default();
        let mut owner = Owner {
            refused: false,
            rounds: 0,
            trace: trace.clone(),
        };
        let mut controls = Controls {
            refused: false,
            consumed,
            trace: trace.clone(),
        };
        retire(&mut owner, &mut controls);
        assert_eq!(&*trace.borrow(), &["owner", "kernel"]);
        assert!(owner.refused && controls.refused);
        assert_eq!(controls.consumed, consumed);
        assert!(join(&mut owner, &mut controls, true).is_err());
        assert_eq!(owner.rounds, 0);
        assert_eq!(controls.consumed, consumed);
    }
}
