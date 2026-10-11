//! Original fixed state observation only; no fence, publication or start.
use super::{open_at, FixedIntentStorageError, FixedIntentStorageRead, StorageLeaf};
use crate::diagnostic_lease::{
    decode_active, validate_state, DiagnosticStage, DiagnosticState, ACTIVE_DIAGNOSTIC_NAME,
};
use std::ffi::CString;
use std::fs::{File, Metadata};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileExt, MetadataExt};
use thiserror::Error;

const MAX_BYTES: u64 = 65_536;
const CONFLICTS: [&str; 4] = [
    ".active-diagnostic.pending",
    ".diagnostic-requests.pending",
    "active-campaign",
    "diagnostic-requests-v1.json",
];

#[derive(Debug, Error)]
pub enum FixedReservedStateError {
    #[error("original fixed storage refused: {0}")]
    Storage(#[from] FixedIntentStorageError),
    #[error("expected state is not a valid Reserved snapshot")]
    Expected,
    #[error("fixed state document, original object, bytes or metadata refused")]
    Document,
    #[error("fixed state has pending or unhandled local context")]
    Conflict,
    #[error("original fixed state/storage binding permanently refused")]
    Refused,
    #[error("fixed state open refused: {0}")]
    Open(#[from] super::super::PreparationError),
    #[error("fixed state I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(PartialEq, Eq)]
struct Stamp {
    device: u64,
    inode: u64,
    uid: u32,
    gid: u32,
    mode: u32,
    links: u64,
    len: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
}
impl Stamp {
    fn of(m: &Metadata) -> Self {
        Self {
            device: m.dev(),
            inode: m.ino(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode(),
            links: m.nlink(),
            len: m.len(),
            mtime: m.mtime(),
            mtime_ns: m.mtime_nsec(),
            ctime: m.ctime(),
            ctime_ns: m.ctime_nsec(),
        }
    }
    fn safe(&self, parent: &Metadata) -> bool {
        self.mode & libc::S_IFMT == libc::S_IFREG
            && self.mode & 0o7777 == 0o600
            && self.links == 1
            && self.len <= MAX_BYTES
            && self.uid == parent.uid()
            && self.gid == parent.gid()
            && self.device == parent.dev()
    }
}
struct Document {
    file: File,
    stamp: Stamp,
    raw: Vec<u8>,
}
impl Document {
    fn read(parent: &File) -> Result<Self, FixedReservedStateError> {
        let file = open_at(parent, ACTIVE_DIAGNOSTIC_NAME, libc::O_RDONLY, 0)?;
        let stamp = Stamp::of(&file.metadata()?);
        if !stamp.safe(&parent.metadata()?) {
            return Err(FixedReservedStateError::Document);
        }
        let raw = read_bytes(&file)?;
        if raw.len() as u64 != stamp.len || Stamp::of(&file.metadata()?) != stamp {
            return Err(FixedReservedStateError::Document);
        }
        Ok(Self { file, stamp, raw })
    }
    fn revalidate(&self, parent: &File) -> Result<(), FixedReservedStateError> {
        let named = Self::read(parent)?;
        if named.stamp != self.stamp
            || named.raw != self.raw
            || Stamp::of(&self.file.metadata()?) != self.stamp
            || read_bytes(&self.file)? != self.raw
            || Stamp::of(&self.file.metadata()?) != self.stamp
        {
            return Err(FixedReservedStateError::Document);
        }
        Ok(())
    }
}
fn read_bytes(file: &File) -> Result<Vec<u8>, FixedReservedStateError> {
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        let n = file.read_at(&mut buffer, bytes.len() as u64)?;
        if n == 0 {
            return Ok(bytes);
        }
        if bytes.len() as u64 + n as u64 > MAX_BYTES {
            return Err(FixedReservedStateError::Document);
        }
        bytes.extend_from_slice(&buffer[..n]);
    }
}
fn parent(storage: &FixedIntentStorageRead) -> Result<&File, FixedReservedStateError> {
    if !matches!(storage.kind, StorageLeaf::Intent) {
        return Err(FixedReservedStateError::Document);
    }
    Ok(&storage
        .components
        .last()
        .ok_or(FixedReservedStateError::Document)?
        .file)
}
fn conflicts(parent: &File) -> Result<(), FixedReservedStateError> {
    for name in CONFLICTS {
        let name = CString::new(name).map_err(|_| FixedReservedStateError::Document)?;
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
            return Err(FixedReservedStateError::Conflict);
        }
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::NotFound {
            return Err(error.into());
        }
    }
    Ok(())
}
fn expected_valid(expected: &DiagnosticState) -> bool {
    expected.stage == DiagnosticStage::Reserved && validate_state(expected).is_ok()
}

/// Exclusive fixed storage borrow. Success observes bytes, never execution.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_output_preparation::fixed_intent_storage::reserved_state::FixedReservedStateRead;
/// fn copy(read: FixedReservedStateRead<'_>) { let _ = read.clone(); }
/// ```
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_output_preparation::fixed_intent_storage::{FixedIntentStorageRead, reserved_state::bind_fixed_reserved_state};
/// use hydracache_long_run_supervisor_074::diagnostic_lease::DiagnosticState;
/// fn reuse(storage: &mut FixedIntentStorageRead, state: &DiagnosticState) {
///     let mut read = bind_fixed_reserved_state(storage, state).unwrap();
///     storage.revalidate().unwrap();
///     read.revalidate(state).unwrap();
/// }
/// ```
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_output_preparation::fixed_intent_storage::reserved_state::FixedReservedStateRead;
/// fn start(read: &mut FixedReservedStateRead<'_>) { read.start_once(); }
/// ```
pub struct FixedReservedStateRead<'storage> {
    storage: &'storage mut FixedIntentStorageRead,
    document: Document,
    expected: DiagnosticState,
}
impl FixedReservedStateRead<'_> {
    pub fn is_refused(&self) -> bool {
        self.storage.is_refused()
    }
    pub fn revalidate(
        &mut self,
        expected: &DiagnosticState,
    ) -> Result<(), FixedReservedStateError> {
        if self.is_refused() {
            return Err(FixedReservedStateError::Refused);
        }
        let result = self.inspect(expected);
        if result.is_err() {
            self.storage.refused = true;
        }
        result
    }
    fn inspect(&mut self, expected: &DiagnosticState) -> Result<(), FixedReservedStateError> {
        if expected != &self.expected {
            return Err(FixedReservedStateError::Expected);
        }
        self.storage.revalidate()?;
        let leaf = parent(self.storage)?;
        conflicts(leaf)?;
        self.document.revalidate(leaf)?;
        conflicts(leaf)?;
        self.storage.revalidate()?;
        Ok(())
    }
}
pub fn bind_fixed_reserved_state<'storage>(
    storage: &'storage mut FixedIntentStorageRead,
    expected: &DiagnosticState,
) -> Result<FixedReservedStateRead<'storage>, FixedReservedStateError> {
    if storage.is_refused() {
        return Err(FixedReservedStateError::Refused);
    }
    if !expected_valid(expected) {
        return Err(FixedReservedStateError::Expected);
    }
    let result = (|| {
        storage.revalidate()?;
        let leaf = parent(storage)?;
        conflicts(leaf)?;
        let document = Document::read(leaf)?;
        let state = decode_active(&document.raw).map_err(|_| FixedReservedStateError::Document)?;
        if state != *expected {
            return Err(FixedReservedStateError::Expected);
        }
        document.revalidate(leaf)?;
        conflicts(leaf)?;
        storage.revalidate()?;
        Ok(document)
    })();
    let document = match result {
        Ok(document) => document,
        Err(error) => {
            storage.refused = true;
            return Err(error);
        }
    };
    Ok(FixedReservedStateRead {
        storage,
        document,
        expected: expected.clone(),
    })
}

#[cfg(test)]
#[path = "diagnostic_reserved_state_tests.rs"]
mod tests;
