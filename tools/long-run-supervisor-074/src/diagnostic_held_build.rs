//! Purpose-bound build assertions, not live enrollment or execution authority.
//! No files, clocks, processes or manager channels are accessed here.
use super::{decode_key, REPOSITORY_ID};
use crate::diagnostic_artifacts::{
    check_elf, ArtifactDigest, ArtifactError, MAX_BINARY_BYTES, MAX_LOCK_BYTES, MAX_LOG_BYTES,
    MAX_RECEIPT_BYTES,
};
use crate::diagnostic_two_phase_launch::LaunchArtifacts;
use crate::{canonical_json, is_hash, sha256_hex};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
#[path = "diagnostic_held_role_files.rs"]
pub mod role_files;

pub const POLICY_SCHEMA: &str = "diagnostic-held-build-policy-074-v1";
pub const BODY_SCHEMA: &str = "diagnostic-held-build-074-v1";
pub const RECEIPT_SCHEMA: &str = "diagnostic-signed-held-build-074-v1";
pub const SIGNATURE_DOMAIN: &[u8] = b"hydracache-diagnostic-held-build-074-v1";
pub const PURPOSE: &str = "original-host-and-held-launcher";
pub const BUILDER_ID: &str = "hydracache-held-runtime-074-v1";
pub const HOST: &str = "hydracache-long-run-supervisor-074";
pub const LAUNCHER: &str = "diagnostic_held_launcher";
pub const BUILD_COMMAND: [&str; 15] = [
    "cargo",
    "+1.94.0",
    "build",
    "--manifest-path",
    "tools/long-run-supervisor-074/Cargo.toml",
    "--release",
    "--locked",
    "--no-default-features",
    "--bin",
    HOST,
    "--bin",
    LAUNCHER,
    "--target",
    "x86_64-unknown-linux-gnu",
    "--message-format=json",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub commit: String,
    pub tree: String,
    pub root_lock: ArtifactDigest,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub schema_version: String,
    pub purpose: String,
    pub repository_id: u64,
    pub builder_id: String,
    pub builder_key_hex: String,
    pub controller_key_hex: String,
    pub authority_epoch: u64,
    pub source: Source,
}
/// Independent reviewed configuration, not caller-derived live authority.
pub struct ExpectedTrust<'a> {
    pub controller_key: &'a VerifyingKey,
    pub authority_epoch: u64,
    pub worker_policy: &'a [u8],
    pub worker_policy_sha256: &'a str,
}
/// Opaque static policy check. It cannot send, start, renew or unlock.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_builder::held_build::CheckedPolicy;
/// fn cannot_start(policy: CheckedPolicy) { policy.start(); }
/// ```
pub struct CheckedPolicy {
    policy: Policy,
    digest: String,
    key: VerifyingKey,
}
pub fn document_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, ArtifactError> {
    let mut raw = canonical_json(value)?;
    raw.push(b'\n');
    Ok(raw)
}
fn bounded(d: &ArtifactDigest, max: u64) -> bool {
    is_hash(&d.sha256) && d.bytes > 0 && d.bytes <= max
}
fn source_id(s: &str) -> bool {
    s.len() == 40
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && s.bytes().any(|b| b != b'0')
}
pub fn load_policy(
    raw: &[u8],
    pin: &str,
    expected: &ExpectedTrust<'_>,
) -> Result<CheckedPolicy, ArtifactError> {
    if raw.is_empty()
        || raw.len() > super::MAX_POLICY_BYTES
        || !is_hash(pin)
        || sha256_hex(raw) != pin
    {
        return Err(ArtifactError::Invalid);
    }
    // Reload the independent old policy against the SAME controller. A checked
    // old builder key alone would not prove that controller relationship.
    let worker = super::load_policy(
        expected.worker_policy,
        expected.worker_policy_sha256,
        expected.controller_key,
    )?;
    let policy: Policy = serde_json::from_slice(raw)?;
    let key = decode_key(&policy.builder_key_hex)?;
    if document_bytes(&policy)? != raw
        || policy.schema_version != POLICY_SCHEMA
        || policy.purpose != PURPOSE
        || policy.repository_id != REPOSITORY_ID
        || policy.builder_id != BUILDER_ID
        || decode_key(&policy.controller_key_hex)? != *expected.controller_key
        || key.is_weak()
        || key == *expected.controller_key
        || key == worker.trust.key
        || expected.authority_epoch == 0
        || policy.authority_epoch != expected.authority_epoch
        || !source_id(&policy.source.commit)
        || !source_id(&policy.source.tree)
        || !bounded(&policy.source.root_lock, MAX_LOCK_BYTES)
    {
        return Err(ArtifactError::Invalid);
    }
    Ok(CheckedPolicy {
        policy,
        digest: pin.into(),
        key,
    })
}

/// Trusted isolated build-job observations; not inferred from ELF or log bytes.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub source_commit_before: String,
    pub source_commit_after: String,
    pub source_tree_before: String,
    pub source_tree_after: String,
    pub source_clean_before: bool,
    pub source_clean_after: bool,
    pub isolated_build: bool,
    pub rustc_version: String,
    pub cargo_version: String,
    pub build_command: Vec<String>,
}
impl CheckedPolicy {
    fn observation(&self, o: &Observation) -> Result<(), ArtifactError> {
        if o.source_commit_before != self.policy.source.commit
            || o.source_commit_after != self.policy.source.commit
            || o.source_tree_before != self.policy.source.tree
            || o.source_tree_after != self.policy.source.tree
            || !o.source_clean_before
            || !o.source_clean_after
            || !o.isolated_build
            || o.rustc_version != "rustc 1.94.0 (4a4ef493e 2026-03-02)"
            || o.cargo_version != "cargo 1.94.0 (85eff7c80 2026-01-15)"
            || o.build_command != BUILD_COMMAND
        {
            return Err(ArtifactError::Invalid);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleMap {
    pub launcher: String,
    pub watchdog: String,
    pub collector: String,
}
fn roles() -> RoleMap {
    RoleMap {
        launcher: "held_launcher".into(),
        watchdog: "original_host".into(),
        collector: "original_host".into(),
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Images {
    pub original_host: ArtifactDigest,
    pub held_launcher: ArtifactDigest,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Statement {
    pub schema_version: String,
    pub purpose: String,
    pub policy_sha256: String,
    pub authority_epoch: u64,
    pub observation: Observation,
    pub roles: RoleMap,
    pub images: Images,
    pub root_lock: ArtifactDigest,
    pub build_log: ArtifactDigest,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedReceipt {
    pub schema_version: String,
    pub statement: Statement,
    pub signature_hex: String,
}
pub struct Contents<'a> {
    pub original_host: &'a [u8],
    pub held_launcher: &'a [u8],
    pub root_lock: &'a [u8],
    pub build_log: &'a [u8],
}
fn message(s: &Statement) -> Result<Vec<u8>, ArtifactError> {
    let mut raw = SIGNATURE_DOMAIN.to_vec();
    raw.push(0);
    raw.extend(document_bytes(s)?);
    Ok(raw)
}
fn hex(raw: &[u8]) -> String {
    raw.iter().map(|b| format!("{b:02x}")).collect()
}
fn inspect(c: &Contents<'_>) -> Result<(), ArtifactError> {
    for (raw, max) in [
        (c.original_host, MAX_BINARY_BYTES),
        (c.held_launcher, MAX_BINARY_BYTES),
        (c.root_lock, MAX_LOCK_BYTES),
        (c.build_log, MAX_LOG_BYTES),
    ] {
        if raw.is_empty() || raw.len() as u64 > max {
            return Err(ArtifactError::Contents);
        }
    }
    check_elf(c.original_host, c.original_host.len() as u64)?;
    check_elf(c.held_launcher, c.held_launcher.len() as u64)?;
    if sha256_hex(c.original_host) == sha256_hex(c.held_launcher) {
        return Err(ArtifactError::Contents);
    }
    cargo_log(c.build_log)
}
/// Derives the body after validation. No general sign-arbitrary-statement API.
pub fn sign_build(
    policy: &CheckedPolicy,
    key: &SigningKey,
    observed: &Observation,
    c: &Contents<'_>,
) -> Result<SignedReceipt, ArtifactError> {
    if key.verifying_key() != policy.key {
        return Err(ArtifactError::Invalid);
    }
    policy.observation(observed)?;
    inspect(c)?;
    if ArtifactDigest::of(c.root_lock) != policy.policy.source.root_lock {
        return Err(ArtifactError::Contents);
    }
    let statement = Statement {
        schema_version: BODY_SCHEMA.into(),
        purpose: PURPOSE.into(),
        policy_sha256: policy.digest.clone(),
        authority_epoch: policy.policy.authority_epoch,
        observation: observed.clone(),
        roles: roles(),
        images: Images {
            original_host: ArtifactDigest::of(c.original_host),
            held_launcher: ArtifactDigest::of(c.held_launcher),
        },
        root_lock: ArtifactDigest::of(c.root_lock),
        build_log: ArtifactDigest::of(c.build_log),
    };
    Ok(SignedReceipt {
        schema_version: RECEIPT_SCHEMA.into(),
        signature_hex: hex(&key.sign(&message(&statement)?).to_bytes()),
        statement,
    })
}
/// Checked static build assertion, deliberately not a launch capability.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_builder::held_build::VerifiedBuild;
/// fn cannot_unlock(build: VerifiedBuild) { build.unlock(); }
/// ```
pub struct VerifiedBuild {
    statement: Statement,
}
pub fn verify_receipt(raw: &[u8], policy: &CheckedPolicy) -> Result<VerifiedBuild, ArtifactError> {
    if raw.is_empty() || raw.len() as u64 > MAX_RECEIPT_BYTES {
        return Err(ArtifactError::Invalid);
    }
    let receipt: SignedReceipt = serde_json::from_slice(raw)?;
    let sig = receipt.signature_hex.as_bytes();
    if document_bytes(&receipt)? != raw
        || receipt.schema_version != RECEIPT_SCHEMA
        || sig.len() != 128
        || !sig
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
    {
        return Err(ArtifactError::Invalid);
    }
    let mut signature = [0; 64];
    for (i, pair) in sig.chunks_exact(2).enumerate() {
        let nibble = |b: u8| if b <= b'9' { b - b'0' } else { b - b'a' + 10 };
        signature[i] = nibble(pair[0]) * 16 + nibble(pair[1]);
    }
    policy
        .key
        .verify_strict(
            &message(&receipt.statement)?,
            &Signature::from_bytes(&signature),
        )
        .map_err(|_| ArtifactError::Invalid)?;
    let s = &receipt.statement;
    policy.observation(&s.observation)?;
    if s.schema_version != BODY_SCHEMA
        || s.purpose != PURPOSE
        || s.policy_sha256 != policy.digest
        || s.authority_epoch != policy.policy.authority_epoch
        || s.roles != roles()
        || !bounded(&s.images.original_host, MAX_BINARY_BYTES)
        || !bounded(&s.images.held_launcher, MAX_BINARY_BYTES)
        || s.images.original_host.sha256 == s.images.held_launcher.sha256
        || s.root_lock != policy.policy.source.root_lock
        || !bounded(&s.build_log, MAX_LOG_BYTES)
    {
        return Err(ArtifactError::Invalid);
    }
    Ok(VerifiedBuild {
        statement: receipt.statement,
    })
}
impl VerifiedBuild {
    pub fn verify_contents(&self, c: &Contents<'_>) -> Result<(), ArtifactError> {
        inspect(c)?;
        let s = &self.statement;
        if ArtifactDigest::of(c.original_host) != s.images.original_host
            || ArtifactDigest::of(c.held_launcher) != s.images.held_launcher
            || ArtifactDigest::of(c.root_lock) != s.root_lock
            || ArtifactDigest::of(c.build_log) != s.build_log
        {
            return Err(ArtifactError::Contents);
        }
        Ok(())
    }
    /// Equality of expected digests only; no original-owner or live-file proof.
    pub fn matches_launch_artifacts(
        &self,
        expected: &LaunchArtifacts,
    ) -> Result<(), ArtifactError> {
        if expected.launcher_sha256 != self.statement.images.held_launcher.sha256
            || expected.watchdog_sha256 != self.statement.images.original_host.sha256
            || expected.collector_sha256 != self.statement.images.original_host.sha256
        {
            return Err(ArtifactError::Invalid);
        }
        Ok(())
    }
}

// Cargo projections accept its unrelated metadata, but retain the entire log
// digest. Known-field duplicates remain serde errors. No lossy Value lookup.
#[derive(Deserialize)]
struct Event {
    reason: String,
    package_id: Option<String>,
    target: Option<Target>,
    profile: Option<Profile>,
    features: Option<Vec<String>>,
    executable: Option<String>,
    success: Option<bool>,
    message: Option<CompilerMessage>,
}
#[derive(Deserialize)]
struct Target {
    name: String,
    kind: Vec<String>,
    crate_types: Vec<String>,
    src_path: String,
}
#[derive(Deserialize)]
struct Profile {
    opt_level: String,
    debug_assertions: bool,
    test: bool,
}
#[derive(Deserialize)]
struct CompilerMessage {
    level: String,
}
fn cargo_log(raw: &[u8]) -> Result<(), ArtifactError> {
    if !raw.ends_with(b"\n") {
        return Err(ArtifactError::Contents);
    }
    let mut seen = [false; 2];
    let mut finished = false;
    for line in raw[..raw.len() - 1].split(|b| *b == b'\n') {
        if finished || line.is_empty() || line.len() > 1_048_576 {
            return Err(ArtifactError::Contents);
        }
        let e: Event = serde_json::from_slice(line)?;
        match e.reason.as_str() {
            "compiler-artifact" => {
                let t = e.target.ok_or(ArtifactError::Contents)?;
                if [HOST, LAUNCHER].contains(&t.name.as_str()) || t.kind.iter().any(|k| k == "bin")
                {
                    let i = [HOST, LAUNCHER]
                        .iter()
                        .position(|name| *name == t.name)
                        .ok_or(ArtifactError::Contents)?;
                    let p = e.profile.ok_or(ArtifactError::Contents)?;
                    let package = e.package_id.ok_or(ArtifactError::Contents)?;
                    let (path, fragment) =
                        package.split_once('#').ok_or(ArtifactError::Contents)?;
                    let src = if i == 0 {
                        "/tools/long-run-supervisor-074/src/main.rs"
                    } else {
                        "/tools/long-run-supervisor-074/src/bin/diagnostic_held_launcher.rs"
                    };
                    let output = format!("/x86_64-unknown-linux-gnu/release/{}", t.name);
                    if seen[i]
                        || t.kind != ["bin"]
                        || t.crate_types != ["bin"]
                        || !t.src_path.starts_with('/')
                        || !t.src_path.ends_with(src)
                        || !path.starts_with("path+file:///")
                        || !path.ends_with("/tools/long-run-supervisor-074")
                        || !["0.0.0", "hydracache-long-run-supervisor-074@0.0.0"]
                            .contains(&fragment)
                        || p.opt_level != "3"
                        || p.debug_assertions
                        || p.test
                        || e.features.as_deref() != Some(&[])
                        || !e
                            .executable
                            .as_deref()
                            .is_some_and(|exe| exe.starts_with('/') && exe.ends_with(&output))
                    {
                        return Err(ArtifactError::Contents);
                    }
                    seen[i] = true;
                }
            }
            "compiler-message" => {
                if e.message.ok_or(ArtifactError::Contents)?.level == "error" {
                    return Err(ArtifactError::Contents);
                }
            }
            "build-script-executed" => {}
            "build-finished" if e.success == Some(true) && seen == [true; 2] => finished = true,
            _ => return Err(ArtifactError::Contents),
        }
    }
    if !finished {
        return Err(ArtifactError::Contents);
    }
    Ok(())
}
