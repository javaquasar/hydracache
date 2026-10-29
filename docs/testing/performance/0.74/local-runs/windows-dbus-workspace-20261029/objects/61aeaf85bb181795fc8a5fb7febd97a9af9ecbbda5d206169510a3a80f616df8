//! Operator-selected signer issuer only, NOT prebuilt/runtime/job or execution authority.
//! The historic generation inventory is not a trust configuration or loader.
use crate::diagnostic_artifacts::ArtifactError;
use crate::diagnostic_builder::decode_key;
use crate::diagnostic_reviewed_trust::{
    BUILDER_KEY_HEX, CONTROLLER_KEY_HEX, WORKER_ISSUER_KEY_HEX,
};
use crate::diagnostic_signer_build::{AssertedSignerBuildSelection, SignerBuildStatement};
use crate::sha256_hex;
use ed25519_dalek::VerifyingKey;

/// Independently selected by the operator on 2026-10-15; no caller override.
pub const KEY_HEX: &str = "08f18b0078ad4d754ed615682ec0c9c076109965548d9c674ac606e3625f79ea";
pub const RAW_PUBLIC_SHA256: &str =
    "0c22ba7d6c6d16b334e4b1d4a0ce161117800135a977aa30020906caa70af884";

/// Fixed key selection only. Does not attest revocation, an artifact, runtime or job.
pub fn issuer_key() -> Result<VerifyingKey, ArtifactError> {
    let key = decode_key(KEY_HEX)?;
    if key.is_weak() || sha256_hex(key.as_bytes()) != RAW_PUBLIC_SHA256 {
        return Err(ArtifactError::Invalid);
    }
    for old in [CONTROLLER_KEY_HEX, BUILDER_KEY_HEX, WORKER_ISSUER_KEY_HEX] {
        if key == decode_key(old)? {
            return Err(ArtifactError::Invalid);
        }
    }
    Ok(key)
}

/// Bind the selected issuer to caller-ASSERTED expected build facts.
/// Expectations still need independent admission; no bootstrap or signing consumer.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_signer_issuer::asserted_build_selection;
/// use hydracache_long_run_supervisor_074::diagnostic_signer_build::SignerBuildStatement;
/// fn override_key(key: ed25519_dalek::VerifyingKey, statement: SignerBuildStatement) {
///     asserted_build_selection(key, statement);
/// }
/// ```
pub fn asserted_build_selection(
    expected: SignerBuildStatement,
) -> Result<AssertedSignerBuildSelection, ArtifactError> {
    AssertedSignerBuildSelection::new(issuer_key()?, expected)
}
