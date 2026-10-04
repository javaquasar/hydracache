use crate::state::{CampaignState, DurableCampaignState};
use crate::{verify_journal, ChainError, Role, VerificationReport};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const CHECKPOINT_JOURNAL_NAME: &str = "checkpoints.jsonl";
pub const CHECKPOINT_HEAD_NAME: &str = "checkpoints.head";

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
    let (directory, expected_role) = role_directory(campaign_directory, state.campaign_state)?;
    let journal = directory.join(CHECKPOINT_JOURNAL_NAME);
    let head = directory.join(CHECKPOINT_HEAD_NAME);
    let report = verify_journal(&journal)?;
    if report.recovered_incomplete_trailing_bytes != 0 {
        return Err(CheckpointEvidenceError::Binding);
    }
    verify_head(&head, &report.head_sha256)?;
    if report.campaign_id != state.identity.campaign_id
        || report.role != expected_role
        || report.records != state.checkpoint.sequence
        || report.head_sha256 != state.checkpoint.record_sha256
        || report.harness != state.harness
        || report.daemon != state.daemon
    {
        return Err(CheckpointEvidenceError::Binding);
    }
    Ok(report)
}

fn role_directory(
    campaign_directory: &Path,
    campaign_state: CampaignState,
) -> Result<(PathBuf, Role), CheckpointEvidenceError> {
    match campaign_state {
        CampaignState::I74Starting | CampaignState::I74Running => {
            Ok((campaign_directory.join("roles").join("i74"), Role::I74))
        }
        CampaignState::C74Starting | CampaignState::C74Running => {
            Ok((campaign_directory.join("roles").join("c74"), Role::C74))
        }
        _ => Err(CheckpointEvidenceError::State),
    }
}

fn verify_head(path: &Path, expected: &str) -> Result<(), CheckpointEvidenceError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| CheckpointEvidenceError::Head)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() != 65 {
        return Err(CheckpointEvidenceError::Head);
    }
    let mut bytes = Vec::with_capacity(65);
    File::open(path)
        .and_then(|mut file| file.read_to_end(&mut bytes))
        .map_err(|_| CheckpointEvidenceError::Head)?;
    if bytes != format!("{expected}\n").as_bytes() {
        return Err(CheckpointEvidenceError::Head);
    }
    Ok(())
}
