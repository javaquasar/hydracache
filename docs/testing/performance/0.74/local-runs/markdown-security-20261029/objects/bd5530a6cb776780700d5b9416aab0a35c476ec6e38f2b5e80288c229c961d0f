//! Read-only local recovery inspection. No replay, cleanup or start capability.
use super::{open_at, open_root, FixtureFence, PreparationError, Stamp};
use crate::diagnostic_lease::{DiagnosticIdentity, DiagnosticStage};
use std::ffi::CString;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileExt, MetadataExt};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FixtureRecoveryError {
    #[error("fixture recovery fence refused: {0}")]
    Fence(#[from] PreparationError),
    #[error("fixture recovery publication or IPC intent is unacknowledged")]
    Uncertain,
    #[error("fixture recovery conflicts with campaign or lifecycle context")]
    Conflict,
    #[error("fixture recovery has no active diagnostic state")]
    Missing,
    #[error("fixture recovery differs from exact asserted identity or original boot")]
    Identity,
    #[error("fixture recovery contains an interrupted execution stage")]
    Interrupted,
    #[error("fixture recovery document metadata, original object or bytes are unsafe")]
    Document,
    #[error("fixture recovery diagnostic state refused: {0}")]
    State(#[from] crate::diagnostic_lease::DiagnosticError),
    #[error("fixture recovery IPC ledger refused: {0}")]
    Ipc(#[from] crate::diagnostic_ipc::DiagnosticIpcError),
}

/// Data only: even a quiet observation is not authorization to start or adopt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixtureRecoveryObservation {
    pub stage: DiagnosticStage,
    pub revision: u64,
}

struct Document {
    name: &'static str,
    file: Option<File>,
    stamp: Option<Stamp>,
    bytes: Vec<u8>,
    maximum: u64,
}
impl Document {
    fn read(
        fence: &FixtureFence,
        name: &'static str,
        maximum: u64,
    ) -> Result<Self, FixtureRecoveryError> {
        let file = match open_at(&fence.root.file, name, libc::O_RDONLY, 0) {
            Err(PreparationError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => None,
            other => Some(other?),
        };
        let (stamp, bytes) = match &file {
            None => (None, Vec::new()),
            Some(file) => {
                let m = file.metadata().map_err(PreparationError::Io)?;
                if !m.is_file()
                    || m.nlink() != 1
                    || m.uid() != fence.uid
                    || m.gid() != fence.gid
                    || m.mode() & 0o7777 != 0o600
                    || m.len() > maximum
                {
                    return Err(FixtureRecoveryError::Document);
                }
                (Some(Stamp::of(&m)), read_bytes(file, maximum)?)
            }
        };
        Ok(Self {
            name,
            file,
            stamp,
            bytes,
            maximum,
        })
    }
    fn revalidate(&self, fence: &FixtureFence) -> Result<(), FixtureRecoveryError> {
        let named = Self::read(fence, self.name, self.maximum)?;
        if self.stamp != named.stamp || self.bytes != named.bytes {
            return Err(FixtureRecoveryError::Document);
        }
        if let Some(original) = &self.file {
            let m = original.metadata().map_err(PreparationError::Io)?;
            if Some(Stamp::of(&m)) != self.stamp
                || m.nlink() != 1
                || read_bytes(original, self.maximum)? != self.bytes
            {
                return Err(FixtureRecoveryError::Document);
            }
        }
        Ok(())
    }
}
fn read_bytes(file: &File, maximum: u64) -> Result<Vec<u8>, FixtureRecoveryError> {
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        let n = file
            .read_at(&mut buffer, bytes.len() as u64)
            .map_err(PreparationError::Io)?;
        if n == 0 {
            return Ok(bytes);
        }
        if bytes.len() as u64 + n as u64 > maximum {
            return Err(FixtureRecoveryError::Document);
        }
        bytes.extend_from_slice(&buffer[..n]);
    }
}
fn exists(parent: &File, name: &str) -> Result<bool, FixtureRecoveryError> {
    let name = CString::new(name).map_err(|_| FixtureRecoveryError::Document)?;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    if unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } == 0
    {
        return Ok(true);
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::NotFound {
        Ok(false)
    } else {
        Err(PreparationError::Io(error).into())
    }
}
fn check_conflicts(fence: &FixtureFence, parent: &File) -> Result<(), FixtureRecoveryError> {
    for name in [".active-diagnostic.pending", ".diagnostic-requests.pending"] {
        if exists(&fence.root.file, name)? {
            return Err(FixtureRecoveryError::Uncertain);
        }
    }
    if exists(&fence.root.file, "active-campaign")? {
        return Err(FixtureRecoveryError::Conflict);
    }
    for name in [
        "campaign-lifecycle-smoke-v1.json",
        "controller-loss-smoke-v1.json",
    ] {
        if exists(parent, name)? {
            return Err(FixtureRecoveryError::Conflict);
        }
    }
    Ok(())
}
pub(super) fn observe(
    fence: &FixtureFence,
    expected: &DiagnosticIdentity,
    checkpoint: impl FnOnce(&FixtureFence),
) -> Result<FixtureRecoveryObservation, FixtureRecoveryError> {
    let parent_path = fence.path.parent().ok_or(FixtureRecoveryError::Document)?;
    let parent = open_root(parent_path)?;
    let parent_stamp = Stamp::of(&parent.metadata().map_err(PreparationError::Io)?);
    check_conflicts(fence, &parent)?;
    let active = Document::read(fence, "active-diagnostic.json", 65_536)?;
    let ledger = Document::read(
        fence,
        "diagnostic-requests-v1.json",
        crate::diagnostic_ipc::MAX_LEDGER_BYTES,
    )?;
    checkpoint(fence);
    fence.revalidate()?;
    let named_parent = open_root(parent_path)?;
    if Stamp::of(&named_parent.metadata().map_err(PreparationError::Io)?) != parent_stamp
        || Stamp::of(&parent.metadata().map_err(PreparationError::Io)?) != parent_stamp
    {
        return Err(FixtureRecoveryError::Document);
    }
    active.revalidate(fence)?;
    ledger.revalidate(fence)?;
    check_conflicts(fence, &parent)?;
    if ledger.file.is_some() && crate::diagnostic_ipc::has_unacknowledged_intent(&ledger.bytes)? {
        return Err(FixtureRecoveryError::Uncertain);
    }
    if active.file.is_none() {
        return Err(FixtureRecoveryError::Missing);
    }
    let state = crate::diagnostic_lease::decode_active(&active.bytes)?;
    if &state.identity != expected {
        return Err(FixtureRecoveryError::Identity);
    }
    if !matches!(
        state.stage,
        DiagnosticStage::Reserved | DiagnosticStage::Terminal
    ) {
        return Err(FixtureRecoveryError::Interrupted);
    }
    Ok(FixtureRecoveryObservation {
        stage: state.stage,
        revision: state.revision,
    })
}
fn inspect_inner<R>(
    root: &Path,
    expected: &DiagnosticIdentity,
    checkpoint: impl FnOnce(&FixtureFence),
    receive: impl FnOnce(Result<FixtureRecoveryObservation, FixtureRecoveryError>) -> R,
) -> R {
    let fence = match FixtureFence::acquire(root) {
        Ok(fence) => fence,
        Err(error) => return receive(Err(error.into())),
    };
    let result = observe(&fence, expected, checkpoint);
    // Keep the original cooperative host fence through the refusal callback.
    // Drop releases it on both normal and error exits; no cleanup or replay.
    receive(result)
}
pub(crate) fn with_fixture_recovery<R>(
    root: &Path,
    expected: &DiagnosticIdentity,
    receive: impl FnOnce(Result<FixtureRecoveryObservation, FixtureRecoveryError>) -> R,
) -> R {
    inspect_inner(root, expected, |_| {}, receive)
}
pub fn inspect_fixture_recovery(
    root: &Path,
    expected: &DiagnosticIdentity,
) -> Result<FixtureRecoveryObservation, FixtureRecoveryError> {
    with_fixture_recovery(root, expected, |result| result)
}

#[cfg(test)]
#[path = "diagnostic_fixture_recovery_tests.rs"]
pub(crate) mod tests;
