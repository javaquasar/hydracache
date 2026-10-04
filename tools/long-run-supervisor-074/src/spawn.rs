use crate::state::{transition, CampaignState, DurableCampaignState, Transition};
use crate::{canonical_json, is_hash, sha256_hex, ProcessIdentity, Role};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Display;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const MAX_SPAWN_DOCUMENT_BYTES: u64 = 65_536;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpawnIntent {
    pub schema_version: u32,
    pub campaign_id: String,
    pub request_id: String,
    pub request_sha256: String,
    pub manifest_sha256: String,
    pub nonce_sha256: String,
    pub role: Role,
    pub unit_name: String,
}

impl SpawnIntent {
    pub fn new(
        campaign_id: String,
        request_id: String,
        request_sha256: String,
        manifest_sha256: String,
        nonce_sha256: String,
        role: Role,
    ) -> Result<Self, SpawnError> {
        let unit_name = deterministic_unit_name(&campaign_id, &role)?;
        let intent = Self {
            schema_version: 1,
            campaign_id,
            request_id,
            request_sha256,
            manifest_sha256,
            nonce_sha256,
            role,
            unit_name,
        };
        validate_intent(&intent)?;
        Ok(intent)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpawnObservation {
    Exact {
        harness: ProcessIdentity,
        daemon: ProcessIdentity,
    },
    Absent,
    Mismatch {
        reason: SpawnMismatch,
        harness: Option<ProcessIdentity>,
        daemon: Option<ProcessIdentity>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SpawnMismatch {
    Identity,
    MultipleExecutors,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SpawnResolution {
    Started,
    Adopted,
    Absent,
    Mismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpawnResult {
    pub schema_version: u32,
    pub campaign_id: String,
    pub intent_sha256: String,
    pub unit_name: String,
    pub resolution: SpawnResolution,
    pub mismatch: Option<SpawnMismatch>,
    pub harness: Option<ProcessIdentity>,
    pub daemon: Option<ProcessIdentity>,
}

impl SpawnResult {
    pub fn exact_identities(&self) -> Option<(&ProcessIdentity, &ProcessIdentity)> {
        match (&self.harness, &self.daemon) {
            (Some(harness), Some(daemon))
                if matches!(
                    self.resolution,
                    SpawnResolution::Started | SpawnResolution::Adopted
                ) =>
            {
                Some((harness, daemon))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentDisposition {
    Created,
    Recovered,
}

pub trait SpawnBackend {
    type Error: Display;

    fn start_once(&mut self, intent: &SpawnIntent) -> Result<SpawnObservation, Self::Error>;
    fn observe(&mut self, unit_name: &str) -> Result<SpawnObservation, Self::Error>;
}

#[derive(Debug, Error)]
pub enum SpawnError {
    #[error("spawn intent, result, or campaign path is unsafe")]
    Path,
    #[error("spawn evidence is empty, oversized, malformed, or non-canonical")]
    Document,
    #[error("spawn evidence digest does not match")]
    Digest,
    #[error("spawn intent conflicts with existing durable intent")]
    IntentConflict,
    #[error("spawn backend failed after intent became durable: {0}")]
    Backend(String),
    #[error("spawn evidence I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("spawn evidence serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("spawn result cannot be applied to the durable campaign state")]
    State,
}

pub fn deterministic_unit_name(campaign_id: &str, role: &Role) -> Result<String, SpawnError> {
    if !is_hash(campaign_id) {
        return Err(SpawnError::Path);
    }
    Ok(format!(
        "hydracache-performance-074-{}-{campaign_id}.service",
        role_name(role)
    ))
}

pub fn prepare_spawn_intent(
    campaign_directory: &Path,
    intent: &SpawnIntent,
) -> Result<IntentDisposition, SpawnError> {
    let directory = canonical_directory(campaign_directory)?;
    validate_intent(intent)?;
    let paths = evidence_paths(&directory, &intent.role);
    match fs::symlink_metadata(&paths.intent) {
        Ok(_) => {
            let existing: SpawnIntent = read_document_pair(&paths.intent, &paths.intent_head)?;
            validate_intent(&existing)?;
            if existing != *intent {
                return Err(SpawnError::IntentConflict);
            }
            Ok(IntentDisposition::Recovered)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if paths.intent_head.exists() || paths.result.exists() || paths.result_head.exists() {
                return Err(SpawnError::Path);
            }
            write_document_pair(&paths.intent, &paths.intent_head, intent)?;
            Ok(IntentDisposition::Created)
        }
        Err(error) => Err(error.into()),
    }
}

pub fn start_or_recover<B: SpawnBackend>(
    campaign_directory: &Path,
    intent: &SpawnIntent,
    backend: &mut B,
) -> Result<SpawnResult, SpawnError> {
    let directory = canonical_directory(campaign_directory)?;
    let disposition = prepare_spawn_intent(&directory, intent)?;
    let paths = evidence_paths(&directory, &intent.role);
    let intent_sha256 = document_digest(intent)?;

    match fs::symlink_metadata(&paths.result) {
        Ok(_) => {
            let result: SpawnResult = read_document_pair(&paths.result, &paths.result_head)?;
            validate_result(&result, intent, &intent_sha256)?;
            return Ok(result);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if paths.result_head.exists() {
                return Err(SpawnError::Path);
            }
        }
        Err(error) => return Err(error.into()),
    }

    let observation = match disposition {
        IntentDisposition::Created => backend.start_once(intent),
        IntentDisposition::Recovered => backend.observe(&intent.unit_name),
    }
    .map_err(|error| SpawnError::Backend(error.to_string()))?;
    let result = result_from_observation(intent, &intent_sha256, disposition, observation);
    validate_result(&result, intent, &intent_sha256)?;
    write_document_pair(&paths.result, &paths.result_head, &result)?;
    Ok(result)
}

pub fn read_spawn_result(
    campaign_directory: &Path,
    intent: &SpawnIntent,
) -> Result<SpawnResult, SpawnError> {
    let directory = canonical_directory(campaign_directory)?;
    validate_intent(intent)?;
    let paths = evidence_paths(&directory, &intent.role);
    let existing: SpawnIntent = read_document_pair(&paths.intent, &paths.intent_head)?;
    if existing != *intent {
        return Err(SpawnError::IntentConflict);
    }
    let result: SpawnResult = read_document_pair(&paths.result, &paths.result_head)?;
    validate_result(&result, intent, &document_digest(intent)?)?;
    Ok(result)
}

pub fn apply_spawn_result(
    state: &DurableCampaignState,
    role: &Role,
    result: &SpawnResult,
) -> Result<DurableCampaignState, SpawnError> {
    let expected_starting = match role {
        Role::I74 => CampaignState::I74Starting,
        Role::C74 => CampaignState::C74Starting,
    };
    if state.campaign_state != expected_starting
        || state.identity.campaign_id != result.campaign_id
        || state.harness.is_some()
        || state.daemon.is_some()
        || state.checkpoint.is_some()
        || result.schema_version != 1
        || !is_hash(&result.intent_sha256)
        || result.unit_name != deterministic_unit_name(&result.campaign_id, role)?
    {
        return Err(SpawnError::State);
    }
    validate_result_shape(result, &result.unit_name).map_err(|_| SpawnError::State)?;
    let mut next = state.clone();
    next.revision = next.revision.checked_add(1).ok_or(SpawnError::State)?;
    match result.resolution {
        SpawnResolution::Started | SpawnResolution::Adopted => {
            let (harness, daemon) = result.exact_identities().ok_or(SpawnError::State)?;
            next.campaign_state = transition(
                state.campaign_state,
                match role {
                    Role::I74 => Transition::MarkI74Running,
                    Role::C74 => Transition::MarkC74Running,
                },
            )
            .map_err(|_| SpawnError::State)?;
            next.harness = Some(harness.clone());
            next.daemon = Some(daemon.clone());
        }
        SpawnResolution::Absent => {
            next.campaign_state = transition(state.campaign_state, Transition::Fail)
                .map_err(|_| SpawnError::State)?;
            next.recorded_failure = true;
        }
        SpawnResolution::Mismatch => {
            next.campaign_state = transition(state.campaign_state, Transition::Quarantine)
                .map_err(|_| SpawnError::State)?;
            next.recorded_failure = true;
            next.durable_history_corrupt = true;
            next.duplicate_executor = result.mismatch == Some(SpawnMismatch::MultipleExecutors);
        }
    }
    Ok(next)
}

fn result_from_observation(
    intent: &SpawnIntent,
    intent_sha256: &str,
    disposition: IntentDisposition,
    observation: SpawnObservation,
) -> SpawnResult {
    let (resolution, mismatch, harness, daemon) = match observation {
        SpawnObservation::Exact { harness, daemon } => (
            match disposition {
                IntentDisposition::Created => SpawnResolution::Started,
                IntentDisposition::Recovered => SpawnResolution::Adopted,
            },
            None,
            Some(harness),
            Some(daemon),
        ),
        SpawnObservation::Absent => (SpawnResolution::Absent, None, None, None),
        SpawnObservation::Mismatch {
            reason,
            harness,
            daemon,
        } => (SpawnResolution::Mismatch, Some(reason), harness, daemon),
    };
    SpawnResult {
        schema_version: 1,
        campaign_id: intent.campaign_id.clone(),
        intent_sha256: intent_sha256.to_owned(),
        unit_name: intent.unit_name.clone(),
        resolution,
        mismatch,
        harness,
        daemon,
    }
}

fn validate_intent(intent: &SpawnIntent) -> Result<(), SpawnError> {
    if intent.schema_version != 1
        || !is_hash(&intent.campaign_id)
        || intent.request_id.is_empty()
        || intent.request_id.len() > 128
        || !intent
            .request_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        || !is_hash(&intent.request_sha256)
        || !is_hash(&intent.manifest_sha256)
        || !is_hash(&intent.nonce_sha256)
        || intent.unit_name != deterministic_unit_name(&intent.campaign_id, &intent.role)?
    {
        return Err(SpawnError::Document);
    }
    Ok(())
}

fn validate_result(
    result: &SpawnResult,
    intent: &SpawnIntent,
    intent_sha256: &str,
) -> Result<(), SpawnError> {
    if result.schema_version != 1
        || result.campaign_id != intent.campaign_id
        || result.intent_sha256 != intent_sha256
        || result.unit_name != intent.unit_name
    {
        return Err(SpawnError::Document);
    }
    validate_result_shape(result, &intent.unit_name)
}

fn validate_result_shape(result: &SpawnResult, unit_name: &str) -> Result<(), SpawnError> {
    match result.resolution {
        SpawnResolution::Started | SpawnResolution::Adopted => {
            if result.mismatch.is_some() {
                return Err(SpawnError::Document);
            }
            let (Some(harness), Some(daemon)) = (&result.harness, &result.daemon) else {
                return Err(SpawnError::Document);
            };
            if harness.unit_name != unit_name || daemon.unit_name != unit_name {
                return Err(SpawnError::Document);
            }
        }
        SpawnResolution::Absent => {
            if result.mismatch.is_some() || result.harness.is_some() || result.daemon.is_some() {
                return Err(SpawnError::Document);
            }
        }
        SpawnResolution::Mismatch => {
            if result.mismatch.is_none() {
                return Err(SpawnError::Document);
            }
        }
    }
    Ok(())
}

fn canonical_directory(path: &Path) -> Result<PathBuf, SpawnError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(SpawnError::Path);
    }
    Ok(fs::canonicalize(path)?)
}

struct EvidencePaths {
    intent: PathBuf,
    intent_head: PathBuf,
    result: PathBuf,
    result_head: PathBuf,
}

fn evidence_paths(directory: &Path, role: &Role) -> EvidencePaths {
    let role = role_name(role);
    EvidencePaths {
        intent: directory.join(format!("{role}-spawn-intent.json")),
        intent_head: directory.join(format!("{role}-spawn-intent.sha256")),
        result: directory.join(format!("{role}-spawn-result.json")),
        result_head: directory.join(format!("{role}-spawn-result.sha256")),
    }
}

fn role_name(role: &Role) -> &'static str {
    match role {
        Role::I74 => "i74",
        Role::C74 => "c74",
    }
}

fn document_digest<T: Serialize>(value: &T) -> Result<String, SpawnError> {
    Ok(sha256_hex(&canonical_json(value)?))
}

fn read_document_pair<T: DeserializeOwned + Serialize>(
    document_path: &Path,
    head_path: &Path,
) -> Result<T, SpawnError> {
    let bytes = read_bounded_file(document_path, MAX_SPAWN_DOCUMENT_BYTES)?;
    let encoded = bytes.strip_suffix(b"\n").unwrap_or(&bytes);
    if encoded.is_empty() || encoded.contains(&b'\n') || encoded.contains(&b'\r') {
        return Err(SpawnError::Document);
    }
    let mut deserializer = serde_json::Deserializer::from_slice(encoded);
    let document = T::deserialize(&mut deserializer).map_err(|_| SpawnError::Document)?;
    deserializer.end().map_err(|_| SpawnError::Document)?;
    let canonical = canonical_json(&document)?;
    if canonical != encoded {
        return Err(SpawnError::Document);
    }
    let head = read_bounded_file(head_path, 65).map_err(|_| SpawnError::Digest)?;
    let expected = format!("{}\n", sha256_hex(&canonical));
    if head != expected.as_bytes() {
        return Err(SpawnError::Digest);
    }
    Ok(document)
}

fn read_bounded_file(path: &Path, maximum: u64) -> Result<Vec<u8>, SpawnError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() == 0
        || metadata.len() > maximum
        || has_multiple_links(&metadata)
    {
        return Err(SpawnError::Path);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)?
        .take(maximum + 1)
        .read_to_end(&mut bytes)?;
    if bytes.is_empty() || bytes.len() as u64 > maximum {
        return Err(SpawnError::Document);
    }
    Ok(bytes)
}

fn write_document_pair<T: Serialize>(
    document_path: &Path,
    head_path: &Path,
    document: &T,
) -> Result<(), SpawnError> {
    let bytes = canonical_json(document)?;
    if bytes.is_empty() || bytes.len() as u64 > MAX_SPAWN_DOCUMENT_BYTES {
        return Err(SpawnError::Document);
    }
    write_new(document_path, &bytes)?;
    write_new(head_path, format!("{}\n", sha256_hex(&bytes)).as_bytes())?;
    sync_directory(document_path.parent().ok_or(SpawnError::Path)?)?;
    Ok(())
}

#[cfg(unix)]
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), SpawnError> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o400)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), SpawnError> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
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
fn sync_directory(path: &Path) -> Result<(), SpawnError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), SpawnError> {
    Ok(())
}
