use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[cfg(target_os = "linux")]
pub mod abort_backend;
#[cfg(target_os = "linux")]
pub mod abort_lifecycle;
pub mod archive;
pub mod artifact;
pub mod auth;
#[cfg(target_os = "linux")]
pub mod campaign_smoke;
pub mod checkpoint_evidence;
#[cfg(target_os = "linux")]
pub mod client;
#[cfg(target_os = "linux")]
pub mod config;
pub mod diagnostic_artifacts;
pub mod diagnostic_ipc;
pub mod diagnostic_lease;
pub mod diagnostic_receipts;
#[cfg(target_os = "linux")]
pub mod diagnostic_unit;
pub mod event;
pub mod host_execution;
pub mod host_receipt;
#[cfg(target_os = "linux")]
pub mod lease_expiry;
pub mod manifest;
pub mod manifest_evidence;
#[cfg(target_os = "linux")]
pub mod measurement_loss;
pub mod mutation;
#[cfg(target_os = "linux")]
pub mod process_identity;
#[cfg(target_os = "linux")]
pub mod progress_loss;
pub mod protocol;
pub mod request_builder;
#[cfg(target_os = "linux")]
pub mod role_overhead_smoke;
pub mod seal_artifact;
pub mod seal_input;
#[cfg(target_os = "linux")]
pub mod seal_lifecycle;
mod sealed_permissions;
#[cfg(target_os = "linux")]
pub mod server;
#[cfg(target_os = "linux")]
pub mod service;
pub mod spawn;
pub mod start_evidence;
pub mod start_lifecycle;
pub mod state;
pub mod state_store;
#[cfg(target_os = "linux")]
pub mod systemd_notify;
#[cfg(target_os = "linux")]
pub mod systemd_smoke;
#[cfg(target_os = "linux")]
pub mod systemd_spawn;
#[cfg(target_os = "linux")]
pub mod systemd_unit;
pub mod watchdog;

#[cfg(target_os = "linux")]
pub mod unix_transport;

pub const DOMAIN: &[u8] = b"hydracache-long-run-record-v1";
pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";
pub const MAX_JOURNAL_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessIdentity {
    pub boot_id: String,
    pub pid: u32,
    pub start_ticks: u64,
    pub process_group: i64,
    pub cgroup_path: String,
    pub cgroup_inode: u64,
    pub unit_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    I74,
    C74,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    Startup,
    Warmup,
    Measured,
    Drain,
    DurableCompanion,
    PostWorkIdle,
    Reconciliation,
    Terminal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointPayload {
    pub campaign_id: String,
    pub role: Role,
    pub phase: Phase,
    pub phase_epoch: u64,
    pub monotonic_elapsed_ns: u64,
    pub wall_clock_utc: String,
    pub observed_unix_seconds: u64,
    pub useful_progress_unix_seconds: u64,
    pub completed: u64,
    pub failed: u64,
    pub rejected: u64,
    pub timed_out: u64,
    pub outstanding: u64,
    pub telemetry_sequence: u64,
    pub milestone: String,
    pub surface_counters: BTreeMap<String, u64>,
    pub resource_counters: BTreeMap<String, u64>,
    pub owner_counters: BTreeMap<String, u64>,
    pub harness: ProcessIdentity,
    pub daemon: ProcessIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordEnvelope {
    pub schema_version: u32,
    pub sequence: u64,
    pub previous_record_sha256: String,
    pub payload: CheckpointPayload,
    pub payload_sha256: String,
    pub record_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VerificationReport {
    pub records: u64,
    pub first_record_sha256: String,
    pub head_sha256: String,
    pub campaign_id: String,
    pub role: Role,
    pub last_phase: Phase,
    pub last_monotonic_elapsed_ns: u64,
    pub last_observed_unix_seconds: u64,
    pub last_useful_progress_unix_seconds: u64,
    pub recovered_incomplete_trailing_bytes: usize,
    pub harness: ProcessIdentity,
    pub daemon: ProcessIdentity,
}

#[derive(Debug, Error)]
pub enum ChainError {
    #[error("journal is empty")]
    Empty,
    #[error("journal is not a bounded regular file")]
    UnsafeJournal,
    #[error("invalid UTF-8 or JSON at record {record}: {message}")]
    Parse { record: usize, message: String },
    #[error("schema version at sequence {sequence} must be 1")]
    Schema { sequence: u64 },
    #[error("sequence mismatch: expected {expected}, got {actual}")]
    Sequence { expected: u64, actual: u64 },
    #[error("previous hash mismatch at sequence {sequence}")]
    PreviousHash { sequence: u64 },
    #[error("payload hash mismatch at sequence {sequence}")]
    PayloadHash { sequence: u64 },
    #[error("record hash mismatch at sequence {sequence}")]
    RecordHash { sequence: u64 },
    #[error("campaign, role or process identity drift at sequence {sequence}")]
    IdentityDrift { sequence: u64 },
    #[error("monotonic or wall-clock timestamp reversal at sequence {sequence}")]
    TimestampReversal { sequence: u64 },
    #[error("invalid lowercase SHA-256 field {field} at sequence {sequence}")]
    InvalidHash { sequence: u64, field: &'static str },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error(
        "journal has {bytes} incomplete trailing bytes; archive them before explicit recovery"
    )]
    TornTailRequiresRecovery { bytes: usize },
}

pub fn build_record(
    sequence: u64,
    previous_record_sha256: &str,
    payload: CheckpointPayload,
) -> Result<RecordEnvelope, ChainError> {
    if !is_hash(previous_record_sha256) {
        return Err(ChainError::InvalidHash {
            sequence,
            field: "previous_record_sha256",
        });
    }
    let payload_sha256 = sha256_hex(&canonical_json(&payload)?);
    let record_sha256 = record_hash(sequence, previous_record_sha256, &payload_sha256)?;
    Ok(RecordEnvelope {
        schema_version: 1,
        sequence,
        previous_record_sha256: previous_record_sha256.to_owned(),
        payload,
        payload_sha256,
        record_sha256,
    })
}

pub fn verify_journal(path: &Path) -> Result<VerificationReport, ChainError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_JOURNAL_BYTES
    {
        return Err(ChainError::UnsafeJournal);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)?
        .take(MAX_JOURNAL_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_JOURNAL_BYTES {
        return Err(ChainError::UnsafeJournal);
    }
    verify_journal_bytes(&bytes)
}

pub fn verify_journal_bytes(bytes: &[u8]) -> Result<VerificationReport, ChainError> {
    if bytes.is_empty() {
        return Err(ChainError::Empty);
    }
    if bytes.len() as u64 > MAX_JOURNAL_BYTES {
        return Err(ChainError::UnsafeJournal);
    }
    let has_terminal_newline = bytes.last() == Some(&b'\n');
    let mut parts = bytes.split(|byte| *byte == b'\n').collect::<Vec<_>>();
    if has_terminal_newline {
        parts.pop();
    }
    let mut recovered = 0;
    if !has_terminal_newline {
        let last = parts.last().copied().unwrap_or_default();
        recovered = last.len();
        parts.pop();
    }
    if parts.is_empty() {
        return if recovered == 0 {
            Err(ChainError::Empty)
        } else {
            Err(ChainError::TornTailRequiresRecovery { bytes: recovered })
        };
    }

    let mut previous = GENESIS_HASH.to_owned();
    let mut first: Option<CheckpointPayload> = None;
    let mut first_record_sha256 = None;
    let mut last_elapsed = 0;
    let mut last_wall = String::new();
    let mut last_observed = 0;
    let mut last_useful = 0;
    let mut last: Option<RecordEnvelope> = None;
    for (index, line) in parts.iter().enumerate() {
        if line.is_empty() {
            return Err(ChainError::Parse {
                record: index + 1,
                message: "empty line".to_owned(),
            });
        }
        let envelope: RecordEnvelope =
            serde_json::from_slice(line).map_err(|error| ChainError::Parse {
                record: index + 1,
                message: error.to_string(),
            })?;
        validate_record(&envelope, index as u64 + 1, &previous)?;
        if let Some(initial) = &first {
            if envelope.payload.campaign_id != initial.campaign_id
                || envelope.payload.role != initial.role
                || envelope.payload.harness != initial.harness
                || envelope.payload.daemon != initial.daemon
            {
                return Err(ChainError::IdentityDrift {
                    sequence: envelope.sequence,
                });
            }
            if envelope.payload.monotonic_elapsed_ns < last_elapsed
                || envelope.payload.wall_clock_utc < last_wall
                || envelope.payload.observed_unix_seconds < last_observed
                || envelope.payload.useful_progress_unix_seconds < last_useful
            {
                return Err(ChainError::TimestampReversal {
                    sequence: envelope.sequence,
                });
            }
        } else {
            first = Some(envelope.payload.clone());
            first_record_sha256 = Some(envelope.record_sha256.clone());
        }
        last_elapsed = envelope.payload.monotonic_elapsed_ns;
        last_wall.clone_from(&envelope.payload.wall_clock_utc);
        last_observed = envelope.payload.observed_unix_seconds;
        last_useful = envelope.payload.useful_progress_unix_seconds;
        previous.clone_from(&envelope.record_sha256);
        last = Some(envelope);
    }
    let first = first.ok_or(ChainError::Empty)?;
    let last = last.ok_or(ChainError::Empty)?;
    Ok(VerificationReport {
        records: last.sequence,
        first_record_sha256: first_record_sha256.ok_or(ChainError::Empty)?,
        head_sha256: last.record_sha256,
        campaign_id: first.campaign_id,
        role: first.role,
        last_phase: last.payload.phase,
        last_monotonic_elapsed_ns: last_elapsed,
        last_observed_unix_seconds: last_observed,
        last_useful_progress_unix_seconds: last_useful,
        recovered_incomplete_trailing_bytes: recovered,
        harness: first.harness,
        daemon: first.daemon,
    })
}

pub fn append_record(
    journal: &Path,
    head: &Path,
    record: &RecordEnvelope,
) -> Result<(), ChainError> {
    let existing = match fs::symlink_metadata(journal) {
        Ok(_) => {
            let report = verify_journal(journal)?;
            if report.recovered_incomplete_trailing_bytes != 0 {
                return Err(ChainError::TornTailRequiresRecovery {
                    bytes: report.recovered_incomplete_trailing_bytes,
                });
            }
            Some(report)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let expected_sequence = existing.as_ref().map_or(1, |report| report.records + 1);
    let expected_previous = existing
        .as_ref()
        .map_or(GENESIS_HASH, |report| report.head_sha256.as_str());
    validate_record(record, expected_sequence, expected_previous)?;
    if let Some(parent) = journal.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(journal)?;
    let mut line = canonical_json(record)?;
    line.push(b'\n');
    file.write_all(&line)?;
    file.sync_data()?;
    atomic_write(head, format!("{}\n", record.record_sha256).as_bytes())?;
    Ok(())
}

fn validate_record(
    envelope: &RecordEnvelope,
    expected_sequence: u64,
    expected_previous: &str,
) -> Result<(), ChainError> {
    if envelope.schema_version != 1
        || envelope.payload.observed_unix_seconds == 0
        || envelope.payload.useful_progress_unix_seconds == 0
        || envelope.payload.useful_progress_unix_seconds > envelope.payload.observed_unix_seconds
    {
        return Err(ChainError::Schema {
            sequence: envelope.sequence,
        });
    }
    if envelope.sequence != expected_sequence {
        return Err(ChainError::Sequence {
            expected: expected_sequence,
            actual: envelope.sequence,
        });
    }
    for (field, value) in [
        ("previous_record_sha256", &envelope.previous_record_sha256),
        ("payload_sha256", &envelope.payload_sha256),
        ("record_sha256", &envelope.record_sha256),
    ] {
        if !is_hash(value) {
            return Err(ChainError::InvalidHash {
                sequence: envelope.sequence,
                field,
            });
        }
    }
    if envelope.previous_record_sha256 != expected_previous {
        return Err(ChainError::PreviousHash {
            sequence: envelope.sequence,
        });
    }
    let expected_payload = sha256_hex(&canonical_json(&envelope.payload)?);
    if envelope.payload_sha256 != expected_payload {
        return Err(ChainError::PayloadHash {
            sequence: envelope.sequence,
        });
    }
    let expected_record = record_hash(
        envelope.sequence,
        &envelope.previous_record_sha256,
        &envelope.payload_sha256,
    )?;
    if envelope.record_sha256 != expected_record {
        return Err(ChainError::RecordHash {
            sequence: envelope.sequence,
        });
    }
    Ok(())
}

pub(crate) fn record_hash(
    sequence: u64,
    previous_record_sha256: &str,
    payload_sha256: &str,
) -> Result<String, ChainError> {
    chain_hash(DOMAIN, sequence, previous_record_sha256, payload_sha256)
}

pub(crate) fn chain_hash(
    domain: &[u8],
    sequence: u64,
    previous_record_sha256: &str,
    payload_sha256: &str,
) -> Result<String, ChainError> {
    let previous = decode_hash(previous_record_sha256).ok_or(ChainError::InvalidHash {
        sequence,
        field: "previous_record_sha256",
    })?;
    let payload = decode_hash(payload_sha256).ok_or(ChainError::InvalidHash {
        sequence,
        field: "payload_sha256",
    })?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(sequence.to_be_bytes());
    digest.update(previous);
    digest.update(payload);
    Ok(hex(&digest.finalize()))
}

pub(crate) fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(&serde_json::to_value(value)?)
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

pub(crate) fn is_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn decode_hash(value: &str) -> Option<[u8; 32]> {
    if !is_hash(value) {
        return None;
    }
    let mut decoded = [0; 32];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        decoded[index] = (nibble(chunk[0])? << 4) | nibble(chunk[1])?;
    }
    Some(decoded)
}

fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(DIGITS[(byte >> 4) as usize] as char);
        encoded.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), ChainError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let temporary = unique_temporary(path);
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

fn unique_temporary(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_owned();
    value.push(format!(".tmp.{}", std::process::id()));
    PathBuf::from(value)
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> Result<(), ChainError> {
    fs::rename(source, destination)?;
    Ok(())
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> Result<(), ChainError> {
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
fn sync_directory(path: &Path) -> Result<(), ChainError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), ChainError> {
    Ok(())
}
