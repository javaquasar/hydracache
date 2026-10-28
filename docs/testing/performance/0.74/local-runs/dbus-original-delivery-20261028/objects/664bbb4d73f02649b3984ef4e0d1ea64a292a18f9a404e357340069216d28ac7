//! Original request retention only. Bytes never reconstruct acknowledgement or start rights.
use super::{ControllerFenceError, FencedControllerPreflight, OidcControllerInput, OidcError};
use crate::diagnostic_ipc::{signing_message, DiagnosticOperation, DiagnosticRequest};
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::attempt_storage::AttemptStorage;
pub use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::attempt_storage::{AttemptStage, StorageAttemptError};
use crate::{canonical_json, is_hash, sha256_hex};
use serde::{Deserialize, Serialize};
use std::fmt;

pub const CONTROLLER_ATTEMPT_SCHEMA: &str = "diagnostic-controller-attempt-074-v1";
const LIMIT: usize = 16_384;

#[path = "diagnostic_original_collector.rs"]
pub mod collector_preflight;

#[derive(Debug, thiserror::Error)]
pub enum ControllerAttemptError {
    #[error("original controller attempt permanently refused")]
    Refused,
    #[error("original controller preflight refused: {0}")]
    Preflight(#[from] ControllerFenceError),
    #[error("original controller chain refused: {0}")]
    Controller(#[from] OidcError),
    #[error("original controller attempt storage refused: {0}")]
    Storage(#[from] StorageAttemptError),
    #[error("original controller attempt binding differs")]
    Binding,
    #[error("retained controller attempt document refused")]
    Document,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema_version: String,
    request: DiagnosticRequest,
    request_sha256: String,
    token_sha256: String,
    builder_policy_sha256: String,
    build_receipt_sha256: String,
    controller_trust_sha256: String,
    oidc_trust_sha256: String,
    worker_trust_sha256: String,
    worker_policy_sha256: String,
    policy_epoch: u64,
    authority_revision: u64,
}
fn encode(record: &Record) -> Result<Vec<u8>, ControllerAttemptError> {
    let mut raw = canonical_json(record).map_err(|_| ControllerAttemptError::Document)?;
    raw.push(b'\n');
    if raw.len() > LIMIT {
        return Err(ControllerAttemptError::Document);
    }
    Ok(raw)
}
/// Strict bounded byte consistency only, not signature verification or recovered admission.
pub fn validate_retained_controller_attempt(raw: &[u8]) -> Result<(), ControllerAttemptError> {
    if raw.is_empty() || raw.len() > LIMIT {
        return Err(ControllerAttemptError::Document);
    }
    let r: Record = serde_json::from_slice(raw).map_err(|_| ControllerAttemptError::Document)?;
    if r.schema_version != CONTROLLER_ATTEMPT_SCHEMA
        || encode(&r)? != raw
        || signing_message(&r.request).is_err()
        || r.request.operation != DiagnosticOperation::Reserve
        || r.request.expected_state_revision != 0
        || r.request_sha256
            != sha256_hex(
                &canonical_json(&r.request).map_err(|_| ControllerAttemptError::Document)?,
            )
        || r.build_receipt_sha256 != r.request.identity.build_provenance_sha256
        || r.policy_epoch == 0
        || r.authority_revision == 0
        || r.authority_revision
            > crate::diagnostic_worker_policy::authority_registry::MAX_GENERATIONS as u64
        || [
            &r.token_sha256,
            &r.builder_policy_sha256,
            &r.build_receipt_sha256,
            &r.controller_trust_sha256,
            &r.oidc_trust_sha256,
            &r.worker_trust_sha256,
            &r.worker_policy_sha256,
        ]
        .iter()
        .any(|hash| !is_hash(hash))
    {
        return Err(ControllerAttemptError::Document);
    }
    Ok(())
}
fn capture(
    original: &mut OidcControllerInput<'_, '_, '_, '_, '_, '_, '_, '_>,
) -> Result<Vec<u8>, ControllerAttemptError> {
    original.revalidate()?;
    let (worker_policy_sha256, policy_epoch, authority_revision, worker_trust_sha256) = original
        .original
        .original
        .worker
        .controller_authority_binding()
        .map_err(OidcError::from)?;
    let record = Record {
        schema_version: CONTROLLER_ATTEMPT_SCHEMA.into(),
        request: original.original.original.request.clone(),
        request_sha256: original.original.original.request_sha256.clone(),
        token_sha256: original.token_digest.clone(),
        builder_policy_sha256: sha256_hex(&original.original.policy_bytes),
        build_receipt_sha256: sha256_hex(&original.original.receipt_bytes),
        controller_trust_sha256: sha256_hex(
            &original.original.original.controller.source.snapshot.raw,
        ),
        oidc_trust_sha256: sha256_hex(&original.trust.source.snapshot.raw),
        worker_trust_sha256,
        worker_policy_sha256,
        policy_epoch,
        authority_revision,
    };
    let raw = encode(&record)?;
    validate_retained_controller_attempt(&raw)?;
    original.revalidate()?;
    Ok(raw)
}
fn latch(
    original: &mut OidcControllerInput<'_, '_, '_, '_, '_, '_, '_, '_>,
    storage: &mut AttemptStorage<'_>,
) {
    storage.latch_refusal();
    original.trust.refused = true;
    original.original.original.latch_refusal();
}

pub struct ControllerAttemptFailure<'storage> {
    error: ControllerAttemptError,
    storage: Box<AttemptStorage<'storage>>,
}
impl ControllerAttemptFailure<'_> {
    pub fn error(&self) -> &ControllerAttemptError {
        &self.error
    }
    pub fn is_refused(&self) -> bool {
        self.storage.is_refused()
    }
}
impl fmt::Debug for ControllerAttemptFailure<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}
impl fmt::Display for ControllerAttemptFailure<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}
impl std::error::Error for ControllerAttemptFailure<'_> {}

/// Original runtime acknowledgement, not permission to execute or publish lifecycle state.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::ControllerRequestAttempt;
/// fn copy(g: ControllerRequestAttempt<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { g.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::ControllerRequestAttempt;
/// fn start(mut g: ControllerRequestAttempt<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { g.start_once(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::ControllerRequestAttempt;
/// fn extract(g: ControllerRequestAttempt<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { let _ = g.original; }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::ControllerRequestAttempt;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<ControllerRequestAttempt<'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static>>();
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::ControllerRequestAttempt;
/// fn send(g: ControllerRequestAttempt<'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static>) { std::thread::spawn(move || drop(g)); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::ControllerRequestAttempt;
/// fn replace(mut g: ControllerRequestAttempt<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { g.revalidate(b"replacement"); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::FencedControllerPreflight;
/// fn replay(g: FencedControllerPreflight<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
///     let first = g.retain_request();
///     let second = g.retain_request();
/// }
/// ```
pub struct ControllerRequestAttempt<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    original: &'join mut OidcControllerInput<'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    storage: AttemptStorage<'storage>,
    raw: Vec<u8>,
}
impl fmt::Debug for ControllerRequestAttempt<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ControllerRequestAttempt")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
impl ControllerRequestAttempt<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    #[cfg(test)]
    pub(in super::super) fn simulate_binding_drift_for_test(&mut self) {
        self.raw[0] ^= 1;
    }
    pub fn is_refused(&self) -> bool {
        self.original.is_refused() || self.storage.is_refused()
    }
    pub fn revalidate(&mut self) -> Result<(), ControllerAttemptError> {
        if self.is_refused() {
            latch(self.original, &mut self.storage);
            return Err(ControllerAttemptError::Refused);
        }
        let result = (|| {
            self.storage.revalidate()?;
            if capture(self.original)? != self.raw {
                return Err(ControllerAttemptError::Binding);
            }
            self.storage.revalidate()?;
            if capture(self.original)? != self.raw {
                return Err(ControllerAttemptError::Binding);
            }
            self.storage.revalidate()?;
            Ok(())
        })();
        if result.is_err() {
            latch(self.original, &mut self.storage);
        }
        result
    }
}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    FencedControllerPreflight<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub fn retain_request(
        self,
    ) -> Result<
        ControllerRequestAttempt<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        ControllerAttemptFailure<'storage>,
    > {
        self.retain_request_with(|_| Ok(()))
    }
    pub(in super::super) fn retain_request_with(
        mut self,
        mut checkpoint: impl FnMut(AttemptStage) -> Result<(), ControllerAttemptError>,
    ) -> Result<
        ControllerRequestAttempt<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        ControllerAttemptFailure<'storage>,
    > {
        let captured = self
            .revalidate()
            .map_err(ControllerAttemptError::from)
            .and_then(|_| capture(self.original));
        let mut attempt = ControllerRequestAttempt {
            original: self.original,
            storage: AttemptStorage::for_controller(self.fence),
            raw: Vec::new(),
        };
        let result = (|| {
            attempt.raw = captured?;
            attempt.storage.retain_with(&attempt.raw, |at| {
                checkpoint(at)?;
                if capture(attempt.original)? != attempt.raw {
                    return Err(ControllerAttemptError::Binding);
                }
                Ok(())
            })?;
            attempt.revalidate()
        })();
        match result {
            Ok(()) => Ok(attempt),
            Err(error) => {
                latch(attempt.original, &mut attempt.storage);
                Err(ControllerAttemptFailure {
                    error,
                    storage: Box::new(attempt.storage),
                })
            }
        }
    }
}
