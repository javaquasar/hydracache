//! Owned nonroot helpers and fixture signatures only; no host enrollment.
use super::super::super::super::tests::Fixture;
use super::super::super::open_signed_fixture_credentials;
use super::*;
use crate::diagnostic_process::{pin_owned_test_helper, OwnedTasksHelper};
use std::fs;

pub(super) fn with_mount(
    threads: usize,
    run: impl FnOnce(&mut FixtureSignedMountRead<'_, '_, '_, '_>, &mut OwnedTasksHelper, &Fixture),
) {
    let mut owned = OwnedTasksHelper::new(threads);
    let process = pin_owned_test_helper(owned.child.id());
    let f = Fixture::new();
    let mut policy = f.checked();
    let refused;
    {
        let mut account = f.account(&mut policy);
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
                let mut mount = super::super::open_signed_fixture_mount(
                    &mut signed,
                    &f.bytes,
                    &f.trust,
                    &f.host,
                )
                .unwrap();
                run(&mut mount, &mut owned, &f);
                refused = mount.is_refused();
                if refused {
                    assert!(mount.reader.mount.is_refused());
                    assert!(matches!(
                        mount.revalidate(&f.bytes, &f.trust, &f.host),
                        Err(SignedMountError::Refused)
                    ));
                } else {
                    mount.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
                }
            }
            assert_eq!(signed.is_refused(), refused);
            assert_eq!(signed.reader.credentials.is_refused(), refused);
        }
        assert_eq!(account.is_refused(), refused);
    }
    assert_eq!(policy.is_refused(), refused);
}
fn assert_refused(read: &mut FixtureSignedTasksRead<'_, '_, '_, '_, '_>, f: &Fixture) {
    assert!(read.is_refused());
    assert!(read.reader.tasks.is_refused());
    assert!(read.reader.original.reader().mount.is_refused());
    assert!(read
        .reader
        .original
        .reader()
        .original
        .reader()
        .credentials
        .is_refused());
    assert!(read
        .reader
        .original
        .reader()
        .original
        .reader()
        .account
        .reader()
        .context
        .policy
        .is_refused());
    assert!(matches!(
        read.revalidate(&f.bytes, &f.trust, &f.host),
        Err(SignedTasksError::Refused)
    ));
}

#[test]
fn signed_tasks_exact_orders_seeded_first_errors_and_no_io_after_refusal() {
    let mut rng = 0x7632026_u64;
    eprintln!("signed tasks seed={rng:#x}; schedules=256");
    for opening in [false, true] {
        let expected = if opening { &OPENING[..] } else { &LATER[..] };
        let mut gate = Gate::default();
        let mut observed = vec![];
        gate.observe(false, opening, |step| {
            observed.push(step);
            Ok(())
        })
        .unwrap();
        assert_eq!(observed, expected);
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
                        Step::Signed => Err(SignedMountError::Mapping.into()),
                        Step::Open => Err(TaskError::Budget.into()),
                        Step::Tasks => Err(SignedTasksError::Mapping),
                    }
                })
                .unwrap_err();
            assert_eq!(calls, failed + 1);
            match expected_step {
                Step::Signed => assert!(matches!(
                    error,
                    SignedTasksError::Signed(SignedMountError::Mapping)
                )),
                Step::Open => assert!(matches!(error, SignedTasksError::Tasks(TaskError::Budget))),
                Step::Tasks => assert!(matches!(error, SignedTasksError::Mapping)),
            }
            assert!(matches!(
                gate.observe(false, opening, |_| panic!("no IO")),
                Err(SignedTasksError::Refused)
            ));
        }
        assert!(matches!(
            Gate::default().observe(true, opening, |_| panic!("no account IO")),
            Err(SignedTasksError::Refused)
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
                    Err(TaskError::Drift.into())
                } else {
                    Ok(())
                }
            }),
            Err(SignedTasksError::Tasks(TaskError::Drift))
        ));
        assert_eq!(calls, failed + 1);
        assert!(matches!(
            gate.observe(false, opening, |_| panic!("no reset")),
            Err(SignedTasksError::Refused)
        ));
    }
}

#[test]
fn signed_tasks_fixed_fixture_guards_are_neither_send_nor_sync() {
    trait AmbiguousSend<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSend<()> for T {}
    impl<T: ?Sized + Send> AmbiguousSend<u8> for T {}
    let _ = <FixedSignedTasksRead<'static, 'static, 'static, 'static, 'static> as AmbiguousSend<
        _,
    >>::marker;
    let _ = <FixtureSignedTasksRead<'static, 'static, 'static, 'static, 'static> as AmbiguousSend<_>>::marker;
    trait AmbiguousSync<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSync<()> for T {}
    impl<T: ?Sized + Sync> AmbiguousSync<u8> for T {}
    let _ = <FixedSignedTasksRead<'static, 'static, 'static, 'static, 'static> as AmbiguousSync<
        _,
    >>::marker;
    let _ = <FixtureSignedTasksRead<'static, 'static, 'static, 'static, 'static> as AmbiguousSync<_>>::marker;
}

#[test]
fn signed_tasks_real_single_multi_thread_healthy_drop_preserves_original_chain() {
    for threads in [0, 2] {
        with_mount(threads, |mount, _, f| {
            let mut read = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
            for _ in 0..4 {
                read.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
            }
            assert!(!read.is_refused());
        });
    }
}

#[test]
fn signed_tasks_real_add_remove_and_restoration_refuse_whole_original_chain() {
    for add in [true, false] {
        with_mount(1, |mount, owned, f| {
            let mut read = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
            owned.command(if add { "add" } else { "remove" });
            assert!(matches!(
                read.revalidate(&f.bytes, &f.trust, &f.host),
                Err(SignedTasksError::Tasks(TaskError::Drift))
            ));
            owned.command(if add { "remove" } else { "add" });
            assert_refused(&mut read, f);
        });
    }
}

#[test]
fn signed_tasks_owned_exit_never_adopts_replacement() {
    with_mount(1, |mount, owned, f| {
        let mut read = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        owned.child.kill().unwrap();
        owned.child.wait().unwrap();
        let replacement = OwnedTasksHelper::new(1);
        assert_ne!(replacement.child.id(), owned.child.id());
        assert!(matches!(
            read.revalidate(&f.bytes, &f.trust, &f.host),
            Err(SignedTasksError::Signed(_))
        ));
        assert_refused(&mut read, f);
    });
}

#[test]
fn signed_tasks_open_failure_and_post_open_signed_failure_latch_inputs() {
    for changed in [false, true] {
        with_mount(1, |mount, _, f| {
            let error = construct(
                Original::Fixture(mount),
                &f.bytes,
                &f.trust,
                &f.host,
                |credentials, policy| {
                    if !changed {
                        return Err(TaskError::Budget);
                    }
                    let read = credentials.pin_original_tasks(policy)?;
                    fs::write(f.dir.path().join("group"), b"changed\n").unwrap();
                    Ok(read)
                },
            )
            .err()
            .unwrap();
            if changed {
                assert!(matches!(
                    error,
                    SignedTasksError::Signed(SignedMountError::Signed(_))
                ));
                fs::write(f.dir.path().join("group"), &f.group).unwrap();
            } else {
                assert!(matches!(error, SignedTasksError::Tasks(TaskError::Budget)));
            }
            assert!(mount.is_refused());
        });
    }
}

#[test]
fn signed_tasks_mapping_checks_both_namespace_pins_and_exact_groups() {
    with_mount(1, |mount, _, f| {
        let mut changed = f.policy.clone();
        changed.uid += 1;
        assert!(matches!(
            mount
                .reader
                .original
                .reader()
                .credentials
                .pin_original_tasks(&changed),
            Err(TaskError::Drift)
        ));
    });
    for case in 0..4 {
        with_mount(1, |mount, _, f| {
            let result = construct(
                Original::Fixture(mount),
                &f.bytes,
                &f.trust,
                &f.host,
                |credentials, policy| {
                    let mut read = credentials.pin_original_tasks(policy)?;
                    read.mutate_signed_mapping_for_test(case);
                    Ok(read)
                },
            );
            if case == 3 {
                assert!(matches!(result, Err(SignedTasksError::Tasks(_))));
            } else {
                assert!(matches!(result, Err(SignedTasksError::Mapping)));
            }
        });
    }
    with_mount(1, |mount, _, f| {
        let mut read = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        assert!(read.reader.tasks.matches_worker_policy(&f.policy));
        let mut changed = f.policy.clone();
        changed.supplementary_gids.push(12345);
        changed.supplementary_gids.sort_unstable();
        assert!(!read.reader.tasks.matches_worker_policy(&changed));
        read.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
    });
}

#[test]
fn signed_tasks_prior_inner_refusal_precedes_bad_signed_bytes_and_opener() {
    for case in 0..4 {
        with_mount(1, |mount, _, f| {
            match case {
                0 => mount.reader.mount.refuse(),
                1 => mount
                    .reader
                    .original
                    .reader_mut()
                    .credentials
                    .refuse_mount_component_for_test(true),
                2 => mount
                    .reader
                    .original
                    .reader_mut()
                    .credentials
                    .refuse_mount_component_for_test(false),
                3 => mount.reader.original.reader_mut().account.refuse(),
                _ => unreachable!(),
            }
            if case == 1 || case == 2 {
                assert!(matches!(
                    mount
                        .reader
                        .original
                        .reader()
                        .credentials
                        .pin_original_tasks(&f.policy),
                    Err(TaskError::Refused)
                ));
            }
            assert!(matches!(
                construct(
                    Original::Fixture(mount),
                    b"",
                    &f.trust,
                    &f.host,
                    |_, _| panic!("no opener after prior refusal")
                ),
                Err(SignedTasksError::Refused)
            ));
        });
    }
    with_mount(1, |mount, _, f| {
        let mut read = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        read.reader.tasks.refuse();
        assert!(matches!(
            read.revalidate(b"", &f.trust, &f.host),
            Err(SignedTasksError::Refused)
        ));
        assert_refused(&mut read, f);
    });
}

#[test]
fn signed_tasks_signed_revocation_and_account_restore_cannot_recover() {
    for bytes in [true, false] {
        with_mount(1, |mount, _, f| {
            let mut read = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
            if !bytes {
                fs::write(f.dir.path().join("group"), b"changed\n").unwrap();
            }
            assert!(matches!(
                read.revalidate(if bytes { b"" } else { &f.bytes }, &f.trust, &f.host),
                Err(SignedTasksError::Signed(_))
            ));
            fs::write(f.dir.path().join("group"), &f.group).unwrap();
            assert_refused(&mut read, f);
        });
    }
}

#[test]
fn signed_tasks_later_substitution_and_inner_refusal_latch_before_signed_io() {
    for case in 0..8 {
        with_mount(1, |mount, _, f| {
            let mut read = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
            if case < 4 {
                read.reader.tasks.mutate_signed_mapping_for_test(case);
                let result = read.revalidate(&f.bytes, &f.trust, &f.host);
                if case == 3 {
                    assert!(matches!(result, Err(SignedTasksError::Tasks(_))));
                } else {
                    assert!(matches!(result, Err(SignedTasksError::Mapping)));
                }
            } else {
                let original = read.reader.original.reader_mut();
                match case {
                    4 => original.mount.refuse(),
                    5 => original
                        .original
                        .reader_mut()
                        .credentials
                        .refuse_mount_component_for_test(true),
                    6 => original
                        .original
                        .reader_mut()
                        .credentials
                        .refuse_mount_component_for_test(false),
                    7 => original.original.reader_mut().account.refuse(),
                    _ => unreachable!(),
                }
                assert!(matches!(
                    read.revalidate(b"", &f.trust, &f.host),
                    Err(SignedTasksError::Refused)
                ));
            }
            assert_refused(&mut read, f);
        });
    }
}

#[test]
fn signed_tasks_concurrent_independent_original_chains_share_no_refusal() {
    let joins = (0..2)
        .map(|fail| {
            std::thread::spawn(move || {
                with_mount(1, |mount, _, f| {
                    let mut read =
                        open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
                    if fail == 0 {
                        read.reader.tasks.refuse();
                    }
                    for _ in 0..4 {
                        assert_eq!(
                            read.revalidate(&f.bytes, &f.trust, &f.host).is_err(),
                            fail == 0
                        );
                    }
                    assert_eq!(read.is_refused(), fail == 0);
                });
            })
        })
        .collect::<Vec<_>>();
    for join in joins {
        join.join().unwrap();
    }
}
