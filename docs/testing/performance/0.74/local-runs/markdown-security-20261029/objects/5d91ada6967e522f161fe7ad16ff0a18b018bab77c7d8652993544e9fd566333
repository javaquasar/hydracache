//! Opt-in constant-depth preparation; original public guards remain the reference.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PreConsentStep {
    StorageBefore,
    ReservedLive,
    Progress,
    Causal,
    Prepared,
    StorageAfter,
    FinalClocks,
}
const STEPS: [PreConsentStep; 7] = [
    PreConsentStep::StorageBefore,
    PreConsentStep::ReservedLive,
    PreConsentStep::Progress,
    PreConsentStep::Causal,
    PreConsentStep::Prepared,
    PreConsentStep::StorageAfter,
    PreConsentStep::FinalClocks,
];
fn valid_age(start: u64, now: u64, seconds: u64) -> bool {
    start != 0
        && now
            .checked_sub(start)
            .is_some_and(|age| age < seconds * 1_000_000_000)
}

/// Original preparation, not a publisher, cached proof or start capability.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::preconsent::FramedConsentPreparation;
/// fn copy(p: FramedConsentPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::preconsent::FramedConsentPreparation;
/// fn start(p: FramedConsentPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.start_once(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::preconsent::FramedConsentPreparation;
/// fn publish(p: FramedConsentPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.publish_starting(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::preconsent::FramedConsentPreparation;
/// fn extract(p: FramedConsentPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.into_original(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::preconsent::FramedConsentPreparation;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<FramedConsentPreparation<'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static>>();
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::preconsent::FramedConsentPreparation;
/// use hydracache_long_run_supervisor_074::diagnostic_kernel_admission::KernelPacket;
/// fn twice(p: FramedConsentPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>, a: KernelPacket, b: KernelPacket) { let first = p.accept_consent(a); let second = p.accept_consent(b); }
/// ```
pub struct FramedConsentPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    prepared: OriginalStartConsentPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
}
impl fmt::Debug for FramedConsentPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FramedConsentPreparation")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
impl FramedConsentPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.prepared.is_refused()
    }
    pub fn consent_to_sign(&self) -> &StartConsentBody {
        self.prepared.consent_to_sign()
    }
    fn freeze_body(&mut self, id: String) -> Result<(), OriginalStartConsentError> {
        if self.prepared.body.is_some() || self.prepared.point.is_some() {
            return Err(OriginalStartConsentError::Binding);
        }
        let r = &mut self.prepared.original.original.reservation;
        let controller = &mut r.original.original.original;
        let response: SignedDiagnosticRequest =
            serde_json::from_slice(self.prepared.original.original.packet.bytes())
                .map_err(|_| OriginalStartConsentError::Binding)?;
        require_new_id(
            &id,
            &controller.request.request_id,
            &response.request.request_id,
        )?;
        let point = controller.worker.original_reservation_time(Some(&r.last))?;
        let mut state = r.state.clone();
        state.stage = DiagnosticStage::Starting;
        state.revision = 2;
        state.last_observed_monotonic_ns = point.boottime_ns();
        state.controller_monotonic_ns = point.boottime_ns();
        state.cell_started_monotonic_ns = Some(point.boottime_ns());
        state.cleanup_confirmed = false;
        crate::diagnostic_lease::validate_transition(&r.state, &state)
            .map_err(|_| OriginalStartConsentError::Binding)?;
        let (policy, epoch, _, _) = controller.worker.controller_authority_binding()?;
        let body = StartConsentBody {
            schema_version: BODY_SCHEMA.into(),
            action: "start-cell-one".into(),
            consent_request_id: id,
            reservation_request_id: controller.request.request_id.clone(),
            controller_attempt_sha256: sha256_hex(&r.request_raw),
            collector_reserved_sha256: sha256_hex(&r.raw),
            reserved_state_sha256: sha256_hex(
                &canonical_json(&r.state).map_err(|_| OriginalStartConsentError::Binding)?,
            ),
            challenge_nonce_sha256: self.prepared.original.nonce.clone(),
            authenticated_response_sha256: sha256_hex(
                self.prepared.original.original.packet.bytes(),
            ),
            worker_policy_sha256: policy,
            worker_policy_epoch: epoch,
            collector_unit_sha256: sha256_hex(
                &collector_unit_projection(&r.state)
                    .map_err(|_| OriginalStartConsentError::Binding)?,
            ),
            controller: controller.request.controller.clone(),
            source_commit: controller.request.source_commit.clone(),
            preset: controller.request.preset.clone(),
            starting_state: state,
            issued_at_unix_seconds: point.unix_seconds(),
            expires_at_unix_seconds: point
                .unix_seconds()
                .checked_add(60)
                .ok_or(OriginalStartConsentError::Binding)?,
        };
        start_consent_signing_message(&body).map_err(|_| OriginalStartConsentError::Binding)?;
        self.prepared.body = Some(body);
        self.prepared.point = Some(point);
        Ok(())
    }
    fn prepared(&mut self) -> Result<(), OriginalStartConsentError> {
        let prepared = &mut self.prepared;
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
        let causal = &mut self.prepared.original;
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
        let causal = &mut self.prepared.original;
        let heartbeat = causal.original.packet.received_time().boottime_ns();
        let r = &mut causal.original.reservation;
        let now = r
            .original
            .original
            .original
            .worker
            .original_reservation_time(Some(&r.last))?;
        let loss = crate::diagnostic_lease::CONTROLLER_LOSS_SECONDS;
        super::super::age(causal.issued.boottime_ns(), now.boottime_ns())?;
        if now.boot_id() != r.state.identity.boot_id
            || !valid_age(heartbeat, now.boottime_ns(), loss)
            || !valid_age(
                r.state.reserved_monotonic_ns,
                now.boottime_ns(),
                crate::diagnostic_lease::TOTAL_SECONDS,
            )
        {
            return Err(OriginalStartConsentError::Binding);
        }
        if let Some(body) = &self.prepared.body {
            if now.unix_seconds() < body.issued_at_unix_seconds
                || now.unix_seconds() >= body.expires_at_unix_seconds
            {
                return Err(OriginalStartConsentError::Binding);
            }
        }
        // Observation cursor only; challenge, heartbeat, Reserved and prepared origins are fixed.
        r.last = now;
        Ok(())
    }
    fn round(
        &mut self,
        mut before: impl FnMut(PreConsentStep),
    ) -> Result<(), OriginalStartConsentError> {
        for step in STEPS {
            before(step);
            match step {
                PreConsentStep::StorageBefore | PreConsentStep::StorageAfter => self
                    .prepared
                    .original
                    .original
                    .reservation
                    .storage
                    .revalidate()
                    .map_err(CollectorReservedError::from)
                    .map_err(CollectorCausalError::from)?,
                PreConsentStep::ReservedLive => self
                    .prepared
                    .original
                    .original
                    .reservation
                    .live()
                    .map_err(CollectorCausalError::from)?,
                PreConsentStep::Progress => self
                    .prepared
                    .original
                    .original
                    .progress()
                    .map_err(CollectorCausalError::from)?,
                PreConsentStep::Causal => self.causal()?,
                PreConsentStep::Prepared => {
                    if self.prepared.body.is_some() {
                        self.prepared()?;
                    } else if self.prepared.point.is_some() {
                        return Err(OriginalStartConsentError::Binding);
                    }
                }
                PreConsentStep::FinalClocks => self.final_clocks()?,
            }
        }
        Ok(())
    }
    fn validate_with(
        &mut self,
        before: impl FnMut(PreConsentStep),
    ) -> Result<(), OriginalStartConsentError> {
        if self.is_refused() {
            self.prepared.refuse();
            return Err(OriginalStartConsentError::Refused);
        }
        let result = self.round(before).and_then(|_| self.round(|_| {}));
        if result.is_err() {
            self.prepared.refuse();
        }
        result
    }
    pub fn revalidate(&mut self) -> Result<(), OriginalStartConsentError> {
        self.validate_with(|_| {})
    }
    #[cfg(test)]
    pub(crate) fn revalidate_with_for_test(
        &mut self,
        before: impl FnMut(PreConsentStep),
    ) -> Result<(), OriginalStartConsentError> {
        self.validate_with(before)
    }
}

enum Held<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    Preparation {
        error: OriginalStartConsentError,
        owner: Box<FramedConsentPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
    },
    Consent(OriginalFramedConsentFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>),
}
pub struct FramedConsentFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    held: Held<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
}
impl FramedConsentFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn error(&self) -> &OriginalStartConsentError {
        match &self.held {
            Held::Preparation { error, .. } => error,
            Held::Consent(f) => f.error(),
        }
    }
    pub fn is_refused(&self) -> bool {
        match &self.held {
            Held::Preparation { owner, .. } => owner.is_refused(),
            Held::Consent(f) => f.is_refused(),
        }
    }
}
impl fmt::Debug for FramedConsentFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.error(), f)
    }
}
impl fmt::Display for FramedConsentFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.error(), f)
    }
}
impl std::error::Error for FramedConsentFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {}

impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalCausalProgress<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub fn prepare_framed_consent(
        self,
        request_id: String,
    ) -> Result<
        FramedConsentPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        FramedConsentFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut owner = FramedConsentPreparation {
            prepared: OriginalStartConsentPreparation {
                original: self,
                body: None,
                point: None,
            },
        };
        let result = owner
            .revalidate()
            .and_then(|_| owner.freeze_body(request_id))
            .and_then(|_| owner.revalidate());
        match result {
            Ok(()) => Ok(owner),
            Err(error) => {
                owner.prepared.refuse();
                Err(FramedConsentFailure {
                    held: Held::Preparation {
                        error,
                        owner: Box::new(owner),
                    },
                })
            }
        }
    }
}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    FramedConsentPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub fn accept_consent(
        self,
        packet: KernelPacket,
    ) -> Result<
        OriginalFramedConsent<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        FramedConsentFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let original = OriginalStartConsent {
            prepared: self.prepared,
            packet,
        };
        original
            .into_validation_frame()
            .map_err(|failure| FramedConsentFailure {
                held: Held::Consent(failure),
            })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preconsent_age_matches_unchanged_challenge_reference() {
        let loss = crate::diagnostic_lease::CONTROLLER_LOSS_SECONDS;
        let mut seed = 0x0740_9c01_u64;
        for _ in 0..512 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let start = seed % 1_000_000_000;
            for now in [
                0,
                start,
                start.saturating_sub(1),
                start + loss * 1_000_000_000 - 1,
                start + loss * 1_000_000_000,
                u64::MAX,
            ] {
                assert_eq!(
                    valid_age(start, now, loss),
                    super::super::super::age(start, now).is_ok()
                );
            }
        }
        println!("preconsent_age seed=0x07409c01 cases=3072 exclusive=true");
    }
}
