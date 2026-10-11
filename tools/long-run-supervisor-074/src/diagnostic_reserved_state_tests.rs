use super::super::{
    tests::{at, tree},
    walk,
};
use super::*;
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};

fn state() -> DiagnosticState {
    use crate::diagnostic_lease::DiagnosticIdentity;
    DiagnosticState {
        identity: DiagnosticIdentity {
            lease_id: "a".repeat(64),
            boot_id: "11111111-2222-3333-4444-555555555555".into(),
            binary_sha256: "b".repeat(64),
            build_provenance_sha256: "c".repeat(64),
        },
        revision: 1,
        stage: DiagnosticStage::Reserved,
        completed_cells: 0,
        reserved_monotonic_ns: 1,
        last_observed_monotonic_ns: 2,
        controller_monotonic_ns: 2,
        cell_started_monotonic_ns: None,
        cgroup_inode: None,
        reason: None,
        cleanup_confirmed: true,
        promotable: false,
        admission_allowed: false,
    }
}
fn encoded(state: &DiagnosticState) -> Vec<u8> {
    let envelope = serde_json::json!({"schema_version":1, "state":state,
        "state_sha256":crate::sha256_hex(&crate::canonical_json(state).unwrap())});
    let mut raw = crate::canonical_json(&envelope).unwrap();
    raw.push(b'\n');
    raw
}
fn persist(root: &std::path::Path, state: &DiagnosticState) {
    let name = at(root, 4).join(ACTIVE_DIAGNOSTIC_NAME);
    fs::write(&name, encoded(state)).unwrap();
    fs::set_permissions(name, fs::Permissions::from_mode(0o600)).unwrap();
}
fn inventory(root: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut entries: Vec<_> = fs::read_dir(at(root, 4))
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().into_string().unwrap(),
                fs::read(e.path()).unwrap(),
            )
        })
        .collect();
    entries.sort();
    entries
}
#[test]
fn reserved_state_modeled_original_read_and_healthy_drop_never_write() {
    let (temp, uid, gid) = tree();
    let expected = state();
    persist(temp.path(), &expected);
    let before = inventory(temp.path());
    let mut storage = walk(temp.path(), uid, gid).unwrap();
    {
        let mut read = bind_fixed_reserved_state(&mut storage, &expected).unwrap();
        read.revalidate(&expected).unwrap();
        fs::create_dir(at(temp.path(), 2).join("unrelated")).unwrap();
        read.revalidate(&expected).unwrap();
        assert!(!read.is_refused());
    }
    storage.revalidate().unwrap();
    assert_eq!(inventory(temp.path()), before);
}
#[test]
fn reserved_state_invalid_initial_assertion_refuses_without_revoking_storage() {
    let (temp, uid, gid) = tree();
    let mut storage = walk(temp.path(), uid, gid).unwrap();
    for kind in 0..5 {
        let mut expected = state();
        match kind {
            0 => expected.revision = 0,
            1 => expected.stage = DiagnosticStage::Starting,
            2 => expected.admission_allowed = true,
            3 => expected.completed_cells = 4,
            _ => expected.identity.boot_id = "foreign".into(),
        }
        assert!(matches!(
            bind_fixed_reserved_state(&mut storage, &expected),
            Err(FixedReservedStateError::Expected)
        ));
        assert!(!storage.is_refused());
        assert_eq!(fs::read_dir(at(temp.path(), 4)).unwrap().count(), 0);
    }
}
#[test]
fn reserved_state_missing_unsafe_or_nonregular_documents_latch_without_repair() {
    for kind in 0..7 {
        let (temp, uid, gid) = tree();
        let expected = state();
        let name = at(temp.path(), 4).join(ACTIVE_DIAGNOSTIC_NAME);
        match kind {
            0 => {}
            1 => {
                fs::create_dir(&name).unwrap();
            }
            2 => {
                let target = temp.path().join("other");
                fs::write(&target, encoded(&expected)).unwrap();
                symlink(&target, &name).unwrap();
            }
            3 => {
                let name = std::ffi::CString::new(name.as_os_str().as_encoded_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
            }
            4 => {
                persist(temp.path(), &expected);
                fs::set_permissions(&name, fs::Permissions::from_mode(0o640)).unwrap();
            }
            5 => {
                persist(temp.path(), &expected);
                fs::hard_link(&name, temp.path().join("alias")).unwrap();
            }
            _ => {
                fs::write(&name, vec![b'x'; MAX_BYTES as usize + 1]).unwrap();
                fs::set_permissions(&name, fs::Permissions::from_mode(0o600)).unwrap();
            }
        }
        let mut storage = walk(temp.path(), uid, gid).unwrap();
        assert!(bind_fixed_reserved_state(&mut storage, &expected).is_err());
        assert!(storage.is_refused());
        assert!(matches!(
            bind_fixed_reserved_state(&mut storage, &expected),
            Err(FixedReservedStateError::Refused)
        ));
    }
}
#[test]
fn reserved_state_canonical_schema_hash_and_full_snapshot_are_required() {
    for kind in 0..6 {
        let (temp, uid, gid) = tree();
        let expected = state();
        persist(temp.path(), &expected);
        let mut raw = encoded(&expected);
        match kind {
            0 => {
                raw.pop();
            }
            1 => {
                raw.push(b' ');
            }
            2 | 3 => {
                let mut value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
                if kind == 2 {
                    value["schema_version"] = 2.into();
                } else {
                    value["state_sha256"] = "0".repeat(64).into();
                }
                raw = crate::canonical_json(&value).unwrap();
                raw.push(b'\n');
            }
            4 => {
                let mut changed = expected.clone();
                changed.revision += 1;
                raw = encoded(&changed);
            }
            _ => {
                let mut value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
                value["extra"] = true.into();
                raw = crate::canonical_json(&value).unwrap();
                raw.push(b'\n');
            }
        }
        fs::write(at(temp.path(), 4).join(ACTIVE_DIAGNOSTIC_NAME), &raw).unwrap();
        let mut storage = walk(temp.path(), uid, gid).unwrap();
        assert!(bind_fixed_reserved_state(&mut storage, &expected).is_err());
        assert!(storage.is_refused());
        assert_eq!(
            fs::read(at(temp.path(), 4).join(ACTIVE_DIAGNOSTIC_NAME)).unwrap(),
            raw
        );
    }
}
#[test]
fn reserved_state_same_bytes_new_inode_or_original_descriptor_never_adopted() {
    for descriptor in [false, true] {
        let (temp, uid, gid) = tree();
        let expected = state();
        persist(temp.path(), &expected);
        let mut storage = walk(temp.path(), uid, gid).unwrap();
        let mut read = bind_fixed_reserved_state(&mut storage, &expected).unwrap();
        if descriptor {
            let other = temp.path().join("other");
            fs::write(&other, encoded(&expected)).unwrap();
            fs::set_permissions(&other, fs::Permissions::from_mode(0o600)).unwrap();
            let original = std::mem::replace(&mut read.document.file, File::open(other).unwrap());
            assert!(read.revalidate(&expected).is_err());
            read.document.file = original;
        } else {
            fs::rename(
                at(temp.path(), 4).join(ACTIVE_DIAGNOSTIC_NAME),
                temp.path().join("original"),
            )
            .unwrap();
            persist(temp.path(), &expected);
            assert!(read.revalidate(&expected).is_err());
        }
        assert!(matches!(
            read.revalidate(&expected),
            Err(FixedReservedStateError::Refused)
        ));
        drop(read);
        assert!(storage.is_refused());
    }
}
#[test]
fn reserved_state_seeded_expected_mutations_refuse_without_io_or_refresh() {
    let seed = 0x0745_u64;
    eprintln!("seed={seed:#x}");
    for kind in 0..8 {
        let (temp, uid, gid) = tree();
        let expected = state();
        persist(temp.path(), &expected);
        let before = inventory(temp.path());
        let mut storage = walk(temp.path(), uid, gid).unwrap();
        let mut read = bind_fixed_reserved_state(&mut storage, &expected).unwrap();
        let mut changed = expected.clone();
        match kind {
            0 => changed.revision += seed,
            1 => changed.completed_cells = 1,
            2 => changed.last_observed_monotonic_ns += seed,
            3 => changed.controller_monotonic_ns = 1,
            4 => changed.identity.binary_sha256 = "d".repeat(64),
            5 => changed.identity.lease_id = "e".repeat(64),
            6 => changed.identity.build_provenance_sha256 = "f".repeat(64),
            _ => changed.identity.boot_id = "66666666-2222-3333-4444-555555555555".into(),
        }
        // Remove the named file first: Expected, not file I/O, must win.
        fs::rename(
            at(temp.path(), 4).join(ACTIVE_DIAGNOSTIC_NAME),
            temp.path().join("original"),
        )
        .unwrap();
        assert!(matches!(
            read.revalidate(&changed),
            Err(FixedReservedStateError::Expected)
        ));
        fs::rename(
            temp.path().join("original"),
            at(temp.path(), 4).join(ACTIVE_DIAGNOSTIC_NAME),
        )
        .unwrap();
        assert!(matches!(
            read.revalidate(&expected),
            Err(FixedReservedStateError::Refused)
        ));
        assert_eq!(inventory(temp.path()), before);
    }
}
#[test]
fn reserved_state_pending_and_unhandled_context_refuse_before_and_after_binding() {
    for name in CONFLICTS {
        for after in [false, true] {
            let (temp, uid, gid) = tree();
            let expected = state();
            persist(temp.path(), &expected);
            let mut storage = walk(temp.path(), uid, gid).unwrap();
            let conflict = at(temp.path(), 4).join(name);
            if after {
                let mut read = bind_fixed_reserved_state(&mut storage, &expected).unwrap();
                symlink("missing", &conflict).unwrap();
                assert!(matches!(
                    read.revalidate(&expected),
                    Err(FixedReservedStateError::Conflict)
                ));
                fs::remove_file(&conflict).unwrap();
                assert!(matches!(
                    read.revalidate(&expected),
                    Err(FixedReservedStateError::Refused)
                ));
            } else {
                symlink("missing", &conflict).unwrap();
                assert!(matches!(
                    bind_fixed_reserved_state(&mut storage, &expected),
                    Err(FixedReservedStateError::Conflict)
                ));
            }
            assert!(storage.is_refused());
        }
    }
}
#[test]
fn reserved_state_ancestry_metadata_and_content_drift_remain_refused() {
    for kind in 0..3 {
        let (temp, uid, gid) = tree();
        let expected = state();
        persist(temp.path(), &expected);
        let mut storage = walk(temp.path(), uid, gid).unwrap();
        let mut read = bind_fixed_reserved_state(&mut storage, &expected).unwrap();
        let name = at(temp.path(), 4).join(ACTIVE_DIAGNOSTIC_NAME);
        match kind {
            0 => {
                fs::set_permissions(at(temp.path(), 2), fs::Permissions::from_mode(0o755)).unwrap()
            }
            1 => fs::set_permissions(&name, fs::Permissions::from_mode(0o640)).unwrap(),
            _ => {
                let mut changed = expected.clone();
                changed.revision += 1;
                persist(temp.path(), &changed);
            }
        }
        assert!(read.revalidate(&expected).is_err());
        fs::set_permissions(at(temp.path(), 2), fs::Permissions::from_mode(0o700)).unwrap();
        persist(temp.path(), &expected);
        assert!(matches!(
            read.revalidate(&expected),
            Err(FixedReservedStateError::Refused)
        ));
    }
}
#[test]
fn reserved_state_independent_concurrent_guards_do_not_share_refusal() {
    std::thread::scope(|s| {
        let handles: Vec<_> = [false, true]
            .into_iter()
            .map(|fail| {
                s.spawn(move || {
                    let (temp, uid, gid) = tree();
                    let expected = state();
                    persist(temp.path(), &expected);
                    let mut storage = walk(temp.path(), uid, gid).unwrap();
                    let mut read = bind_fixed_reserved_state(&mut storage, &expected).unwrap();
                    let mut observed = expected.clone();
                    if fail {
                        observed.revision += 1;
                    }
                    assert_eq!(read.revalidate(&observed).is_err(), fail);
                    assert_eq!(read.is_refused(), fail);
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
    });
}
#[test]
fn reserved_state_metadata_policy_and_public_type_remain_original_only() {
    let (temp, _, _) = tree();
    persist(temp.path(), &state());
    let parent_metadata = fs::metadata(at(temp.path(), 4)).unwrap();
    let mut stamp =
        Stamp::of(&fs::metadata(at(temp.path(), 4).join(ACTIVE_DIAGNOSTIC_NAME)).unwrap());
    assert!(stamp.safe(&parent_metadata));
    stamp.uid += 1;
    assert!(!stamp.safe(&parent_metadata));
    stamp.uid -= 1;
    stamp.gid += 1;
    assert!(!stamp.safe(&parent_metadata));
    stamp.gid -= 1;
    stamp.device += 1;
    assert!(!stamp.safe(&parent_metadata));
    trait AmbiguousSend<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSend<()> for T {}
    impl<T: ?Sized + Send> AmbiguousSend<u8> for T {}
    let _ = <FixedReservedStateRead<'static> as AmbiguousSend<_>>::marker;
    trait AmbiguousSync<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSync<()> for T {}
    impl<T: ?Sized + Sync> AmbiguousSync<u8> for T {}
    let _ = <FixedReservedStateRead<'static> as AmbiguousSync<_>>::marker;
    // Private seams exercise defenses that have no public origin/FD selector.
    let (uid, gid) = (unsafe { libc::geteuid() }, unsafe { libc::getegid() });
    let mut storage = walk(temp.path(), uid, gid).unwrap();
    storage.kind = StorageLeaf::Authority;
    assert!(matches!(
        parent(&storage),
        Err(FixedReservedStateError::Document)
    ));
    storage.kind = StorageLeaf::Intent;
    storage.components.clear();
    assert!(matches!(
        parent(&storage),
        Err(FixedReservedStateError::Document)
    ));
    let document = File::open(at(temp.path(), 4).join(ACTIVE_DIAGNOSTIC_NAME)).unwrap();
    assert!(matches!(
        conflicts(&document),
        Err(FixedReservedStateError::Io(_))
    ));
    let large = temp.path().join("oversized-private-seam");
    fs::write(&large, vec![b'x'; MAX_BYTES as usize + 1]).unwrap();
    assert!(matches!(
        read_bytes(&File::open(large).unwrap()),
        Err(FixedReservedStateError::Document)
    ));
    assert!(matches!(
        read_bytes(&File::open(at(temp.path(), 4)).unwrap()),
        Err(FixedReservedStateError::Io(_))
    ));
}
