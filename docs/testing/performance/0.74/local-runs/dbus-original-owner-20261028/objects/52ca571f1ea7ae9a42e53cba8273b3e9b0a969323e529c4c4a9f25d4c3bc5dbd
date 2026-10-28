//! One fresh device/inode observation. No owner, namespace or freshness cache.
use std::fs::File;
use std::io;

pub(super) fn fresh(file: &File) -> io::Result<(u64, u64)> {
    #[cfg(target_arch = "x86_64")]
    {
        use std::os::fd::AsRawFd;
        from_fd(file.as_raw_fd())
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = file.metadata()?;
        Ok((metadata.dev(), metadata.ino()))
    }
}

#[cfg(target_arch = "x86_64")]
fn from_fd(fd: std::os::fd::RawFd) -> io::Result<(u64, u64)> {
    let mut value: libc::stat = unsafe { std::mem::zeroed() };
    // SAFETY: synchronous metadata query; writable correctly sized stat object.
    // No ownership is constructed/transferred from the borrowed integer.
    if unsafe { libc::fstat(fd, &mut value) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((value.st_dev, value.st_ino))
}

#[cfg(all(test, target_arch = "x86_64"))]
#[path = "diagnostic_kernel_fd_identity_tests.rs"]
mod tests;
