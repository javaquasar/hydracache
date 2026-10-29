//! Static cross-document signature inspection, never original live admission.
//! No key enrollment, file, clock, process, manager or replay slot is created.
use super::*;
use crate::diagnostic_two_phase_launch::held_placement::{
    inspect_placement_signature, SignedPlacement,
};

pub const BODY_SCHEMA: &str = "diagnostic-held-role-binding-074-v4";
pub const ENVELOPE_SCHEMA: &str = "diagnostic-signed-held-role-binding-074-v4";
pub const SIGNATURE_DOMAIN: &[u8] = b"hydracache-diagnostic-held-role-binding-074-v4";
pub const MAX_BODY_BYTES: usize = 40_960;
pub const MAX_ENVELOPE_BYTES: usize = 49_152;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Body {
    pub schema_version: String,
    pub action: String,
    pub placement: SignedPlacement,
    pub held_policy_sha256: String,
    pub held_receipt_sha256: String,
    pub authority_epoch: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedBinding {
    pub schema_version: String,
    pub binding: Body,
    pub signature_hex: String,
}
#[derive(Debug, thiserror::Error)]
pub enum BindingError {
    #[error("held role binding is malformed, noncanonical or over budget")]
    Document,
    #[error("held role binding differs from policy, receipt or placement")]
    Binding,
    #[error("held role signature differs from the independently checked policy controller")]
    Signature,
    #[error("held role provenance refused: {0}")]
    Build(#[from] ArtifactError),
}
fn message(body: &Body) -> Result<Vec<u8>, BindingError> {
    if body.schema_version != BODY_SCHEMA
        || body.action != "bind-held-build-to-placement"
        || !is_hash(&body.held_policy_sha256)
        || !is_hash(&body.held_receipt_sha256)
        || body.authority_epoch == 0
    {
        return Err(BindingError::Document);
    }
    let raw = document_bytes(body)?;
    if raw.len() > MAX_BODY_BYTES {
        return Err(BindingError::Document);
    }
    let mut message = SIGNATURE_DOMAIN.to_vec();
    message.push(0);
    message.extend(raw);
    Ok(message)
}
fn controller(policy: &CheckedPolicy) -> Result<VerifyingKey, BindingError> {
    // This policy was checked against an independent controller and digest.
    // Do not select any verifier from the incoming placement or binding.
    Ok(decode_key(&policy.policy.controller_key_hex)?)
}
fn check(
    body: &Body,
    policy: &CheckedPolicy,
    receipt: &[u8],
) -> Result<(Vec<u8>, VerifiedBuild), BindingError> {
    if receipt.is_empty() || receipt.len() as u64 > MAX_RECEIPT_BYTES {
        return Err(BindingError::Document);
    }
    if body.held_policy_sha256 != policy.digest
        || body.held_receipt_sha256 != sha256_hex(receipt)
        || body.authority_epoch != policy.policy.authority_epoch
    {
        return Err(BindingError::Binding);
    }
    let placement_raw = document_bytes(&body.placement)?;
    let placement = inspect_placement_signature(&placement_raw, &controller(policy)?)
        .map_err(|_| BindingError::Binding)?;
    let consent = &placement.launch_claims.consent_claims;
    if consent.worker_policy_epoch != body.authority_epoch
        || consent.controller.repository_id != REPOSITORY_ID
    {
        return Err(BindingError::Binding);
    }
    let build = verify_receipt(receipt, policy)?;
    build.matches_launch_artifacts(&placement.launch_claims.artifacts)?;
    Ok((placement_raw, build))
}
/// External signing bytes only. Both the old placement and held receipt must
/// already verify under the independently checked policy's respective keys.
pub fn signing_message(
    body: &Body,
    policy: &CheckedPolicy,
    receipt: &[u8],
) -> Result<Vec<u8>, BindingError> {
    let bytes = message(body)?;
    check(body, policy, receipt)?;
    Ok(bytes)
}
fn signature(raw: &str) -> Result<Signature, BindingError> {
    if raw.len() != 128
        || !raw
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(BindingError::Document);
    }
    let mut bytes = [0; 64];
    for (index, pair) in raw.as_bytes().chunks_exact(2).enumerate() {
        let nibble = |b: u8| if b <= b'9' { b - b'0' } else { b - b'a' + 10 };
        bytes[index] = nibble(pair[0]) * 16 + nibble(pair[1]);
    }
    Ok(Signature::from_bytes(&bytes))
}
/// Opaque static inspection, not an admitted policy/epoch or original owner.
/// ```compile_fail,E0599
/// use hydracache_long_run_supervisor_074::diagnostic_builder::held_build::role_binding::Inspection;
/// fn no_start(value: Inspection) {
///     value.start();
/// }
/// ```
/// ```compile_fail,E0616
/// use hydracache_long_run_supervisor_074::diagnostic_builder::held_build::role_binding::Inspection;
/// fn no_extract(value: Inspection) {
///     let _ = value.build;
/// }
/// ```
pub struct Inspection {
    placement: Vec<u8>,
    build: VerifiedBuild,
}
/// Verifies static documents using independently checked policy inputs. Success
/// does not admit those inputs into any original frame, grant replay or renew time.
pub fn inspect_signature(
    raw: &[u8],
    policy: &CheckedPolicy,
    receipt: &[u8],
) -> Result<Inspection, BindingError> {
    if raw.is_empty() || raw.len() > MAX_ENVELOPE_BYTES {
        return Err(BindingError::Document);
    }
    let doc: SignedBinding = serde_json::from_slice(raw).map_err(|_| BindingError::Document)?;
    if doc.schema_version != ENVELOPE_SCHEMA || document_bytes(&doc)? != raw {
        return Err(BindingError::Document);
    }
    controller(policy)?
        .verify_strict(&message(&doc.binding)?, &signature(&doc.signature_hex)?)
        .map_err(|_| BindingError::Signature)?;
    let (placement, build) = check(&doc.binding, policy, receipt)?;
    Ok(Inspection { placement, build })
}
impl Inspection {
    /// Exact signed v3 envelope bytes; no caller-supplied snapshot becomes live.
    pub fn matches_placement_bytes(&self, expected: &[u8]) -> Result<(), BindingError> {
        if self.placement != expected {
            return Err(BindingError::Binding);
        }
        Ok(())
    }
    pub fn verify_contents(&self, contents: &Contents<'_>) -> Result<(), BindingError> {
        Ok(self.build.verify_contents(contents)?)
    }
}
