//! Consumed original bootstrap consent; no durable acknowledgement or execution.
use super::*;
#[path = "diagnostic_validation_frame.rs"]
mod validation_frame;
pub use validation_frame::{OriginalFramedConsent, OriginalFramedConsentFailure};

use crate::diagnostic_start_consent::{
    start_consent_signing_message, StartConsentBody, BODY_SCHEMA,
};
use crate::diagnostic_worker_policy::intent_projection::collector::collector_unit_projection;

#[derive(Debug, thiserror::Error)]
pub enum OriginalStartConsentError {
    #[error("original start consent permanently refused")]
    Refused,
    #[error("original causal owner refused: {0}")]
    Causal(#[from] CollectorCausalError),
    #[error("original kernel/worker input refused: {0}")]
    Input(#[from] TrustInputError),
    #[error("start consent document differs or cannot be constructed")]
    Binding,
}
fn equal_body(
    expected: &StartConsentBody,
    actual: &StartConsentBody,
) -> Result<(), OriginalStartConsentError> {
    if canonical_json(expected).map_err(|_| OriginalStartConsentError::Binding)?
        != canonical_json(actual).map_err(|_| OriginalStartConsentError::Binding)?
    {
        return Err(OriginalStartConsentError::Binding);
    }
    Ok(())
}
fn require_new_id(
    id: &str,
    reservation: &str,
    response: &str,
) -> Result<(), OriginalStartConsentError> {
    if !is_hash(id) || id == reservation || id == response {
        return Err(OriginalStartConsentError::Binding);
    }
    Ok(())
}

/// Frozen claims from retained original owners, not a start or publish capability.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::OriginalStartConsentPreparation;
/// fn start(p: OriginalStartConsentPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.start_once(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::OriginalStartConsentPreparation;
/// use hydracache_long_run_supervisor_074::diagnostic_kernel_admission::KernelPacket;
/// fn twice(p: OriginalStartConsentPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>, a: KernelPacket, b: KernelPacket) {
///     let first = p.accept_consent(a); let second = p.accept_consent(b);
/// }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::OriginalStartConsentPreparation;
/// fn copy(p: OriginalStartConsentPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.clone(); }
/// ```
pub struct OriginalStartConsentPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    original: OriginalCausalProgress<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    body: Option<StartConsentBody>,
    point: Option<KernelTimePoint>,
}
impl fmt::Debug for OriginalStartConsentPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OriginalStartConsentPreparation")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
impl OriginalStartConsentPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.original.is_refused()
    }
    fn refuse(&mut self) {
        self.original.refuse();
    }
    fn prepare(&mut self, id: String) -> Result<(), OriginalStartConsentError> {
        self.original.revalidate()?;
        let r = &mut self.original.original.reservation;
        let controller = &mut r.original.original.original;
        let response: SignedDiagnosticRequest =
            serde_json::from_slice(self.original.original.packet.bytes())
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
            challenge_nonce_sha256: self.original.nonce.clone(),
            authenticated_response_sha256: sha256_hex(self.original.original.packet.bytes()),
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
        self.body = Some(body);
        self.point = Some(point);
        self.revalidate()
    }
    /// Frozen untrusted data for an independent signer; it cannot acknowledge or launch.
    pub fn consent_to_sign(&self) -> &StartConsentBody {
        self.body
            .as_ref()
            .expect("only successfully prepared owners are returned")
    }
    pub fn revalidate(&mut self) -> Result<(), OriginalStartConsentError> {
        if self.is_refused() {
            self.refuse();
            return Err(OriginalStartConsentError::Refused);
        }
        let result = (|| {
            self.original.revalidate()?;
            let r = &mut self.original.original.reservation;
            let body = self
                .body
                .as_ref()
                .ok_or(OriginalStartConsentError::Binding)?;
            let point = self
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
            self.original.revalidate()?;
            Ok(())
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }
}

/// Accepted once in memory under the original owners. Durable replay is still required.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::OriginalStartConsent;
/// fn copy(p: OriginalStartConsent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::OriginalStartConsent;
/// fn publish(p: OriginalStartConsent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.publish_starting(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::OriginalStartConsent;
/// fn replace(mut p: OriginalStartConsent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.revalidate(b"replacement"); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::OriginalStartConsent;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<OriginalStartConsent<'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static>>();
/// ```
pub struct OriginalStartConsent<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    prepared: OriginalStartConsentPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    packet: KernelPacket,
}
impl fmt::Debug for OriginalStartConsent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OriginalStartConsent")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
impl OriginalStartConsent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.prepared.is_refused() || self.packet.is_refused()
    }
    fn refuse(&mut self) {
        self.prepared.refuse();
        self.packet.latch_refusal();
    }
    fn authenticate(&mut self) -> Result<(), OriginalStartConsentError> {
        let controller = &mut self
            .prepared
            .original
            .original
            .reservation
            .original
            .original
            .original;
        let body = controller.worker.authenticate_original_start_consent(
            &mut self.packet,
            &controller.controller.source.policy,
            self.prepared
                .point
                .as_ref()
                .ok_or(OriginalStartConsentError::Binding)?,
        )?;
        equal_body(self.prepared.consent_to_sign(), &body)
    }
    pub fn revalidate(&mut self) -> Result<(), OriginalStartConsentError> {
        if self.is_refused() {
            self.refuse();
            return Err(OriginalStartConsentError::Refused);
        }
        let result = (|| {
            self.prepared.revalidate()?;
            self.authenticate()?;
            self.prepared.revalidate()?;
            self.authenticate()
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }
}
enum Held<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    Preparation(
        Box<OriginalStartConsentPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
    ),
    Consent(Box<OriginalStartConsent<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>),
}
pub struct OriginalStartConsentFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: OriginalStartConsentError,
    held: Held<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
}
impl OriginalStartConsentFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn error(&self) -> &OriginalStartConsentError {
        &self.error
    }
    pub fn is_refused(&self) -> bool {
        match &self.held {
            Held::Preparation(p) => p.is_refused(),
            Held::Consent(c) => c.is_refused(),
        }
    }
}
impl fmt::Debug for OriginalStartConsentFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.error, f)
    }
}
impl fmt::Display for OriginalStartConsentFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}
impl std::error::Error for OriginalStartConsentFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {}

impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalCausalProgress<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub fn prepare_start_consent(
        self,
        request_id: String,
    ) -> Result<
        OriginalStartConsentPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalStartConsentFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut prepared = OriginalStartConsentPreparation {
            original: self,
            body: None,
            point: None,
        };
        match prepared.prepare(request_id) {
            Ok(()) => Ok(prepared),
            Err(error) => {
                prepared.refuse();
                Err(OriginalStartConsentFailure {
                    error,
                    held: Held::Preparation(Box::new(prepared)),
                })
            }
        }
    }
}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalStartConsentPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub fn accept_consent(
        self,
        packet: KernelPacket,
    ) -> Result<
        OriginalStartConsent<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalStartConsentFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut read = OriginalStartConsent {
            prepared: self,
            packet,
        };
        match read.revalidate() {
            Ok(()) => Ok(read),
            Err(error) => {
                read.refuse();
                Err(OriginalStartConsentFailure {
                    error,
                    held: Held::Consent(Box::new(read)),
                })
            }
        }
    }
}

#[cfg(test)]
#[path = "diagnostic_original_start_consent_binding_tests.rs"]
mod tests;

#[path = "diagnostic_preconsent_frame.rs"]
pub mod preconsent;
