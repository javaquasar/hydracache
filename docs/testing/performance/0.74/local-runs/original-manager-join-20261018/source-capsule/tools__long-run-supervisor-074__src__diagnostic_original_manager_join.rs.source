//! Consuming original context join, never dispatch or execution authority.
use super::super::{BarredCustodyError, OriginalEnrollmentFrame};
use super::{kernel::receiver::ReceiverPin, FixedManagerChannel};
use crate::diagnostic_kernel_admission::{KernelError, OriginalReceiverBinding};

#[derive(Debug, thiserror::Error)]
pub(crate) enum JoinError {
    #[error("original enrollment refused: {0}")]
    Original(#[from] BarredCustodyError),
    #[error("original receiver context refused: {0}")]
    Kernel(#[from] KernelError),
    #[error("fixed live manager channel refused")]
    Channel,
    #[error("original manager join is refused or incomplete")]
    Refused,
}

pub(crate) struct OriginalManagerJoin<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    // Original child cancellation precedes original writers/final custody as before.
    original: OriginalEnrollmentFrame<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    channel: Option<FixedManagerChannel>,
    receiver: Option<OriginalReceiverBinding>,
    pin: Option<ReceiverPin>,
    ready: bool,
}
pub(crate) struct ManagerJoinFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: JoinError,
    held: Box<OriginalManagerJoin<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl std::fmt::Debug for ManagerJoinFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl std::fmt::Debug for OriginalManagerJoin<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OriginalManagerJoin")
            .field("ready", &self.ready)
            .finish_non_exhaustive()
    }
}
fn retain_receiver(
    original: &mut OriginalEnrollmentFrame<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
) -> std::result::Result<OriginalReceiverBinding, JoinError> {
    let material = &mut original.original.original.transport.material;
    Ok(material
        .outputs
        .pipes
        .starting
        .original
        .original
        .prepared
        .original
        .original
        .packet
        .retain_original_receiver_binding()?)
}

// Sealed implementation detail; test channels never construct OriginalManagerJoin.
trait ChannelChecks {
    fn check(
        &mut self,
        receiver: &mut OriginalReceiverBinding,
        pin: &ReceiverPin,
    ) -> std::result::Result<(), JoinError>;
}
impl ChannelChecks for FixedManagerChannel {
    fn check(
        &mut self,
        receiver: &mut OriginalReceiverBinding,
        pin: &ReceiverPin,
    ) -> std::result::Result<(), JoinError> {
        self.revalidate().map_err(|_| JoinError::Channel)?;
        pin.require(self, receiver)?;
        Ok(())
    }
}
fn validation_round(
    original: &mut OriginalEnrollmentFrame<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    channel: &mut impl ChannelChecks,
    receiver: &mut OriginalReceiverBinding,
    pin: &ReceiverPin,
) -> std::result::Result<(), JoinError> {
    let result = (|| {
        if !original.ready || original.original.is_refused() {
            return Err(JoinError::Refused);
        }
        original.observe()?;
        channel.check(receiver, pin)?;
        original.observe()?;
        original.revalidate()?;
        channel.check(receiver, pin)?;
        original.observe()?;
        Ok(())
    })();
    if result.is_err() {
        original.original.refuse();
    }
    result
}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalEnrollmentFrame<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn bind_fixed_manager(
        self,
    ) -> std::result::Result<
        OriginalManagerJoin<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        ManagerJoinFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut held = OriginalManagerJoin {
            original: self,
            channel: None,
            receiver: None,
            pin: None,
            ready: false,
        };
        let result = (|| {
            if !held.original.ready || held.original.original.is_refused() {
                return Err(JoinError::Refused);
            }
            held.original.observe()?;
            held.receiver = Some(retain_receiver(&mut held.original)?);
            held.pin = Some(ReceiverPin::capture().map_err(|_| JoinError::Channel)?);
            held.channel = Some(FixedManagerChannel::open_fixed().map_err(|_| JoinError::Channel)?);
            held.validate()?;
            held.ready = true;
            Ok(())
        })();
        match result {
            Ok(()) => Ok(held),
            Err(error) => {
                held.original.original.refuse();
                Err(ManagerJoinFailure {
                    error,
                    held: Box::new(held),
                })
            }
        }
    }
}
impl OriginalManagerJoin<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn validate(&mut self) -> std::result::Result<(), JoinError> {
        validation_round(
            &mut self.original,
            self.channel.as_mut().ok_or(JoinError::Refused)?,
            self.receiver.as_mut().ok_or(JoinError::Refused)?,
            self.pin.as_ref().ok_or(JoinError::Refused)?,
        )
    }
    pub(crate) fn revalidate(&mut self) -> std::result::Result<(), JoinError> {
        let result = if !self.ready {
            Err(JoinError::Refused)
        } else {
            self.validate()
        };
        if result.is_err() {
            self.ready = false;
            self.original.original.refuse();
        }
        result
    }
}

#[cfg(test)]
#[test]
fn original_manager_join_requires_the_original_consuming_constructor() {
    fn signature(original: OriginalEnrollmentFrame<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
        let _ = original.bind_fixed_manager();
    }
    let _contract = signature;
}

#[cfg(test)]
#[path = "diagnostic_original_manager_join_tests.rs"]
mod tests;
