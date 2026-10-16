//! Constant-depth orchestration; all original live leaf guards remain active.
use super::*;

#[path = "diagnostic_collector_starting_publication.rs"]
pub mod starting;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ValidationFrameStep {
    StorageBefore,
    ReservedLive,
    Progress,
    Causal,
    Prepared,
    Consent,
    StorageAfter,
    FinalClocks,
}
const STEPS: [ValidationFrameStep; 8] = [
    ValidationFrameStep::StorageBefore,
    ValidationFrameStep::ReservedLive,
    ValidationFrameStep::Progress,
    ValidationFrameStep::Causal,
    ValidationFrameStep::Prepared,
    ValidationFrameStep::Consent,
    ValidationFrameStep::StorageAfter,
    ValidationFrameStep::FinalClocks,
];

/// Owns the admitted originals, not a cached validation result or start capability.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::OriginalFramedConsent;
/// fn clone(p: OriginalFramedConsent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::OriginalFramedConsent;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<OriginalFramedConsent<'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static>>();
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::OriginalFramedConsent;
/// fn extract(p: OriginalFramedConsent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.into_original(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::OriginalFramedConsent;
/// fn start(p: OriginalFramedConsent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.start_once(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::OriginalFramedConsent;
/// fn publish(p: OriginalFramedConsent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.publish_starting(b"replacement"); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::OriginalFramedConsent;
/// fn replace(mut p: OriginalFramedConsent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.revalidate(b"replacement"); }
/// ```
pub struct OriginalFramedConsent<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    original: OriginalStartConsent<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
}
impl fmt::Debug for OriginalFramedConsent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OriginalFramedConsent")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
fn valid_age(start: u64, now: u64, seconds: u64) -> bool {
    start != 0
        && now
            .checked_sub(start)
            .is_some_and(|age| age < seconds * 1_000_000_000)
}
impl OriginalFramedConsent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.original.is_refused()
    }
    fn refuse(&mut self) {
        self.original.refuse();
    }
    fn prepared(&mut self) -> Result<(), OriginalStartConsentError> {
        let prepared = &mut self.original.prepared;
        let r = &mut prepared.original.original.reservation;
        let body = prepared
            .body
            .as_ref()
            .ok_or(OriginalStartConsentError::Binding)?;
        let point = prepared
            .point
            .as_ref()
            .ok_or(OriginalStartConsentError::Binding)?;
        let controller = &mut r.original.original.original;
        let now = controller.worker.original_reservation_time(Some(point))?;
        let (policy, epoch, _, _) = controller.worker.controller_authority_binding()?;
        if body.worker_policy_sha256 != policy
            || body.worker_policy_epoch != epoch
            || now.unix_seconds() < body.issued_at_unix_seconds
            || now.unix_seconds() >= body.expires_at_unix_seconds
        {
            return Err(OriginalStartConsentError::Binding);
        }
        crate::diagnostic_lease::validate_transition(&r.state, &body.starting_state)
            .map_err(|_| OriginalStartConsentError::Binding)?;
        Ok(())
    }
    fn causal(&mut self) -> Result<(), OriginalStartConsentError> {
        let causal = &mut self.original.prepared.original;
        super::super::check(
            &causal.original.reservation,
            &causal.raw,
            &causal.nonce,
            &causal.issued,
        )?;
        let response: SignedDiagnosticRequest =
            serde_json::from_slice(causal.original.packet.bytes())
                .map_err(|_| CollectorCausalError::Document)?;
        if response.request.nonce_sha256 != causal.nonce {
            return Err(CollectorCausalError::Binding.into());
        }
        super::super::window(&mut causal.original.reservation, &causal.issued)?;
        Ok(())
    }
    fn final_clocks(&mut self) -> Result<(), OriginalStartConsentError> {
        let prepared = &mut self.original.prepared;
        let causal = &mut prepared.original;
        let heartbeat = causal.original.packet.received_time().boottime_ns();
        let consent = self.original.packet.received_time().boottime_ns();
        let r = &mut causal.original.reservation;
        let now = r
            .original
            .original
            .original
            .worker
            .original_reservation_time(Some(&r.last))?;
        let body = prepared
            .body
            .as_ref()
            .ok_or(OriginalStartConsentError::Binding)?;
        let loss = crate::diagnostic_lease::CONTROLLER_LOSS_SECONDS;
        super::super::age(causal.issued.boottime_ns(), now.boottime_ns())?;
        if now.boot_id() != r.state.identity.boot_id
            || !valid_age(heartbeat, now.boottime_ns(), loss)
            || !valid_age(consent, now.boottime_ns(), loss)
            || !valid_age(
                r.state.reserved_monotonic_ns,
                now.boottime_ns(),
                crate::diagnostic_lease::TOTAL_SECONDS,
            )
            || now.unix_seconds() < body.issued_at_unix_seconds
            || now.unix_seconds() >= body.expires_at_unix_seconds
        {
            return Err(OriginalStartConsentError::Binding);
        }
        // Rolling observation cursor only: every admission origin remains unchanged.
        r.last = now;
        Ok(())
    }
    fn round(
        &mut self,
        mut before: impl FnMut(ValidationFrameStep),
    ) -> Result<(), OriginalStartConsentError> {
        for step in STEPS {
            before(step);
            match step {
                ValidationFrameStep::StorageBefore | ValidationFrameStep::StorageAfter => self
                    .original
                    .prepared
                    .original
                    .original
                    .reservation
                    .storage
                    .revalidate()
                    .map_err(CollectorReservedError::from)
                    .map_err(CollectorCausalError::from)?,
                ValidationFrameStep::ReservedLive => self
                    .original
                    .prepared
                    .original
                    .original
                    .reservation
                    .live()
                    .map_err(CollectorCausalError::from)?,
                ValidationFrameStep::Progress => self
                    .original
                    .prepared
                    .original
                    .original
                    .progress()
                    .map_err(CollectorCausalError::from)?,
                ValidationFrameStep::Causal => self.causal()?,
                ValidationFrameStep::Prepared => self.prepared()?,
                ValidationFrameStep::Consent => self.original.authenticate()?,
                ValidationFrameStep::FinalClocks => self.final_clocks()?,
            }
        }
        Ok(())
    }
    fn validate_with(
        &mut self,
        before: impl FnMut(ValidationFrameStep),
    ) -> Result<(), OriginalStartConsentError> {
        if self.is_refused() {
            self.refuse();
            return Err(OriginalStartConsentError::Refused);
        }
        let result = self.round(before).and_then(|_| self.round(|_| {}));
        if result.is_err() {
            self.refuse();
        }
        result
    }
    pub fn revalidate(&mut self) -> Result<(), OriginalStartConsentError> {
        self.validate_with(|_| {})
    }
    #[cfg(test)]
    pub(crate) fn revalidate_with_for_test(
        &mut self,
        before: impl FnMut(ValidationFrameStep),
    ) -> Result<(), OriginalStartConsentError> {
        self.validate_with(before)
    }
}
pub struct OriginalFramedConsentFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: OriginalStartConsentError,
    held: Box<OriginalFramedConsent<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl OriginalFramedConsentFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn error(&self) -> &OriginalStartConsentError {
        &self.error
    }
    pub fn is_refused(&self) -> bool {
        self.held.is_refused()
    }
}
impl fmt::Debug for OriginalFramedConsentFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.error, f)
    }
}
impl fmt::Display for OriginalFramedConsentFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}
impl std::error::Error for OriginalFramedConsentFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalStartConsent<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub fn into_validation_frame(
        self,
    ) -> Result<
        OriginalFramedConsent<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalFramedConsentFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut held = OriginalFramedConsent { original: self };
        match held.revalidate() {
            Ok(()) => Ok(held),
            Err(error) => {
                held.refuse();
                Err(OriginalFramedConsentFailure {
                    error,
                    held: Box::new(held),
                })
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validation_frame_exclusive_age_matches_original_challenge_reference() {
        let loss = crate::diagnostic_lease::CONTROLLER_LOSS_SECONDS;
        for start in [0, 1, 100, u64::MAX - 1] {
            for now in [0, 1, 99, 100, 101, 10_000_000_000, 10_000_000_001, u64::MAX] {
                assert_eq!(
                    valid_age(start, now, loss),
                    super::super::super::age(start, now).is_ok()
                );
            }
        }
        let mut seed = 0x0740_f4a1_u64;
        for _ in 0..512 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let start = 1 + seed % 1_000_000_000;
            for age in [
                0,
                loss * 1_000_000_000 - 1,
                loss * 1_000_000_000,
                loss * 1_000_000_000 + 1,
            ] {
                let now = start + age;
                assert_eq!(
                    valid_age(start, now, loss),
                    super::super::super::age(start, now).is_ok()
                );
            }
        }
    }
}
