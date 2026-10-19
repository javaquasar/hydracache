//! Distinct test channel only; never constructs a production message owner.
use super::super::message_owner::{self, MessageError, Payload};
use super::*;
pub(crate) struct MessageProbe {
    payload: Payload,
    receiver: OriginalReceiverBinding,
    pin: ReceiverPin,
    channel: TestChannel,
}

#[path = "diagnostic_original_watchdog_context_test_api.rs"]
mod watchdog;
impl OriginalEnrollmentFrame<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn message_probe_for_test(&mut self) -> MessageProbe {
        MessageProbe {
            payload: Payload::default(),
            receiver: retain_receiver(self).unwrap(),
            pin: ReceiverPin::capture().unwrap(),
            channel: TestChannel::new("ok"),
        }
    }
    pub(crate) fn message_round_for_test(
        &mut self,
        probe: &mut MessageProbe,
        case: &str,
    ) -> (Result<(), MessageError>, u8) {
        probe.channel.calls = 0;
        probe.channel.case = case.into();
        let fault = if [
            "alias", "client", "flags-fd", "body", "swapped", "serial", "missing", "delivery",
        ]
        .contains(&case)
            || case.starts_with("inside-")
            || case.starts_with("reply-")
            || case.starts_with("closing-reply-")
        {
            Some(case)
        } else {
            None
        };
        let result = message_owner::round(
            self,
            &mut probe.channel,
            &mut probe.receiver,
            &probe.pin,
            &mut probe.payload,
            (":1.74", ":1.75"),
            fault,
        );
        (result, probe.channel.calls)
    }
}
