//! Fixed procfs current-inode consistency; NOT trusted namespace/root enrollment.
use super::*;
fn metadata(fd: i32, descriptor: i32) -> io::Result<[u64; 11]> {
    let mut m: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstat(fd, &mut m) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    need(
        m.st_mode & libc::S_IFMT == libc::S_IFREG
            && m.st_size > 0
            && m.st_size <= 134217728
            && m.st_nlink == 1
            && matches!(m.st_mode & 0o7777, 0o555 | 0o755)
            && unsafe { libc::fcntl(fd, libc::F_GETFD) } == descriptor
            && flags >= 0
            && flags & (libc::O_ACCMODE | libc::O_PATH) == libc::O_RDONLY
            && unsafe { libc::lseek(fd, 0, libc::SEEK_CUR) } == 0,
    )?;
    Ok([
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
    ])
}
pub(super) fn observe(role: &kernel::RoleStamp) -> io::Result<[u64; 11]> {
    need(metadata(role.fd, 0)? == role.metadata)?;
    let directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open("/proc/thread-self")?;
    let mut fs: libc::statfs = unsafe { std::mem::zeroed() };
    need(
        directory.as_raw_fd() >= 17
            && unsafe { libc::fstatfs(directory.as_raw_fd(), &mut fs) } == 0
            && fs.f_type == libc::PROC_SUPER_MAGIC as libc::c_long,
    )?;
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            c"exe".as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    use std::os::fd::FromRawFd;
    // Only this opener's new result is owned. Never adopt an inherited descriptor.
    let current = unsafe { std::fs::File::from_raw_fd(fd) };
    let fd = current.as_raw_fd();
    need(fd >= 17)?;
    let observed = metadata(fd, libc::FD_CLOEXEC)?;
    need(
        observed == role.metadata
            && metadata(role.fd, 0)? == role.metadata
            && metadata(fd, libc::FD_CLOEXEC)? == observed,
    )?;
    Ok(observed)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn file() -> (tempfile::TempDir, std::fs::File) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("owned-image");
        std::fs::write(&path, [1; 128]).unwrap();
        let file = std::fs::File::open(&path).unwrap();
        assert_eq!(
            unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFD, 0) },
            0
        );
        assert_eq!(unsafe { libc::fchmod(file.as_raw_fd(), 0o555) }, 0);
        (directory, file)
    }
    #[test]
    fn bootstrap_installed_executable_metadata_no_flag_offset_or_mode_repair() {
        assert!(metadata(-1, 0).is_err());
        let (_directory, f) = file();
        let fd = f.as_raw_fd();
        assert_eq!(unsafe { libc::fchmod(fd, 0o555) }, 0);
        assert!(metadata(fd, 0).is_ok());
        assert!(metadata(fd, libc::FD_CLOEXEC).is_err());
        assert_eq!(unsafe { libc::fchmod(fd, 0o777) }, 0);
        assert!(metadata(fd, 0).is_err());
        assert_eq!(unsafe { libc::fchmod(fd, 0o555) }, 0);
        assert_eq!(unsafe { libc::lseek(fd, 1, libc::SEEK_SET) }, 1);
        assert!(metadata(fd, 0).is_err());
        assert_eq!(unsafe { libc::lseek(fd, 0, libc::SEEK_CUR) }, 1);
    }
    #[test]
    fn bootstrap_installed_executable_same_bytes_new_inode_are_not_same_metadata() {
        let (_directory_a, a) = file();
        let (_directory_b, b) = file();
        for f in [&a, &b] {
            assert_eq!(unsafe { libc::fchmod(f.as_raw_fd(), 0o555) }, 0);
        }
        assert_ne!(
            metadata(a.as_raw_fd(), 0).unwrap(),
            metadata(b.as_raw_fd(), 0).unwrap()
        );
    }
}
