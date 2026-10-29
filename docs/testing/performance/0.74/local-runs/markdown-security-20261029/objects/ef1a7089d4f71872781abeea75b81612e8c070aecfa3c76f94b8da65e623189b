use super::*;
use super::super::reservation_publication::*;
use super::super::reservation_publication::PublicationStage;
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::retained_context_refuses_for_test;
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::with_owned_empty_fence_for_test;
use crate::host_execution::{HostExecutionClaim, HostExecutionError};

macro_rules! pair {
    ($i:ident, $original:ident, $fence:ident, $dir:ident, $root:ident, $leaf:ident, $f:ident, $raw:ident, $body:block) => {
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
            with_owned_empty_fence_for_test(|$fence, $root, $leaf| $body);
        });
    };
}
fn busy(root: &std::path::Path) {
    assert!(matches!(
        HostExecutionClaim::acquire(root, &"a".repeat(64)),
        Err(HostExecutionError::Busy)
    ));
}
#[test]
fn reservation_publication_healthy_canonical_repeat_drop_retains_state() {
    pair!(i, original, fence, dir, root, leaf, f, raw, {
        let before = inventory(&dir);
        let path = leaf.join("active-diagnostic.json");
        {
            let mut read =
                publish_fixed_reserved(&mut original, fence, &raw, &i.check(f, 100)).unwrap();
            busy(root);
            assert_eq!(
                crate::diagnostic_lease::decode_active(&fs::read(&path).unwrap()).unwrap(),
                i.state
            );
            read.revalidate(&raw, &i.check(f, 101)).unwrap();
            read.revalidate(&raw, &i.check(f, 102)).unwrap();
            assert!(!read.is_refused());
        }
        assert!(!original.is_refused());
        assert_eq!(inventory(&dir), before);
        assert!(path.exists());
        assert_eq!(fs::read_dir(leaf).unwrap().count(), 1);
        retained_context_refuses_for_test(root);
    });
}
#[test]
fn reservation_publication_unbound_snapshot_mismatch_is_nonrevoking_and_no_write() {
    pair!(i, original, fence, dir, root, leaf, f, raw, {
        let mut state = i.state.clone();
        state.revision += 1;
        let mut check = i.check(f, 100);
        check.state = &state;
        let failure = publish_fixed_reserved(&mut original, fence, &raw, &check).unwrap_err();
        assert!(matches!(
            failure.error(),
            ReservationPublicationError::Binding
        ));
        assert!(!failure.is_refused());
        busy(root);
        assert_eq!(fs::read_dir(leaf).unwrap().count(), 0);
        drop(failure);
        assert!(!original.is_refused());
        assert_eq!(inventory(&dir).len(), 2);
    });
}
#[test]
fn reservation_publication_all_interruption_stages_retain_lock_context_and_first_error() {
    let seed = 0x074b_u64;
    eprintln!("seed={seed:#x}");
    for stage in ALL_STAGES {
        pair!(i, original, fence, dir, root, leaf, f, raw, {
            let failure = publish_with(&mut original, fence, &raw, &i.check(f, 100), |at| {
                busy(root);
                if at == stage {
                    Err(ReservationPublicationError::Storage(
                        StoragePublicationError::Interrupted(at),
                    ))
                } else {
                    Ok(())
                }
            })
            .unwrap_err();
            assert!(
                matches!(failure.error(), ReservationPublicationError::Storage(
                StoragePublicationError::Interrupted(at)) if *at == stage)
            );
            assert!(failure.is_refused());
            busy(root);
            assert_eq!(
                leaf.join("active-diagnostic.json").exists(),
                stage != PublicationStage::BeforeCreate
            );
            let retained = fs::read(leaf.join("active-diagnostic.json")).ok();
            drop(failure);
            assert!(original.is_refused());
            assert_eq!(fs::read(leaf.join("active-diagnostic.json")).ok(), retained);
            assert_eq!(inventory(&dir).len(), 4);
            if stage != PublicationStage::BeforeCreate {
                retained_context_refuses_for_test(root);
            }
        });
    }
}

#[test]
fn reservation_publication_exclusive_create_never_adopts_or_overwrites_late_target() {
    pair!(i, original, fence, dir, root, leaf, f, raw, {
        let path = leaf.join("active-diagnostic.json");
        let foreign = crate::diagnostic_lease::encode(&i.state).unwrap();
        let failure = publish_with(&mut original, fence, &raw, &i.check(f, 100), |stage| {
            if stage == PublicationStage::BeforeCreate {
                fs::write(&path, &foreign).unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            }
            Ok(())
        })
        .unwrap_err();
        assert!(
            matches!(failure.error(), ReservationPublicationError::Storage(
            StoragePublicationError::Open(crate::diagnostic_output_preparation::PreparationError::Io(error)))
            if error.kind() == std::io::ErrorKind::AlreadyExists)
        );
        busy(root);
        assert_eq!(fs::read(&path).unwrap(), foreign);
        drop(failure);
        assert!(original.is_refused());
        assert_eq!(inventory(&dir).len(), 4);
        retained_context_refuses_for_test(root);
    });
}
#[test]
fn reservation_publication_late_unknown_or_legacy_context_retains_created_state() {
    for legacy in [false, true] {
        pair!(i, original, fence, dir, root, leaf, f, raw, {
            let conflict = if legacy {
                root.join("active-campaign")
            } else {
                leaf.join("unknown")
            };
            let failure = publish_with(&mut original, fence, &raw, &i.check(f, 100), |stage| {
                if stage == PublicationStage::DirectorySynced {
                    fs::write(&conflict, b"retained conflict").unwrap();
                }
                Ok(())
            })
            .unwrap_err();
            assert!(matches!(
                failure.error(),
                ReservationPublicationError::Storage(_)
            ));
            assert!(failure.is_refused());
            busy(root);
            let retained = fs::read(leaf.join("active-diagnostic.json")).unwrap();
            drop(failure);
            assert!(original.is_refused());
            assert_eq!(
                fs::read(leaf.join("active-diagnostic.json")).unwrap(),
                retained
            );
            assert_eq!(fs::read(&conflict).unwrap(), b"retained conflict");
            assert_eq!(inventory(&dir).len(), 4);
        });
    }
}
#[test]
fn reservation_publication_named_replacement_or_writer_drift_is_sticky() {
    for replace in [false, true] {
        pair!(i, original, fence, dir, root, leaf, f, raw, {
            let mut read =
                publish_fixed_reserved(&mut original, fence, &raw, &i.check(f, 100)).unwrap();
            let path = leaf.join("active-diagnostic.json");
            let old = fs::read(&path).unwrap();
            if replace {
                fs::rename(&path, leaf.join("saved")).unwrap();
            }
            fs::write(
                &path,
                if replace {
                    old.clone()
                } else {
                    vec![b'x'; old.len()]
                },
            )
            .unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            assert!(matches!(
                read.revalidate(&raw, &i.check(f, 101)),
                Err(ReservationPublicationError::Storage(_))
            ));
            if replace {
                fs::remove_file(&path).unwrap();
                fs::rename(leaf.join("saved"), &path).unwrap();
            } else {
                fs::write(&path, &old).unwrap();
            }
            let after = inventory(&dir);
            assert!(matches!(
                read.revalidate(&raw, &i.check(f, 100)),
                Err(ReservationPublicationError::Refused)
            ));
            busy(root);
            drop(read);
            assert!(original.is_refused());
            assert_eq!(inventory(&dir), after);
            assert_eq!(after.len(), 4);
            retained_context_refuses_for_test(root);
        });
    }
}
#[test]
fn reservation_publication_bound_expiry_state_peer_signature_and_worker_refuse_once() {
    for kind in 0..5 {
        pair!(i, original, fence, dir, root, leaf, f, raw, {
            let mut read =
                publish_fixed_reserved(&mut original, fence, &raw, &i.check(f, 100)).unwrap();
            let retained = fs::read(leaf.join("active-diagnostic.json")).unwrap();
            let mut check = i.check(f, 101);
            let mut state = i.state.clone();
            state.revision += 1;
            let peer = DiagnosticPeer {
                uid: 901,
                gids: vec![902],
            };
            let mut bytes = raw.clone();
            match kind {
                0 => check.now_unix_seconds = 160,
                1 => check.state = &state,
                2 => check.peer = &peer,
                3 => {
                    let mut document: SignedIntentProjection =
                        serde_json::from_slice(&raw).unwrap();
                    document.signature_hex = "00".repeat(64);
                    bytes = crate::canonical_json(&document).unwrap();
                    bytes.push(b'\n');
                }
                _ => check.worker_bytes = b"invalid worker",
            }
            assert!(matches!(
                read.revalidate(&bytes, &check),
                Err(ReservationPublicationError::Intent(_))
            ));
            assert!(read.is_refused());
            let after = inventory(&dir);
            assert!(matches!(
                read.revalidate(&raw, &i.check(f, 100)),
                Err(ReservationPublicationError::Refused)
            ));
            assert_eq!(inventory(&dir), after);
            assert_eq!(after.len(), 4);
            busy(root);
            drop(read);
            assert!(original.is_refused());
            assert_eq!(
                fs::read(leaf.join("active-diagnostic.json")).unwrap(),
                retained
            );
        });
    }
}
#[test]
fn reservation_publication_unsafe_or_advanced_authority_never_appended_on_storage_failure() {
    for advanced in [false, true] {
        pair!(i, original, fence, dir, root, leaf, f, raw, {
            let mut read =
                publish_fixed_reserved(&mut original, fence, &raw, &i.check(f, 100)).unwrap();
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
            let error = read.revalidate(&raw, &i.check(f, 101)).unwrap_err();
            assert!(
                matches!(error, ReservationPublicationError::RefusalUncertain { failure, .. }
                if matches!(*failure, ReservationPublicationError::Storage(StoragePublicationError::Context)))
            );
            assert!(read.is_refused());
            assert_eq!(inventory(&dir), before);
            fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
            assert!(matches!(
                read.revalidate(&raw, &i.check(f, 100)),
                Err(ReservationPublicationError::Refused)
            ));
            assert_eq!(inventory(&dir), before);
            busy(root);
        });
    }
}
#[test]
fn reservation_publication_partial_policy_refusal_keeps_original_failure_and_uncertainty() {
    use crate::diagnostic_worker_policy::authority_registry::PublicationStage as AuthorityStage;
    for stage in [
        AuthorityStage::RecordCreated,
        AuthorityStage::RecordSynced,
        AuthorityStage::MarkerCreated,
        AuthorityStage::DurableCommit,
    ] {
        pair!(i, original, fence, dir, root, leaf, f, raw, {
            let mut read =
                publish_fixed_reserved(&mut original, fence, &raw, &i.check(f, 100)).unwrap();
            let error = refuse_with(
                read.original,
                &mut read.storage,
                ReservationPublicationError::Storage(StoragePublicationError::Interrupted(
                    PublicationStage::DocumentBound,
                )),
                |journal, reason| match journal {
                    Journal::Fixed(registry) => registry.refuse_at_for_test(reason, stage),
                    _ => unreachable!(),
                },
            );
            assert!(
                matches!(error, ReservationPublicationError::RefusalUncertain { failure, publication: AuthorityError::Injected(at) }
                if at == stage && matches!(*failure, ReservationPublicationError::Storage(
                    StoragePublicationError::Interrupted(PublicationStage::DocumentBound))))
            );
            let after = inventory(&dir);
            assert!(matches!(
                read.revalidate(&raw, &i.check(f, 100)),
                Err(ReservationPublicationError::Refused)
            ));
            assert_eq!(inventory(&dir), after);
            busy(root);
            assert!(leaf.join("active-diagnostic.json").exists());
        });
    }
}
#[test]
fn reservation_publication_already_refused_inputs_cross_latch_without_write_or_journal_io() {
    for storage_failed in [false, true] {
        pair!(i, original, fence, dir, root, leaf, f, raw, {
            let mut fence = fence;
            if storage_failed {
                fence.latch_refusal();
            } else {
                original.revalidate(&raw, &i.check(f, 160)).unwrap_err();
            }
            let before = inventory(&dir);
            fs::set_permissions(leaf, fs::Permissions::from_mode(0o000)).unwrap();
            let failure =
                publish_fixed_reserved(&mut original, fence, b"invalid", &i.check(f, u64::MAX))
                    .unwrap_err();
            assert!(matches!(
                failure.error(),
                ReservationPublicationError::Refused
            ));
            assert!(failure.is_refused());
            busy(root);
            assert_eq!(inventory(&dir), before);
            drop(failure);
            assert!(original.is_refused());
            fs::set_permissions(leaf, fs::Permissions::from_mode(0o700)).unwrap();
            assert_eq!(fs::read_dir(leaf).unwrap().count(), 0);
        });
    }
}
#[test]
fn reservation_publication_post_create_original_intent_failure_is_not_duplicated() {
    pair!(i, original, fence, dir, root, leaf, f, raw, {
        let failure = publish_with(&mut original, fence, &raw, &i.check(f, 100), |stage| {
            if stage == PublicationStage::StateCreated {
                fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o770)).unwrap();
            }
            Ok(())
        })
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            ReservationPublicationError::Intent(_)
        ));
        assert!(failure.is_refused());
        busy(root);
        assert_eq!(inventory(&dir).len(), 2);
        assert_eq!(fs::read(leaf.join("active-diagnostic.json")).unwrap(), b"");
        drop(failure);
        assert!(original.is_refused());
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        retained_context_refuses_for_test(root);
    });
}
#[test]
fn reservation_publication_independent_concurrent_chains_keep_separate_state_and_refusal() {
    std::thread::scope(|scope| {
        let handles: Vec<_> = [false, true]
            .into_iter()
            .map(|fail| {
                scope.spawn(move || {
                    pair!(i, original, fence, dir, root, leaf, f, raw, {
                        let result =
                            publish_with(&mut original, fence, &raw, &i.check(f, 100), |stage| {
                                if fail && stage == PublicationStage::PrefixWritten {
                                    Err(StoragePublicationError::Interrupted(stage).into())
                                } else {
                                    Ok(())
                                }
                            });
                        assert_eq!(result.is_err(), fail);
                        busy(root);
                        assert!(leaf.join("active-diagnostic.json").exists());
                        drop(result);
                        assert_eq!(original.is_refused(), fail);
                        assert_eq!(inventory(&dir).len(), if fail { 4 } else { 2 });
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
fn reservation_publication_wrappers_and_failure_are_original_thread_only() {
    macro_rules! check {
        ($kind:ty) => {{
            trait AmbiguousSend<A> {
                fn marker() {}
            }
            impl<T: ?Sized> AmbiguousSend<()> for T {}
            impl<T: ?Sized + Send> AmbiguousSend<u8> for T {}
            let _ = <$kind as AmbiguousSend<_>>::marker;
            trait AmbiguousSync<A> {
                fn marker() {}
            }
            impl<T: ?Sized> AmbiguousSync<()> for T {}
            impl<T: ?Sized + Sync> AmbiguousSync<u8> for T {}
            let _ = <$kind as AmbiguousSync<_>>::marker;
        }};
    }
    check!(
        FixedReservedPublicationRead<
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
        >
    );
    check!(ReservationPublicationFailure<'static>);
}

#[test]
fn reservation_publication_valid_later_reserved_snapshots_are_not_initial_reservations() {
    for (revision, cells) in [(2, 0), (1, 1)] {
        with_mount(1, |mount, _, f| {
            let (dir, fixture) = journal(f);
            let mut registry = modeled_registry_for_test(fixture);
            let fixture = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
            let mut tasks = FixedSignedTasksRead {
                reader: fixture.reader,
            };
            let mut authority =
                bind_fixed_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host)
                    .unwrap();
            let mut i = Intent::new(f);
            i.state.revision = revision;
            i.state.completed_cells = cells;
            let raw = i.signed(&i.body(f));
            let mut original = bind_fixed_intent(&mut authority, &raw, &i.check(f, 100)).unwrap();
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let failure = publish_fixed_reserved(&mut original, fence, &raw, &i.check(f, 100))
                    .unwrap_err();
                assert!(matches!(
                    failure.error(),
                    ReservationPublicationError::Binding
                ));
                assert!(!failure.is_refused());
                busy(root);
                assert_eq!(fs::read_dir(leaf).unwrap().count(), 0);
            });
            assert!(!original.is_refused());
            assert_eq!(inventory(&dir).len(), 2);
        });
    }
}
#[test]
fn reservation_publication_created_permissions_links_and_name_substitution_refuse_before_write() {
    for kind in 0..3 {
        pair!(i, original, fence, dir, root, leaf, f, raw, {
            let path = leaf.join("active-diagnostic.json");
            let failure = publish_with(&mut original, fence, &raw, &i.check(f, 100), |stage| {
                if stage == PublicationStage::StateCreated {
                    match kind {
                        0 => fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap(),
                        1 => fs::hard_link(&path, root.join("retained-link")).unwrap(),
                        _ => {
                            fs::rename(&path, root.join("retained-original")).unwrap();
                            fs::write(&path, b"").unwrap();
                            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
                        }
                    }
                }
                Ok(())
            })
            .unwrap_err();
            assert!(matches!(failure.error(),ReservationPublicationError::Storage(
                StoragePublicationError::State(crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::FixedReservedStateError::Document))));
            assert!(failure.is_refused());
            busy(root);
            assert_eq!(fs::read(&path).unwrap(), b"");
            drop(failure);
            assert!(original.is_refused());
            assert_eq!(inventory(&dir).len(), 4);
            retained_context_refuses_for_test(root);
        });
    }
}
