//! Test metadata probe only; never constructs OriginalWatchdogContext.
use super::super::super::message_owner::watchdog_context::{self, Binding};
use super::*;

pub(crate) struct WatchdogProbe {
    message: MessageProbe,
    binding: Option<Binding>,
}
impl OriginalEnrollmentFrame<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn watchdog_probe_for_test(&mut self) -> WatchdogProbe {
        WatchdogProbe {
            message: self.message_probe_for_test(),
            binding: None,
        }
    }
    pub(crate) fn watchdog_round_for_test(
        &mut self,
        probe: &mut WatchdogProbe,
        case: &str,
    ) -> (Result<(), MessageError>, u8) {
        let message = &mut probe.message;
        message.channel.calls = 0;
        message.channel.case = case.into();
        if let Some(kind) = case.strip_prefix("retained-") {
            probe.binding.as_mut().unwrap().tamper_for_test(kind);
        }
        let result = watchdog_context::preparation_round(
            self,
            &mut message.channel,
            &mut message.receiver,
            &message.pin,
            &mut message.payload,
            ((":1.74", ":1.75"), probe.binding.as_ref()),
            if case.starts_with("closing-context-") || case.starts_with("inside-") {
                Some(case)
            } else {
                None
            },
        );
        let result = result.map(|binding| {
            probe.binding = Some(binding);
        });
        (result, message.channel.calls)
    }
}
