//! Signed expected placement only, not a held unit, live root or execution permit.
//! Nested v2 claims are authenticated by the new outer signature, not upgraded.
//! Supplied-key inspection does not select an enrolled issuer or observe a host.
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_two_phase_launch::held_placement::PlacementBody;
//! fn start(body: PlacementBody) { body.start_once(); }
//! ```
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_two_phase_launch::held_placement::PlacementBody;
//! fn release(body: PlacementBody) { body.release_barrier(); }
//! ```
use super::root_enrollment::{inspect_packet, Kind, PacketClaims, RootBindingClaims, PACKET_BYTES};
use super::{launch_signing_message, LaunchBody};
use crate::{canonical_json, is_hash, sha256_hex};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

pub const REQUIREMENTS_SCHEMA: &str = "diagnostic-held-placement-requirements-074-v3";
pub const BODY_SCHEMA: &str = "diagnostic-held-placement-074-v3";
pub const ENVELOPE_SCHEMA: &str = "diagnostic-signed-held-placement-074-v3";
pub const PLACEMENT_DOMAIN: &[u8] = b"hydracache-diagnostic-held-placement-074-v3";
pub const MAX_BODY_BYTES: usize = 16384;
pub const MAX_ENVELOPE_BYTES: usize = 32768;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlacementBody {
    pub schema_version: String,
    pub action: String,
    pub root_enrollment_hex: String,
    pub requirements_sha256: String,
    pub launch_claims: LaunchBody,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedPlacement {
    pub schema_version: String,
    pub placement: PlacementBody,
    pub signature_hex: String,
}
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PlacementError {
    #[error("held placement document is malformed, noncanonical or over budget")]
    Document,
    #[error("held placement signature differs from the supplied public key")]
    Signature,
    #[error("held placement requirements or attempt binding differs")]
    Binding,
}
fn line(value: &impl Serialize) -> Result<Vec<u8>, PlacementError> {
    let mut bytes = canonical_json(value).map_err(|_| PlacementError::Document)?;
    bytes.push(b'\n');
    Ok(bytes)
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn decode<const N: usize>(raw: &str) -> Result<[u8; N], PlacementError> {
    if raw.len() != 2 * N
        || !raw
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(PlacementError::Document);
    }
    let mut bytes = [0; N];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&raw[2 * index..2 * index + 2], 16)
            .map_err(|_| PlacementError::Document)?;
    }
    Ok(bytes)
}
/// Derive expectations from the same original identity and exact v2 Enroll bytes.
/// Neither an inode nor a signed timestamp is a live kernel observation.
pub fn placement_requirements(
    launch: &LaunchBody,
    root: &RootBindingClaims,
) -> Result<Vec<u8>, PlacementError> {
    let launch_message = launch_signing_message(launch).map_err(|_| PlacementError::Document)?;
    let packet = PacketClaims {
        kind: Kind::Enroll,
        binding: *root,
    }
    .encode()
    .map_err(|_| PlacementError::Document)?;
    // Keep the existing serde tuple bytes, not a newly canonicalized identity.
    let attempt: [u8; 32] = Sha256::digest(
        serde_json::to_vec(&(
            "hc074-original-root-attempt-v1",
            &launch.consent_claims.starting_state.identity,
        ))
        .map_err(|_| PlacementError::Document)?,
    )
    .into();
    if root.attempt_sha256 != attempt {
        return Err(PlacementError::Binding);
    }
    let slice = format!("system-hc074{}.slice", hex(&attempt));
    let root_path = format!("/system.slice/{slice}");
    let unit = crate::diagnostic_lease::cell_intent(&launch.consent_claims.starting_state)
        .map_err(|_| PlacementError::Document)?
        .unit_name;
    line(&json!({
        "schema_version": REQUIREMENTS_SCHEMA,
        "launch_signing_message_sha256": sha256_hex(&launch_message),
        "root_enrollment_hex": hex(&packet),
        "placement": {
            "slice_unit_name": slice,
            "slice_parent_unit_name": "system.slice",
            "expected_root_control_group": root_path,
            "held_unit_name": unit,
            "expected_held_control_group": format!("{root_path}/{unit}"),
            "exclusive_population_required": true,
            "guardians_and_collectors_outside_attempt_root_required": true,
            "actual_placement_ownership_and_inodes_must_match": true
        },
        "absolute_deadline_boottime_ns": root.deadline_boottime_ns,
        "renewal": false,
        "admission": {
            "root_is_enrolled": false,
            "held_unit_is_projected": false,
            "original_send_allowed": false,
            "barrier_release_allowed": false,
            "completion_allowed": false,
            "fence_release_allowed": false
        }
    }))
}
/// External signing bytes only. No old signature or supplied ACK grants rights.
pub fn placement_signing_message(body: &PlacementBody) -> Result<Vec<u8>, PlacementError> {
    if body.schema_version != BODY_SCHEMA
        || body.action != "project-held-cell-one"
        || !is_hash(&body.requirements_sha256)
    {
        return Err(PlacementError::Document);
    }
    let packet = inspect_packet(&decode::<PACKET_BYTES>(&body.root_enrollment_hex)?)
        .map_err(|_| PlacementError::Document)?;
    if packet.kind != Kind::Enroll {
        return Err(PlacementError::Document);
    }
    let requirements = placement_requirements(&body.launch_claims, &packet.binding)?;
    if sha256_hex(&requirements) != body.requirements_sha256 {
        return Err(PlacementError::Binding);
    }
    let raw = line(body)?;
    if raw.len() > MAX_BODY_BYTES {
        return Err(PlacementError::Document);
    }
    let mut message = PLACEMENT_DOMAIN.to_vec();
    message.push(0);
    message.extend(raw);
    Ok(message)
}
/// Supplied-key byte inspection, not production policy, enrollment or admission.
pub fn inspect_placement_signature(
    raw: &[u8],
    key: &VerifyingKey,
) -> Result<PlacementBody, PlacementError> {
    if raw.is_empty() || raw.len() > MAX_ENVELOPE_BYTES {
        return Err(PlacementError::Document);
    }
    let doc: SignedPlacement = serde_json::from_slice(raw).map_err(|_| PlacementError::Document)?;
    if doc.schema_version != ENVELOPE_SCHEMA || line(&doc)? != raw {
        return Err(PlacementError::Document);
    }
    key.verify_strict(
        &placement_signing_message(&doc.placement)?,
        &Signature::from_bytes(&decode(&doc.signature_hex)?),
    )
    .map_err(|_| PlacementError::Signature)?;
    Ok(doc.placement)
}
