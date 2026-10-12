use super::super::super::super::{
    tests::{at, tree},
    walk,
};
use super::super::{CAMPAIGN_CONTEXTS, PARENT_CONTEXTS};
use super::*;
use crate::host_execution::{
    lock_host_root, HostExecutionClaim, HostExecutionError, HOST_EXECUTION_LOCK_NAME,
};
use std::fs;
use std::os::unix::{
    ffi::OsStringExt,
    fs::{symlink, PermissionsExt},
};
use std::path::PathBuf;

fn fixture() -> (tempfile::TempDir, FixedIntentStorageRead, PathBuf, PathBuf) {
    let (temp, uid, gid) = tree();
    let root = at(temp.path(), 3).join("campaigns");
    let leaf = at(temp.path(), 4);
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(root.join(HOST_EXECUTION_LOCK_NAME), []).unwrap();
    fs::set_permissions(
        root.join(HOST_EXECUTION_LOCK_NAME),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let storage = walk(temp.path(), uid, gid).unwrap();
    (temp, storage, root, leaf)
}

pub(crate) fn with_owned_empty_fence_for_test<R>(
    run: impl FnOnce(OriginalEmptyReservationFence<'_>, &std::path::Path, &std::path::Path) -> R,
) -> R {
    let (_temp, mut storage, root, leaf) = fixture();
    let fence = bind_original_empty_reservation_fence(&mut storage).unwrap();
    run(fence, &root, &leaf)
}

pub(crate) fn retained_context_refuses_for_test(root: &std::path::Path) {
    let origin = root.ancestors().nth(4).unwrap();
    let mut storage = walk(origin, unsafe { libc::geteuid() }, unsafe {
        libc::getegid()
    })
    .unwrap();
    assert!(matches!(
        bind_original_empty_reservation_fence(&mut storage),
        Err(EmptyReservationFenceError::Conflict)
    ));
}

pub(crate) fn with_empty_fence_for_test<R>(
    run: impl FnOnce(&mut OriginalEmptyReservationFence<'_>, &std::path::Path) -> R,
) -> R {
    let (_temp, mut storage, root, _leaf) = fixture();
    let mut fence = bind_original_empty_reservation_fence(&mut storage).unwrap();
    run(&mut fence, &root)
}

#[test]
fn empty_reservation_fence_healthy_empty_repeat_excludes_legacy_and_releases_only_lock() {
    let (_temp, mut storage, root, leaf) = fixture();
    {
        let mut read = bind_original_empty_reservation_fence(&mut storage).unwrap();
        read.revalidate().unwrap();
        read.revalidate().unwrap();
        assert!(matches!(
            HostExecutionClaim::acquire(&root, &"a".repeat(64)),
            Err(HostExecutionError::Busy)
        ));
        assert!(matches!(
            lock_host_root(&root),
            Err(HostExecutionError::Busy)
        ));
        assert!(!root.join("active-campaign").exists());
        assert_eq!(fs::read_dir(&leaf).unwrap().count(), 0);
    }
    storage.revalidate().unwrap();
    assert_eq!(fs::read(root.join(HOST_EXECUTION_LOCK_NAME)).unwrap(), b"");
    assert_eq!(fs::read_dir(&leaf).unwrap().count(), 0);
    assert!(HostExecutionClaim::acquire(&root, &"a".repeat(64)).is_ok());
}
#[test]
fn empty_reservation_fence_initial_busy_keeps_storage_and_other_owner_healthy() {
    for campaign in [false, true] {
        let (_temp, mut storage, root, _) = fixture();
        let held = campaign.then(|| HostExecutionClaim::acquire(&root, &"a".repeat(64)).unwrap());
        let short = (!campaign).then(|| lock_host_root(&root).unwrap());
        assert!(matches!(
            bind_original_empty_reservation_fence(&mut storage),
            Err(EmptyReservationFenceError::Fence(
                OriginalHostFenceError::Busy
            ))
        ));
        assert!(!storage.is_refused());
        assert!(matches!(
            lock_host_root(&root),
            Err(HostExecutionError::Busy)
        ));
        drop(held);
        drop(short);
        if campaign {
            fs::remove_file(root.join("active-campaign")).unwrap();
        }
        bind_original_empty_reservation_fence(&mut storage)
            .unwrap()
            .revalidate()
            .unwrap();
    }
}
#[test]
fn empty_reservation_fence_every_fixed_leaf_entry_blocks_without_decode_or_cleanup() {
    for name in [
        "active-diagnostic.json",
        ".active-diagnostic.pending",
        ".diagnostic-requests.pending",
        "diagnostic-requests-v1.json",
        "terminal",
        "diagnostic-intent-cell-1.commit",
        "unknown",
    ] {
        for kind in 0..4 {
            let (_temp, mut storage, _, leaf) = fixture();
            let path = leaf.join(name);
            match kind {
                0 => fs::write(&path, []).unwrap(),
                1 => fs::create_dir(&path).unwrap(),
                2 => symlink("missing", &path).unwrap(),
                _ => {
                    let name = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
                    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
                }
            }
            assert!(matches!(
                bind_original_empty_reservation_fence(&mut storage),
                Err(EmptyReservationFenceError::Conflict)
            ));
            assert!(storage.is_refused());
            assert!(fs::symlink_metadata(&path).is_ok());
        }
    }
    let (_temp, mut storage, _, leaf) = fixture();
    fs::write(
        leaf.join(std::ffi::OsString::from_vec(vec![0xff, b'x'])),
        [],
    )
    .unwrap();
    assert!(matches!(
        bind_original_empty_reservation_fence(&mut storage),
        Err(EmptyReservationFenceError::Conflict)
    ));
}
#[test]
fn empty_reservation_fence_all_legacy_and_parent_contexts_refuse_dangling_links() {
    for parent in [false, true] {
        let names = if parent {
            PARENT_CONTEXTS.as_slice()
        } else {
            CAMPAIGN_CONTEXTS.as_slice()
        };
        for name in names {
            for link in [false, true] {
                let (_temp, mut storage, root, _) = fixture();
                let path = if parent {
                    root.parent().unwrap().join(name)
                } else {
                    root.join(name)
                };
                if link {
                    symlink("missing", &path).unwrap();
                } else {
                    fs::write(&path, []).unwrap();
                }
                assert!(matches!(
                    bind_original_empty_reservation_fence(&mut storage),
                    Err(EmptyReservationFenceError::Fence(
                        OriginalHostFenceError::Conflict
                    ))
                ));
                assert!(storage.is_refused());
                assert!(fs::symlink_metadata(&path).is_ok());
            }
        }
    }
}
#[test]
fn empty_reservation_fence_unsafe_or_missing_lock_and_legacy_ownership_never_repaired() {
    for kind in 0..9 {
        let (_temp, mut storage, root, _) = fixture();
        let path = root.join(HOST_EXECUTION_LOCK_NAME);
        match kind {
            0 => fs::remove_file(&path).unwrap(),
            1 => fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap(),
            2 => {
                fs::remove_file(&path).unwrap();
                symlink("missing", &path).unwrap();
            }
            3 => {
                fs::remove_file(&path).unwrap();
                let name = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
            }
            4 => fs::hard_link(&path, root.join("alias")).unwrap(),
            5 => fs::write(&path, b"not empty").unwrap(),
            6 => fs::set_permissions(&root, fs::Permissions::from_mode(0o750)).unwrap(),
            7 => {
                fs::remove_file(&path).unwrap();
                fs::create_dir(&path).unwrap();
            }
            _ => fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap(),
        }
        assert!(bind_original_empty_reservation_fence(&mut storage).is_err());
        assert!(storage.is_refused());
        if kind == 0 {
            assert!(!path.exists());
        }
        if kind == 5 {
            assert_eq!(fs::read(&path).unwrap(), b"not empty");
        }
        if kind == 6 {
            assert_eq!(
                fs::metadata(&root).unwrap().permissions().mode() & 0o7777,
                0o750
            );
        }
    }
}
#[test]
fn empty_reservation_fence_bound_named_or_original_object_drift_never_adopted() {
    for kind in 0..6 {
        let (_temp, mut storage, root, leaf) = fixture();
        let mut read = bind_original_empty_reservation_fence(&mut storage).unwrap();
        let mut saved = None;
        match kind {
            0 => {
                let path = root.join(HOST_EXECUTION_LOCK_NAME);
                fs::rename(&path, path.with_extension("saved")).unwrap();
                fs::write(&path, []).unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            }
            1 => {
                fs::rename(&root, root.with_extension("saved")).unwrap();
                fs::create_dir(&root).unwrap();
                fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
            }
            2 => {
                fs::rename(&leaf, leaf.with_extension("saved")).unwrap();
                fs::create_dir(&leaf).unwrap();
                fs::set_permissions(&leaf, fs::Permissions::from_mode(0o700)).unwrap();
            }
            3 => fs::set_permissions(root.parent().unwrap(), fs::Permissions::from_mode(0o750))
                .unwrap(),
            4 => {
                let path = leaf.join("foreign");
                fs::write(&path, []).unwrap();
                saved = Some(std::mem::replace(
                    &mut read.fence.lock,
                    File::open(path).unwrap(),
                ));
            }
            _ => {
                saved = Some(std::mem::replace(
                    &mut read.fence.directory,
                    File::open(&leaf).unwrap(),
                ));
            }
        }
        assert!(read.revalidate().is_err());
        assert!(read.is_refused());
        if let Some(old) = saved {
            if kind == 4 {
                read.fence.lock = old;
            } else {
                read.fence.directory = old;
            }
        }
        assert!(matches!(
            read.revalidate_with(|| panic!("latched guard must do no further I/O")),
            Err(EmptyReservationFenceError::Refused)
        ));
    }
}
#[test]
fn empty_reservation_fence_seeded_between_scan_context_is_seen_and_retained() {
    let seed = 0x0749_u64;
    eprintln!("seed={seed:#x}");
    let (_temp, mut storage, _, leaf) = fixture();
    let mut read = bind_original_empty_reservation_fence(&mut storage).unwrap();
    let mut calls = 0;
    assert!(matches!(
        read.revalidate_with(|| {
            calls += 1;
            fs::write(leaf.join("late"), seed.to_le_bytes()).unwrap();
        }),
        Err(EmptyReservationFenceError::Conflict)
    ));
    assert_eq!(calls, 1);
    assert_eq!(fs::read(leaf.join("late")).unwrap(), seed.to_le_bytes());
    assert!(matches!(
        read.revalidate_with(|| panic!("latched")),
        Err(EmptyReservationFenceError::Refused)
    ));
}
#[test]
fn empty_reservation_fence_missing_unsafe_or_restored_storage_remains_refused() {
    for kind in 0..3 {
        let (_temp, mut storage, _, leaf) = fixture();
        let mut read = bind_original_empty_reservation_fence(&mut storage).unwrap();
        match kind {
            0 => fs::remove_dir(&leaf).unwrap(),
            1 => fs::set_permissions(&leaf, fs::Permissions::from_mode(0o750)).unwrap(),
            _ => {
                fs::rename(&leaf, leaf.with_extension("saved")).unwrap();
                symlink(leaf.with_extension("saved"), &leaf).unwrap();
            }
        }
        assert!(read.revalidate().is_err());
        if kind == 1 {
            fs::set_permissions(&leaf, fs::Permissions::from_mode(0o700)).unwrap();
        }
        assert!(matches!(
            read.revalidate_with(|| panic!("latched")),
            Err(EmptyReservationFenceError::Refused)
        ));
        drop(read);
        assert!(matches!(
            bind_original_empty_reservation_fence(&mut storage),
            Err(EmptyReservationFenceError::Refused)
        ));
    }
}
#[test]
fn empty_reservation_fence_wrong_private_origin_and_nonroot_fixed_open_refuse() {
    let (_temp, mut storage, _, _) = fixture();
    storage.kind = super::super::super::super::StorageLeaf::Authority;
    assert!(matches!(
        bind_original_empty_reservation_fence(&mut storage),
        Err(EmptyReservationFenceError::Origin)
    ));
    assert!(!storage.is_refused());
    assert!(super::super::super::super::open_fixed_intent_storage().is_err());
}
#[test]
fn empty_reservation_fence_independent_concurrent_storage_keeps_local_refusal() {
    std::thread::scope(|scope| {
        let handles: Vec<_> = [false, true]
            .into_iter()
            .map(|fail| {
                scope.spawn(move || {
                    let (_temp, mut storage, _, leaf) = fixture();
                    let mut read = bind_original_empty_reservation_fence(&mut storage).unwrap();
                    if fail {
                        fs::write(leaf.join("context"), []).unwrap();
                    }
                    assert_eq!(read.revalidate().is_err(), fail);
                    assert_eq!(read.is_refused(), fail);
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
}
#[test]
fn empty_reservation_fence_public_guard_is_original_thread_only() {
    trait AmbiguousSend<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSend<()> for T {}
    impl<T: ?Sized + Send> AmbiguousSend<u8> for T {}
    let _ = <OriginalEmptyReservationFence<'static> as AmbiguousSend<_>>::marker;
    trait AmbiguousSync<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSync<()> for T {}
    impl<T: ?Sized + Sync> AmbiguousSync<u8> for T {}
    let _ = <OriginalEmptyReservationFence<'static> as AmbiguousSync<_>>::marker;
}
