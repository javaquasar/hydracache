use super::*;
use super::super::attempt_publication::*;
use super::super::early_attempt::retain_fixed_attempt;
use super::super::reservation_publication::{self, publish_fixed_reserved, PublicationStage, ReservationPublicationError, StoragePublicationError};
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::{retained_context_refuses_for_test, with_owned_empty_fence_for_test};
use crate::host_execution::{HostExecutionClaim, HostExecutionError};
use std::os::unix::fs::MetadataExt;

macro_rules! pair {
    ($i:ident, $original:ident, $read:ident, $dir:ident, $root:ident, $leaf:ident, $f:ident, $raw:ident, $body:block) => {
        with_mount(1, |mount, _, $f| {
            let ($dir, fixture) = journal($f);
            let mut registry = modeled_registry_for_test(fixture);
            let fixture = open_signed_fixture_tasks(mount, &$f.bytes, &$f.trust, &$f.host).unwrap();
            let mut tasks = FixedSignedTasksRead {
                reader: fixture.reader,
            };
            let mut authority =
                bind_fixed_authority(&mut tasks, &mut registry, &$f.bytes, &$f.trust, &$f.host)
                    .unwrap();
            let $i = Intent::new($f);
            let $raw = $i.signed(&$i.body($f));
            let mut $original =
                bind_fixed_intent(&mut authority, &$raw, &$i.check($f, 100)).unwrap();
            with_owned_empty_fence_for_test(|fence, $root, $leaf| {
                let $read =
                    retain_fixed_attempt(&mut $original, fence, &$raw, &$i.check($f, 100)).unwrap();
                $body
            });
        });
    };
}
fn busy(root: &std::path::Path) {
    assert!(matches!(
        HostExecutionClaim::acquire(root, &"a".repeat(64)),
        Err(HostExecutionError::Busy)
    ));
}
fn attempt(leaf: &std::path::Path) -> (Vec<u8>, [u64; 11]) {
    let path = leaf.join("diagnostic-attempt-074.json");
    let m = fs::metadata(&path).unwrap();
    (
        fs::read(path).unwrap(),
        [
            m.dev(),
            m.ino(),
            u64::from(m.uid()),
            u64::from(m.gid()),
            u64::from(m.mode()),
            m.nlink(),
            m.len(),
            m.mtime() as u64,
            m.mtime_nsec() as u64,
            m.ctime() as u64,
            m.ctime_nsec() as u64,
        ],
    )
}

#[test]
fn attempt_publication_healthy_transfer_and_cell_one_conserve_original_attempt() {
    pair!(i, original, read, dir, root, leaf, f, raw, {
        let before = inventory(&dir);
        let owned = attempt(leaf);
        let mut read = publish_reserved_from_attempt(read, &raw, &i.check(f, 100)).unwrap();
        busy(root);
        assert_eq!(
            crate::diagnostic_lease::decode_active(
                &fs::read(leaf.join("active-diagnostic.json")).unwrap()
            )
            .unwrap(),
            i.state
        );
        assert_eq!(fs::read_dir(leaf).unwrap().count(), 2);
        read.revalidate(&raw, &i.check(f, 101)).unwrap();
        assert_eq!(attempt(leaf), owned);
        let mut read = super::super::initial_consumption::consume_fixed_initial_intent(
            read,
            &raw,
            &i.check(f, 101),
        )
        .unwrap();
        busy(root);
        read.revalidate(&raw, &i.check(f, 102)).unwrap();
        assert_eq!(fs::read_dir(leaf).unwrap().count(), 4);
        assert_eq!(
            fs::read(leaf.join("diagnostic-intent-cell-1.json")).unwrap(),
            owned.0
        );
        assert_eq!(attempt(leaf), owned);
        drop(read);
        assert!(!original.is_refused());
        assert_eq!(inventory(&dir), before);
        retained_context_refuses_for_test(root);
    });
}
#[test]
fn attempt_publication_unbound_mismatch_keeps_owned_attempt_without_revocation() {
    pair!(i, original, read, dir, root, leaf, f, raw, {
        let owned = attempt(leaf);
        let mut state = i.state.clone();
        state.revision += 1;
        let mut check = i.check(f, 100);
        check.state = &state;
        let failure = publish_reserved_from_attempt(read, &raw, &check).unwrap_err();
        assert!(matches!(failure.error(), AttemptPublicationError::Binding));
        assert!(!failure.is_refused());
        busy(root);
        assert_eq!(attempt(leaf), owned);
        assert!(!leaf.join("active-diagnostic.json").exists());
        drop(failure);
        assert!(!original.is_refused());
        assert_eq!(inventory(&dir).len(), 2);
        retained_context_refuses_for_test(root);
    });
}
#[test]
fn attempt_publication_all_boundaries_keep_attempt_lock_and_partial_state() {
    eprintln!("attempt publication seed=0x074d");
    for stage in reservation_publication::ALL_STAGES {
        pair!(i, original, read, dir, root, leaf, f, raw, {
            let owned = attempt(leaf);
            let failure = publish_with(read, &raw, &i.check(f, 100), |at| {
                busy(root);
                if at == stage {
                    Err(StoragePublicationError::Interrupted(at).into())
                } else {
                    Ok(())
                }
            })
            .unwrap_err();
            assert!(
                matches!(failure.error(), AttemptPublicationError::Publication(ReservationPublicationError::Storage(StoragePublicationError::Interrupted(at))) if *at == stage)
            );
            busy(root);
            assert!(failure.is_refused());
            assert_eq!(attempt(leaf), owned);
            let path = leaf.join("active-diagnostic.json");
            assert_eq!(path.exists(), stage != PublicationStage::BeforeCreate);
            let state = fs::read(&path).ok();
            drop(failure);
            assert!(original.is_refused());
            assert_eq!(fs::read(&path).ok(), state);
            assert_eq!(attempt(leaf), owned);
            assert_eq!(inventory(&dir).len(), 4);
            retained_context_refuses_for_test(root);
        });
    }
}
#[test]
fn attempt_publication_late_target_is_never_adopted_or_overwritten() {
    pair!(i, original, read, dir, root, leaf, f, raw, {
        let owned = attempt(leaf);
        let path = leaf.join("active-diagnostic.json");
        let foreign = crate::diagnostic_lease::encode(&i.state).unwrap();
        let failure = publish_with(read, &raw, &i.check(f, 100), |stage| {
            if stage == PublicationStage::BeforeCreate {
                fs::write(&path, &foreign).unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            }
            Ok(())
        })
        .unwrap_err();
        assert!(failure.is_refused());
        busy(root);
        drop(failure);
        assert!(original.is_refused());
        assert_eq!(fs::read(path).unwrap(), foreign);
        assert_eq!(attempt(leaf), owned);
        assert_eq!(inventory(&dir).len(), 4);
        retained_context_refuses_for_test(root);
    });
}
#[test]
fn attempt_publication_unknown_or_legacy_context_is_retained_not_repaired() {
    for legacy in [false, true] {
        pair!(i, original, read, dir, root, leaf, f, raw, {
            let owned = attempt(leaf);
            let path = if legacy {
                root.join("active-campaign")
            } else {
                leaf.join("unknown")
            };
            let failure = publish_with(read, &raw, &i.check(f, 100), |stage| {
                if stage == PublicationStage::DocumentBound {
                    fs::write(&path, b"conflict").unwrap();
                }
                Ok(())
            })
            .unwrap_err();
            assert!(failure.is_refused());
            busy(root);
            drop(failure);
            assert!(original.is_refused());
            assert_eq!(attempt(leaf), owned);
            assert_eq!(fs::read(path).unwrap(), b"conflict");
            assert!(leaf.join("active-diagnostic.json").exists());
            assert_eq!(inventory(&dir).len(), 4);
        });
    }
}
#[test]
fn attempt_publication_attempt_drift_during_publication_is_sticky() {
    for mode in 0..4 {
        pair!(i, original, read, dir, root, leaf, f, raw, {
            let path = leaf.join("diagnostic-attempt-074.json");
            let failure = publish_with(read, &raw, &i.check(f, 100), |stage| {
                if stage == PublicationStage::PrefixWritten {
                    match mode {
                        0 => {
                            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
                            fs::write(&path, b"changed").unwrap();
                        }
                        1 => fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap(),
                        2 => fs::hard_link(&path, leaf.join("alias")).unwrap(),
                        _ => {
                            let bytes = fs::read(&path).unwrap();
                            fs::rename(&path, leaf.join("saved")).unwrap();
                            fs::write(&path, bytes).unwrap();
                            fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
                        }
                    }
                }
                Ok(())
            })
            .unwrap_err();
            assert!(failure.is_refused());
            busy(root);
            assert!(leaf.join("active-diagnostic.json").exists());
            drop(failure);
            assert!(original.is_refused());
            assert_eq!(inventory(&dir).len(), 4);
            retained_context_refuses_for_test(root);
        });
    }
}
#[test]
fn attempt_publication_cell_one_refuses_changed_attempt_before_record_creation() {
    pair!(i, original, read, dir, root, leaf, f, raw, {
        let read = publish_reserved_from_attempt(read, &raw, &i.check(f, 100)).unwrap();
        let path = leaf.join("diagnostic-attempt-074.json");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let failure = super::super::initial_consumption::consume_fixed_initial_intent(
            read,
            &raw,
            &i.check(f, 101),
        )
        .unwrap_err();
        assert!(failure.is_refused());
        busy(root);
        assert!(!leaf.join("diagnostic-intent-cell-1.json").exists());
        drop(failure);
        assert!(original.is_refused());
        assert_eq!(inventory(&dir).len(), 4);
        retained_context_refuses_for_test(root);
    });
}
#[test]
fn attempt_publication_expired_or_changed_original_intent_does_not_write_state() {
    for mode in 0..3 {
        pair!(i, original, read, dir, root, leaf, f, raw, {
            let owned = attempt(leaf);
            let mut bytes = raw.clone();
            let mut check = i.check(f, 101);
            let peer = DiagnosticPeer {
                uid: 901,
                gids: i.peer.gids.clone(),
            };
            match mode {
                0 => check.now_unix_seconds = 161,
                1 => bytes[0] = b'x',
                _ => check.peer = &peer,
            }
            let failure = publish_reserved_from_attempt(read, &bytes, &check).unwrap_err();
            assert!(matches!(
                failure.error(),
                AttemptPublicationError::Attempt(_)
            ));
            assert!(failure.is_refused());
            busy(root);
            assert!(!leaf.join("active-diagnostic.json").exists());
            drop(failure);
            assert!(original.is_refused());
            assert_eq!(attempt(leaf), owned);
            assert_eq!(inventory(&dir).len(), 4);
            retained_context_refuses_for_test(root);
        });
    }
}
#[test]
fn attempt_publication_already_refused_guard_retains_attempt_without_new_io() {
    pair!(i, original, read, dir, root, leaf, f, raw, {
        let mut read = read;
        read.revalidate(&raw, &i.check(f, 160)).unwrap_err();
        let before = inventory(&dir);
        let owned = attempt(leaf);
        let failure =
            publish_reserved_from_attempt(read, b"invalid", &i.check(f, u64::MAX)).unwrap_err();
        assert!(failure.is_refused());
        busy(root);
        assert_eq!(attempt(leaf), owned);
        assert!(!leaf.join("active-diagnostic.json").exists());
        drop(failure);
        assert!(original.is_refused());
        assert_eq!(inventory(&dir), before);
        retained_context_refuses_for_test(root);
    });
}
#[test]
fn attempt_publication_unsafe_or_advanced_authority_is_not_appended() {
    for advanced in [false, true] {
        pair!(i, original, read, dir, root, leaf, f, raw, {
            let mut read = publish_reserved_from_attempt(read, &raw, &i.check(f, 100)).unwrap();
            if advanced {
                let changed = changed_checked(f, 0);
                match &mut read.original.original.journal {
                    Journal::Fixed(registry) => {
                        registry.refuse(RefusalReason::Operator).unwrap();
                        registry.advance(&changed).unwrap();
                    }
                    _ => unreachable!(),
                }
            } else {
                fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o770)).unwrap();
            }
            fs::write(leaf.join("unknown"), []).unwrap();
            let before = inventory(&dir);
            assert!(matches!(
                read.revalidate(&raw, &i.check(f, 101)),
                Err(ReservationPublicationError::RefusalUncertain { .. })
            ));
            assert_eq!(inventory(&dir), before);
            busy(root);
            fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
            assert!(matches!(
                read.revalidate(&raw, &i.check(f, 100)),
                Err(ReservationPublicationError::Refused)
            ));
            drop(read);
            assert!(original.is_refused());
            assert_eq!(inventory(&dir), before);
        });
    }
}
#[test]
fn attempt_publication_partial_refusal_preserves_original_error() {
    pair!(i, original, read, dir, root, leaf, f, raw, {
        let owned = attempt(leaf);
        let mut read = publish_reserved_from_attempt(read, &raw, &i.check(f, 100)).unwrap();
        let error = reservation_publication::refuse_with(
            read.original,
            &mut read.storage,
            ReservationPublicationError::Storage(StoragePublicationError::Interrupted(
                PublicationStage::DocumentBound,
            )),
            |journal, reason| match journal {
                Journal::Fixed(registry) => {
                    registry.refuse_at_for_test(reason, PublicationStageAuthority::RecordCreated)
                }
                _ => unreachable!(),
            },
        );
        assert!(
            matches!(error, ReservationPublicationError::RefusalUncertain { failure, .. } if matches!(*failure, ReservationPublicationError::Storage(StoragePublicationError::Interrupted(PublicationStage::DocumentBound))))
        );
        let before = inventory(&dir);
        assert!(matches!(
            read.revalidate(&raw, &i.check(f, 100)),
            Err(ReservationPublicationError::Refused)
        ));
        busy(root);
        drop(read);
        assert!(original.is_refused());
        assert_eq!(attempt(leaf), owned);
        assert_eq!(inventory(&dir), before);
    });
}
#[test]
fn attempt_publication_independent_concurrent_chains_preserve_separate_ownership() {
    std::thread::scope(|scope| {
        let handles: Vec<_> = [false, true]
            .into_iter()
            .map(|fail| {
                scope.spawn(move || {
                    pair!(i, original, read, dir, root, leaf, f, raw, {
                        let owned = attempt(leaf);
                        let result = publish_with(read, &raw, &i.check(f, 100), |stage| {
                            if fail && stage == PublicationStage::PrefixWritten {
                                Err(StoragePublicationError::Interrupted(stage).into())
                            } else {
                                Ok(())
                            }
                        });
                        assert_eq!(result.is_err(), fail);
                        busy(root);
                        assert!(leaf.join("active-diagnostic.json").exists());
                        assert_eq!(attempt(leaf), owned);
                        drop(result);
                        assert_eq!(original.is_refused(), fail);
                        assert_eq!(inventory(&dir).len(), if fail { 4 } else { 2 });
                        retained_context_refuses_for_test(root);
                    });
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
}
#[test]
fn attempt_publication_legacy_publisher_refuses_unowned_attempt_filename() {
    with_mount(1, |mount, _, f| {
        let (dir, fixture) = journal(f);
        let mut registry = modeled_registry_for_test(fixture);
        let fixture = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        let mut tasks = FixedSignedTasksRead {
            reader: fixture.reader,
        };
        let mut authority =
            bind_fixed_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host).unwrap();
        let i = Intent::new(f);
        let raw = i.signed(&i.body(f));
        let mut original = bind_fixed_intent(&mut authority, &raw, &i.check(f, 100)).unwrap();
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            fs::write(
                leaf.join("diagnostic-attempt-074.json"),
                b"not an owned attempt",
            )
            .unwrap();
            let failure =
                publish_fixed_reserved(&mut original, fence, &raw, &i.check(f, 100)).unwrap_err();
            assert!(failure.is_refused());
            busy(root);
            assert!(!leaf.join("active-diagnostic.json").exists());
            drop(failure);
            assert_eq!(inventory(&dir).len(), 4);
            retained_context_refuses_for_test(root);
        });
    });
}
use crate::diagnostic_worker_policy::authority_registry::PublicationStage as PublicationStageAuthority;

#[test]
fn attempt_publication_unacknowledged_storage_cannot_mint_publication() {
    use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::attempt_storage::AttemptStorage;
    for refused in [false, true] {
        with_owned_empty_fence_for_test(|mut fence, root, leaf| {
            if refused {
                fence.latch_refusal();
            }
            let held = AttemptStorage::new(fence)
                .into_publication()
                .err()
                .expect("unacknowledged original cannot transfer");
            assert_eq!(held.is_refused(), refused);
            busy(root);
            assert_eq!(fs::read_dir(leaf).unwrap().count(), 0);
            drop(held);
            drop(crate::host_execution::lock_host_root(root).unwrap());
        });
    }
}
