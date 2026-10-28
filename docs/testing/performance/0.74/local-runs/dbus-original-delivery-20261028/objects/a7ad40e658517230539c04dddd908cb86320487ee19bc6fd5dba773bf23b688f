#![cfg(target_os = "linux")]
use hydracache_long_run_supervisor_074::diagnostic_artifacts::ArtifactDigest;
use hydracache_long_run_supervisor_074::diagnostic_prebuilt_copy::prepare_asserted_copy;
use std::fs::{self, File, OpenOptions};
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt, PermissionsExt};

const ATTEMPT: &str = "attempt-00000000000000000000000000000001";
struct Fixture {
    temp: tempfile::TempDir,
    source: File,
    parent: File,
    expected: ArtifactDigest,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let path = temp.path().join("cargo-output");
        let bytes = vec![0x74; 131_089];
        fs::write(&path, &bytes).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        fs::hard_link(&path, temp.path().join("cargo-alias")).unwrap();
        let source = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_CLOEXEC)
            .open(path)
            .unwrap();
        let parent = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC)
            .open(temp.path())
            .unwrap();
        Self {
            temp,
            source,
            parent,
            expected: ArtifactDigest::of(&bytes),
        }
    }
    fn output(&self) -> std::path::PathBuf {
        self.temp.path().join(ATTEMPT).join("diagnostic_controller")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for entry in fs::read_dir(self.temp.path()).unwrap() {
            let path = entry.unwrap().path();
            if fs::symlink_metadata(&path).unwrap().is_dir() {
                fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
            }
        }
    }
}
#[test]
fn copy_is_distinct_readonly_single_link_without_repairing_cargo_source() {
    let f = Fixture::new();
    let before = f.source.metadata().unwrap();
    let mut prepared =
        prepare_asserted_copy(&f.source, &f.parent, ATTEMPT, f.expected.clone()).unwrap();
    assert_eq!(prepared.check_original().unwrap(), &f.expected);
    let out = fs::metadata(f.output()).unwrap();
    assert_ne!((before.dev(), before.ino()), (out.dev(), out.ino()));
    assert_eq!(out.nlink(), 1);
    assert_eq!(out.mode() & 0o7777, 0o555);
    assert_eq!(
        fs::metadata(f.output().parent().unwrap()).unwrap().mode() & 0o7777,
        0o555
    );
    let after = f.source.metadata().unwrap();
    assert_eq!(
        (
            after.mode(),
            after.nlink(),
            after.ctime(),
            after.ctime_nsec()
        ),
        (before.mode(), 2, before.ctime(), before.ctime_nsec())
    );
    assert!(prepare_asserted_copy(&f.source, &f.parent, ATTEMPT, f.expected.clone()).is_err());
    assert_eq!(prepared.check_original().unwrap(), &f.expected);
}
#[test]
fn source_alias_writer_after_copy_does_not_change_the_new_inode() {
    let f = Fixture::new();
    let mut copy =
        prepare_asserted_copy(&f.source, &f.parent, ATTEMPT, f.expected.clone()).unwrap();
    let alias = OpenOptions::new()
        .write(true)
        .open(f.temp.path().join("cargo-alias"))
        .unwrap();
    alias.write_all_at(b"different source", 0).unwrap();
    assert_eq!(copy.check_original().unwrap(), &f.expected);
}
#[test]
fn invalid_expected_digest_creates_no_attempt() {
    let f = Fixture::new();
    let mut bad = f.expected.clone();
    bad.sha256 = "BAD".into();
    assert!(prepare_asserted_copy(&f.source, &f.parent, ATTEMPT, bad).is_err());
    assert!(!f.output().parent().unwrap().exists());
}
