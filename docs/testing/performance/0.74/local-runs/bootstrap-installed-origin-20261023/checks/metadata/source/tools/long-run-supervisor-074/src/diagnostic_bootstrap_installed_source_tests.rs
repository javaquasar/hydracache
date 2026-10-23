//! Private modeled roots/UIDs, not production root or namespace enrollment.
use super::*;
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
fn owner() -> [u32; 2] {
    unsafe { [libc::geteuid(), libc::getegid()] }
}
struct Fixture {
    root: tempfile::TempDir,
    path: PathBuf,
    build: VerifiedBuild,
    bytes: Vec<u8>,
}
impl Fixture {
    fn new() -> Self {
        let model = super::super::tests::Fixture::new();
        let root = tempfile::tempdir().unwrap();
        let path = root
            .path()
            .join("opt/hydracache-perf/bin/diagnostic-held-bootstrap-074");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let f = Self {
            root,
            path,
            build: model.verified(),
            bytes: model.image,
        };
        f.write(&f.bytes);
        f
    }
    fn write(&self, bytes: &[u8]) {
        fs::write(&self.path, bytes).unwrap();
        fs::set_permissions(&self.path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fn open(&self) -> Result<InstalledSource, ArtifactError> {
        InstalledSource::open_under(self.root.path(), owner(), &self.build)
    }
}
#[test]
fn bootstrap_installed_source_private_root_matches_exact_build_without_offset_changes() {
    let f = Fixture::new();
    let mut source = f.open().unwrap();
    for _ in 0..3 {
        source.revalidate(&f.build).unwrap();
    }
    assert_eq!(
        unsafe { libc::lseek(source.image.as_raw_fd(), 0, libc::SEEK_CUR) },
        0
    );
    assert_eq!(source.directories.len(), 4);
    assert!(!source.refused);
    fs::set_permissions(&f.path, fs::Permissions::from_mode(0o555)).unwrap();
    assert!(f.open().is_ok());
    assert!(source.revalidate(&f.build).is_err());
}
#[test]
fn bootstrap_installed_source_same_bytes_replacement_is_sticky_after_restore() {
    let f = Fixture::new();
    let mut source = f.open().unwrap();
    let old = f.path.with_extension("old");
    fs::rename(&f.path, &old).unwrap();
    f.write(&f.bytes);
    assert!(source.revalidate(&f.build).is_err());
    fs::remove_file(&f.path).unwrap();
    fs::rename(old, &f.path).unwrap();
    assert!(source.revalidate(&f.build).is_err());
    assert!(source.refused);
}
#[test]
fn bootstrap_installed_source_symlink_hardlink_mode_and_owner_cannot_enroll() {
    for mode in [0o400, 0o777, 0o4755, 0o2755, 0o1755] {
        let f = Fixture::new();
        fs::set_permissions(&f.path, fs::Permissions::from_mode(mode)).unwrap();
        assert!(f.open().is_err());
    }
    let f = Fixture::new();
    fs::hard_link(&f.path, f.path.with_extension("hardlink")).unwrap();
    assert!(f.open().is_err());
    let f = Fixture::new();
    let old = f.path.with_extension("old");
    fs::rename(&f.path, &old).unwrap();
    symlink(old, &f.path).unwrap();
    assert!(f.open().is_err());
    let f = Fixture::new();
    let mut wrong = owner();
    wrong[0] ^= 1;
    assert!(InstalledSource::open_under(f.root.path(), wrong, &f.build).is_err());
}
#[test]
fn bootstrap_installed_source_content_size_elf_and_ancestor_changes_refuse() {
    for at in [0, 30, 119] {
        let f = Fixture::new();
        let mut bad = f.bytes.clone();
        bad[at] ^= 1;
        f.write(&bad);
        assert!(f.open().is_err());
    }
    let f = Fixture::new();
    f.write(&[0; 64]);
    assert!(f.open().is_err());
    let f = Fixture::new();
    let mut source = f.open().unwrap();
    let bin = f.path.parent().unwrap();
    let moved = bin.with_extension("old");
    fs::rename(bin, &moved).unwrap();
    fs::create_dir(bin).unwrap();
    f.write(&f.bytes);
    assert!(source.revalidate(&f.build).is_err());
    let f = Fixture::new();
    fs::set_permissions(f.path.parent().unwrap(), fs::Permissions::from_mode(0o777)).unwrap();
    assert!(f.open().is_err());
}
#[test]
fn bootstrap_installed_source_flags_offsets_and_missing_inputs_are_not_repaired() {
    for kind in 0..4 {
        let f = Fixture::new();
        let mut source = f.open().unwrap();
        let fd = source.image.as_raw_fd();
        match kind {
            0 => assert_eq!(unsafe { libc::lseek(fd, 1, libc::SEEK_SET) }, 1),
            1 => assert_eq!(unsafe { libc::fcntl(fd, libc::F_SETFD, 0) }, 0),
            2 => assert_eq!(unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_RDONLY) }, 0),
            _ => fs::remove_file(&f.path).unwrap(),
        }
        assert!(source.revalidate(&f.build).is_err());
        assert!(source.refused);
        if kind == 0 {
            assert_eq!(unsafe { libc::lseek(fd, 0, libc::SEEK_CUR) }, 1);
        }
        if kind == 1 {
            assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, 0);
        }
    }
}
