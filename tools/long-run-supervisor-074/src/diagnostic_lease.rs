//! Local-only diagnostic reservation/coordinator. No production CLI, IPC or
//! live execution backend is enrolled. Backend observations are a test seam,
//! not authentication, build provenance or independently verified host evidence.

use crate::host_execution::{lock_host_root, HostExecutionError, ACTIVE_CAMPAIGN_NAME};
use crate::{canonical_json, is_hash, sha256_hex};
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const ACTIVE_DIAGNOSTIC_NAME: &str = "active-diagnostic.json";
const PENDING_NAME: &str = ".active-diagnostic.pending";
const MAX_DOCUMENT_BYTES: u64 = 65_536;
pub const CELL_SECONDS: u64 = 60;
pub const TOTAL_SECONDS: u64 = 300;
pub const CONTROLLER_LOSS_SECONDS: u64 = 10;
pub const MAX_RECEIPT_BYTES: u64 = 16_777_216;
const NS: u64 = 1_000_000_000;
pub const SOURCE_COMMIT: &str = "62114be0f5da3218706e30d7424acfb5d0579d07";
pub const BINARY_PATH: &str =
    "/opt/hydracache-performance/0.74/diagnostic-pilot/timing-controls-074";
const SURFACES: [&str; 4] = ["embedded", "direct", "resp2", "resp3"];
const CONFIG_HASHES: [&str; 4] = [
    "5c219730c8a782ed47dfa0b291a29f44f753c3688f8548c136174a89cee55149",
    "5485b10f50906833006ad3f3a40b4909760ef01e3f2c86395e4366bf83cc6ef9",
    "f0aba16dfe0354801d0b3c83ee9fc2d1b4751be5f50280634690ed4bcf3030ae",
    "b680683c291d0a4dadbac1901f051ba0fedcac0ac9127d197e0261fe6fdabdd7",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticIdentity {
    pub lease_id: String,
    pub boot_id: String,
    pub binary_sha256: String,
    /// Identity of a future trusted build receipt, not provenance proved here.
    pub build_provenance_sha256: String,
}

#[derive(Debug, Clone)]
pub struct DiagnosticClock {
    pub boot_id: String,
    pub monotonic_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticStage {
    Reserved,
    Starting,
    Running,
    Stopping,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticStopReason {
    Completed,
    ControllerLost,
    CellDeadline,
    TotalDeadline,
    ReceiptOverflow,
    InvalidReceipt,
    InterruptedStart,
    OperatorCancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticState {
    pub identity: DiagnosticIdentity,
    pub revision: u64,
    pub stage: DiagnosticStage,
    pub completed_cells: usize,
    pub reserved_monotonic_ns: u64,
    pub last_observed_monotonic_ns: u64,
    pub controller_monotonic_ns: u64,
    pub cell_started_monotonic_ns: Option<u64>,
    pub cgroup_inode: Option<u64>,
    pub reason: Option<DiagnosticStopReason>,
    pub cleanup_confirmed: bool,
    pub promotable: bool,
    pub admission_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellIntent {
    pub lease_id: String,
    pub boot_id: String,
    pub surface: String,
    pub unit_name: String,
    pub cgroup_path: String,
    pub binary_path: String,
    pub binary_sha256: String,
    pub config_path: String,
    pub config_sha256: String,
    pub source_commit: String,
    pub maximum_runtime_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct CellOutcome {
    pub successful: bool,
    pub valid: bool,
}

#[derive(Debug, Clone)]
pub struct TreeObservation {
    pub unit_name: String,
    pub boot_id: String,
    pub cgroup_path: String,
    pub cgroup_inode: u64,
    /// True if *any* process in the entire recursive owned cgroup remains.
    pub populated: bool,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub receipts_retained: bool,
    pub outcome: Option<CellOutcome>,
}

pub trait DiagnosticBackend {
    fn start_once(&mut self, intent: &CellIntent) -> Result<(), String>;
    fn observe_tree(&mut self, intent: &CellIntent) -> Result<TreeObservation, String>;
    fn stop_tree(&mut self, intent: &CellIntent) -> Result<(), String>;
}

#[derive(Debug, Error)]
pub enum DiagnosticError {
    #[error("diagnostic lease document, path, state or identity is unsafe")]
    Invalid,
    #[error("host campaign, fixture, diagnostic lease or terminal identity already exists")]
    Conflict,
    #[error("diagnostic boot identity or monotonic clock drifted")]
    Clock,
    #[error("exact empty cgroup and retained receipts were not proved")]
    Cleanup,
    #[error("diagnostic backend failed: {0}")]
    Backend(String),
    #[error("diagnostic host transaction failed: {0}")]
    Host(#[from] HostExecutionError),
    #[error("diagnostic I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("diagnostic document failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    schema_version: u32,
    state: DiagnosticState,
    state_sha256: String,
}

pub struct DiagnosticCoordinator {
    root: PathBuf,
    lease_id: String,
}

impl DiagnosticCoordinator {
    pub fn reserve(
        root: &Path,
        identity: DiagnosticIdentity,
        now: &DiagnosticClock,
    ) -> Result<Self, DiagnosticError> {
        validate_identity(&identity)?;
        if now.boot_id != identity.boot_id {
            return Err(DiagnosticError::Clock);
        }
        let (root, _fence) = lock_host_root(root)?;
        Self::reserve_fenced(&root, identity, now)
    }

    /// Caller must hold the shared host fence throughout this transaction.
    pub(crate) fn reserve_fenced(
        root: &Path,
        identity: DiagnosticIdentity,
        now: &DiagnosticClock,
    ) -> Result<Self, DiagnosticError> {
        validate_identity(&identity)?;
        if now.boot_id != identity.boot_id {
            return Err(DiagnosticError::Clock);
        }
        ensure_absent(&root.join(ACTIVE_CAMPAIGN_NAME))?;
        ensure_absent(&root.join(ACTIVE_DIAGNOSTIC_NAME))?;
        ensure_absent(&root.join(PENDING_NAME))?;
        ensure_absent(&terminal_path(root, &identity.lease_id))?;
        let parent = root.parent().ok_or(DiagnosticError::Invalid)?;
        for name in [
            "campaign-lifecycle-smoke-v1.json",
            "controller-loss-smoke-v1.json",
        ] {
            ensure_absent(&parent.join(name))?;
        }
        let state = DiagnosticState {
            identity: identity.clone(),
            revision: 1,
            stage: DiagnosticStage::Reserved,
            completed_cells: 0,
            reserved_monotonic_ns: now.monotonic_ns,
            last_observed_monotonic_ns: now.monotonic_ns,
            controller_monotonic_ns: now.monotonic_ns,
            cell_started_monotonic_ns: None,
            cgroup_inode: None,
            reason: None,
            cleanup_confirmed: true,
            promotable: false,
            admission_allowed: false,
        };
        publish(root, &state)?;
        Ok(Self {
            root: root.to_owned(),
            lease_id: identity.lease_id,
        })
    }

    pub fn recover(root: &Path, lease_id: &str) -> Result<Self, DiagnosticError> {
        if !is_hash(lease_id) {
            return Err(DiagnosticError::Invalid);
        }
        let (root, _fence) = lock_host_root(root)?;
        Self::recover_fenced(&root, lease_id)
    }

    pub(crate) fn recover_fenced(root: &Path, lease_id: &str) -> Result<Self, DiagnosticError> {
        let state = read_active(root)?.ok_or(DiagnosticError::Invalid)?;
        if state.identity.lease_id != lease_id {
            return Err(DiagnosticError::Invalid);
        }
        Ok(Self {
            root: root.to_owned(),
            lease_id: lease_id.to_owned(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn state(&self) -> Result<DiagnosticState, DiagnosticError> {
        let (_root, _fence) = lock_host_root(&self.root)?;
        self.load()
    }

    fn load(&self) -> Result<DiagnosticState, DiagnosticError> {
        let state = read_active(&self.root)?.ok_or(DiagnosticError::Invalid)?;
        if state.identity.lease_id != self.lease_id {
            return Err(DiagnosticError::Invalid);
        }
        Ok(state)
    }

    /// Direct local model API. The separate local IPC handler authenticates and
    /// journals requests before calling the fenced form; no production route.
    pub fn heartbeat(&self, now: &DiagnosticClock) -> Result<(), DiagnosticError> {
        let (_root, _fence) = lock_host_root(&self.root)?;
        self.heartbeat_fenced(now).map(|_| ())
    }

    pub(crate) fn heartbeat_fenced(
        &self,
        now: &DiagnosticClock,
    ) -> Result<DiagnosticState, DiagnosticError> {
        let mut state = self.load()?;
        check_clock(&state, now)?;
        // A lost controller cannot resurrect its lease by arriving late.
        if now.monotonic_ns - state.controller_monotonic_ns >= CONTROLLER_LOSS_SECONDS * NS
            || state.stage == DiagnosticStage::Terminal
        {
            return Err(DiagnosticError::Clock);
        }
        state.controller_monotonic_ns = now.monotonic_ns;
        state.last_observed_monotonic_ns = now.monotonic_ns;
        bump_publish(&self.root, &mut state)?;
        Ok(state)
    }

    pub fn cancel<B: DiagnosticBackend>(
        &self,
        now: &DiagnosticClock,
        backend: &mut B,
    ) -> Result<DiagnosticState, DiagnosticError> {
        self.drive(now, backend, true)
    }

    pub fn advance<B: DiagnosticBackend>(
        &self,
        now: &DiagnosticClock,
        backend: &mut B,
    ) -> Result<DiagnosticState, DiagnosticError> {
        self.drive(now, backend, false)
    }

    fn drive<B: DiagnosticBackend>(
        &self,
        now: &DiagnosticClock,
        backend: &mut B,
        cancel: bool,
    ) -> Result<DiagnosticState, DiagnosticError> {
        let (_root, _fence) = lock_host_root(&self.root)?;
        self.drive_fenced(now, backend, cancel)
    }

    pub(crate) fn drive_fenced<B: DiagnosticBackend>(
        &self,
        now: &DiagnosticClock,
        backend: &mut B,
        cancel: bool,
    ) -> Result<DiagnosticState, DiagnosticError> {
        let mut state = self.load()?;
        check_clock(&state, now)?;
        if state.stage == DiagnosticStage::Terminal {
            self.finish(&state)?;
            return Ok(state);
        }
        state.last_observed_monotonic_ns = now.monotonic_ns;
        let due = if now.monotonic_ns - state.reserved_monotonic_ns >= TOTAL_SECONDS * NS {
            Some(DiagnosticStopReason::TotalDeadline)
        } else if now.monotonic_ns - state.controller_monotonic_ns >= CONTROLLER_LOSS_SECONDS * NS {
            Some(DiagnosticStopReason::ControllerLost)
        } else if cancel {
            Some(DiagnosticStopReason::OperatorCancelled)
        } else if state
            .cell_started_monotonic_ns
            .is_some_and(|start| now.monotonic_ns - start >= CELL_SECONDS * NS)
        {
            Some(DiagnosticStopReason::CellDeadline)
        } else {
            None
        };
        if state.reason.is_none() {
            if let Some(reason) = due {
                state.reason = Some(reason);
            } else if state.stage == DiagnosticStage::Starting {
                state.reason = Some(DiagnosticStopReason::InterruptedStart);
            }
        }
        if state.reason.is_some() {
            if state.stage == DiagnosticStage::Reserved {
                return self.terminal(state);
            }
            return self.stop(state, backend);
        }
        if state.stage == DiagnosticStage::Reserved {
            let intent = cell_intent(&state)?;
            state.stage = DiagnosticStage::Starting;
            state.cleanup_confirmed = false;
            state.cell_started_monotonic_ns = Some(now.monotonic_ns);
            bump_publish(&self.root, &mut state)?;
            backend
                .start_once(&intent)
                .map_err(DiagnosticError::Backend)?;
            state.stage = DiagnosticStage::Running;
            bump_publish(&self.root, &mut state)?;
            return Ok(state);
        }
        let intent = cell_intent(&state)?;
        let observed = backend
            .observe_tree(&intent)
            .map_err(DiagnosticError::Backend)?;
        bind_tree(&mut state, &intent, &observed)?;
        if observed
            .stdout_bytes
            .checked_add(observed.stderr_bytes)
            .is_none_or(|bytes| bytes > MAX_RECEIPT_BYTES)
        {
            state.reason = Some(DiagnosticStopReason::ReceiptOverflow);
            return self.stop(state, backend);
        }
        if !observed.populated {
            if !observed.receipts_retained {
                return Err(DiagnosticError::Cleanup);
            }
            if !observed
                .outcome
                .is_some_and(|outcome| outcome.successful && outcome.valid)
            {
                state.reason = Some(DiagnosticStopReason::InvalidReceipt);
                return self.stop(state, backend);
            }
            state.cleanup_confirmed = true;
            state.completed_cells += 1;
            if state.completed_cells == 4 {
                state.reason = Some(DiagnosticStopReason::Completed);
                return self.terminal(state);
            }
            state.stage = DiagnosticStage::Reserved;
            state.cell_started_monotonic_ns = None;
            state.cgroup_inode = None;
        }
        bump_publish(&self.root, &mut state)?;
        Ok(state)
    }

    fn stop<B: DiagnosticBackend>(
        &self,
        mut state: DiagnosticState,
        backend: &mut B,
    ) -> Result<DiagnosticState, DiagnosticError> {
        state.stage = DiagnosticStage::Stopping;
        bump_publish(&self.root, &mut state)?;
        let intent = cell_intent(&state)?;
        let before = backend
            .observe_tree(&intent)
            .map_err(DiagnosticError::Backend)?;
        bind_tree(&mut state, &intent, &before)?;
        // Persist the inode before a destructive action, so restart cannot adopt
        // a replacement cgroup with the same unit name.
        bump_publish(&self.root, &mut state)?;
        if before.populated {
            backend
                .stop_tree(&intent)
                .map_err(DiagnosticError::Backend)?;
        }
        let after = backend
            .observe_tree(&intent)
            .map_err(DiagnosticError::Backend)?;
        bind_tree(&mut state, &intent, &after)?;
        if after.populated || !after.receipts_retained {
            return Err(DiagnosticError::Cleanup);
        }
        state.cleanup_confirmed = true;
        self.terminal(state)
    }

    fn terminal(&self, mut state: DiagnosticState) -> Result<DiagnosticState, DiagnosticError> {
        if !state.cleanup_confirmed {
            return Err(DiagnosticError::Cleanup);
        }
        state.stage = DiagnosticStage::Terminal;
        bump_publish(&self.root, &mut state)?;
        self.finish(&state)?;
        Ok(state)
    }

    fn finish(&self, state: &DiagnosticState) -> Result<(), DiagnosticError> {
        if state.stage != DiagnosticStage::Terminal
            || !state.cleanup_confirmed
            || state.reason.is_none()
        {
            return Err(DiagnosticError::Cleanup);
        }
        let bytes = encode(state)?;
        let receipt = terminal_path(&self.root, &state.identity.lease_id);
        match fs::symlink_metadata(&receipt) {
            Ok(_) => {
                if read_document(&receipt)? != bytes {
                    return Err(DiagnosticError::Invalid);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                write_new(&receipt, &bytes)?
            }
            Err(error) => return Err(error.into()),
        }
        sync_directory(&self.root)?;
        fs::remove_file(self.root.join(ACTIVE_DIAGNOSTIC_NAME))?;
        sync_directory(&self.root)?;
        Ok(())
    }
}

pub fn cell_intent(state: &DiagnosticState) -> Result<CellIntent, DiagnosticError> {
    validate_state(state)?;
    let index = if state.completed_cells == 4 {
        3
    } else {
        state.completed_cells
    };
    let surface = SURFACES[index];
    let unit_name = format!(
        "hydracache-diagnostic-074-{}-{}.service",
        state.identity.lease_id,
        index + 1
    );
    Ok(CellIntent {
        lease_id: state.identity.lease_id.clone(),
        boot_id: state.identity.boot_id.clone(),
        surface: surface.into(),
        cgroup_path: format!("/system.slice/{unit_name}"),
        unit_name,
        binary_path: BINARY_PATH.into(),
        binary_sha256: state.identity.binary_sha256.clone(),
        config_path: format!("/opt/hydracache-performance/0.74/diagnostic-pilot/{surface}.json"),
        config_sha256: CONFIG_HASHES[index].into(),
        source_commit: SOURCE_COMMIT.into(),
        maximum_runtime_seconds: CELL_SECONDS.min(
            (TOTAL_SECONDS * NS).saturating_sub(
                state
                    .cell_started_monotonic_ns
                    .unwrap_or(state.last_observed_monotonic_ns)
                    - state.reserved_monotonic_ns,
            ) / NS,
        ),
    })
}

fn bind_tree(
    state: &mut DiagnosticState,
    intent: &CellIntent,
    observation: &TreeObservation,
) -> Result<(), DiagnosticError> {
    if observation.unit_name != intent.unit_name
        || observation.boot_id != intent.boot_id
        || observation.cgroup_path != intent.cgroup_path
        || observation.cgroup_inode == 0
        || state
            .cgroup_inode
            .is_some_and(|inode| inode != observation.cgroup_inode)
    {
        return Err(DiagnosticError::Cleanup);
    }
    state.cgroup_inode = Some(observation.cgroup_inode);
    Ok(())
}

fn check_clock(state: &DiagnosticState, now: &DiagnosticClock) -> Result<(), DiagnosticError> {
    if now.boot_id != state.identity.boot_id || now.monotonic_ns < state.last_observed_monotonic_ns
    {
        Err(DiagnosticError::Clock)
    } else {
        Ok(())
    }
}

pub(crate) fn validate_identity(identity: &DiagnosticIdentity) -> Result<(), DiagnosticError> {
    if !is_hash(&identity.lease_id)
        || !is_hash(&identity.binary_sha256)
        || !is_hash(&identity.build_provenance_sha256)
        || identity.boot_id.len() != 36
        || !identity.boot_id.bytes().enumerate().all(|(i, byte)| {
            if [8, 13, 18, 23].contains(&i) {
                byte == b'-'
            } else {
                byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
            }
        })
    {
        return Err(DiagnosticError::Invalid);
    }
    Ok(())
}

pub(crate) fn validate_state(state: &DiagnosticState) -> Result<(), DiagnosticError> {
    validate_identity(&state.identity)?;
    if state.revision == 0
        || state.completed_cells > 4
        || state.promotable
        || state.admission_allowed
        || state.controller_monotonic_ns < state.reserved_monotonic_ns
        || state.last_observed_monotonic_ns < state.controller_monotonic_ns
        || state.cell_started_monotonic_ns.is_some_and(|start| {
            start < state.reserved_monotonic_ns || start > state.last_observed_monotonic_ns
        })
        || state.cgroup_inode == Some(0)
        || (matches!(
            state.stage,
            DiagnosticStage::Starting | DiagnosticStage::Running | DiagnosticStage::Stopping
        ) && state.cleanup_confirmed)
        || (state.completed_cells == 4 && state.stage != DiagnosticStage::Terminal)
        || (matches!(
            state.stage,
            DiagnosticStage::Starting | DiagnosticStage::Running | DiagnosticStage::Stopping
        ) && state.cell_started_monotonic_ns.is_none())
        || (state.stage == DiagnosticStage::Reserved
            && (state.cell_started_monotonic_ns.is_some()
                || state.cgroup_inode.is_some()
                || !state.cleanup_confirmed
                || state.reason.is_some()))
        || (state.stage == DiagnosticStage::Stopping && state.reason.is_none())
        || (state.stage == DiagnosticStage::Terminal
            && (!state.cleanup_confirmed || state.reason.is_none()))
        || (state.reason == Some(DiagnosticStopReason::Completed) && state.completed_cells != 4)
        || (state.completed_cells == 4 && state.reason != Some(DiagnosticStopReason::Completed))
    {
        return Err(DiagnosticError::Invalid);
    }
    Ok(())
}

pub(crate) fn read_active(root: &Path) -> Result<Option<DiagnosticState>, DiagnosticError> {
    ensure_absent(&root.join(PENDING_NAME))?;
    let path = root.join(ACTIVE_DIAGNOSTIC_NAME);
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
        Ok(_) => {}
    }
    ensure_absent(&root.join(ACTIVE_CAMPAIGN_NAME))?;
    let bytes = read_document(&path)?;
    let envelope: Envelope = serde_json::from_slice(&bytes)?;
    validate_state(&envelope.state)?;
    if envelope.schema_version != 1
        || envelope.state_sha256
            != sha256_hex(&canonical_json(&envelope.state).map_err(|_| DiagnosticError::Invalid)?)
        || encode(&envelope.state)? != bytes
    {
        return Err(DiagnosticError::Invalid);
    }
    Ok(Some(envelope.state))
}

fn encode(state: &DiagnosticState) -> Result<Vec<u8>, DiagnosticError> {
    validate_state(state)?;
    let envelope = Envelope {
        schema_version: 1,
        state: state.clone(),
        state_sha256: sha256_hex(&canonical_json(state).map_err(|_| DiagnosticError::Invalid)?),
    };
    let mut bytes = canonical_json(&envelope).map_err(|_| DiagnosticError::Invalid)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn bump_publish(root: &Path, state: &mut DiagnosticState) -> Result<(), DiagnosticError> {
    state.revision = state
        .revision
        .checked_add(1)
        .ok_or(DiagnosticError::Invalid)?;
    publish(root, state)
}

fn publish(root: &Path, state: &DiagnosticState) -> Result<(), DiagnosticError> {
    let bytes = encode(state)?;
    let pending = root.join(PENDING_NAME);
    write_new(&pending, &bytes)?;
    fs::rename(&pending, root.join(ACTIVE_DIAGNOSTIC_NAME))?;
    sync_directory(root)
}

fn ensure_absent(path: &Path) -> Result<(), DiagnosticError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
        Ok(_) => Err(DiagnosticError::Conflict),
    }
}

fn terminal_path(root: &Path, id: &str) -> PathBuf {
    root.join(format!("diagnostic-{id}.terminal.json"))
}

pub(crate) fn read_document(path: &Path) -> Result<Vec<u8>, DiagnosticError> {
    read_bounded_document(path, MAX_DOCUMENT_BYTES)
}

pub(crate) fn read_bounded_document(path: &Path, maximum: u64) -> Result<Vec<u8>, DiagnosticError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > maximum {
        return Err(DiagnosticError::Invalid);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err(DiagnosticError::Invalid);
        }
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(path)?;
    let opened = file.metadata()?;
    if !opened.is_file() || opened.len() > maximum {
        return Err(DiagnosticError::Invalid);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if opened.nlink() != 1 || opened.ino() != metadata.ino() || opened.dev() != metadata.dev() {
            return Err(DiagnosticError::Invalid);
        }
    }
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum {
        return Err(DiagnosticError::Invalid);
    }
    Ok(bytes)
}

pub(crate) fn write_new(path: &Path, bytes: &[u8]) -> Result<(), DiagnosticError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(unix)]
pub(crate) fn sync_directory(root: &Path) -> Result<(), DiagnosticError> {
    File::open(root)?.sync_all()?;
    Ok(())
}
#[cfg(not(unix))]
pub(crate) fn sync_directory(_root: &Path) -> Result<(), DiagnosticError> {
    Ok(())
}
