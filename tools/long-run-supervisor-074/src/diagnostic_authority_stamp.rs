//! Fresh projection at existing held/after comparisons, never a metadata cache.
use super::{AuthorityError, Stamp};
use std::fs::File;
use std::io;

impl Stamp {
    pub(super) fn fresh_regular(
        file: &File,
        uid: u32,
        gid: u32,
        mode: u32,
    ) -> Result<Self, AuthorityError> {
        #[cfg(target_arch = "x86_64")]
        {
            let stamp = Self::fresh(file)?;
            stamp.require_regular(uid, gid, mode)?;
            Ok(stamp)
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            let metadata = file.metadata()?;
            super::regular(&metadata, uid, gid, mode)?;
            Ok(Self::of(&metadata))
        }
    }

    #[cfg(target_arch = "x86_64")]
    pub(super) fn require_regular(
        &self,
        uid: u32,
        gid: u32,
        mode: u32,
    ) -> Result<(), AuthorityError> {
        if self.mode & libc::S_IFMT != libc::S_IFREG
            || self.uid != uid
            || self.gid != gid
            || self.mode & 0o7777 != mode
            || self.links != 1
        {
            return Err(AuthorityError::Invalid);
        }
        Ok(())
    }

    pub(super) fn directory_matches_file(&self, file: &File) -> io::Result<bool> {
        #[cfg(target_arch = "x86_64")]
        {
            let m = Self::fresh(file)?;
            Ok(m.mode & libc::S_IFMT == libc::S_IFDIR
                && self.dev == m.dev
                && self.ino == m.ino
                && self.uid == m.uid
                && self.gid == m.gid
                && self.mode == m.mode)
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            Ok(self.directory_matches(&file.metadata()?))
        }
    }

    pub(super) fn fresh(file: &File) -> io::Result<Self> {
        #[cfg(target_arch = "x86_64")]
        {
            use std::os::fd::AsRawFd;
            from_fd(file.as_raw_fd())
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            Ok(Self::of(&file.metadata()?))
        }
    }
}

#[cfg(target_arch = "x86_64")]
fn from_fd(fd: std::os::fd::RawFd) -> io::Result<Stamp> {
    let mut m: libc::stat = unsafe { std::mem::zeroed() };
    // SAFETY: synchronous query of the borrowed descriptor into a complete stat
    // object. No ownership is constructed from fd and errno is captured at once.
    if unsafe { libc::fstat(fd, &mut m) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(Stamp {
        dev: m.st_dev,
        ino: m.st_ino,
        uid: m.st_uid,
        gid: m.st_gid,
        mode: m.st_mode,
        links: m.st_nlink,
        len: m.st_size as u64,
        mtime: m.st_mtime,
        mtime_ns: m.st_mtime_nsec,
        ctime: m.st_ctime,
        ctime_ns: m.st_ctime_nsec,
    })
}

#[cfg(all(test, target_arch = "x86_64"))]
#[path = "diagnostic_authority_stamp_tests.rs"]
mod tests;
