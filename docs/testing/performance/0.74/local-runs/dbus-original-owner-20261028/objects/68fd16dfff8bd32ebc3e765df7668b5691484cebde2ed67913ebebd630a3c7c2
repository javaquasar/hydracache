use super::*;
use crate::diagnostic_lease::{DiagnosticStage, DiagnosticStopReason};
use crate::diagnostic_output_preparation::recovery::tests::Fixture as Lifecycle;

#[test]
fn recovery_authority_quiet_state_does_not_mutate_or_grant_start() {
    for terminal in [false, true] {
        with_mount(1, |mount, _, f| {
            let mut lifecycle = Lifecycle::new(&f.host.boot_id);
            if terminal {
                lifecycle.state.stage = DiagnosticStage::Terminal;
                lifecycle.state.reason = Some(DiagnosticStopReason::OperatorCancelled);
                lifecycle.persist();
            }
            let (dir, mut registry) = journal(f);
            let before = inventory(&dir);
            let state = fs::read(lifecycle.root.join("active-diagnostic.json")).unwrap();
            let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
            {
                let mut bound =
                    bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host)
                        .unwrap();
                let view = bound
                    .check_fixture_lifecycle(
                        &f.bytes,
                        &f.trust,
                        &f.host,
                        &lifecycle.root,
                        &lifecycle.state.identity,
                    )
                    .unwrap();
                assert_eq!(view.stage, lifecycle.state.stage);
                assert!(!bound.is_refused());
            }
            assert!(!tasks.is_refused());
            assert_eq!(inventory(&dir), before);
            assert_eq!(
                fs::read(lifecycle.root.join("active-diagnostic.json")).unwrap(),
                state
            );
        });
    }
}

#[test]
fn recovery_authority_interrupted_and_pending_intents_durably_refuse_original() {
    for case in 0..7 {
        with_mount(1, |mount, _, f| {
            let mut lifecycle = Lifecycle::new(&f.host.boot_id);
            match case {
                0..=2 => {
                    lifecycle.state.stage = [
                        DiagnosticStage::Starting,
                        DiagnosticStage::Running,
                        DiagnosticStage::Stopping,
                    ][case];
                    lifecycle.state.cleanup_confirmed = false;
                    lifecycle.state.cell_started_monotonic_ns = Some(1_000_000_000);
                    if case == 2 {
                        lifecycle.state.reason = Some(DiagnosticStopReason::OperatorCancelled);
                    }
                    lifecycle.persist();
                }
                3 => fs::write(
                    lifecycle.root.join(".active-diagnostic.pending"),
                    b"retained",
                )
                .unwrap(),
                4 => fs::write(
                    lifecycle.root.join(".diagnostic-requests.pending"),
                    b"retained",
                )
                .unwrap(),
                5 => lifecycle.ledger(true),
                _ => fs::remove_file(lifecycle.root.join("active-diagnostic.json")).unwrap(),
            }
            let (dir, mut registry) = journal(f);
            let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
            {
                let mut bound =
                    bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host)
                        .unwrap();
                let result = bound.check_fixture_lifecycle(
                    &f.bytes,
                    &f.trust,
                    &f.host,
                    &lifecycle.root,
                    &lifecycle.state.identity,
                );
                match case {
                    0..=2 => assert!(matches!(
                        result,
                        Err(SignedAuthorityError::Lifecycle(
                            FixtureRecoveryError::Interrupted
                        ))
                    )),
                    3..=5 => assert!(matches!(
                        result,
                        Err(SignedAuthorityError::Lifecycle(
                            FixtureRecoveryError::Uncertain
                        ))
                    )),
                    _ => assert!(matches!(
                        result,
                        Err(SignedAuthorityError::Lifecycle(
                            FixtureRecoveryError::Missing
                        ))
                    )),
                }
                assert!(bound.is_refused());
                assert!(matches!(
                    bound.check_fixture_lifecycle(
                        b"bad",
                        &f.trust,
                        &f.host,
                        std::path::Path::new("/var/lib/hydracache-performance/campaigns"),
                        &lifecycle.state.identity
                    ),
                    Err(SignedAuthorityError::Refused)
                ));
            }
            assert!(tasks.is_refused());
            assert_eq!(registry.view().unwrap().revision, 2);
            drop(registry);
            assert!(reopen(&dir, f).unwrap().view().unwrap().refused);
            if case == 3 || case == 4 {
                let name = if case == 3 {
                    ".active-diagnostic.pending"
                } else {
                    ".diagnostic-requests.pending"
                };
                assert_eq!(fs::read(lifecycle.root.join(name)).unwrap(), b"retained");
            }
        });
    }
}

#[test]
fn recovery_authority_foreign_boot_and_identity_cannot_select_new_authority() {
    for boot in [false, true] {
        with_mount(1, |mount, _, f| {
            let lifecycle = Lifecycle::new(&f.host.boot_id);
            let (dir, mut registry) = journal(f);
            let mut expected = lifecycle.state.identity.clone();
            if boot {
                expected.boot_id = "11111111-1111-1111-1111-111111111111".into();
            } else {
                expected.lease_id = "d".repeat(64);
            }
            let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
            {
                let mut bound =
                    bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host)
                        .unwrap();
                assert!(matches!(
                    bound.check_fixture_lifecycle(
                        &f.bytes,
                        &f.trust,
                        &f.host,
                        &lifecycle.root,
                        &expected
                    ),
                    Err(SignedAuthorityError::Lifecycle(
                        FixtureRecoveryError::Identity
                    ))
                ));
            }
            assert!(tasks.is_refused());
            drop(registry);
            assert!(reopen(&dir, f).unwrap().view().unwrap().refused);
        });
    }
}

#[test]
fn recovery_authority_unsafe_journal_stops_before_lifecycle_without_append() {
    with_mount(1, |mount, _, f| {
        let lifecycle = Lifecycle::new(&f.host.boot_id);
        let (dir, mut registry) = journal(f);
        let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        {
            let mut bound =
                bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host)
                    .unwrap();
            fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o770)).unwrap();
            let before = inventory(&dir);
            assert!(matches!(
                bound.check_fixture_lifecycle(
                    &f.bytes,
                    &f.trust,
                    &f.host,
                    std::path::Path::new("absent"),
                    &lifecycle.state.identity
                ),
                Err(SignedAuthorityError::Authority(_))
            ));
            assert!(bound.is_refused());
            assert_eq!(inventory(&dir), before);
        }
        assert!(tasks.is_refused());
        assert!(registry.is_failed());
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    });
}

#[test]
fn recovery_authority_corrupt_lifecycle_refuses_without_repair() {
    with_mount(1, |mount, _, f| {
        let lifecycle = Lifecycle::new(&f.host.boot_id);
        let (dir, mut registry) = journal(f);
        let path = lifecycle.root.join("active-diagnostic.json");
        fs::write(&path, b"retained-invalid").unwrap();
        let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        {
            let mut bound =
                bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host)
                    .unwrap();
            assert!(matches!(
                bound.check_fixture_lifecycle(
                    &f.bytes,
                    &f.trust,
                    &f.host,
                    &lifecycle.root,
                    &lifecycle.state.identity
                ),
                Err(SignedAuthorityError::Lifecycle(
                    FixtureRecoveryError::State(_)
                ))
            ));
        }
        assert_eq!(fs::read(path).unwrap(), b"retained-invalid");
        drop(registry);
        assert!(reopen(&dir, f).unwrap().view().unwrap().refused);
    });
}

#[test]
fn recovery_authority_four_publication_faults_keep_first_failure_and_no_retry() {
    for stage in [
        PublicationStage::RecordCreated,
        PublicationStage::RecordSynced,
        PublicationStage::MarkerCreated,
        PublicationStage::DurableCommit,
    ] {
        with_mount(1, |mount, _, f| {
            let lifecycle = Lifecycle::new(&f.host.boot_id);
            fs::write(
                lifecycle.root.join(".active-diagnostic.pending"),
                b"retained",
            )
            .unwrap();
            let (dir, mut registry) = journal(f);
            let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
            {
                let mut bound =
                    bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host)
                        .unwrap();
                let result = bound.check_lifecycle_with(
                    &f.bytes,
                    &f.trust,
                    &f.host,
                    &lifecycle.root,
                    &lifecycle.state.identity,
                    |journal, reason| match journal {
                        Journal::Fixture(read) => read.refuse_at_for_test(reason, stage),
                        Journal::Fixed(_) => panic!("fixture-only route"),
                    },
                );
                match result {
                    Err(SignedAuthorityError::RefusalUncertain {
                        failure,
                        persistence: AuthorityError::Injected(at),
                    }) => {
                        assert!(matches!(
                            *failure,
                            SignedAuthorityError::Lifecycle(FixtureRecoveryError::Uncertain)
                        ));
                        assert_eq!(at, stage);
                    }
                    other => panic!("unexpected {other:?}"),
                }
                let after = inventory(&dir);
                assert!(matches!(
                    bound.check_fixture_lifecycle(
                        &f.bytes,
                        &f.trust,
                        &f.host,
                        &lifecycle.root,
                        &lifecycle.state.identity
                    ),
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
            assert_eq!(
                fs::read(lifecycle.root.join(".active-diagnostic.pending")).unwrap(),
                b"retained"
            );
        });
    }
}
