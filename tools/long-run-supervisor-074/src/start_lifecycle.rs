use crate::event::{
    append_lifecycle_event, request_sha256, verify_event_journal, EventError, LifecycleEvent,
    EVENT_HEAD_NAME, EVENT_JOURNAL_NAME,
};
use crate::host_execution::HostExecutionClaim;
use crate::mutation::{reconcile_campaign, MutationError};
use crate::protocol::{Operation, Request};
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
    if request.operation != Operation::Start
        || request.expected_state_revision != 0
        || request.campaign_id != identity.campaign_id
        || request.manifest_sha256 != identity.manifest_sha256
        || host_claim.campaign_id() != request.campaign_id
        || lock.campaign_directory().parent() != Some(host_claim.campaign_root())
    {
        return Err(StartLifecycleError::Binding);
    }
    let digest = request_sha256(request)?;
    let mut state = load_or_prepare(lock, request, identity, now_unix_seconds, &digest)?;

    if state.campaign_state == CampaignState::Prepared {
        let mut starting = state.clone();
        starting.revision = starting
            .revision
            .checked_add(1)
            .ok_or(StartLifecycleError::Binding)?;
        starting.campaign_state = transition(state.campaign_state, Transition::StartI74)
            .map_err(|_| StartLifecycleError::Binding)?;
        append_transition(
            lock,
            request,
            &digest,
            now_unix_seconds,
            LifecycleEvent::I74Starting,
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
        Role::I74,
    )?;
    if state.campaign_state != CampaignState::I74Starting {
        verify_completed_spawn(lock, &state, &intent)?;
        return Ok(state);
    }

    let result = start_or_recover(lock.campaign_directory(), &intent, backend)?;
    let next = apply_spawn_result(&state, &Role::I74, &result)?;
    let lifecycle = match result.resolution {
        SpawnResolution::Started => LifecycleEvent::I74Started,
        SpawnResolution::Adopted => LifecycleEvent::I74Adopted,
        SpawnResolution::Absent => LifecycleEvent::I74SpawnAbsent,
        SpawnResolution::Mismatch => LifecycleEvent::I74SpawnMismatch,
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

fn verify_completed_spawn(
    lock: &CampaignLock,
    state: &DurableCampaignState,
    intent: &SpawnIntent,
) -> Result<(), StartLifecycleError> {
    let result = read_spawn_result(lock.campaign_directory(), intent)?;
    match state.campaign_state {
        CampaignState::I74Running => {
            let (harness, daemon) = result
                .exact_identities()
                .ok_or(StartLifecycleError::Binding)?;
            if state.harness.as_ref() != Some(harness) || state.daemon.as_ref() != Some(daemon) {
                return Err(StartLifecycleError::Binding);
            }
        }
        CampaignState::FailedIncomplete if result.resolution == SpawnResolution::Absent => {}
        CampaignState::CorruptQuarantined if result.resolution == SpawnResolution::Mismatch => {}
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
