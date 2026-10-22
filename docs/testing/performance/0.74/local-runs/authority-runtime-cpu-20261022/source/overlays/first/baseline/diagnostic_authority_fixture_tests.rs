use super::super::tests::checked;
use super::*;
use std::os::unix::fs::symlink;
use tempfile::TempDir;

#[test]
fn authority_publication_checkpoint_preserves_fault_and_scope() {
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = calls.clone();
    publication_checkpoint::with(
        move |stage| {
            assert_eq!(stage, PublicationStage::RecordCreated);
            observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        },
        || {
            assert!(matches!(
                fail(
                    Some(PublicationStage::RecordCreated),
                    PublicationStage::RecordCreated
                ),
                Err(AuthorityError::Injected(PublicationStage::RecordCreated))
            ));
        },
    );
    fail(None, PublicationStage::RecordCreated).unwrap();
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn authority_publication_checkpoint_order_matches_original_refusal_protocol() {
    let (dir, uid, gid) = fixture();
    let p = checked(7);
    let mut registry = initialize_fixture_registry(dir.path(), uid, gid, &p).unwrap();
    let stages = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let observed = stages.clone();
    publication_checkpoint::with(
        move |stage| observed.lock().unwrap().push(stage),
        || registry.refuse(RefusalReason::Policy).unwrap(),
    );
    assert_eq!(
        *stages.lock().unwrap(),
        [
            PublicationStage::RecordCreated,
            PublicationStage::RecordSynced,
            PublicationStage::MarkerCreated,
            PublicationStage::DurableCommit,
        ]
    );
    assert!(registry.inspect(&p.trust, &p.context).unwrap().refused);
}

fn fixture() -> (TempDir, u32, u32) {
    let dir = tempfile::Builder::new()
        .prefix("hydracache-authority-")
        .tempdir_in("/tmp")
        .unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    // SAFETY: scalar queries for the nonroot local fixture owner only.
    let uid = unsafe { libc::geteuid() };
    let gid = unsafe { libc::getegid() };
    assert!(uid > 0 && gid > 0);
    (dir, uid, gid)
}

#[test]
fn authority_fixture_close_reopen_refusal_and_explicit_higher_epoch_are_persistent() {
    let (dir, uid, gid) = fixture();
    let old = checked(7);
    let new = checked(8);
    assert!(open_fixture_registry(dir.path(), uid, gid, &old.trust, &old.context).is_err());
    let mut registry = initialize_fixture_registry(dir.path(), uid, gid, &old).unwrap();
    assert!(!registry.inspect(&old.trust, &old.context).unwrap().refused);
    registry.refuse(RefusalReason::Credentials).unwrap();
    drop(registry);
    let mut registry =
        open_fixture_registry(dir.path(), uid, gid, &old.trust, &old.context).unwrap();
    assert!(registry.inspect(&old.trust, &old.context).unwrap().refused);
    registry.advance(&new).unwrap();
    assert_eq!(
        registry.inspect(&new.trust, &new.context).unwrap().revision,
        3
    );
    drop(registry);
    assert!(open_fixture_registry(dir.path(), uid, gid, &old.trust, &old.context).is_err());
    let mut registry =
        open_fixture_registry(dir.path(), uid, gid, &new.trust, &new.context).unwrap();
    assert!(!registry.inspect(&new.trust, &new.context).unwrap().refused);
    drop(registry);
    assert!(initialize_fixture_registry(dir.path(), uid, gid, &new).is_err());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 6);
}

#[test]
fn authority_fixture_logical_faults_retain_partial_tail_or_committed_refusal_without_replay() {
    for stage in [
        PublicationStage::RecordCreated,
        PublicationStage::RecordSynced,
        PublicationStage::MarkerCreated,
        PublicationStage::DurableCommit,
    ] {
        let (dir, uid, gid) = fixture();
        let p = checked(7);
        let mut registry = initialize_fixture_registry(dir.path(), uid, gid, &p).unwrap();
        let before = registry.entries[0].record.raw.clone();
        assert!(
            matches!(registry.change(|record| record.refuse(RefusalReason::Storage), Some(stage)), Err(AuthorityError::Injected(at)) if at == stage)
        );
        assert!(registry.is_failed());
        assert!(matches!(registry.view(), Err(AuthorityError::Refused)));
        let entries = fs::read_dir(dir.path()).unwrap().count();
        assert!(matches!(
            registry.refuse(RefusalReason::Operator),
            Err(AuthorityError::Refused)
        ));
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), entries);
        assert_eq!(fs::read(dir.path().join(name(1, "json"))).unwrap(), before);
        drop(registry);
        let reopened = open_fixture_registry(dir.path(), uid, gid, &p.trust, &p.context);
        if stage == PublicationStage::DurableCommit {
            assert!(reopened.unwrap().view().unwrap().refused);
        } else {
            assert!(reopened.is_err());
        }
    }
}

#[test]
fn authority_fixture_owner_drop_unlocks_with_inherited_description_still_open() {
    let (dir, uid, gid) = fixture();
    let p = checked(7);
    let registry = initialize_fixture_registry(dir.path(), uid, gid, &p).unwrap();
    // Same open file description as a fork-inherited descriptor, without forking.
    let inherited = registry.directory.leaf.try_clone().unwrap();
    assert!(matches!(
        open_fixture_registry(dir.path(), uid, gid, &p.trust, &p.context),
        Err(AuthorityError::Io(error)) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
    drop(registry);
    let mut reopened = open_fixture_registry(dir.path(), uid, gid, &p.trust, &p.context).unwrap();
    // Closing the old alias must not release a new writer's independent lock.
    drop(inherited);
    assert!(matches!(
        open_fixture_registry(dir.path(), uid, gid, &p.trust, &p.context),
        Err(AuthorityError::Io(error)) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
    reopened.refuse(RefusalReason::Operator).unwrap();
}

#[test]
fn authority_fixture_exclusive_writer_and_thread_local_origin_cannot_be_duplicated() {
    let (dir, uid, gid) = fixture();
    let p = checked(7);
    let registry = initialize_fixture_registry(dir.path(), uid, gid, &p).unwrap();
    let path = dir.path().to_owned();
    let joined = std::thread::spawn(move || {
        let p = checked(7);
        assert!(
            matches!(open_fixture_registry(&path, uid, gid, &p.trust, &p.context),
            Err(AuthorityError::Io(error)) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
    });
    joined.join().unwrap();
    drop(registry);
    open_fixture_registry(dir.path(), uid, gid, &p.trust, &p.context).unwrap();
    trait AmbiguousSend<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSend<()> for T {}
    impl<T: ?Sized + Send> AmbiguousSend<u8> for T {}
    let _ = <FixtureAuthorityRegistry as AmbiguousSend<_>>::marker;
    trait AmbiguousSync<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSync<()> for T {}
    impl<T: ?Sized + Sync> AmbiguousSync<u8> for T {}
    let _ = <FixtureAuthorityRegistry as AmbiguousSync<_>>::marker;
}

#[test]
fn authority_fixture_original_path_file_and_descriptor_drift_latch_without_refresh() {
    for case in 0..4 {
        let (dir, uid, gid) = fixture();
        let p = checked(7);
        let mut registry = initialize_fixture_registry(dir.path(), uid, gid, &p).unwrap();
        let path = dir.path().join(name(1, "json"));
        match case {
            0 => {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
                fs::write(&path, &registry.entries[0].record.raw).unwrap();
                // A same-byte rewrite can share the original filesystem clock tick.
                // Force observable metadata drift; no claim of transient owner tamper detection.
                File::open(&path)
                    .unwrap()
                    .set_times(fs::FileTimes::new().set_modified(std::time::SystemTime::UNIX_EPOCH))
                    .unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
                assert!(
                    Stamp::of(&fs::metadata(&path).unwrap()) != registry.entries[0].record.stamp
                );
            }
            1 => registry.entries[0].record.file = File::open("/dev/null").unwrap(),
            2 => fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap(),
            3 => {
                let mut replacement = create_new(&registry.directory.leaf, "replacement").unwrap();
                write_seal(&mut replacement, &registry.entries[0].record.raw).unwrap();
                fs::rename(dir.path().join("replacement"), &path).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(registry.inspect(&p.trust, &p.context).is_err());
        assert!(registry.is_failed());
        assert!(matches!(
            registry.inspect(&p.trust, &p.context),
            Err(AuthorityError::Refused)
        ));
    }
}

#[test]
fn authority_fixture_unsafe_linked_special_and_oversized_records_refuse() {
    for case in 0..6 {
        let (dir, uid, gid) = fixture();
        let p = checked(7);
        let registry = initialize_fixture_registry(dir.path(), uid, gid, &p).unwrap();
        drop(registry);
        let record = dir.path().join(name(1, "json"));
        match case {
            0 => {
                fs::remove_file(&record).unwrap();
                symlink("/dev/null", &record).unwrap();
            }
            1 => fs::hard_link(&record, dir.path().join("another")).unwrap(),
            2 => fs::set_permissions(&record, fs::Permissions::from_mode(0o600)).unwrap(),
            3 => {
                fs::remove_file(&record).unwrap();
                let path = CString::new(record.to_str().unwrap()).unwrap();
                // SAFETY: fixture-owned fixed path, no unrelated file operations.
                assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o400) }, 0);
            }
            4 => {
                fs::set_permissions(&record, fs::Permissions::from_mode(0o600)).unwrap();
                fs::write(&record, vec![b'x'; MAX_RECORD_BYTES + 1]).unwrap();
                fs::set_permissions(&record, fs::Permissions::from_mode(0o400)).unwrap();
            }
            5 => {
                let marker = dir.path().join(name(1, "commit"));
                fs::set_permissions(&marker, fs::Permissions::from_mode(0o600)).unwrap();
                fs::write(&marker, b"wrong\n").unwrap();
                fs::set_permissions(&marker, fs::Permissions::from_mode(0o400)).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(open_fixture_registry(dir.path(), uid, gid, &p.trust, &p.context).is_err());
    }
}

#[test]
fn authority_fixture_wrong_owner_path_and_external_pins_never_choose_authority() {
    let p = checked(7);
    for path in [
        "/",
        "/tmp",
        "/var/lib/hydracache-performance",
        "/opt/hydracache-performance",
        "/tmp/hydracache-authority-x/..",
        "relative",
    ] {
        assert!(initialize_fixture_registry(Path::new(path), 1000, 1000, &p).is_err());
    }
    let deep = format!("/tmp/hydracache-authority-x/{}", "x/".repeat(10_000));
    assert!(initialize_fixture_registry(Path::new(&deep), 1000, 1000, &p).is_err());
    let (dir, uid, gid) = fixture();
    assert!(initialize_fixture_registry(dir.path(), 0, 0, &p).is_err());
    assert!(initialize_fixture_registry(dir.path(), uid + 1, gid, &p).is_err());
    let mut registry = initialize_fixture_registry(dir.path(), uid, gid, &p).unwrap();
    let newer = checked(8);
    assert!(matches!(
        registry.inspect(&newer.trust, &newer.context),
        Err(AuthorityError::Binding)
    ));
    assert!(matches!(
        registry.inspect(&p.trust, &p.context),
        Err(AuthorityError::Refused)
    ));
}

#[test]
fn authority_fixture_bounded_inventory_and_invalid_transition_make_no_new_generation() {
    let (dir, uid, gid) = fixture();
    let p = checked(7);
    let mut registry = initialize_fixture_registry(dir.path(), uid, gid, &p).unwrap();
    assert!(matches!(
        registry.advance(&checked(8)),
        Err(AuthorityError::Transition)
    ));
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    assert!(matches!(
        registry.refuse(RefusalReason::Operator),
        Err(AuthorityError::Refused)
    ));
    drop(registry);
    for i in 0..255 {
        fs::write(dir.path().join(format!("extra-{i}")), b"").unwrap();
    }
    assert!(matches!(
        open_fixture_registry(dir.path(), uid, gid, &p.trust, &p.context),
        Err(AuthorityError::Budget)
    ));
}

#[test]
fn authority_fixture_partial_write_errors_preserve_bytes_and_do_not_complete_a_marker() {
    struct Partial {
        bytes: Vec<u8>,
    }
    impl Write for Partial {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if !self.bytes.is_empty() {
                return Err(std::io::Error::from_raw_os_error(libc::ENOSPC));
            }
            let n = bytes.len().min(3);
            self.bytes.extend_from_slice(&bytes[..n]);
            Ok(n)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Partial { bytes: vec![] };
    assert!(matches!(write_bytes(&mut writer, b"abcdef"),
        Err(AuthorityError::Io(error)) if error.raw_os_error() == Some(libc::ENOSPC)));
    assert_eq!(writer.bytes, b"abc");
}

#[path = "../tests/support/runtime_cpu_probe.rs"]
mod runtime_cpu;
