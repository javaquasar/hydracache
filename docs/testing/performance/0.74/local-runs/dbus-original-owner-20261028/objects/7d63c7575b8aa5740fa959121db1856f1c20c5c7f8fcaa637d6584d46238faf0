use super::*;
use super::super::early_attempt::*;
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::{retained_context_refuses_for_test, with_owned_empty_fence_for_test};
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
fn early_attempt_healthy_canonical_sealed_and_drop_blocks_reservation() {
    pair!(i, original, fence, dir, root, leaf, f, raw, {
        let before = inventory(&dir);
        let mut read = retain_fixed_attempt(&mut original, fence, &raw, &i.check(f, 100)).unwrap();
        busy(root);
        let path = leaf.join("diagnostic-attempt-074.json");
        let retained = fs::read(&path).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&retained).unwrap();
        assert_eq!(value["schema_version"], "diagnostic-retained-intent-074-v1");
        assert_eq!(
            value["signed_intent"],
            String::from_utf8(raw.clone()).unwrap()
        );
        assert_eq!(value["state"], serde_json::to_value(&i.state).unwrap());
        assert!(value["previous_record_sha256"].is_null());
        assert_eq!(retained.last(), Some(&b'\n'));
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o7777,
            0o400
        );
        assert!(!leaf.join("active-diagnostic.json").exists());
        assert_eq!(fs::read_dir(leaf).unwrap().count(), 1);
        read.revalidate(&raw, &i.check(f, 101)).unwrap();
        read.revalidate(&raw, &i.check(f, 102)).unwrap();
        assert!(!read.is_refused());
        drop(read);
        assert_eq!(fs::read(&path).unwrap(), retained);
        assert_eq!(inventory(&dir), before);
        assert!(!original.is_refused());
        retained_context_refuses_for_test(root);
    });
}

#[test]
fn early_attempt_unbound_state_mismatch_is_nonrevoking_and_no_write() {
    pair!(i, original, fence, dir, root, leaf, f, raw, {
        let mut changed = i.state.clone();
        changed.revision += 1;
        let mut check = i.check(f, 100);
        check.state = &changed;
        let failure = retain_fixed_attempt(&mut original, fence, &raw, &check).unwrap_err();
        assert!(matches!(failure.error(), EarlyAttemptError::Binding));
        assert!(!failure.is_refused());
        busy(root);
        assert_eq!(fs::read_dir(leaf).unwrap().count(), 0);
        drop(failure);
        assert!(!original.is_refused());
        assert_eq!(inventory(&dir).len(), 2);
    });
}

#[test]
fn early_attempt_all_boundaries_retain_first_error_lock_and_partial_context() {
    eprintln!("early attempt seed=0x074d");
    for stage in ALL_ATTEMPT_STAGES {
        pair!(i, original, fence, dir, root, leaf, f, raw, {
            let failure = retain_with(&mut original, fence, &raw, &i.check(f, 100), |at| {
                busy(root);
                if at == stage {
                    Err(EarlyAttemptError::Storage(
                        StorageAttemptError::Interrupted(at),
                    ))
                } else {
                    Ok(())
                }
            })
            .unwrap_err();
            assert!(
                matches!(failure.error(), EarlyAttemptError::Storage(StorageAttemptError::Interrupted(at)) if *at == stage)
            );
            assert!(failure.is_refused());
            busy(root);
            let path = leaf.join("diagnostic-attempt-074.json");
            let retained = fs::read(&path).ok();
            assert_eq!(path.exists(), stage != AttemptStage::BeforeCreate);
            drop(failure);
            assert!(original.is_refused());
            assert_eq!(fs::read(&path).ok(), retained);
            assert_eq!(inventory(&dir).len(), 4);
            assert!(!leaf.join("active-diagnostic.json").exists());
            if stage != AttemptStage::BeforeCreate {
                retained_context_refuses_for_test(root);
            }
        });
    }
}

#[test]
fn early_attempt_late_target_or_unknown_context_is_not_adopted() {
    for target in [true, false] {
        pair!(i, original, fence, dir, root, leaf, f, raw, {
            let path = leaf.join(if target {
                "diagnostic-attempt-074.json"
            } else {
                "unknown"
            });
            let foreign = b"original foreign context";
            let failure = retain_with(&mut original, fence, &raw, &i.check(f, 100), |at| {
                if at == AttemptStage::BeforeCreate {
                    fs::write(&path, foreign).unwrap();
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
                }
                Ok(())
            })
            .unwrap_err();
            assert!(failure.is_refused());
            busy(root);
            drop(failure);
            assert_eq!(fs::read(path).unwrap(), foreign);
            assert_eq!(inventory(&dir).len(), 4);
            retained_context_refuses_for_test(root);
        });
    }
}

#[test]
fn early_attempt_original_object_byte_mode_link_and_named_drift_are_sticky() {
    for mode in 0..4 {
        pair!(i, original, fence, dir, root, leaf, f, raw, {
            let mut read =
                retain_fixed_attempt(&mut original, fence, &raw, &i.check(f, 100)).unwrap();
            let path = leaf.join("diagnostic-attempt-074.json");
            match mode {
                0 => {
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
                    fs::write(&path, b"drift").unwrap();
                }
                1 => fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap(),
                2 => fs::hard_link(&path, leaf.join("alias")).unwrap(),
                _ => {
                    let old = fs::read(&path).unwrap();
                    fs::rename(&path, leaf.join("saved")).unwrap();
                    fs::write(&path, old).unwrap();
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
                }
            }
            assert!(matches!(
                read.revalidate(&raw, &i.check(f, 101)),
                Err(EarlyAttemptError::Storage(_))
            ));
            let after = inventory(&dir);
            assert!(matches!(
                read.revalidate(&raw, &i.check(f, 100)),
                Err(EarlyAttemptError::Refused)
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
fn early_attempt_bound_expiry_signature_peer_and_state_refuse_once() {
    for mode in 0..4 {
        pair!(i, original, fence, dir, root, leaf, f, raw, {
            let mut read =
                retain_fixed_attempt(&mut original, fence, &raw, &i.check(f, 100)).unwrap();
            let retained = fs::read(leaf.join("diagnostic-attempt-074.json")).unwrap();
            let mut check = i.check(f, 101);
            let mut state = i.state.clone();
            let mut peer = DiagnosticPeer {
                uid: i.peer.uid,
                gids: i.peer.gids.clone(),
            };
            let mut bytes = raw.clone();
            match mode {
                0 => check.now_unix_seconds = 161,
                1 => bytes[0] = b'x',
                2 => {
                    peer.uid = 901;
                    check.peer = &peer;
                }
                _ => {
                    state.revision += 1;
                    check.state = &state;
                }
            }
            assert!(read.revalidate(&bytes, &check).is_err());
            assert!(read.is_refused());
            let after = inventory(&dir);
            assert!(matches!(
                read.revalidate(&raw, &i.check(f, 100)),
                Err(EarlyAttemptError::Refused)
            ));
            busy(root);
            drop(read);
            assert_eq!(inventory(&dir), after);
            assert_eq!(after.len(), 4);
            assert_eq!(
                fs::read(leaf.join("diagnostic-attempt-074.json")).unwrap(),
                retained
            );
        });
    }
}

#[test]
fn early_attempt_unsafe_or_advanced_authority_is_not_appended() {
    for advanced in [false, true] {
        pair!(i, original, fence, dir, root, leaf, f, raw, {
            let mut read =
                retain_fixed_attempt(&mut original, fence, &raw, &i.check(f, 100)).unwrap();
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
                matches!(error, EarlyAttemptError::RefusalUncertain { failure, .. }
                if matches!(*failure, EarlyAttemptError::Storage(StorageAttemptError::Context)))
            );
            assert_eq!(inventory(&dir), before);
            assert!(read.is_refused());
            fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
            assert!(matches!(
                read.revalidate(&raw, &i.check(f, 100)),
                Err(EarlyAttemptError::Refused)
            ));
            assert_eq!(inventory(&dir), before);
            busy(root);
        });
    }
}
#[test]
fn early_attempt_partial_policy_refusal_preserves_first_failure() {
    for stage in [
        PublicationStage::RecordCreated,
        PublicationStage::RecordSynced,
        PublicationStage::MarkerCreated,
        PublicationStage::DurableCommit,
    ] {
        pair!(i, original, fence, dir, root, leaf, f, raw, {
            let mut read =
                retain_fixed_attempt(&mut original, fence, &raw, &i.check(f, 100)).unwrap();
            let error = refuse_with(
                read.original,
                &mut read.storage,
                EarlyAttemptError::Storage(StorageAttemptError::Interrupted(
                    AttemptStage::Acknowledging,
                )),
                |journal, reason| match journal {
                    Journal::Fixed(registry) => registry.refuse_at_for_test(reason, stage),
                    _ => unreachable!(),
                },
            );
            assert!(
                matches!(error, EarlyAttemptError::RefusalUncertain { failure, publication: AuthorityError::Injected(at) }
                if at == stage && matches!(*failure, EarlyAttemptError::Storage(StorageAttemptError::Interrupted(AttemptStage::Acknowledging))))
            );
            let before = inventory(&dir);
            assert!(matches!(
                read.revalidate(&raw, &i.check(f, 100)),
                Err(EarlyAttemptError::Refused)
            ));
            assert_eq!(inventory(&dir), before);
            busy(root);
            assert!(leaf.join("diagnostic-attempt-074.json").exists());
        });
    }
}
#[test]
fn early_attempt_already_refused_inputs_cross_latch_without_io() {
    for failed_storage in [false, true] {
        pair!(i, original, fence, dir, root, leaf, f, raw, {
            let mut fence = fence;
            if failed_storage {
                fence.latch_refusal();
            } else {
                original.revalidate(&raw, &i.check(f, 160)).unwrap_err();
            }
            let before = inventory(&dir);
            fs::set_permissions(leaf, fs::Permissions::from_mode(0o000)).unwrap();
            let failure =
                retain_fixed_attempt(&mut original, fence, b"invalid", &i.check(f, u64::MAX))
                    .unwrap_err();
            assert!(matches!(failure.error(), EarlyAttemptError::Refused));
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
fn early_attempt_independent_concurrent_chains_do_not_share_refusal() {
    std::thread::scope(|scope| {
        let handles: Vec<_> = [false, true]
            .into_iter()
            .map(|fail| {
                scope.spawn(move || {
                    pair!(i, original, fence, dir, root, leaf, f, raw, {
                        let result =
                            retain_with(&mut original, fence, &raw, &i.check(f, 100), |at| {
                                if fail && at == AttemptStage::PrefixWritten {
                                    Err(StorageAttemptError::Interrupted(at).into())
                                } else {
                                    Ok(())
                                }
                            });
                        assert_eq!(result.is_err(), fail);
                        busy(root);
                        assert!(leaf.join("diagnostic-attempt-074.json").exists());
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
fn early_attempt_storage_budget_and_same_guard_replay_refuse_before_creation() {
    use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::attempt_storage::AttemptStorage;
    for bytes in [Vec::new(), vec![b'x'; 16_385]] {
        pair!(i, original, fence, _dir, root, leaf, f, _raw, {
            let mut storage = AttemptStorage::new(fence);
            assert!(matches!(
                storage.retain_with::<StorageAttemptError>(&bytes, |_| Ok(())),
                Err(StorageAttemptError::Document)
            ));
            assert!(matches!(
                storage.retain_with::<StorageAttemptError>(b"x", |_| Ok(())),
                Err(StorageAttemptError::Refused)
            ));
            busy(root);
            assert_eq!(fs::read_dir(leaf).unwrap().count(), 0);
            drop(storage);
            assert!(!original.is_refused());
            original.revalidate(&_raw, &i.check(f, 100)).unwrap();
        });
    }
}
#[test]
fn early_attempt_runtime_success_and_failure_are_original_thread_only() {
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
        FixedEarlyAttemptRead<
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
    check!(EarlyAttemptFailure<'static>);
}
