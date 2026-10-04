use crate::event::{
    append_or_replay, request_sha256, verify_event_journal, EventAppend, EventError, EventOutcome,
    EventVerificationReport, EVENT_HEAD_NAME, EVENT_JOURNAL_NAME,
};
use crate::protocol::{sign_response, Operation, Request, Response, ResponseBody};
use crate::state::DurableCampaignState;
use crate::state_store::{CampaignLock, StateStoreError};
use std::fs;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MutationError {
    #[error("campaign state and authoritative event journal diverged")]
    Diverged,
    #[error("only attach is supported by the current mutation transaction")]
    Operation,
    #[error("durable event failed: {0}")]
    Event(#[from] EventError),
    #[error("durable state failed: {0}")]
    State(#[from] StateStoreError),
    #[error("response serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("event journal metadata failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
pub enum BeginAttach<'a> {
    New(Box<AttachMutation<'a>>),
    Replayed(Response),
}

#[derive(Debug)]
pub struct AttachMutation<'a> {
    lock: &'a CampaignLock,
    request: &'a Request,
    now_unix_seconds: u64,
    state: DurableCampaignState,
}

impl AttachMutation<'_> {
    pub fn state(&self) -> &DurableCampaignState {
        &self.state
    }

    pub fn accept(self, next: DurableCampaignState) -> Result<Response, MutationError> {
        if next.revision != self.state.revision.saturating_add(1)
            || next.identity.campaign_id != self.state.identity.campaign_id
            || next.identity.manifest_sha256 != self.state.identity.manifest_sha256
        {
            return Err(MutationError::Diverged);
        }
        let response = sign_response(ResponseBody {
            schema_version: 1,
            request_id: self.request.request_id.clone(),
            campaign_id: self.request.campaign_id.clone(),
            ok: true,
            state_revision: next.revision,
            server_time_unix_seconds: self.now_unix_seconds,
            result: Some(serde_json::to_value(&next)?),
            error_code: None,
        })?;
        let (journal, head) = event_paths(self.lock);
        match append_or_replay(
            &journal,
            &head,
            self.now_unix_seconds,
            self.request.clone(),
            EventOutcome::Accepted,
            response.clone(),
            Some(next.clone()),
        )? {
            EventAppend::Appended(_) => {
                self.lock.compare_and_swap(self.state.revision, &next)?;
                Ok(response)
            }
            EventAppend::Replayed(recorded) => Ok(recorded),
        }
    }

    pub fn reject(self, error_code: u32) -> Result<Response, MutationError> {
        self.reject_with_result(error_code, None)
    }

    pub fn reject_with_result(
        self,
        error_code: u32,
        result: Option<serde_json::Value>,
    ) -> Result<Response, MutationError> {
        let response = sign_response(ResponseBody {
            schema_version: 1,
            request_id: self.request.request_id.clone(),
            campaign_id: self.request.campaign_id.clone(),
            ok: false,
            state_revision: self.state.revision,
            server_time_unix_seconds: self.now_unix_seconds,
            result,
            error_code: Some(error_code),
        })?;
        let (journal, head) = event_paths(self.lock);
        match append_or_replay(
            &journal,
            &head,
            self.now_unix_seconds,
            self.request.clone(),
            EventOutcome::Rejected,
            response.clone(),
            None,
        )? {
            EventAppend::Appended(_) => Ok(response),
            EventAppend::Replayed(recorded) => Ok(recorded),
        }
    }
}

pub fn begin_attach<'a>(
    lock: &'a CampaignLock,
    request: &'a Request,
    now_unix_seconds: u64,
) -> Result<BeginAttach<'a>, MutationError> {
    if request.operation != Operation::Attach {
        return Err(MutationError::Operation);
    }
    let (state, report) = load_campaign(lock)?;
    if let Some(report) = &report {
        if report.campaign_id != request.campaign_id {
            return Err(MutationError::Diverged);
        }
        if let Some(recorded) = report.replay_index.get(&request.request_id) {
            return if recorded.request_sha256 == request_sha256(request)? {
                Ok(BeginAttach::Replayed(recorded.response.clone()))
            } else {
                Err(EventError::ReplayConflict {
                    request_id: request.request_id.clone(),
                }
                .into())
            };
        }
    }
    Ok(BeginAttach::New(Box::new(AttachMutation {
        lock,
        request,
        now_unix_seconds,
        state,
    })))
}

pub fn reconcile_campaign(lock: &CampaignLock) -> Result<DurableCampaignState, MutationError> {
    let (state, _) = load_campaign(lock)?;
    Ok(state)
}

fn load_campaign(
    lock: &CampaignLock,
) -> Result<(DurableCampaignState, Option<EventVerificationReport>), MutationError> {
    let mut state = lock.read()?;
    let (journal, head) = event_paths(lock);
    let report = match fs::symlink_metadata(&journal) {
        Ok(_) => Some(verify_event_journal(&journal, &head)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    if let Some(report) = &report {
        if report.recovered_incomplete_trailing_bytes != 0 {
            return Err(MutationError::Diverged);
        }
        state = reconcile_snapshot(lock, state, report)?;
    }
    Ok((state, report))
}

fn reconcile_snapshot(
    lock: &CampaignLock,
    current: DurableCampaignState,
    report: &EventVerificationReport,
) -> Result<DurableCampaignState, MutationError> {
    let Some(authoritative) = &report.latest_state_after else {
        return Ok(current);
    };
    if authoritative.revision == current.revision {
        return if authoritative == &current {
            Ok(current)
        } else {
            Err(MutationError::Diverged)
        };
    }
    if authoritative.revision == current.revision.saturating_add(1)
        && authoritative.identity == current.identity
    {
        lock.compare_and_swap(current.revision, authoritative)?;
        return Ok(authoritative.clone());
    }
    Err(MutationError::Diverged)
}

fn event_paths(lock: &CampaignLock) -> (std::path::PathBuf, std::path::PathBuf) {
    (
        lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        lock.campaign_directory().join(EVENT_HEAD_NAME),
    )
}
