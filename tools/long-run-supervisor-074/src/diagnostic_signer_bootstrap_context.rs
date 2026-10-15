//! Structured bootstrap ASSERTIONS, not observed launcher/job protection or enrollment.
use crate::diagnostic_artifacts::{ArtifactDigest, ArtifactError};
use crate::diagnostic_signer_build::{SignerBuildStatement, MAX_BINARY_BYTES, MAX_CONTEXT_BYTES};
use serde::{Deserialize, Serialize};

pub const SCHEMA: &str = "diagnostic-signer-bootstrap-context-074-v1";
pub const WORKFLOW_REF: &str = "javaquasar/hydracache/.github/workflows/diagnostic-controller-074.yml@refs/heads/feat/0.74-resp-native-throughput";
pub const ENVIRONMENT: &str = "performance-diagnostic-controller-074";
pub const SUBJECT: &str =
    "repo:javaquasar/hydracache:environment:performance-diagnostic-controller-074";

/// Serializable requirements, never proof that a job/launcher obeys them.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssertedBootstrapContext {
    pub schema_version: String,
    pub repository_id: u64,
    pub source_commit: String,
    pub source_tree: String,
    pub binary: ArtifactDigest,
    pub runtime_manifest: ArtifactDigest,
    pub launcher: ArtifactDigest,
    pub workflow_ref: String,
    pub workflow_sha: String,
    pub environment: String,
    pub subject: String,
    pub event_name: String,
    pub runner_environment: String,
    pub build_has_signing_authority: bool,
    pub build_has_oidc_authority: bool,
    pub signer_compilation_allowed: bool,
    pub signer_dependency_install_allowed: bool,
    pub signer_checkout_allowed: bool,
    pub inherits_unreviewed_environment: bool,
}

pub(crate) fn check(raw: &[u8], expected: &SignerBuildStatement) -> Result<(), ArtifactError> {
    if raw.is_empty()
        || raw.len() as u64 > MAX_CONTEXT_BYTES
        || raw.len() as u64 != expected.bootstrap_context.bytes
    {
        return Err(ArtifactError::Contents);
    }
    if crate::sha256_hex(raw) != expected.bootstrap_context.sha256 {
        return Err(ArtifactError::Contents);
    }
    let c: AssertedBootstrapContext = serde_json::from_slice(raw)?;
    let mut canonical = crate::canonical_json(&c)?;
    canonical.push(b'\n');
    let source_id = |s: &str| {
        s.len() == 40
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && s.bytes().any(|b| b != b'0')
    };
    if canonical != raw
        || c.schema_version != SCHEMA
        || c.repository_id != expected.repository_id
        || c.source_commit != expected.source_commit
        || c.source_tree != expected.source_tree
        || !source_id(&c.source_commit)
        || !source_id(&c.source_tree)
        || c.binary != expected.binary
        || c.runtime_manifest != expected.runtime_manifest
        || !crate::is_hash(&c.launcher.sha256)
        || c.launcher.sha256.bytes().all(|b| b == b'0')
        || c.launcher.sha256 == expected.binary.sha256
        || c.launcher.bytes == 0
        || c.launcher.bytes > MAX_BINARY_BYTES
        || c.workflow_ref != WORKFLOW_REF
        || !source_id(&c.workflow_sha)
        || c.environment != ENVIRONMENT
        || c.subject != SUBJECT
        || c.event_name != "workflow_dispatch"
        || c.runner_environment != "github-hosted"
        || c.build_has_signing_authority
        || c.build_has_oidc_authority
        || c.signer_compilation_allowed
        || c.signer_dependency_install_allowed
        || c.signer_checkout_allowed
        || c.inherits_unreviewed_environment
    {
        return Err(ArtifactError::Invalid);
    }
    Ok(())
}

/// Receipt/content/profile consistency only; no acquisition, bootstrap or signing authority.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_signer_bootstrap_context::CheckedSignerBootstrapModel;
/// fn duplicate(m: CheckedSignerBootstrapModel) { let _ = m.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_signer_bootstrap_context::CheckedSignerBootstrapModel;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<CheckedSignerBootstrapModel>();
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::{diagnostic_signer_bootstrap_context::CheckedSignerBootstrapModel, diagnostic_signer_build::CheckedSignerBuildModel};
/// fn weaken(m: CheckedSignerBootstrapModel) -> CheckedSignerBuildModel { m.into() }
/// ```
pub struct CheckedSignerBootstrapModel {
    receipt_sha256: String,
    context_sha256: String,
}
impl CheckedSignerBootstrapModel {
    pub(crate) fn new(receipt_sha256: String, context_sha256: String) -> Self {
        Self {
            receipt_sha256,
            context_sha256,
        }
    }
    pub fn diagnostic_hashes(&self) -> (&str, &str) {
        (&self.receipt_sha256, &self.context_sha256)
    }
}
