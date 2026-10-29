//! Original Pending custody only; no target, delivery, Armed or completion.
use super::*;
use crate::diagnostic_two_phase_launch::InheritedPending;
use canonical::grouped_preparation::custody::OriginalCustodyError;
use std::os::fd::{AsFd, BorrowedFd};

#[derive(Debug, thiserror::Error)]
pub(crate) enum PendingError {
    #[error("original context refused: {0}")]
    Context(#[from] MessageError),
    #[error("pending watchdog mechanics refused: {0}")]
    Kernel(#[from] std::io::Error),
}
// Sealed here. A test probe can exercise the same join, but cannot manufacture
// OriginalWatchdogContext or OriginalPendingWatchdogs.
pub(in super::super::super) trait PendingOwner {
    fn full_round(&mut self) -> Result<(), PendingError>;
    fn descriptors(&self) -> Result<[BorrowedFd<'_>; 2], PendingError>;
    fn refuse(&mut self);
}
pub(in super::super::super) fn prepare_pending(
    original: &mut impl PendingOwner,
    #[cfg(test)] fault: u8,
) -> Result<InheritedPending, PendingError> {
    let result = (|| {
        original.full_round()?;
        let [fence, alarm] = original.descriptors()?;
        #[allow(unused_mut)]
        let mut pending = InheritedPending::spawn(
            fence,
            alarm,
            #[cfg(test)]
            fault,
        )?;
        #[cfg(test)]
        if fault == 5 || fault == 6 {
            pending.kill_for_test((fault - 5) as usize);
        }
        pending.observe()?;
        original.full_round()?;
        pending.observe()?;
        Ok(pending)
    })();
    if result.is_err() {
        original.refuse();
    }
    result
}
pub(in super::super::super) fn revalidate_pending(
    original: &mut impl PendingOwner,
    pending: &InheritedPending,
) -> Result<(), PendingError> {
    let result = (|| {
        pending.observe()?;
        original.full_round()?;
        pending.observe()?;
        Ok(())
    })();
    if result.is_err() {
        original.refuse();
    }
    result
}
pub(in super::super::super) fn original_descriptors<'held>(
    frame: &'held OriginalEnrollmentFrame<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    binding: &Binding,
) -> Result<[BorrowedFd<'held>; 2], PendingError> {
    let grouped = &frame.original.original;
    let custody = frame
        .original
        .custody
        .as_ref()
        .ok_or(MessageError::Binding)?;
    let outputs = &grouped.transport.material.outputs;
    let fence = custody
        .original_descriptor(outputs.reservation())
        .map_err(OriginalCustodyError::from)
        .map_err(BarredCustodyError::from)
        .map_err(JoinError::from)
        .map_err(MessageError::from)?;
    let alarm = &grouped.alarm.as_ref().ok_or(MessageError::Binding)?.file;
    if [fence.as_raw_fd(), alarm.as_raw_fd()] != binding.source.descriptors[..2] {
        return Err(MessageError::Binding.into());
    }
    Ok([fence, alarm.as_fd()])
}
impl PendingOwner for OriginalWatchdogContext<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn full_round(&mut self) -> Result<(), PendingError> {
        self.revalidate().map_err(PendingError::from)
    }
    fn descriptors(&self) -> Result<[BorrowedFd<'_>; 2], PendingError> {
        original_descriptors(&self.original.original.original, &self.binding)
    }
    fn refuse(&mut self) {
        self.original.refuse();
    }
}

pub(crate) struct OriginalPendingWatchdogs<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    // Cancel only the new Pending children before returning the original borrow.
    pending: InheritedPending,
    original: OriginalWatchdogContext<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
}
pub(crate) struct PendingFailure<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: Box<PendingError>,
    held: Box<OriginalWatchdogContext<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl std::fmt::Debug for PendingFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl OriginalPendingWatchdogs<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn revalidate(&mut self) -> Result<(), PendingError> {
        revalidate_pending(&mut self.original, &self.pending)
    }
}
impl<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalWatchdogContext<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn prepare_pending_watchdogs(
        mut self,
    ) -> Result<
        OriginalPendingWatchdogs<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        PendingFailure<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        match prepare_pending(
            &mut self,
            #[cfg(test)]
            0,
        ) {
            Ok(pending) => Ok(OriginalPendingWatchdogs {
                pending,
                original: self,
            }),
            Err(error) => Err(PendingFailure {
                error: Box::new(error),
                held: Box::new(self),
            }),
        }
    }
}

#[cfg(test)]
#[test]
fn original_pending_watchdog_constructor_consumes_only_original_borrow() {
    fn signature(context: OriginalWatchdogContext<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
        if let Ok(mut pending) = context.prepare_pending_watchdogs() {
            let _ = pending.revalidate();
        }
    }
    let _contract = signature;
}
