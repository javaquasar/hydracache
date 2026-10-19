//! Unsent uniquely addressed original message, not dispatch authority.
use super::*;
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport as canonical;
use crate::systemd_unit::TransientUnitSpec;
use std::fs::File;
use std::num::NonZeroU32;
use std::os::fd::AsRawFd;
use zbus::message::{Message, Type};
use super::super::super::dispatch_exchange::OriginalReplyState;

#[derive(Debug, thiserror::Error)]
pub(crate) enum MessageError {
    #[error("original live manager frame refused: {0}")]
    Frame(#[from] JoinError),
    #[error("original canonical message refused: {0}")]
    Canonical(#[from] canonical::OriginalPipeTransportError),
    #[error("addressed original message binding differs")]
    Binding,
    #[error("addressed original message is refused")]
    Refused,
}
struct Inputs<'a> {
    legacy: &'a Message,
    spec: TransientUnitSpec,
    writers: [&'a File; 2],
}
fn inputs<'held>(
    frame: &'held OriginalEnrollmentFrame<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
) -> Result<Inputs<'held>, MessageError> {
    let transport = &frame.original.original.transport;
    let material = &transport.material;
    let prepared = &material.outputs.pipes.starting.original.original.prepared;
    let state = &prepared.original.original.reservation.state;
    let spec = canonical::worker(state, &material.projection)?;
    let [Some(stdout), Some(stderr)] = &material.outputs.pipes.pipes else {
        return Err(MessageError::Binding);
    };
    let legacy = transport.message.as_ref().ok_or(MessageError::Binding)?;
    canonical::inspect_message(legacy, &spec, &stdout.writer, &stderr.writer)?;
    Ok(Inputs {
        legacy,
        spec,
        writers: [&stdout.writer, &stderr.writer],
    })
}
fn names(manager: &str, client: &str) -> Result<(), MessageError> {
    if !super::super::unique(manager) || !super::super::unique(client) || manager == client {
        return Err(MessageError::Binding);
    }
    Ok(())
}
fn encode(legacy: &Message, manager: &str, client: &str) -> Result<Message, MessageError> {
    names(manager, client)?;
    let body = legacy.body();
    let body: canonical::Body<'_> = body
        .deserialize()
        .map_err(canonical::OriginalPipeTransportError::from)?;
    let build = || -> zbus::Result<Message> {
        Message::method_call(canonical::PATH, canonical::METHOD)?
            .destination(manager)?
            .interface(canonical::INTERFACE)?
            .serial(legacy.primary_header().serial_num())
            .build(&body)
    };
    Ok(build().map_err(canonical::OriginalPipeTransportError::from)?)
}
fn aliases(message: &Message) -> Result<[i32; 2], MessageError> {
    let fds = message.data().fds();
    if fds.len() != 2 {
        return Err(MessageError::Binding);
    }
    let result = [fds[0].as_raw_fd(), fds[1].as_raw_fd()];
    if result[0] == result[1] {
        return Err(MessageError::Binding);
    }
    Ok(result)
}
fn inspect(
    input: &Inputs<'_>,
    message: &Message,
    manager: &str,
    client: &str,
    pinned: [i32; 2],
    serial: NonZeroU32,
) -> Result<(), MessageError> {
    names(manager, client)?;
    canonical::inspect_message(
        input.legacy,
        &input.spec,
        input.writers[0],
        input.writers[1],
    )?;
    let h = message.header();
    if message.message_type() != Type::MethodCall
        || message.data().bytes().len() > crate::protocol::MAX_PACKET_BYTES
        || !message.primary_header().flags().is_empty()
        || message.primary_header().serial_num() != serial
        || serial != input.legacy.primary_header().serial_num()
        || h.path().map(|v| v.as_str()) != Some(canonical::PATH)
        || h.interface().map(|v| v.as_str()) != Some(canonical::INTERFACE)
        || h.member().map(|v| v.as_str()) != Some(canonical::METHOD)
        || h.destination().map(|v| v.as_str()) != Some(manager)
        || h.sender().is_some()
        || h.reply_serial().is_some()
        || h.error_name().is_some()
        || h.unix_fds() != Some(2)
        || aliases(message)? != pinned
        || message.body().signature() != input.legacy.body().signature()
        || message.body().data().bytes() != input.legacy.body().data().bytes()
    {
        return Err(MessageError::Binding);
    }
    let body = message.body();
    let (_, _, props, _): canonical::Body<'_> = body
        .deserialize()
        .map_err(canonical::OriginalPipeTransportError::from)?;
    if props.len() != input.spec.properties.len() + 2 {
        return Err(MessageError::Binding);
    }
    let streams = &props[input.spec.properties.len()..];
    canonical::descriptor(&streams[0].1, input.writers[0])?;
    canonical::descriptor(&streams[1].1, input.writers[1])?;
    let old = aliases(input.legacy)?;
    if pinned
        .iter()
        .any(|fd| old.contains(fd) || input.writers.iter().any(|file| file.as_raw_fd() == *fd))
    {
        return Err(MessageError::Binding);
    }
    Ok(())
}

#[derive(Default)]
pub(super) struct Payload {
    message: Option<Message>,
    reply: Option<OriginalReplyState>,
    pinned: Option<[i32; 2]>,
    serial: Option<NonZeroU32>,
    names: Option<(String, String)>,
    refused: bool,
}
impl Payload {
    fn check(
        &mut self,
        input: &Inputs<'_>,
        manager: &str,
        client: &str,
    ) -> Result<(), MessageError> {
        if self.refused {
            return Err(MessageError::Refused);
        }
        if self.names.as_ref().map(|(m, c)| (m.as_str(), c.as_str())) != Some((manager, client)) {
            return Err(MessageError::Binding);
        }
        let message = self.message.as_ref().ok_or(MessageError::Binding)?;
        self.reply
            .as_ref()
            .ok_or(MessageError::Binding)?
            .check(message, manager, client)
            .map_err(|_| MessageError::Binding)?;
        inspect(
            input,
            message,
            manager,
            client,
            self.pinned.ok_or(MessageError::Binding)?,
            self.serial.ok_or(MessageError::Binding)?,
        )
    }
}
pub(super) fn round(
    frame: &mut OriginalEnrollmentFrame<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    channel: &mut impl ChannelChecks,
    receiver: &mut OriginalReceiverBinding,
    pin: &ReceiverPin,
    payload: &mut Payload,
    (manager, client): (&str, &str),
    #[cfg(test)] fault: Option<&str>,
) -> Result<(), MessageError> {
    let result = (|| {
        if payload.refused || !frame.ready || frame.original.is_refused() {
            return Err(MessageError::Refused);
        }
        frame.observe().map_err(JoinError::from)?;
        channel.check(receiver, pin)?;
        channel.require_names(manager, client)?;
        frame.observe().map_err(JoinError::from)?;
        let input = inputs(frame)?;
        if payload.message.is_none() {
            if payload.pinned.is_some()
                || payload.serial.is_some()
                || payload.names.is_some()
                || payload.reply.is_some()
            {
                return Err(MessageError::Binding);
            }
            let message = encode(input.legacy, manager, client)?;
            payload.reply = Some(
                OriginalReplyState::prepare(&message, manager, client)
                    .map_err(|_| MessageError::Binding)?,
            );
            payload.pinned = Some(aliases(&message)?);
            payload.serial = Some(message.primary_header().serial_num());
            payload.names = Some((manager.into(), client.into()));
            payload.message = Some(message);
        }
        #[cfg(test)]
        if let Some(fault) = fault.filter(|fault| {
            *fault != "delivery" && !fault.starts_with("inside-") && !fault.starts_with("closing-")
        }) {
            payload.tamper(&input, fault);
        }
        payload.check(&input, manager, client)?;
        #[cfg(test)]
        if fault == Some("delivery") {
            payload.transfer_for_test(frame);
        }
        #[cfg(test)]
        if let Some(case) = fault.and_then(|case| case.strip_prefix("inside-")) {
            frame.tamper_for_test(case);
        }
        // One complete unchanged predicate set encloses the pure preparation.
        frame.revalidate().map_err(JoinError::from)?;
        channel.check(receiver, pin)?;
        channel.require_names(manager, client)?;
        #[cfg(test)]
        if let Some(case) = fault.and_then(|case| case.strip_prefix("closing-")) {
            payload.tamper(&inputs(frame)?, case);
        }
        // Closing full frame may observe drift; also recheck message after it.
        payload.check(&inputs(frame)?, manager, client)?;
        frame.observe().map_err(JoinError::from)?;
        Ok(())
    })();
    if result.is_err() {
        payload.refused = true;
        frame.original.refuse();
    }
    result
}

pub(crate) struct OriginalManagerMessage<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    // Extra writer aliases close before the unchanged original owner/custody.
    payload: Payload,
    original: OriginalManagerJoin<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    ready: bool,
}
pub(crate) struct MessageFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: Box<MessageError>,
    held: Box<OriginalManagerMessage<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl std::fmt::Debug for MessageFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl std::fmt::Debug for OriginalManagerMessage<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OriginalManagerMessage")
            .field("ready", &self.ready)
            .finish_non_exhaustive()
    }
}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalEnrollmentFrame<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn prepare_fixed_manager_message(
        self,
    ) -> Result<
        OriginalManagerMessage<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        MessageFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut held = OriginalManagerMessage {
            payload: Payload::default(),
            original: OriginalManagerJoin {
                original: self,
                channel: None,
                receiver: None,
                pin: None,
                ready: false,
            },
            ready: false,
        };
        let result = (|| {
            if !held.original.original.ready || held.original.original.original.is_refused() {
                return Err(MessageError::Refused);
            }
            held.original.original.observe().map_err(JoinError::from)?;
            held.original.receiver = Some(retain_receiver(&mut held.original.original)?);
            held.original.pin = Some(ReceiverPin::capture().map_err(|_| JoinError::Channel)?);
            held.original.channel =
                Some(FixedManagerChannel::open_fixed().map_err(|_| JoinError::Channel)?);
            held.validate()
        })();
        match result {
            Ok(()) => {
                held.original.ready = true;
                held.ready = true;
                Ok(held)
            }
            Err(error) => {
                held.refuse();
                Err(MessageFailure {
                    error: Box::new(error),
                    held: Box::new(held),
                })
            }
        }
    }
}
impl OriginalManagerMessage<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn refuse(&mut self) {
        self.ready = false;
        self.payload.refused = true;
        self.original.ready = false;
        self.original.original.original.refuse();
    }
    fn validate(&mut self) -> Result<(), MessageError> {
        let channel = self
            .original
            .channel
            .as_mut()
            .ok_or(MessageError::Refused)?;
        let manager = channel.manager.clone();
        let client = channel.client.clone();
        round(
            &mut self.original.original,
            channel,
            self.original
                .receiver
                .as_mut()
                .ok_or(MessageError::Refused)?,
            self.original.pin.as_ref().ok_or(MessageError::Refused)?,
            &mut self.payload,
            (&manager, &client),
            #[cfg(test)]
            None,
        )
    }
    pub(crate) fn revalidate(&mut self) -> Result<(), MessageError> {
        let result = if self.ready && self.original.ready {
            self.validate()
        } else {
            Err(MessageError::Refused)
        };
        if result.is_err() {
            self.refuse();
        }
        result
    }
}

#[cfg(test)]
#[path = "diagnostic_original_manager_message_units.rs"]
mod units;

#[path = "diagnostic_original_watchdog_context.rs"]
pub(super) mod watchdog_context;
