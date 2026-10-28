use crate::state::DurableCampaignState;
use fs2::FileExt;
use serde::Deserialize;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const STATE_NAME: &str = "state.json";
pub const PREVIOUS_STATE_NAME: &str = "state.previous.json";
pub const LOCK_NAME: &str = "lock";
pub const MAX_STATE_BYTES: u64 = 65_536;

#[derive(Debug, Error)]
pub enum StateStoreError {
    #[error("campaign identity or state path is invalid")]
    Path,
    #[error("campaign lock is already held")]
    Busy,
    #[error("state document is empty, oversized, malformed, or non-canonical")]
    Document,
    #[error("state revision compare-and-swap failed: expected {expected}, found {actual}")]
    Revision { expected: u64, actual: u64 },
    #[error("state I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("state serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug)]
pub struct CampaignLock {
    file: File,
    campaign_directory: PathBuf,
}

impl CampaignLock {
    pub fn acquire(campaign_root: &Path, campaign_id: &str) -> Result<Self, StateStoreError> {
        validate_campaign_id(campaign_id)?;
        let root = fs::canonicalize(campaign_root)?;
        let campaign_directory = root.join(campaign_id);
        let metadata = fs::symlink_metadata(&campaign_directory)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(StateStoreError::Path);
        }
        let resolved = fs::canonicalize(&campaign_directory)?;
        if resolved.parent() != Some(root.as_path()) {
            return Err(StateStoreError::Path);
        }
        let lock_path = resolved.join(LOCK_NAME);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;
        FileExt::try_lock_exclusive(&file).map_err(map_lock_error)?;
        Ok(Self {
            file,
            campaign_directory: resolved,
        })
    }

    pub fn campaign_directory(&self) -> &Path {
        &self.campaign_directory
    }

    pub fn read(&self) -> Result<DurableCampaignState, StateStoreError> {
        read_state(&self.campaign_directory.join(STATE_NAME))
    }

    pub fn compare_and_swap(
        &self,
        expected_revision: u64,
        next: &DurableCampaignState,
    ) -> Result<(), StateStoreError> {
        let state_path = self.campaign_directory.join(STATE_NAME);
        let current_bytes = read_document(&state_path)?;
        let current = parse_state(&current_bytes)?;
        if current.revision != expected_revision {
            return Err(StateStoreError::Revision {
                expected: expected_revision,
                actual: current.revision,
            });
        }
        if next.revision != expected_revision.saturating_add(1)
            || next.identity.campaign_id != current.identity.campaign_id
            || next.identity.manifest_sha256 != current.identity.manifest_sha256
        {
            return Err(StateStoreError::Revision {
                expected: expected_revision.saturating_add(1),
                actual: next.revision,
            });
        }
        let next_bytes = encode_state(next)?;
        atomic_replace(
            &self.campaign_directory.join(PREVIOUS_STATE_NAME),
            &current_bytes,
        )?;
        atomic_replace(&state_path, &next_bytes)?;
        Ok(())
    }

    pub fn initialize(&self, state: &DurableCampaignState) -> Result<(), StateStoreError> {
        if state.revision != 0 || self.campaign_directory.join(STATE_NAME).exists() {
            return Err(StateStoreError::Revision {
                expected: 0,
                actual: state.revision,
            });
        }
        let bytes = encode_state(state)?;
        atomic_create(&self.campaign_directory.join(STATE_NAME), &bytes)
    }
}

impl Drop for CampaignLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

pub fn read_state(path: &Path) -> Result<DurableCampaignState, StateStoreError> {
    parse_state(&read_document(path)?)
}

fn read_document(path: &Path) -> Result<Vec<u8>, StateStoreError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > MAX_STATE_BYTES
    {
        return Err(StateStoreError::Document);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)?
        .take(MAX_STATE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.is_empty() || bytes.len() as u64 > MAX_STATE_BYTES {
        return Err(StateStoreError::Document);
    }
    Ok(bytes)
}

fn parse_state(bytes: &[u8]) -> Result<DurableCampaignState, StateStoreError> {
    let encoded = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    if encoded.is_empty() || encoded.contains(&b'\n') || encoded.contains(&b'\r') {
        return Err(StateStoreError::Document);
    }
    let mut deserializer = serde_json::Deserializer::from_slice(encoded);
    let state = DurableCampaignState::deserialize(&mut deserializer)
        .map_err(|_| StateStoreError::Document)?;
    deserializer.end().map_err(|_| StateStoreError::Document)?;
    if serde_json::to_vec(&state)? != encoded {
        return Err(StateStoreError::Document);
    }
    Ok(state)
}

fn encode_state(state: &DurableCampaignState) -> Result<Vec<u8>, StateStoreError> {
    let mut bytes = serde_json::to_vec(state)?;
    bytes.push(b'\n');
    if bytes.len() as u64 > MAX_STATE_BYTES {
        return Err(StateStoreError::Document);
    }
    Ok(bytes)
}

fn atomic_create(path: &Path, bytes: &[u8]) -> Result<(), StateStoreError> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    sync_directory(path.parent().ok_or(StateStoreError::Path)?)?;
    Ok(())
}

fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<(), StateStoreError> {
    let parent = path.parent().ok_or(StateStoreError::Path)?;
    let temporary = parent.join(format!(
        ".{}.tmp.{}",
        path.file_name()
            .and_then(|value| value.to_str())
            .ok_or(StateStoreError::Path)?,
        std::process::id()
    ));
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

fn validate_campaign_id(value: &str) -> Result<(), StateStoreError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(StateStoreError::Path)
    }
}

fn map_lock_error(error: std::io::Error) -> StateStoreError {
    if error.kind() == std::io::ErrorKind::WouldBlock
        || matches!(error.raw_os_error(), Some(32 | 33))
    {
        StateStoreError::Busy
    } else {
        StateStoreError::Io(error)
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> Result<(), StateStoreError> {
    fs::rename(source, destination)?;
    Ok(())
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> Result<(), StateStoreError> {
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
fn sync_directory(path: &Path) -> Result<(), StateStoreError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), StateStoreError> {
    Ok(())
}
