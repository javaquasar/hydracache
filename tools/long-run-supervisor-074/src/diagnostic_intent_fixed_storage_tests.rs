use super::*;
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};

pub(super) fn tree() -> (tempfile::TempDir, u32, u32) {
    let temp = tempfile::Builder::new()
        .prefix("hydracache-fixed-intent-")
        .tempdir_in("/tmp")
        .unwrap();
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut path = temp.path().to_owned();
    for name in COMPONENTS {
        path.push(name);
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let uid = unsafe { libc::geteuid() };
    let gid = unsafe { libc::getegid() };
    assert!(uid > 0 && gid > 0, "modeled tests must remain nonroot");
    (temp, uid, gid)
}
pub(super) fn at(root: &Path, level: usize) -> PathBuf {
    COMPONENTS
        .iter()
        .take(level)
        .fold(root.into(), |p, n| p.join(n))
}

#[test]
fn intent_fixed_storage_metadata_accepts_only_safe_original_root_policy() {
    let (temp, _, _) = tree();
    let mut stamp = Stamp::of(&fs::metadata(temp.path()).unwrap());
    stamp.uid = 0;
    stamp.gid = 0;
    for level in 0..=4 {
        for mode in [
            0o700, 0o750, 0o755, 0o555, 0o770, 0o702, 0o1777, 0o2700, 0o4700,
        ] {
            stamp.mode = libc::S_IFDIR | mode;
            assert_eq!(
                stamp.safe(0, 0, level),
                if level == 4 {
                    mode == 0o700
                } else {
                    mode & 0o7022 == 0
                }
            );
        }
        stamp.mode = libc::S_IFDIR | 0o700;
        stamp.gid = 986;
        assert_eq!(stamp.safe(0, 0, level), level == 3);
        stamp.gid = 0;
        stamp.uid = 986;
        assert!(!stamp.safe(0, 0, level));
        stamp.uid = 0;
        stamp.mode = libc::S_IFREG | 0o700;
        assert!(!stamp.safe(0, 0, level));
    }
}
#[test]
fn intent_fixed_storage_modeled_walk_is_readonly_and_tolerates_directory_growth() {
    let (temp, uid, gid) = tree();
    let mut read = walk(temp.path(), uid, gid).unwrap();
    for level in 0..=4 {
        fs::create_dir(at(temp.path(), level).join("unrelated")).unwrap();
        read.revalidate().unwrap();
    }
    assert!(!read.is_refused());
    drop(read);
    assert_eq!(fs::read_dir(at(temp.path(), 4)).unwrap().count(), 1);
    walk(temp.path(), uid, gid).unwrap().revalidate().unwrap();
}
#[test]
fn intent_fixed_storage_every_original_descriptor_replacement_latches() {
    for level in 0..=4 {
        let (temp, uid, gid) = tree();
        let mut read = walk(temp.path(), uid, gid).unwrap();
        let original = if level == 0 {
            &mut read.root
        } else {
            &mut read.components[level - 1]
        };
        let old = std::mem::replace(&mut original.file, File::open("/tmp").unwrap());
        assert!(read.revalidate().is_err());
        assert!(read.is_refused());
        if level == 0 {
            read.root.file = old;
        } else {
            read.components[level - 1].file = old;
        }
        assert!(matches!(
            read.revalidate(),
            Err(FixedIntentStorageError::Refused)
        ));
    }
}
#[test]
fn intent_fixed_storage_every_named_directory_replacement_refuses_adoption() {
    for level in 0..=4 {
        let (temp, uid, gid) = tree();
        let mut read = walk(temp.path(), uid, gid).unwrap();
        let path = at(temp.path(), level);
        let backup = tempfile::tempdir_in("/tmp").unwrap();
        fs::rename(&path, backup.path().join("original")).unwrap();
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(read.revalidate().is_err());
        assert!(read.is_refused());
        assert_eq!(fs::read_dir(&path).unwrap().count(), 0);
    }
}
#[test]
fn intent_fixed_storage_unsafe_missing_symlink_and_wrong_owner_refuse() {
    for level in 1..=4 {
        for mutation in 0..3 {
            let (temp, uid, gid) = tree();
            let path = at(temp.path(), level);
            let backup = path.with_extension("retained");
            match mutation {
                0 => {
                    fs::rename(&path, &backup).unwrap();
                    symlink(&backup, &path).unwrap();
                }
                1 => {
                    fs::rename(&path, &backup).unwrap();
                }
                _ => fs::set_permissions(&path, fs::Permissions::from_mode(0o770)).unwrap(),
            }
            assert!(walk(temp.path(), uid, gid).is_err());
        }
    }
    let (temp, uid, gid) = tree();
    assert!(walk(temp.path(), uid + 1, gid).is_err());
    assert!(walk(temp.path(), uid, gid + 1).is_err());
    assert!(walk(temp.path(), 0, 0).is_err());
}
#[test]
fn intent_fixed_storage_restoring_modes_cannot_refresh_original_guard() {
    for level in 0..=4 {
        let (temp, uid, gid) = tree();
        let mut read = walk(temp.path(), uid, gid).unwrap();
        let path = at(temp.path(), level);
        // Even another safe mode changes the original policy; never refresh it.
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(read.revalidate().is_err());
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(matches!(
            read.revalidate(),
            Err(FixedIntentStorageError::Refused)
        ));
    }
}
#[test]
fn intent_fixed_storage_public_guard_is_original_thread_only() {
    trait AmbiguousSend<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSend<()> for T {}
    impl<T: ?Sized + Send> AmbiguousSend<u8> for T {}
    let _ = <FixedIntentStorageRead as AmbiguousSend<_>>::marker;
    trait AmbiguousSync<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSync<()> for T {}
    impl<T: ?Sized + Sync> AmbiguousSync<u8> for T {}
    let _ = <FixedIntentStorageRead as AmbiguousSync<_>>::marker;
    assert_eq!(
        FIXED_INTENT_STORAGE_PATH,
        format!("/{}", COMPONENTS.join("/"))
    );
}
