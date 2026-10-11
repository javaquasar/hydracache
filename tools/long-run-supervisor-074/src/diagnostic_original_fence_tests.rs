use super::super::super::{
    tests::{at, tree},
    walk,
};
use super::super::{
    bind_fixed_reserved_state,
    tests::{persist, state},
};
use super::*;
use crate::host_execution::{lock_host_root, HostExecutionClaim, HostExecutionError};
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};

fn campaigns(root: &Path) -> PathBuf {
    at(root, 3).join("campaigns")
}
fn fixture() -> (tempfile::TempDir, u32, u32) {
    let (temp, uid, gid) = tree();
    let root = campaigns(temp.path());
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(root.join(HOST_EXECUTION_LOCK_NAME), []).unwrap();
    fs::set_permissions(
        root.join(HOST_EXECUTION_LOCK_NAME),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    persist(temp.path(), &state());
    (temp, uid, gid)
}

pub(crate) fn with_fence_for_test<R>(
    expected: &crate::diagnostic_lease::DiagnosticState,
    run: impl FnOnce(&mut FencedReservedStateRead<'_, '_>, &Path) -> R,
) -> R {
    let (temp, uid, gid) = fixture();
    persist(temp.path(), expected);
    let mut storage = walk(temp.path(), uid, gid).unwrap();
    let mut state_read = bind_fixed_reserved_state(&mut storage, expected).unwrap();
    let mut fence = bind_original_host_fence(&mut state_read).unwrap();
    run(&mut fence, &campaigns(temp.path()))
}

#[test]
fn original_fence_same_legacy_lock_blocks_both_campaign_entry_points() {
    let (temp, uid, gid) = fixture();
    let root = campaigns(temp.path());
    let original_bytes = fs::read(at(temp.path(), 4).join("active-diagnostic.json")).unwrap();
    let mut storage = walk(temp.path(), uid, gid).unwrap();
    let mut state_read = bind_fixed_reserved_state(&mut storage, &state()).unwrap();
    {
        let mut read = bind_original_host_fence(&mut state_read).unwrap();
        read.revalidate().unwrap();
        assert!(matches!(
            HostExecutionClaim::acquire(&root, &"d".repeat(64)),
            Err(HostExecutionError::Busy)
        ));
        assert!(matches!(
            lock_host_root(&root),
            Err(HostExecutionError::Busy)
        ));
        fs::create_dir(root.join("unrelated")).unwrap();
        read.revalidate().unwrap();
    }
    state_read.revalidate(&state()).unwrap();
    assert_eq!(
        fs::read(at(temp.path(), 4).join("active-diagnostic.json")).unwrap(),
        original_bytes
    );
    assert!(fs::read(root.join(HOST_EXECUTION_LOCK_NAME))
        .unwrap()
        .is_empty());
    // Deliberate negative proof: the legacy route does NOT consult fixed state.
    let claim = HostExecutionClaim::acquire(&root, &"d".repeat(64)).unwrap();
    assert!(at(temp.path(), 4).join("active-diagnostic.json").exists());
    assert_eq!(claim.campaign_id(), "d".repeat(64));
    drop(claim);
}

#[test]
fn original_fence_initial_busy_is_retryable_and_never_unlocks_legacy_owner() {
    for campaign in [false, true] {
        let (temp, uid, gid) = fixture();
        let root = campaigns(temp.path());
        let held = if campaign {
            Some(HostExecutionClaim::acquire(&root, &"d".repeat(64)).unwrap())
        } else {
            None
        };
        let short = if !campaign {
            Some(lock_host_root(&root).unwrap())
        } else {
            None
        };
        let mut storage = walk(temp.path(), uid, gid).unwrap();
        let mut state_read = bind_fixed_reserved_state(&mut storage, &state()).unwrap();
        assert!(matches!(
            bind_original_host_fence(&mut state_read),
            Err(OriginalHostFenceError::Busy)
        ));
        assert!(!state_read.is_refused());
        assert!(matches!(
            lock_host_root(&root),
            Err(HostExecutionError::Busy)
        ));
        drop(held);
        drop(short);
        if campaign {
            fs::remove_file(root.join("active-campaign")).unwrap();
        }
        bind_original_host_fence(&mut state_read)
            .unwrap()
            .revalidate()
            .unwrap();
        assert!(!state_read.is_refused());
    }
}

#[test]
fn original_fence_missing_or_unsafe_lock_never_created_repaired_or_adopted() {
    for kind in 0..7 {
        let (temp, uid, gid) = fixture();
        let lock = campaigns(temp.path()).join(HOST_EXECUTION_LOCK_NAME);
        match kind {
            0 => fs::remove_file(&lock).unwrap(),
            1 => {
                fs::remove_file(&lock).unwrap();
                symlink("missing", &lock).unwrap();
            }
            2 => {
                fs::remove_file(&lock).unwrap();
                let name = CString::new(lock.as_os_str().as_encoded_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
            }
            3 => {
                fs::remove_file(&lock).unwrap();
                fs::create_dir(&lock).unwrap();
            }
            4 => fs::hard_link(&lock, temp.path().join("alias")).unwrap(),
            5 => fs::set_permissions(&lock, fs::Permissions::from_mode(0o640)).unwrap(),
            _ => fs::write(&lock, b"changed").unwrap(),
        }
        let mut storage = walk(temp.path(), uid, gid).unwrap();
        let mut state_read = bind_fixed_reserved_state(&mut storage, &state()).unwrap();
        assert!(bind_original_host_fence(&mut state_read).is_err());
        assert!(state_read.is_refused());
        assert!(matches!(
            bind_original_host_fence(&mut state_read),
            Err(OriginalHostFenceError::Refused)
        ));
        if kind == 0 {
            assert!(!lock.exists());
        }
        if kind == 5 {
            assert_eq!(
                fs::metadata(lock).unwrap().permissions().mode() & 0o7777,
                0o640
            );
        }
    }
}

#[test]
fn original_fence_deployed_mode_or_unsafe_campaign_directory_requires_enrollment() {
    for kind in 0..3 {
        let (temp, uid, gid) = fixture();
        let root = campaigns(temp.path());
        match kind {
            0 => fs::set_permissions(&root, fs::Permissions::from_mode(0o750)).unwrap(),
            1 => {
                fs::rename(&root, temp.path().join("original")).unwrap();
                symlink(temp.path().join("original"), &root).unwrap();
            }
            _ => {
                fs::rename(&root, temp.path().join("original")).unwrap();
            }
        }
        let mut storage = walk(temp.path(), uid, gid).unwrap();
        let mut state_read = bind_fixed_reserved_state(&mut storage, &state()).unwrap();
        assert!(bind_original_host_fence(&mut state_read).is_err());
        assert!(state_read.is_refused());
    }
}

#[test]
fn original_fence_all_contexts_refuse_initial_and_bound_dangling_symlinks() {
    for (parent, names) in [
        (false, CAMPAIGN_CONTEXTS.as_slice()),
        (true, PARENT_CONTEXTS.as_slice()),
    ] {
        for name in names {
            for bound in [false, true] {
                let (temp, uid, gid) = fixture();
                let context = if parent {
                    at(temp.path(), 3)
                } else {
                    campaigns(temp.path())
                }
                .join(name);
                let mut storage = walk(temp.path(), uid, gid).unwrap();
                let mut state_read = bind_fixed_reserved_state(&mut storage, &state()).unwrap();
                if bound {
                    let mut read = bind_original_host_fence(&mut state_read).unwrap();
                    symlink("missing", &context).unwrap();
                    assert!(matches!(
                        read.revalidate(),
                        Err(OriginalHostFenceError::Conflict)
                    ));
                    fs::remove_file(&context).unwrap();
                    assert!(matches!(
                        read.revalidate(),
                        Err(OriginalHostFenceError::Refused)
                    ));
                } else {
                    symlink("missing", &context).unwrap();
                    assert!(matches!(
                        bind_original_host_fence(&mut state_read),
                        Err(OriginalHostFenceError::Conflict)
                    ));
                }
                assert!(state_read.is_refused());
                assert!(lock_host_root(&campaigns(temp.path())).is_ok());
            }
        }
    }
}

#[test]
fn original_fence_named_lock_replacement_never_adopted_and_original_remains_held() {
    let (temp, uid, gid) = fixture();
    let root = campaigns(temp.path());
    let name = root.join(HOST_EXECUTION_LOCK_NAME);
    let saved = temp.path().join("original-lock");
    let mut storage = walk(temp.path(), uid, gid).unwrap();
    let mut state_read = bind_fixed_reserved_state(&mut storage, &state()).unwrap();
    let mut read = bind_original_host_fence(&mut state_read).unwrap();
    fs::rename(&name, &saved).unwrap();
    fs::write(&name, []).unwrap();
    fs::set_permissions(&name, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(matches!(
        read.revalidate(),
        Err(OriginalHostFenceError::Object)
    ));
    let original = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&saved)
        .unwrap();
    assert_eq!(
        FileExt::try_lock_exclusive(&original).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    drop(read);
    FileExt::try_lock_exclusive(&original).unwrap();
    FileExt::unlock(&original).unwrap();
    assert!(state_read.is_refused());
}

#[test]
fn original_fence_seeded_bound_identity_state_and_metadata_failures_latch() {
    let seed = 0x0746_u64;
    eprintln!("seed={seed:#x}");
    for kind in 0..6 {
        let (temp, uid, gid) = fixture();
        let root = campaigns(temp.path());
        let mut storage = walk(temp.path(), uid, gid).unwrap();
        let mut state_read = bind_fixed_reserved_state(&mut storage, &state()).unwrap();
        let mut read = bind_original_host_fence(&mut state_read).unwrap();
        match kind {
            0 => fs::set_permissions(&root, fs::Permissions::from_mode(0o750)).unwrap(),
            1 => fs::set_permissions(
                root.join(HOST_EXECUTION_LOCK_NAME),
                fs::Permissions::from_mode(0o640),
            )
            .unwrap(),
            2 => {
                let mut changed = state();
                changed.revision += seed;
                persist(temp.path(), &changed);
            }
            3 => {
                fs::set_permissions(at(temp.path(), 2), fs::Permissions::from_mode(0o755)).unwrap()
            }
            4 => {
                fs::rename(&root, temp.path().join("old-root")).unwrap();
                fs::create_dir(&root).unwrap();
            }
            _ => fs::write(root.join(HOST_EXECUTION_LOCK_NAME), b"changed").unwrap(),
        }
        assert!(read.revalidate().is_err());
        assert!(matches!(
            read.revalidate(),
            Err(OriginalHostFenceError::Refused)
        ));
        drop(read);
        drop(state_read);
        assert!(storage.is_refused());
    }
}

#[test]
fn original_fence_private_descriptor_substitutions_latch_without_refresh() {
    for lock in [false, true] {
        let (temp, uid, gid) = fixture();
        let mut storage = walk(temp.path(), uid, gid).unwrap();
        let mut state_read = bind_fixed_reserved_state(&mut storage, &state()).unwrap();
        let mut read = bind_original_host_fence(&mut state_read).unwrap();
        let other = temp.path().join("other");
        fs::write(&other, []).unwrap();
        let slot = if lock {
            &mut read.fence.lock
        } else {
            &mut read.fence.directory
        };
        let original = std::mem::replace(slot, File::open(&other).unwrap());
        assert!(read.revalidate().is_err());
        if lock {
            read.fence.lock = original;
        } else {
            read.fence.directory = original;
        }
        assert!(matches!(
            read.revalidate(),
            Err(OriginalHostFenceError::Refused)
        ));
    }
}

#[test]
fn original_fence_concurrent_independent_open_cannot_bypass_same_flock() {
    let (temp, uid, gid) = fixture();
    let mut storage = walk(temp.path(), uid, gid).unwrap();
    let mut state_read = bind_fixed_reserved_state(&mut storage, &state()).unwrap();
    let read = bind_original_host_fence(&mut state_read).unwrap();
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let mut storage = walk(temp.path(), uid, gid).unwrap();
                let mut state_read = bind_fixed_reserved_state(&mut storage, &state()).unwrap();
                assert!(matches!(
                    bind_original_host_fence(&mut state_read),
                    Err(OriginalHostFenceError::Busy)
                ));
                assert!(!state_read.is_refused());
            })
            .join()
            .unwrap();
    });
    assert!(!read.is_refused());
}

#[test]
fn original_fence_policy_types_and_error_seams_remain_fail_closed() {
    let (temp, uid, gid) = fixture();
    let root = campaigns(temp.path());
    assert!(Fence::acquire(&File::open(at(temp.path(), 3)).unwrap(), uid + 1, gid).is_err());
    let parent = File::open(&root).unwrap().metadata().unwrap();
    let mut stamp = Stamp::of(
        &File::open(root.join(HOST_EXECUTION_LOCK_NAME))
            .unwrap()
            .metadata()
            .unwrap(),
    );
    for field in 0..3 {
        match field {
            0 => stamp.uid += 1,
            1 => stamp.gid += 1,
            _ => stamp.device += 1,
        }
        assert!(!stamp.safe(&parent));
        match field {
            0 => stamp.uid -= 1,
            1 => stamp.gid -= 1,
            _ => stamp.device -= 1,
        }
    }
    assert!(matches!(
        absent(
            &File::open(root.join(HOST_EXECUTION_LOCK_NAME)).unwrap(),
            &["x"]
        ),
        Err(OriginalHostFenceError::Io(_))
    ));
    assert!(matches!(
        absent(&File::open(&root).unwrap(), &["nul\0"]),
        Err(OriginalHostFenceError::Object)
    ));
    trait AmbiguousSend<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSend<()> for T {}
    impl<T: ?Sized + Send> AmbiguousSend<u8> for T {}
    let _ = <FencedReservedStateRead<'static, 'static> as AmbiguousSend<_>>::marker;
    trait AmbiguousSync<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSync<()> for T {}
    impl<T: ?Sized + Sync> AmbiguousSync<u8> for T {}
    let _ = <FencedReservedStateRead<'static, 'static> as AmbiguousSync<_>>::marker;
}
