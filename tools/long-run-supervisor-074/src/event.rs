use crate::protocol::{verify_response, Operation, Request, Response};
use crate::state::DurableCampaignState;
use crate::{canonical_json, chain_hash, is_hash, sha256_hex, GENESIS_HASH};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const EVENT_JOURNAL_NAME: &str = "events.jsonl";
pub const EVENT_HEAD_NAME: &str = "events.head";
pub const MAX_EVENT_JOURNAL_BYTES: u64 = 64 * 1024 * 1024;
const EVENT_DOMAIN: &[u8] = b"hydracache-long-run-event-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventOutcome {
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LifecycleEvent {
    Prepared,
    I74Starting,
    I74Started,
    I74Adopted,
    I74SpawnAbsent,
    I74SpawnMismatch,
    C74Starting,
    C74Started,
    C74Adopted,
    C74SpawnAbsent,
    C74SpawnMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case", deny_unknown_fields)]
pub enum SupervisorEventKind {
    Request {
        request_sha256: String,
        request: Box<Request>,
        outcome: EventOutcome,
        response: Box<Response>,
    },
    Lifecycle {
        cause_request_id: String,
        cause_request_sha256: String,
        transition: LifecycleEvent,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupervisorEventPayload {
    pub campaign_id: String,
    pub occurred_at_unix_seconds: u64,
    #[serde(flatten)]
    pub event: SupervisorEventKind,
    pub state_after: Option<Box<DurableCampaignState>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventEnvelope {
    pub schema_version: u32,
    pub sequence: u64,
    pub previous_record_sha256: String,
    pub payload: SupervisorEventPayload,
    pub payload_sha256: String,
    pub record_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayEntry {
    pub request_sha256: String,
    pub response: Response,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventVerificationReport {
    pub records: u64,
    pub head_sha256: String,
    pub campaign_id: String,
    pub last_occurred_at_unix_seconds: u64,
    pub recovered_incomplete_trailing_bytes: usize,
    pub replay_index: BTreeMap<String, ReplayEntry>,
    pub latest_state_after: Option<DurableCampaignState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventAppend {
    Appended(Box<EventEnvelope>),
    Replayed(Response),
}

#[derive(Debug, Error)]
pub enum EventError {
    #[error("event journal is empty")]
    Empty,
    #[error("event journal exceeds the 64 MiB local safety limit")]
    Oversized,
    #[error("invalid UTF-8 or JSON at event {record}: {message}")]
    Parse { record: usize, message: String },
    #[error("event journal is not in canonical JSON form at sequence {sequence}")]
    NonCanonical { sequence: u64 },
    #[error("event schema version at sequence {sequence} must be 1")]
    Schema { sequence: u64 },
    #[error("event sequence mismatch: expected {expected}, got {actual}")]
    Sequence { expected: u64, actual: u64 },
    #[error("event previous hash mismatch at sequence {sequence}")]
    PreviousHash { sequence: u64 },
    #[error("event payload hash mismatch at sequence {sequence}")]
    PayloadHash { sequence: u64 },
    #[error("event record hash mismatch at sequence {sequence}")]
    RecordHash { sequence: u64 },
    #[error("invalid lowercase SHA-256 field {field} at sequence {sequence}")]
    InvalidHash { sequence: u64, field: &'static str },
    #[error("event campaign changed at sequence {sequence}")]
    CampaignDrift { sequence: u64 },
    #[error("event timestamp reversed at sequence {sequence}")]
    TimestampReversal { sequence: u64 },
    #[error("event request or response binding is invalid at sequence {sequence}")]
    Binding { sequence: u64 },
    #[error("request id was reused with different request bytes: {request_id}")]
    ReplayConflict { request_id: String },
    #[error("event outcome disagrees with the signed response at sequence {sequence}")]
    Outcome { sequence: u64 },
    #[error("event head file is missing, malformed, or does not match the journal")]
    Head,
    #[error("event journal has {bytes} incomplete trailing bytes; explicit recovery is required")]
    TornTailRequiresRecovery { bytes: usize },
    #[error("event I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("event serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("event hash construction failed: {0}")]
    Chain(#[from] crate::ChainError),
}

pub fn request_sha256(request: &Request) -> Result<String, EventError> {
    Ok(sha256_hex(&canonical_json(request)?))
}

pub fn build_event(
    sequence: u64,
    previous_record_sha256: &str,
    occurred_at_unix_seconds: u64,
    request: Request,
    outcome: EventOutcome,
    response: Response,
    state_after: Option<DurableCampaignState>,
) -> Result<EventEnvelope, EventError> {
    let request_sha256 = request_sha256(&request)?;
    let payload = SupervisorEventPayload {
        campaign_id: request.campaign_id.clone(),
        occurred_at_unix_seconds,
        event: SupervisorEventKind::Request {
            request_sha256,
            request: Box::new(request),
            outcome,
            response: Box::new(response),
        },
        state_after: state_after.map(Box::new),
    };
    let payload_sha256 = sha256_hex(&canonical_json(&payload)?);
    let record_sha256 = event_record_hash(sequence, previous_record_sha256, &payload_sha256)?;
    let event = EventEnvelope {
        schema_version: 1,
        sequence,
        previous_record_sha256: previous_record_sha256.to_owned(),
        payload,
        payload_sha256,
        record_sha256,
    };
    validate_event(&event, sequence, previous_record_sha256)?;
    Ok(event)
}

pub fn append_lifecycle_event(
    journal: &Path,
    head: &Path,
    occurred_at_unix_seconds: u64,
    cause_request_id: String,
    cause_request_sha256: String,
    transition: LifecycleEvent,
    state_after: DurableCampaignState,
) -> Result<Box<EventEnvelope>, EventError> {
    let existing = load_existing(journal, head)?;
    let sequence = existing.as_ref().map_or(1, |report| report.records + 1);
    let previous = existing
        .as_ref()
        .map_or(GENESIS_HASH, |report| report.head_sha256.as_str());
    if existing.as_ref().is_some_and(|report| {
        report.campaign_id != state_after.identity.campaign_id
            || occurred_at_unix_seconds < report.last_occurred_at_unix_seconds
    }) {
        return Err(EventError::Binding { sequence });
    }
    let payload = SupervisorEventPayload {
        campaign_id: state_after.identity.campaign_id.clone(),
        occurred_at_unix_seconds,
        event: SupervisorEventKind::Lifecycle {
            cause_request_id,
            cause_request_sha256,
            transition,
        },
        state_after: Some(Box::new(state_after)),
    };
    let payload_sha256 = sha256_hex(&canonical_json(&payload)?);
    let event = EventEnvelope {
        schema_version: 1,
        sequence,
        previous_record_sha256: previous.to_owned(),
        record_sha256: event_record_hash(sequence, previous, &payload_sha256)?,
        payload,
        payload_sha256,
    };
    validate_event(&event, sequence, previous)?;
    append_event(journal, head, &event, existing.as_ref())?;
    Ok(Box::new(event))
}

pub fn verify_event_journal(
    journal: &Path,
    head: &Path,
) -> Result<EventVerificationReport, EventError> {
    let metadata = fs::symlink_metadata(journal)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_EVENT_JOURNAL_BYTES
    {
        return Err(EventError::Oversized);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(journal)?
        .take(MAX_EVENT_JOURNAL_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_EVENT_JOURNAL_BYTES {
        return Err(EventError::Oversized);
    }
    let report = verify_event_bytes(&bytes)?;
    verify_head(head, &report.head_sha256)?;
    Ok(report)
}

pub fn verify_event_bytes(bytes: &[u8]) -> Result<EventVerificationReport, EventError> {
    if bytes.is_empty() {
        return Err(EventError::Empty);
    }
    if bytes.len() as u64 > MAX_EVENT_JOURNAL_BYTES {
        return Err(EventError::Oversized);
    }
    let has_terminal_newline = bytes.last() == Some(&b'\n');
    let mut parts = bytes.split(|byte| *byte == b'\n').collect::<Vec<_>>();
    if has_terminal_newline {
        parts.pop();
    }
    let mut recovered = 0;
    if !has_terminal_newline {
        let tail = parts.last().copied().unwrap_or_default();
        recovered = tail.len();
        parts.pop();
    }
    if parts.is_empty() {
        return if recovered == 0 {
            Err(EventError::Empty)
        } else {
            Err(EventError::TornTailRequiresRecovery { bytes: recovered })
        };
    }

    let mut previous = GENESIS_HASH.to_owned();
    let mut campaign_id = None;
    let mut last_time = 0;
    let mut last = None;
    let mut replay_index = BTreeMap::new();
    let mut latest_state_after = None;
    for (index, line) in parts.iter().enumerate() {
        if line.is_empty() {
            return Err(EventError::Parse {
                record: index + 1,
                message: "empty line".to_owned(),
            });
        }
        let event: EventEnvelope =
            serde_json::from_slice(line).map_err(|error| EventError::Parse {
                record: index + 1,
                message: error.to_string(),
            })?;
        if canonical_json(&event)? != *line {
            return Err(EventError::NonCanonical {
                sequence: event.sequence,
            });
        }
        validate_event(&event, index as u64 + 1, &previous)?;
        if let Some(campaign) = &campaign_id {
            if &event.payload.campaign_id != campaign {
                return Err(EventError::CampaignDrift {
                    sequence: event.sequence,
                });
            }
            if event.payload.occurred_at_unix_seconds < last_time {
                return Err(EventError::TimestampReversal {
                    sequence: event.sequence,
                });
            }
        } else {
            campaign_id = Some(event.payload.campaign_id.clone());
        }
        if let SupervisorEventKind::Request {
            request_sha256,
            request,
            outcome,
            response,
        } = &event.payload.event
        {
            if request.operation == Operation::Start {
                let recorded: DurableCampaignState = response
                    .body
                    .result
                    .clone()
                    .and_then(|value| serde_json::from_value(value).ok())
                    .ok_or(EventError::Binding {
                        sequence: event.sequence,
                    })?;
                if latest_state_after.as_ref() != Some(&recorded)
                    || recorded.revision != response.body.state_revision
                    || (recorded.campaign_state == crate::state::CampaignState::I74Running)
                        != (*outcome == EventOutcome::Accepted)
                {
                    return Err(EventError::Binding {
                        sequence: event.sequence,
                    });
                }
            }
            if replay_index.contains_key(&request.request_id) {
                return Err(EventError::ReplayConflict {
                    request_id: request.request_id.clone(),
                });
            }
            replay_index.insert(
                request.request_id.clone(),
                ReplayEntry {
                    request_sha256: request_sha256.clone(),
                    response: response.as_ref().clone(),
                },
            );
        }
        if let Some(state) = event.payload.state_after.as_deref() {
            if let Some(previous_state) = &latest_state_after {
                let previous_state: &DurableCampaignState = previous_state;
                if state.identity != previous_state.identity
                    || state.revision != previous_state.revision.saturating_add(1)
                {
                    return Err(EventError::Binding {
                        sequence: event.sequence,
                    });
                }
            }
            latest_state_after = Some(state.clone());
        }
        last_time = event.payload.occurred_at_unix_seconds;
        previous.clone_from(&event.record_sha256);
        last = Some(event);
    }
    let last = last.ok_or(EventError::Empty)?;
    Ok(EventVerificationReport {
        records: last.sequence,
        head_sha256: last.record_sha256,
        campaign_id: campaign_id.ok_or(EventError::Empty)?,
        last_occurred_at_unix_seconds: last_time,
        recovered_incomplete_trailing_bytes: recovered,
        replay_index,
        latest_state_after,
    })
}

pub fn append_or_replay(
    journal: &Path,
    head: &Path,
    occurred_at_unix_seconds: u64,
    request: Request,
    outcome: EventOutcome,
    response: Response,
    state_after: Option<DurableCampaignState>,
) -> Result<EventAppend, EventError> {
    let digest = request_sha256(&request)?;
    let existing = match load_existing(journal, head)? {
        Some(report) => {
            if report.campaign_id != request.campaign_id {
                return Err(EventError::CampaignDrift {
                    sequence: report.records + 1,
                });
            }
            if let Some(recorded) = report.replay_index.get(&request.request_id) {
                return if recorded.request_sha256 == digest {
                    Ok(EventAppend::Replayed(recorded.response.clone()))
                } else {
                    Err(EventError::ReplayConflict {
                        request_id: request.request_id,
                    })
                };
            }
            Some(report)
        }
        None => None,
    };
    let sequence = existing.as_ref().map_or(1, |report| report.records + 1);
    if request.operation == Operation::Start {
        let recorded: DurableCampaignState = response
            .body
            .result
            .clone()
            .and_then(|value| serde_json::from_value(value).ok())
            .ok_or(EventError::Binding { sequence })?;
        if existing
            .as_ref()
            .and_then(|report| report.latest_state_after.as_ref())
            != Some(&recorded)
            || recorded.revision != response.body.state_revision
        {
            return Err(EventError::Binding { sequence });
        }
    }
    let previous = existing
        .as_ref()
        .map_or(GENESIS_HASH, |report| report.head_sha256.as_str());
    if existing
        .as_ref()
        .is_some_and(|report| occurred_at_unix_seconds < report.last_occurred_at_unix_seconds)
    {
        return Err(EventError::TimestampReversal { sequence });
    }
    let event = build_event(
        sequence,
        previous,
        occurred_at_unix_seconds,
        request,
        outcome,
        response,
        state_after,
    )?;
    append_event(journal, head, &event, existing.as_ref())?;
    Ok(EventAppend::Appended(Box::new(event)))
}

fn load_existing(
    journal: &Path,
    head: &Path,
) -> Result<Option<EventVerificationReport>, EventError> {
    match fs::symlink_metadata(journal) {
        Ok(_) => {
            let report = verify_event_journal(journal, head)?;
            if report.recovered_incomplete_trailing_bytes != 0 {
                return Err(EventError::TornTailRequiresRecovery {
                    bytes: report.recovered_incomplete_trailing_bytes,
                });
            }
            Ok(Some(report))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn append_event(
    journal: &Path,
    head: &Path,
    event: &EventEnvelope,
    existing: Option<&EventVerificationReport>,
) -> Result<(), EventError> {
    let expected_sequence = existing.map_or(1, |report| report.records + 1);
    let expected_previous = existing.map_or(GENESIS_HASH, |report| report.head_sha256.as_str());
    validate_event(event, expected_sequence, expected_previous)?;
    if let Some(parent) = journal.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut line = canonical_json(event)?;
    line.push(b'\n');
    let current_len = fs::metadata(journal).map_or(0, |metadata| metadata.len());
    if current_len.saturating_add(line.len() as u64) > MAX_EVENT_JOURNAL_BYTES {
        return Err(EventError::Oversized);
    }
    let mut file = OpenOptions::new().create(true).append(true).open(journal)?;
    file.write_all(&line)?;
    file.sync_data()?;
    atomic_replace(head, format!("{}\n", event.record_sha256).as_bytes())?;
    Ok(())
}

fn validate_event(
    event: &EventEnvelope,
    expected_sequence: u64,
    expected_previous: &str,
) -> Result<(), EventError> {
    if event.schema_version != 1 {
        return Err(EventError::Schema {
            sequence: event.sequence,
        });
    }
    if event.sequence != expected_sequence {
        return Err(EventError::Sequence {
            expected: expected_sequence,
            actual: event.sequence,
        });
    }
    for (field, value) in [
        ("previous_record_sha256", &event.previous_record_sha256),
        ("payload_sha256", &event.payload_sha256),
        ("record_sha256", &event.record_sha256),
    ] {
        if !is_hash(value) {
            return Err(EventError::InvalidHash {
                sequence: event.sequence,
                field,
            });
        }
    }
    if event.previous_record_sha256 != expected_previous {
        return Err(EventError::PreviousHash {
            sequence: event.sequence,
        });
    }
    match &event.payload.event {
        SupervisorEventKind::Request {
            request_sha256: digest,
            request,
            outcome,
            response,
        } => validate_request_event(event, digest, request.as_ref(), *outcome, response.as_ref())?,
        SupervisorEventKind::Lifecycle {
            cause_request_id,
            cause_request_sha256,
            transition,
        } => validate_lifecycle_event(event, cause_request_id, cause_request_sha256, *transition)?,
    }
    let expected_payload = sha256_hex(&canonical_json(&event.payload)?);
    if expected_payload != event.payload_sha256 {
        return Err(EventError::PayloadHash {
            sequence: event.sequence,
        });
    }
    let expected_record = event_record_hash(
        event.sequence,
        &event.previous_record_sha256,
        &event.payload_sha256,
    )?;
    if expected_record != event.record_sha256 {
        return Err(EventError::RecordHash {
            sequence: event.sequence,
        });
    }
    Ok(())
}

fn validate_request_event(
    event: &EventEnvelope,
    digest: &str,
    request: &Request,
    outcome: EventOutcome,
    response: &Response,
) -> Result<(), EventError> {
    if !is_hash(digest)
        || event.payload.campaign_id != request.campaign_id
        || response.body.request_id != request.request_id
        || response.body.campaign_id != request.campaign_id
        || request.operation == Operation::Verify
        || verify_response(response).is_err()
        || request_sha256(request)? != digest
    {
        return Err(EventError::Binding {
            sequence: event.sequence,
        });
    }
    if (outcome == EventOutcome::Accepted) != response.body.ok {
        return Err(EventError::Outcome {
            sequence: event.sequence,
        });
    }
    match (
        request.operation,
        outcome,
        event.payload.state_after.as_deref(),
    ) {
        (Operation::Attach, EventOutcome::Accepted, Some(state))
            if state.identity.campaign_id == event.payload.campaign_id
                && state.identity.manifest_sha256 == request.manifest_sha256
                && state.revision == request.expected_state_revision.saturating_add(1)
                && state.revision == response.body.state_revision
                && state.controller_lease.as_ref().is_some_and(|lease| {
                    lease.holder_request_id == request.request_id
                        && lease.authorization_sha256 == request.controller.authorization_sha256
                }) => {}
        (Operation::Attach, EventOutcome::Accepted, _) => {
            return Err(EventError::Binding {
                sequence: event.sequence,
            });
        }
        (Operation::Start, EventOutcome::Accepted, None)
            if response
                .body
                .result
                .clone()
                .and_then(|value| serde_json::from_value::<DurableCampaignState>(value).ok())
                .is_some_and(|state| {
                    state.identity.campaign_id == request.campaign_id
                        && state.identity.manifest_sha256 == request.manifest_sha256
                        && state.revision == response.body.state_revision
                        && state.campaign_state == crate::state::CampaignState::I74Running
                }) => {}
        (Operation::Start, EventOutcome::Rejected, None)
            if response
                .body
                .result
                .clone()
                .and_then(|value| serde_json::from_value::<DurableCampaignState>(value).ok())
                .is_some_and(|state| {
                    state.identity.campaign_id == request.campaign_id
                        && state.identity.manifest_sha256 == request.manifest_sha256
                        && state.revision == response.body.state_revision
                        && matches!(
                            state.campaign_state,
                            crate::state::CampaignState::FailedIncomplete
                                | crate::state::CampaignState::CorruptQuarantined
                        )
                }) => {}
        (operation, EventOutcome::Rejected, None) if operation != Operation::Start => {}
        (Operation::Status, EventOutcome::Accepted, None) => {}
        _ => {
            return Err(EventError::Binding {
                sequence: event.sequence,
            });
        }
    }
    Ok(())
}

fn validate_lifecycle_event(
    event: &EventEnvelope,
    cause_request_id: &str,
    cause_request_sha256: &str,
    transition: LifecycleEvent,
) -> Result<(), EventError> {
    let Some(state) = event.payload.state_after.as_deref() else {
        return Err(EventError::Binding {
            sequence: event.sequence,
        });
    };
    let expected_state = match transition {
        LifecycleEvent::Prepared => crate::state::CampaignState::Prepared,
        LifecycleEvent::I74Starting => crate::state::CampaignState::I74Starting,
        LifecycleEvent::I74Started | LifecycleEvent::I74Adopted => {
            crate::state::CampaignState::I74Running
        }
        LifecycleEvent::I74SpawnAbsent | LifecycleEvent::C74SpawnAbsent => {
            crate::state::CampaignState::FailedIncomplete
        }
        LifecycleEvent::I74SpawnMismatch | LifecycleEvent::C74SpawnMismatch => {
            crate::state::CampaignState::CorruptQuarantined
        }
        LifecycleEvent::C74Starting => crate::state::CampaignState::C74Starting,
        LifecycleEvent::C74Started | LifecycleEvent::C74Adopted => {
            crate::state::CampaignState::C74Running
        }
    };
    if cause_request_id.is_empty()
        || cause_request_id.len() > 128
        || !is_hash(cause_request_sha256)
        || state.identity.campaign_id != event.payload.campaign_id
        || state.campaign_state != expected_state
        || (transition == LifecycleEvent::Prepared && state.revision != 0)
        || (transition != LifecycleEvent::Prepared && state.revision == 0)
        || (matches!(
            transition,
            LifecycleEvent::Prepared
                | LifecycleEvent::I74Starting
                | LifecycleEvent::C74Starting
                | LifecycleEvent::I74SpawnAbsent
                | LifecycleEvent::C74SpawnAbsent
                | LifecycleEvent::I74SpawnMismatch
                | LifecycleEvent::C74SpawnMismatch
        ) && (state.harness.is_some() || state.daemon.is_some() || state.checkpoint.is_some()))
        || (matches!(
            transition,
            LifecycleEvent::I74Started
                | LifecycleEvent::I74Adopted
                | LifecycleEvent::C74Started
                | LifecycleEvent::C74Adopted
        ) && (state.harness.is_none() || state.daemon.is_none() || state.checkpoint.is_some()))
    {
        return Err(EventError::Binding {
            sequence: event.sequence,
        });
    }
    Ok(())
}

fn event_record_hash(
    sequence: u64,
    previous_record_sha256: &str,
    payload_sha256: &str,
) -> Result<String, EventError> {
    Ok(chain_hash(
        EVENT_DOMAIN,
        sequence,
        previous_record_sha256,
        payload_sha256,
    )?)
}

fn verify_head(path: &Path, expected: &str) -> Result<(), EventError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| EventError::Head)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() != 65 {
        return Err(EventError::Head);
    }
    let mut bytes = Vec::with_capacity(65);
    File::open(path)
        .and_then(|mut file| file.read_to_end(&mut bytes))
        .map_err(|_| EventError::Head)?;
    if bytes != format!("{expected}\n").as_bytes() {
        return Err(EventError::Head);
    }
    Ok(())
}

fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<(), EventError> {
    let parent = path.parent().ok_or(EventError::Head)?;
    fs::create_dir_all(parent)?;
    let temporary = temporary_path(path)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    replace_file(&temporary, path)?;
    sync_directory(parent)?;
    Ok(())
}

fn temporary_path(path: &Path) -> Result<PathBuf, EventError> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(EventError::Head)?;
    Ok(path.with_file_name(format!(".{name}.tmp.{}", std::process::id())))
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> Result<(), EventError> {
    fs::rename(source, destination)?;
    Ok(())
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> Result<(), EventError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // SAFETY: both paths are valid nul-terminated buffers for the duration of the call.
    if unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), EventError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), EventError> {
    Ok(())
}
