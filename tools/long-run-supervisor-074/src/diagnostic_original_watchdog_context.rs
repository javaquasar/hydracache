//! Opaque original preparation borrow; not watchdog enrollment or delivery.
use super::*;
use crate::diagnostic_lease::DiagnosticState;

#[derive(Clone, PartialEq, Eq)]
struct Source {
    owner: [i64; 6],
    state: DiagnosticState,
    origins: [u64; 4],
    bounds: [u64; 4],
    descriptors: [i32; 10],
    guardians: [i32; 2],
    names: (String, String),
    serial: NonZeroU32,
}
fn current_owner() -> [i64; 6] {
    unsafe {
        [
            i64::from(libc::getpid()),
            libc::syscall(libc::SYS_gettid),
            i64::from(libc::getuid()),
            i64::from(libc::getgid()),
            i64::from(libc::geteuid()),
            i64::from(libc::getegid()),
        ]
    }
}
fn capture(
    frame: &OriginalEnrollmentFrame<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    names: (&str, &str),
) -> Result<Source, MessageError> {
    if !frame.ready || frame.original.is_refused() {
        return Err(MessageError::Refused);
    }
    frame.observe().map_err(JoinError::from)?;
    let grouped = &frame.original.original;
    let material = &grouped.transport.material;
    let prepared = &material.outputs.pipes.starting.original.original.prepared;
    let progress = &prepared.original.original;
    let state = material.starting.as_ref().ok_or(MessageError::Binding)?;
    let bounds = grouped.bounds.ok_or(MessageError::Binding)?;
    let alarm = grouped.alarm.as_ref().ok_or(MessageError::Binding)?;
    if alarm.deadline != bounds.first() {
        return Err(MessageError::Binding);
    }
    Ok(Source {
        owner: current_owner(),
        state: state.clone(),
        origins: [
            progress.reservation.state.reserved_monotonic_ns,
            progress.packet.received_time().boottime_ns(),
            prepared.original.issued.boottime_ns(),
            state
                .cell_started_monotonic_ns
                .ok_or(MessageError::Binding)?,
        ],
        bounds: [bounds.total, bounds.cell, bounds.progress, alarm.deadline],
        descriptors: frame.inventory.as_ref().ok_or(MessageError::Binding)?.fds,
        guardians: [
            frame
                .collector
                .as_ref()
                .ok_or(MessageError::Binding)?
                .pinned
                .as_raw_fd(),
            frame
                .custodian
                .as_ref()
                .ok_or(MessageError::Binding)?
                .pinned
                .as_raw_fd(),
        ],
        names: (names.0.into(), names.1.into()),
        serial: grouped
            .transport
            .message
            .as_ref()
            .ok_or(MessageError::Binding)?
            .primary_header()
            .serial_num(),
    })
}

pub(in super::super) struct Binding {
    source: Source,
    addressed: [i32; 2],
}
pub(in super::super) fn preparation_round(
    frame: &mut OriginalEnrollmentFrame<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    channel: &mut impl ChannelChecks,
    receiver: &mut OriginalReceiverBinding,
    pin: &ReceiverPin,
    payload: &mut Payload,
    ((manager, client), retained): ((&str, &str), Option<&Binding>),
    #[cfg(test)] fault: Option<&str>,
) -> Result<Binding, MessageError> {
    let result = (|| {
        if payload.refused {
            return Err(MessageError::Refused);
        }
        let opening = capture(frame, (manager, client))?;
        if retained
            .is_some_and(|old| old.source != opening || payload.pinned != Some(old.addressed))
        {
            return Err(MessageError::Binding);
        }
        // Exactly the existing complete round, not a cached authorization result.
        super::round(
            frame,
            channel,
            receiver,
            pin,
            payload,
            (manager, client),
            #[cfg(test)]
            fault.filter(|case| !case.starts_with("closing-context-")),
        )?;
        let closing = capture(frame, (manager, client))?;
        #[cfg(test)]
        let closing = {
            let mut closing = closing;
            if let Some(kind) = fault.and_then(|case| case.strip_prefix("closing-context-")) {
                closing.tamper(kind);
            }
            closing
        };
        let addressed = payload.pinned.ok_or(MessageError::Binding)?;
        if opening != closing || retained.is_some_and(|old| old.addressed != addressed) {
            return Err(MessageError::Binding);
        }
        Ok(Binding {
            source: closing,
            addressed,
        })
    })();
    if result.is_err() {
        payload.refused = true;
        frame.original.refuse();
    }
    result
}

/// Exclusive original owner borrow; no raw inputs, FD getters or execution API.
pub(crate) struct OriginalWatchdogContext<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    original: &'held mut OriginalManagerMessage<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    binding: Binding,
}
impl OriginalWatchdogContext<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn revalidate(&mut self) -> Result<(), MessageError> {
        self.original
            .watchdog_context_round(Some(&self.binding))
            .map(|_| ())
    }
}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalManagerMessage<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn prepare_original_watchdog_context(
        &mut self,
    ) -> Result<
        OriginalWatchdogContext<'_, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        MessageError,
    > {
        let binding = self.watchdog_context_round(None)?;
        Ok(OriginalWatchdogContext {
            original: self,
            binding,
        })
    }
    fn watchdog_context_round(
        &mut self,
        retained: Option<&Binding>,
    ) -> Result<Binding, MessageError> {
        let result = (|| {
            if !self.ready || !self.original.ready {
                return Err(MessageError::Refused);
            }
            let channel = self
                .original
                .channel
                .as_mut()
                .ok_or(MessageError::Refused)?;
            let manager = channel.manager.clone();
            let client = channel.client.clone();
            preparation_round(
                &mut self.original.original,
                channel,
                self.original
                    .receiver
                    .as_mut()
                    .ok_or(MessageError::Refused)?,
                self.original.pin.as_ref().ok_or(MessageError::Refused)?,
                &mut self.payload,
                ((&manager, &client), retained),
                #[cfg(test)]
                None,
            )
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }
}

#[cfg(test)]
#[path = "diagnostic_original_watchdog_context_tests.rs"]
mod tests;
