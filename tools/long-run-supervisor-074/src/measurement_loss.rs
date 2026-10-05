use crate::event::{
    append_lifecycle_event, verify_event_journal, EventError, EventVerificationReport,
    LifecycleEvent, EVENT_HEAD_NAME, EVENT_JOURNAL_NAME,
};
use crate::host_execution::{HostExecutionClaim, HostExecutionError};
use crate::mutation::{reconcile_campaign, MutationError};
use crate::state::{transition, CampaignState, CheckpointHead, DurableCampaignState, Transition};
use crate::state_store::{CampaignLock, StateStoreError};
use crate::{canonical_json, is_hash, sha256_hex, ProcessIdentity};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MeasurementLossReason {
    HostIdentityDrift,
    UnitAbsent,
    UnitIdentityDrift,
    ProcessIdentityDrift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementObservation {
    Healthy,
    Terminal,
    Lost(MeasurementLossReason),
}

impl MeasurementLossReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::HostIdentityDrift => "host-identity-drift",
            Self::UnitAbsent => "unit-absent",
            Self::UnitIdentityDrift => "unit-identity-drift",
            Self::ProcessIdentityDrift => "process-identity-drift",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "host-identity-drift" => Some(Self::HostIdentityDrift),
            "unit-absent" => Some(Self::UnitAbsent),
            "unit-identity-drift" => Some(Self::UnitIdentityDrift),
            "process-identity-drift" => Some(Self::ProcessIdentityDrift),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeasurementLossCause {
    pub schema_version: u32,
    pub campaign_id: String,
    pub reason: MeasurementLossReason,
    pub observed_unix_seconds: u64,
    pub harness: ProcessIdentity,
    pub daemon: ProcessIdentity,
    pub checkpoint: Option<CheckpointHead>,
}

impl MeasurementLossCause {
    fn from_live_state(
        state: &DurableCampaignState,
        reason: MeasurementLossReason,
        observed_unix_seconds: u64,
    ) -> Result<Self, MeasurementLossError> {
        if !matches!(
            state.campaign_state,
            CampaignState::I74Running | CampaignState::C74Running
        ) || state.recorded_failure
            || observed_unix_seconds == 0
        {
            return Err(MeasurementLossError::Binding);
        }
        Self::from_parts(state, reason, observed_unix_seconds)
    }

    fn recover(
        state: &DurableCampaignState,
        report: &EventVerificationReport,
    ) -> Result<Self, MeasurementLossError> {
        if state.campaign_state != CampaignState::FailedIncomplete || !state.recorded_failure {
            return Err(MeasurementLossError::Binding);
        }
        let latest = report
            .latest_lifecycle
            .as_ref()
            .filter(|latest| latest.transition == LifecycleEvent::MeasurementLossRequested)
            .ok_or(MeasurementLossError::Binding)?;
        let (reason, observed_unix_seconds) =
            parse_event_id(&latest.cause_request_id).ok_or(MeasurementLossError::Binding)?;
        let cause = Self::from_parts(state, reason, observed_unix_seconds)?;
        if cause.event_id() != latest.cause_request_id
            || cause.sha256()? != latest.cause_request_sha256
        {
            return Err(MeasurementLossError::Binding);
        }
        Ok(cause)
    }

    fn from_parts(
        state: &DurableCampaignState,
        reason: MeasurementLossReason,
        observed_unix_seconds: u64,
    ) -> Result<Self, MeasurementLossError> {
        let harness = state
            .harness
            .as_ref()
            .ok_or(MeasurementLossError::Binding)?;
        let daemon = state.daemon.as_ref().ok_or(MeasurementLossError::Binding)?;
        if observed_unix_seconds == 0 {
            return Err(MeasurementLossError::Binding);
        }
        Ok(Self {
            schema_version: 1,
            campaign_id: state.identity.campaign_id.clone(),
            reason,
            observed_unix_seconds,
            harness: harness.clone(),
            daemon: daemon.clone(),
            checkpoint: state.checkpoint.clone(),
        })
    }

    pub fn event_id(&self) -> String {
        format!(
            "measurement-loss:{}:{}",
            self.reason.as_str(),
            self.observed_unix_seconds
        )
    }

    pub fn sha256(&self) -> Result<String, serde_json::Error> {
        Ok(sha256_hex(&canonical_json(self)?))
    }
}

pub trait MeasurementLossBackend {
    /// Capture bounded diagnostics and stop only the retained campaign unit when it is still safe.
    /// Implementations must recover an identical cause without starting a process.
    fn capture_and_stop_lost(
        &mut self,
        campaign_directory: &Path,
        cause: &MeasurementLossCause,
        state: &DurableCampaignState,
    ) -> Result<(), String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementLossOutcome {
    Completed { state_revision: u64 },
}

#[derive(Debug, Error)]
pub enum MeasurementLossError {
    #[error("measurement-loss host claim, identity, cause, or phase is invalid")]
    Binding,
    #[error("measurement-loss event failed: {0}")]
    Event(#[from] EventError),
    #[error("measurement-loss state failed: {0}")]
    State(#[from] StateStoreError),
    #[error("measurement-loss reconciliation failed: {0}")]
    Mutation(#[from] MutationError),
    #[error("measurement-loss host claim failed: {0}")]
    Host(#[from] HostExecutionError),
    #[error("measurement-loss backend failed: {0}")]
    Backend(String),
    #[error("measurement-loss cause serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn drive_measurement_loss<B: MeasurementLossBackend + ?Sized>(
    host_claim: &HostExecutionClaim,
    lock: &CampaignLock,
    now_unix_seconds: u64,
    observed_reason: Option<MeasurementLossReason>,
    observed_unix_seconds: Option<u64>,
    backend: &mut B,
) -> Result<MeasurementLossOutcome, MeasurementLossError> {
    let mut state = reconcile_campaign(lock)?;
    validate_common(host_claim, lock, &state)?;
    if state.campaign_state == CampaignState::FailedIncomplete
        && state.recorded_failure
        && state.harness.is_none()
        && state.daemon.is_none()
        && state.checkpoint.is_none()
        && state.controller_lease.is_none()
    {
        let report = event_report(lock)?;
        let latest = report
            .latest_lifecycle
            .as_ref()
            .ok_or(MeasurementLossError::Binding)?;
        if parse_event_id(&latest.cause_request_id).is_none()
            || latest.transition != LifecycleEvent::MeasurementLossCompleted
            || !is_hash(&latest.cause_request_sha256)
        {
            return Err(MeasurementLossError::Binding);
        }
        host_claim.release_after_terminal(&state)?;
        return Ok(MeasurementLossOutcome::Completed {
            state_revision: state.revision,
        });
    }

    let mut report = event_report(lock).ok();
    let cause = if state.campaign_state == CampaignState::FailedIncomplete {
        MeasurementLossCause::recover(
            &state,
            report.as_ref().ok_or(MeasurementLossError::Binding)?,
        )?
    } else {
        let observed_unix_seconds = observed_unix_seconds.ok_or(MeasurementLossError::Binding)?;
        if observed_unix_seconds > now_unix_seconds {
            return Err(MeasurementLossError::Binding);
        }
        MeasurementLossCause::from_live_state(
            &state,
            observed_reason.ok_or(MeasurementLossError::Binding)?,
            observed_unix_seconds,
        )?
    };
    let event_id = cause.event_id();
    let digest = cause.sha256()?;

    if matches!(
        state.campaign_state,
        CampaignState::I74Running | CampaignState::C74Running
    ) {
        let mut requested = state.clone();
        requested.revision = requested
            .revision
            .checked_add(1)
            .ok_or(MeasurementLossError::Binding)?;
        requested.campaign_state = transition(state.campaign_state, Transition::Fail)
            .map_err(|_| MeasurementLossError::Binding)?;
        requested.recorded_failure = true;
        append_transition(
            lock,
            &event_id,
            &digest,
            now_unix_seconds,
            LifecycleEvent::MeasurementLossRequested,
            &requested,
        )?;
        lock.compare_and_swap(state.revision, &requested)?;
        state = requested;
        report = Some(event_report(lock)?);
    }

    if state.campaign_state != CampaignState::FailedIncomplete || !state.recorded_failure {
        return Err(MeasurementLossError::Binding);
    }
    if state.harness.is_some() || state.daemon.is_some() {
        if state.harness.is_none() || state.daemon.is_none() {
            return Err(MeasurementLossError::Binding);
        }
        verify_lifecycle_cause(
            report.as_ref().ok_or(MeasurementLossError::Binding)?,
            &event_id,
            &digest,
            LifecycleEvent::MeasurementLossRequested,
        )?;
        backend
            .capture_and_stop_lost(lock.campaign_directory(), &cause, &state)
            .map_err(MeasurementLossError::Backend)?;
        let mut completed = state.clone();
        completed.revision = completed
            .revision
            .checked_add(1)
            .ok_or(MeasurementLossError::Binding)?;
        completed.harness = None;
        completed.daemon = None;
        completed.checkpoint = None;
        completed.controller_lease = None;
        append_transition(
            lock,
            &event_id,
            &digest,
            now_unix_seconds,
            LifecycleEvent::MeasurementLossCompleted,
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
        || !state.recorded_failure
    {
        return Err(MeasurementLossError::Binding);
    }
    verify_lifecycle_cause(
        report.as_ref().ok_or(MeasurementLossError::Binding)?,
        &event_id,
        &digest,
        LifecycleEvent::MeasurementLossCompleted,
    )?;
    host_claim.release_after_terminal(&state)?;
    Ok(MeasurementLossOutcome::Completed {
        state_revision: state.revision,
    })
}

fn validate_common(
    host_claim: &HostExecutionClaim,
    lock: &CampaignLock,
    state: &DurableCampaignState,
) -> Result<(), MeasurementLossError> {
    if state.identity.campaign_id != host_claim.campaign_id()
        || lock.campaign_directory().parent() != Some(host_claim.campaign_root())
        || state.duplicate_executor
        || state.durable_history_corrupt
        || state.campaign_state.is_corrupt()
        || (state.recorded_failure && state.campaign_state != CampaignState::FailedIncomplete)
        || (!state.recorded_failure && state.campaign_state == CampaignState::FailedIncomplete)
    {
        return Err(MeasurementLossError::Binding);
    }
    Ok(())
}

fn parse_event_id(value: &str) -> Option<(MeasurementLossReason, u64)> {
    let suffix = value.strip_prefix("measurement-loss:")?;
    let (reason, timestamp) = suffix.rsplit_once(':')?;
    let timestamp = timestamp
        .parse::<u64>()
        .ok()
        .filter(|timestamp| *timestamp != 0)?;
    Some((MeasurementLossReason::parse(reason)?, timestamp))
}

fn event_report(lock: &CampaignLock) -> Result<EventVerificationReport, MeasurementLossError> {
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
) -> Result<(), MeasurementLossError> {
    if !report.latest_lifecycle.as_ref().is_some_and(|latest| {
        latest.cause_request_id == event_id
            && latest.cause_request_sha256 == digest
            && latest.transition == transition
    }) {
        return Err(MeasurementLossError::Binding);
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
) -> Result<(), MeasurementLossError> {
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
