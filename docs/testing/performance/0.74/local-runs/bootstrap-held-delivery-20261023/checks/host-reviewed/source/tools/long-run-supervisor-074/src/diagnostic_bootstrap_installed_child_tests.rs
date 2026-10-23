use super::*;
use crate::bootstrap_child_material as material;
fn raw() -> [u8; BYTES] {
    let original = super::super::super::AssertedEntry::decode(&material::raw()).unwrap();
    AssertedInstalledEntry {
        installed_identity: [1, 999, original.identities[2][2]],
        installed_sha256: original.image_hashes[2],
        original,
        parent_inventory_sha256: [7; 32],
    }
    .encode()
    .unwrap()
}
#[test]
fn bootstrap_installed_child_record_roundtrip_only_asserts_and_preserves_offset() {
    let f = material::sealed(&raw(), SEALS);
    let fd = f.as_raw_fd();
    let first = record(fd).unwrap();
    assert_eq!(first.raw, raw());
    assert!(record(fd).unwrap() == first);
    assert_eq!(unsafe { libc::lseek(fd, 0, libc::SEEK_CUR) }, 0);
}
#[test]
fn bootstrap_installed_child_record_seals_flags_lengths_shape_and_offset_refuse() {
    assert!(record(-1).is_err());
    for seals in [
        0,
        SEALS & !libc::F_SEAL_WRITE,
        SEALS | libc::F_SEAL_FUTURE_WRITE,
    ] {
        assert!(record(material::sealed(&raw(), seals).as_raw_fd()).is_err());
    }
    for n in [256, 1024, 1279, 1281] {
        let mut v = raw().to_vec();
        v.resize(n, 0);
        assert!(record(material::sealed(&v, SEALS).as_raw_fd()).is_err());
    }
    let mut v = raw();
    v[1200] = 1;
    assert!(record(material::sealed(&v, SEALS).as_raw_fd()).is_err());
    let f = material::sealed(&raw(), SEALS);
    let fd = f.as_raw_fd();
    assert_eq!(
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) },
        0
    );
    assert!(record(fd).is_err());
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, libc::FD_CLOEXEC);
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_SETFD, 0) }, 0);
    assert_eq!(unsafe { libc::lseek(fd, 1, libc::SEEK_SET) }, 1);
    assert!(record(fd).is_err());
    assert_eq!(unsafe { libc::lseek(fd, 0, libc::SEEK_CUR) }, 1);
}
#[test]
fn bootstrap_installed_child_record_same_bytes_other_inode_and_metadata_change_differ() {
    let a = material::sealed(&raw(), SEALS);
    let b = material::sealed(&raw(), SEALS);
    let first = record(a.as_raw_fd()).unwrap();
    assert!(record(b.as_raw_fd()).unwrap() != first);
    assert_eq!(unsafe { libc::fchmod(a.as_raw_fd(), 0o600) }, 0);
    assert!(record(a.as_raw_fd()).unwrap() != first);
}
