use crate::state::{CampaignState, DurableCampaignState};
use fs2::FileExt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const HOST_EXECUTION_LOCK_NAME: &str = ".host-execution.lock";
pub const ACTIVE_CAMPAIGN_NAME: &str = "active-campaign";

/// Observe the fixed host-wide claim without creating, recovering, releasing,
/// or otherwise mutating it. Callers must bracket the work they are guarding.
pub fn active_campaign_absent(campaign_root: &Path) -> Result<bool, HostExecutionError> {
    let metadata = fs::symlink_metadata(campaign_root)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(HostExecutionError::Path);
    }
    let root = fs::canonicalize(campaign_root)?;
    if diagnostic_present(&root)? {
        return Ok(false);
    }
    let marker = root.join(ACTIVE_CAMPAIGN_NAME);
    match fs::symlink_metadata(&marker) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Ok(_) => {
            read_marker(&marker)?;
            Ok(false)
        }
        Err(error) => Err(error.into()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimDisposition {
    Created,
    Recovered,
}

#[derive(Debug, Error)]
pub enum HostExecutionError {
    #[error("host execution root, lock, marker, or campaign identity is unsafe")]
    Path,
    #[error("host execution lock is already held")]
    Busy,
    #[error("host is already claimed by campaign {campaign_id}")]
    Conflict { campaign_id: String },
    #[error("host is reserved by a diagnostic lease")]
    DiagnosticConflict,
    #[error("host execution I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct HostExecutionClaim {
    lock: File,
    root: PathBuf,
    campaign_id: String,
    disposition: ClaimDisposition,
}

impl HostExecutionClaim {
    pub fn acquire(campaign_root: &Path, campaign_id: &str) -> Result<Self, HostExecutionError> {
        Self::acquire_inner(campaign_root, campaign_id, true)
    }

    pub fn recover(campaign_root: &Path, campaign_id: &str) -> Result<Self, HostExecutionError> {
        Self::acquire_inner(campaign_root, campaign_id, false)
    }

    pub fn recover_active(campaign_root: &Path) -> Result<Option<Self>, HostExecutionError> {
        let metadata = fs::symlink_metadata(campaign_root)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(HostExecutionError::Path);
        }
        let root = fs::canonicalize(campaign_root)?;
        let lock = open_lock(&root.join(HOST_EXECUTION_LOCK_NAME))?;
        FileExt::try_lock_exclusive(&lock).map_err(map_lock_error)?;
        if diagnostic_present(&root)? {
            if fs::symlink_metadata(root.join(ACTIVE_CAMPAIGN_NAME)).is_ok() {
                return Err(HostExecutionError::Path);
            }
            return Ok(None);
        }
        let marker = root.join(ACTIVE_CAMPAIGN_NAME);
        let campaign_id = match fs::symlink_metadata(&marker) {
            Ok(_) => read_marker(&marker)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        Ok(Some(Self {
            lock,
            root,
            campaign_id,
            disposition: ClaimDisposition::Recovered,
        }))
    }

    fn acquire_inner(
        campaign_root: &Path,
        campaign_id: &str,
        create_if_missing: bool,
    ) -> Result<Self, HostExecutionError> {
        if !is_hash(campaign_id) {
            return Err(HostExecutionError::Path);
        }
        let metadata = fs::symlink_metadata(campaign_root)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(HostExecutionError::Path);
        }
        let root = fs::canonicalize(campaign_root)?;
        let lock_path = root.join(HOST_EXECUTION_LOCK_NAME);
        let lock = open_lock(&lock_path)?;
        FileExt::try_lock_exclusive(&lock).map_err(map_lock_error)?;
        if diagnostic_present(&root)? {
            return Err(HostExecutionError::DiagnosticConflict);
        }

        let marker = root.join(ACTIVE_CAMPAIGN_NAME);
        let disposition = match fs::symlink_metadata(&marker) {
            Ok(_) => {
                let active = read_marker(&marker)?;
                if active != campaign_id {
                    return Err(HostExecutionError::Conflict {
                        campaign_id: active,
                    });
                }
                ClaimDisposition::Recovered
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && create_if_missing => {
                create_marker(&marker, campaign_id)?;
                sync_directory(&root)?;
                ClaimDisposition::Created
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(HostExecutionError::Path)
            }
            Err(error) => return Err(error.into()),
        };
        Ok(Self {
            lock,
            root,
            campaign_id: campaign_id.to_owned(),
            disposition,
        })
    }

    pub fn campaign_id(&self) -> &str {
        &self.campaign_id
    }

    pub fn disposition(&self) -> ClaimDisposition {
        self.disposition
    }

    pub fn campaign_root(&self) -> &Path {
        &self.root
    }

    pub fn release_after_complete_seal(
        &self,
        state: &DurableCampaignState,
    ) -> Result<(), HostExecutionError> {
        if state.campaign_state != CampaignState::CompleteSealed {
            return Err(HostExecutionError::Path);
        }
        self.release_after_terminal(state)
    }

    pub fn release_after_terminal(
        &self,
        state: &DurableCampaignState,
    ) -> Result<(), HostExecutionError> {
        if !matches!(
            state.campaign_state,
            CampaignState::CompleteSealed
                | CampaignState::FailedIncomplete
                | CampaignState::AbortedIncomplete
                | CampaignState::LeaseExpiredIncomplete
        ) || state.identity.campaign_id != self.campaign_id
            || state.harness.is_some()
            || state.daemon.is_some()
            || state.checkpoint.is_some()
            || state.controller_lease.is_some()
        {
            return Err(HostExecutionError::Path);
        }
        let marker = self.root.join(ACTIVE_CAMPAIGN_NAME);
        match fs::symlink_metadata(&marker) {
            Ok(_) => {
                if read_marker(&marker)? != self.campaign_id {
                    return Err(HostExecutionError::Path);
                }
                fs::remove_file(marker)?;
                sync_directory(&self.root)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        Ok(())
    }
}

/// Only supervisor-owned diagnostic transactions use this fence. It must not
/// be retained while a workload runs; the durable typed marker owns that lease.
pub(crate) fn lock_host_root(campaign_root: &Path) -> Result<(PathBuf, File), HostExecutionError> {
    let metadata = fs::symlink_metadata(campaign_root)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(HostExecutionError::Path);
    }
    let root = fs::canonicalize(campaign_root)?;
    let lock = open_lock(&root.join(HOST_EXECUTION_LOCK_NAME))?;
    FileExt::try_lock_exclusive(&lock).map_err(map_lock_error)?;
    Ok((root, lock))
}

fn diagnostic_present(root: &Path) -> Result<bool, HostExecutionError> {
    crate::diagnostic_lease::read_active(root)
        .map(|state| state.is_some())
        .map_err(|_| HostExecutionError::Path)
}

impl Drop for HostExecutionClaim {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.lock);
    }
}

fn open_lock(path: &Path) -> Result<File, HostExecutionError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => validate_lock_metadata(&metadata)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(path)?;
            file.sync_all()?;
            return Ok(file);
        }
        Err(error) => return Err(error.into()),
    }
    let file = OpenOptions::new().read(true).write(true).open(path)?;
    let metadata = file.metadata()?;
    validate_lock_metadata(&metadata)?;
    Ok(file)
}

fn validate_lock_metadata(metadata: &fs::Metadata) -> Result<(), HostExecutionError> {
    if !metadata.is_file() || metadata.file_type().is_symlink() || has_multiple_links(metadata) {
        Err(HostExecutionError::Path)
    } else {
        Ok(())
    }
}

fn read_marker(path: &Path) -> Result<String, HostExecutionError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() != 65
        || has_multiple_links(&metadata)
    {
        return Err(HostExecutionError::Path);
    }
    let mut bytes = Vec::with_capacity(65);
    File::open(path)?.take(66).read_to_end(&mut bytes)?;
    let value = bytes
        .strip_suffix(b"\n")
        .and_then(|value| std::str::from_utf8(value).ok())
        .filter(|value| is_hash(value))
        .ok_or(HostExecutionError::Path)?;
    Ok(value.to_owned())
}

#[cfg(unix)]
fn create_marker(path: &Path, campaign_id: &str) -> Result<(), HostExecutionError> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o400)
        .open(path)?;
    file.write_all(format!("{campaign_id}\n").as_bytes())?;
    file.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn create_marker(path: &Path, campaign_id: &str) -> Result<(), HostExecutionError> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(format!("{campaign_id}\n").as_bytes())?;
    file.sync_all()?;
    Ok(())
}

fn is_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn map_lock_error(error: std::io::Error) -> HostExecutionError {
    if error.kind() == std::io::ErrorKind::WouldBlock
        || matches!(error.raw_os_error(), Some(32 | 33))
    {
        HostExecutionError::Busy
    } else {
        HostExecutionError::Io(error)
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
fn sync_directory(path: &Path) -> Result<(), HostExecutionError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), HostExecutionError> {
    Ok(())
}
