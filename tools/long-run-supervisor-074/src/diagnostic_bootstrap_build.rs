//! Static signed bootstrap assertions. No enrollment, kernel or execution rights.
use super::{
    decode_key,
    held_build::{document_bytes, Observation, Source},
    MAX_POLICY_BYTES, REPOSITORY_ID,
};
use crate::diagnostic_artifacts::{
    check_elf, ArtifactDigest, ArtifactError, MAX_BINARY_BYTES, MAX_LOCK_BYTES, MAX_LOG_BYTES,
    MAX_RECEIPT_BYTES,
};
use crate::{is_hash, sha256_hex};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

pub const POLICY_SCHEMA: &str = "diagnostic-bootstrap-build-policy-074-v1";
pub const SIGNED_POLICY_SCHEMA: &str = "diagnostic-signed-bootstrap-policy-074-v1";
pub const BODY_SCHEMA: &str = "diagnostic-bootstrap-build-074-v1";
pub const RECEIPT_SCHEMA: &str = "diagnostic-signed-bootstrap-build-074-v1";
pub const POLICY_DOMAIN: &[u8] = b"hydracache-diagnostic-bootstrap-policy-074-v1";
pub const RECEIPT_DOMAIN: &[u8] = b"hydracache-diagnostic-bootstrap-build-074-v1";
pub const PURPOSE: &str = "original-sealed-launcher-bootstrap";
pub const BUILDER_ID: &str = "hydracache-held-bootstrap-074-v1";
pub const BINARY: &str = "diagnostic_held_bootstrap";
pub const BUILD_COMMAND: [&str; 13] = [
    "cargo",
    "+1.94.0",
    "build",
    "--manifest-path",
    "tools/long-run-supervisor-074/Cargo.toml",
    "--release",
    "--locked",
    "--no-default-features",
    "--bin",
    BINARY,
    "--target",
    "x86_64-unknown-linux-gnu",
    "--message-format=json",
];

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
    pub bootstrap_sha256: String,
    pub held_policy_sha256: String,
    pub held_receipt_sha256: String,
    pub original_host_sha256: String,
    pub held_launcher_sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedPolicy {
    pub schema_version: String,
    pub policy: Policy,
    pub signature_hex: String,
}
/// Static expectations only. Live join obtains these from retained protected input.
pub struct ExpectedTrust<'a> {
    pub builder: &'a VerifyingKey,
    pub controller: &'a VerifyingKey,
    pub epoch: u64,
    pub held_policy_sha256: &'a str,
    pub bootstrap_sha256: &'a str,
}
pub struct CheckedPolicy {
    policy: Policy,
    digest: String,
    key: VerifyingKey,
}
fn hash(value: &str) -> bool {
    is_hash(value) && value.bytes().any(|b| b != b'0')
}
fn bounded(d: &ArtifactDigest, limit: u64) -> bool {
    hash(&d.sha256) && d.bytes > 0 && d.bytes <= limit
}
fn source_id(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && value.bytes().any(|b| b != b'0')
}
fn message(domain: &[u8], value: &impl Serialize) -> Result<Vec<u8>, ArtifactError> {
    let mut result = domain.to_vec();
    result.push(0);
    result.extend(document_bytes(value)?);
    Ok(result)
}
fn signature(
    key: &SigningKey,
    domain: &[u8],
    value: &impl Serialize,
) -> Result<String, ArtifactError> {
    Ok(crate::hex(&key.sign(&message(domain, value)?).to_bytes()))
}
fn verify(
    key: &VerifyingKey,
    domain: &[u8],
    value: &impl Serialize,
    hex: &str,
) -> Result<(), ArtifactError> {
    if hex.len() != 128
        || !hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(ArtifactError::Invalid);
    }
    let mut raw = [0; 64];
    for (i, pair) in hex.as_bytes().chunks_exact(2).enumerate() {
        let nibble = |b: u8| if b <= b'9' { b - b'0' } else { b - b'a' + 10 };
        raw[i] = nibble(pair[0]) * 16 + nibble(pair[1]);
    }
    key.verify_strict(&message(domain, value)?, &Signature::from_bytes(&raw))
        .map_err(|_| ArtifactError::Invalid)
}
fn check_policy(p: &Policy, expected: &ExpectedTrust<'_>) -> Result<(), ArtifactError> {
    if p.schema_version != POLICY_SCHEMA
        || p.purpose != PURPOSE
        || p.repository_id != REPOSITORY_ID
        || p.builder_id != BUILDER_ID
        || decode_key(&p.builder_key_hex)? != *expected.builder
        || decode_key(&p.controller_key_hex)? != *expected.controller
        || expected.builder.is_weak()
        || expected.controller.is_weak()
        || expected.builder == expected.controller
        || expected.epoch == 0
        || p.authority_epoch != expected.epoch
        || p.held_policy_sha256 != expected.held_policy_sha256
        || p.bootstrap_sha256 != expected.bootstrap_sha256
        || !source_id(&p.source.commit)
        || !source_id(&p.source.tree)
        || !bounded(&p.source.root_lock, MAX_LOCK_BYTES)
        || [
            &p.held_policy_sha256,
            &p.held_receipt_sha256,
            &p.bootstrap_sha256,
            &p.original_host_sha256,
            &p.held_launcher_sha256,
        ]
        .iter()
        .any(|v| !hash(v))
        || p.original_host_sha256 == p.held_launcher_sha256
        || [
            p.original_host_sha256.as_str(),
            p.held_launcher_sha256.as_str(),
        ]
        .contains(&p.bootstrap_sha256.as_str())
    {
        return Err(ArtifactError::Invalid);
    }
    Ok(())
}
/// Purpose-bounded policy assertion; no key generation or I/O.
pub fn sign_policy(
    policy: &Policy,
    key: &SigningKey,
    expected: &ExpectedTrust<'_>,
) -> Result<SignedPolicy, ArtifactError> {
    check_policy(policy, expected)?;
    if key.verifying_key() != *expected.builder {
        return Err(ArtifactError::Invalid);
    }
    let doc = SignedPolicy {
        schema_version: SIGNED_POLICY_SCHEMA.into(),
        policy: policy.clone(),
        signature_hex: signature(key, POLICY_DOMAIN, policy)?,
    };
    if document_bytes(&doc)?.len() > MAX_POLICY_BYTES {
        return Err(ArtifactError::Invalid);
    }
    Ok(doc)
}
pub fn load_policy(
    raw: &[u8],
    pin: &str,
    expected: &ExpectedTrust<'_>,
) -> Result<CheckedPolicy, ArtifactError> {
    if raw.is_empty() || raw.len() > MAX_POLICY_BYTES || !hash(pin) || sha256_hex(raw) != pin {
        return Err(ArtifactError::Invalid);
    }
    let doc: SignedPolicy = serde_json::from_slice(raw)?;
    if document_bytes(&doc)? != raw || doc.schema_version != SIGNED_POLICY_SCHEMA {
        return Err(ArtifactError::Invalid);
    }
    check_policy(&doc.policy, expected)?;
    verify(
        expected.builder,
        POLICY_DOMAIN,
        &doc.policy,
        &doc.signature_hex,
    )?;
    Ok(CheckedPolicy {
        policy: doc.policy,
        digest: pin.into(),
        key: *expected.builder,
    })
}
fn observation(policy: &Policy, o: &Observation) -> Result<(), ArtifactError> {
    if o.source_commit_before != policy.source.commit
        || o.source_commit_after != policy.source.commit
        || o.source_tree_before != policy.source.tree
        || o.source_tree_after != policy.source.tree
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
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Statement {
    pub schema_version: String,
    pub purpose: String,
    pub policy_sha256: String,
    pub authority_epoch: u64,
    pub observation: Observation,
    pub bootstrap: ArtifactDigest,
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
    pub bootstrap: &'a [u8],
    pub root_lock: &'a [u8],
    pub build_log: &'a [u8],
}
fn contents(c: &Contents<'_>) -> Result<(), ArtifactError> {
    for (raw, max) in [
        (c.bootstrap, MAX_BINARY_BYTES),
        (c.root_lock, MAX_LOCK_BYTES),
        (c.build_log, MAX_LOG_BYTES),
    ] {
        if raw.is_empty() || raw.len() as u64 > max {
            return Err(ArtifactError::Contents);
        }
    }
    check_elf(c.bootstrap, c.bootstrap.len() as u64)?;
    cargo_log(c.build_log)
}
pub fn sign_build(
    p: &CheckedPolicy,
    key: &SigningKey,
    o: &Observation,
    c: &Contents<'_>,
) -> Result<SignedReceipt, ArtifactError> {
    if key.verifying_key() != p.key {
        return Err(ArtifactError::Invalid);
    }
    observation(&p.policy, o)?;
    contents(c)?;
    if ArtifactDigest::of(c.root_lock) != p.policy.source.root_lock
        || sha256_hex(c.bootstrap) != p.policy.bootstrap_sha256
    {
        return Err(ArtifactError::Contents);
    }
    let statement = Statement {
        schema_version: BODY_SCHEMA.into(),
        purpose: PURPOSE.into(),
        policy_sha256: p.digest.clone(),
        authority_epoch: p.policy.authority_epoch,
        observation: o.clone(),
        bootstrap: ArtifactDigest::of(c.bootstrap),
        root_lock: ArtifactDigest::of(c.root_lock),
        build_log: ArtifactDigest::of(c.build_log),
    };
    Ok(SignedReceipt {
        schema_version: RECEIPT_SCHEMA.into(),
        signature_hex: signature(key, RECEIPT_DOMAIN, &statement)?,
        statement,
    })
}
pub struct VerifiedBuild {
    statement: Statement,
    #[cfg(target_os = "linux")]
    policy: Policy,
}
pub fn verify_receipt(raw: &[u8], p: &CheckedPolicy) -> Result<VerifiedBuild, ArtifactError> {
    if raw.is_empty() || raw.len() as u64 > MAX_RECEIPT_BYTES {
        return Err(ArtifactError::Invalid);
    }
    let doc: SignedReceipt = serde_json::from_slice(raw)?;
    if document_bytes(&doc)? != raw || doc.schema_version != RECEIPT_SCHEMA {
        return Err(ArtifactError::Invalid);
    }
    verify(&p.key, RECEIPT_DOMAIN, &doc.statement, &doc.signature_hex)?;
    let s = &doc.statement;
    observation(&p.policy, &s.observation)?;
    if s.schema_version != BODY_SCHEMA
        || s.purpose != PURPOSE
        || s.policy_sha256 != p.digest
        || s.authority_epoch != p.policy.authority_epoch
        || !bounded(&s.bootstrap, MAX_BINARY_BYTES)
        || s.bootstrap.sha256 != p.policy.bootstrap_sha256
        || s.root_lock != p.policy.source.root_lock
        || !bounded(&s.build_log, MAX_LOG_BYTES)
    {
        return Err(ArtifactError::Invalid);
    }
    Ok(VerifiedBuild {
        statement: doc.statement,
        #[cfg(target_os = "linux")]
        policy: p.policy.clone(),
    })
}
impl VerifiedBuild {
    pub fn verify_contents(&self, c: &Contents<'_>) -> Result<(), ArtifactError> {
        contents(c)?;
        if ArtifactDigest::of(c.bootstrap) != self.statement.bootstrap
            || ArtifactDigest::of(c.root_lock) != self.statement.root_lock
            || ArtifactDigest::of(c.build_log) != self.statement.build_log
        {
            return Err(ArtifactError::Contents);
        }
        Ok(())
    }
    #[cfg(target_os = "linux")]
    pub(crate) fn matches_original(&self, digests: [&str; 4]) -> Result<(), ArtifactError> {
        if digests
            != [
                &self.policy.held_policy_sha256,
                &self.policy.held_receipt_sha256,
                &self.policy.original_host_sha256,
                &self.policy.held_launcher_sha256,
            ]
        {
            return Err(ArtifactError::Invalid);
        }
        Ok(())
    }
}

// Cargo metadata may contain unrelated fields; known-field duplicates still fail.
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
    if raw.is_empty() || raw.len() as u64 > MAX_LOG_BYTES || !raw.ends_with(b"\n") {
        return Err(ArtifactError::Contents);
    }
    let mut seen = false;
    let mut finished = false;
    for line in raw[..raw.len() - 1].split(|b| *b == b'\n') {
        if finished || line.is_empty() || line.len() > 1_048_576 {
            return Err(ArtifactError::Contents);
        }
        let e: Event = serde_json::from_slice(line)?;
        match e.reason.as_str() {
            "compiler-artifact" => {
                let t = e.target.ok_or(ArtifactError::Contents)?;
                if t.name == BINARY || t.kind.iter().any(|v| v == "bin") {
                    let p = e.profile.ok_or(ArtifactError::Contents)?;
                    let package = e.package_id.ok_or(ArtifactError::Contents)?;
                    let (path, fragment) =
                        package.split_once('#').ok_or(ArtifactError::Contents)?;
                    if seen
                        || t.name != BINARY
                        || t.kind != ["bin"]
                        || t.crate_types != ["bin"]
                        || !t.src_path.starts_with('/')
                        || !t.src_path.ends_with(
                            "/tools/long-run-supervisor-074/src/bin/diagnostic_held_bootstrap.rs",
                        )
                        || !path.starts_with("path+file:///")
                        || !path.ends_with("/tools/long-run-supervisor-074")
                        || !["0.0.0", "hydracache-long-run-supervisor-074@0.0.0"]
                            .contains(&fragment)
                        || p.opt_level != "3"
                        || p.debug_assertions
                        || p.test
                        || e.features.as_deref() != Some(&[])
                        || !e.executable.as_deref().is_some_and(|s| {
                            s.starts_with('/')
                                && s.ends_with(
                                    "/x86_64-unknown-linux-gnu/release/diagnostic_held_bootstrap",
                                )
                        })
                    {
                        return Err(ArtifactError::Contents);
                    }
                    seen = true;
                }
            }
            "compiler-message" => {
                if e.message.ok_or(ArtifactError::Contents)?.level == "error" {
                    return Err(ArtifactError::Contents);
                }
            }
            "build-script-executed" => {}
            "build-finished" if e.success == Some(true) && seen => finished = true,
            _ => return Err(ArtifactError::Contents),
        }
    }
    if !finished {
        return Err(ArtifactError::Contents);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
#[path = "diagnostic_bootstrap_sealed.rs"]
pub(crate) mod sealed;
#[cfg(test)]
#[path = "diagnostic_bootstrap_build_tests.rs"]
pub(crate) mod tests;

#[path = "diagnostic_bootstrap_entry_format.rs"]
pub mod entry_format;
