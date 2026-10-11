use super::super::tests::{at, tree};
use super::*;
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;

fn fixture() -> (tempfile::TempDir, PathBuf, File) {
    let (temp, _, _) = tree();
    let root = at(temp.path(), 3).join("campaigns");
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(root.join(HOST_EXECUTION_LOCK_NAME), []).unwrap();
    fs::set_permissions(
        root.join(HOST_EXECUTION_LOCK_NAME),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let (_, lock) = crate::host_execution::lock_host_root(&root).unwrap();
    (temp, root, lock)
}

#[test]
fn fixed_admission_scan_scan_reopens_offsets_and_nonutf8_entries_also_block() {
    let (temp, root, lock) = fixture();
    let mut read = FixedCampaignAdmission::modeled(temp.path(), &root, &lock).unwrap();
    assert!(!read.present(&root, &lock).unwrap());
    assert!(!read.present(&root, &lock).unwrap());
    let name = std::ffi::OsString::from_vec(vec![0xff, b'x']);
    fs::write(at(temp.path(), 4).join(name), []).unwrap();
    assert!(read.present(&root, &lock).unwrap());
    assert!(read.present(&root, &lock).unwrap());
    assert!(
        !read.storage.is_refused(),
        "presence is exclusion, not ownership revocation"
    );
}

#[test]
fn fixed_admission_scan_original_storage_descriptor_or_lock_argument_drift_latches() {
    for descriptor in [false, true] {
        let (temp, root, lock) = fixture();
        let mut read = FixedCampaignAdmission::modeled(temp.path(), &root, &lock).unwrap();
        if descriptor {
            let old = std::mem::replace(
                &mut read.storage.components[3].file,
                File::open("/tmp").unwrap(),
            );
            assert!(read.present(&root, &lock).is_err());
            read.storage.components[3].file = old;
        } else {
            let path = temp.path().join("foreign-lock");
            fs::write(&path, []).unwrap();
            assert!(read.present(&root, &File::open(path).unwrap()).is_err());
        }
        assert!(read.storage.is_refused());
        assert!(read.present(&root, &lock).is_err());
    }
}

#[test]
fn fixed_admission_scan_world_writable_hardlinked_lock_or_foreign_root_refuses() {
    for kind in 0..3 {
        let (temp, root, lock) = fixture();
        let argument = if kind == 2 { temp.path() } else { &root };
        match kind {
            0 => fs::set_permissions(
                root.join(HOST_EXECUTION_LOCK_NAME),
                fs::Permissions::from_mode(0o666),
            )
            .unwrap(),
            1 => fs::hard_link(
                root.join(HOST_EXECUTION_LOCK_NAME),
                temp.path().join("alias"),
            )
            .unwrap(),
            _ => {}
        }
        assert!(FixedCampaignAdmission::modeled(temp.path(), argument, &lock).is_err());
        assert!(!root.join("active-campaign").exists());
    }
}

#[test]
fn fixed_admission_scan_directory_stream_refuses_non_directory_without_repair() {
    let (temp, root, _) = fixture();
    let file = File::open(root.join(HOST_EXECUTION_LOCK_NAME)).unwrap();
    assert!(matches!(
        directory_present(file),
        Err(HostExecutionError::Io(_))
    ));
    assert!(fs::read(root.join(HOST_EXECUTION_LOCK_NAME))
        .unwrap()
        .is_empty());
    assert!(!directory_present(File::open(at(temp.path(), 4)).unwrap()).unwrap());
}
