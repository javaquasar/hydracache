use crate::archive::ArchiveLimits;
use crate::artifact::{PacketLimits, PacketPlan};
use crate::checkpoint_evidence::{verify_terminal_checkpoint_evidence, CheckpointEvidenceError};
use crate::event::{
    append_lifecycle_event, append_or_replay, request_sha256, verify_event_journal, EventAppend,
    EventError, EventOutcome, EventVerificationReport, LifecycleEvent, EVENT_HEAD_NAME,
    EVENT_JOURNAL_NAME,
};
use crate::host_execution::{HostExecutionClaim, HostExecutionError};
use crate::mutation::{reconcile_campaign, MutationError};
use crate::protocol::{sign_response, Operation, Request, Response, ResponseBody};
use crate::seal_artifact::{build_or_recover_seal_artifact, SealArtifactError, SealResponseResult};
use crate::state::{
    controller_lease_authorizes, transition, CampaignState, DurableCampaignState, Transition,
};
use crate::state_store::{CampaignLock, StateStoreError};
use crate::systemd_unit::{verify_unit_terminal, UnitError, UnitSnapshot};
use crate::Role;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SealLifecycleError {
    #[error("seal lifecycle request, host claim, identity, lease, or phase is invalid")]
    Binding,
    #[error("seal lifecycle event failed: {0}")]
    Event(#[from] EventError),
    #[error("seal lifecycle state failed: {0}")]
    State(#[from] StateStoreError),
    #[error("seal lifecycle reconciliation failed: {0}")]
    Mutation(#[from] MutationError),
    #[error("seal lifecycle terminal checkpoint failed: {0}")]
    Checkpoint(#[from] CheckpointEvidenceError),
    #[error("seal lifecycle terminal unit failed: {0}")]
    Unit(#[from] UnitError),
    #[error("seal lifecycle artifact failed: {0}")]
    Artifact(#[from] SealArtifactError),
    #[error("seal lifecycle host claim failed: {0}")]
    Host(#[from] HostExecutionError),
    #[error("seal lifecycle response serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[allow(clippy::too_many_arguments)]
pub fn drive_seal_request(
    host_claim: &HostExecutionClaim,
    lock: &CampaignLock,
    request: &Request,
    now_unix_seconds: u64,
    terminal_unit: &UnitSnapshot,
    plan: &PacketPlan,
    seal_root: &Path,
    packet_limits: PacketLimits,
    archive_limits: ArchiveLimits,
) -> Result<Response, SealLifecycleError> {
    let digest = request_sha256(request)?;
    let mut state = reconcile_campaign(lock)?;
    let mut report = event_report(lock)?;
    if let Some(recorded) = report.replay_index.get(&request.request_id) {
        if recorded.request_sha256 != digest {
            return Err(EventError::ReplayConflict {
                request_id: request.request_id.clone(),
            }
            .into());
        }
        let result = response_result(&recorded.response)?;
        if result.state != state {
            return Err(SealLifecycleError::Binding);
        }
        build_or_recover_seal_artifact(
            lock.campaign_directory(),
            seal_root,
            &request.request_id,
            &digest,
            result.artifact.role.clone(),
            plan,
            packet_limits,
            archive_limits,
        )?;
        release_complete_claim(host_claim, &state)?;
        return Ok(recorded.response.clone());
    }

    validate_common(host_claim, lock, request, &state, plan)?;
    let role = role_for_state(state.campaign_state).ok_or(SealLifecycleError::Binding)?;
    let (running, terminal, sealed, mark_terminal, seal_transition, terminal_event, sealed_event) =
        match role {
            Role::I74 => (
                CampaignState::I74Running,
                CampaignState::I74Terminal,
                CampaignState::I74Sealed,
                Transition::MarkI74Terminal,
                Transition::SealI74,
                LifecycleEvent::I74Terminal,
                LifecycleEvent::I74Sealed,
            ),
            Role::C74 => (
                CampaignState::C74Running,
                CampaignState::C74Terminal,
                CampaignState::CompleteSealed,
                Transition::MarkC74Terminal,
                Transition::SealComplete,
                LifecycleEvent::C74Terminal,
                LifecycleEvent::CompleteSealed,
            ),
        };

    if state.campaign_state == running {
        if state.revision != request.expected_state_revision
            || !controller_lease_authorizes(
                &state,
                request.controller.repository_id,
                request.controller.run_id,
                request.controller.actor_id,
                now_unix_seconds,
            )
        {
            return Err(SealLifecycleError::Binding);
        }
        verify_terminal_inputs(lock, &state, terminal_unit)?;
        let mut next = state.clone();
        next.revision = next
            .revision
            .checked_add(1)
            .ok_or(SealLifecycleError::Binding)?;
        next.campaign_state = transition(state.campaign_state, mark_terminal)
            .map_err(|_| SealLifecycleError::Binding)?;
        append_transition(
            lock,
            request,
            &digest,
            now_unix_seconds,
            terminal_event,
            &next,
        )?;
        lock.compare_and_swap(state.revision, &next)?;
        state = next;
        report = event_report(lock)?;
    }

    if state.campaign_state == terminal {
        if state.revision != request.expected_state_revision.saturating_add(1) {
            return Err(SealLifecycleError::Binding);
        }
        verify_lifecycle_cause(&report, request, &digest, terminal_event)?;
        verify_terminal_inputs(lock, &state, terminal_unit)?;
        let artifact = build_or_recover_seal_artifact(
            lock.campaign_directory(),
            seal_root,
            &request.request_id,
            &digest,
            role.clone(),
            plan,
            packet_limits,
            archive_limits,
        )?;
        let mut next = state.clone();
        next.revision = next
            .revision
            .checked_add(1)
            .ok_or(SealLifecycleError::Binding)?;
        next.campaign_state = transition(state.campaign_state, seal_transition)
            .map_err(|_| SealLifecycleError::Binding)?;
        next.harness = None;
        next.daemon = None;
        next.checkpoint = None;
        next.controller_lease = None;
        append_transition(
            lock,
            request,
            &digest,
            now_unix_seconds,
            sealed_event,
            &next,
        )?;
        lock.compare_and_swap(state.revision, &next)?;
        state = next;
        return finish_response(
            host_claim,
            lock,
            request,
            now_unix_seconds,
            &digest,
            state,
            artifact,
        );
    }

    if state.campaign_state != sealed
        || state.revision != request.expected_state_revision.saturating_add(2)
    {
        return Err(SealLifecycleError::Binding);
    }
    verify_lifecycle_cause(&report, request, &digest, sealed_event)?;
    let artifact = build_or_recover_seal_artifact(
        lock.campaign_directory(),
        seal_root,
        &request.request_id,
        &digest,
        role,
        plan,
        packet_limits,
        archive_limits,
    )?;
    finish_response(
        host_claim,
        lock,
        request,
        now_unix_seconds,
        &digest,
        state,
        artifact,
    )
}

fn validate_common(
    host_claim: &HostExecutionClaim,
    lock: &CampaignLock,
    request: &Request,
    state: &DurableCampaignState,
    plan: &PacketPlan,
) -> Result<(), SealLifecycleError> {
    if request.operation != Operation::Seal
        || request.campaign_id != state.identity.campaign_id
        || request.manifest_sha256 != state.identity.manifest_sha256
        || request.campaign_id != plan.campaign_id
        || host_claim.campaign_id() != request.campaign_id
        || lock.campaign_directory().parent() != Some(host_claim.campaign_root())
        || state.recorded_failure
        || state.duplicate_executor
        || state.durable_history_corrupt
    {
        return Err(SealLifecycleError::Binding);
    }
    Ok(())
}

fn role_for_state(state: CampaignState) -> Option<Role> {
    match state {
        CampaignState::I74Running | CampaignState::I74Terminal | CampaignState::I74Sealed => {
            Some(Role::I74)
        }
        CampaignState::C74Running | CampaignState::C74Terminal | CampaignState::CompleteSealed => {
            Some(Role::C74)
        }
        _ => None,
    }
}

fn verify_terminal_inputs(
    lock: &CampaignLock,
    state: &DurableCampaignState,
    terminal_unit: &UnitSnapshot,
) -> Result<(), SealLifecycleError> {
    verify_terminal_checkpoint_evidence(lock.campaign_directory(), state)?;
    verify_unit_terminal(
        state.harness.as_ref().ok_or(SealLifecycleError::Binding)?,
        state.daemon.as_ref().ok_or(SealLifecycleError::Binding)?,
        terminal_unit,
    )?;
    Ok(())
}

fn event_report(lock: &CampaignLock) -> Result<EventVerificationReport, SealLifecycleError> {
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
) -> Result<(), SealLifecycleError> {
    if !report.latest_lifecycle.as_ref().is_some_and(|latest| {
        latest.cause_request_id == request.request_id
            && latest.cause_request_sha256 == digest
            && latest.transition == transition
    }) {
        return Err(SealLifecycleError::Binding);
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
) -> Result<(), SealLifecycleError> {
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

#[allow(clippy::too_many_arguments)]
fn finish_response(
    host_claim: &HostExecutionClaim,
    lock: &CampaignLock,
    request: &Request,
    now_unix_seconds: u64,
    _digest: &str,
    state: DurableCampaignState,
    artifact: crate::seal_artifact::SealArtifactResult,
) -> Result<Response, SealLifecycleError> {
    let response = sign_response(ResponseBody {
        schema_version: 1,
        request_id: request.request_id.clone(),
        campaign_id: request.campaign_id.clone(),
        ok: true,
        state_revision: state.revision,
        server_time_unix_seconds: now_unix_seconds,
        result: Some(serde_json::to_value(SealResponseResult {
            state: state.clone(),
            artifact,
        })?),
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
    release_complete_claim(host_claim, &state)?;
    Ok(recorded)
}

fn response_result(response: &Response) -> Result<SealResponseResult, SealLifecycleError> {
    response
        .body
        .result
        .clone()
        .ok_or(SealLifecycleError::Binding)
        .and_then(|value| serde_json::from_value(value).map_err(SealLifecycleError::from))
}

fn release_complete_claim(
    host_claim: &HostExecutionClaim,
    state: &DurableCampaignState,
) -> Result<(), SealLifecycleError> {
    if state.campaign_state == CampaignState::CompleteSealed {
        host_claim.release_after_complete_seal(state)?;
    }
    Ok(())
}
