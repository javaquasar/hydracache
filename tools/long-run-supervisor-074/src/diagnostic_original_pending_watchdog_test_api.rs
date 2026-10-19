//! Distinct test-channel probe; cannot construct an original Pending owner.
use super::*;
use crate::diagnostic_two_phase_launch::InheritedPending;
use std::os::fd::BorrowedFd;
use watchdog_context::pending::{self, PendingError, PendingOwner};

pub(crate) struct PendingProbe<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    frame: &'held mut OriginalEnrollmentFrame<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    probe: WatchdogProbe,
    case: String,
    rounds: u8,
    calls: u8,
}
impl PendingOwner for PendingProbe<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn full_round(&mut self) -> Result<(), PendingError> {
        let kind = match (self.case.as_str(), self.rounds) {
            ("retained-origin", 0) | ("closing-origin", 1) => "retained-origin",
            ("inside-custody-missing", 1) => "inside-custody-missing",
            _ => "ok",
        };
        self.rounds += 1;
        let (result, calls) = self.frame.watchdog_round_for_test(&mut self.probe, kind);
        self.calls += calls;
        result.map_err(PendingError::from)
    }
    fn descriptors(&self) -> Result<[BorrowedFd<'_>; 2], PendingError> {
        pending::original_descriptors(self.frame, self.probe.binding.as_ref().unwrap())
    }
    fn refuse(&mut self) {
        self.frame.original.refuse();
    }
}
impl PendingProbe<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn prepare_for_test(&mut self) -> (Result<InheritedPending, PendingError>, u8) {
        self.calls = 0;
        let fault = match self.case.as_str() {
            "clone-first" => 1,
            "clone-second" => 2,
            "restore-first" => 3,
            "restore-second" => 4,
            "guardian-first-dead" => 5,
            "guardian-second-dead" => 6,
            _ => 0,
        };
        let result = pending::prepare_pending(self, fault);
        (result, self.calls)
    }
    pub(crate) fn revalidate_for_test(
        &mut self,
        held: &InheritedPending,
    ) -> (Result<(), PendingError>, u8) {
        self.calls = 0;
        let result = pending::revalidate_pending(self, held);
        (result, self.calls)
    }
    pub(crate) fn retry_for_test(&mut self) -> (Result<(), PendingError>, u8) {
        self.calls = 0;
        let result = self.full_round();
        (result, self.calls)
    }
}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalEnrollmentFrame<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn pending_probe_for_test(
        &mut self,
        case: &str,
    ) -> PendingProbe<'_, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
        let mut probe = self.watchdog_probe_for_test();
        self.watchdog_round_for_test(&mut probe, "ok").0.unwrap();
        PendingProbe {
            frame: self,
            probe,
            case: case.into(),
            rounds: 0,
            calls: 0,
        }
    }
}

#[path = "diagnostic_original_root_guardian_test_api.rs"]
mod root_guardians;
