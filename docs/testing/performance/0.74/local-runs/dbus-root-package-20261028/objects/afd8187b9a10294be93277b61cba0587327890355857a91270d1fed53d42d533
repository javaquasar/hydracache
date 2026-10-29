//! Fixture authority only; the original owned helper is never production enrollment.
use super::super::super::tests::{Fixture, Owned};
use super::super::open_signed_fixture_credentials;
use super::*;
use crate::diagnostic_process::pin_owned_test_helper;
use std::fs::{self, File};

#[test]
fn signed_mount_api_requires_original_signed_reader() {
    let _ = open_signed_fixture_mount;
    let _ = open_signed_fixed_mount;
}

#[test]
fn signed_mount_context_restoration_refuses_mount_credentials_and_policy() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let f = Fixture::new();
    let mut p = f.checked();
    {
        let mut account = f.account(&mut p);
        {
            let mut signed = open_signed_fixture_credentials(
                &mut account,
                &process,
                &f.bytes,
                &f.trust,
                &f.host,
            )
            .unwrap();
            let mut read =
                open_signed_fixture_mount(&mut signed, &f.bytes, &f.trust, &f.host).unwrap();
            fs::write(f.dir.path().join("group"), b"changed\n").unwrap();
            assert!(matches!(
                read.revalidate(&f.bytes, &f.trust, &f.host),
                Err(SignedMountError::Signed(PolicyCredentialError::Account(_)))
            ));
            assert!(read.reader.mount.is_refused());
            assert!(read.reader.original.reader().credentials.is_refused());
            assert!(read
                .reader
                .original
                .reader()
                .account
                .reader()
                .context
                .policy
                .is_refused());
            fs::write(f.dir.path().join("group"), &f.group).unwrap();
            assert!(matches!(
                read.revalidate(&f.bytes, &f.trust, &f.host),
                Err(SignedMountError::Refused)
            ));
        }
        assert!(account.is_refused());
    }
    assert!(p.is_refused());
    process.revalidate().unwrap();
}

#[test]
fn signed_mount_orders_first_errors_and_seeded_refusal_are_exact() {
    let seed = 0x7592026_u64;
    eprintln!("signed mount mutation seed={seed:#x}; mutations=256");
    let mut rng = seed;
    for opening in [false, true] {
        let expected = if opening { &OPENING[..] } else { &LATER[..] };
        let mut gate = Gate::default();
        let mut order = vec![];
        gate.observe(false, opening, |step| {
            order.push(step);
            Ok(())
        })
        .unwrap();
        assert_eq!(order, expected);
        for (failed, expected_step) in expected.iter().enumerate() {
            let mut gate = Gate::default();
            let mut calls = 0;
            let error = gate
                .observe(false, opening, |step| {
                    let here = calls;
                    calls += 1;
                    if here != failed {
                        return Ok(());
                    }
                    match step {
                        Step::Signed => Err(PolicyCredentialError::Refused.into()),
                        Step::Open => Err(MountNamespaceError::Invalid.into()),
                        Step::Mount => Err(SignedMountError::Mapping),
                    }
                })
                .unwrap_err();
            assert_eq!(calls, failed + 1);
            match expected_step {
                Step::Signed => assert!(matches!(
                    error,
                    SignedMountError::Signed(PolicyCredentialError::Refused)
                )),
                Step::Open => assert!(matches!(
                    error,
                    SignedMountError::Mount(MountNamespaceError::Invalid)
                )),
                Step::Mount => assert!(matches!(error, SignedMountError::Mapping)),
            }
            assert!(matches!(
                gate.observe(false, opening, |_| panic!("no retry")),
                Err(SignedMountError::Refused)
            ));
        }
        let mut gate = Gate::default();
        assert!(matches!(
            gate.observe(true, opening, |_| panic!("no IO")),
            Err(SignedMountError::Refused)
        ));
    }
    for _ in 0..256 {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        let opening = rng & 1 == 0;
        let failed = rng as usize % if opening { 5 } else { 3 };
        let mut calls = 0;
        let mut gate = Gate::default();
        assert!(matches!(
            gate.observe(false, opening, |_| {
                let here = calls;
                calls += 1;
                if here == failed {
                    Err(SignedMountError::Mapping)
                } else {
                    Ok(())
                }
            }),
            Err(SignedMountError::Mapping)
        ));
        assert_eq!(calls, failed + 1);
        assert!(matches!(
            gate.observe(false, opening, |_| panic!("no reset")),
            Err(SignedMountError::Refused)
        ));
    }
}

#[test]
fn signed_mount_fixed_fixture_types_are_not_send_or_sync() {
    trait AmbiguousSend<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSend<()> for T {}
    impl<T: ?Sized + Send> AmbiguousSend<u8> for T {}
    let _ = <FixedSignedMountRead<'static, 'static, 'static, 'static> as AmbiguousSend<_>>::marker;
    let _ =
        <FixtureSignedMountRead<'static, 'static, 'static, 'static> as AmbiguousSend<_>>::marker;
    trait AmbiguousSync<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSync<()> for T {}
    impl<T: ?Sized + Sync> AmbiguousSync<u8> for T {}
    let _ = <FixedSignedMountRead<'static, 'static, 'static, 'static> as AmbiguousSync<_>>::marker;
    let _ =
        <FixtureSignedMountRead<'static, 'static, 'static, 'static> as AmbiguousSync<_>>::marker;
}

#[test]
fn signed_mount_owned_roundtrip_and_healthy_drop_keep_original_usable() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let f = Fixture::new();
    let mut p = f.checked();
    {
        let mut account = f.account(&mut p);
        {
            let mut signed = open_signed_fixture_credentials(
                &mut account,
                &process,
                &f.bytes,
                &f.trust,
                &f.host,
            )
            .unwrap();
            {
                let mut read =
                    open_signed_fixture_mount(&mut signed, &f.bytes, &f.trust, &f.host).unwrap();
                for _ in 0..4 {
                    read.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
                }
                assert!(!read.is_refused());
            }
            signed.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
        }
        account.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
    }
    assert!(!p.is_refused());
    process.revalidate().unwrap();
}

#[test]
fn signed_mount_prior_refusal_never_opens_and_latches_policy() {
    for component in 0..3 {
        let child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        let f = Fixture::new();
        let mut p = f.checked();
        {
            let mut account = f.account(&mut p);
            {
                let mut signed = open_signed_fixture_credentials(
                    &mut account,
                    &process,
                    &f.bytes,
                    &f.trust,
                    &f.host,
                )
                .unwrap();
                if component == 0 {
                    signed.reader.credentials.refuse();
                } else {
                    signed
                        .reader
                        .credentials
                        .refuse_mount_component_for_test(component == 1);
                }
                fs::remove_file(f.dir.path().join("group")).unwrap();
                assert!(matches!(
                    construct(
                        Original::Fixture(&mut signed),
                        &f.bytes,
                        &f.trust,
                        &f.host,
                        |_| panic!("no open")
                    ),
                    Err(SignedMountError::Refused)
                ));
                assert!(signed.reader.account.reader().is_refused());
            }
            assert!(account.is_refused());
        }
        assert!(p.is_refused());
    }
}

#[test]
fn signed_mount_post_open_signed_error_precedes_invalid_mount_and_is_sticky() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let f = Fixture::new();
    let mut p = f.checked();
    {
        let mut account = f.account(&mut p);
        {
            let mut signed = open_signed_fixture_credentials(
                &mut account,
                &process,
                &f.bytes,
                &f.trust,
                &f.host,
            )
            .unwrap();
            let result = construct(
                Original::Fixture(&mut signed),
                &f.bytes,
                &f.trust,
                &f.host,
                |credentials| {
                    let mut mount = credentials.pin_original_mount()?;
                    mount.replace_worker_for_test(File::open("/dev/null").unwrap());
                    fs::write(f.dir.path().join("group"), b"changed\n").unwrap();
                    Ok(mount)
                },
            );
            assert!(matches!(
                result,
                Err(SignedMountError::Signed(PolicyCredentialError::Account(_)))
            ));
            fs::write(f.dir.path().join("group"), &f.group).unwrap();
            assert!(signed.is_refused());
            assert!(signed.reader.credentials.is_refused());
            assert!(matches!(
                signed.revalidate(&f.bytes, &f.trust, &f.host),
                Err(PolicyCredentialError::Refused)
            ));
        }
        assert!(account.is_refused());
    }
    assert!(p.is_refused());
}

#[test]
fn signed_mount_open_failure_and_original_exit_never_adopt_replacement() {
    for exit in [false, true] {
        let mut child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        let f = Fixture::new();
        let mut p = f.checked();
        {
            let mut account = f.account(&mut p);
            {
                let mut signed = open_signed_fixture_credentials(
                    &mut account,
                    &process,
                    &f.bytes,
                    &f.trust,
                    &f.host,
                )
                .unwrap();
                let result = construct(
                    Original::Fixture(&mut signed),
                    &f.bytes,
                    &f.trust,
                    &f.host,
                    |credentials| {
                        if exit {
                            child.finish();
                            credentials.pin_original_mount()
                        } else {
                            Err(MountNamespaceError::Invalid)
                        }
                    },
                );
                if exit {
                    assert!(matches!(
                        result,
                        Err(SignedMountError::Mount(MountNamespaceError::Process(_)))
                    ));
                } else {
                    assert!(matches!(
                        result,
                        Err(SignedMountError::Mount(MountNamespaceError::Invalid))
                    ));
                }
                assert!(signed.is_refused());
                assert!(signed.reader.credentials.is_refused());
            }
            assert!(account.is_refused());
        }
        assert!(p.is_refused());
    }
}

#[test]
fn signed_mount_substitution_restoration_and_component_refusal_latch_all() {
    for component in 0..3 {
        let child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        let f = Fixture::new();
        let mut p = f.checked();
        {
            let mut account = f.account(&mut p);
            {
                let mut signed = open_signed_fixture_credentials(
                    &mut account,
                    &process,
                    &f.bytes,
                    &f.trust,
                    &f.host,
                )
                .unwrap();
                {
                    let mut read =
                        open_signed_fixture_mount(&mut signed, &f.bytes, &f.trust, &f.host)
                            .unwrap();
                    let restored = match component {
                        0 => Some(
                            read.reader
                                .mount
                                .replace_worker_for_test(File::open("/dev/null").unwrap()),
                        ),
                        1 => {
                            read.reader.mount.refuse();
                            None
                        }
                        _ => {
                            read.reader.original.reader_mut().credentials.refuse();
                            None
                        }
                    };
                    let result = read.revalidate(&f.bytes, &f.trust, &f.host);
                    if component == 0 {
                        assert!(matches!(
                            result,
                            Err(SignedMountError::Mount(MountNamespaceError::Invalid))
                        ));
                    } else {
                        assert!(matches!(result, Err(SignedMountError::Refused)));
                    }
                    assert!(read.reader.mount.is_refused());
                    assert!(read.reader.original.reader().credentials.is_refused());
                    if let Some(file) = restored {
                        read.reader.mount.replace_worker_for_test(file);
                    }
                    assert!(matches!(
                        read.revalidate(&f.bytes, &f.trust, &f.host),
                        Err(SignedMountError::Refused)
                    ));
                }
                assert!(signed.is_refused());
            }
            assert!(account.is_refused());
        }
        assert!(p.is_refused());
        process.revalidate().unwrap();
    }
}

#[test]
fn signed_mount_revocation_precedes_dead_process_and_bad_mount() {
    let mut child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let f = Fixture::new();
    let mut p = f.checked();
    let revoked = WorkerPolicyTrust::new(
        ed25519_dalek::SigningKey::from_bytes(&[74; 32]).verifying_key(),
        p.policy_sha256(),
        8,
    )
    .unwrap();
    {
        let mut account = f.account(&mut p);
        {
            let mut signed = open_signed_fixture_credentials(
                &mut account,
                &process,
                &f.bytes,
                &f.trust,
                &f.host,
            )
            .unwrap();
            let mut read =
                open_signed_fixture_mount(&mut signed, &f.bytes, &f.trust, &f.host).unwrap();
            child.finish();
            read.reader
                .mount
                .replace_worker_for_test(File::open("/dev/null").unwrap());
            assert!(matches!(
                read.revalidate(&f.bytes, &revoked, &f.host),
                Err(SignedMountError::Signed(PolicyCredentialError::Account(
                    super::super::ContextFilesError::Context(
                        crate::diagnostic_worker_policy::local_context::WorkerContextError::Policy(
                            crate::diagnostic_worker_policy::WorkerPolicyError::Revoked
                        )
                    )
                )))
            ));
            assert!(read.reader.mount.is_refused());
        }
        assert!(account.is_refused());
    }
    assert!(p.is_refused());
}

#[test]
fn signed_mount_original_exit_latches_owned_mount_after_drop() {
    let mut child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let f = Fixture::new();
    let mut p = f.checked();
    {
        let mut account = f.account(&mut p);
        {
            let mut signed = open_signed_fixture_credentials(
                &mut account,
                &process,
                &f.bytes,
                &f.trust,
                &f.host,
            )
            .unwrap();
            {
                let mut read =
                    open_signed_fixture_mount(&mut signed, &f.bytes, &f.trust, &f.host).unwrap();
                child.finish();
                assert!(matches!(
                    read.revalidate(&f.bytes, &f.trust, &f.host),
                    Err(SignedMountError::Signed(
                        PolicyCredentialError::Credentials(_)
                    ))
                ));
                assert!(read.reader.mount.is_refused());
            }
            assert!(signed.is_refused());
        }
        assert!(account.is_refused());
    }
    assert!(p.is_refused());
}

#[test]
fn signed_mount_policy_pin_drift_is_not_adopted() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let f = Fixture::new();
    let mut mount = crate::diagnostic_process::pin_same_mount_namespace(&process).unwrap();
    assert!(mount.matches_worker_policy(&f.policy));
    for field in 0..2 {
        let mut changed = f.policy.clone();
        if field == 0 {
            changed.mount_namespace.device += 1;
        } else {
            changed.mount_namespace.inode += 1;
        }
        assert!(!mount.matches_worker_policy(&changed));
    }
    mount.refuse();
    assert!(!mount.matches_worker_policy(&f.policy));
}

#[test]
fn signed_mount_independent_concurrent_readers_share_no_latch() {
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
                        let mut signed = open_signed_fixture_credentials(
                            &mut account,
                            &process,
                            &f.bytes,
                            &f.trust,
                            &f.host,
                        )
                        .unwrap();
                        let mut read =
                            open_signed_fixture_mount(&mut signed, &f.bytes, &f.trust, &f.host)
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
