use super::super::fenced_state::*;
use super::super::fenced_state::{bracket, Step};
use super::*;
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::{
    with_fence_for_test, OriginalHostFenceError,
};
use crate::host_execution::{HostExecutionClaim, HostExecutionError};

macro_rules! pair {
    ($i:ident, $original:ident, $fence:ident, $dir:ident, $root:ident, $f:ident, $raw:ident, $body:block $(, $after:block)?) => {
        with_mount(1, |mount, _, $f| {
            let ($dir, mut registry) = journal($f);
            let mut tasks =
                open_signed_fixture_tasks(mount, &$f.bytes, &$f.trust, &$f.host).unwrap();
            let mut authority =
                bind_fixture_authority(&mut tasks, &mut registry, &$f.bytes, &$f.trust, &$f.host)
                    .unwrap();
            let $i = Intent::new($f);
            let $raw = $i.signed(&$i.body($f));
            let mut $original =
                bind_fixture_intent(&mut authority, &$raw, &$i.check($f, 100)).unwrap();
            with_fence_for_test(&$i.state, |$fence, $root| $body);
            $(
                drop($original);
                drop(authority);
                drop(tasks);
                drop(registry);
                $after
            )?
        });
    };
}
fn leaf(root: &std::path::Path) -> std::path::PathBuf {
    root.parent().unwrap().join("diagnostic-intents-074")
}
#[test]
fn fenced_intent_refused_fence_revokes_original_without_journal_io() {
    pair!(i, original, fence, dir, root, f, raw, {
        fence.latch_refusal();
        let before = inventory(&dir);
        fs::remove_file(leaf(root).join("active-diagnostic.json")).unwrap();
        assert!(matches!(
            bind_fixture_fenced_intent(&mut original, fence, b"invalid", &i.check(f, u64::MAX)),
            Err(FencedIntentError::Refused)
        ));
        assert!(original.is_refused());
        assert_eq!(inventory(&dir), before);
    });
}
#[test]
fn fenced_intent_partial_refusal_preserves_first_fence_error_and_uncertainty() {
    for stage in [
        PublicationStage::RecordCreated,
        PublicationStage::RecordSynced,
        PublicationStage::MarkerCreated,
        PublicationStage::DurableCommit,
    ] {
        pair!(i, original, fence, dir, root, f, raw, {
            let mut read =
                bind_fixture_fenced_intent(&mut original, fence, &raw, &i.check(f, 100)).unwrap();
            let error = read.reader.reject_fence_with(
                FencedIntentError::Fence(OriginalHostFenceError::Conflict),
                |journal, reason| match journal {
                    Journal::Fixture(registry) => registry.refuse_at_for_test(reason, stage),
                    _ => unreachable!(),
                },
            );
            assert!(matches!(error, FencedIntentError::RefusalUncertain {
                failure, publication: AuthorityError::Injected(at),
            } if at == stage && matches!(*failure, FencedIntentError::Fence(OriginalHostFenceError::Conflict))));
            let after = inventory(&dir);
            assert!(read.is_refused());
            fs::remove_file(leaf(root).join("active-diagnostic.json")).unwrap();
            assert!(matches!(
                read.revalidate(&raw, &i.check(f, 100)),
                Err(FencedIntentError::Refused)
            ));
            assert_eq!(inventory(&dir), after);
        });
    }
}
#[test]
fn fenced_intent_public_wrappers_are_original_thread_only() {
    trait AmbiguousSend<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSend<()> for T {}
    impl<T: ?Sized + Send> AmbiguousSend<u8> for T {}
    let _ = <FixedFencedIntentRead<
        'static,
        'static,
        'static,
        'static,
        'static,
        'static,
        'static,
        'static,
        'static,
        'static,
    > as AmbiguousSend<_>>::marker;
    let _ = <FixtureFencedIntentRead<
        'static,
        'static,
        'static,
        'static,
        'static,
        'static,
        'static,
        'static,
        'static,
        'static,
    > as AmbiguousSend<_>>::marker;
    trait AmbiguousSync<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSync<()> for T {}
    impl<T: ?Sized + Sync> AmbiguousSync<u8> for T {}
    let _ = <FixedFencedIntentRead<
        'static,
        'static,
        'static,
        'static,
        'static,
        'static,
        'static,
        'static,
        'static,
        'static,
    > as AmbiguousSync<_>>::marker;
    let _ = <FixtureFencedIntentRead<
        'static,
        'static,
        'static,
        'static,
        'static,
        'static,
        'static,
        'static,
        'static,
        'static,
    > as AmbiguousSync<_>>::marker;
}
#[test]
fn fenced_intent_bracket_preserves_all_first_error_positions() {
    let order = [
        Step::Fence,
        Step::Intent,
        Step::Fence,
        Step::Intent,
        Step::Fence,
    ];
    for fail in 0..=5 {
        let mut seen = vec![];
        let result = bracket(|step| {
            seen.push(step);
            if seen.len() - 1 == fail {
                Err(FencedIntentError::Binding)
            } else {
                Ok(())
            }
        });
        assert_eq!(seen, order[..(fail + 1).min(5)]);
        assert_eq!(result.is_err(), fail < 5);
    }
}
#[test]
fn fenced_intent_healthy_repeat_and_drop_keep_original_flock_and_bytes() {
    pair!(i, original, fence, dir, root, f, raw, {
        let before = inventory(&dir);
        let bytes = fs::read(leaf(root).join("active-diagnostic.json")).unwrap();
        {
            let mut read =
                bind_fixture_fenced_intent(&mut original, fence, &raw, &i.check(f, 100)).unwrap();
            read.revalidate(&raw, &i.check(f, 101)).unwrap();
            read.revalidate(&raw, &i.check(f, 102)).unwrap();
            assert!(!read.is_refused());
            assert!(matches!(
                HostExecutionClaim::acquire(root, &"d".repeat(64)),
                Err(HostExecutionError::Busy)
            ));
        }
        fence.revalidate().unwrap();
        original.revalidate(&raw, &i.check(f, 103)).unwrap();
        assert!(matches!(
            HostExecutionClaim::acquire(root, &"d".repeat(64)),
            Err(HostExecutionError::Busy)
        ));
        assert_eq!(inventory(&dir), before);
        assert_eq!(
            fs::read(leaf(root).join("active-diagnostic.json")).unwrap(),
            bytes
        );
    });
}
#[test]
fn fenced_intent_unbound_full_state_mismatch_keeps_both_inputs_healthy() {
    pair!(i, original, fence, dir, root, f, raw, {
        let before = inventory(&dir);
        let mut other = i.state.clone();
        other.revision += 1;
        let mut check = i.check(f, 100);
        check.state = &other;
        assert!(matches!(
            bind_fixture_fenced_intent(&mut original, fence, &raw, &check),
            Err(FencedIntentError::Binding)
        ));
        assert!(!original.is_refused() && !fence.is_refused());
        assert_eq!(inventory(&dir), before);
        assert!(leaf(root).exists());
        // Also exercise the second mismatch: intent matches but persisted state does not.
        with_fence_for_test(&other, |foreign, _| {
            assert!(matches!(
                bind_fixture_fenced_intent(&mut original, foreign, &raw, &i.check(f, 100)),
                Err(FencedIntentError::Binding)
            ));
            assert!(!original.is_refused() && !foreign.is_refused());
        });
        assert_eq!(inventory(&dir), before);
        bind_fixture_fenced_intent(&mut original, fence, &raw, &i.check(f, 100)).unwrap();
    });
}
#[test]
fn fenced_intent_bound_expiry_signature_state_and_peer_drift_latch_both() {
    for kind in 0..4 {
        pair!(i, original, fence, dir, root, f, raw, {
            {
                let mut read =
                    bind_fixture_fenced_intent(&mut original, fence, &raw, &i.check(f, 100))
                        .unwrap();
                let mut bad = raw.clone();
                let mut check = i.check(f, 101);
                let mut state = i.state.clone();
                state.revision += 1;
                let peer = DiagnosticPeer {
                    uid: 901,
                    gids: vec![902],
                };
                match kind {
                    0 => check.now_unix_seconds = 160,
                    1 => bad[0] = b'x',
                    2 => check.state = &state,
                    _ => check.peer = &peer,
                }
                assert!(matches!(
                    read.revalidate(&bad, &check),
                    Err(FencedIntentError::Intent(_))
                ));
                assert!(read.is_refused());
                let after = inventory(&dir);
                fs::remove_file(leaf(root).join("active-diagnostic.json")).unwrap();
                assert!(matches!(
                    read.revalidate(&raw, &i.check(f, 100)),
                    Err(FencedIntentError::Refused)
                ));
                assert_eq!(inventory(&dir), after);
            }
            assert!(original.is_refused() && fence.is_refused());
            assert_eq!(inventory(&dir).len(), 4);
        });
    }
}
#[test]
fn fenced_intent_state_object_drift_preserves_first_error_and_durable_refusal() {
    pair!(
        i,
        original,
        fence,
        dir,
        root,
        f,
        raw,
        {
            {
                let mut read =
                    bind_fixture_fenced_intent(&mut original, fence, &raw, &i.check(f, 100))
                        .unwrap();
                let path = leaf(root).join("active-diagnostic.json");
                let bytes = fs::read(&path).unwrap();
                fs::rename(&path, path.with_extension("saved")).unwrap();
                fs::write(&path, bytes).unwrap();
                assert!(matches!(
                    read.revalidate(&raw, &i.check(f, 101)),
                    Err(FencedIntentError::Fence(_))
                ));
                let after = inventory(&dir);
                fs::remove_file(&path).unwrap();
                fs::rename(path.with_extension("saved"), &path).unwrap();
                assert!(matches!(
                    read.revalidate(&raw, &i.check(f, 100)),
                    Err(FencedIntentError::Refused)
                ));
                assert_eq!(inventory(&dir), after);
            }
            assert!(original.is_refused() && fence.is_refused());
        },
        {
            assert!(reopen(&dir, f).unwrap().view().unwrap().refused);
        }
    );
}
#[test]
fn fenced_intent_seeded_late_context_failure_latches_originals() {
    let seed = 0x0748_u64;
    eprintln!("seed={seed:#x}");
    pair!(i, original, fence, dir, root, f, raw, {
        let mut read =
            bind_fixture_fenced_intent(&mut original, fence, &raw, &i.check(f, 100)).unwrap();
        let mut checks = 0;
        let error = read
            .reader
            .observe_with(&raw, &i.check(f, 101), |_| {
                checks += 1;
                if checks == 3 {
                    fs::write(
                        leaf(root).join(".diagnostic-requests.pending"),
                        seed.to_le_bytes(),
                    )
                    .unwrap();
                }
            })
            .unwrap_err();
        assert!(matches!(error, FencedIntentError::Fence(_)));
        assert_eq!(checks, 3);
        assert!(read.is_refused());
        assert_eq!(inventory(&dir).len(), 4);
    });
}
#[test]
fn fenced_intent_constructor_storage_failure_revokes_bound_originals() {
    pair!(i, original, fence, dir, root, f, raw, {
        fs::write(root.join("active-campaign"), b"retained conflict").unwrap();
        assert!(matches!(
            bind_fixture_fenced_intent(&mut original, fence, &raw, &i.check(f, 100)),
            Err(FencedIntentError::Fence(OriginalHostFenceError::Conflict))
        ));
        assert!(original.is_refused() && fence.is_refused());
        assert_eq!(inventory(&dir).len(), 4);
        assert_eq!(
            fs::read(root.join("active-campaign")).unwrap(),
            b"retained conflict"
        );
    });
}
#[test]
fn fenced_intent_unsafe_or_advanced_authority_never_appended_on_fence_failure() {
    for advanced in [false, true] {
        pair!(i, original, fence, dir, root, f, raw, {
            let mut read =
                bind_fixture_fenced_intent(&mut original, fence, &raw, &i.check(f, 100)).unwrap();
            if advanced {
                let changed = changed_checked(f, 0);
                match &mut read.reader.original.original.journal {
                    Journal::Fixture(journal) => {
                        journal.refuse(RefusalReason::Operator).unwrap();
                        journal.advance(&changed).unwrap();
                    }
                    _ => unreachable!(),
                }
            } else {
                fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o770)).unwrap();
            }
            fs::write(root.join("active-campaign"), []).unwrap();
            let before = inventory(&dir);
            let error = read.revalidate(&raw, &i.check(f, 101)).unwrap_err();
            assert!(
                matches!(error, FencedIntentError::RefusalUncertain { failure, .. } if matches!(*failure, FencedIntentError::Fence(OriginalHostFenceError::Conflict)))
            );
            assert!(read.is_refused());
            assert_eq!(inventory(&dir), before);
            fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
            assert!(matches!(
                read.revalidate(&raw, &i.check(f, 100)),
                Err(FencedIntentError::Refused)
            ));
            assert_eq!(inventory(&dir), before);
        });
    }
}
#[test]
fn fenced_intent_already_refused_join_latches_counterpart_without_io() {
    pair!(i, original, fence, dir, root, f, raw, {
        original.revalidate(&raw, &i.check(f, 160)).unwrap_err();
        let before = inventory(&dir);
        fs::remove_file(leaf(root).join("active-diagnostic.json")).unwrap();
        assert!(matches!(
            bind_fixture_fenced_intent(&mut original, fence, b"invalid", &i.check(f, u64::MAX)),
            Err(FencedIntentError::Refused)
        ));
        assert!(fence.is_refused());
        assert_eq!(inventory(&dir), before);
    });
}
#[test]
fn fenced_intent_fixed_dispatch_remains_private_nonroot_model() {
    with_mount(1, |mount, _, f| {
        let (_dir, fixture) = journal(f);
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
        with_fence_for_test(&i.state, |fence, _| {
            let mut read =
                bind_fixed_fenced_intent(&mut original, fence, &raw, &i.check(f, 100)).unwrap();
            read.revalidate(&raw, &i.check(f, 101)).unwrap();
            assert!(!read.is_refused());
        });
    });
}
#[test]
fn fenced_intent_independent_concurrent_chains_do_not_share_refusal() {
    std::thread::scope(|scope| {
        let handles: Vec<_> = [false, true]
            .into_iter()
            .map(|fail| {
                scope.spawn(move || {
                    pair!(i, original, fence, dir, root, f, raw, {
                        let mut read = bind_fixture_fenced_intent(
                            &mut original,
                            fence,
                            &raw,
                            &i.check(f, 100),
                        )
                        .unwrap();
                        let result =
                            read.revalidate(&raw, &i.check(f, if fail { 160 } else { 101 }));
                        assert_eq!(result.is_err(), fail);
                        assert_eq!(read.is_refused(), fail);
                        assert!(leaf(root).exists());
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
