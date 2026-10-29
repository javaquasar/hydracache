use super::*;
use super::super::initial_consumption::*;
use super::super::reservation_publication::publish_fixed_reserved;
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::{retained_context_refuses_for_test, with_owned_empty_fence_for_test};
use crate::host_execution::{HostExecutionClaim, HostExecutionError};

#[test]
fn initial_consumption_success_and_failure_are_original_thread_only() {
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
        FixedConsumedInitialIntentRead<
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
    check!(
        InitialConsumptionFailure<
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
}

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
                    publish_fixed_reserved(&mut $original, fence, &$raw, &$i.check($f, 100))
                        .unwrap();
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
fn retained(leaf: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut entries: Vec<_> = fs::read_dir(leaf)
        .unwrap()
        .map(|e| {
            let p = e.unwrap().path();
            (
                p.file_name().unwrap().to_str().unwrap().to_owned(),
                fs::read(p).unwrap(),
            )
        })
        .collect();
    entries.sort();
    entries
}
#[test]
fn initial_consumption_healthy_canonical_sealed_repeat_and_drop() {
    pair!(i, original, read, dir, root, leaf, f, raw, {
        let before = inventory(&dir);
        let state_before = fs::read(leaf.join("active-diagnostic.json")).unwrap();
        let mut read = consume_fixed_initial_intent(read, &raw, &i.check(f, 100)).unwrap();
        busy(root);
        let record = fs::read(leaf.join("diagnostic-intent-cell-1.json")).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&record).unwrap();
        assert_eq!(value["schema_version"], "diagnostic-retained-intent-074-v1");
        assert_eq!(
            value["signed_intent"],
            String::from_utf8(raw.clone()).unwrap()
        );
        assert!(value["previous_record_sha256"].is_null());
        assert_eq!(value["state"], serde_json::to_value(&i.state).unwrap());
        assert_eq!(record.last(), Some(&b'\n'));
        assert_eq!(
            fs::read(leaf.join("diagnostic-intent-cell-1.commit")).unwrap(),
            format!("{}\n", crate::sha256_hex(&record)).as_bytes()
        );
        for name in [
            "diagnostic-intent-cell-1.json",
            "diagnostic-intent-cell-1.commit",
        ] {
            assert_eq!(
                fs::metadata(leaf.join(name)).unwrap().permissions().mode() & 0o7777,
                0o400
            );
        }
        assert_eq!(
            fs::read(leaf.join("active-diagnostic.json")).unwrap(),
            state_before
        );
        let bytes = retained(leaf);
        read.revalidate(&raw, &i.check(f, 101)).unwrap();
        read.revalidate(&raw, &i.check(f, 102)).unwrap();
        assert!(!read.is_refused());
        assert_eq!(retained(leaf), bytes);
        drop(read);
        assert!(!original.is_refused());
        assert_eq!(inventory(&dir), before);
        retained_context_refuses_for_test(root);
    });
}
#[test]
fn initial_consumption_all_interruptions_keep_lock_and_partial_objects() {
    eprintln!("seed=0x074c");
    for stage in ALL_CONSUMPTION_STAGES {
        pair!(i, original, read, dir, root, leaf, f, raw, {
            let failure = consume_with(read, &raw, &i.check(f, 100), |at| {
                busy(root);
                if stage == at {
                    Err(ReservationPublicationError::Storage(
                        StoragePublicationError::ConsumptionInterrupted(at),
                    ))
                } else {
                    Ok(())
                }
            })
            .unwrap_err();
            assert!(
                matches!(failure.error(), ReservationPublicationError::Storage(StoragePublicationError::ConsumptionInterrupted(at)) if *at == stage)
            );
            assert!(failure.is_refused());
            busy(root);
            let bytes = retained(leaf);
            assert_eq!(
                bytes.len(),
                if stage == ConsumptionStage::BeforeRecord {
                    1
                } else if stage < ConsumptionStage::CommitCreated {
                    2
                } else {
                    3
                }
            );
            drop(failure);
            assert!(original.is_refused());
            assert_eq!(retained(leaf), bytes);
            assert_eq!(inventory(&dir).len(), 4);
            retained_context_refuses_for_test(root);
        });
    }
}

#[test]
fn initial_consumption_unbound_state_mismatch_is_nonrevoking() {
    pair!(i, original, read, dir, root, leaf, f, raw, {
        let before = retained(leaf);
        let journal_before = inventory(&dir);
        let mut state = i.state.clone();
        state.revision += 1;
        let mut check = i.check(f, 100);
        check.state = &state;
        let failure = consume_fixed_initial_intent(read, &raw, &check).unwrap_err();
        assert!(matches!(
            failure.error(),
            ReservationPublicationError::Binding
        ));
        assert!(!failure.is_refused());
        busy(root);
        drop(failure);
        assert!(!original.is_refused());
        assert_eq!(retained(leaf), before);
        assert_eq!(inventory(&dir), journal_before);
    });
}
#[test]
fn initial_consumption_late_targets_unknown_and_legacy_context_never_adopted() {
    for kind in 0..4 {
        pair!(i, original, read, dir, root, leaf, f, raw, {
            let target = match kind {
                0 => leaf.join("diagnostic-intent-cell-1.json"),
                1 => leaf.join("diagnostic-intent-cell-1.commit"),
                2 => leaf.join("unknown"),
                _ => root.join("active-campaign"),
            };
            let failure = consume_with(read, &raw, &i.check(f, 100), |at| {
                if at
                    == if kind == 1 {
                        ConsumptionStage::BeforeCommit
                    } else {
                        ConsumptionStage::BeforeRecord
                    }
                {
                    fs::write(&target, b"foreign retained bytes").unwrap();
                    fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
                }
                Ok(())
            })
            .unwrap_err();
            assert!(failure.is_refused());
            busy(root);
            assert_eq!(fs::read(&target).unwrap(), b"foreign retained bytes");
            drop(failure);
            assert!(original.is_refused());
            assert_eq!(inventory(&dir).len(), 4);
            if kind == 3 {
                assert!(HostExecutionClaim::acquire(root, &"a".repeat(64)).is_err());
            } else {
                retained_context_refuses_for_test(root);
            }
        });
    }
}
#[test]
fn initial_consumption_bound_drift_after_creation_does_not_duplicate_refusal() {
    pair!(i, original, read, dir, root, leaf, f, raw, {
        let failure = consume_with(read, &raw, &i.check(f, 100), |at| {
            if at == ConsumptionStage::RecordCreated {
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
        assert_eq!(
            fs::read(leaf.join("diagnostic-intent-cell-1.json")).unwrap(),
            b""
        );
        assert_eq!(inventory(&dir).len(), 2);
        drop(failure);
        assert!(original.is_refused());
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    });
}
#[test]
fn initial_consumption_bound_expiry_peer_signature_worker_refuse_before_record() {
    for kind in 0..4 {
        pair!(i, original, read, dir, root, leaf, f, raw, {
            let before = retained(leaf);
            let peer = DiagnosticPeer {
                uid: 901,
                gids: vec![902],
            };
            let mut check = i.check(f, 101);
            let mut bytes = raw.clone();
            match kind {
                0 => check.now_unix_seconds = 160,
                1 => check.peer = &peer,
                2 => {
                    let mut signed: SignedIntentProjection = serde_json::from_slice(&raw).unwrap();
                    signed.signature_hex = "00".repeat(64);
                    bytes = crate::canonical_json(&signed).unwrap();
                    bytes.push(b'\n');
                }
                _ => check.worker_bytes = b"invalid worker",
            }
            let failure = consume_fixed_initial_intent(read, &bytes, &check).unwrap_err();
            assert!(matches!(
                failure.error(),
                ReservationPublicationError::Intent(_)
            ));
            assert!(failure.is_refused());
            busy(root);
            drop(failure);
            assert_eq!(retained(leaf), before);
            assert_eq!(inventory(&dir).len(), 4);
        });
    }
}
#[test]
fn initial_consumption_created_metadata_and_link_faults_refuse() {
    for kind in 0..4 {
        pair!(i, original, read, dir, root, leaf, f, raw, {
            let path = leaf.join("diagnostic-intent-cell-1.json");
            let failure = consume_with(read, &raw, &i.check(f, 100), |at| {
                if at == ConsumptionStage::RecordCreated {
                    match kind {
                        0 => fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap(),
                        1 => fs::hard_link(&path, root.join("retained-link")).unwrap(),
                        2 => {
                            fs::rename(&path, root.join("retained-original")).unwrap();
                            std::os::unix::fs::symlink(root.join("retained-original"), &path)
                                .unwrap();
                        }
                        _ => {
                            fs::rename(&path, root.join("retained-original")).unwrap();
                            fs::write(&path, []).unwrap();
                            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
                        }
                    }
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
            drop(failure);
            assert!(original.is_refused());
            assert_eq!(inventory(&dir).len(), 4);
        });
    }
}
#[test]
fn initial_consumption_post_ack_objects_state_and_namespace_are_sticky() {
    for kind in 0..6 {
        pair!(i, original, read, dir, root, leaf, f, raw, {
            let mut read = consume_fixed_initial_intent(read, &raw, &i.check(f, 100)).unwrap();
            let path = leaf.join(if kind == 1 {
                "diagnostic-intent-cell-1.commit"
            } else if kind == 2 {
                "active-diagnostic.json"
            } else {
                "diagnostic-intent-cell-1.json"
            });
            let bytes = fs::read(&path).unwrap();
            match kind {
                0..=2 => {
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
                    fs::write(&path, vec![b'x'; bytes.len()]).unwrap();
                }
                3 => {
                    fs::rename(&path, root.join("saved")).unwrap();
                    fs::write(&path, &bytes).unwrap();
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
                }
                4 => fs::hard_link(&path, root.join("saved")).unwrap(),
                _ => fs::write(leaf.join("unknown"), b"retained").unwrap(),
            }
            assert!(matches!(
                read.revalidate(&raw, &i.check(f, 101)),
                Err(ReservationPublicationError::Storage(_))
            ));
            assert!(read.is_refused());
            busy(root);
            fs::set_permissions(leaf, fs::Permissions::from_mode(0o000)).unwrap();
            let after = inventory(&dir);
            assert!(matches!(
                read.revalidate(b"invalid", &i.check(f, u64::MAX)),
                Err(ReservationPublicationError::Refused)
            ));
            assert_eq!(inventory(&dir), after);
            drop(read);
            fs::set_permissions(leaf, fs::Permissions::from_mode(0o700)).unwrap();
            assert!(original.is_refused());
            assert_eq!(after.len(), 4);
        });
    }
}
#[test]
fn initial_consumption_unsafe_or_advanced_authority_keeps_first_storage_error() {
    for advanced in [false, true] {
        pair!(i, original, read, dir, root, leaf, f, raw, {
            let mut read = consume_fixed_initial_intent(read, &raw, &i.check(f, 100)).unwrap();
            if advanced {
                let changed = changed_checked(f, 0);
                match &mut read.read.original.original.journal {
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
            assert!(
                matches!(read.revalidate(&raw, &i.check(f, 101)), Err(ReservationPublicationError::RefusalUncertain { failure, .. }) if matches!(*failure, ReservationPublicationError::Storage(StoragePublicationError::Context)))
            );
            assert!(read.is_refused());
            busy(root);
            assert_eq!(inventory(&dir), before);
            drop(read);
            fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        });
    }
}
#[test]
fn initial_consumption_already_refused_is_no_io_and_cannot_replay_storage() {
    pair!(i, original, read, dir, root, leaf, f, raw, {
        let mut read = consume_fixed_initial_intent(read, &raw, &i.check(f, 100)).unwrap();
        let before = retained(leaf);
        let mut entered = false;
        assert!(matches!(
            read.read
                .storage
                .consume_initial_with::<ReservationPublicationError>(b"second record", |_| {
                    entered = true;
                    Ok(())
                }),
            Err(ReservationPublicationError::Storage(
                StoragePublicationError::Refused
            ))
        ));
        assert!(!entered);
        assert_eq!(retained(leaf), before);
        read.read.storage.latch_refusal();
        let journal_before = inventory(&dir);
        fs::set_permissions(leaf, fs::Permissions::from_mode(0o000)).unwrap();
        assert!(matches!(
            read.revalidate(b"invalid", &i.check(f, u64::MAX)),
            Err(ReservationPublicationError::Refused)
        ));
        assert_eq!(inventory(&dir), journal_before);
        busy(root);
        drop(read);
        fs::set_permissions(leaf, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(original.is_refused());
    });
}
#[test]
fn initial_consumption_independent_concurrent_chains_do_not_share_refusal() {
    std::thread::scope(|scope| {
        let handles: Vec<_> = [false, true]
            .into_iter()
            .map(|fail| {
                scope.spawn(move || {
                    pair!(i, original, read, dir, root, leaf, f, raw, {
                        let result = consume_with(read, &raw, &i.check(f, 100), |at| {
                            if fail && at == ConsumptionStage::CommitPrefixWritten {
                                Err(StoragePublicationError::ConsumptionInterrupted(at).into())
                            } else {
                                Ok(())
                            }
                        });
                        assert_eq!(result.is_err(), fail);
                        busy(root);
                        drop(result);
                        assert_eq!(original.is_refused(), fail);
                        assert_eq!(inventory(&dir).len(), if fail { 4 } else { 2 });
                        retained_context_refuses_for_test(root);
                        assert_eq!(retained(leaf).len(), 3);
                    });
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
}
