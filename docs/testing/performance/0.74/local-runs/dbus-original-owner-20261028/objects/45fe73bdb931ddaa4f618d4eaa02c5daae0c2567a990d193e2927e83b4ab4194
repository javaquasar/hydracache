#![cfg(target_os = "linux")]
use hydracache_long_run_supervisor_074::{
    diagnostic_artifacts::ArtifactDigest, diagnostic_root_owned_input::inspect_root_owned_input,
};
use std::{fs, path::Path};
#[test]
fn actual_root_owned_system_file_is_inventory_not_execution() {
    let path = Path::new("/usr/bin/true");
    let expected = ArtifactDigest::of(&fs::read(path).unwrap());
    let mut read = inspect_root_owned_input(path, expected.clone()).unwrap();
    assert_eq!(read.check_original().unwrap(), &expected);
    assert!(!read.is_refused());
}
#[test]
fn caller_owned_copy_cannot_be_promoted_by_readonly_mode() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("not-root-owned");
    fs::write(&path, b"unsigned copy").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
    assert!(inspect_root_owned_input(&path, ArtifactDigest::of(b"unsigned copy")).is_err());
}
