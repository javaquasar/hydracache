//! Explicit collector start consent data, never kernel/lifecycle/start authority.
//! Successful signature inspection does not enroll its supplied key or check
//! freshness, original objects, replay, peer credentials or any execution path.
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_start_consent::StartConsentBody;
//! fn start(body: StartConsentBody) { body.start_once(); }
//! ```
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_start_consent::SignedStartConsent;
//! fn publish(doc: SignedStartConsent) { doc.publish_starting(); }
//! ```
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_start_consent::StartConsentBody;
//! fn release(body: StartConsentBody) { body.release_resources(); }
//! ```
use crate::diagnostic_ipc::{DiagnosticController, PRESET};
use crate::diagnostic_lease::{validate_state, DiagnosticStage, DiagnosticState, SOURCE_COMMIT};
use crate::{canonical_json, is_hash};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

pub const BODY_SCHEMA: &str = "diagnostic-collector-start-consent-074-v1";
pub const ENVELOPE_SCHEMA: &str = "diagnostic-signed-collector-start-consent-074-v1";
pub const SIGNATURE_DOMAIN: &[u8] = b"hydracache-diagnostic-collector-start-consent-074-v1";
pub const MAX_BODY_BYTES: usize = 4096;
pub const MAX_ENVELOPE_BYTES: usize = 8192;

/// Untrusted signed claims; timestamps and hashes are not observations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartConsentBody {
    pub schema_version: String,
    pub action: String,
    pub consent_request_id: String,
    pub reservation_request_id: String,
    pub controller_attempt_sha256: String,
    pub collector_reserved_sha256: String,
    pub reserved_state_sha256: String,
    pub challenge_nonce_sha256: String,
    pub authenticated_response_sha256: String,
    pub worker_policy_sha256: String,
    pub worker_policy_epoch: u64,
    pub collector_unit_sha256: String,
    pub controller: DiagnosticController,
    pub source_commit: String,
    pub preset: String,
    pub starting_state: DiagnosticState,
    pub issued_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedStartConsent {
    pub schema_version: String,
    pub consent: StartConsentBody,
    pub signature_hex: String,
}
#[derive(Debug, thiserror::Error)]
pub enum StartConsentError {
    #[error("collector start consent is malformed, noncanonical or over budget")]
    Document,
    #[error("collector start consent signature differs from the supplied public key")]
    Signature,
}
fn line(value: &impl Serialize) -> Result<Vec<u8>, StartConsentError> {
    let mut raw = canonical_json(value).map_err(|_| StartConsentError::Document)?;
    raw.push(b'\n');
    Ok(raw)
}
fn body_line(body: &StartConsentBody) -> Result<Vec<u8>, StartConsentError> {
    let s = &body.starting_state;
    let window = body
        .expires_at_unix_seconds
        .checked_sub(body.issued_at_unix_seconds);
    if body.schema_version != BODY_SCHEMA
        || body.action != "start-cell-one"
        || body.source_commit != SOURCE_COMMIT
        || body.preset != PRESET
        || body.worker_policy_epoch == 0
        || body.issued_at_unix_seconds == 0
        || !matches!(window, Some(1..=60))
        || body.controller.repository_id == 0
        || body.controller.run_id == 0
        || body.controller.run_attempt == 0
        || body.controller.actor_id == 0
        || body.consent_request_id == body.reservation_request_id
        || [
            &body.consent_request_id,
            &body.reservation_request_id,
            &body.controller_attempt_sha256,
            &body.collector_reserved_sha256,
            &body.reserved_state_sha256,
            &body.challenge_nonce_sha256,
            &body.authenticated_response_sha256,
            &body.worker_policy_sha256,
            &body.collector_unit_sha256,
        ]
        .iter()
        .any(|h| !is_hash(h))
        || validate_state(s).is_err()
        || s.stage != DiagnosticStage::Starting
        || s.revision != 2
        || s.completed_cells != 0
        || s.reserved_monotonic_ns == 0
        || s.controller_monotonic_ns <= s.reserved_monotonic_ns
        || s.cell_started_monotonic_ns != Some(s.last_observed_monotonic_ns)
        || s.cgroup_inode.is_some()
        || s.reason.is_some()
        || s.cleanup_confirmed
        || s.last_observed_monotonic_ns
            .checked_sub(s.controller_monotonic_ns)
            .filter(|age| *age < crate::diagnostic_lease::CONTROLLER_LOSS_SECONDS * 1_000_000_000)
            .is_none()
        || s.last_observed_monotonic_ns
            .checked_sub(s.reserved_monotonic_ns)
            .filter(|age| *age < crate::diagnostic_lease::TOTAL_SECONDS * 1_000_000_000)
            .is_none()
    {
        return Err(StartConsentError::Document);
    }
    let raw = line(body)?;
    if raw.len() > MAX_BODY_BYTES {
        return Err(StartConsentError::Document);
    }
    Ok(raw)
}
/// Bytes for an external signer. No private key is read or generated.
pub fn start_consent_signing_message(
    body: &StartConsentBody,
) -> Result<Vec<u8>, StartConsentError> {
    let raw = body_line(body)?;
    let mut message = SIGNATURE_DOMAIN.to_vec();
    message.push(0);
    message.extend(raw);
    Ok(message)
}
fn signature(raw: &str) -> Result<Signature, StartConsentError> {
    if raw.len() != 128
        || !raw
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(StartConsentError::Document);
    }
    let mut bytes = [0; 64];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&raw[2 * i..2 * i + 2], 16)
            .map_err(|_| StartConsentError::Document)?;
    }
    Ok(Signature::from_bytes(&bytes))
}
/// Byte/signature inspection with a supplied key, NOT enrolled trust selection.
/// No actual clock, kernel peer, original chain, replay or launch is checked.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_start_consent::inspect_start_consent_signature;
/// fn permit(raw: &[u8], key: &ed25519_dalek::VerifyingKey) {
///     inspect_start_consent_signature(raw, key).unwrap().start_once();
/// }
/// ```
pub fn inspect_start_consent_signature(
    raw: &[u8],
    key: &VerifyingKey,
) -> Result<StartConsentBody, StartConsentError> {
    if raw.is_empty() || raw.len() > MAX_ENVELOPE_BYTES {
        return Err(StartConsentError::Document);
    }
    let document: SignedStartConsent =
        serde_json::from_slice(raw).map_err(|_| StartConsentError::Document)?;
    if document.schema_version != ENVELOPE_SCHEMA || line(&document)? != raw {
        return Err(StartConsentError::Document);
    }
    let message = start_consent_signing_message(&document.consent)?;
    key.verify_strict(&message, &signature(&document.signature_hex)?)
        .map_err(|_| StartConsentError::Signature)?;
    Ok(document.consent)
}
