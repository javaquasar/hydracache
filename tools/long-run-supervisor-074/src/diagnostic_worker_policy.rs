//! Signed policy bytes under external pins; Linux file consistency is a separate child.
use crate::{canonical_json, is_hash, sha256_hex};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const WORKER_POLICY_SCHEMA: &str = "diagnostic-worker-policy-074-v1";
pub const SIGNED_WORKER_POLICY_SCHEMA: &str = "diagnostic-signed-worker-policy-074-v1";
pub const WORKER_POLICY_DOMAIN: &[u8] = b"hydracache-diagnostic-worker-policy-074-v1";
pub const MAX_WORKER_POLICY_BYTES: usize = 4096;
pub const MAX_SIGNED_WORKER_POLICY_BYTES: usize = 8192;
const REPOSITORY_ID: u64 = 1_217_101_761;
const GROUP_LIMIT: usize = 32;

#[cfg(target_os = "linux")]
#[path = "diagnostic_worker_files.rs"]
pub mod local_files;

/// Values in a policy or caller assertion, not verified kernel namespace handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamespaceIdentity {
    pub device: u64,
    pub inode: u64,
}

/// Unverified wire body. Only verify_worker_policy constructs a checked guard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerPolicy {
    pub schema_version: String,
    pub repository_id: u64,
    pub purpose: String,
    pub account_source: String,
    pub account: String,
    pub group: String,
    pub policy_epoch: u64,
    pub uid: u32,
    pub gid: u32,
    pub supplementary_gids: Vec<u32>,
    pub machine_id: String,
    pub boot_id: String,
    pub user_namespace: NamespaceIdentity,
    pub mount_namespace: NamespaceIdentity,
    pub passwd_sha256: String,
    pub group_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedWorkerPolicy {
    pub schema_version: String,
    pub policy: WorkerPolicy,
    pub signature_hex: String,
}

/// Caller assertion only; validation authenticates no live host or namespace.
#[derive(Clone, PartialEq, Eq)]
pub struct AssertedWorkerHost {
    pub machine_id: String,
    pub boot_id: String,
    pub user_namespace: NamespaceIdentity,
    pub mount_namespace: NamespaceIdentity,
}

/// Independent operator configuration, never derived from a policy envelope.
#[derive(Clone, PartialEq, Eq)]
pub struct WorkerPolicyTrust {
    key: VerifyingKey,
    policy_sha256: String,
    epoch: u64,
}
impl WorkerPolicyTrust {
    pub fn new(
        issuer_key: VerifyingKey,
        policy_sha256: &str,
        epoch: u64,
    ) -> Result<Self, WorkerPolicyError> {
        if issuer_key.is_weak() || !is_hash(policy_sha256) || epoch == 0 {
            return Err(WorkerPolicyError::Invalid);
        }
        Ok(Self {
            key: issuer_key,
            policy_sha256: policy_sha256.into(),
            epoch,
        })
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WorkerPolicyError {
    #[error("worker policy document is malformed, noncanonical or over budget")]
    Document,
    #[error("worker policy or asserted context violates fixed invariants")]
    Invalid,
    #[error("worker policy digest differs from the external pin")]
    Pin,
    #[error("worker policy signature is invalid under the external issuer key")]
    Signature,
    #[error("worker policy differs from the asserted host/boot/namespace context")]
    Context,
    #[error("worker policy epoch or original external trust has changed")]
    Revoked,
    #[error("original worker policy envelope or projection has changed")]
    Drift,
    #[error("original worker policy guard previously refused")]
    Refused,
}

/// Policy-byte verification only: no conversion into a process/start capability.
pub struct CheckedWorkerPolicy {
    original: WorkerPolicy,
    trust: WorkerPolicyTrust,
    context: AssertedWorkerHost,
    envelope_sha256: String,
    refused: bool,
}
impl CheckedWorkerPolicy {
    pub fn policy_sha256(&self) -> &str {
        &self.trust.policy_sha256
    }
    pub fn is_refused(&self) -> bool {
        self.refused
    }
    pub fn revalidate(
        &mut self,
        bytes: &[u8],
        current_trust: &WorkerPolicyTrust,
        asserted_context: &AssertedWorkerHost,
    ) -> Result<(), WorkerPolicyError> {
        if self.refused {
            return Err(WorkerPolicyError::Refused);
        }
        let result = (|| {
            if current_trust != &self.trust {
                return Err(WorkerPolicyError::Revoked);
            }
            if asserted_context != &self.context {
                return Err(WorkerPolicyError::Context);
            }
            if bytes.len() > MAX_SIGNED_WORKER_POLICY_BYTES {
                return Err(WorkerPolicyError::Document);
            }
            if sha256_hex(bytes) != self.envelope_sha256 {
                return Err(WorkerPolicyError::Drift);
            }
            let observed = verify_document(bytes, current_trust, asserted_context)?;
            if observed != self.original {
                return Err(WorkerPolicyError::Drift);
            }
            Ok(())
        })();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
}

pub fn verify_worker_policy(
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    asserted_context: &AssertedWorkerHost,
) -> Result<CheckedWorkerPolicy, WorkerPolicyError> {
    let original = verify_document(bytes, trust, asserted_context)?;
    Ok(CheckedWorkerPolicy {
        original,
        trust: trust.clone(),
        context: asserted_context.clone(),
        envelope_sha256: sha256_hex(bytes),
        refused: false,
    })
}

fn verify_document(
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<WorkerPolicy, WorkerPolicyError> {
    if bytes.is_empty() || bytes.len() > MAX_SIGNED_WORKER_POLICY_BYTES {
        return Err(WorkerPolicyError::Document);
    }
    let document: SignedWorkerPolicy =
        serde_json::from_slice(bytes).map_err(|_| WorkerPolicyError::Document)?;
    let body_bytes = canonical_line(&document.policy)?;
    if body_bytes.len() > MAX_WORKER_POLICY_BYTES || canonical_line(&document)? != bytes {
        return Err(WorkerPolicyError::Document);
    }
    let policy = &document.policy;
    if document.schema_version != SIGNED_WORKER_POLICY_SCHEMA
        || policy.schema_version != WORKER_POLICY_SCHEMA
        || policy.repository_id != REPOSITORY_ID
        || policy.purpose != "diagnostic-worker-only"
        || policy.account_source != "local-files"
        || policy.account != "hydracache-perf"
        || policy.group != "hydracache-perf"
        || policy.policy_epoch == 0
        || !valid_id(policy.uid)
        || !valid_id(policy.gid)
        || policy.supplementary_gids.len() > GROUP_LIMIT
        || policy.supplementary_gids.iter().any(|id| !valid_id(*id))
        || policy.supplementary_gids.windows(2).any(|p| p[0] >= p[1])
        || (policy.supplementary_gids.len() == GROUP_LIMIT
            && !policy.supplementary_gids.contains(&policy.gid))
        || !is_hash(&policy.passwd_sha256)
        || !is_hash(&policy.group_sha256)
        || !valid_context(host)
        || !valid_context(&AssertedWorkerHost {
            machine_id: policy.machine_id.clone(),
            boot_id: policy.boot_id.clone(),
            user_namespace: policy.user_namespace,
            mount_namespace: policy.mount_namespace,
        })
    {
        return Err(WorkerPolicyError::Invalid);
    }
    if policy.policy_epoch != trust.epoch {
        return Err(WorkerPolicyError::Revoked);
    }
    if sha256_hex(&body_bytes) != trust.policy_sha256 {
        return Err(WorkerPolicyError::Pin);
    }
    let signature = signature(&document.signature_hex)?;
    let mut message = WORKER_POLICY_DOMAIN.to_vec();
    message.push(0);
    message.extend(body_bytes);
    trust
        .key
        .verify_strict(&message, &signature)
        .map_err(|_| WorkerPolicyError::Signature)?;
    if policy.machine_id != host.machine_id
        || policy.boot_id != host.boot_id
        || policy.user_namespace != host.user_namespace
        || policy.mount_namespace != host.mount_namespace
    {
        return Err(WorkerPolicyError::Context);
    }
    Ok(document.policy)
}

fn canonical_line(value: &impl Serialize) -> Result<Vec<u8>, WorkerPolicyError> {
    let mut bytes = canonical_json(value).map_err(|_| WorkerPolicyError::Document)?;
    bytes.push(b'\n');
    Ok(bytes)
}
fn valid_id(id: u32) -> bool {
    !matches!(id, 0 | u32::MAX)
}
fn lower_hex(byte: u8) -> bool {
    byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
}
fn valid_context(host: &AssertedWorkerHost) -> bool {
    let boot = host.boot_id.as_bytes();
    host.machine_id.len() == 32
        && host.machine_id.bytes().all(lower_hex)
        && host.machine_id.bytes().any(|b| b != b'0')
        && boot.len() == 36
        && boot.iter().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                *b == b'-'
            } else {
                lower_hex(*b)
            }
        })
        && boot.iter().any(|b| *b != b'0' && *b != b'-')
        && host.user_namespace.device != 0
        && host.user_namespace.inode != 0
        && host.mount_namespace.device != 0
        && host.mount_namespace.inode != 0
}
fn signature(value: &str) -> Result<Signature, WorkerPolicyError> {
    if value.len() != 128 || !value.bytes().all(lower_hex) {
        return Err(WorkerPolicyError::Document);
    }
    let mut decoded = [0; 64];
    for (i, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let nibble = |b: u8| if b <= b'9' { b - b'0' } else { b - b'a' + 10 };
        decoded[i] = (nibble(pair[0]) << 4) | nibble(pair[1]);
    }
    Ok(Signature::from_bytes(&decoded))
}
