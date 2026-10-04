use hydracache_long_run_supervisor_074::watchdog::{
    ProgressObservation, ProgressWatchdog, WatchdogError,
};
use hydracache_long_run_supervisor_074::{
    append_record, build_record, verify_journal, ChainError, CheckpointPayload, Phase,
    VerificationReport, GENESIS_HASH,
};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const CHECKPOINT_JOURNAL_NAME: &str = "checkpoints.jsonl";
pub const CHECKPOINT_HEAD_NAME: &str = "checkpoints.head";
pub const WARNING_GAP_SECONDS: u64 = 90;
pub const REJECTION_GAP_SECONDS: u64 = 180;

#[derive(Debug, Error)]
pub enum CheckpointWriterError {
    #[error("checkpoint directory or initial state violates the frozen harness contract")]
    Initialization,
    #[error("checkpoint phase progress is invalid: {0}")]
    Progress(#[from] WatchdogError),
    #[error("checkpoint chain update failed: {0}")]
    Chain(#[from] ChainError),
    #[error("checkpoint writer I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("final checkpoint is not terminal")]
    NonTerminal,
}

#[derive(Debug)]
pub struct DurableCheckpointWriter {
    journal: PathBuf,
    head: PathBuf,
    sequence: u64,
    previous_record_sha256: String,
    watchdog: ProgressWatchdog,
}

impl DurableCheckpointWriter {
    pub fn start(
        role_directory: &Path,
        initial: CheckpointPayload,
        observed_unix_seconds: u64,
    ) -> Result<Self, CheckpointWriterError> {
        let metadata = fs::symlink_metadata(role_directory)?;
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || initial.phase != Phase::Startup
        {
            return Err(CheckpointWriterError::Initialization);
        }
        let journal = role_directory.join(CHECKPOINT_JOURNAL_NAME);
        let head = role_directory.join(CHECKPOINT_HEAD_NAME);
        if journal.exists() || head.exists() {
            return Err(CheckpointWriterError::Initialization);
        }
        let record = build_record(1, GENESIS_HASH, initial.clone())?;
        append_record(&journal, &head, &record)?;
        let watchdog = ProgressWatchdog::new(
            1,
            initial,
            observed_unix_seconds,
            WARNING_GAP_SECONDS,
            REJECTION_GAP_SECONDS,
        )?;
        Ok(Self {
            journal,
            head,
            sequence: 1,
            previous_record_sha256: record.record_sha256,
            watchdog,
        })
    }

    pub fn append(
        &mut self,
        payload: CheckpointPayload,
        observed_unix_seconds: u64,
    ) -> Result<ProgressObservation, CheckpointWriterError> {
        let next_sequence = self.sequence.saturating_add(1);
        let mut next_watchdog = self.watchdog.clone();
        let observation =
            next_watchdog.observe(next_sequence, payload.clone(), observed_unix_seconds)?;
        let record = build_record(next_sequence, &self.previous_record_sha256, payload)?;
        append_record(&self.journal, &self.head, &record)?;
        self.sequence = next_sequence;
        self.previous_record_sha256 = record.record_sha256;
        self.watchdog = next_watchdog;
        Ok(observation)
    }

    pub fn report(&self) -> Result<VerificationReport, CheckpointWriterError> {
        Ok(verify_journal(&self.journal)?)
    }

    pub fn finish(self) -> Result<VerificationReport, CheckpointWriterError> {
        let report = verify_journal(&self.journal)?;
        if report.last_phase != Phase::Terminal || report.recovered_incomplete_trailing_bytes != 0 {
            return Err(CheckpointWriterError::NonTerminal);
        }
        Ok(report)
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn journal_path(&self) -> &Path {
        &self.journal
    }
}
