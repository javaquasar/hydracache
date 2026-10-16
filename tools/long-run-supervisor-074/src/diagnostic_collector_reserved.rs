//! Original initial Reserved publication only; never recovered acknowledgement or start.
use super::{OriginalCollectorError, OriginalObservedReservation};
use super::super::super::{capture as capture_request, validate_retained_controller_attempt, OidcControllerInput, Record as RequestRecord};
use crate::diagnostic_kernel_admission::KernelTimePoint;
use crate::diagnostic_lease::{DiagnosticStage, DiagnosticState};
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::attempt_storage::{
    collector_reservation_storage::CollectorReservationStorage, StorageAttemptError,
};
pub use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::attempt_storage::collector_reservation_storage::CollectorReservedStage;
use crate::diagnostic_worker_policy::intent_projection::collector::{
    collector_intent_signing_message, project_collector_reserved_state, SignedCollectorIntentProjection,
    SIGNED_COLLECTOR_INTENT_SCHEMA,
};
use crate::diagnostic_worker_policy::trusted_input::TrustInputError;
use crate::{canonical_json, is_hash, sha256_hex};
use serde::{Deserialize, Serialize};
use std::fmt;

pub const COLLECTOR_RESERVED_SCHEMA: &str = "diagnostic-collector-reserved-074-v1";
const LIMIT: usize = 16_384;
#[derive(Debug, thiserror::Error)]
pub enum CollectorReservedError {
    #[error("original collector reservation permanently refused")]
    Refused,
    #[error("original observed collector refused: {0}")]
    Original(#[from] OriginalCollectorError),
    #[error("original request refused: {0}")]
    Attempt(#[from] super::super::super::ControllerAttemptError),
    #[error("original live input refused: {0}")]
    Input(#[from] TrustInputError),
    #[error("original collector reservation storage refused: {0}")]
    Storage(#[from] StorageAttemptError),
    #[error("collector Reserved record is malformed or inconsistent")]
    Document,
    #[error("original collector Reserved binding differs")]
    Binding,
    #[error("private injected interruption at {0:?}")]
    Interrupted(CollectorReservedStage),
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema_version: String,
    controller_attempt_sha256: String,
    state: DiagnosticState,
    collector_intent: SignedCollectorIntentProjection,
}
fn line(value: &impl Serialize) -> Result<Vec<u8>, CollectorReservedError> {
    let mut raw = canonical_json(value).map_err(|_| CollectorReservedError::Document)?;
    raw.push(b'\n');
    Ok(raw)
}
/// Strict paired byte consistency only. No key verification, file open or recovered permit.
pub fn validate_retained_collector_reserved(
    raw: &[u8],
    request_raw: &[u8],
) -> Result<(), CollectorReservedError> {
    if raw.is_empty() || raw.len() > LIMIT {
        return Err(CollectorReservedError::Document);
    }
    validate_retained_controller_attempt(request_raw)?;
    let request: RequestRecord =
        serde_json::from_slice(request_raw).map_err(|_| CollectorReservedError::Document)?;
    let r: Record = serde_json::from_slice(raw).map_err(|_| CollectorReservedError::Document)?;
    let state = &r.state;
    let b = &r.collector_intent.intent;
    let p =
        project_collector_reserved_state(state).map_err(|_| CollectorReservedError::Document)?;
    if r.schema_version != COLLECTOR_RESERVED_SCHEMA
        || line(&r)? != raw
        || r.controller_attempt_sha256 != sha256_hex(request_raw)
        || r.collector_intent.schema_version != SIGNED_COLLECTOR_INTENT_SCHEMA
        || line(&r.collector_intent)?.len() > 8192
        || collector_intent_signing_message(b).is_err()
        || r.collector_intent.signature_hex.len() != 128
        || !r
            .collector_intent
            .signature_hex
            .bytes()
            .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
        || state.stage != DiagnosticStage::Reserved
        || state.revision != 1
        || state.completed_cells != 0
        || state.reserved_monotonic_ns == 0
        || state.reserved_monotonic_ns != state.last_observed_monotonic_ns
        || state.reserved_monotonic_ns != state.controller_monotonic_ns
        || state.identity != request.request.identity
        || b.request_id != request.request.request_id
        || b.nonce_sha256 != request.request.nonce_sha256
        || b.controller != request.request.controller
        || b.issued_at_unix_seconds != request.request.issued_at_unix_seconds
        || b.expires_at_unix_seconds != request.request.expires_at_unix_seconds
        || b.boot_id != state.identity.boot_id
        || b.worker_policy_sha256 != request.worker_policy_sha256
        || b.worker_policy_epoch != request.policy_epoch
        || b.state_sha256 != p.state_sha256
        || b.unit_spec_sha256 != p.unit_spec_sha256
        || [&b.request_id, &b.nonce_sha256, &b.worker_policy_sha256]
            .iter()
            .any(|v| !is_hash(v))
    {
        return Err(CollectorReservedError::Document);
    }
    Ok(())
}
fn encode(
    request: &[u8],
    state: &DiagnosticState,
    collector: &[u8],
) -> Result<Vec<u8>, CollectorReservedError> {
    if collector.is_empty() || collector.len() > 8192 {
        return Err(CollectorReservedError::Document);
    }
    let signed = serde_json::from_slice(collector).map_err(|_| CollectorReservedError::Document)?;
    if line(&signed)? != collector {
        return Err(CollectorReservedError::Document);
    }
    let raw = line(&Record {
        schema_version: COLLECTOR_RESERVED_SCHEMA.into(),
        controller_attempt_sha256: sha256_hex(request),
        state: state.clone(),
        collector_intent: signed,
    })?;
    validate_retained_collector_reserved(&raw, request)?;
    Ok(raw)
}

/// Runtime acknowledgement under the original fence, not authenticated start authority.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::OriginalCollectorReservation;
/// fn clone(g: OriginalCollectorReservation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { g.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::OriginalCollectorReservation;
/// fn start(mut g: OriginalCollectorReservation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { g.start_once(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::OriginalCollectorReservation;
/// fn replace(mut g: OriginalCollectorReservation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { g.revalidate(b"replacement"); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::OriginalCollectorReservation;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<OriginalCollectorReservation<'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static>>();
/// ```
pub struct OriginalCollectorReservation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    original: &'join mut OidcControllerInput<'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    storage: CollectorReservationStorage<'storage>,
    request_raw: Vec<u8>,
    collector: Vec<u8>,
    state: DiagnosticState,
    last: KernelTimePoint,
    raw: Vec<u8>,
}
impl fmt::Debug for OriginalCollectorReservation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OriginalCollectorReservation")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
impl OriginalCollectorReservation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.original.is_refused() || self.storage.is_refused()
    }
    fn refuse(&mut self) {
        self.storage.latch_refusal();
        self.original.trust.refused = true;
        self.original.original.original.latch_refusal();
    }
    fn live(&mut self) -> Result<(), CollectorReservedError> {
        if self.is_refused() {
            self.refuse();
            return Err(CollectorReservedError::Refused);
        }
        live_parts(
            self.original,
            &self.request_raw,
            &self.collector,
            &self.state,
            &mut self.last,
            &self.raw,
        )
    }
    pub fn revalidate(&mut self) -> Result<(), CollectorReservedError> {
        if self.is_refused() {
            self.refuse();
            return Err(CollectorReservedError::Refused);
        }
        let result = (|| {
            self.storage.revalidate()?;
            self.live()?;
            self.storage.revalidate()?;
            self.live()?;
            self.storage.revalidate()?;
            Ok(())
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }
}
fn live_parts(
    original: &mut OidcControllerInput<'_, '_, '_, '_, '_, '_, '_, '_>,
    request_raw: &[u8],
    collector: &[u8],
    state: &DiagnosticState,
    last: &mut KernelTimePoint,
    raw: &[u8],
) -> Result<(), CollectorReservedError> {
    if original.is_refused() {
        return Err(CollectorReservedError::Refused);
    }
    if capture_request(original)? != request_raw || encode(request_raw, state, collector)? != raw {
        return Err(CollectorReservedError::Binding);
    }
    let controller = &mut original.original.original;
    let now = controller.worker.original_reservation_time(Some(last))?;
    let body = controller.worker.verify_original_collector_requirements(
        collector,
        state,
        &controller.controller.source.policy,
    )?;
    let request = &controller.request;
    if now.boot_id() != state.identity.boot_id
        || now.boottime_ns() < state.reserved_monotonic_ns
        || body.request_id != request.request_id
        || body.nonce_sha256 != request.nonce_sha256
        || body.controller != request.controller
        || state.identity != request.identity
        || body.issued_at_unix_seconds != request.issued_at_unix_seconds
        || body.expires_at_unix_seconds != request.expires_at_unix_seconds
    {
        return Err(CollectorReservedError::Binding);
    }
    if capture_request(original)? != request_raw {
        return Err(CollectorReservedError::Binding);
    }
    *last = now;
    Ok(())
}
enum Held<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    Observed(Box<OriginalObservedReservation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>),
    Publication(Box<OriginalCollectorReservation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>),
}
pub struct CollectorReservationFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: CollectorReservedError,
    held: Held<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
}
impl CollectorReservationFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn error(&self) -> &CollectorReservedError {
        &self.error
    }
    pub fn is_refused(&self) -> bool {
        match &self.held {
            Held::Observed(v) => v.is_refused(),
            Held::Publication(v) => v.is_refused(),
        }
    }
}
impl fmt::Debug for CollectorReservationFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.error, f)
    }
}
impl fmt::Display for CollectorReservationFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}
impl std::error::Error for CollectorReservationFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {}

impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalObservedReservation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    /// Consuming publication; failure retains the original chain and uncertain context.
    /// ```compile_fail
    /// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::OriginalObservedReservation;
    /// fn twice(g: OriginalObservedReservation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
    ///     let first = g.publish_reserved(b""); let second = g.publish_reserved(b"");
    /// }
    /// ```
    pub fn publish_reserved(
        self,
        bytes: &[u8],
    ) -> Result<
        OriginalCollectorReservation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        CollectorReservationFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        self.publish_with(bytes, |_| Ok(()))
    }
    #[cfg(test)]
    pub(crate) fn publish_reserved_with(
        self,
        bytes: &[u8],
        checkpoint: impl FnMut(CollectorReservedStage) -> Result<(), CollectorReservedError>,
    ) -> Result<
        OriginalCollectorReservation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        CollectorReservationFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        self.publish_with(bytes, checkpoint)
    }
    fn publish_with(
        mut self,
        bytes: &[u8],
        mut checkpoint: impl FnMut(CollectorReservedStage) -> Result<(), CollectorReservedError>,
    ) -> Result<
        OriginalCollectorReservation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        CollectorReservationFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let prepared = self
            .check_collector(bytes)
            .map_err(CollectorReservedError::from)
            .and_then(|_| {
                encode(
                    &self.attempt.raw,
                    self.state.as_ref().expect("checked observed state"),
                    bytes,
                )
            });
        let raw = match prepared {
            Ok(raw) => raw,
            Err(error) => {
                super::latch(self.attempt.original, &mut self.attempt.storage);
                return Err(CollectorReservationFailure {
                    error,
                    held: Held::Observed(Box::new(self)),
                });
            }
        };
        let mut read = OriginalCollectorReservation {
            original: self.attempt.original,
            storage: CollectorReservationStorage::from_original(self.attempt.storage),
            request_raw: self.attempt.raw,
            collector: bytes.to_vec(),
            state: self.state.expect("checked observed state"),
            last: self.last.expect("checked observed clock"),
            raw,
        };
        let mut storage = read.storage;
        // Keep the original storage/fence while borrowing the live owner at each checkpoint.
        let result = storage.publish_with(&read.raw, |at| {
            checkpoint(at)?;
            // No storage inspection through the old one-document owner after creation.
            live_parts(
                read.original,
                &read.request_raw,
                &read.collector,
                &read.state,
                &mut read.last,
                &read.raw,
            )
        });
        read.storage = storage;
        let result = result.and_then(|_| read.revalidate());
        match result {
            Ok(()) => Ok(read),
            Err(error) => {
                read.refuse();
                Err(CollectorReservationFailure {
                    error,
                    held: Held::Publication(Box::new(read)),
                })
            }
        }
    }
}
