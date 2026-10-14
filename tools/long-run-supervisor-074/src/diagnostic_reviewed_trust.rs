//! Public pins selected in the reviewed 2026-10-09 enrollment, not live authority.
//! No key generation, secret access, worker trust default or production route.
use crate::diagnostic_artifacts::ArtifactError;
use crate::diagnostic_builder::{self, CheckedBuilderPolicy};
use crate::diagnostic_ipc::DiagnosticPolicy;
use ed25519_dalek::VerifyingKey;

pub const CONTROLLER_KEY_HEX: &str =
    "c56f89595c45d0656225f1b8915babed046d278dc9ce1908c6bcb6e9c93e2fda";
pub const BUILDER_KEY_HEX: &str =
    "39215088b0102dffb987b937efa9f49a6c5c7704192fa1f16b4e19ccad12c3ba";
pub const BUILDER_POLICY_SHA256: &str =
    "dfe81cf770088029e03f04faa4da429ef163583c0ba1b7002b8ec5606e766e22";

/// Decode the previously approved public key, without a caller-selected override.
/// This does not attest current revocation status or an installed controller.
pub fn controller_key() -> Result<VerifyingKey, ArtifactError> {
    diagnostic_builder::decode_key(CONTROLLER_KEY_HEX)
}

/// Verify supplied policy bytes against independently retained fixed pins.
/// The returned existing guard authenticates build statements, not execution.
/// No filename, environment variable or incoming document can select the verifier.
pub fn load_builder_policy(bytes: &[u8]) -> Result<CheckedBuilderPolicy, ArtifactError> {
    diagnostic_builder::load_policy(bytes, BUILDER_POLICY_SHA256, &controller_key()?)
}

/// Key/repository selection only; principal allowlists remain independently supplied.
/// A match grants no replay, epoch, host, start or cleanup authority.
pub fn require_controller_pin(policy: &DiagnosticPolicy) -> Result<(), ArtifactError> {
    if policy.repository_id != diagnostic_builder::REPOSITORY_ID
        || policy.verifying_key != controller_key()?
    {
        return Err(ArtifactError::Invalid);
    }
    Ok(())
}
