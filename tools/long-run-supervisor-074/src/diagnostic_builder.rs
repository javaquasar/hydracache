//! Separate, externally pinned builder authority. No host execution enrollment.
use crate::diagnostic_artifacts::{
    self, ArtifactContents, ArtifactDigest, ArtifactError, BuildStatement, BuildTrust, SignedBuild,
    VerifiedBuild, BUILD_COMMAND, SOURCE_TREE,
};
use crate::diagnostic_lease::{DiagnosticIdentity, SOURCE_COMMIT};
use crate::{canonical_json, is_hash, sha256_hex};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
#[path = "diagnostic_builder_linux.rs"]
pub mod linux;

pub const POLICY_SCHEMA: &str = "diagnostic-builder-policy-074-v1";
pub const OBSERVATION_SCHEMA: &str = "diagnostic-builder-observation-074-v1";
pub const REPOSITORY_ID: u64 = 1_217_101_761;
pub const BUILDER_ID: &str = "hydracache-linux-observer-074-v1";
pub const MAX_POLICY_BYTES: usize = 4096;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuilderPolicy {
    pub schema_version: String,
    pub repository_id: u64,
    pub builder_id: String,
    pub builder_key_hex: String,
    pub controller_key_hex: String,
}

pub fn policy_bytes(policy: &BuilderPolicy) -> Result<Vec<u8>, ArtifactError> {
    let mut bytes = canonical_json(policy)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Cannot be constructed from a receipt or unsigned content audit. The pin and
/// controller key must come from independent reviewed configuration.
pub struct CheckedBuilderPolicy {
    trust: BuildTrust,
}

pub fn load_policy(
    bytes: &[u8],
    expected_sha256: &str,
    controller_key: &VerifyingKey,
) -> Result<CheckedBuilderPolicy, ArtifactError> {
    if bytes.is_empty()
        || bytes.len() > MAX_POLICY_BYTES
        || !is_hash(expected_sha256)
        || sha256_hex(bytes) != expected_sha256
    {
        return Err(ArtifactError::Invalid);
    }
    let policy: BuilderPolicy = serde_json::from_slice(bytes)?;
    let key = decode_key(&policy.builder_key_hex)?;
    if policy_bytes(&policy)? != bytes
        || policy.schema_version != POLICY_SCHEMA
        || policy.repository_id != REPOSITORY_ID
        || policy.builder_id != BUILDER_ID
        || decode_key(&policy.controller_key_hex)? != *controller_key
        || key == *controller_key
        || key.is_weak()
        || controller_key.is_weak()
    {
        return Err(ArtifactError::Invalid);
    }
    Ok(CheckedBuilderPolicy {
        trust: BuildTrust {
            key,
            repository_id: policy.repository_id,
            builder_id: policy.builder_id,
        },
    })
}

pub fn decode_key(hex: &str) -> Result<VerifyingKey, ArtifactError> {
    if !is_hash(hex) {
        return Err(ArtifactError::Invalid);
    }
    let mut bytes = [0; 32];
    for (index, pair) in hex.as_bytes().chunks_exact(2).enumerate() {
        let nibble = |b: u8| if b <= b'9' { b - b'0' } else { b - b'a' + 10 };
        bytes[index] = (nibble(pair[0]) << 4) | nibble(pair[1]);
    }
    VerifyingKey::from_bytes(&bytes).map_err(|_| ArtifactError::Invalid)
}

impl CheckedBuilderPolicy {
    pub fn verify_receipt(
        &self,
        bytes: &[u8],
        identity: &DiagnosticIdentity,
    ) -> Result<VerifiedBuild, ArtifactError> {
        diagnostic_artifacts::verify_receipt(bytes, identity, &self.trust)
    }
}

/// Trusted build-job observations, not facts reconstructed from ELF bytes.
/// Only a reviewed isolated build/signing procedure may assert these values.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildObservation {
    pub schema_version: String,
    pub source_commit_before: String,
    pub source_commit_after: String,
    pub source_tree_before: String,
    pub source_tree_after: String,
    pub source_clean_before: bool,
    pub source_clean_after: bool,
    pub rustc_version: String,
    pub cargo_version: String,
    pub build_command: Vec<String>,
}

/// Not a general statement-signing API: all signed fields are fixed or derived
/// from checked observation/content. Never executes the observer or Cargo.
pub fn sign_build(
    policy: &CheckedBuilderPolicy,
    key: &SigningKey,
    observed: &BuildObservation,
    contents: &ArtifactContents<'_>,
) -> Result<SignedBuild, ArtifactError> {
    if key.verifying_key() != policy.trust.key
        || observed.schema_version != OBSERVATION_SCHEMA
        || observed.source_commit_before != SOURCE_COMMIT
        || observed.source_commit_after != SOURCE_COMMIT
        || observed.source_tree_before != SOURCE_TREE
        || observed.source_tree_after != SOURCE_TREE
        || !observed.source_clean_before
        || !observed.source_clean_after
        || observed.rustc_version != "rustc 1.94.0 (4a4ef493e 2026-03-02)"
        || observed.cargo_version != "cargo 1.94.0 (85eff7c80 2026-01-15)"
        || observed.build_command != BUILD_COMMAND
    {
        return Err(ArtifactError::Invalid);
    }
    diagnostic_artifacts::inspect_unsigned_contents(contents)?;
    let statement = BuildStatement {
        schema_version: 1,
        repository_id: policy.trust.repository_id,
        builder_id: policy.trust.builder_id.clone(),
        source_commit: observed.source_commit_before.clone(),
        source_tree: observed.source_tree_before.clone(),
        source_clean_before: true,
        source_clean_after: true,
        rustc_version: observed.rustc_version.clone(),
        cargo_version: observed.cargo_version.clone(),
        target: "x86_64-unknown-linux-gnu".into(),
        profile: "release".into(),
        features: vec![],
        allocator: "System".into(),
        counting_allocator: false,
        build_command: observed.build_command.clone(),
        binary: ArtifactDigest::of(contents.binary),
        root_lock: ArtifactDigest::of(contents.root_lock),
        observer_lock: ArtifactDigest::of(contents.observer_lock),
        build_log: ArtifactDigest::of(contents.build_log),
        configs: contents
            .configs
            .iter()
            .map(|(name, bytes)| (name.clone(), ArtifactDigest::of(bytes)))
            .collect(),
    };
    let signature = key.sign(&diagnostic_artifacts::signing_message(&statement)?);
    let signature_hex = signature
        .to_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok(SignedBuild {
        statement,
        signature_hex,
    })
}
