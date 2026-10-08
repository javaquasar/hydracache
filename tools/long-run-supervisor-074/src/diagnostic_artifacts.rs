//! Local attestation/content verification. A trusted signature is an assertion,
//! not independent compilation proof. No builder key or execution route enrolled.

use crate::diagnostic_lease::{
    validate_identity, CellIntent, DiagnosticIdentity, BINARY_PATH, CONFIG_HASHES, SOURCE_COMMIT,
    SURFACES,
};
use crate::{canonical_json, is_hash, sha256_hex};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[cfg(target_os = "linux")]
#[path = "diagnostic_artifacts_linux.rs"]
pub mod linux;

pub const SIGNATURE_DOMAIN: &[u8] = b"hydracache-diagnostic-build-074-v1";
pub const INSTALL_ROOT: &str = "/opt/hydracache-performance/0.74/diagnostic-pilot";
pub const SOURCE_TREE: &str = "a9f059d752f5497c8dc5f07fbee4db6b22a366f4";
pub const ROOT_LOCK_SHA256: &str =
    "be46eaf8e97e507bda5e3f2b671d0d622e6a639ce1d4613769a4693723d62b19";
pub const OBSERVER_LOCK_SHA256: &str =
    "e3be470f5a1bff4e917e841fc5bfbd257ff12559206bbc1481c3c4bca528eb1d";
pub const MAX_BINARY_BYTES: u64 = 134_217_728;
pub const MAX_RECEIPT_BYTES: u64 = 65_536;
pub const MAX_CONFIG_BYTES: u64 = 65_536;
pub const MAX_LOG_BYTES: u64 = 16_777_216;
pub const MAX_LOCK_BYTES: u64 = 1_048_576;
pub const BUILD_COMMAND: [&str; 13] = [
    "cargo",
    "+1.94.0",
    "build",
    "--manifest-path",
    "tools/get-owner-scheduled-controls-074/Cargo.toml",
    "--release",
    "--locked",
    "--no-default-features",
    "--bin",
    "timing-controls-074",
    "--target",
    "x86_64-unknown-linux-gnu",
    "--message-format=json",
];

#[derive(Debug, Error)]
pub enum ArtifactError {
    #[error("diagnostic build signature, identity, schema or fixed inputs refused")]
    Invalid,
    #[error("diagnostic artifact bytes, log, ELF, path or metadata refused")]
    Contents,
    #[error("diagnostic artifact I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("diagnostic build document failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactDigest {
    pub sha256: String,
    pub bytes: u64,
}

impl ArtifactDigest {
    pub fn of(bytes: &[u8]) -> Self {
        Self {
            sha256: sha256_hex(bytes),
            bytes: bytes.len() as u64,
        }
    }

    fn bounded(&self, maximum: u64) -> bool {
        is_hash(&self.sha256) && self.bytes > 0 && self.bytes <= maximum
    }

    fn matches(&self, bytes: &[u8]) -> bool {
        self.bytes == bytes.len() as u64 && self.sha256 == sha256_hex(bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildStatement {
    pub schema_version: u32,
    pub repository_id: u64,
    pub builder_id: String,
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
    pub observer_lock: ArtifactDigest,
    pub build_log: ArtifactDigest,
    pub configs: BTreeMap<String, ArtifactDigest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedBuild {
    pub statement: BuildStatement,
    pub signature_hex: String,
}

/// Supplied by a future reviewed builder policy, never inferred from the receipt.
pub struct BuildTrust {
    pub key: VerifyingKey,
    pub repository_id: u64,
    pub builder_id: String,
}

pub fn signing_message(statement: &BuildStatement) -> Result<Vec<u8>, ArtifactError> {
    let mut bytes = SIGNATURE_DOMAIN.to_vec();
    bytes.push(0);
    bytes.extend(canonical_json(statement)?);
    Ok(bytes)
}

pub fn receipt_bytes(receipt: &SignedBuild) -> Result<Vec<u8>, ArtifactError> {
    let mut bytes = canonical_json(receipt)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// This capability cannot be constructed from caller-supplied hashes alone.
pub struct VerifiedBuild {
    statement: BuildStatement,
    identity: DiagnosticIdentity,
}

pub fn verify_receipt(
    bytes: &[u8],
    identity: &DiagnosticIdentity,
    trust: &BuildTrust,
) -> Result<VerifiedBuild, ArtifactError> {
    if bytes.is_empty()
        || bytes.len() as u64 > MAX_RECEIPT_BYTES
        || validate_identity(identity).is_err()
        || sha256_hex(bytes) != identity.build_provenance_sha256
    {
        return Err(ArtifactError::Invalid);
    }
    let receipt: SignedBuild = serde_json::from_slice(bytes)?;
    if receipt_bytes(&receipt)? != bytes {
        return Err(ArtifactError::Invalid);
    }
    let raw = receipt.signature_hex.as_bytes();
    if raw.len() != 128
        || !raw
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
    {
        return Err(ArtifactError::Invalid);
    }
    let mut signature = [0; 64];
    for (index, pair) in raw.chunks_exact(2).enumerate() {
        let nibble = |b: u8| if b <= b'9' { b - b'0' } else { b - b'a' + 10 };
        signature[index] = (nibble(pair[0]) << 4) | nibble(pair[1]);
    }
    trust
        .key
        .verify_strict(
            &signing_message(&receipt.statement)?,
            &Signature::from_bytes(&signature),
        )
        .map_err(|_| ArtifactError::Invalid)?;
    let s = &receipt.statement;
    if s.schema_version != 1
        || trust.repository_id == 0
        || s.repository_id != trust.repository_id
        || trust.builder_id.is_empty()
        || trust.builder_id.len() > 64
        || s.builder_id != trust.builder_id
        || s.source_commit != SOURCE_COMMIT
        || s.source_tree != SOURCE_TREE
        || !s.source_clean_before
        || !s.source_clean_after
        || s.rustc_version != "rustc 1.94.0 (4a4ef493e 2026-03-02)"
        || s.cargo_version != "cargo 1.94.0 (85eff7c80 2026-01-15)"
        || s.target != "x86_64-unknown-linux-gnu"
        || s.profile != "release"
        || !s.features.is_empty()
        || s.allocator != "System"
        || s.counting_allocator
        || s.build_command != BUILD_COMMAND
        || !s.binary.bounded(MAX_BINARY_BYTES)
        || s.binary.sha256 != identity.binary_sha256
        || !s.root_lock.bounded(MAX_LOCK_BYTES)
        || s.root_lock.sha256 != ROOT_LOCK_SHA256
        || !s.observer_lock.bounded(MAX_LOCK_BYTES)
        || s.observer_lock.sha256 != OBSERVER_LOCK_SHA256
        || !s.build_log.bounded(MAX_LOG_BYTES)
        || s.configs.len() != SURFACES.len()
    {
        return Err(ArtifactError::Invalid);
    }
    for (surface, hash) in SURFACES.iter().zip(CONFIG_HASHES) {
        if !s
            .configs
            .get(*surface)
            .is_some_and(|d| d.bounded(MAX_CONFIG_BYTES) && d.sha256 == hash)
        {
            return Err(ArtifactError::Invalid);
        }
    }
    Ok(VerifiedBuild {
        statement: receipt.statement,
        identity: identity.clone(),
    })
}

pub struct ArtifactContents<'a> {
    pub binary: &'a [u8],
    pub root_lock: &'a [u8],
    pub observer_lock: &'a [u8],
    pub build_log: &'a [u8],
    pub configs: BTreeMap<String, &'a [u8]>,
}

/// Content inspection only. Private fields and no Deserialize/conversion to
/// VerifiedBuild prevent this unsigned result from becoming build authority.
#[derive(Debug, Serialize)]
pub struct UnsignedContentInspection {
    schema_version: &'static str,
    binary: ArtifactDigest,
    build_log: ArtifactDigest,
    configs: BTreeMap<String, ArtifactDigest>,
    attestation_verified: bool,
    source_git_identity_verified: bool,
    installed_paths_verified: bool,
    execution_authorized: bool,
    admission_allowed: bool,
}

pub fn inspect_unsigned_contents(
    contents: &ArtifactContents<'_>,
) -> Result<UnsignedContentInspection, ArtifactError> {
    for (bytes, limit) in [
        (contents.binary, MAX_BINARY_BYTES),
        (contents.root_lock, MAX_LOCK_BYTES),
        (contents.observer_lock, MAX_LOCK_BYTES),
        (contents.build_log, MAX_LOG_BYTES),
    ] {
        if bytes.is_empty() || bytes.len() as u64 > limit {
            return Err(ArtifactError::Contents);
        }
    }
    if sha256_hex(contents.root_lock) != ROOT_LOCK_SHA256
        || sha256_hex(contents.observer_lock) != OBSERVER_LOCK_SHA256
        || contents.configs.len() != SURFACES.len()
    {
        return Err(ArtifactError::Contents);
    }
    for (surface, digest) in SURFACES.iter().zip(CONFIG_HASHES) {
        let bytes = contents
            .configs
            .get(*surface)
            .ok_or(ArtifactError::Contents)?;
        if bytes.is_empty() || bytes.len() as u64 > MAX_CONFIG_BYTES || sha256_hex(bytes) != digest
        {
            return Err(ArtifactError::Contents);
        }
    }
    check_elf(contents.binary, contents.binary.len() as u64)?;
    check_cargo_log(contents.build_log)?;
    Ok(UnsignedContentInspection {
        schema_version: "diagnostic-unsigned-content-inspection-074-v1",
        binary: ArtifactDigest::of(contents.binary),
        build_log: ArtifactDigest::of(contents.build_log),
        configs: contents
            .configs
            .iter()
            .map(|(name, bytes)| (name.clone(), ArtifactDigest::of(bytes)))
            .collect(),
        attestation_verified: false,
        source_git_identity_verified: false,
        installed_paths_verified: false,
        execution_authorized: false,
        admission_allowed: false,
    })
}

/// Operator-only local file inspection. Does not execute a binary, pin installed
/// paths, authenticate the coordinator's Git identity or enroll builder trust.
pub fn inspect_unsigned_files(
    binary: &std::path::Path,
    log: &std::path::Path,
    root_lock: &std::path::Path,
    observer_lock: &std::path::Path,
    configs: &std::path::Path,
) -> Result<UnsignedContentInspection, ArtifactError> {
    let binary = read_local_file(binary, MAX_BINARY_BYTES)?;
    let log = read_local_file(log, MAX_LOG_BYTES)?;
    let root_lock = read_local_file(root_lock, MAX_LOCK_BYTES)?;
    let observer_lock = read_local_file(observer_lock, MAX_LOCK_BYTES)?;
    let configs: BTreeMap<String, Vec<u8>> = SURFACES
        .iter()
        .map(|surface| {
            Ok((
                surface.to_string(),
                read_local_file(&configs.join(format!("{surface}.json")), MAX_CONFIG_BYTES)?,
            ))
        })
        .collect::<Result<_, ArtifactError>>()?;
    inspect_unsigned_contents(&ArtifactContents {
        binary: &binary,
        build_log: &log,
        root_lock: &root_lock,
        observer_lock: &observer_lock,
        configs: configs
            .iter()
            .map(|(name, bytes)| (name.clone(), bytes.as_slice()))
            .collect(),
    })
}

fn read_local_file(path: &std::path::Path, limit: u64) -> Result<Vec<u8>, ArtifactError> {
    use std::io::Read;
    if !std::fs::symlink_metadata(path)?.is_file() {
        return Err(ArtifactError::Contents);
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
    }
    let mut file = options.open(path)?;
    let before = file.metadata()?;
    if !before.is_file() || before.len() == 0 || before.len() > limit {
        return Err(ArtifactError::Contents);
    }
    let mut bytes = vec![];
    file.by_ref().take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 != before.len() || file.metadata()?.len() != before.len() {
        return Err(ArtifactError::Contents);
    }
    Ok(bytes)
}

#[cfg(test)]
mod local_audit_tests {
    use super::*;
    #[test]
    fn local_file_reader_enforces_exact_budget_and_regular_leaf() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("document");
        std::fs::write(&path, b"abcd").unwrap();
        assert_eq!(read_local_file(&path, 4).unwrap(), b"abcd");
        assert!(read_local_file(&path, 3).is_err());
        assert!(read_local_file(directory.path(), 4).is_err());
        std::fs::write(&path, b"").unwrap();
        assert!(read_local_file(&path, 4).is_err());
        #[cfg(unix)]
        {
            let linked = directory.path().join("link");
            std::os::unix::fs::symlink(&path, &linked).unwrap();
            assert!(read_local_file(&linked, 4).is_err());
        }
    }
}

impl VerifiedBuild {
    /// Identity of the externally checked attestation, not installed-file proof.
    pub fn identity(&self) -> &DiagnosticIdentity {
        &self.identity
    }

    pub fn statement(&self) -> &BuildStatement {
        &self.statement
    }

    pub fn verify_contents(&self, contents: &ArtifactContents<'_>) -> Result<(), ArtifactError> {
        let s = &self.statement;
        if !s.binary.matches(contents.binary)
            || !s.root_lock.matches(contents.root_lock)
            || !s.observer_lock.matches(contents.observer_lock)
            || !s.build_log.matches(contents.build_log)
            || contents.configs.len() != s.configs.len()
        {
            return Err(ArtifactError::Contents);
        }
        for (surface, expected) in &s.configs {
            if !contents
                .configs
                .get(surface)
                .is_some_and(|bytes| expected.matches(bytes))
            {
                return Err(ArtifactError::Contents);
            }
        }
        check_elf(contents.binary, contents.binary.len() as u64)?;
        check_cargo_log(contents.build_log)
    }

    /// Bind a checked statement to the existing deterministic local model intent.
    /// Does not certify paths, an installed bundle, execution or cleanup.
    pub fn bind_intent(&self, intent: &CellIntent) -> Result<(), ArtifactError> {
        let index = SURFACES
            .iter()
            .position(|s| *s == intent.surface)
            .ok_or(ArtifactError::Invalid)?;
        let unit = format!(
            "hydracache-diagnostic-074-{}-{}.service",
            self.identity.lease_id,
            index + 1
        );
        if intent.lease_id != self.identity.lease_id
            || intent.boot_id != self.identity.boot_id
            || intent.binary_sha256 != self.identity.binary_sha256
            || intent.binary_path != BINARY_PATH
            || intent.source_commit != SOURCE_COMMIT
            || intent.config_sha256 != CONFIG_HASHES[index]
            || intent.config_path != format!("{INSTALL_ROOT}/{}.json", intent.surface)
            || intent.unit_name != unit
            || intent.cgroup_path != format!("/system.slice/{unit}")
            || !(1..=60).contains(&intent.maximum_runtime_seconds)
        {
            return Err(ArtifactError::Invalid);
        }
        Ok(())
    }
}

pub(crate) fn check_elf(header: &[u8], length: u64) -> Result<(), ArtifactError> {
    if header.len() < 64
        || &header[..7] != b"\x7fELF\x02\x01\x01"
        || !matches!(u16::from_le_bytes([header[16], header[17]]), 2 | 3)
        || u16::from_le_bytes([header[18], header[19]]) != 62
        || u32::from_le_bytes(header[20..24].try_into().unwrap()) != 1
        || u16::from_le_bytes([header[52], header[53]]) != 64
        || u16::from_le_bytes([header[54], header[55]]) != 56
    {
        return Err(ArtifactError::Contents);
    }
    let offset = u64::from_le_bytes(header[32..40].try_into().unwrap());
    let count = u16::from_le_bytes([header[56], header[57]]) as u64;
    if count == 0
        || offset < 64
        || offset
            .checked_add(count * 56)
            .is_none_or(|end| end > length)
    {
        return Err(ArtifactError::Contents);
    }
    Ok(())
}

// Cargo's signed raw log is retained unchanged. Projection accepts Cargo's
// unrelated fields but rejects duplicate known fields and invalid root metadata.
#[derive(Deserialize)]
struct CargoEvent {
    reason: String,
    package_id: Option<String>,
    target: Option<CargoTarget>,
    profile: Option<CargoProfile>,
    features: Option<Vec<String>>,
    executable: Option<String>,
    success: Option<bool>,
    message: Option<CargoMessage>,
}
#[derive(Deserialize)]
struct CargoTarget {
    name: String,
    kind: Vec<String>,
    crate_types: Vec<String>,
    src_path: String,
}
#[derive(Deserialize)]
struct CargoProfile {
    opt_level: String,
    debug_assertions: bool,
    test: bool,
}
#[derive(Deserialize)]
struct CargoMessage {
    level: String,
}

fn local_observer_package_id(package: &str) -> bool {
    let Some((path, fragment)) = package.rsplit_once('#') else {
        return false;
    };
    path.starts_with("path+file:///")
        && path.ends_with("/tools/get-owner-scheduled-controls-074")
        && matches!(fragment, "0.0.0" | "get-owner-scheduled-controls-074@0.0.0")
}

pub(crate) fn check_cargo_log(bytes: &[u8]) -> Result<(), ArtifactError> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_LOG_BYTES || bytes.last() != Some(&b'\n') {
        return Err(ArtifactError::Contents);
    }
    let mut artifacts = 0;
    let mut finished = false;
    for line in bytes[..bytes.len() - 1].split(|b| *b == b'\n') {
        if finished || line.is_empty() || line.len() > 1_048_576 {
            return Err(ArtifactError::Contents);
        }
        let event: CargoEvent = serde_json::from_slice(line)?;
        match event.reason.as_str() {
            "compiler-artifact" => {
                let target = event.target.ok_or(ArtifactError::Contents)?;
                if target.name == "timing-controls-074" {
                    let profile = event.profile.ok_or(ArtifactError::Contents)?;
                    if target.kind != ["bin"] || target.crate_types != ["bin"]
                        || !target.src_path.ends_with("/tools/get-owner-scheduled-controls-074/src/bin/timing_controls.rs")
                        || !event.package_id.is_some_and(|p| local_observer_package_id(&p))
                        || profile.opt_level != "3" || profile.debug_assertions || profile.test
                        || event.features != Some(vec![])
                        || !event.executable.is_some_and(|p| p.ends_with("/tools/get-owner-scheduled-controls-074/target/x86_64-unknown-linux-gnu/release/timing-controls-074"))
                    { return Err(ArtifactError::Contents); }
                    artifacts += 1;
                    if artifacts != 1 {
                        return Err(ArtifactError::Contents);
                    }
                }
            }
            "compiler-message" => {
                if event.message.is_none_or(|m| m.level == "error") {
                    return Err(ArtifactError::Contents);
                }
            }
            "build-script-executed" => {}
            "build-finished" => {
                if artifacts != 1 || event.success != Some(true) {
                    return Err(ArtifactError::Contents);
                }
                finished = true;
            }
            _ => return Err(ArtifactError::Contents),
        }
    }
    if !finished {
        return Err(ArtifactError::Contents);
    }
    Ok(())
}
