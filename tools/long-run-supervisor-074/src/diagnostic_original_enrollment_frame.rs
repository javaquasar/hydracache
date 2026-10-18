//! Approved experiment: custody and barred enrollment inside one full frame.
use super::*;
#[path = "diagnostic_original_enrollment_frame_build.rs"]
mod build;

pub(crate) struct OriginalEnrollmentFrame<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    // Cancellation/reaping precedes original writers and the last fence alias.
    collector: Option<KernelCollector>,
    custodian: Option<KernelCustodian>,
    original: OriginalCustodiedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    inventory: Option<Inventory>,
    allow_fixture: bool,
    ready: bool,
}
pub(crate) struct EnrollmentFrameFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: BarredCustodyError,
    held: Box<OriginalEnrollmentFrame<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl std::fmt::Debug for OriginalEnrollmentFrame<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OriginalEnrollmentFrame")
            .field("refused", &self.original.is_refused())
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for EnrollmentFrameFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl OriginalEnrollmentFrame<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn observe(&self) -> Result<(), BarredCustodyError> {
        let inventory = self.inventory.as_ref().ok_or(BarredCustodyError::Child)?;
        let collector = self.collector.as_ref().ok_or(BarredCustodyError::Child)?;
        let custodian = self.custodian.as_ref().ok_or(BarredCustodyError::Child)?;
        self.original.inspect_custody()?;
        self.original.inspect_deadline()?;
        collector.alive()?;
        collector.observe(inventory.fds[1], false)?;
        custodian.alive()?;
        custodian.observe(inventory.fds[1], false)?;
        if capture(&self.original)? != *inventory {
            return Err(OriginalCustodyError::Binding.into());
        }
        self.original.inspect_deadline()?;
        Ok(())
    }
    pub(crate) fn revalidate(&mut self) -> Result<(), BarredCustodyError> {
        let result = (|| {
            if !self.ready || self.original.is_refused() {
                return Err(BarredCustodyError::Refused);
            }
            self.observe()?;
            self.original.validate(self.allow_fixture)?;
            self.observe()
        })();
        if result.is_err() {
            self.original.refuse();
        }
        result
    }
    fn enroll(&mut self, #[cfg(test)] fault: u8) -> Result<(), BarredCustodyError> {
        self.original.inspect_custody()?;
        self.original.inspect_deadline()?;
        self.inventory = Some(capture(&self.original)?);
        let inventory = self.inventory.as_ref().ok_or(BarredCustodyError::Child)?;
        self.custodian = Some(KernelCustodian::spawn(
            [inventory.fds[0], inventory.fds[1]],
            #[cfg(test)]
            0,
        )?);
        self.custodian
            .as_ref()
            .unwrap()
            .observe(inventory.fds[1], true)?;
        self.collector = Some(KernelCollector::spawn(
            inventory.fds[..6]
                .try_into()
                .map_err(|_| BarredCustodyError::Child)?,
            #[cfg(test)]
            fault,
        )?);
        #[cfg(test)]
        if fault == 3 {
            // Lose the actual initial B; the unchanged observer must refuse.
            self.collector
                .as_ref()
                .unwrap()
                .observe(inventory.fds[1], true)?;
        }
        self.collector
            .as_ref()
            .unwrap()
            .observe(inventory.fds[1], true)?;
        self.observe()
    }
}

#[cfg(test)]
#[path = "diagnostic_original_enrollment_frame_test_api.rs"]
mod test_api;
