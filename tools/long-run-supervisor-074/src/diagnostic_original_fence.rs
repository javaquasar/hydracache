//! Short observation under the existing lock, not durable campaign admission.
use super::super::Stamp as DirectoryStamp;
use super::{open_at, FixedReservedStateError, FixedReservedStateRead, Stamp};
use crate::host_execution::HOST_EXECUTION_LOCK_NAME;
use fs2::FileExt;
use std::ffi::CString;
use std::fs::File;
use std::os::fd::AsRawFd;
use thiserror::Error;

const CAMPAIGN_CONTEXTS: [&str; 5] = [
    "active-campaign",
    "active-diagnostic.json",
    ".active-diagnostic.pending",
    ".diagnostic-requests.pending",
    "diagnostic-requests-v1.json",
];
const PARENT_CONTEXTS: [&str; 2] = [
    "campaign-lifecycle-smoke-v1.json",
    "controller-loss-smoke-v1.json",
];

#[derive(Debug, Error)]
pub enum OriginalHostFenceError {
    #[error("original Reserved state refused: {0}")]
    State(#[from] FixedReservedStateError),
    #[error("existing common campaign fence is busy")]
    Busy,
    #[error("original campaign directory or lock identity/metadata refused")]
    Object,
    #[error("legacy campaign, diagnostic or lifecycle context is present")]
    Conflict,
    #[error("original state/storage binding permanently refused")]
    Refused,
    #[error("original fence component open refused: {0}")]
    Open(#[from] super::super::super::PreparationError),
    #[error("original fence I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

struct Fence {
    directory: File,
    directory_stamp: DirectoryStamp,
    lock: File,
    lock_stamp: Stamp,
}
impl Fence {
    fn acquire(parent: &File, uid: u32, gid: u32) -> Result<Self, OriginalHostFenceError> {
        let directory = open_at(parent, "campaigns", libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
        let directory_stamp = DirectoryStamp::of(&directory.metadata()?);
        if !directory_stamp.safe(uid, gid, 4) {
            return Err(OriginalHostFenceError::Object);
        }
        // Existing lock only: no create, replacement, chmod or second namespace.
        let lock = open_at(&directory, HOST_EXECUTION_LOCK_NAME, libc::O_RDWR, 0)?;
        let lock_stamp = Stamp::of(&lock.metadata()?);
        if !lock_stamp.safe(&directory.metadata()?) || lock_stamp.len != 0 {
            return Err(OriginalHostFenceError::Object);
        }
        FileExt::try_lock_exclusive(&lock).map_err(|error| {
            if error.kind() == std::io::ErrorKind::WouldBlock {
                OriginalHostFenceError::Busy
            } else {
                OriginalHostFenceError::Io(error)
            }
        })?;
        Ok(Self {
            directory,
            directory_stamp,
            lock,
            lock_stamp,
        })
    }
    fn inspect(&self, parent: &File) -> Result<(), OriginalHostFenceError> {
        let named = open_at(parent, "campaigns", libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
        if DirectoryStamp::of(&named.metadata()?) != self.directory_stamp
            || DirectoryStamp::of(&self.directory.metadata()?) != self.directory_stamp
        {
            return Err(OriginalHostFenceError::Object);
        }
        let named = open_at(&self.directory, HOST_EXECUTION_LOCK_NAME, libc::O_RDWR, 0)?;
        if Stamp::of(&named.metadata()?) != self.lock_stamp
            || Stamp::of(&self.lock.metadata()?) != self.lock_stamp
        {
            return Err(OriginalHostFenceError::Object);
        }
        absent(&self.directory, &CAMPAIGN_CONTEXTS)?;
        absent(parent, &PARENT_CONTEXTS)?;
        Ok(())
    }
}
impl Drop for Fence {
    fn drop(&mut self) {
        // Never remove state/markers or unlock a caller-supplied descriptor.
        let _ = FileExt::unlock(&self.lock);
    }
}
fn absent(parent: &File, names: &[&str]) -> Result<(), OriginalHostFenceError> {
    for name in names {
        let name = CString::new(*name).map_err(|_| OriginalHostFenceError::Object)?;
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
            return Err(OriginalHostFenceError::Conflict);
        }
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::NotFound {
            return Err(error.into());
        }
    }
    Ok(())
}
fn performance_parent<'a>(
    read: &'a FixedReservedStateRead<'_>,
) -> Result<&'a File, OriginalHostFenceError> {
    Ok(&read
        .storage
        .components
        .get(2)
        .ok_or(OriginalHostFenceError::Object)?
        .file)
}

/// Original state exclusively borrowed while holding the existing common flock.
/// Not a start or a durable campaign exclusion capability.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::FencedReservedStateRead;
/// fn copy(read: FencedReservedStateRead<'_, '_>) { let _ = read.clone(); }
/// ```
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_output_preparation::fixed_intent_storage::reserved_state::{FixedReservedStateRead, original_fence::bind_original_host_fence};
/// fn reuse(read: &mut FixedReservedStateRead<'_>) {
///     let mut fence = bind_original_host_fence(read).unwrap();
///     let _ = read.is_refused();
///     fence.revalidate().unwrap();
/// }
/// ```
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::FencedReservedStateRead;
/// fn start(read: &mut FencedReservedStateRead<'_, '_>) { read.start_once(); }
/// ```
pub struct FencedReservedStateRead<'read, 'storage> {
    original: &'read mut FixedReservedStateRead<'storage>,
    fence: Fence,
}
impl FencedReservedStateRead<'_, '_> {
    pub fn is_refused(&self) -> bool {
        self.original.is_refused()
    }
    pub fn revalidate(&mut self) -> Result<(), OriginalHostFenceError> {
        if self.is_refused() {
            return Err(OriginalHostFenceError::Refused);
        }
        let result = (|| {
            self.fence.inspect(performance_parent(self.original)?)?;
            self.original.revalidate(&self.original.expected.clone())?;
            self.fence.inspect(performance_parent(self.original)?)?;
            // Last ancestry/state observation must still be under this same lock.
            self.original.revalidate(&self.original.expected.clone())?;
            self.fence.inspect(performance_parent(self.original)?)?;
            Ok(())
        })();
        if result.is_err() {
            self.original.storage.refused = true;
        }
        result
    }
}

pub fn bind_original_host_fence<'read, 'storage>(
    original: &'read mut FixedReservedStateRead<'storage>,
) -> Result<FencedReservedStateRead<'read, 'storage>, OriginalHostFenceError> {
    if original.is_refused() {
        return Err(OriginalHostFenceError::Refused);
    }
    original.revalidate(&original.expected.clone())?;
    let result = (|| {
        let leaf = original
            .storage
            .components
            .last()
            .ok_or(OriginalHostFenceError::Object)?;
        Fence::acquire(
            performance_parent(original)?,
            leaf.stamp.uid,
            leaf.stamp.gid,
        )
    })();
    let fence = match result {
        Ok(fence) => fence,
        Err(error) => {
            if !matches!(error, OriginalHostFenceError::Busy) {
                original.storage.refused = true;
            }
            return Err(error);
        }
    };
    let mut read = FencedReservedStateRead { original, fence };
    read.revalidate()?;
    Ok(read)
}

#[cfg(test)]
#[path = "diagnostic_original_fence_tests.rs"]
mod tests;
