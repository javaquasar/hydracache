//! Fresh projection at existing held/after comparisons, never a metadata cache.
use super::Stamp;
use std::fs::File;
use std::io;

impl Stamp {
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
