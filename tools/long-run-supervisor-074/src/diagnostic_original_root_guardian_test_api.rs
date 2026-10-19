//! Distinct simulated kernel, same original complete frame and join algorithm.
use super::*;
use crate::diagnostic_two_phase_launch::root_enrollment::RootBindingClaims;
use watchdog_context::root_guardians::{self, RootControls, RootOwner};

impl RootOwner for PendingProbe<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn full_round(&mut self) -> Result<(), PendingError> {
        PendingOwner::full_round(self)
    }
    fn descriptors(&self) -> Result<[BorrowedFd<'_>; 2], PendingError> {
        PendingOwner::descriptors(self)
    }
    fn binding(&self, _: [u64; 2]) -> Result<RootBindingClaims, PendingError> {
        panic!("frame probe never constructs a root binding or original capability")
    }
    fn refuse(&mut self) {
        PendingOwner::refuse(self);
    }
}
pub(crate) struct RootProbeControls {
    observations: u8,
    sends: u8,
    refused: bool,
    fault: u8,
}
impl RootControls for RootProbeControls {
    fn observe(&mut self) -> Result<(), PendingError> {
        self.observations += 1;
        if self.refused || self.observations == self.fault {
            Err(MessageError::Refused.into())
        } else {
            Ok(())
        }
    }
    fn enroll_once(&mut self) -> Result<(), PendingError> {
        if self.sends != 0 {
            return Err(MessageError::Refused.into());
        }
        self.sends = 1; // consume before simulated syscall, even its failure
        if self.fault == 4 {
            Err(MessageError::Refused.into())
        } else {
            Ok(())
        }
    }
    fn refuse(&mut self) {
        self.refused = true;
    }
}
impl PendingProbe<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn root_controls_for_test(&self, fault: u8) -> RootProbeControls {
        RootProbeControls {
            observations: 0,
            sends: 0,
            refused: false,
            fault,
        }
    }
    pub(crate) fn root_join_for_test(
        &mut self,
        controls: &mut RootProbeControls,
        enroll: bool,
    ) -> (Result<(), PendingError>, u8) {
        self.calls = 0;
        let result = root_guardians::join(self, controls, enroll);
        (result, self.calls)
    }
}
