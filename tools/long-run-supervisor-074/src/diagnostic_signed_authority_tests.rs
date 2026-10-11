use super::super::{open_signed_fixture_tasks, tests::with_mount};
use super::*;
use crate::diagnostic_worker_policy::authority_registry::{
    fixture::{initialize_fixture_registry, open_fixture_registry},
    PublicationStage,
};
use crate::diagnostic_worker_policy::local_context::account_files::kernel_binding::tests::Fixture;
use std::{fs, os::unix::fs::PermissionsExt};
use tempfile::TempDir;

#[path = "diagnostic_recovery_authority_tests.rs"]
mod lifecycle_tests;

fn changed_checked(
    f: &Fixture,
    kind: usize,
) -> crate::diagnostic_worker_policy::CheckedWorkerPolicy {
    use crate::diagnostic_worker_policy::{
        verify_worker_policy, SignedWorkerPolicy, SIGNED_WORKER_POLICY_SCHEMA, WORKER_POLICY_DOMAIN,
    };
    use ed25519_dalek::{Signer, SigningKey};
    let mut policy = f.policy.clone();
    let mut host = f.host.clone();
    match kind {
        0 => policy.policy_epoch += 1,
        1 => {
            host.boot_id = "11111111-1111-1111-1111-111111111111".into();
            policy.boot_id = host.boot_id.clone();
        }
        _ => policy.passwd_sha256 = "a".repeat(64),
    }
    let key = SigningKey::from_bytes(&[74; 32]);
    let mut body = crate::canonical_json(&policy).unwrap();
    body.push(b'\n');
    let trust = WorkerPolicyTrust::new(
        key.verifying_key(),
        &crate::sha256_hex(&body),
        policy.policy_epoch,
    )
    .unwrap();
    let mut message = WORKER_POLICY_DOMAIN.to_vec();
    message.push(0);
    message.extend(&body);
    let signature_hex = key
        .sign(&message)
        .to_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let mut envelope = crate::canonical_json(&SignedWorkerPolicy {
        schema_version: SIGNED_WORKER_POLICY_SCHEMA.into(),
        policy,
        signature_hex,
    })
    .unwrap();
    envelope.push(b'\n');
    verify_worker_policy(&envelope, &trust, &host).unwrap()
}

#[test]
fn signed_authority_changed_binding_and_advanced_epoch_are_not_adopted() {
    for kind in 0..3 {
        with_mount(1, |mount, _, f| {
            let changed = changed_checked(f, kind);
            let (dir, mut registry) = journal(f);
            if kind == 0 {
                registry.refuse(RefusalReason::Operator).unwrap();
                registry.advance(&changed).unwrap();
            } else {
                drop(registry);
                let fresh = tempfile::Builder::new()
                    .prefix("hydracache-authority-")
                    .tempdir_in("/tmp")
                    .unwrap();
                fs::set_permissions(fresh.path(), fs::Permissions::from_mode(0o700)).unwrap();
                let mut registry =
                    initialize_fixture_registry(fresh.path(), f.policy.uid, f.policy.gid, &changed)
                        .unwrap();
                let before = inventory(&fresh);
                let mut tasks =
                    open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
                assert!(matches!(
                    bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host),
                    Err(SignedAuthorityError::Authority(AuthorityError::Binding))
                ));
                assert!(tasks.is_refused());
                assert!(registry.is_failed());
                assert_eq!(inventory(&fresh), before);
                return;
            }
            let before = inventory(&dir);
            let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
            assert!(matches!(
                bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host),
                Err(SignedAuthorityError::Authority(AuthorityError::Binding))
            ));
            assert!(tasks.is_refused());
            assert!(registry.is_failed());
            assert_eq!(inventory(&dir), before);
            drop(registry);
            assert!(reopen(&dir, f).is_err());
            let reopened = open_fixture_registry(
                dir.path(),
                f.policy.uid,
                f.policy.gid,
                &changed.trust,
                &changed.context,
            )
            .unwrap();
            assert!(!reopened.view().unwrap().refused);
            // The failed bridge could not safely append to the advanced authority.
            // This active reopen is a deliberate limit, not durable refusal proof.
        });
    }
}

#[test]
fn signed_authority_fixed_fixture_guards_are_neither_send_nor_sync() {
    trait AmbiguousSend<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSend<()> for T {}
    impl<T: ?Sized + Send> AmbiguousSend<u8> for T {}
    let _ = <FixedAuthorityBoundRead<'static, 'static, 'static, 'static, 'static, 'static> as AmbiguousSend<_>>::marker;
    let _ = <FixtureAuthorityBoundRead<'static, 'static, 'static, 'static, 'static, 'static> as AmbiguousSend<_>>::marker;
    trait AmbiguousSync<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSync<()> for T {}
    impl<T: ?Sized + Sync> AmbiguousSync<u8> for T {}
    let _ = <FixedAuthorityBoundRead<'static, 'static, 'static, 'static, 'static, 'static> as AmbiguousSync<_>>::marker;
    let _ = <FixtureAuthorityBoundRead<'static, 'static, 'static, 'static, 'static, 'static> as AmbiguousSync<_>>::marker;
}

#[test]
fn signed_authority_concurrent_independent_chains_share_no_refusal() {
    std::thread::scope(|scope| {
        let handles: Vec<_> = [false, true]
            .into_iter()
            .map(|refuse| {
                scope.spawn(move || {
                    with_mount(1, |mount, _, f| {
                        let (_dir, mut registry) = journal(f);
                        let mut tasks =
                            open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
                        {
                            let mut bound = bind_fixture_authority(
                                &mut tasks,
                                &mut registry,
                                &f.bytes,
                                &f.trust,
                                &f.host,
                            )
                            .unwrap();
                            if refuse {
                                bound.refuse(RefusalReason::Operator).unwrap();
                            } else {
                                bound.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
                            }
                            assert_eq!(bound.is_refused(), refuse);
                        }
                        assert_eq!(tasks.is_refused(), refuse);
                        assert_eq!(registry.view().unwrap().refused, refuse);
                    });
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
}

fn journal(f: &Fixture) -> (TempDir, FixtureAuthorityRegistry) {
    let dir = tempfile::Builder::new()
        .prefix("hydracache-authority-")
        .tempdir_in("/tmp")
        .unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let registry =
        initialize_fixture_registry(dir.path(), f.policy.uid, f.policy.gid, &f.checked()).unwrap();
    (dir, registry)
}
fn reopen(dir: &TempDir, f: &Fixture) -> Result<FixtureAuthorityRegistry, AuthorityError> {
    open_fixture_registry(dir.path(), f.policy.uid, f.policy.gid, &f.trust, &f.host)
}
fn inventory(dir: &TempDir) -> Vec<(String, Vec<u8>)> {
    let mut entries: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            (
                path.file_name().unwrap().to_str().unwrap().to_owned(),
                fs::read(path).unwrap(),
            )
        })
        .collect();
    entries.sort();
    entries
}

#[test]
fn signed_authority_bracket_stops_at_first_error_seeded() {
    let mut seed = 0x7632026_u64;
    for _ in 0..256 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let fail = (seed % 4) as usize;
        let mut calls = Vec::new();
        let result = bracket(|step| {
            calls.push(step);
            if calls.len() - 1 == fail {
                Err(SignedAuthorityError::Inactive)
            } else {
                Ok(())
            }
        });
        assert_eq!(calls, BRACKET[..(fail + 1).min(3)]);
        assert_eq!(result.is_ok(), fail == 3);
    }
}

#[test]
fn signed_authority_account_restore_cannot_restore_durable_guard() {
    with_mount(1, |mount, _, f| {
        let (dir, mut registry) = journal(f);
        let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        {
            let mut bound =
                bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host)
                    .unwrap();
            fs::write(f.dir.path().join("group"), b"changed-account").unwrap();
            assert!(matches!(
                bound.revalidate(&f.bytes, &f.trust, &f.host),
                Err(SignedAuthorityError::Signed(_))
            ));
            fs::write(f.dir.path().join("group"), &f.group).unwrap();
            assert!(matches!(
                bound.revalidate(&f.bytes, &f.trust, &f.host),
                Err(SignedAuthorityError::Refused)
            ));
        }
        assert!(tasks.is_refused());
        drop(registry);
        assert!(reopen(&dir, f).unwrap().view().unwrap().refused);
    });
}

#[test]
fn signed_authority_prior_guard_failure_with_unsafe_storage_is_uncertain_not_repaired() {
    with_mount(1, |mount, _, f| {
        let (dir, mut registry) = journal(f);
        let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        assert!(tasks.revalidate(b"bad", &f.trust, &f.host).is_err());
        fs::write(dir.path().join("foreign"), b"unsafe").unwrap();
        let before = inventory(&dir);
        match bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host) {
            Err(SignedAuthorityError::RefusalUncertain {
                failure,
                persistence: AuthorityError::Invalid,
            }) => {
                assert!(matches!(
                    *failure,
                    SignedAuthorityError::Signed(SignedTasksError::Refused)
                ));
            }
            _ => panic!("must preserve first refusal plus unsafe storage failure"),
        }
        assert!(tasks.is_refused());
        assert!(registry.is_failed());
        assert_eq!(inventory(&dir), before);
    });
}

#[test]
fn signed_authority_fixed_dispatch_unsafe_storage_latches_without_append() {
    with_mount(1, |mount, _, f| {
        let (dir, journal) = journal(f);
        let mut registry =
            crate::diagnostic_worker_policy::authority_registry::fixed::modeled_registry_for_test(
                journal,
            );
        let fixture = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        let mut tasks = FixedSignedTasksRead {
            reader: fixture.reader,
        };
        {
            let mut bound =
                bind_fixed_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host)
                    .unwrap();
            fs::write(dir.path().join("foreign"), b"unsafe").unwrap();
            let before = inventory(&dir);
            assert!(matches!(
                bound.revalidate(&f.bytes, &f.trust, &f.host),
                Err(SignedAuthorityError::Authority(_))
            ));
            assert!(bound.is_refused());
            assert_eq!(inventory(&dir), before);
        }
        assert!(tasks.is_refused());
        assert!(registry.is_failed());
    });
}
#[test]
fn signed_authority_healthy_drop_preserves_both_originals() {
    for threads in [1, 2] {
        with_mount(threads, |mount, _, f| {
            let (dir, mut registry) = journal(f);
            let before = inventory(&dir);
            let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
            {
                let mut bound =
                    bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host)
                        .unwrap();
                bound.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
                assert!(!bound.is_refused());
            }
            tasks.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
            assert_eq!(inventory(&dir), before);
            assert!(!registry.inspect(&f.trust, &f.host).unwrap().refused);
        });
    }
}
#[test]
fn signed_authority_explicit_refusal_reopens_and_never_retries() {
    with_mount(1, |mount, _, f| {
        let (dir, mut registry) = journal(f);
        let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        {
            let mut bound =
                bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host)
                    .unwrap();
            bound.refuse(RefusalReason::Operator).unwrap();
            let after = inventory(&dir);
            assert_eq!(after.len(), 4);
            assert!(matches!(
                bound.refuse(RefusalReason::Operator),
                Err(SignedAuthorityError::Refused)
            ));
            assert!(matches!(
                bound.revalidate(&[], &f.trust, &f.host),
                Err(SignedAuthorityError::Refused)
            ));
            assert_eq!(inventory(&dir), after);
        }
        assert!(tasks.is_refused());
        drop(registry);
        let mut reopened = reopen(&dir, f).unwrap();
        assert!(reopened.inspect(&f.trust, &f.host).unwrap().refused);
        assert!(reopened.advance(&f.checked()).is_err());
    });
}
#[test]
fn signed_authority_signed_drift_persists_under_original_not_caller_pins() {
    for kind in 0..3 {
        with_mount(1, |mount, _, f| {
            let (dir, mut registry) = journal(f);
            let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
            {
                let mut bound =
                    bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host)
                        .unwrap();
                let mut changed_host = f.host.clone();
                changed_host.boot_id = "0".repeat(32);
                let changed_trust =
                    WorkerPolicyTrust::new(f.trust.key, &"a".repeat(64), 8).unwrap();
                let (bytes, trust, host) = match kind {
                    0 => (&b"changed"[..], &f.trust, &f.host),
                    1 => (&f.bytes[..], &changed_trust, &f.host),
                    _ => (&f.bytes[..], &f.trust, &changed_host),
                };
                assert!(matches!(
                    bound.revalidate(bytes, trust, host),
                    Err(SignedAuthorityError::Signed(_))
                ));
                assert!(bound.is_refused());
                assert!(matches!(
                    bound.revalidate(&f.bytes, &f.trust, &f.host),
                    Err(SignedAuthorityError::Refused)
                ));
            }
            assert!(tasks.is_refused());
            drop(registry);
            assert!(reopen(&dir, f).unwrap().view().unwrap().refused);
        });
    }
}
#[test]
fn signed_authority_constructor_failure_and_already_refused_guard_persist() {
    for prior in [false, true] {
        with_mount(1, |mount, _, f| {
            let (dir, mut registry) = journal(f);
            let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
            if prior {
                assert!(tasks.revalidate(b"bad", &f.trust, &f.host).is_err());
            }
            assert!(matches!(
                bind_fixture_authority(&mut tasks, &mut registry, b"bad", &f.trust, &f.host),
                Err(SignedAuthorityError::Signed(_))
            ));
            assert!(tasks.is_refused());
            drop(registry);
            assert!(reopen(&dir, f).unwrap().view().unwrap().refused);
        });
    }
}
#[test]
fn signed_authority_foreign_journal_binding_is_never_written() {
    with_mount(1, |mount, _, f| {
        let (dir, mut registry) = journal(f);
        let original = inventory(&dir);
        let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        // A caller cannot replace the original host binding; the journal is poisoned by
        // an independently attempted mismatching inspection before composition.
        let mut changed = f.host.clone();
        changed.boot_id = "0".repeat(32);
        assert!(registry.inspect(&f.trust, &changed).is_err());
        assert!(matches!(
            bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &changed),
            Err(SignedAuthorityError::Authority(AuthorityError::Refused))
        ));
        assert!(tasks.is_refused());
        assert_eq!(inventory(&dir), original);
    });
}
#[test]
fn signed_authority_durable_inactive_stops_before_signed_without_append() {
    with_mount(1, |mount, _, f| {
        let (dir, mut registry) = journal(f);
        registry.refuse(RefusalReason::Operator).unwrap();
        let original = inventory(&dir);
        let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        assert!(matches!(
            bind_fixture_authority(&mut tasks, &mut registry, b"bad", &f.trust, &f.host),
            Err(SignedAuthorityError::Inactive)
        ));
        assert!(tasks.is_refused());
        assert!(!registry.is_failed());
        assert_eq!(inventory(&dir), original);
    });
}
#[test]
fn signed_authority_task_churn_persists_refusal() {
    with_mount(1, |mount, owned, f| {
        let (dir, mut registry) = journal(f);
        let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        {
            let mut bound =
                bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host)
                    .unwrap();
            owned.command("add");
            assert!(matches!(
                bound.revalidate(&f.bytes, &f.trust, &f.host),
                Err(SignedAuthorityError::Signed(_))
            ));
            owned.command("remove");
            assert!(bound.is_refused());
        }
        drop(registry);
        assert!(reopen(&dir, f).unwrap().view().unwrap().refused);
    });
}
#[test]
fn signed_authority_unsafe_storage_refuses_without_append_or_repair() {
    for operator in [false, true] {
        with_mount(1, |mount, _, f| {
            let (dir, mut registry) = journal(f);
            let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
            {
                let mut bound =
                    bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host)
                        .unwrap();
                fs::write(dir.path().join("foreign"), b"unsafe").unwrap();
                let before = inventory(&dir);
                if operator {
                    assert!(matches!(
                        bound.refuse(RefusalReason::Operator),
                        Err(SignedAuthorityError::Publication(_))
                    ));
                } else {
                    assert!(matches!(
                        bound.revalidate(&f.bytes, &f.trust, &f.host),
                        Err(SignedAuthorityError::Authority(_))
                    ));
                }
                assert!(bound.is_refused());
                assert!(matches!(
                    bound.revalidate(&f.bytes, &f.trust, &f.host),
                    Err(SignedAuthorityError::Refused)
                ));
                assert_eq!(inventory(&dir), before);
            }
            assert!(tasks.is_refused());
            assert!(registry.is_failed());
        });
    }
}
#[test]
fn signed_authority_partial_publications_preserve_first_error_and_refuse_reopen() {
    for stage in [
        PublicationStage::RecordCreated,
        PublicationStage::RecordSynced,
        PublicationStage::MarkerCreated,
        PublicationStage::DurableCommit,
    ] {
        with_mount(1, |mount, _, f| {
            let (dir, mut registry) = journal(f);
            let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
            {
                let mut bound =
                    bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host)
                        .unwrap();
                let error = bound.reader.reject_with(
                    SignedAuthorityError::Signed(SignedTasksError::Refused),
                    |journal, reason| match journal {
                        Journal::Fixture(read) => read.refuse_at_for_test(reason, stage),
                        Journal::Fixed(_) => panic!("fixture only"),
                    },
                );
                match error {
                    SignedAuthorityError::RefusalUncertain {
                        failure,
                        persistence: AuthorityError::Injected(at),
                    } => {
                        assert!(matches!(
                            *failure,
                            SignedAuthorityError::Signed(SignedTasksError::Refused)
                        ));
                        assert_eq!(at, stage);
                    }
                    other => panic!("unexpected {other:?}"),
                }
                let after = inventory(&dir);
                assert!(matches!(
                    bound.refuse(RefusalReason::Storage),
                    Err(SignedAuthorityError::Refused)
                ));
                assert!(matches!(
                    bound.revalidate(&f.bytes, &f.trust, &f.host),
                    Err(SignedAuthorityError::Refused)
                ));
                assert_eq!(inventory(&dir), after);
            }
            assert!(tasks.is_refused());
            assert!(registry.is_failed());
            drop(registry);
            if stage == PublicationStage::DurableCommit {
                assert!(reopen(&dir, f).unwrap().view().unwrap().refused);
            } else {
                assert!(reopen(&dir, f).is_err());
            }
        });
    }
}
#[test]
fn signed_authority_fixed_dispatch_model_is_not_root_enrollment() {
    with_mount(1, |mount, _, f| {
        let (dir, journal) = journal(f);
        let mut registry =
            crate::diagnostic_worker_policy::authority_registry::fixed::modeled_registry_for_test(
                journal,
            );
        // Test-only typed dispatch, not a public fixture-to-fixed conversion.
        let fixture = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        let mut tasks = FixedSignedTasksRead {
            reader: fixture.reader,
        };
        {
            let mut bound =
                bind_fixed_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host)
                    .unwrap();
            bound.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
            bound.refuse(RefusalReason::Operator).unwrap();
            assert!(bound.is_refused());
        }
        assert!(tasks.is_refused());
        assert!(registry.inspect(&f.trust, &f.host).unwrap().refused);
        drop(registry);
        assert!(reopen(&dir, f).unwrap().view().unwrap().refused);
    });
}
