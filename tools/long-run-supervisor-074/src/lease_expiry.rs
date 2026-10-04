use crate::event::{
    append_lifecycle_event, verify_event_journal, EventError, EventVerificationReport,
    LifecycleEvent, EVENT_HEAD_NAME, EVENT_JOURNAL_NAME,
};
use crate::host_execution::{HostExecutionClaim, HostExecutionError};
use crate::mutation::{reconcile_campaign, MutationError};
use crate::state::{transition, CampaignState, DurableCampaignState, Transition};
use crate::state_store::{CampaignLock, StateStoreError};
use crate::{canonical_json, sha256_hex};
use serde::Serialize;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LeaseExpiryCause {
    pub schema_version: u32,
    pub campaign_id: String,
    pub lease_id: String,
    pub lease_deadline_unix_seconds: u64,
}

impl LeaseExpiryCause {
    fn from_state(state: &DurableCampaignState) -> Self {
        Self {
            schema_version: 1,
            campaign_id: state.identity.campaign_id.clone(),
            lease_id: state.identity.lease_id.clone(),
            lease_deadline_unix_seconds: state.identity.lease_deadline_unix_seconds,
        }
    }

    pub fn event_id(&self) -> String {
        format!("lease-expiry:{}", self.lease_id)
    }

    pub fn sha256(&self) -> Result<String, serde_json::Error> {
        Ok(sha256_hex(&canonical_json(self)?))
    }
}

pub trait LeaseExpiryBackend {
    /// Capture bounded diagnostics and stop the exact retained unit.
    /// Implementations must recover an identical cause without starting a process.
    fn capture_and_stop_expired(
        &mut self,
        campaign_directory: &Path,
        cause: &LeaseExpiryCause,
        state: &DurableCampaignState,
    ) -> Result<(), String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseExpiryOutcome {
    NotDue,
    Completed { state_revision: u64 },
}

#[derive(Debug, Error)]
pub enum LeaseExpiryError {
    #[error("lease-expiry host claim, identity, deadline, or phase is invalid")]
    Binding,
    #[error("lease-expiry event failed: {0}")]
    Event(#[from] EventError),
    #[error("lease-expiry state failed: {0}")]
    State(#[from] StateStoreError),
    #[error("lease-expiry reconciliation failed: {0}")]
    Mutation(#[from] MutationError),
    #[error("lease-expiry host claim failed: {0}")]
    Host(#[from] HostExecutionError),
    #[error("lease-expiry backend failed: {0}")]
    Backend(String),
    #[error("lease-expiry cause serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn drive_lease_expiry<B: LeaseExpiryBackend + ?Sized>(
    host_claim: &HostExecutionClaim,
    lock: &CampaignLock,
    now_unix_seconds: u64,
    backend: &mut B,
) -> Result<LeaseExpiryOutcome, LeaseExpiryError> {
    let mut state = reconcile_campaign(lock)?;
    validate_common(host_claim, lock, &state)?;
    if now_unix_seconds <= state.identity.lease_deadline_unix_seconds {
        return Ok(LeaseExpiryOutcome::NotDue);
    }
    let cause = LeaseExpiryCause::from_state(&state);
    let event_id = cause.event_id();
    let digest = cause.sha256()?;
    let mut report = event_report(lock).ok();

    if state.campaign_state.is_live() {
        if state.harness.is_none() || state.daemon.is_none() {
            return Err(LeaseExpiryError::Binding);
        }
        let mut requested = state.clone();
        requested.revision = requested
            .revision
            .checked_add(1)
            .ok_or(LeaseExpiryError::Binding)?;
        requested.campaign_state = transition(state.campaign_state, Transition::ExpireLease)
            .map_err(|_| LeaseExpiryError::Binding)?;
        append_transition(
            lock,
            &event_id,
            &digest,
            now_unix_seconds,
            LifecycleEvent::LeaseExpiryRequested,
            &requested,
        )?;
        lock.compare_and_swap(state.revision, &requested)?;
        state = requested;
        report = Some(event_report(lock)?);
    }

    if state.campaign_state != CampaignState::LeaseExpiredIncomplete {
        return Err(LeaseExpiryError::Binding);
    }
    if state.harness.is_some() || state.daemon.is_some() || state.checkpoint.is_some() {
        if state.harness.is_none() || state.daemon.is_none() {
            return Err(LeaseExpiryError::Binding);
        }
        verify_lifecycle_cause(
            report.as_ref().ok_or(LeaseExpiryError::Binding)?,
            &event_id,
            &digest,
            LifecycleEvent::LeaseExpiryRequested,
        )?;
        backend
            .capture_and_stop_expired(lock.campaign_directory(), &cause, &state)
            .map_err(LeaseExpiryError::Backend)?;
        let mut completed = state.clone();
        completed.revision = completed
            .revision
            .checked_add(1)
            .ok_or(LeaseExpiryError::Binding)?;
        completed.harness = None;
        completed.daemon = None;
        completed.checkpoint = None;
        completed.controller_lease = None;
        append_transition(
            lock,
            &event_id,
            &digest,
            now_unix_seconds,
            LifecycleEvent::LeaseExpiryCompleted,
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
        return Err(LeaseExpiryError::Binding);
    }
    verify_lifecycle_cause(
        report.as_ref().ok_or(LeaseExpiryError::Binding)?,
        &event_id,
        &digest,
        LifecycleEvent::LeaseExpiryCompleted,
    )?;
    host_claim.release_after_terminal(&state)?;
    Ok(LeaseExpiryOutcome::Completed {
        state_revision: state.revision,
    })
}

fn validate_common(
    host_claim: &HostExecutionClaim,
    lock: &CampaignLock,
    state: &DurableCampaignState,
) -> Result<(), LeaseExpiryError> {
    if state.identity.campaign_id != host_claim.campaign_id()
        || lock.campaign_directory().parent() != Some(host_claim.campaign_root())
        || state.recorded_failure
        || state.duplicate_executor
        || state.durable_history_corrupt
        || state.campaign_state.is_corrupt()
    {
        return Err(LeaseExpiryError::Binding);
    }
    Ok(())
}

fn event_report(lock: &CampaignLock) -> Result<EventVerificationReport, LeaseExpiryError> {
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
) -> Result<(), LeaseExpiryError> {
    if !report.latest_lifecycle.as_ref().is_some_and(|latest| {
        latest.cause_request_id == event_id
            && latest.cause_request_sha256 == digest
            && latest.transition == transition
    }) {
        return Err(LeaseExpiryError::Binding);
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
) -> Result<(), LeaseExpiryError> {
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
