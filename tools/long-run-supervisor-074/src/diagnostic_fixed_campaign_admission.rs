//! Conservative fixed context predicate, not root enrollment or publication.
use super::reserved_state::Stamp as LockStamp;
use super::{open_at, open_fixed_intent_storage, open_root, FixedIntentStorageRead, Stamp};
use crate::host_execution::{HostExecutionError, HOST_EXECUTION_LOCK_NAME};
use std::ffi::CStr;
use std::fs::File;
use std::os::fd::IntoRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

fn refused<T>(_: T) -> HostExecutionError {
    HostExecutionError::Path
}

pub(crate) struct FixedCampaignAdmission {
    storage: FixedIntentStorageRead,
    root_name: PathBuf,
    directory: File,
    directory_stamp: Stamp,
    lock_stamp: LockStamp,
}
impl FixedCampaignAdmission {
    pub(crate) fn open(root: &Path, lock: &File) -> Result<Self, HostExecutionError> {
        Self::capture(open_fixed_intent_storage().map_err(refused)?, root, lock)
    }
    fn capture(
        mut storage: FixedIntentStorageRead,
        root: &Path,
        lock: &File,
    ) -> Result<Self, HostExecutionError> {
        storage.revalidate().map_err(refused)?;
        let parent = &storage
            .components
            .get(2)
            .ok_or(HostExecutionError::Path)?
            .file;
        let directory =
            open_at(parent, "campaigns", libc::O_RDONLY | libc::O_DIRECTORY, 0).map_err(refused)?;
        let m = directory.metadata()?;
        // Pin legacy ownership; this check does not turn it into root authority.
        if !m.is_dir() || m.mode() & 0o7022 != 0 {
            return Err(HostExecutionError::Path);
        }
        let directory_stamp = Stamp::of(&m);
        let lock_stamp = LockStamp::of(&lock.metadata()?);
        let mut read = Self {
            storage,
            root_name: root.into(),
            directory,
            directory_stamp,
            lock_stamp,
        };
        read.check_objects(root, lock)?;
        read.storage.revalidate().map_err(refused)?;
        Ok(read)
    }
    fn check_objects(&self, root: &Path, lock: &File) -> Result<(), HostExecutionError> {
        if root != self.root_name {
            return Err(HostExecutionError::Path);
        }
        let parent = &self
            .storage
            .components
            .get(2)
            .ok_or(HostExecutionError::Path)?
            .file;
        let named =
            open_at(parent, "campaigns", libc::O_RDONLY | libc::O_DIRECTORY, 0).map_err(refused)?;
        let by_path = open_root(root).map_err(refused)?;
        if Stamp::of(&named.metadata()?) != self.directory_stamp
            || Stamp::of(&by_path.metadata()?) != self.directory_stamp
            || Stamp::of(&self.directory.metadata()?) != self.directory_stamp
        {
            return Err(HostExecutionError::Path);
        }
        let named = open_at(&self.directory, HOST_EXECUTION_LOCK_NAME, libc::O_RDONLY, 0)
            .map_err(refused)?;
        let m = named.metadata()?;
        if !m.is_file()
            || m.nlink() != 1
            || m.dev() != self.directory.metadata()?.dev()
            || m.mode() & 0o7002 != 0
            || LockStamp::of(&m) != self.lock_stamp
            || LockStamp::of(&lock.metadata()?) != self.lock_stamp
        {
            return Err(HostExecutionError::Path);
        }
        Ok(())
    }
    pub(crate) fn present(&mut self, root: &Path, lock: &File) -> Result<bool, HostExecutionError> {
        if self.storage.is_refused() {
            return Err(HostExecutionError::Path);
        }
        let result = (|| {
            self.storage.revalidate().map_err(refused)?;
            self.check_objects(root, lock)?;
            let present = leaf_present(&mut self.storage)?;
            self.check_objects(root, lock)?;
            self.storage.revalidate().map_err(refused)?;
            Ok(present)
        })();
        if result.is_err() {
            self.storage.refused = true;
        }
        result
    }
    #[cfg(test)]
    pub(crate) fn modeled(
        origin: &Path,
        root: &Path,
        lock: &File,
    ) -> Result<Self, HostExecutionError> {
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        assert!(
            uid != 0 && gid != 0,
            "private admission model remains nonroot"
        );
        Self::capture(super::walk(origin, uid, gid).map_err(refused)?, root, lock)
    }
}

/// Unlocked conservative observation only, never a fenced admission capability.
pub(crate) fn observe() -> Result<bool, HostExecutionError> {
    leaf_present(&mut open_fixed_intent_storage().map_err(refused)?)
}
fn leaf_present(storage: &mut FixedIntentStorageRead) -> Result<bool, HostExecutionError> {
    if storage.is_refused() {
        return Err(HostExecutionError::Path);
    }
    let result = (|| {
        storage.revalidate().map_err(refused)?;
        let leaf = &storage
            .components
            .last()
            .ok_or(HostExecutionError::Path)?
            .file;
        // Independent open file description: never reuse a previous readdir offset.
        let fresh = open_at(leaf, ".", libc::O_RDONLY | libc::O_DIRECTORY, 0).map_err(refused)?;
        let present = directory_present(fresh)?;
        storage.revalidate().map_err(refused)?;
        Ok(present)
    })();
    if result.is_err() {
        storage.refused = true;
    }
    result
}
struct DirectoryStream(*mut libc::DIR);
impl Drop for DirectoryStream {
    fn drop(&mut self) {
        unsafe {
            libc::closedir(self.0);
        }
    }
}
fn directory_present(file: File) -> Result<bool, HostExecutionError> {
    let fd = file.into_raw_fd();
    let pointer = unsafe { libc::fdopendir(fd) };
    if pointer.is_null() {
        let error = std::io::Error::last_os_error();
        unsafe {
            libc::close(fd);
        }
        return Err(error.into());
    }
    let stream = DirectoryStream(pointer);
    loop {
        // readdir null means EOF only if errno is still zero.
        unsafe {
            *libc::__errno_location() = 0;
        }
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            let errno = unsafe { *libc::__errno_location() };
            return if errno == 0 {
                Ok(false)
            } else {
                Err(std::io::Error::from_raw_os_error(errno).into())
            };
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if name != b"." && name != b".." {
            return Ok(true);
        }
    }
}

#[cfg(test)]
#[path = "diagnostic_fixed_campaign_admission_tests.rs"]
mod tests;
