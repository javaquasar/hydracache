use crate::event::{
    append_lifecycle_event, append_or_replay, request_sha256, verify_event_journal, EventAppend,
    EventError, EventOutcome, EventVerificationReport, LifecycleEvent, EVENT_HEAD_NAME,
    EVENT_JOURNAL_NAME,
};
use crate::host_execution::{HostExecutionClaim, HostExecutionError};
use crate::mutation::{reconcile_campaign, MutationError};
use crate::protocol::{sign_response, Operation, Request, Response, ResponseBody};
use crate::spawn::{read_spawn_evidence, SpawnIntent, SpawnMismatch, SpawnResolution, SpawnResult};
use crate::state::{
    controller_lease_authorizes, transition, CampaignState, DurableCampaignState, Transition,
};
use crate::state_store::{CampaignLock, StateStoreError};
use crate::Role;
use std::path::Path;
use thiserror::Error;

pub trait AbortBackend {
    /// Capture the bounded allowlisted diagnostics and stop the exact retained unit.
    /// Implementations must recover or replay an identical request without starting a process.
    fn capture_and_stop(
        &mut self,
        campaign_directory: &Path,
        request: &Request,
        state: &DurableCampaignState,
    ) -> Result<(), String>;

    /// Capture an exact identity-mismatch quarantine and retire its empty,
    /// failed deterministic unit. No live or ambiguously owned process may be
    /// stopped through this recovery path.
    fn capture_quarantine_and_stop(
        &mut self,
        _campaign_directory: &Path,
        _request: &Request,
        _state: &DurableCampaignState,
        _cause: &QuarantineRecoveryCause,
    ) -> Result<(), String> {
        Err("quarantine recovery is not implemented by this backend".to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuarantineRecoveryCause {
    pub role: Role,
    pub intent: SpawnIntent,
    pub result: SpawnResult,
}

#[derive(Debug, Error)]
pub enum AbortLifecycleError {
    #[error("abort lifecycle request, host claim, identity, lease, or phase is invalid")]
    Binding,
    #[error("abort lifecycle event failed: {0}")]
    Event(#[from] EventError),
    #[error("abort lifecycle state failed: {0}")]
    State(#[from] StateStoreError),
    #[error("abort lifecycle reconciliation failed: {0}")]
    Mutation(#[from] MutationError),
    #[error("abort lifecycle host claim failed: {0}")]
    Host(#[from] HostExecutionError),
    #[error("abort lifecycle backend failed: {0}")]
    Backend(String),
    #[error("abort lifecycle response serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn drive_abort_request<B: AbortBackend + ?Sized>(
    host_claim: &HostExecutionClaim,
    lock: &CampaignLock,
    request: &Request,
    now_unix_seconds: u64,
    backend: &mut B,
) -> Result<Response, AbortLifecycleError> {
    let digest = request_sha256(request)?;
    let mut state = reconcile_campaign(lock)?;
    let mut report = event_report(lock).ok();
    if let Some(recorded) = report
        .as_ref()
        .and_then(|report| report.replay_index.get(&request.request_id))
    {
        if recorded.request_sha256 != digest {
            return Err(EventError::ReplayConflict {
                request_id: request.request_id.clone(),
            }
            .into());
        }
        let response_state = response_state(&recorded.response)?;
        if response_state != state {
            return Err(AbortLifecycleError::Binding);
        }
        release_claim(host_claim, &state)?;
        return Ok(recorded.response.clone());
    }

    let quarantine_cause =
        validate_common(host_claim, lock, request, &state, report.as_ref(), &digest)?;
    if state.campaign_state != CampaignState::AbortedIncomplete {
        if state.revision != request.expected_state_revision
            || (quarantine_cause.is_none()
                && (!controller_lease_authorizes(
                    &state,
                    request.controller.repository_id,
                    request.controller.run_id,
                    request.controller.actor_id,
                    now_unix_seconds,
                ) || state.harness.is_none()
                    || state.daemon.is_none()))
        {
            return Err(AbortLifecycleError::Binding);
        }
        let mut requested = state.clone();
        requested.revision = requested
            .revision
            .checked_add(1)
            .ok_or(AbortLifecycleError::Binding)?;
        requested.campaign_state = transition(state.campaign_state, Transition::Abort)
            .map_err(|_| AbortLifecycleError::Binding)?;
        append_transition(
            lock,
            request,
            &digest,
            now_unix_seconds,
            LifecycleEvent::AbortRequested,
            &requested,
        )?;
        lock.compare_and_swap(state.revision, &requested)?;
        state = requested;
        report = Some(event_report(lock)?);
    }

    if state.revision == request.expected_state_revision.saturating_add(1)
        && (state.harness.is_some()
            || state.daemon.is_some()
            || state.checkpoint.is_some()
            || quarantine_cause.is_some())
    {
        if state.revision != request.expected_state_revision.saturating_add(1) {
            return Err(AbortLifecycleError::Binding);
        }
        verify_lifecycle_cause(
            report.as_ref().ok_or(AbortLifecycleError::Binding)?,
            request,
            &digest,
            LifecycleEvent::AbortRequested,
        )?;
        if let Some(cause) = quarantine_cause.as_ref() {
            backend
                .capture_quarantine_and_stop(lock.campaign_directory(), request, &state, cause)
                .map_err(AbortLifecycleError::Backend)?;
        } else {
            backend
                .capture_and_stop(lock.campaign_directory(), request, &state)
                .map_err(AbortLifecycleError::Backend)?;
        }
        let mut completed = state.clone();
        completed.revision = completed
            .revision
            .checked_add(1)
            .ok_or(AbortLifecycleError::Binding)?;
        completed.harness = None;
        completed.daemon = None;
        completed.checkpoint = None;
        completed.controller_lease = None;
        append_transition(
            lock,
            request,
            &digest,
            now_unix_seconds,
            LifecycleEvent::AbortCompleted,
            &completed,
        )?;
        lock.compare_and_swap(state.revision, &completed)?;
        state = completed;
        report = Some(event_report(lock)?);
    }

    if state.campaign_state != CampaignState::AbortedIncomplete
        || state.revision != request.expected_state_revision.saturating_add(2)
        || state.harness.is_some()
        || state.daemon.is_some()
        || state.checkpoint.is_some()
        || state.controller_lease.is_some()
    {
        return Err(AbortLifecycleError::Binding);
    }
    verify_lifecycle_cause(
        report.as_ref().ok_or(AbortLifecycleError::Binding)?,
        request,
        &digest,
        LifecycleEvent::AbortCompleted,
    )?;
    finish_response(host_claim, lock, request, now_unix_seconds, state)
}

fn validate_common(
    host_claim: &HostExecutionClaim,
    lock: &CampaignLock,
    request: &Request,
    state: &DurableCampaignState,
    report: Option<&EventVerificationReport>,
    request_digest: &str,
) -> Result<Option<QuarantineRecoveryCause>, AbortLifecycleError> {
    if request.operation != Operation::Abort
        || request
            .abort_reason
            .as_deref()
            .is_none_or(|reason| !matches!(reason, "operator-request" | "guard-failure"))
        || request
            .approval_nonce_sha256
            .as_deref()
            .is_none_or(|value| {
                value.len() != 64
                    || !value
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
        || request.campaign_id != state.identity.campaign_id
        || request.manifest_sha256 != state.identity.manifest_sha256
        || host_claim.campaign_id() != request.campaign_id
        || lock.campaign_directory().parent() != Some(host_claim.campaign_root())
        || state.campaign_state == CampaignState::CompleteSealed
    {
        return Err(AbortLifecycleError::Binding);
    }
    if state.campaign_state == CampaignState::CorruptQuarantined
        || (state.campaign_state == CampaignState::AbortedIncomplete
            && state.recorded_failure
            && state.durable_history_corrupt)
    {
        return validate_quarantine_recovery(
            lock,
            request,
            state,
            report.ok_or(AbortLifecycleError::Binding)?,
            request_digest,
        )
        .map(Some);
    }
    if state.recorded_failure
        || state.duplicate_executor
        || state.durable_history_corrupt
        || state.campaign_state.is_corrupt()
    {
        return Err(AbortLifecycleError::Binding);
    }
    Ok(None)
}

fn validate_quarantine_recovery(
    lock: &CampaignLock,
    request: &Request,
    state: &DurableCampaignState,
    report: &EventVerificationReport,
    request_digest: &str,
) -> Result<QuarantineRecoveryCause, AbortLifecycleError> {
    if !state.recorded_failure
        || !state.durable_history_corrupt
        || state.duplicate_executor
        || state.harness.is_some()
        || state.daemon.is_some()
        || state.checkpoint.is_some()
        || state.controller_lease.is_some()
    {
        return Err(AbortLifecycleError::Binding);
    }
    let spawn = report
        .lifecycle_events
        .iter()
        .rev()
        .find(|entry| {
            matches!(
                entry.transition,
                LifecycleEvent::I74SpawnMismatch | LifecycleEvent::C74SpawnMismatch
            )
        })
        .ok_or(AbortLifecycleError::Binding)?;
    let role = match spawn.transition {
        LifecycleEvent::I74SpawnMismatch => Role::I74,
        LifecycleEvent::C74SpawnMismatch => Role::C74,
        _ => return Err(AbortLifecycleError::Binding),
    };
    let (intent, result) = read_spawn_evidence(lock.campaign_directory(), &role)
        .map_err(|_| AbortLifecycleError::Binding)?;
    if intent.campaign_id != request.campaign_id
        || intent.manifest_sha256 != request.manifest_sha256
        || intent.request_id != spawn.cause_request_id
        || intent.request_sha256 != spawn.cause_request_sha256
        || result.resolution != SpawnResolution::Mismatch
        || result.mismatch != Some(SpawnMismatch::Identity)
        || result.harness.is_some()
        || result.daemon.is_some()
    {
        return Err(AbortLifecycleError::Binding);
    }
    let latest_valid = match state.campaign_state {
        CampaignState::CorruptQuarantined => report.latest_lifecycle.as_ref() == Some(spawn),
        CampaignState::AbortedIncomplete => report.latest_lifecycle.as_ref().is_some_and(|entry| {
            let expected_transition =
                if state.revision == request.expected_state_revision.saturating_add(1) {
                    LifecycleEvent::AbortRequested
                } else if state.revision == request.expected_state_revision.saturating_add(2) {
                    LifecycleEvent::AbortCompleted
                } else {
                    return false;
                };
            entry.transition == expected_transition
                && entry.cause_request_id == request.request_id
                && entry.cause_request_sha256 == request_digest
        }),
        _ => false,
    };
    if !latest_valid {
        return Err(AbortLifecycleError::Binding);
    }
    Ok(QuarantineRecoveryCause {
        role,
        intent,
        result,
    })
}

fn event_report(lock: &CampaignLock) -> Result<EventVerificationReport, AbortLifecycleError> {
    Ok(verify_event_journal(
        &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &lock.campaign_directory().join(EVENT_HEAD_NAME),
    )?)
}

fn verify_lifecycle_cause(
    report: &EventVerificationReport,
    request: &Request,
    digest: &str,
    transition: LifecycleEvent,
) -> Result<(), AbortLifecycleError> {
    if !report.latest_lifecycle.as_ref().is_some_and(|latest| {
        latest.cause_request_id == request.request_id
            && latest.cause_request_sha256 == digest
            && latest.transition == transition
    }) {
        return Err(AbortLifecycleError::Binding);
    }
    Ok(())
}

fn append_transition(
    lock: &CampaignLock,
    request: &Request,
    digest: &str,
    now_unix_seconds: u64,
    transition: LifecycleEvent,
    state: &DurableCampaignState,
) -> Result<(), AbortLifecycleError> {
    append_lifecycle_event(
        &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &lock.campaign_directory().join(EVENT_HEAD_NAME),
        now_unix_seconds,
        request.request_id.clone(),
        digest.to_owned(),
        transition,
        state.clone(),
    )?;
    Ok(())
}

fn finish_response(
    host_claim: &HostExecutionClaim,
    lock: &CampaignLock,
    request: &Request,
    now_unix_seconds: u64,
    state: DurableCampaignState,
) -> Result<Response, AbortLifecycleError> {
    let response = sign_response(ResponseBody {
        schema_version: 1,
        request_id: request.request_id.clone(),
        campaign_id: request.campaign_id.clone(),
        ok: true,
        state_revision: state.revision,
        server_time_unix_seconds: now_unix_seconds,
        result: Some(serde_json::to_value(&state)?),
        error_code: None,
    })?;
    let recorded = match append_or_replay(
        &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &lock.campaign_directory().join(EVENT_HEAD_NAME),
        now_unix_seconds,
        request.clone(),
        EventOutcome::Accepted,
        response.clone(),
        None,
    )? {
        EventAppend::Appended(_) => response,
        EventAppend::Replayed(recorded) => recorded,
    };
    release_claim(host_claim, &state)?;
    Ok(recorded)
}

fn response_state(response: &Response) -> Result<DurableCampaignState, AbortLifecycleError> {
    response
        .body
        .result
        .clone()
        .ok_or(AbortLifecycleError::Binding)
        .and_then(|value| serde_json::from_value(value).map_err(AbortLifecycleError::from))
}

fn release_claim(
    host_claim: &HostExecutionClaim,
    state: &DurableCampaignState,
) -> Result<(), AbortLifecycleError> {
    host_claim.release_after_terminal(state)?;
    Ok(())
}
