//! Exact owned fixture adapter, no production transport or negotiated bus claim.
use super::*;
impl super::super::super::super::OriginalEnrollmentFrame<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn addressed_rights_roundtrip_for_test(&self, message: &Message) -> Message {
        self.observe().unwrap();
        let [sender, receiver] = pair();
        let received = transfer(&sender, &receiver, message);
        self.observe().unwrap();
        received
    }
}
