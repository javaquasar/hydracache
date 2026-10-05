use crate::event::{
    append_lifecycle_event, verify_event_journal, EventError, EventVerificationReport,
    LifecycleEvent, EVENT_HEAD_NAME, EVENT_JOURNAL_NAME,
};
use crate::host_execution::{HostExecutionClaim, HostExecutionError};
use crate::mutation::{reconcile_campaign, MutationError};
use crate::state::{transition, CampaignState, CheckpointHead, DurableCampaignState, Transition};
use crate::state_store::{CampaignLock, StateStoreError};
use crate::{canonical_json, is_hash, sha256_hex};
use serde::Serialize;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressLossCause {
    pub schema_version: u32,
    pub campaign_id: String,
    pub checkpoint: CheckpointHead,
    pub rejection_gap_seconds: u64,
    pub rejection_deadline_unix_seconds: u64,
}

impl ProgressLossCause {
    fn from_state(
        state: &DurableCampaignState,
        observed_checkpoint: CheckpointHead,
        rejection_gap_seconds: u64,
    ) -> Result<Self, ProgressLossError> {
        if !matches!(
            state.campaign_state,
            CampaignState::I74Running | CampaignState::C74Running | CampaignState::FailedIncomplete
        ) {
            return Err(ProgressLossError::Binding);
        }
        if state.campaign_state == CampaignState::FailedIncomplete
            && state.checkpoint.as_ref() != Some(&observed_checkpoint)
        {
            return Err(ProgressLossError::Binding);
        }
        let checkpoint = observed_checkpoint;
        let rejection_deadline_unix_seconds = checkpoint
            .useful_progress_unix_seconds
            .checked_add(rejection_gap_seconds)
            .ok_or(ProgressLossError::Binding)?;
        Ok(Self {
            schema_version: 1,
            campaign_id: state.identity.campaign_id.clone(),
            checkpoint,
            rejection_gap_seconds,
            rejection_deadline_unix_seconds,
        })
    }

    pub fn event_id(&self) -> String {
        format!("progress-loss:{}", self.checkpoint.record_sha256)
    }

    pub fn sha256(&self) -> Result<String, serde_json::Error> {
        Ok(sha256_hex(&canonical_json(self)?))
    }
}

pub trait ProgressLossBackend {
    /// Capture bounded diagnostics and stop the exact retained unit.
    /// Implementations must recover an identical cause without starting a process.
    fn capture_and_stop_stalled(
        &mut self,
        campaign_directory: &Path,
        cause: &ProgressLossCause,
        state: &DurableCampaignState,
    ) -> Result<(), String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressLossOutcome {
    NotDue,
    Completed { state_revision: u64 },
}

#[derive(Debug, Error)]
pub enum ProgressLossError {
    #[error("progress-loss host claim, identity, deadline, or phase is invalid")]
    Binding,
    #[error("progress-loss event failed: {0}")]
    Event(#[from] EventError),
    #[error("progress-loss state failed: {0}")]
    State(#[from] StateStoreError),
    #[error("progress-loss reconciliation failed: {0}")]
    Mutation(#[from] MutationError),
    #[error("progress-loss host claim failed: {0}")]
    Host(#[from] HostExecutionError),
    #[error("progress-loss backend failed: {0}")]
    Backend(String),
    #[error("progress-loss cause serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn drive_progress_loss<B: ProgressLossBackend + ?Sized>(
    host_claim: &HostExecutionClaim,
    lock: &CampaignLock,
    now_unix_seconds: u64,
    observed_checkpoint: Option<CheckpointHead>,
    rejection_gap_seconds: u64,
    backend: &mut B,
) -> Result<ProgressLossOutcome, ProgressLossError> {
    if rejection_gap_seconds == 0 {
        return Err(ProgressLossError::Binding);
    }
    let mut state = reconcile_campaign(lock)?;
    validate_common(host_claim, lock, &state)?;
    if state.campaign_state == CampaignState::FailedIncomplete
        && state.harness.is_none()
        && state.daemon.is_none()
        && state.checkpoint.is_none()
        && state.controller_lease.is_none()
    {
        let report = event_report(lock)?;
        let latest = report
            .latest_lifecycle
            .as_ref()
            .ok_or(ProgressLossError::Binding)?;
        let cause_record = latest
            .cause_request_id
            .strip_prefix("progress-loss:")
            .ok_or(ProgressLossError::Binding)?;
        if !is_hash(cause_record)
            || latest.transition != LifecycleEvent::ProgressLossCompleted
            || !is_hash(&latest.cause_request_sha256)
        {
            return Err(ProgressLossError::Binding);
        }
        host_claim.release_after_terminal(&state)?;
        return Ok(ProgressLossOutcome::Completed {
            state_revision: state.revision,
        });
    }
    let cause = ProgressLossCause::from_state(
        &state,
        observed_checkpoint.ok_or(ProgressLossError::Binding)?,
        rejection_gap_seconds,
    )?;
    if now_unix_seconds <= cause.rejection_deadline_unix_seconds {
        return Ok(ProgressLossOutcome::NotDue);
    }
    let event_id = cause.event_id();
    let digest = cause.sha256()?;
    let mut report = event_report(lock).ok();

    if state.campaign_state.is_live() {
        if state.harness.is_none() || state.daemon.is_none() {
            return Err(ProgressLossError::Binding);
        }
        let mut requested = state.clone();
        requested.revision = requested
            .revision
            .checked_add(1)
            .ok_or(ProgressLossError::Binding)?;
        requested.campaign_state = transition(state.campaign_state, Transition::Fail)
            .map_err(|_| ProgressLossError::Binding)?;
        requested.checkpoint = Some(cause.checkpoint.clone());
        append_transition(
            lock,
            &event_id,
            &digest,
            now_unix_seconds,
            LifecycleEvent::ProgressLossRequested,
            &requested,
        )?;
        lock.compare_and_swap(state.revision, &requested)?;
        state = requested;
        report = Some(event_report(lock)?);
    }

    if state.campaign_state != CampaignState::FailedIncomplete {
        return Err(ProgressLossError::Binding);
    }
    if state.harness.is_some() || state.daemon.is_some() || state.checkpoint.is_some() {
        if state.harness.is_none() || state.daemon.is_none() || state.checkpoint.is_none() {
            return Err(ProgressLossError::Binding);
        }
        verify_lifecycle_cause(
            report.as_ref().ok_or(ProgressLossError::Binding)?,
            &event_id,
            &digest,
            LifecycleEvent::ProgressLossRequested,
        )?;
        backend
            .capture_and_stop_stalled(lock.campaign_directory(), &cause, &state)
            .map_err(ProgressLossError::Backend)?;
        let mut completed = state.clone();
        completed.revision = completed
            .revision
            .checked_add(1)
            .ok_or(ProgressLossError::Binding)?;
        completed.harness = None;
        completed.daemon = None;
        completed.checkpoint = None;
        completed.controller_lease = None;
        append_transition(
            lock,
            &event_id,
            &digest,
            now_unix_seconds,
            LifecycleEvent::ProgressLossCompleted,
            &completed,
        )?;
        lock.compare_and_swap(state.revision, &completed)?;
        state = completed;
        report = Some(event_report(lock)?);
    }

    if state.harness.is_some()
        || state.daemon.is_some()
        || state.checkpoint.is_some()
        || state.controller_lease.is_some()
    {
        return Err(ProgressLossError::Binding);
    }
    verify_lifecycle_cause(
        report.as_ref().ok_or(ProgressLossError::Binding)?,
        &event_id,
        &digest,
        LifecycleEvent::ProgressLossCompleted,
    )?;
    host_claim.release_after_terminal(&state)?;
    Ok(ProgressLossOutcome::Completed {
        state_revision: state.revision,
    })
}

fn validate_common(
    host_claim: &HostExecutionClaim,
    lock: &CampaignLock,
    state: &DurableCampaignState,
) -> Result<(), ProgressLossError> {
    if state.identity.campaign_id != host_claim.campaign_id()
        || lock.campaign_directory().parent() != Some(host_claim.campaign_root())
        || state.recorded_failure
        || state.duplicate_executor
        || state.durable_history_corrupt
        || state.campaign_state.is_corrupt()
    {
        return Err(ProgressLossError::Binding);
    }
    Ok(())
}

fn event_report(lock: &CampaignLock) -> Result<EventVerificationReport, ProgressLossError> {
    Ok(verify_event_journal(
        &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &lock.campaign_directory().join(EVENT_HEAD_NAME),
    )?)
}

fn verify_lifecycle_cause(
    report: &EventVerificationReport,
    event_id: &str,
    digest: &str,
    transition: LifecycleEvent,
) -> Result<(), ProgressLossError> {
    if !report.latest_lifecycle.as_ref().is_some_and(|latest| {
        latest.cause_request_id == event_id
            && latest.cause_request_sha256 == digest
            && latest.transition == transition
    }) {
        return Err(ProgressLossError::Binding);
    }
    Ok(())
}

fn append_transition(
    lock: &CampaignLock,
    event_id: &str,
    digest: &str,
    now_unix_seconds: u64,
    transition: LifecycleEvent,
    state: &DurableCampaignState,
) -> Result<(), ProgressLossError> {
    append_lifecycle_event(
        &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &lock.campaign_directory().join(EVENT_HEAD_NAME),
        now_unix_seconds,
        event_id.to_owned(),
        digest.to_owned(),
        transition,
        state.clone(),
    )?;
    Ok(())
}
