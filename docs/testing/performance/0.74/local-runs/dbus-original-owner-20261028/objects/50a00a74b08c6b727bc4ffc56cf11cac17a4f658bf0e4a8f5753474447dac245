//! Original owned process + actual reader context + synthetic account authority only.
use super::super::tests::{ids, Fixture, Owned};
use super::*;
use crate::diagnostic_process::{pin_owned_test_helper, CredentialError, NamespaceError};
use crate::diagnostic_worker_policy::WorkerPolicyError;
use std::fs;
use std::process::{Command, Stdio};

#[test]
fn signed_opening_constructor_and_later_orders_are_exact() {
    let mut gate = Gate::default();
    let mut order = vec![];
    gate.observe(false, true, |s| {
        order.push(s);
        Ok(())
    })
    .unwrap();
    assert_eq!(
        order,
        [
            Step::Account,
            Step::Open,
            Step::Account,
            Step::Credentials,
            Step::Account
        ]
    );
    order.clear();
    gate.observe(false, false, |s| {
        order.push(s);
        Ok(())
    })
    .unwrap();
    assert_eq!(order, [Step::Account, Step::Credentials, Step::Account]);
}
#[test]
fn signed_opening_every_failure_preserves_first_typed_error_and_stops() {
    for opening in [false, true] {
        let len = if opening { 5 } else { 3 };
        for failed in 0..len {
            let mut gate = Gate::default();
            let mut calls = 0;
            let e = gate
                .observe(false, opening, |s| {
                    let here = calls;
                    calls += 1;
                    if here != failed {
                        return Ok(());
                    }
                    match s {
                        Step::Account => Err(ContextFilesError::Refused.into()),
                        Step::Open => {
                            Err(NamespaceCredentialError::Namespace(NamespaceError::Drift).into())
                        }
                        Step::Credentials => Err(PolicyCredentialError::Mapping),
                    }
                })
                .unwrap_err();
            assert_eq!(calls, failed + 1);
            let step = if opening {
                OPENING[failed]
            } else {
                LATER[failed]
            };
            match step {
                Step::Account => assert!(matches!(
                    e,
                    PolicyCredentialError::Account(ContextFilesError::Refused)
                )),
                Step::Open => assert!(matches!(
                    e,
                    PolicyCredentialError::Credentials(NamespaceCredentialError::Namespace(
                        NamespaceError::Drift
                    ))
                )),
                Step::Credentials => assert!(matches!(e, PolicyCredentialError::Mapping)),
            }
            assert!(matches!(
                gate.observe(false, opening, |_| panic!("no refresh")),
                Err(PolicyCredentialError::Refused)
            ));
        }
    }
}
#[test]
fn signed_opening_prior_refusal_never_opens_or_reads() {
    for opening in [false, true] {
        let mut gate = Gate::default();
        assert!(matches!(
            gate.observe(true, opening, |_| panic!("no IO")),
            Err(PolicyCredentialError::Refused)
        ));
        assert!(matches!(
            gate.observe(false, opening, |_| panic!("no restoration")),
            Err(PolicyCredentialError::Refused)
        ));
    }
}
#[test]
fn signed_opening_seeded_failure_positions_remain_sticky() {
    let seed = 0x7572026_u64;
    eprintln!("signed opening mutation seed={seed:#x}; mutations=256");
    let mut rng = seed;
    for _ in 0..256 {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        let opening = rng & 1 == 0;
        let failed = (rng as usize) % if opening { 5 } else { 3 };
        let mut gate = Gate::default();
        let mut calls = 0;
        assert!(matches!(
            gate.observe(false, opening, |_| {
                let here = calls;
                calls += 1;
                if here == failed {
                    Err(PolicyCredentialError::Mapping)
                } else {
                    Ok(())
                }
            }),
            Err(PolicyCredentialError::Mapping)
        ));
        assert_eq!(calls, failed + 1);
        assert!(matches!(
            gate.observe(false, opening, |_| panic!("no refresh")),
            Err(PolicyCredentialError::Refused)
        ));
    }
}
#[test]
fn signed_opening_origin_types_are_neither_send_nor_sync() {
    trait AmbiguousSend<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSend<()> for T {}
    impl<T: ?Sized + Send> AmbiguousSend<u8> for T {}
    let _ = <FixedSignedCredentialRead<'static, 'static, 'static> as AmbiguousSend<_>>::marker;
    let _ = <FixtureSignedCredentialRead<'static, 'static, 'static> as AmbiguousSend<_>>::marker;
    trait AmbiguousSync<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSync<()> for T {}
    impl<T: ?Sized + Sync> AmbiguousSync<u8> for T {}
    let _ = <FixedSignedCredentialRead<'static, 'static, 'static> as AmbiguousSync<_>>::marker;
    let _ = <FixtureSignedCredentialRead<'static, 'static, 'static> as AmbiguousSync<_>>::marker;
}
#[test]
fn signed_opening_owned_roundtrip_and_drop_preserve_original_inputs() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let f = Fixture::new();
    let mut p = f.checked();
    {
        let mut account = f.account(&mut p);
        {
            let mut read = open_signed_fixture_credentials(
                &mut account,
                &process,
                &f.bytes,
                &f.trust,
                &f.host,
            )
            .unwrap();
            for _ in 0..4 {
                read.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
            }
            assert!(!read.is_refused());
        }
        account.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
    }
    assert!(!p.is_refused());
    process.revalidate().unwrap();
}
#[test]
fn signed_opening_refused_account_precedes_dead_process_and_missing_files() {
    let mut child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let f = Fixture::new();
    let mut p = f.checked();
    {
        let mut account = f.account(&mut p);
        account.reader.context.policy.refused = true;
        child.finish();
        fs::remove_file(f.dir.path().join("group")).unwrap();
        assert!(matches!(
            open_signed_fixture_credentials(&mut account, &process, &f.bytes, &f.trust, &f.host),
            Err(PolicyCredentialError::Refused)
        ));
    }
    assert!(p.is_refused());
}
#[test]
fn signed_opening_revocation_precedes_process_and_file_errors() {
    let mut child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let f = Fixture::new();
    let mut p = f.checked();
    let digest = p.policy_sha256().to_owned();
    {
        let mut account = f.account(&mut p);
        let trust = WorkerPolicyTrust::new(
            ed25519_dalek::SigningKey::from_bytes(&[74; 32]).verifying_key(),
            &digest,
            8,
        )
        .unwrap();
        child.finish();
        fs::remove_file(f.dir.path().join("group")).unwrap();
        assert!(matches!(
            open_signed_fixture_credentials(&mut account, &process, &f.bytes, &trust, &f.host),
            Err(PolicyCredentialError::Account(ContextFilesError::Context(
                crate::diagnostic_worker_policy::local_context::WorkerContextError::Policy(
                    WorkerPolicyError::Revoked
                )
            )))
        ));
    }
    assert!(p.is_refused());
}
#[test]
fn signed_opening_dead_original_process_latches_policy_at_construction() {
    let mut child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let f = Fixture::new();
    let mut p = f.checked();
    {
        let mut account = f.account(&mut p);
        child.finish();
        assert!(matches!(
            open_signed_fixture_credentials(&mut account, &process, &f.bytes, &f.trust, &f.host),
            Err(PolicyCredentialError::Credentials(
                NamespaceCredentialError::Namespace(NamespaceError::Process(_))
            ))
        ));
        assert!(account.is_refused());
    }
    assert!(p.is_refused());
}
#[test]
fn signed_opening_unhardened_process_refuses_only_at_projection_read() {
    let child = Owned(
        Command::new("/bin/cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let process = pin_owned_test_helper(child.0.id());
    let f = Fixture::new();
    let mut p = f.checked();
    {
        let mut account = f.account(&mut p);
        assert!(matches!(
            open_signed_fixture_credentials(&mut account, &process, &f.bytes, &f.trust, &f.host),
            Err(PolicyCredentialError::Credentials(
                NamespaceCredentialError::Credentials(CredentialError::Drift)
            ))
        ));
    }
    assert!(p.is_refused());
}
#[test]
fn signed_opening_valid_signed_mapping_mismatch_is_not_caller_numbers() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    for field in 0..3 {
        let (mut uid, mut gid, mut groups) = ids();
        match field {
            0 => uid += 1,
            1 => gid += 1,
            _ => {
                if let Ok(i) = groups.binary_search(&gid) {
                    groups.remove(i);
                } else {
                    groups.push(gid);
                    groups.sort_unstable();
                }
            }
        }
        let f = Fixture::with_ids(uid, gid, groups);
        let mut p = f.checked();
        {
            let mut account = f.account(&mut p);
            assert!(matches!(
                open_signed_fixture_credentials(
                    &mut account,
                    &process,
                    &f.bytes,
                    &f.trust,
                    &f.host
                ),
                Err(PolicyCredentialError::Credentials(
                    NamespaceCredentialError::Credentials(CredentialError::Drift)
                ))
            ));
            assert!(account.is_refused());
        }
        assert!(p.is_refused());
    }
}
#[test]
fn signed_opening_post_open_account_failure_precedes_first_projection_stage() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let f = Fixture::new();
    let mut p = f.checked();
    let mut calls = 0;
    {
        let mut account = f.account(&mut p);
        let result = construct(
            Account::Fixture(&mut account),
            &process,
            &f.bytes,
            &f.trust,
            &f.host,
            |process, policy| {
                calls += 1;
                let read = open_namespace_checked_credentials_unobserved(process, policy)?;
                // Deterministic private seam at the real status-open/outer-context boundary.
                fs::write(f.dir.path().join("group"), b"changed\n").unwrap();
                Ok(read)
            },
        );
        assert!(matches!(result, Err(PolicyCredentialError::Account(_))));
        assert_eq!(calls, 1);
        fs::write(f.dir.path().join("group"), &f.group).unwrap();
        assert!(account.is_refused());
    }
    assert!(p.is_refused());
}
#[test]
fn signed_opening_restoration_latches_owned_credentials_and_policy_after_drop() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let f = Fixture::new();
    let mut p = f.checked();
    {
        let mut account = f.account(&mut p);
        {
            let mut read = open_signed_fixture_credentials(
                &mut account,
                &process,
                &f.bytes,
                &f.trust,
                &f.host,
            )
            .unwrap();
            fs::write(f.dir.path().join("group"), b"changed\n").unwrap();
            assert!(matches!(
                read.revalidate(&f.bytes, &f.trust, &f.host),
                Err(PolicyCredentialError::Account(_))
            ));
            assert!(read.reader.credentials.is_refused());
            fs::write(f.dir.path().join("group"), &f.group).unwrap();
            assert!(matches!(
                read.revalidate(&f.bytes, &f.trust, &f.host),
                Err(PolicyCredentialError::Refused)
            ));
        }
        assert!(account.is_refused());
    }
    assert!(p.is_refused());
    process.revalidate().unwrap();
}
#[test]
fn signed_opening_original_exit_refuses_without_replacement_or_signal() {
    let mut child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let f = Fixture::new();
    let mut p = f.checked();
    {
        let mut account = f.account(&mut p);
        {
            let mut read = open_signed_fixture_credentials(
                &mut account,
                &process,
                &f.bytes,
                &f.trust,
                &f.host,
            )
            .unwrap();
            child.finish();
            assert!(matches!(
                read.revalidate(&f.bytes, &f.trust, &f.host),
                Err(PolicyCredentialError::Credentials(
                    NamespaceCredentialError::Namespace(NamespaceError::Process(_))
                ))
            ));
            assert!(read.reader.credentials.is_refused());
        }
        assert!(account.is_refused());
    }
    assert!(p.is_refused());
}
#[test]
fn signed_opening_independent_concurrent_readers_share_no_refusal() {
    std::thread::scope(|scope| {
        for index in 0..4 {
            scope.spawn(move || {
                let child = Owned::new();
                let process = pin_owned_test_helper(child.0.id());
                let f = Fixture::new();
                let mut p = f.checked();
                {
                    let mut account = f.account(&mut p);
                    {
                        let mut read = open_signed_fixture_credentials(
                            &mut account,
                            &process,
                            &f.bytes,
                            &f.trust,
                            &f.host,
                        )
                        .unwrap();
                        if index == 0 {
                            fs::remove_file(f.dir.path().join("group")).unwrap();
                            assert!(read.revalidate(&f.bytes, &f.trust, &f.host).is_err());
                        } else {
                            for _ in 0..4 {
                                read.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
                            }
                        }
                        assert_eq!(read.is_refused(), index == 0);
                    }
                    assert_eq!(account.is_refused(), index == 0);
                }
                assert_eq!(p.is_refused(), index == 0);
            });
        }
    });
}
