//! Same-inode consistency, not protected procfs namespace or signed build trust.
use super::{AssertedEntry, ObservedAssertions, SEALS};
use std::{
    io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};
fn need(ok: bool) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "bootstrap current executable consistency",
        ))
    }
}
fn owned(fd: i32) -> io::Result<OwnedFd> {
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // Only a successful opener's newly owned result, never an inherited role.
    let file = unsafe { OwnedFd::from_raw_fd(fd) };
    need(fd >= 15)?;
    Ok(file)
}
fn stat(fd: i32) -> io::Result<libc::stat> {
    let mut m = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstat(fd, &mut m) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(m)
}
#[derive(Debug, PartialEq, Eq)]
pub(in super::super) struct ObservedImage([u64; 11]);
fn stamp(fd: i32, descriptor_flags: i32) -> io::Result<ObservedImage> {
    let m = stat(fd)?;
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    need(
        m.st_mode & libc::S_IFMT == libc::S_IFREG
            && m.st_size > 0
            && m.st_size <= 134217728
            && flags >= 0
            && flags & (libc::O_ACCMODE | libc::O_PATH) == libc::O_RDONLY
            && unsafe { libc::fcntl(fd, libc::F_GETFD) } == descriptor_flags
            && unsafe { libc::fcntl(fd, libc::F_GET_SEALS) } == SEALS
            && unsafe { libc::lseek(fd, 0, libc::SEEK_CUR) } == 0,
    )?;
    Ok(ObservedImage([
        m.st_dev,
        m.st_ino,
        m.st_size as u64,
        u64::from(m.st_uid),
        u64::from(m.st_gid),
        u64::from(m.st_mode),
        m.st_nlink,
        m.st_mtime as u64,
        m.st_mtime_nsec as u64,
        m.st_ctime as u64,
        m.st_ctime_nsec as u64,
    ]))
}
pub(in super::super) fn observe(entry: &ObservedAssertions) -> io::Result<ObservedImage> {
    super::revalidate(14, entry)?;
    let assertions = AssertedEntry::decode(&entry.raw).map_err(|_| {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            "bootstrap executable entry",
        )
    })?;
    let role = stamp(13, 0)?;
    need(role.0[..3] == assertions.identities[2])?;
    // Deliberately follow only these fixed proc magic links. PROC_SUPER_MAGIC
    // rejects ordinary substituted filesystems, NOT an unenrolled mount policy.
    let directory = owned(unsafe {
        libc::open(
            c"/proc/thread-self".as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    })?;
    let mut fs: libc::statfs = unsafe { std::mem::zeroed() };
    need(
        unsafe { libc::fstatfs(directory.as_raw_fd(), &mut fs) } == 0
            && fs.f_type == libc::PROC_SUPER_MAGIC
            && stat(directory.as_raw_fd())?.st_mode & libc::S_IFMT == libc::S_IFDIR,
    )?;
    let executable = owned(unsafe {
        libc::openat(
            directory.as_raw_fd(),
            c"exe".as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    })?;
    let current = stamp(executable.as_raw_fd(), libc::FD_CLOEXEC)?;
    need(current == role)?;
    need(stamp(13, 0)? == role && stamp(executable.as_raw_fd(), libc::FD_CLOEXEC)? == current)?;
    super::revalidate(14, entry)?;
    // Both owned inspection FDs drop before the next closed-set census.
    Ok(current)
}
pub(in super::super) fn revalidate(
    entry: &ObservedAssertions,
    old: &ObservedImage,
) -> io::Result<()> {
    need(observe(entry)? == *old)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap_child_material as material;
    #[test]
    fn current_image_stamp_rejects_invalid_unsealed_and_wrong_flags_without_repair() {
        assert!(stamp(-1, 0).is_err());
        let file = material::sealed(&[1; 128], SEALS);
        let fd = file.as_raw_fd();
        assert!(stamp(fd, 0).is_ok());
        assert!(stamp(fd, libc::FD_CLOEXEC).is_err());
        assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, 0);
        assert_eq!(unsafe { libc::lseek(fd, 0, libc::SEEK_CUR) }, 0);
        let unsealed = material::sealed(&[1; 128], 0);
        assert!(stamp(unsealed.as_raw_fd(), 0).is_err());
    }
    #[test]
    fn identical_bytes_at_another_inode_do_not_identify_the_same_image() {
        let a = material::sealed(&[1; 128], SEALS);
        let b = material::sealed(&[1; 128], SEALS);
        let first = stamp(a.as_raw_fd(), 0).unwrap();
        assert_ne!(first, stamp(b.as_raw_fd(), 0).unwrap());
        assert_eq!(first, stamp(a.as_raw_fd(), 0).unwrap());
    }
}
