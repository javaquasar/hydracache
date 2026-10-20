//! Local authority journal bytes; independently supplied pins, never start authority.
#[cfg(any(target_os = "linux", test))]
use super::CheckedWorkerPolicy;
use super::{AssertedWorkerHost, WorkerPolicyTrust};
use crate::{canonical_json, is_hash, sha256_hex};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const AUTHORITY_RECORD_SCHEMA: &str = "diagnostic-authority-record-074-v1";
pub const MAX_GENERATIONS: usize = 128;
pub const MAX_RECORD_BYTES: usize = 2048;
pub const COMMIT_MARKER_BYTES: usize = 65;

#[cfg(target_os = "linux")]
#[path = "diagnostic_authority_fixture.rs"]
pub mod fixture;

#[cfg(target_os = "linux")]
pub use fixture::fixed;

#[derive(Debug, Error)]
pub enum AuthorityError {
    #[error("authority document, metadata or history is invalid")]
    Invalid,
    #[error("authority journal exceeds its frozen bound")]
    Budget,
    #[error("authority binding differs from independently supplied pins or context")]
    Binding,
    #[error("authority epoch transition is not monotonic or not explicitly refused")]
    Transition,
    #[error("original authority writer permanently refused")]
    Refused,
    #[error("authority I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("logical authority publication fault at {0:?}")]
    Injected(PublicationStage),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RefusalReason {
    Operator,
    Policy,
    Context,
    Credentials,
    Storage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationStage {
    RecordCreated,
    RecordSynced,
    MarkerCreated,
    DurableCommit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum State {
    Active,
    Refused,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema_version: String,
    repository_id: u64,
    purpose: String,
    revision: u64,
    previous_sha256: Option<String>,
    issuer_key_hex: String,
    policy_sha256: String,
    policy_epoch: u64,
    machine_id: String,
    boot_id: String,
    user_namespace: super::NamespaceIdentity,
    mount_namespace: super::NamespaceIdentity,
    state: State,
    refusal_reason: Option<RefusalReason>,
}

/// Inspection data only; no conversion into enrollment or a launch capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorityView {
    pub revision: u64,
    pub epoch: u64,
    pub refused: bool,
}
impl Record {
    fn view(&self) -> AuthorityView {
        AuthorityView {
            revision: self.revision,
            epoch: self.policy_epoch,
            refused: self.state == State::Refused,
        }
    }
    fn context(&self) -> AssertedWorkerHost {
        AssertedWorkerHost {
            machine_id: self.machine_id.clone(),
            boot_id: self.boot_id.clone(),
            user_namespace: self.user_namespace,
            mount_namespace: self.mount_namespace,
        }
    }
    fn bind(
        &self,
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<(), AuthorityError> {
        if self.issuer_key_hex != key_hex(trust)
            || self.policy_sha256 != trust.policy_sha256
            || self.policy_epoch != trust.epoch
            || self.context() != *host
        {
            return Err(AuthorityError::Binding);
        }
        Ok(())
    }
    fn bytes(&self) -> Result<Vec<u8>, AuthorityError> {
        let mut bytes = canonical_json(self).map_err(|_| AuthorityError::Invalid)?;
        bytes.push(b'\n');
        if bytes.len() > MAX_RECORD_BYTES {
            return Err(AuthorityError::Budget);
        }
        Ok(bytes)
    }
    fn parse(bytes: &[u8]) -> Result<Self, AuthorityError> {
        if bytes.is_empty() || bytes.len() > MAX_RECORD_BYTES {
            return Err(AuthorityError::Budget);
        }
        let record: Self = serde_json::from_slice(bytes).map_err(|_| AuthorityError::Invalid)?;
        if record.bytes()? != bytes
            || record.schema_version != AUTHORITY_RECORD_SCHEMA
            || record.repository_id != super::REPOSITORY_ID
            || record.purpose != "diagnostic-worker-only"
            || record.revision == 0
            || record.revision > MAX_GENERATIONS as u64
            || record.policy_epoch == 0
            || !is_hash(&record.issuer_key_hex)
            || !is_hash(&record.policy_sha256)
            || record
                .previous_sha256
                .as_ref()
                .is_some_and(|hash| !is_hash(hash))
            || !super::valid_context(&record.context())
            || (record.state == State::Refused) != record.refusal_reason.is_some()
        {
            return Err(AuthorityError::Invalid);
        }
        Ok(record)
    }
    #[cfg(any(target_os = "linux", test))]
    fn from_checked(policy: &CheckedWorkerPolicy) -> Result<Self, AuthorityError> {
        if policy.is_refused() {
            return Err(AuthorityError::Refused);
        }
        Ok(Self {
            schema_version: AUTHORITY_RECORD_SCHEMA.into(),
            repository_id: super::REPOSITORY_ID,
            purpose: "diagnostic-worker-only".into(),
            revision: 1,
            previous_sha256: None,
            issuer_key_hex: key_hex(&policy.trust),
            policy_sha256: policy.trust.policy_sha256.clone(),
            policy_epoch: policy.trust.epoch,
            machine_id: policy.context.machine_id.clone(),
            boot_id: policy.context.boot_id.clone(),
            user_namespace: policy.context.user_namespace,
            mount_namespace: policy.context.mount_namespace,
            state: State::Active,
            refusal_reason: None,
        })
    }
    fn refuse(&self, reason: RefusalReason) -> Result<Self, AuthorityError> {
        if self.state != State::Active {
            return Err(AuthorityError::Transition);
        }
        let mut next = self.clone();
        next.revision = self.next_revision()?;
        next.previous_sha256 = Some(sha256_hex(&self.bytes()?));
        next.state = State::Refused;
        next.refusal_reason = Some(reason);
        Ok(next)
    }
    #[cfg(any(target_os = "linux", test))]
    fn advance(&self, policy: &CheckedWorkerPolicy) -> Result<Self, AuthorityError> {
        let mut next = Self::from_checked(policy)?;
        if self.state != State::Refused
            || next.policy_epoch <= self.policy_epoch
            || next.machine_id != self.machine_id
            || next.issuer_key_hex != self.issuer_key_hex
            || next.policy_sha256 == self.policy_sha256
        {
            return Err(AuthorityError::Transition);
        }
        next.revision = self.next_revision()?;
        next.previous_sha256 = Some(sha256_hex(&self.bytes()?));
        Ok(next)
    }
    fn next_revision(&self) -> Result<u64, AuthorityError> {
        if self.revision >= MAX_GENERATIONS as u64 {
            return Err(AuthorityError::Budget);
        }
        Ok(self.revision + 1)
    }
}
fn key_hex(trust: &WorkerPolicyTrust) -> String {
    crate::hex(trust.key.as_bytes())
}

#[cfg(all(test, target_os = "linux"))]
#[path = "../tests/support/authority_hex_controls.rs"]
pub(crate) mod hex_controls;

fn history(records: &[Vec<u8>]) -> Result<Vec<Record>, AuthorityError> {
    if records.is_empty() || records.len() > MAX_GENERATIONS {
        return Err(AuthorityError::Budget);
    }
    let mut checked: Vec<Record> = Vec::with_capacity(records.len());
    for (index, raw) in records.iter().enumerate() {
        let record = Record::parse(raw)?;
        if record.revision != (index + 1) as u64 {
            return Err(AuthorityError::Transition);
        }
        if let Some(previous) = checked.last() {
            if record.previous_sha256.as_deref() != Some(sha256_hex(&records[index - 1]).as_str()) {
                return Err(AuthorityError::Transition);
            }
            if record.state == State::Refused {
                let expected =
                    previous.refuse(record.refusal_reason.ok_or(AuthorityError::Invalid)?)?;
                if record != expected {
                    return Err(AuthorityError::Transition);
                }
            } else if previous.state != State::Refused
                || record.policy_epoch <= previous.policy_epoch
                || record.machine_id != previous.machine_id
                || record.issuer_key_hex != previous.issuer_key_hex
                || record.policy_sha256 == previous.policy_sha256
            {
                return Err(AuthorityError::Transition);
            }
        } else if record.state != State::Active || record.previous_sha256.is_some() {
            return Err(AuthorityError::Transition);
        }
        checked.push(record);
    }
    Ok(checked)
}

/// Strict bounded byte inspection against independent current pins, not a signed store.
pub fn inspect_registry_history(
    records: &[Vec<u8>],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<AuthorityView, AuthorityError> {
    let records = history(records)?;
    let last = records.last().ok_or(AuthorityError::Invalid)?;
    last.bind(trust, host)?;
    Ok(last.view())
}

#[cfg(test)]
#[path = "diagnostic_authority_registry_tests.rs"]
mod tests;
