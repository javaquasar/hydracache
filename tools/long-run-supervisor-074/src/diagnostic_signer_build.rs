//! Separate signer-build receipt model over explicitly asserted local inputs.
//!
//! A signature and matching supplied bytes do not authenticate a build procedure,
//! key selection, original mapped executable, runtime or bootstrap custody. This
//! module has no production enrollment, signing, IO or execution integration.
use crate::diagnostic_artifacts::{ArtifactDigest, ArtifactError};
use crate::diagnostic_builder::{decode_key, REPOSITORY_ID};
use crate::diagnostic_reviewed_trust::{
    BUILDER_KEY_HEX, CONTROLLER_KEY_HEX, WORKER_ISSUER_KEY_HEX,
};
use crate::{canonical_json, is_hash, sha256_hex};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

pub const SCHEMA: &str = "diagnostic-signer-build-074-v1";
pub const SIGNATURE_DOMAIN: &[u8] = b"hydracache-diagnostic-signer-build-074-v1";
pub const ROLE: &str = "diagnostic-controller-signer";
pub const MAX_RECEIPT_BYTES: usize = 8192;
pub const MAX_BINARY_BYTES: u64 = 134_217_728;
pub const MAX_LOCK_BYTES: u64 = 1_048_576;
pub const MAX_LOG_BYTES: u64 = 16_777_216;
pub const MAX_CONTEXT_BYTES: u64 = 65_536;
pub const RUSTC_VERSION: &str = "rustc 1.94.0 (4a4ef493e 2026-03-02)";
pub const CARGO_VERSION: &str = "cargo 1.94.0 (85eff7c80 2026-01-15)";
pub const BUILD_COMMAND: [&str; 11] = [
    "cargo",
    "+1.94.0",
    "build",
    "-p",
    "hydracache-long-run-supervisor-074",
    "--bin",
    "diagnostic_controller",
    "--release",
    "--locked",
    "--target",
    "x86_64-unknown-linux-gnu",
];

/// Signed assertions, not independently observed compilation/runtime facts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignerBuildStatement {
    pub schema_version: String,
    pub repository_id: u64,
    pub role: String,
    pub source_commit: String,
    pub source_tree: String,
    pub source_clean_before: bool,
    pub source_clean_after: bool,
    pub rustc_version: String,
    pub cargo_version: String,
    pub target: String,
    pub profile: String,
    pub features: Vec<String>,
    pub allocator: String,
    pub counting_allocator: bool,
    pub build_command: Vec<String>,
    pub binary: ArtifactDigest,
    pub root_lock: ArtifactDigest,
    pub build_log: ArtifactDigest,
    pub runtime_manifest: ArtifactDigest,
    pub bootstrap_context: ArtifactDigest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedSignerBuild {
    pub statement: SignerBuildStatement,
    pub signature_hex: String,
}

/// Supplied slices only. Context bytes are opaque; no ELF/runtime inspection.
pub struct SignerBuildContents<'a> {
    pub binary: &'a [u8],
    pub root_lock: &'a [u8],
    pub build_log: &'a [u8],
    pub runtime_manifest: &'a [u8],
    pub bootstrap_context: &'a [u8],
}

/// Explicit local assertions, NOT an independently authenticated trust policy.
/// Never promote this caller-selected model to signing or bootstrap authority.
pub struct AssertedSignerBuildSelection {
    key: VerifyingKey,
    expected: SignerBuildStatement,
}

/// Byte/signature consistency result only, not an observer build or capability.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_signer_build::CheckedSignerBuildModel;
/// fn duplicate(model: CheckedSignerBuildModel) { let _ = model.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_signer_build::CheckedSignerBuildModel;
/// fn deserialize<T: serde::de::DeserializeOwned>() {}
/// deserialize::<CheckedSignerBuildModel>();
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::{diagnostic_signer_build::CheckedSignerBuildModel,
///     diagnostic_artifacts::VerifiedBuild};
/// fn authorize(model: CheckedSignerBuildModel) -> VerifiedBuild { model.into() }
/// ```
pub struct CheckedSignerBuildModel {
    receipt_sha256: String,
}

impl CheckedSignerBuildModel {
    /// Diagnostic inventory only; not a production enrollment pin.
    pub fn receipt_sha256(&self) -> &str {
        &self.receipt_sha256
    }
}

impl AssertedSignerBuildSelection {
    pub fn new(key: VerifyingKey, expected: SignerBuildStatement) -> Result<Self, ArtifactError> {
        if key.is_weak() {
            return Err(ArtifactError::Invalid);
        }
        for old_role in [CONTROLLER_KEY_HEX, BUILDER_KEY_HEX, WORKER_ISSUER_KEY_HEX] {
            if key == decode_key(old_role)? {
                return Err(ArtifactError::Invalid);
            }
        }
        validate_statement(&expected)?;
        Ok(Self { key, expected })
    }

    /// Match an asserted selection, domain signature and all supplied content.
    /// No issuer acquisition, compilation, live custody, replay or start proof.
    pub fn verify_supplied(
        &self,
        bytes: &[u8],
        contents: &SignerBuildContents<'_>,
    ) -> Result<CheckedSignerBuildModel, ArtifactError> {
        self.check_receipt(bytes)?;
        for (digest, supplied) in [
            (&self.expected.binary, contents.binary),
            (&self.expected.root_lock, contents.root_lock),
            (&self.expected.build_log, contents.build_log),
            (&self.expected.runtime_manifest, contents.runtime_manifest),
            (&self.expected.bootstrap_context, contents.bootstrap_context),
        ] {
            // Length first: over-budget slices never reach hashing.
            if supplied.len() as u64 != digest.bytes || sha256_hex(supplied) != digest.sha256 {
                return Err(ArtifactError::Contents);
            }
        }
        Ok(CheckedSignerBuildModel {
            receipt_sha256: sha256_hex(bytes),
        })
    }

    // Internal streaming-reader seam only. Still an explicitly asserted model,
    // never a public digest-to-bootstrap or execution authorization factory.
    #[cfg(target_os = "linux")]
    pub(crate) fn verify_original_digests(
        &self,
        bytes: &[u8],
        digests: &[ArtifactDigest; 5],
    ) -> Result<CheckedSignerBuildModel, ArtifactError> {
        self.check_receipt(bytes)?;
        for (actual, expected) in digests.iter().zip([
            &self.expected.binary,
            &self.expected.root_lock,
            &self.expected.build_log,
            &self.expected.runtime_manifest,
            &self.expected.bootstrap_context,
        ]) {
            if actual != expected {
                return Err(ArtifactError::Contents);
            }
        }
        Ok(CheckedSignerBuildModel {
            receipt_sha256: sha256_hex(bytes),
        })
    }

    fn check_receipt(&self, bytes: &[u8]) -> Result<(), ArtifactError> {
        if bytes.is_empty() || bytes.len() > MAX_RECEIPT_BYTES {
            return Err(ArtifactError::Invalid);
        }
        let receipt: SignedSignerBuild = serde_json::from_slice(bytes)?;
        let mut canonical = canonical_json(&receipt)?;
        canonical.push(b'\n');
        if canonical != bytes || receipt.statement != self.expected {
            return Err(ArtifactError::Invalid);
        }
        let hex = &receipt.signature_hex;
        if hex.len() != 128
            || !hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(ArtifactError::Invalid);
        }
        let mut raw_signature = [0; 64];
        for (index, pair) in hex.as_bytes().chunks_exact(2).enumerate() {
            let nibble = |b: u8| if b <= b'9' { b - b'0' } else { b - b'a' + 10 };
            raw_signature[index] = (nibble(pair[0]) << 4) | nibble(pair[1]);
        }
        let mut message = SIGNATURE_DOMAIN.to_vec();
        message.push(0);
        message.extend(canonical_json(&receipt.statement)?);
        self.key
            .verify_strict(&message, &Signature::from_bytes(&raw_signature))
            .map_err(|_| ArtifactError::Invalid)?;
        Ok(())
    }
}

fn validate_statement(s: &SignerBuildStatement) -> Result<(), ArtifactError> {
    let source_id = |value: &str| {
        value.len() == 40
            && value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    };
    if s.schema_version != SCHEMA
        || s.repository_id != REPOSITORY_ID
        || s.role != ROLE
        || !source_id(&s.source_commit)
        || !source_id(&s.source_tree)
        || !s.source_clean_before
        || !s.source_clean_after
        || s.rustc_version != RUSTC_VERSION
        || s.cargo_version != CARGO_VERSION
        || s.target != "x86_64-unknown-linux-gnu"
        || s.profile != "release"
        || !s.features.is_empty()
        || s.allocator != "System"
        || s.counting_allocator
        || s.build_command != BUILD_COMMAND
    {
        return Err(ArtifactError::Invalid);
    }
    for (digest, limit) in [
        (&s.binary, MAX_BINARY_BYTES),
        (&s.root_lock, MAX_LOCK_BYTES),
        (&s.build_log, MAX_LOG_BYTES),
        (&s.runtime_manifest, MAX_CONTEXT_BYTES),
        (&s.bootstrap_context, MAX_CONTEXT_BYTES),
    ] {
        if !is_hash(&digest.sha256) || digest.bytes == 0 || digest.bytes > limit {
            return Err(ArtifactError::Invalid);
        }
    }
    Ok(())
}
