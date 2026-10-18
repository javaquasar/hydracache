//! Original readers/sinks held by a barred child, never an execution permit.
use super::*;
#[path = "diagnostic_original_barred_collector_kernel.rs"]
mod kernel;
use kernel::KernelCollector;

pub(crate) struct OriginalBarredCollector<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    // Exact-stop/reap collector while original ownership and custodian persist.
    collector: Option<KernelCollector>,
    custody: OriginalBarredCustody<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    ready: bool,
}
pub(crate) struct BarredCollectorFailure<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: BarredCustodyError,
    held: Box<OriginalBarredCollector<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl std::fmt::Debug for OriginalBarredCollector<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OriginalBarredCollector")
            .field("refused", &self.custody.is_refused())
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for BarredCollectorFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl OriginalBarredCollector<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn validate(&mut self) -> Result<(), BarredCustodyError> {
        let result = (|| {
            if self.custody.is_refused() {
                return Err(BarredCustodyError::Refused);
            }
            let child = self.collector.as_ref().ok_or(BarredCustodyError::Child)?;
            let alarm = self
                .custody
                .inventory
                .as_ref()
                .ok_or(BarredCustodyError::Child)?
                .fds[1];
            child.alive()?;
            child.observe(alarm, false)?;
            // One complete unchanged frame, including original inventory and
            // custodian observation. No descriptor-plan recursive constructor.
            self.custody.validate()?;
            child.alive()?;
            child.observe(alarm, false)?;
            self.custody.original.inspect_deadline()?;
            Ok(())
        })();
        if result.is_err() {
            self.custody.original.refuse();
        }
        result
    }
    pub(crate) fn revalidate(&mut self) -> Result<(), BarredCustodyError> {
        if !self.ready {
            self.custody.original.refuse();
            return Err(BarredCustodyError::Refused);
        }
        self.validate()
    }
}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalCustodiedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn prepare_barred_collector_original(
        &mut self,
    ) -> Result<
        OriginalBarredCollector<'_, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        BarredCollectorFailure<'_, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        self.prepare_barred_collector_with(
            false,
            #[cfg(test)]
            0,
            #[cfg(test)]
            |_| {},
        )
    }
    fn prepare_barred_collector_with<'held>(
        &'held mut self,
        allow_fixture: bool,
        #[cfg(test)] fault: u8,
        #[cfg(test)] checkpoint: impl FnOnce(
            &mut OriginalBarredCollector<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        ),
    ) -> Result<
        OriginalBarredCollector<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        BarredCollectorFailure<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut held = OriginalBarredCollector {
            collector: None,
            custody: OriginalBarredCustody {
                kernel: None,
                original: self,
                inventory: None,
                allow_fixture,
                ready: false,
            },
            ready: false,
        };
        let result = (|| {
            if held.custody.is_refused() || !held.custody.original.ready {
                return Err(BarredCustodyError::Refused);
            }
            held.custody.inventory = Some(capture(held.custody.original)?);
            held.custody.original.inspect_deadline()?;
            let inventory = held
                .custody
                .inventory
                .as_ref()
                .ok_or(BarredCustodyError::Child)?;
            held.custody.kernel = Some(KernelCustodian::spawn(
                [inventory.fds[0], inventory.fds[1]],
                #[cfg(test)]
                0,
            )?);
            held.custody
                .kernel
                .as_ref()
                .ok_or(BarredCustodyError::Child)?
                .observe(inventory.fds[1], true)?;
            held.collector = Some(KernelCollector::spawn(
                inventory.fds[..6]
                    .try_into()
                    .map_err(|_| BarredCustodyError::Child)?,
                #[cfg(test)]
                fault,
            )?);
            held.collector
                .as_ref()
                .ok_or(BarredCustodyError::Child)?
                .observe(inventory.fds[1], true)?;
            #[cfg(test)]
            checkpoint(&mut held);
            held.validate()?;
            held.custody.ready = true;
            held.ready = true;
            Ok(())
        })();
        match result {
            Ok(()) => Ok(held),
            Err(error) => {
                held.custody.original.refuse();
                Err(BarredCollectorFailure {
                    error,
                    held: Box::new(held),
                })
            }
        }
    }
}

#[cfg(test)]
#[path = "diagnostic_original_barred_collector_test_api.rs"]
mod test_api;

#[path = "diagnostic_original_enrollment_frame.rs"]
mod enrollment_frame;
