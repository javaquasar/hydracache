use crate::event::{
    append_lifecycle_event, append_or_replay, request_sha256, verify_event_journal, EventAppend,
    EventError, EventOutcome, LifecycleEvent, EVENT_HEAD_NAME, EVENT_JOURNAL_NAME,
};
use crate::host_execution::HostExecutionClaim;
use crate::mutation::{reconcile_campaign, MutationError};
use crate::protocol::{sign_response, Operation, Request, Response, ResponseBody};
use crate::spawn::{
    apply_spawn_result, read_spawn_result, start_or_recover, SpawnBackend, SpawnError, SpawnIntent,
    SpawnResolution,
};
use crate::state::{transition, CampaignState, DurableCampaignState, FrozenIdentity, Transition};
use crate::state_store::{CampaignLock, StateStoreError};
use crate::Role;
use std::fs;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StartLifecycleError {
    #[error("start lifecycle request, host claim, identity, or phase is invalid")]
    Binding,
    #[error("start lifecycle event failed: {0}")]
    Event(#[from] EventError),
    #[error("start lifecycle state failed: {0}")]
    State(#[from] StateStoreError),
    #[error("start lifecycle reconciliation failed: {0}")]
    Mutation(#[from] MutationError),
    #[error("start lifecycle spawn failed: {0}")]
    Spawn(#[from] SpawnError),
}

pub fn drive_i74_start<B: SpawnBackend>(
    host_claim: &HostExecutionClaim,
    lock: &CampaignLock,
    request: &Request,
    identity: FrozenIdentity,
    nonce_sha256: String,
    now_unix_seconds: u64,
    backend: &mut B,
) -> Result<DurableCampaignState, StartLifecycleError> {
    drive_role_start(
        host_claim,
        lock,
        request,
        identity,
        nonce_sha256,
        now_unix_seconds,
        Role::I74,
        backend,
    )
}

pub fn drive_c74_start<B: SpawnBackend>(
    host_claim: &HostExecutionClaim,
    lock: &CampaignLock,
    request: &Request,
    identity: FrozenIdentity,
    nonce_sha256: String,
    now_unix_seconds: u64,
    backend: &mut B,
) -> Result<DurableCampaignState, StartLifecycleError> {
    drive_role_start(
        host_claim,
        lock,
        request,
        identity,
        nonce_sha256,
        now_unix_seconds,
        Role::C74,
        backend,
    )
}

#[allow(clippy::too_many_arguments)]
fn drive_role_start<B: SpawnBackend>(
    host_claim: &HostExecutionClaim,
    lock: &CampaignLock,
    request: &Request,
    identity: FrozenIdentity,
    nonce_sha256: String,
    now_unix_seconds: u64,
    role: Role,
    backend: &mut B,
) -> Result<DurableCampaignState, StartLifecycleError> {
    let is_i74 = role == Role::I74;
    if request.operation != Operation::Start
        || is_i74 != (request.expected_state_revision == 0)
        || is_i74 != request.manifest_path.is_some()
        || request.campaign_id != identity.campaign_id
        || request.manifest_sha256 != identity.manifest_sha256
        || host_claim.campaign_id() != request.campaign_id
        || lock.campaign_directory().parent() != Some(host_claim.campaign_root())
    {
        return Err(StartLifecycleError::Binding);
    }
    let digest = request_sha256(request)?;
    let mut state = match role {
        Role::I74 => load_or_prepare(lock, request, identity, now_unix_seconds, &digest)?,
        Role::C74 => load_existing(lock, identity)?,
    };

    let (ready, starting, start_transition, starting_event) = match role {
        Role::I74 => (
            CampaignState::Prepared,
            CampaignState::I74Starting,
            Transition::StartI74,
            LifecycleEvent::I74Starting,
        ),
        Role::C74 => (
            CampaignState::I74Sealed,
            CampaignState::C74Starting,
            Transition::StartC74,
            LifecycleEvent::C74Starting,
        ),
    };
    if state.campaign_state == ready {
        if state.revision != request.expected_state_revision
            || state.harness.is_some()
            || state.daemon.is_some()
            || state.checkpoint.is_some()
        {
            return Err(StartLifecycleError::Binding);
        }
        let mut starting = state.clone();
        starting.revision = starting
            .revision
            .checked_add(1)
            .ok_or(StartLifecycleError::Binding)?;
        starting.campaign_state = transition(state.campaign_state, start_transition)
            .map_err(|_| StartLifecycleError::Binding)?;
        append_transition(
            lock,
            request,
            &digest,
            now_unix_seconds,
            starting_event,
            &starting,
        )?;
        lock.compare_and_swap(state.revision, &starting)?;
        state = starting;
    }

    let intent = SpawnIntent::new(
        request.campaign_id.clone(),
        request.request_id.clone(),
        digest,
        request.manifest_sha256.clone(),
        nonce_sha256,
        role.clone(),
    )?;
    if state.campaign_state != starting {
        verify_completed_spawn(lock, &state, &intent, &role)?;
        return Ok(state);
    }

    let result = start_or_recover(lock.campaign_directory(), &intent, backend)?;
    let next = apply_spawn_result(&state, &role, &result)?;
    let lifecycle = match (&role, result.resolution) {
        (Role::I74, SpawnResolution::Started) => LifecycleEvent::I74Started,
        (Role::I74, SpawnResolution::Adopted) => LifecycleEvent::I74Adopted,
        (Role::I74, SpawnResolution::Absent) => LifecycleEvent::I74SpawnAbsent,
        (Role::I74, SpawnResolution::Mismatch) => LifecycleEvent::I74SpawnMismatch,
        (Role::C74, SpawnResolution::Started) => LifecycleEvent::C74Started,
        (Role::C74, SpawnResolution::Adopted) => LifecycleEvent::C74Adopted,
        (Role::C74, SpawnResolution::Absent) => LifecycleEvent::C74SpawnAbsent,
        (Role::C74, SpawnResolution::Mismatch) => LifecycleEvent::C74SpawnMismatch,
    };
    append_transition(
        lock,
        request,
        &intent.request_sha256,
        now_unix_seconds,
        lifecycle,
        &next,
    )?;
    lock.compare_and_swap(state.revision, &next)?;
    Ok(next)
}

pub fn drive_i74_start_request<B: SpawnBackend>(
    host_claim: &HostExecutionClaim,
    lock: &CampaignLock,
    request: &Request,
    identity: FrozenIdentity,
    nonce_sha256: String,
    now_unix_seconds: u64,
    backend: &mut B,
) -> Result<Response, StartLifecycleError> {
    drive_role_start_request(
        host_claim,
        lock,
        request,
        identity,
        nonce_sha256,
        now_unix_seconds,
        Role::I74,
        backend,
    )
}

pub fn drive_c74_start_request<B: SpawnBackend>(
    host_claim: &HostExecutionClaim,
    lock: &CampaignLock,
    request: &Request,
    identity: FrozenIdentity,
    nonce_sha256: String,
    now_unix_seconds: u64,
    backend: &mut B,
) -> Result<Response, StartLifecycleError> {
    drive_role_start_request(
        host_claim,
        lock,
        request,
        identity,
        nonce_sha256,
        now_unix_seconds,
        Role::C74,
        backend,
    )
}

#[allow(clippy::too_many_arguments)]
fn drive_role_start_request<B: SpawnBackend>(
    host_claim: &HostExecutionClaim,
    lock: &CampaignLock,
    request: &Request,
    identity: FrozenIdentity,
    nonce_sha256: String,
    now_unix_seconds: u64,
    role: Role,
    backend: &mut B,
) -> Result<Response, StartLifecycleError> {
    let digest = request_sha256(request)?;
    let journal = lock.campaign_directory().join(EVENT_JOURNAL_NAME);
    let head = lock.campaign_directory().join(EVENT_HEAD_NAME);
    if let Ok(report) = verify_event_journal(&journal, &head) {
        if let Some(recorded) = report.replay_index.get(&request.request_id) {
            return if recorded.request_sha256 == digest {
                Ok(recorded.response.clone())
            } else {
                Err(EventError::ReplayConflict {
                    request_id: request.request_id.clone(),
                }
                .into())
            };
        }
    }

    let state = drive_role_start(
        host_claim,
        lock,
        request,
        identity,
        nonce_sha256,
        now_unix_seconds,
        role.clone(),
        backend,
    )?;
    let accepted = state.campaign_state
        == match role {
            Role::I74 => CampaignState::I74Running,
            Role::C74 => CampaignState::C74Running,
        };
    if !accepted
        && !matches!(
            state.campaign_state,
            CampaignState::FailedIncomplete | CampaignState::CorruptQuarantined
        )
    {
        return Err(StartLifecycleError::Binding);
    }
    let response = sign_response(ResponseBody {
        schema_version: 1,
        request_id: request.request_id.clone(),
        campaign_id: request.campaign_id.clone(),
        ok: accepted,
        state_revision: state.revision,
        server_time_unix_seconds: now_unix_seconds,
        result: Some(serde_json::to_value(&state).map_err(EventError::from)?),
        error_code: (!accepted).then_some(11),
    })
    .map_err(EventError::from)?;
    match append_or_replay(
        &journal,
        &head,
        now_unix_seconds,
        request.clone(),
        if accepted {
            EventOutcome::Accepted
        } else {
            EventOutcome::Rejected
        },
        response.clone(),
        None,
    )? {
        EventAppend::Appended(_) => Ok(response),
        EventAppend::Replayed(recorded) => Ok(recorded),
    }
}

fn load_or_prepare(
    lock: &CampaignLock,
    request: &Request,
    identity: FrozenIdentity,
    now_unix_seconds: u64,
    request_digest: &str,
) -> Result<DurableCampaignState, StartLifecycleError> {
    match lock.read() {
        Ok(_) => {
            let journal = lock.campaign_directory().join(EVENT_JOURNAL_NAME);
            if fs::symlink_metadata(journal).is_err() {
                return Err(StartLifecycleError::Binding);
            }
            let state = reconcile_campaign(lock)?;
            if state.identity != identity {
                return Err(StartLifecycleError::Binding);
            }
            Ok(state)
        }
        Err(StateStoreError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            let journal = lock.campaign_directory().join(EVENT_JOURNAL_NAME);
            let head = lock.campaign_directory().join(EVENT_HEAD_NAME);
            let prepared = match fs::symlink_metadata(&journal) {
                Ok(_) => verify_event_journal(&journal, &head)?
                    .latest_state_after
                    .ok_or(StartLifecycleError::Binding)?,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    let prepared = DurableCampaignState {
                        revision: 0,
                        campaign_state: CampaignState::Prepared,
                        identity: identity.clone(),
                        harness: None,
                        daemon: None,
                        checkpoint: None,
                        controller_lease: None,
                        recorded_failure: false,
                        duplicate_executor: false,
                        durable_history_corrupt: false,
                    };
                    append_transition(
                        lock,
                        request,
                        request_digest,
                        now_unix_seconds,
                        LifecycleEvent::Prepared,
                        &prepared,
                    )?;
                    prepared
                }
                Err(error) => return Err(StateStoreError::Io(error).into()),
            };
            if prepared.identity != identity {
                return Err(StartLifecycleError::Binding);
            }
            lock.initialize(&prepared)?;
            Ok(prepared)
        }
        Err(error) => Err(error.into()),
    }
}

fn load_existing(
    lock: &CampaignLock,
    identity: FrozenIdentity,
) -> Result<DurableCampaignState, StartLifecycleError> {
    let journal = lock.campaign_directory().join(EVENT_JOURNAL_NAME);
    if fs::symlink_metadata(journal).is_err() {
        return Err(StartLifecycleError::Binding);
    }
    let state = reconcile_campaign(lock)?;
    if state.identity != identity {
        return Err(StartLifecycleError::Binding);
    }
    Ok(state)
}

fn verify_completed_spawn(
    lock: &CampaignLock,
    state: &DurableCampaignState,
    intent: &SpawnIntent,
    role: &Role,
) -> Result<(), StartLifecycleError> {
    let result = read_spawn_result(lock.campaign_directory(), intent)?;
    match (role, state.campaign_state) {
        (Role::I74, CampaignState::I74Running) | (Role::C74, CampaignState::C74Running) => {
            let (harness, daemon) = result
                .exact_identities()
                .ok_or(StartLifecycleError::Binding)?;
            if state.harness.as_ref() != Some(harness) || state.daemon.as_ref() != Some(daemon) {
                return Err(StartLifecycleError::Binding);
            }
        }
        (_, CampaignState::FailedIncomplete) if result.resolution == SpawnResolution::Absent => {}
        (_, CampaignState::CorruptQuarantined)
            if result.resolution == SpawnResolution::Mismatch => {}
        _ => return Err(StartLifecycleError::Binding),
    }
    Ok(())
}

fn append_transition(
    lock: &CampaignLock,
    request: &Request,
    request_digest: &str,
    now_unix_seconds: u64,
    transition: LifecycleEvent,
    state: &DurableCampaignState,
) -> Result<(), EventError> {
    append_lifecycle_event(
        &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &lock.campaign_directory().join(EVENT_HEAD_NAME),
        now_unix_seconds,
        request.request_id.clone(),
        request_digest.to_owned(),
        transition,
        state.clone(),
    )?;
    Ok(())
}
