use super::*;
use std::os::fd::AsRawFd;
#[path = "../tests/support/bootstrap_child_material.rs"]
mod material;

#[test]
fn bootstrap_reader_roundtrip_and_revalidate_are_only_assertions_and_preserve_offset() {
    let file = material::sealed(&material::raw(), material::SEALS);
    let fd = file.as_raw_fd();
    let first = read(fd).unwrap();
    assert!(first.raw == material::raw());
    revalidate(fd, &first).unwrap();
    assert_eq!(unsafe { libc::lseek(fd, 0, libc::SEEK_CUR) }, 0);
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, 0);
}
#[test]
fn bootstrap_reader_missing_seals_cloexec_length_padding_and_offset_refuse_without_repair() {
    assert!(read(-1).is_err());
    for seals in [
        0,
        material::SEALS & !libc::F_SEAL_SEAL,
        material::SEALS & !libc::F_SEAL_GROW,
        material::SEALS & !libc::F_SEAL_SHRINK,
        material::SEALS & !libc::F_SEAL_WRITE,
        material::SEALS | libc::F_SEAL_FUTURE_WRITE,
    ] {
        let f = material::sealed(&material::raw(), seals);
        assert!(read(f.as_raw_fd()).is_err());
    }
    for n in [0, 256, 1023, 1025] {
        let mut raw = material::raw().to_vec();
        raw.resize(n, 0);
        let f = material::sealed(&raw, material::SEALS);
        assert!(read(f.as_raw_fd()).is_err());
    }
    let f = material::sealed(&material::raw(), material::SEALS);
    let fd = f.as_raw_fd();
    assert_eq!(
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) },
        0
    );
    assert!(read(fd).is_err());
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, libc::FD_CLOEXEC);
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_SETFD, 0) }, 0);
    assert_eq!(unsafe { libc::lseek(fd, 1, libc::SEEK_SET) }, 1);
    assert!(read(fd).is_err());
    assert_eq!(unsafe { libc::lseek(fd, 0, libc::SEEK_CUR) }, 1);
    let mut bad = material::raw();
    bad[12] = 1;
    let f = material::sealed(&bad, material::SEALS);
    assert!(read(f.as_raw_fd()).is_err());
}
#[test]
fn bootstrap_reader_inode_descriptor_and_metadata_substitution_cannot_revalidate() {
    use std::os::unix::fs::PermissionsExt;
    let f = material::sealed(&material::raw(), material::SEALS);
    let first = read(f.as_raw_fd()).unwrap();
    let other = material::sealed(&material::raw(), material::SEALS);
    assert!(revalidate(other.as_raw_fd(), &first).is_err());
    let alias = f.try_clone().unwrap();
    assert_eq!(
        unsafe { libc::fcntl(alias.as_raw_fd(), libc::F_SETFD, 0) },
        0
    );
    assert!(revalidate(alias.as_raw_fd(), &first).is_err());
    f.set_permissions(std::fs::Permissions::from_mode(0o600))
        .unwrap();
    assert!(revalidate(f.as_raw_fd(), &first).is_err());
}
#[test]
fn bootstrap_reader_wrong_access_and_nonregular_objects_refuse() {
    use std::os::{fd::FromRawFd, unix::fs::PermissionsExt};
    let f = material::sealed(&material::raw(), material::SEALS);
    f.set_permissions(std::fs::Permissions::from_mode(0o600))
        .unwrap();
    let path = std::ffi::CString::new(format!("/proc/thread-self/fd/{}", f.as_raw_fd())).unwrap();
    for access in [libc::O_RDWR, libc::O_PATH] {
        let fd = unsafe { libc::open(path.as_ptr(), access) };
        assert!(fd >= 0);
        let bad = unsafe { std::fs::File::from_raw_fd(fd) };
        assert!(read(bad.as_raw_fd()).is_err());
    }
    let (a, _b) = std::os::unix::net::UnixStream::pair().unwrap();
    assert!(read(a.as_raw_fd()).is_err());
}
