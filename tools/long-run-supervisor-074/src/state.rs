use crate::{ProcessIdentity, GENESIS_HASH};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CampaignState {
    Prepared,
    I74Starting,
    I74Running,
    I74Terminal,
    I74Sealed,
    C74Starting,
    C74Running,
    C74Terminal,
    CompleteSealed,
    FailedIncomplete,
    AbortedIncomplete,
    LeaseExpiredIncomplete,
    CorruptQuarantined,
}

impl CampaignState {
    pub fn is_live(self) -> bool {
        matches!(
            self,
            Self::I74Starting | Self::I74Running | Self::C74Starting | Self::C74Running
        )
    }

    pub fn is_corrupt(self) -> bool {
        self == Self::CorruptQuarantined
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    StartI74,
    MarkI74Running,
    MarkI74Terminal,
    SealI74,
    StartC74,
    MarkC74Running,
    MarkC74Terminal,
    SealComplete,
    Fail,
    Abort,
    ExpireLease,
    Quarantine,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("transition {transition:?} is invalid from {state:?}")]
pub struct TransitionError {
    pub state: CampaignState,
    pub transition: Transition,
}

pub fn transition(
    state: CampaignState,
    operation: Transition,
) -> Result<CampaignState, TransitionError> {
    use CampaignState as S;
    use Transition as T;
    let next = match (state, operation) {
        (S::Prepared, T::StartI74) => S::I74Starting,
        (S::I74Starting, T::MarkI74Running) => S::I74Running,
        (S::I74Starting | S::I74Running, T::MarkI74Terminal) => S::I74Terminal,
        (S::I74Terminal, T::SealI74) => S::I74Sealed,
        (S::I74Sealed, T::StartC74) => S::C74Starting,
        (S::C74Starting, T::MarkC74Running) => S::C74Running,
        (S::C74Starting | S::C74Running, T::MarkC74Terminal) => S::C74Terminal,
        (S::C74Terminal, T::SealComplete) => S::CompleteSealed,
        (
            S::Prepared
            | S::I74Starting
            | S::I74Running
            | S::I74Terminal
            | S::I74Sealed
            | S::C74Starting
            | S::C74Running
            | S::C74Terminal,
            T::Fail,
        ) => S::FailedIncomplete,
        (
            S::Prepared
            | S::I74Starting
            | S::I74Running
            | S::I74Terminal
            | S::I74Sealed
            | S::C74Starting
            | S::C74Running
            | S::C74Terminal
            | S::FailedIncomplete
            | S::CorruptQuarantined,
            T::Abort,
        ) => S::AbortedIncomplete,
        (current, T::ExpireLease) if current.is_live() => S::LeaseExpiredIncomplete,
        (_, T::Quarantine) => S::CorruptQuarantined,
        _ => {
            return Err(TransitionError {
                state,
                transition: operation,
            });
        }
    };
    Ok(next)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenIdentity {
    pub campaign_id: String,
    pub manifest_sha256: String,
    pub contract_sha256: String,
    pub scenario_sha256: String,
    pub tooling_sha256: String,
    pub source_bundle_sha256: String,
    pub binary_bundle_sha256: String,
    pub workload_bundle_sha256: String,
    pub machine_id: String,
    pub boot_id: String,
    pub host_receipt_sha256: String,
    pub mount_identity: String,
    pub isolated_cpuset: String,
    pub housekeeping_cpuset: String,
    pub command_environment_sha256: String,
    pub lease_id: String,
    pub lease_deadline_unix_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointHead {
    pub sequence: u64,
    pub record_sha256: String,
    pub useful_progress_unix_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControllerLease {
    pub holder_request_id: String,
    pub authorization_sha256: String,
    pub repository_id: u64,
    pub run_id: u64,
    pub actor_id: u64,
    pub expires_unix_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableCampaignState {
    pub revision: u64,
    pub campaign_state: CampaignState,
    pub identity: FrozenIdentity,
    pub harness: Option<ProcessIdentity>,
    pub daemon: Option<ProcessIdentity>,
    pub checkpoint: Option<CheckpointHead>,
    pub controller_lease: Option<ControllerLease>,
    pub recorded_failure: bool,
    pub duplicate_executor: bool,
    pub durable_history_corrupt: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachRequest {
    pub request_id: String,
    pub request_sha256: String,
    pub expected_revision: u64,
    pub authorization_sha256: String,
    pub repository_id: u64,
    pub run_id: u64,
    pub actor_id: u64,
    pub identity: FrozenIdentity,
    pub harness: ProcessIdentity,
    pub daemon: ProcessIdentity,
    pub checkpoint: CheckpointHead,
    pub now_unix_seconds: u64,
    pub requested_controller_lease_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AttachFailure {
    InvalidRequestIdentity,
    StaleRevision,
    CampaignIdentityDrift,
    HostOrBootDrift,
    ProcessIdentityDrift,
    CheckpointDrift,
    CheckpointStale,
    ProductLeaseExpired,
    ControllerLeaseConflict,
    RecordedFailure,
    DuplicateExecutor,
    CorruptHistory,
    NonLiveCampaign,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AttachDecision {
    pub admitted: bool,
    pub failures: Vec<AttachFailure>,
}

pub fn evaluate_attach(
    state: &DurableCampaignState,
    request: &AttachRequest,
    progress_rejection_gap_seconds: u64,
) -> AttachDecision {
    let mut failures = Vec::new();
    if request.request_id.is_empty()
        || !is_hash(&request.request_sha256)
        || !is_hash(&request.authorization_sha256)
        || request.repository_id == 0
        || request.run_id == 0
        || request.actor_id == 0
        || request.requested_controller_lease_seconds == 0
    {
        failures.push(AttachFailure::InvalidRequestIdentity);
    }
    if request.expected_revision != state.revision {
        failures.push(AttachFailure::StaleRevision);
    }
    if request.identity != state.identity {
        if request.identity.machine_id != state.identity.machine_id
            || request.identity.boot_id != state.identity.boot_id
            || request.identity.host_receipt_sha256 != state.identity.host_receipt_sha256
            || request.identity.mount_identity != state.identity.mount_identity
            || request.identity.isolated_cpuset != state.identity.isolated_cpuset
            || request.identity.housekeeping_cpuset != state.identity.housekeeping_cpuset
        {
            failures.push(AttachFailure::HostOrBootDrift);
        }
        failures.push(AttachFailure::CampaignIdentityDrift);
    }
    if state.harness.as_ref() != Some(&request.harness)
        || state.daemon.as_ref() != Some(&request.daemon)
    {
        failures.push(AttachFailure::ProcessIdentityDrift);
    }
    match &state.checkpoint {
        Some(checkpoint) => {
            if &request.checkpoint != checkpoint
                || checkpoint.useful_progress_unix_seconds > request.now_unix_seconds
            {
                failures.push(AttachFailure::CheckpointDrift);
            }
            if request.now_unix_seconds
                > checkpoint
                    .useful_progress_unix_seconds
                    .saturating_add(progress_rejection_gap_seconds)
            {
                failures.push(AttachFailure::CheckpointStale);
            }
        }
        None => failures.push(AttachFailure::CheckpointDrift),
    }
    if request.now_unix_seconds > state.identity.lease_deadline_unix_seconds {
        failures.push(AttachFailure::ProductLeaseExpired);
    }
    if state.controller_lease.as_ref().is_some_and(|lease| {
        lease.expires_unix_seconds >= request.now_unix_seconds
            && lease.authorization_sha256 != request.authorization_sha256
    }) {
        failures.push(AttachFailure::ControllerLeaseConflict);
    }
    if state.recorded_failure {
        failures.push(AttachFailure::RecordedFailure);
    }
    if state.duplicate_executor {
        failures.push(AttachFailure::DuplicateExecutor);
    }
    if state.durable_history_corrupt || state.campaign_state.is_corrupt() {
        failures.push(AttachFailure::CorruptHistory);
    }
    if !state.campaign_state.is_live() {
        failures.push(AttachFailure::NonLiveCampaign);
    }
    failures.sort_unstable();
    failures.dedup();
    AttachDecision {
        admitted: failures.is_empty(),
        failures,
    }
}

pub fn apply_attach(
    state: &DurableCampaignState,
    request: &AttachRequest,
    progress_rejection_gap_seconds: u64,
) -> Result<DurableCampaignState, AttachDecision> {
    let decision = evaluate_attach(state, request, progress_rejection_gap_seconds);
    if !decision.admitted {
        return Err(decision);
    }
    let mut next = state.clone();
    next.revision = next.revision.saturating_add(1);
    next.controller_lease = Some(ControllerLease {
        holder_request_id: request.request_id.clone(),
        authorization_sha256: request.authorization_sha256.clone(),
        repository_id: request.repository_id,
        run_id: request.run_id,
        actor_id: request.actor_id,
        expires_unix_seconds: request
            .now_unix_seconds
            .saturating_add(request.requested_controller_lease_seconds)
            .min(state.identity.lease_deadline_unix_seconds),
    });
    Ok(next)
}

pub fn controller_lease_authorizes(
    state: &DurableCampaignState,
    repository_id: u64,
    run_id: u64,
    actor_id: u64,
    now_unix_seconds: u64,
) -> bool {
    now_unix_seconds <= state.identity.lease_deadline_unix_seconds
        && state.controller_lease.as_ref().is_some_and(|lease| {
            now_unix_seconds <= lease.expires_unix_seconds
                && repository_id == lease.repository_id
                && run_id == lease.run_id
                && actor_id == lease.actor_id
        })
}

#[derive(Debug, Default)]
pub struct ReplayMap {
    requests: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayDecision {
    New,
    Idempotent,
    DigestConflict,
}

impl ReplayMap {
    pub fn observe(&mut self, request_id: &str, request_sha256: &str) -> ReplayDecision {
        match self.requests.get(request_id) {
            None => {
                self.requests
                    .insert(request_id.to_owned(), request_sha256.to_owned());
                ReplayDecision::New
            }
            Some(existing) if existing == request_sha256 => ReplayDecision::Idempotent,
            Some(_) => ReplayDecision::DigestConflict,
        }
    }
}

fn is_hash(value: &str) -> bool {
    value != GENESIS_HASH
        && value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
