//! Bounded three-record byte consistency only; no publication or recovered rights.
use super::{validate_retained_collector_reserved, RequestRecord};
use crate::diagnostic_lease::{validate_transition, DiagnosticState};
use crate::diagnostic_start_consent::{
    start_consent_signing_message, SignedStartConsent, ENVELOPE_SCHEMA, MAX_ENVELOPE_BYTES,
};
use crate::diagnostic_worker_policy::intent_projection::collector::collector_unit_projection;
use crate::{canonical_json, sha256_hex};
use serde::{Deserialize, Serialize};

pub const COLLECTOR_STARTING_SCHEMA: &str = "diagnostic-collector-starting-074-v1";
pub const COLLECTOR_STARTING_NAME: &str = "diagnostic-collector-starting-074.json";
pub const MAX_STARTING_BYTES: usize = 16_384;

#[derive(Debug, thiserror::Error)]
pub enum CollectorStartingRecordError {
    #[error("collector Starting record is malformed, noncanonical or inconsistent")]
    Document,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema_version: String,
    controller_attempt_sha256: String,
    collector_reserved_sha256: String,
    signed_consent_sha256: String,
    signed_consent: SignedStartConsent,
    state: DiagnosticState,
}
fn invalid() -> CollectorStartingRecordError {
    CollectorStartingRecordError::Document
}
fn line(value: &impl Serialize) -> Result<Vec<u8>, CollectorStartingRecordError> {
    let mut raw = canonical_json(value).map_err(|_| invalid())?;
    raw.push(b'\n');
    Ok(raw)
}
fn consent_line(signed: &SignedStartConsent) -> Result<Vec<u8>, CollectorStartingRecordError> {
    let raw = line(signed)?;
    if raw.len() > MAX_ENVELOPE_BYTES
        || signed.schema_version != ENVELOPE_SCHEMA
        || start_consent_signing_message(&signed.consent).is_err()
        || signed.signature_hex.len() != 128
        || !signed
            .signature_hex
            .bytes()
            .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
    {
        return Err(invalid());
    }
    Ok(raw)
}
fn prior(
    request_raw: &[u8],
    reserved_raw: &[u8],
) -> Result<(RequestRecord, super::Record), CollectorStartingRecordError> {
    // The original paired reader owns strict prior-history grammar and bounds.
    validate_retained_collector_reserved(reserved_raw, request_raw).map_err(|_| invalid())?;
    Ok((
        serde_json::from_slice(request_raw).map_err(|_| invalid())?,
        serde_json::from_slice(reserved_raw).map_err(|_| invalid())?,
    ))
}
fn binding(
    record: &Record,
    request_raw: &[u8],
    reserved_raw: &[u8],
) -> Result<(), CollectorStartingRecordError> {
    let (request, reserved) = prior(request_raw, reserved_raw)?;
    let body = &record.signed_consent.consent;
    let signed_raw = consent_line(&record.signed_consent)?;
    if record.schema_version != COLLECTOR_STARTING_SCHEMA
        || record.controller_attempt_sha256 != sha256_hex(request_raw)
        || record.collector_reserved_sha256 != sha256_hex(reserved_raw)
        || record.signed_consent_sha256 != sha256_hex(&signed_raw)
        || body.controller_attempt_sha256 != record.controller_attempt_sha256
        || body.collector_reserved_sha256 != record.collector_reserved_sha256
        || body.reserved_state_sha256
            != sha256_hex(&canonical_json(&reserved.state).map_err(|_| invalid())?)
        || body.collector_unit_sha256
            != sha256_hex(&collector_unit_projection(&reserved.state).map_err(|_| invalid())?)
        || body.reservation_request_id != request.request.request_id
        || body.controller != request.request.controller
        || body.source_commit != request.request.source_commit
        || body.preset != request.request.preset
        || body.worker_policy_sha256 != request.worker_policy_sha256
        || body.worker_policy_epoch != request.policy_epoch
        || canonical_json(&record.state).map_err(|_| invalid())?
            != canonical_json(&body.starting_state).map_err(|_| invalid())?
        || validate_transition(&reserved.state, &record.state).is_err()
    {
        return Err(invalid());
    }
    Ok(())
}
/// Construct untrusted byte evidence, not a file, authenticated owner or permit.
/// Signatures are checked for grammar only, never verified against enrolled keys.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::starting_record::encode_collector_starting_record;
/// fn start(a: &[u8], b: &[u8], c: &[u8]) { encode_collector_starting_record(a, b, c).unwrap().start_once(); }
/// ```
pub fn encode_collector_starting_record(
    request_raw: &[u8],
    reserved_raw: &[u8],
    consent_raw: &[u8],
) -> Result<Vec<u8>, CollectorStartingRecordError> {
    prior(request_raw, reserved_raw)?;
    if consent_raw.is_empty() || consent_raw.len() > MAX_ENVELOPE_BYTES {
        return Err(invalid());
    }
    let signed: SignedStartConsent = serde_json::from_slice(consent_raw).map_err(|_| invalid())?;
    if consent_line(&signed)? != consent_raw {
        return Err(invalid());
    }
    let record = Record {
        schema_version: COLLECTOR_STARTING_SCHEMA.into(),
        controller_attempt_sha256: sha256_hex(request_raw),
        collector_reserved_sha256: sha256_hex(reserved_raw),
        signed_consent_sha256: sha256_hex(consent_raw),
        state: signed.consent.starting_state.clone(),
        signed_consent: signed,
    };
    binding(&record, request_raw, reserved_raw)?;
    let raw = line(&record)?;
    if raw.len() > MAX_STARTING_BYTES {
        return Err(invalid());
    }
    Ok(raw)
}
/// Strict byte consistency only. Reinspection never recovers acknowledgement,
/// signature authority, freshness, replay ownership or a storage/runtime guard.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::starting_record::validate_retained_collector_starting;
/// fn recover(a: &[u8], b: &[u8], c: &[u8]) { validate_retained_collector_starting(a, b, c).unwrap().publish_starting(); }
/// ```
pub fn validate_retained_collector_starting(
    raw: &[u8],
    request_raw: &[u8],
    reserved_raw: &[u8],
) -> Result<(), CollectorStartingRecordError> {
    if raw.is_empty() || raw.len() > MAX_STARTING_BYTES {
        return Err(invalid());
    }
    let record: Record = serde_json::from_slice(raw).map_err(|_| invalid())?;
    if line(&record)? != raw {
        return Err(invalid());
    }
    binding(&record, request_raw, reserved_raw)
}

#[cfg(test)]
#[path = "diagnostic_collector_starting_record_tests.rs"]
mod tests;
