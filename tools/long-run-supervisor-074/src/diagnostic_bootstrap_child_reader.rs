//! Point-in-time assertions from an inherited descriptor; no authority or FD ownership.
use hydracache_long_run_supervisor_074::diagnostic_builder::bootstrap_build::entry_format::{
    AssertedEntry, BYTES,
};
use std::io;
const SEALS: i32 = libc::F_SEAL_SEAL | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_WRITE;
fn need(ok: bool) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "bootstrap entry descriptor assertions",
        ))
    }
}
#[derive(PartialEq, Eq)]
struct Stamp {
    fd: i32,
    metadata: [u64; 11],
    flags: [i32; 3],
    offset: i64,
}
fn stamp(fd: i32) -> io::Result<Stamp> {
    let mut m: libc::stat = unsafe { std::mem::zeroed() };
    // Numeric descriptors are untrusted. No FromRawFd/BorrowedFd validity or
    // ownership is assumed, and a missing descriptor safely refuses at fstat.
    if unsafe { libc::fstat(fd, &mut m) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let flags = unsafe {
        [
            libc::fcntl(fd, libc::F_GETFD),
            libc::fcntl(fd, libc::F_GETFL),
            libc::fcntl(fd, libc::F_GET_SEALS),
        ]
    };
    need(
        fd >= 3
            && m.st_mode & libc::S_IFMT == libc::S_IFREG
            && m.st_size == BYTES as i64
            && flags[0] == 0
            && flags[1] >= 0
            && flags[1] & (libc::O_ACCMODE | libc::O_PATH) == libc::O_RDONLY
            && flags[2] == SEALS,
    )?;
    let offset = unsafe { libc::lseek(fd, 0, libc::SEEK_CUR) };
    need(offset == 0)?;
    Ok(Stamp {
        fd,
        metadata: [
            m.st_dev,
            m.st_ino,
            u64::from(m.st_uid),
            u64::from(m.st_gid),
            u64::from(m.st_mode),
            m.st_nlink,
            m.st_size as u64,
            m.st_mtime as u64,
            m.st_mtime_nsec as u64,
            m.st_ctime as u64,
            m.st_ctime_nsec as u64,
        ],
        flags,
        offset,
    })
}
#[derive(PartialEq, Eq)]
pub(super) struct ObservedAssertions {
    stamp: Stamp,
    raw: [u8; BYTES],
}
pub(super) fn read(fd: i32) -> io::Result<ObservedAssertions> {
    let before = stamp(fd)?;
    let mut raw = [0; BYTES];
    let mut at = 0;
    // At most BYTES successful reads. EINTR/errors refuse rather than retrying
    // indefinitely or interpreting a partial record. No shared offset mutation.
    while at < raw.len() {
        let n = unsafe {
            libc::pread(
                fd,
                raw[at..].as_mut_ptr().cast(),
                raw.len() - at,
                at as libc::off_t,
            )
        };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        need(n > 0 && n as usize <= raw.len() - at)?;
        at += n as usize;
    }
    AssertedEntry::decode(&raw)
        .map_err(|_| io::Error::new(io::ErrorKind::PermissionDenied, "bootstrap entry shape"))?;
    need(stamp(fd)? == before)?;
    Ok(ObservedAssertions { stamp: before, raw })
}
pub(super) fn revalidate(fd: i32, old: &ObservedAssertions) -> io::Result<()> {
    need(old.stamp.fd == fd)?;
    need(read(fd)? == *old)
}

#[cfg(test)]
#[path = "diagnostic_bootstrap_child_reader_tests.rs"]
mod tests;

#[path = "diagnostic_bootstrap_child_roles.rs"]
pub(super) mod roles;
