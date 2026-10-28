use crate::state::{CampaignState, CheckpointHead, DurableCampaignState};
use crate::{verify_journal, ChainError, Phase, Role, VerificationReport};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use thiserror::Error;

pub const CHECKPOINT_JOURNAL_NAME: &str = "checkpoints.jsonl";
pub const CHECKPOINT_HEAD_NAME: &str = "checkpoints.head";
const SNAPSHOT_RETRY_ATTEMPTS: usize = 20;
const SNAPSHOT_RETRY_DELAY: Duration = Duration::from_millis(5);

#[derive(Debug, Error)]
pub enum CheckpointEvidenceError {
    #[error("campaign state has no attachable live role")]
    State,
    #[error("checkpoint chain failed: {0}")]
    Chain(#[from] ChainError),
    #[error("checkpoint head is missing, unsafe, malformed, or stale")]
    Head,
    #[error("checkpoint chain does not match durable campaign state")]
    Binding,
    #[error("checkpoint evidence I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

pub fn verify_checkpoint_evidence(
    campaign_directory: &Path,
    state: &DurableCampaignState,
) -> Result<VerificationReport, CheckpointEvidenceError> {
    let checkpoint = state
        .checkpoint
        .as_ref()
        .ok_or(CheckpointEvidenceError::Binding)?;
    let (report, observed) = observe_live_checkpoint_evidence(campaign_directory, state)?;
    if observed.sequence != checkpoint.sequence
        || report.head_sha256 != checkpoint.record_sha256
        || observed.useful_progress_unix_seconds != checkpoint.useful_progress_unix_seconds
    {
        return Err(CheckpointEvidenceError::Binding);
    }
    Ok(report)
}

pub fn observe_live_checkpoint_evidence(
    campaign_directory: &Path,
    state: &DurableCampaignState,
) -> Result<(VerificationReport, CheckpointHead), CheckpointEvidenceError> {
    let harness = state
        .harness
        .as_ref()
        .ok_or(CheckpointEvidenceError::Binding)?;
    let daemon = state
        .daemon
        .as_ref()
        .ok_or(CheckpointEvidenceError::Binding)?;
    let (directory, expected_role) = role_directory(campaign_directory, state.campaign_state)?;
    let journal = directory.join(CHECKPOINT_JOURNAL_NAME);
    let head = directory.join(CHECKPOINT_HEAD_NAME);
    let report = observe_consistent_snapshot(&journal, &head)?;
    if report.campaign_id != state.identity.campaign_id
        || report.role != expected_role
        || &report.harness != harness
        || &report.daemon != daemon
    {
        return Err(CheckpointEvidenceError::Binding);
    }
    let observed = CheckpointHead {
        sequence: report.records,
        record_sha256: report.head_sha256.clone(),
        useful_progress_unix_seconds: report.last_useful_progress_unix_seconds,
    };
    if state.checkpoint.as_ref().is_some_and(|previous| {
        observed.sequence < previous.sequence
            || (observed.sequence == previous.sequence && &observed != previous)
            || observed.useful_progress_unix_seconds < previous.useful_progress_unix_seconds
    }) {
        return Err(CheckpointEvidenceError::Binding);
    }
    Ok((report, observed))
}

pub fn verify_terminal_checkpoint_evidence(
    campaign_directory: &Path,
    state: &DurableCampaignState,
) -> Result<VerificationReport, CheckpointEvidenceError> {
    if !matches!(
        state.campaign_state,
        CampaignState::I74Running
            | CampaignState::I74Terminal
            | CampaignState::C74Running
            | CampaignState::C74Terminal
    ) {
        return Err(CheckpointEvidenceError::State);
    }
    let report = verify_checkpoint_evidence(campaign_directory, state)?;
    if report.last_phase != Phase::Terminal {
        return Err(CheckpointEvidenceError::Binding);
    }
    Ok(report)
}

fn role_directory(
    campaign_directory: &Path,
    campaign_state: CampaignState,
) -> Result<(PathBuf, Role), CheckpointEvidenceError> {
    match campaign_state {
        CampaignState::I74Starting | CampaignState::I74Running | CampaignState::I74Terminal => {
            Ok((campaign_directory.join("roles").join("i74"), Role::I74))
        }
        CampaignState::C74Starting | CampaignState::C74Running | CampaignState::C74Terminal => {
            Ok((campaign_directory.join("roles").join("c74"), Role::C74))
        }
        _ => Err(CheckpointEvidenceError::State),
    }
}

fn observe_consistent_snapshot(
    journal: &Path,
    head: &Path,
) -> Result<VerificationReport, CheckpointEvidenceError> {
    for attempt in 0..SNAPSHOT_RETRY_ATTEMPTS {
        let head_before = match read_head(head) {
            Ok(value) => value,
            Err(head_error) => match verify_journal(journal) {
                Err(ChainError::TornTailRequiresRecovery { .. })
                    if attempt + 1 < SNAPSHOT_RETRY_ATTEMPTS =>
                {
                    thread::sleep(SNAPSHOT_RETRY_DELAY);
                    continue;
                }
                Err(error) => return Err(error.into()),
                Ok(_) if attempt + 1 == SNAPSHOT_RETRY_ATTEMPTS => return Err(head_error),
                Ok(_) => {
                    thread::sleep(SNAPSHOT_RETRY_DELAY);
                    continue;
                }
            },
        };
        let report = match verify_journal(journal) {
            Ok(value) => value,
            Err(ChainError::TornTailRequiresRecovery { .. })
                if attempt + 1 < SNAPSHOT_RETRY_ATTEMPTS =>
            {
                thread::sleep(SNAPSHOT_RETRY_DELAY);
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        let head_after = match read_head(head) {
            Ok(value) => value,
            Err(error) if attempt + 1 == SNAPSHOT_RETRY_ATTEMPTS => return Err(error),
            Err(_) => {
                thread::sleep(SNAPSHOT_RETRY_DELAY);
                continue;
            }
        };
        if report.recovered_incomplete_trailing_bytes == 0
            && head_before == head_after
            && head_after == report.head_sha256
        {
            return Ok(report);
        }
        if attempt + 1 < SNAPSHOT_RETRY_ATTEMPTS {
            thread::sleep(SNAPSHOT_RETRY_DELAY);
        }
    }
    Err(CheckpointEvidenceError::Head)
}

fn read_head(path: &Path) -> Result<String, CheckpointEvidenceError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| CheckpointEvidenceError::Head)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() != 65 {
        return Err(CheckpointEvidenceError::Head);
    }
    let mut bytes = Vec::with_capacity(65);
    File::open(path)
        .and_then(|mut file| file.read_to_end(&mut bytes))
        .map_err(|_| CheckpointEvidenceError::Head)?;
    let Some(encoded) = bytes.strip_suffix(b"\n") else {
        return Err(CheckpointEvidenceError::Head);
    };
    let value = std::str::from_utf8(encoded).map_err(|_| CheckpointEvidenceError::Head)?;
    if !crate::is_hash(value) {
        return Err(CheckpointEvidenceError::Head);
    }
    Ok(value.to_owned())
}
