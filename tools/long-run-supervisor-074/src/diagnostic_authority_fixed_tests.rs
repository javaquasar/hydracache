use super::*;
use crate::diagnostic_worker_policy::authority_registry::tests::checked;
use std::os::unix::fs::symlink;
use tempfile::TempDir;

// Private test-only origin, never the public fixed path, and never root enrollment.
fn modeled_tree() -> (TempDir, u32, u32) {
    let dir = tempfile::Builder::new()
        .prefix("hydracache-fixed-authority-")
        .tempdir_in("/tmp")
        .unwrap();
    let uid = unsafe { libc::geteuid() };
    let gid = unsafe { libc::getegid() };
    assert!(uid > 0 && gid > 0);
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut path = dir.path().to_owned();
    for name in ANCESTORS.into_iter().chain([FIXED_LEAF]) {
        path.push(name);
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    (dir, uid, gid)
}
fn modeled_directory(dir: &TempDir, uid: u32, gid: u32) -> Result<Directory, AuthorityError> {
    let root = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(dir.path())?;
    walk(root, uid, gid, dir.path())
}
fn leaf(dir: &TempDir) -> std::path::PathBuf {
    dir.path()
        .join("var/lib/hydracache-performance/worker-authority-074")
}
fn modeled_initialize(dir: &TempDir, uid: u32, gid: u32) -> FixedAuthorityRegistry {
    FixedAuthorityRegistry {
        journal: initialize_directory(modeled_directory(dir, uid, gid).unwrap(), &checked(7))
            .unwrap(),
    }
}

#[test]
fn authority_fixed_metadata_requires_root_owner_safe_ancestors_and_private_leaf() {
    let (dir, _, _) = modeled_tree();
    let mut stamp = Stamp::of(&fs::metadata(dir.path()).unwrap());
    stamp.uid = 0;
    stamp.gid = 0;
    for mode in [0o700, 0o750, 0o755, 0o555] {
        stamp.mode = libc::S_IFDIR | mode;
        assert!(safe_directory(&stamp, 0, 0, false));
        assert_eq!(safe_directory(&stamp, 0, 0, true), mode == 0o700);
    }
    for mode in [0o777, 0o770, 0o702, 0o1777, 0o2700, 0o4700] {
        stamp.mode = libc::S_IFDIR | mode;
        assert!(!safe_directory(&stamp, 0, 0, false));
        assert!(!safe_directory(&stamp, 0, 0, true));
    }
    stamp.mode = libc::S_IFREG | 0o700;
    assert!(!safe_directory(&stamp, 0, 0, false));
    stamp.mode = libc::S_IFDIR | 0o700;
    stamp.uid = 1;
    assert!(!safe_directory(&stamp, 0, 0, false));
    stamp.uid = 0;
    stamp.gid = 1;
    assert!(!safe_directory(&stamp, 0, 0, true));
}

#[test]
fn authority_fixed_modeled_writer_persists_and_exclusively_locks_shared_engine() {
    let (dir, uid, gid) = modeled_tree();
    let mut writer = modeled_initialize(&dir, uid, gid);
    let p = checked(7);
    assert_eq!(writer.inspect(&p.trust, &p.context).unwrap().revision, 1);
    assert!(matches!(modeled_directory(&dir, uid, gid),
        Err(AuthorityError::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock));
    writer.refuse(RefusalReason::Operator).unwrap();
    drop(writer);
    let mut writer = FixedAuthorityRegistry {
        journal: open_directory(
            modeled_directory(&dir, uid, gid).unwrap(),
            &p.trust,
            &p.context,
        )
        .unwrap(),
    };
    assert!(writer.inspect(&p.trust, &p.context).unwrap().refused);
    let next = checked(8);
    writer.advance(&next).unwrap();
    assert_eq!(
        writer.inspect(&next.trust, &next.context).unwrap().revision,
        3
    );
    assert!(!writer.is_failed());
    drop(writer);
    assert!(initialize_directory(modeled_directory(&dir, uid, gid).unwrap(), &next).is_err());
    assert!(open_directory(
        modeled_directory(&dir, uid, gid).unwrap(),
        &p.trust,
        &p.context
    )
    .is_err());
    assert_eq!(fs::read_dir(leaf(&dir)).unwrap().count(), 6);
}

#[test]
fn authority_fixed_every_original_ancestor_and_root_drift_latches_without_new_record() {
    for level in 0..=3 {
        for descriptor_drift in [false, true] {
            let (dir, uid, gid) = modeled_tree();
            let mut writer = modeled_initialize(&dir, uid, gid);
            if descriptor_drift {
                let replacement = File::open("/tmp").unwrap();
                if level == 0 {
                    writer.journal.directory.root = replacement;
                } else {
                    writer
                        .journal
                        .directory
                        .fixed_ancestry
                        .as_mut()
                        .unwrap()
                        .ancestors[level - 1]
                        .file = replacement;
                }
            } else {
                let mut path = dir.path().to_owned();
                for name in ANCESTORS.into_iter().take(level) {
                    path.push(name);
                }
                fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
            }
            assert!(writer.refuse(RefusalReason::Context).is_err());
            assert!(writer.is_failed());
            assert!(matches!(writer.view(), Err(AuthorityError::Refused)));
            assert!(matches!(
                writer.refuse(RefusalReason::Operator),
                Err(AuthorityError::Refused)
            ));
            assert_eq!(fs::read_dir(leaf(&dir)).unwrap().count(), 2);
        }
    }
}

#[test]
fn authority_fixed_named_ancestor_replacement_is_not_adopted() {
    for level in 1..=3 {
        let (dir, uid, gid) = modeled_tree();
        let mut writer = modeled_initialize(&dir, uid, gid);
        let mut path = dir.path().to_owned();
        for name in ANCESTORS.into_iter().take(level) {
            path.push(name);
        }
        let old = path.with_extension("old");
        fs::rename(&path, &old).unwrap();
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let p = checked(7);
        assert!(writer.inspect(&p.trust, &p.context).is_err());
        assert!(writer.is_failed());
        assert_eq!(fs::read_dir(path).unwrap().count(), 0);
    }
}

#[test]
fn authority_fixed_named_root_replacement_is_not_hidden_by_original_descriptor() {
    let (dir, uid, gid) = modeled_tree();
    let mut writer = modeled_initialize(&dir, uid, gid);
    let backup = tempfile::Builder::new()
        .prefix("hydracache-fixed-authority-backup-")
        .tempdir_in("/tmp")
        .unwrap();
    fs::rename(dir.path(), backup.path().join("original")).unwrap();
    fs::create_dir(dir.path()).unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let p = checked(7);
    assert!(writer.inspect(&p.trust, &p.context).is_err());
    assert!(writer.is_failed());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn authority_fixed_leaf_and_parent_alias_drift_latch_but_benign_parent_entries_do_not() {
    for case in 0..4 {
        let (dir, uid, gid) = modeled_tree();
        let mut writer = modeled_initialize(&dir, uid, gid);
        let p = checked(7);
        // Mutable timestamps/link counts are not object or ownership changes.
        fs::create_dir(dir.path().join("unrelated")).unwrap();
        fs::create_dir(leaf(&dir).parent().unwrap().join("unrelated")).unwrap();
        writer.inspect(&p.trust, &p.context).unwrap();
        match case {
            0 => fs::set_permissions(leaf(&dir), fs::Permissions::from_mode(0o755)).unwrap(),
            1 => writer.journal.directory.temporary = File::open("/tmp").unwrap(),
            2 => writer.journal.directory.leaf = File::open("/tmp").unwrap(),
            3 => {
                fs::rename(leaf(&dir), leaf(&dir).with_extension("old")).unwrap();
                fs::create_dir(leaf(&dir)).unwrap();
                fs::set_permissions(leaf(&dir), fs::Permissions::from_mode(0o700)).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(writer.inspect(&p.trust, &p.context).is_err());
        assert!(writer.is_failed());
        assert!(matches!(
            writer.refuse(RefusalReason::Storage),
            Err(AuthorityError::Refused)
        ));
    }
}

#[test]
fn authority_fixed_walker_rejects_symlinks_missing_unsafe_and_wrong_owner_components() {
    for level in 1..=4 {
        for case in 0..3 {
            let (dir, uid, gid) = modeled_tree();
            let mut path = dir.path().to_owned();
            for name in ANCESTORS.into_iter().chain([FIXED_LEAF]).take(level) {
                path.push(name);
            }
            match case {
                0 => {
                    let old = path.with_extension("old");
                    fs::rename(&path, &old).unwrap();
                    symlink(&old, &path).unwrap();
                }
                1 => {
                    fs::rename(&path, path.with_extension("old")).unwrap();
                }
                2 => fs::set_permissions(&path, fs::Permissions::from_mode(0o770)).unwrap(),
                _ => unreachable!(),
            }
            assert!(modeled_directory(&dir, uid, gid).is_err());
        }
    }
    let (dir, uid, gid) = modeled_tree();
    assert!(modeled_directory(&dir, uid + 1, gid).is_err());
    assert!(modeled_directory(&dir, uid, gid + 1).is_err());
    assert!(modeled_directory(&dir, 0, 0).is_err());
}

#[test]
fn authority_fixed_modeled_publication_faults_do_not_repair_partial_tail() {
    for stage in [
        PublicationStage::RecordCreated,
        PublicationStage::RecordSynced,
        PublicationStage::MarkerCreated,
        PublicationStage::DurableCommit,
    ] {
        let (dir, uid, gid) = modeled_tree();
        let mut writer = modeled_initialize(&dir, uid, gid);
        assert!(
            matches!(writer.journal.change(|r| r.refuse(RefusalReason::Storage), Some(stage)),
            Err(AuthorityError::Injected(at)) if at == stage)
        );
        assert!(writer.is_failed());
        drop(writer);
        let p = checked(7);
        let reopened = open_directory(
            modeled_directory(&dir, uid, gid).unwrap(),
            &p.trust,
            &p.context,
        );
        if stage == PublicationStage::DurableCommit {
            assert!(reopened.unwrap().view().unwrap().refused);
        } else {
            assert!(reopened.is_err());
        }
    }
}

#[test]
fn authority_fixed_guard_is_not_send_sync_or_fixture_conversion() {
    trait AmbiguousSend<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSend<()> for T {}
    impl<T: ?Sized + Send> AmbiguousSend<u8> for T {}
    let _ = <FixedAuthorityRegistry as AmbiguousSend<_>>::marker;
    trait AmbiguousSync<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSync<()> for T {}
    impl<T: ?Sized + Sync> AmbiguousSync<u8> for T {}
    let _ = <FixedAuthorityRegistry as AmbiguousSync<_>>::marker;
    assert_eq!(
        FIXED_AUTHORITY_PATH,
        format!("/{}/{}", ANCESTORS.join("/"), FIXED_LEAF)
    );
}
