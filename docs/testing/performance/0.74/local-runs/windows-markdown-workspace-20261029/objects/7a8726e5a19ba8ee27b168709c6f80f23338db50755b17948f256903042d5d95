//! Explicit consent under the original receiver, credentials and clock.
use super::*;
use crate::diagnostic_start_consent::{inspect_start_consent_signature, StartConsentBody};

fn age(received: u64, after: u64, now: u64) -> Result<(), KernelError> {
    if received <= after
        || now < received
        || now - received >= crate::diagnostic_lease::CONTROLLER_LOSS_SECONDS * 1_000_000_000
    {
        return Err(KernelError::Invalid);
    }
    Ok(())
}
fn principal_and_time(
    body: &StartConsentBody,
    policy: &DiagnosticPolicy,
    peer: &DiagnosticPeer,
    now: u64,
    boot: &str,
) -> Result<(), KernelError> {
    if !policy.client_uids.contains(&peer.uid)
        || !peer.gids.contains(&policy.client_gid)
        || body.controller.repository_id != policy.repository_id
        || !policy.actor_ids.contains(&body.controller.actor_id)
        || body.starting_state.identity.boot_id != boot
        || now < body.issued_at_unix_seconds
        || now >= body.expires_at_unix_seconds
    {
        return Err(KernelError::Invalid);
    }
    Ok(())
}
impl KernelPacket {
    pub(crate) fn authenticate_original_start_consent(
        &mut self,
        next: &mut KernelPacket,
        policy: &DiagnosticPolicy,
        after: &KernelTimePoint,
    ) -> Result<StartConsentBody, KernelError> {
        let result = (|| {
            if self.is_refused() || next.is_refused() {
                return Err(KernelError::Refused);
            }
            if !Arc::ptr_eq(&self.origin, &next.origin) {
                return Err(KernelError::Invalid);
            }
            ordered(&self.received, &next.received)?;
            ordered(after, &next.received)?;
            let before = self.original_reservation_time(Some(after))?;
            ordered(&next.received, &before)?;
            age(
                next.received.boottime_ns(),
                after.boottime_ns(),
                before.boottime_ns(),
            )?;
            let (peer, unix) = next.original_collector_inputs()?;
            let body = inspect_start_consent_signature(next.bytes(), &policy.verifying_key)
                .map_err(|_| KernelError::Invalid)?;
            principal_and_time(&body, policy, &peer, unix, before.boot_id())?;
            let now = self.original_reservation_time(Some(&before))?;
            ordered(&next.received, &now)?;
            age(
                next.received.boottime_ns(),
                after.boottime_ns(),
                now.boottime_ns(),
            )?;
            let (peer, unix) = next.original_collector_inputs()?;
            principal_and_time(&body, policy, &peer, unix, now.boot_id())?;
            Ok(body)
        })();
        if result.is_err() {
            self.latch_refusal();
            next.latch_refusal();
        }
        result
    }
}

#[cfg(test)]
#[path = "diagnostic_kernel_start_consent_tests.rs"]
mod tests;
