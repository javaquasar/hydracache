use hydracache_long_run_supervisor_074::artifact::PacketResult;
use hydracache_long_run_supervisor_074::manifest::{CampaignManifest, MAX_MANIFEST_BYTES};
use hydracache_long_run_supervisor_074::seal_input::{
    InventoryGuardEvidence, SealInputInventory, SEAL_INPUT_INVENTORY_NAME,
};
use hydracache_long_run_supervisor_074::watchdog::{
    ProgressObservation, ProgressWatchdog, WatchdogError,
};
use hydracache_long_run_supervisor_074::{
    append_record, build_record, verify_journal, ChainError, CheckpointPayload, Phase, Role,
    VerificationReport, GENESIS_HASH,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

pub const CHECKPOINT_JOURNAL_NAME: &str = "checkpoints.jsonl";
pub const CHECKPOINT_HEAD_NAME: &str = "checkpoints.head";
pub const WARNING_GAP_SECONDS: u64 = 90;
pub const REJECTION_GAP_SECONDS: u64 = 180;
const RELEASE: &str = "0.74";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalGuardResult {
    pub id: String,
    pub passed: bool,
    pub evidence_relative_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalSealInput {
    pub campaign_manifest: CampaignManifest,
    pub campaign_manifest_sha256: String,
    pub result: PacketResult,
    pub terminal_reason: Option<String>,
    pub guards: Vec<TerminalGuardResult>,
    pub additional_raw_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalPublicationReceipt {
    pub report: VerificationReport,
    pub inventory_path: PathBuf,
    pub inventory_sha256: String,
}

#[derive(Debug, Serialize)]
struct GuardResultDocument<'a> {
    schema_version: u32,
    release: &'static str,
    campaign_id: &'a str,
    role: &'a Role,
    guard_id: &'a str,
    passed: bool,
    evidence_relative_paths: &'a [PathBuf],
}

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
    #[error("terminal seal inputs do not match the frozen campaign contract")]
    SealBinding,
    #[error("terminal seal input path is unsafe")]
    SealPath,
    #[error("terminal seal input exceeds the frozen file or byte limit")]
    SealLimit,
    #[error("terminal seal JSON failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug)]
pub struct DurableCheckpointWriter {
    role_directory: PathBuf,
    journal: PathBuf,
    head: PathBuf,
    sequence: u64,
    previous_record_sha256: String,
    watchdog: ProgressWatchdog,
}

impl DurableCheckpointWriter {
    pub fn start(
        role_directory: &Path,
        mut initial: CheckpointPayload,
        observed_unix_seconds: u64,
    ) -> Result<Self, CheckpointWriterError> {
        let metadata = fs::symlink_metadata(role_directory)?;
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || initial.phase != Phase::Startup
        {
            return Err(CheckpointWriterError::Initialization);
        }
        let role_directory = fs::canonicalize(role_directory)?;
        let journal = role_directory.join(CHECKPOINT_JOURNAL_NAME);
        let head = role_directory.join(CHECKPOINT_HEAD_NAME);
        if journal.exists() || head.exists() {
            return Err(CheckpointWriterError::Initialization);
        }
        initial.observed_unix_seconds = observed_unix_seconds;
        initial.useful_progress_unix_seconds = observed_unix_seconds;
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
            role_directory,
            journal,
            head,
            sequence: 1,
            previous_record_sha256: record.record_sha256,
            watchdog,
        })
    }

    pub fn append(
        &mut self,
        mut payload: CheckpointPayload,
        observed_unix_seconds: u64,
    ) -> Result<ProgressObservation, CheckpointWriterError> {
        let next_sequence = self.sequence.saturating_add(1);
        let mut next_watchdog = self.watchdog.clone();
        payload.observed_unix_seconds = observed_unix_seconds;
        let observation =
            next_watchdog.observe(next_sequence, payload.clone(), observed_unix_seconds)?;
        payload.useful_progress_unix_seconds = next_watchdog.last_useful_progress_unix_seconds();
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
        terminal_report(&self.journal)
    }

    pub fn finish_and_publish(
        self,
        input: TerminalSealInput,
    ) -> Result<TerminalPublicationReceipt, CheckpointWriterError> {
        let report = terminal_report(&self.journal)?;
        let role_name = role_name(&report.role);
        let roles_directory = self
            .role_directory
            .parent()
            .filter(|path| path.file_name().and_then(|name| name.to_str()) == Some("roles"))
            .ok_or(CheckpointWriterError::SealPath)?;
        let campaign_directory = roles_directory
            .parent()
            .ok_or(CheckpointWriterError::SealPath)?;
        if self
            .role_directory
            .file_name()
            .and_then(|name| name.to_str())
            != Some(role_name)
            || campaign_directory
                .file_name()
                .and_then(|name| name.to_str())
                != Some(input.campaign_manifest.campaign_id.as_str())
            || report.campaign_id != input.campaign_manifest.campaign_id
        {
            return Err(CheckpointWriterError::SealBinding);
        }
        verify_campaign_manifest(
            campaign_directory,
            &input.campaign_manifest,
            &input.campaign_manifest_sha256,
        )?;
        validate_terminal_result(
            input.result,
            input.terminal_reason.as_deref(),
            &input.guards,
        )?;

        let role_root = PathBuf::from(format!("roles/{role_name}"));
        let guard_root = role_root.join("guards");
        let mut by_id = BTreeMap::new();
        for mut guard in input.guards {
            guard
                .evidence_relative_paths
                .sort_by_key(|path| normalized(path));
            guard.evidence_relative_paths.dedup();
            if guard.evidence_relative_paths.is_empty()
                || guard
                    .evidence_relative_paths
                    .iter()
                    .any(|path| validate_role_file(campaign_directory, path, &role_root).is_err())
                || by_id.insert(guard.id.clone(), guard).is_some()
            {
                return Err(CheckpointWriterError::SealBinding);
            }
        }
        if by_id.keys().cloned().collect::<Vec<_>>()
            != sorted_unique(&input.campaign_manifest.required_final_guards)?
        {
            return Err(CheckpointWriterError::SealBinding);
        }

        let mut raw_files = input
            .additional_raw_files
            .into_iter()
            .collect::<BTreeSet<_>>();
        for path in &raw_files {
            validate_role_file(campaign_directory, path, &role_root)?;
        }
        raw_files.insert(PathBuf::from("campaign-start.json"));
        let journal_relative_path = role_root.join(CHECKPOINT_JOURNAL_NAME);
        raw_files.insert(journal_relative_path.clone());

        let guard_directory = campaign_directory.join(&guard_root);
        ensure_directory(&guard_directory)?;
        let mut guard_evidence = Vec::new();
        for guard_id in &input.campaign_manifest.required_final_guards {
            let guard = by_id
                .get(guard_id)
                .ok_or(CheckpointWriterError::SealBinding)?;
            for path in &guard.evidence_relative_paths {
                raw_files.insert(path.clone());
            }
            let result_relative_path =
                guard_root.join(format!("{}.json", sha256_hex(guard_id.as_bytes())));
            let document = canonical_json(&GuardResultDocument {
                schema_version: 1,
                release: RELEASE,
                campaign_id: &report.campaign_id,
                role: &report.role,
                guard_id,
                passed: guard.passed,
                evidence_relative_paths: &guard.evidence_relative_paths,
            })?;
            publish_exact(&campaign_directory.join(&result_relative_path), &document)?;
            raw_files.insert(result_relative_path.clone());
            guard_evidence.push(InventoryGuardEvidence {
                id: guard_id.clone(),
                passed: guard.passed,
                source_relative_path: result_relative_path,
            });
        }

        let inventory_relative_path = role_root.join(SEAL_INPUT_INVENTORY_NAME);
        raw_files.insert(inventory_relative_path.clone());
        let raw_files = raw_files.into_iter().collect::<Vec<_>>();
        for path in &raw_files {
            if path == Path::new("campaign-start.json")
                || path == &inventory_relative_path
                || guard_evidence
                    .iter()
                    .any(|guard| &guard.source_relative_path == path)
            {
                continue;
            }
            validate_role_file(campaign_directory, path, &role_root)?;
        }
        if raw_files.len()
            > usize::try_from(input.campaign_manifest.output_limits.files)
                .map_err(|_| CheckpointWriterError::SealLimit)?
        {
            return Err(CheckpointWriterError::SealLimit);
        }
        let inventory = SealInputInventory {
            schema_version: 1,
            release: RELEASE.to_owned(),
            campaign_id: report.campaign_id.clone(),
            campaign_manifest_sha256: input.campaign_manifest_sha256,
            role: report.role.clone(),
            result: input.result,
            terminal_reason: input.terminal_reason,
            journal_relative_path,
            guard_evidence,
            raw_files,
        };
        let inventory_bytes = canonical_json(&inventory)?;
        enforce_total_limit(
            campaign_directory,
            &inventory,
            inventory_bytes.len() as u64,
            input.campaign_manifest.output_limits.final_artifact_bytes,
            input.campaign_manifest.maximum_campaign_bytes,
        )?;
        let inventory_path = campaign_directory.join(&inventory_relative_path);
        publish_exact(&inventory_path, &inventory_bytes)?;
        Ok(TerminalPublicationReceipt {
            report,
            inventory_path,
            inventory_sha256: sha256_hex(&inventory_bytes),
        })
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn journal_path(&self) -> &Path {
        &self.journal
    }
}

fn terminal_report(path: &Path) -> Result<VerificationReport, CheckpointWriterError> {
    let report = verify_journal(path)?;
    if report.last_phase != Phase::Terminal || report.recovered_incomplete_trailing_bytes != 0 {
        return Err(CheckpointWriterError::NonTerminal);
    }
    Ok(report)
}

fn validate_terminal_result(
    result: PacketResult,
    reason: Option<&str>,
    guards: &[TerminalGuardResult],
) -> Result<(), CheckpointWriterError> {
    let shape_is_valid = match (result, reason) {
        (PacketResult::Complete, None) => guards.iter().all(|guard| guard.passed),
        (PacketResult::Incomplete, Some(reason)) => {
            !reason.is_empty() && reason.len() <= 128 && !reason.contains(['\0', '\n', '\r'])
        }
        _ => false,
    };
    if !shape_is_valid {
        return Err(CheckpointWriterError::SealBinding);
    }
    Ok(())
}

fn verify_campaign_manifest(
    campaign_directory: &Path,
    expected: &CampaignManifest,
    expected_sha256: &str,
) -> Result<(), CheckpointWriterError> {
    if !is_hash(expected_sha256) {
        return Err(CheckpointWriterError::SealBinding);
    }
    let bytes = read_regular(
        &campaign_directory.join("campaign-start.json"),
        MAX_MANIFEST_BYTES as u64,
    )?;
    let encoded = bytes.strip_suffix(b"\n").unwrap_or(&bytes);
    let decoded: CampaignManifest = serde_json::from_slice(encoded)?;
    let value: serde_json::Value = serde_json::from_slice(encoded)?;
    if encoded.is_empty()
        || encoded.contains(&b'\n')
        || encoded.contains(&b'\r')
        || serde_json::to_vec(&value)? != encoded
        || sha256_hex(encoded) != expected_sha256
        || &decoded != expected
    {
        return Err(CheckpointWriterError::SealBinding);
    }
    Ok(())
}

fn sorted_unique(values: &[String]) -> Result<Vec<String>, CheckpointWriterError> {
    let sorted = values.iter().cloned().collect::<BTreeSet<_>>();
    if sorted.len() != values.len() {
        return Err(CheckpointWriterError::SealBinding);
    }
    Ok(sorted.into_iter().collect())
}

fn validate_role_file(
    campaign_directory: &Path,
    relative: &Path,
    role_root: &Path,
) -> Result<(), CheckpointWriterError> {
    validate_relative_path(relative)?;
    if !relative.starts_with(role_root) {
        return Err(CheckpointWriterError::SealPath);
    }
    reject_symlink_components(campaign_directory, relative)?;
    let _ = read_regular(&campaign_directory.join(relative), u64::MAX)?;
    Ok(())
}

fn reject_symlink_components(root: &Path, relative: &Path) -> Result<(), CheckpointWriterError> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        if fs::symlink_metadata(&current)?.file_type().is_symlink() {
            return Err(CheckpointWriterError::SealPath);
        }
    }
    Ok(())
}

fn enforce_total_limit(
    campaign_directory: &Path,
    inventory: &SealInputInventory,
    inventory_bytes: u64,
    artifact_limit: u64,
    campaign_limit: u64,
) -> Result<(), CheckpointWriterError> {
    let inventory_relative_path = PathBuf::from(format!(
        "roles/{}/{}",
        role_name(&inventory.role),
        SEAL_INPUT_INVENTORY_NAME
    ));
    let mut total = 0_u64;
    for path in &inventory.raw_files {
        let size = if path == &inventory_relative_path {
            inventory_bytes
        } else {
            fs::symlink_metadata(campaign_directory.join(path))?.len()
        };
        total = total
            .checked_add(size)
            .ok_or(CheckpointWriterError::SealLimit)?;
    }
    if total > artifact_limit || total > campaign_limit {
        return Err(CheckpointWriterError::SealLimit);
    }
    Ok(())
}

fn ensure_directory(path: &Path) -> Result<(), CheckpointWriterError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(CheckpointWriterError::SealPath),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path)?;
            sync_directory(path.parent().ok_or(CheckpointWriterError::SealPath)?)?;
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

fn publish_exact(path: &Path, bytes: &[u8]) -> Result<(), CheckpointWriterError> {
    let parent = path.parent().ok_or(CheckpointWriterError::SealPath)?;
    ensure_directory(parent)?;
    let mut pending_name = path
        .file_name()
        .ok_or(CheckpointWriterError::SealPath)?
        .to_os_string();
    pending_name.push(".pending");
    let pending = parent.join(pending_name);
    if path.exists() {
        let pending_exists = pending.exists();
        if read_regular_with_link_policy(path, bytes.len() as u64, pending_exists)? != bytes {
            return Err(CheckpointWriterError::SealBinding);
        }
        remove_matching_pending(&pending, bytes)?;
        if read_regular(path, bytes.len() as u64)? != bytes {
            return Err(CheckpointWriterError::SealBinding);
        }
        return Ok(());
    }
    if pending.exists() {
        if read_regular(&pending, bytes.len() as u64)? != bytes {
            return Err(CheckpointWriterError::SealBinding);
        }
    } else {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pending)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    match fs::hard_link(&pending, path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if read_regular_with_link_policy(path, bytes.len() as u64, true)? != bytes {
                return Err(CheckpointWriterError::SealBinding);
            }
        }
        Err(error) => return Err(error.into()),
    }
    fs::remove_file(&pending)?;
    sync_directory(parent)?;
    if read_regular(path, bytes.len() as u64)? != bytes {
        return Err(CheckpointWriterError::SealBinding);
    }
    Ok(())
}

fn remove_matching_pending(path: &Path, bytes: &[u8]) -> Result<(), CheckpointWriterError> {
    match fs::symlink_metadata(path) {
        Ok(_) => {
            if read_regular_with_link_policy(path, bytes.len() as u64, true)? != bytes {
                return Err(CheckpointWriterError::SealBinding);
            }
            fs::remove_file(path)?;
            sync_directory(path.parent().ok_or(CheckpointWriterError::SealPath)?)?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

fn read_regular(path: &Path, maximum_bytes: u64) -> Result<Vec<u8>, CheckpointWriterError> {
    read_regular_with_link_policy(path, maximum_bytes, false)
}

fn read_regular_with_link_policy(
    path: &Path,
    maximum_bytes: u64,
    allow_multiple_links: bool,
) -> Result<Vec<u8>, CheckpointWriterError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > maximum_bytes
        || (!allow_multiple_links && has_multiple_links(&metadata))
    {
        return Err(CheckpointWriterError::SealPath);
    }
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or(0));
    File::open(path)?
        .take(maximum_bytes.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 != metadata.len() {
        return Err(CheckpointWriterError::SealPath);
    }
    Ok(bytes)
}

fn validate_relative_path(path: &Path) -> Result<(), CheckpointWriterError> {
    let value = normalized(path);
    if path.is_absolute()
        || value.is_empty()
        || value.len() > 1024
        || value.contains('\\')
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(CheckpointWriterError::SealPath);
    }
    Ok(())
}

fn normalized(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(&serde_json::to_value(value)?)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(64);
    for byte in digest {
        value.push(HEX[(byte >> 4) as usize] as char);
        value.push(HEX[(byte & 0x0f) as usize] as char);
    }
    value
}

fn is_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn role_name(role: &Role) -> &'static str {
    match role {
        Role::I74 => "i74",
        Role::C74 => "c74",
    }
}

#[cfg(unix)]
fn has_multiple_links(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    metadata.nlink() != 1
}

#[cfg(not(unix))]
fn has_multiple_links(_metadata: &fs::Metadata) -> bool {
    false
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), CheckpointWriterError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), CheckpointWriterError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{publish_exact, CheckpointWriterError};
    use std::fs;

    #[test]
    fn exact_pending_file_is_completed_and_removed() {
        let temporary = tempfile::tempdir().unwrap();
        let output = temporary.path().join("result.json");
        let pending = temporary.path().join("result.json.pending");
        fs::write(&pending, b"exact").unwrap();

        publish_exact(&output, b"exact").unwrap();

        assert_eq!(fs::read(output).unwrap(), b"exact");
        assert!(!pending.exists());
    }

    #[test]
    fn linked_crash_window_is_reconciled_without_replacement() {
        let temporary = tempfile::tempdir().unwrap();
        let output = temporary.path().join("result.json");
        let pending = temporary.path().join("result.json.pending");
        fs::write(&pending, b"exact").unwrap();
        fs::hard_link(&pending, &output).unwrap();

        publish_exact(&output, b"exact").unwrap();

        assert_eq!(fs::read(output).unwrap(), b"exact");
        assert!(!pending.exists());
    }

    #[test]
    fn divergent_existing_publication_is_not_overwritten() {
        let temporary = tempfile::tempdir().unwrap();
        let output = temporary.path().join("result.json");
        fs::write(&output, b"first").unwrap();

        assert!(matches!(
            publish_exact(&output, b"second"),
            Err(CheckpointWriterError::SealBinding)
        ));
        assert_eq!(fs::read(output).unwrap(), b"first");
    }
}
