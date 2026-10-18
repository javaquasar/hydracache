//! Signed expected two-phase requirements, not an executable unit or live permit.
//!
//! The nested v1 consent is a historical claim. A new outer signature binds a
//! different protocol; neither verifier upgrades the other schema. Supplied-key
//! inspection authenticates no enrolled issuer, peer, clock, channel or process.
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_two_phase_launch::LaunchBody;
//! fn start(body: LaunchBody) { body.start_once(); }
//! ```
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_two_phase_launch::LaunchBody;
//! fn release(body: LaunchBody) { body.release_barrier(); }
//! ```
use crate::diagnostic_start_consent::{start_consent_signing_message, StartConsentBody};
use crate::{canonical_json, is_hash, sha256_hex};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub const REQUIREMENTS_SCHEMA: &str = "diagnostic-two-phase-launch-requirements-074-v2";
pub const BODY_SCHEMA: &str = "diagnostic-two-phase-launch-074-v2";
pub const ENVELOPE_SCHEMA: &str = "diagnostic-signed-two-phase-launch-074-v2";
pub const SIGNATURE_DOMAIN: &[u8] = b"hydracache-diagnostic-two-phase-launch-074-v2";
pub const MAX_BODY_BYTES: usize = 16384;
pub const MAX_ENVELOPE_BYTES: usize = 32768;

// Kernel mechanics only. No constructor from signed bytes or original owners.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[allow(dead_code)]
#[path = "diagnostic_held_barrier.rs"]
mod held_barrier;

/// Expected artifacts only; hashes are not opened executable or custody proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaunchArtifacts {
    pub launcher_sha256: String,
    pub watchdog_sha256: String,
    pub collector_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaunchBody {
    pub schema_version: String,
    pub action: String,
    pub consent_claims: StartConsentBody,
    pub artifacts: LaunchArtifacts,
    pub requirements_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedLaunch {
    pub schema_version: String,
    pub launch: LaunchBody,
    pub signature_hex: String,
}
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LaunchError {
    #[error("two-phase launch document is malformed, noncanonical or over budget")]
    Document,
    #[error("two-phase launch signature differs from the supplied public key")]
    Signature,
    #[error("two-phase launch requirements digest differs from its signed claims")]
    Binding,
}
fn line(value: &impl Serialize) -> Result<Vec<u8>, LaunchError> {
    let mut raw = canonical_json(value).map_err(|_| LaunchError::Document)?;
    raw.push(b'\n');
    Ok(raw)
}
/// Deterministic expected semantics, deliberately NOT a TransientUnitSpec.
/// No timestamp in these claims substitutes for the original live clock frame.
pub fn launch_requirements(
    consent: &StartConsentBody,
    artifacts: &LaunchArtifacts,
) -> Result<Vec<u8>, LaunchError> {
    // Reuse only the unchanged grammar, not v1 launch authority or signatures.
    start_consent_signing_message(consent).map_err(|_| LaunchError::Document)?;
    if [
        &artifacts.launcher_sha256,
        &artifacts.watchdog_sha256,
        &artifacts.collector_sha256,
    ]
    .iter()
    .any(|hash| !is_hash(hash))
    {
        return Err(LaunchError::Document);
    }
    line(&json!({
        "schema_version": REQUIREMENTS_SCHEMA,
        "consent_claims_sha256": sha256_hex(&line(consent)?),
        "legacy_collector_unit_sha256": consent.collector_unit_sha256,
        "worker_executable_sha256": consent.starting_state.identity.binary_sha256,
        "artifacts": artifacts,
        "protocol": {
            "order": [
                "independent-watchdog-armed-before-first-delivery",
                "held-launcher-delivery-through-original-fixed-manager-channel",
                "original-executable-streams-process-and-complete-cgroup-bound",
                "unchanged-complete-live-validation-frame",
                "one-shot-barrier-release"
            ],
            "before_release": "no-worker-execution-or-product-output",
            "release": "one-shot-after-original-live-binding-and-complete-frame",
            "unknown_outcome": "retain-incomplete-attempt-and-fence-no-resend",
            "watchdog": "independent-of-controller-and-collector-io",
            "revocation": "pinned-complete-cgroup-and-all-external-writer-copies",
            "fence_release_requires": [
                "revocation-completed-and-pinned-cgroup-empty",
                "creator-manager-and-all-external-writer-copies-closed",
                "both-kernel-read-eofs-after-bounded-drain",
                "sole-root-collector-byte-and-hash-reconciliation",
                "files-and-directory-synced-and-all-evidence-writers-closed"
            ]
        },
        "deadlines": {
            "origins": ["original-reservation", "original-cell", "authenticated-controller-receive"],
            "seconds": [crate::diagnostic_lease::TOTAL_SECONDS,
                crate::diagnostic_lease::CELL_SECONDS, crate::diagnostic_lease::CONTROLLER_LOSS_SECONDS],
            "selection": "minimum-of-original-absolute-boottime-bounds",
            "renewal": false,
            "signed_timestamps_are_live_observations": false
        }
    }))
}
/// External signing bytes only; no private key or runtime operation is accessed.
pub fn launch_signing_message(body: &LaunchBody) -> Result<Vec<u8>, LaunchError> {
    if body.schema_version != BODY_SCHEMA
        || body.action != "prepare-held-cell-one"
        || !is_hash(&body.requirements_sha256)
    {
        return Err(LaunchError::Document);
    }
    let requirements = launch_requirements(&body.consent_claims, &body.artifacts)?;
    if sha256_hex(&requirements) != body.requirements_sha256 {
        return Err(LaunchError::Binding);
    }
    let raw = line(body)?;
    if raw.len() > MAX_BODY_BYTES {
        return Err(LaunchError::Document);
    }
    let mut message = SIGNATURE_DOMAIN.to_vec();
    message.push(0);
    message.extend(raw);
    Ok(message)
}
fn signature(raw: &str) -> Result<Signature, LaunchError> {
    if raw.len() != 128
        || !raw
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(LaunchError::Document);
    }
    let mut bytes = [0; 64];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte =
            u8::from_str_radix(&raw[2 * i..2 * i + 2], 16).map_err(|_| LaunchError::Document)?;
    }
    Ok(Signature::from_bytes(&bytes))
}
/// Inspect expected bytes using a supplied key, NOT production trust selection.
pub fn inspect_launch_signature(raw: &[u8], key: &VerifyingKey) -> Result<LaunchBody, LaunchError> {
    if raw.is_empty() || raw.len() > MAX_ENVELOPE_BYTES {
        return Err(LaunchError::Document);
    }
    let doc: SignedLaunch = serde_json::from_slice(raw).map_err(|_| LaunchError::Document)?;
    if doc.schema_version != ENVELOPE_SCHEMA || line(&doc)? != raw {
        return Err(LaunchError::Document);
    }
    key.verify_strict(
        &launch_signing_message(&doc.launch)?,
        &signature(&doc.signature_hex)?,
    )
    .map_err(|_| LaunchError::Signature)?;
    Ok(doc.launch)
}

#[cfg(test)]
#[path = "diagnostic_two_phase_launch_tests.rs"]
mod tests;
