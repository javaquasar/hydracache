use super::*;
use super::super::{bind_fixture_authority, bind_fixed_authority, Journal, FixedSignedTasksRead};
use super::super::tests::{journal, reopen, inventory, with_mount, changed_checked};
use crate::diagnostic_worker_policy::local_context::account_files::kernel_binding::tests::Fixture;
use crate::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::open_signed_fixture_tasks;
use crate::diagnostic_worker_policy::intent_projection::*;
use crate::diagnostic_ipc::{DiagnosticPolicy, DiagnosticPeer, DiagnosticController};
use crate::diagnostic_lease::{DiagnosticState, DiagnosticStage, DiagnosticIdentity};
use crate::diagnostic_worker_policy::authority_registry::{PublicationStage, fixture::fixed::modeled_registry_for_test};
use ed25519_dalek::{Signer, SigningKey};
use std::{fs, os::unix::fs::PermissionsExt};

#[path = "diagnostic_fenced_intent_tests.rs"]
mod fenced_state_tests;

#[path = "diagnostic_empty_fenced_intent_tests.rs"]
mod empty_fence_tests;

#[path = "diagnostic_reservation_publication_tests.rs"]
mod reservation_publication_tests;

#[path = "diagnostic_initial_consumption_tests.rs"]
mod initial_consumption_tests;

#[path = "diagnostic_process_crash_tests.rs"]
mod process_crash_tests;

#[path = "diagnostic_early_attempt_tests.rs"]
mod early_attempt_tests;

#[path = "diagnostic_attempt_publication_tests.rs"]
mod attempt_publication_tests;

struct Intent {
    state: DiagnosticState,
    controller: DiagnosticPolicy,
    peer: DiagnosticPeer,
    key: SigningKey,
}
impl Intent {
    fn new(f: &Fixture) -> Self {
        let key = SigningKey::from_bytes(&[75; 32]);
        Self {
            state: DiagnosticState {
                identity: DiagnosticIdentity {
                    lease_id: "d".repeat(64),
                    boot_id: f.host.boot_id.clone(),
                    binary_sha256: "e".repeat(64),
                    build_provenance_sha256: "f".repeat(64),
                },
                revision: 1,
                stage: DiagnosticStage::Reserved,
                completed_cells: 0,
                reserved_monotonic_ns: 1_000_000_000,
                last_observed_monotonic_ns: 2_000_000_000,
                controller_monotonic_ns: 2_000_000_000,
                cell_started_monotonic_ns: None,
                cgroup_inode: None,
                reason: None,
                cleanup_confirmed: true,
                promotable: false,
                admission_allowed: false,
            },
            controller: DiagnosticPolicy {
                repository_id: 1_217_101_761,
                actor_ids: vec![42, 43],
                client_uids: vec![900, 901],
                client_gid: 902,
                verifying_key: key.verifying_key(),
            },
            peer: DiagnosticPeer {
                uid: 900,
                gids: vec![902],
            },
            key,
        }
    }
    fn body(&self, f: &Fixture) -> IntentProjectionBody {
        let projected = project_reserved_state(&self.state).unwrap();
        IntentProjectionBody {
            schema_version: INTENT_SCHEMA.into(),
            request_id: "1".repeat(64),
            nonce_sha256: "2".repeat(64),
            controller: DiagnosticController {
                repository_id: 1_217_101_761,
                run_id: 9,
                run_attempt: 2,
                actor_id: 42,
            },
            issued_at_unix_seconds: 100,
            expires_at_unix_seconds: 160,
            state_sha256: projected.state_sha256,
            unit_spec_sha256: projected.unit_spec_sha256,
            boot_id: f.host.boot_id.clone(),
            worker_policy_sha256: f.checked().policy_sha256().into(),
            worker_policy_epoch: f.policy.policy_epoch,
        }
    }
    fn signed(&self, body: &IntentProjectionBody) -> Vec<u8> {
        let mut raw = crate::canonical_json(&SignedIntentProjection {
            schema_version: SIGNED_INTENT_SCHEMA.into(),
            intent: body.clone(),
            signature_hex: self
                .key
                .sign(&intent_signing_message(body).unwrap())
                .to_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        })
        .unwrap();
        raw.push(b'\n');
        raw
    }
    fn check<'a>(&'a self, f: &'a Fixture, now: u64) -> IntentCheck<'a> {
        IntentCheck {
            state: &self.state,
            peer: &self.peer,
            controller_policy: &self.controller,
            now_unix_seconds: now,
            worker_bytes: &f.bytes,
            worker_trust: &f.trust,
            worker_host: &f.host,
        }
    }
}

#[test]
fn authority_intent_bracket_preserves_order_and_first_failure() {
    let order = [Step::Authority, Step::Intent, Step::Authority];
    for fail in 0..=3 {
        let mut seen = vec![];
        let result = bracket(|step| {
            seen.push(step);
            if seen.len() - 1 == fail {
                Err(IntentProjectionError::Time.into())
            } else {
                Ok(())
            }
        });
        assert_eq!(seen, order[..(fail + 1).min(3)]);
        if fail == 3 {
            assert!(result.is_ok());
        } else {
            assert!(matches!(
                result,
                Err(AuthorityIntentError::Projection(
                    IntentProjectionError::Time
                ))
            ));
        }
    }
}
#[test]
fn authority_intent_unbound_bad_signature_state_and_worker_claim_do_not_revoke() {
    with_mount(1, |mount, _, f| {
        let (dir, mut registry) = journal(f);
        let before = inventory(&dir);
        let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        let mut authority =
            bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host).unwrap();
        let mut intent = Intent::new(f);
        let raw = intent.signed(&intent.body(f));
        for case in 0..4 {
            let mut check = intent.check(f, 100);
            let mut bad = raw.clone();
            match case {
                0 => bad[0] = b'x',
                1 => {
                    let mut doc: SignedIntentProjection = serde_json::from_slice(&raw).unwrap();
                    doc.signature_hex = "0".repeat(128);
                    bad = crate::canonical_json(&doc).unwrap();
                    bad.push(b'\n');
                }
                2 => check.now_unix_seconds = 160,
                _ => check.worker_bytes = b"foreign",
            }
            assert!(matches!(
                bind_fixture_intent(&mut authority, &bad, &check),
                Err(AuthorityIntentError::Projection(_))
            ));
            assert!(!authority.is_refused());
            authority.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
            assert_eq!(inventory(&dir), before);
        }
        intent.state.stage = DiagnosticStage::Starting;
        assert!(matches!(
            bind_fixture_intent(&mut authority, &raw, &intent.check(f, 100)),
            Err(AuthorityIntentError::Projection(
                IntentProjectionError::State
            ))
        ));
        assert!(!authority.is_refused());
    });
}
#[test]
fn authority_intent_original_chain_repeats_and_healthy_drop_are_readonly() {
    with_mount(2, |mount, _, f| {
        let (dir, mut registry) = journal(f);
        let before = inventory(&dir);
        let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        {
            let mut authority =
                bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host)
                    .unwrap();
            let intent = Intent::new(f);
            let raw = intent.signed(&intent.body(f));
            {
                let mut bound =
                    bind_fixture_intent(&mut authority, &raw, &intent.check(f, 100)).unwrap();
                for now in [100, 101, 159] {
                    bound.revalidate(&raw, &intent.check(f, now)).unwrap();
                }
                assert!(!bound.is_refused());
            }
            authority.revalidate(&f.bytes, &f.trust, &f.host).unwrap();
        }
        assert!(!tasks.is_refused());
        assert_eq!(inventory(&dir), before);
    });
}
#[test]
fn authority_intent_bound_drift_refuses_original_and_never_refreshes() {
    for case in 0..8 {
        with_mount(1, |mount, _, f| {
            let (dir, mut registry) = journal(f);
            let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
            {
                let mut authority =
                    bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host)
                        .unwrap();
                let mut intent = Intent::new(f);
                let original = intent.signed(&intent.body(f));
                let mut bound =
                    bind_fixture_intent(&mut authority, &original, &intent.check(f, 100)).unwrap();
                let mut body = intent.body(f);
                let mut now = 101;
                match case {
                    0 => body.request_id = "3".repeat(64),
                    1 => {
                        intent.state.revision += 1;
                        body = intent.body(f);
                    }
                    2 => intent.peer.uid = 901,
                    3 => intent.controller.actor_ids = vec![42],
                    4 => now = 160,
                    5 => body.controller.run_attempt += 1,
                    6 => body.nonce_sha256 = "4".repeat(64),
                    _ => {
                        body.issued_at_unix_seconds = 101;
                        body.expires_at_unix_seconds = 161;
                    }
                }
                let changed = intent.signed(&body);
                assert!(matches!(
                    bound.revalidate(&changed, &intent.check(f, now)),
                    Err(AuthorityIntentError::Projection(_))
                ));
                assert!(bound.is_refused());
                let after = inventory(&dir);
                assert_eq!(after.len(), 4);
                assert!(matches!(
                    bound.revalidate(b"bad", &intent.check(f, u64::MAX)),
                    Err(AuthorityIntentError::Refused)
                ));
                assert_eq!(inventory(&dir), after);
            }
            assert!(tasks.is_refused());
            drop(registry);
            assert!(reopen(&dir, f).unwrap().view().unwrap().refused);
        });
    }
}
#[test]
fn authority_intent_original_worker_failure_keeps_authority_error() {
    with_mount(1, |mount, _, f| {
        let (dir, mut registry) = journal(f);
        let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        {
            let mut authority =
                bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host)
                    .unwrap();
            let intent = Intent::new(f);
            let raw = intent.signed(&intent.body(f));
            let mut bound =
                bind_fixture_intent(&mut authority, &raw, &intent.check(f, 100)).unwrap();
            let mut check = intent.check(f, 101);
            check.worker_bytes = b"bad";
            assert!(matches!(
                bound.revalidate(&raw, &check),
                Err(AuthorityIntentError::Authority(
                    SignedAuthorityError::Signed(_)
                ))
            ));
            assert!(bound.is_refused());
        }
        assert!(tasks.is_refused());
        drop(registry);
        assert!(reopen(&dir, f).unwrap().view().unwrap().refused);
    });
}
#[test]
fn authority_intent_unsafe_storage_is_not_repaired_after_refusal() {
    with_mount(1, |mount, _, f| {
        let (dir, mut registry) = journal(f);
        let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        let mut authority =
            bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host).unwrap();
        let intent = Intent::new(f);
        let raw = intent.signed(&intent.body(f));
        let mut bound = bind_fixture_intent(&mut authority, &raw, &intent.check(f, 100)).unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o770)).unwrap();
        let before = inventory(&dir);
        assert!(matches!(
            bound.revalidate(&raw, &intent.check(f, 101)),
            Err(AuthorityIntentError::Authority(_))
        ));
        assert!(bound.is_refused());
        assert_eq!(inventory(&dir), before);
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        assert!(matches!(
            bound.revalidate(&raw, &intent.check(f, 100)),
            Err(AuthorityIntentError::Refused)
        ));
        assert_eq!(inventory(&dir), before);
    });
}
#[test]
fn authority_intent_partial_refusal_keeps_first_error_and_uncertainty() {
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
                let mut authority =
                    bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host)
                        .unwrap();
                let intent = Intent::new(f);
                let raw = intent.signed(&intent.body(f));
                let mut bound =
                    bind_fixture_intent(&mut authority, &raw, &intent.check(f, 100)).unwrap();
                let error = bound.reader.reject_projection_with(
                    IntentProjectionError::Time,
                    |journal, reason| match journal {
                        Journal::Fixture(registry) => registry.refuse_at_for_test(reason, stage),
                        Journal::Fixed(_) => panic!("fixture test only"),
                    },
                );
                match error {
                    AuthorityIntentError::RefusalUncertain {
                        failure,
                        publication: AuthorityError::Injected(at),
                    } => {
                        assert_eq!(at, stage);
                        assert!(matches!(
                            *failure,
                            AuthorityIntentError::Projection(IntentProjectionError::Time)
                        ));
                    }
                    _ => panic!("first error and publication uncertainty must survive"),
                }
                let after = inventory(&dir);
                assert!(matches!(
                    bound.revalidate(&raw, &intent.check(f, 100)),
                    Err(AuthorityIntentError::Refused)
                ));
                assert_eq!(inventory(&dir), after);
            }
            assert!(tasks.is_refused());
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
fn authority_intent_advanced_authority_is_never_appended_under_old_pins() {
    with_mount(1, |mount, _, f| {
        let (dir, mut registry) = journal(f);
        let changed = changed_checked(f, 0);
        let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        let mut authority =
            bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host).unwrap();
        let intent = Intent::new(f);
        let raw = intent.signed(&intent.body(f));
        let mut bound = bind_fixture_intent(&mut authority, &raw, &intent.check(f, 100)).unwrap();
        match &mut bound.reader.original.journal {
            Journal::Fixture(registry) => {
                registry.refuse(RefusalReason::Operator).unwrap();
                registry.advance(&changed).unwrap();
            }
            Journal::Fixed(_) => unreachable!(),
        }
        let before = inventory(&dir);
        assert!(matches!(
            bound.revalidate(&raw, &intent.check(f, 101)),
            Err(AuthorityIntentError::Authority(_))
        ));
        assert!(bound.is_refused());
        assert_eq!(inventory(&dir), before);
    });
}
#[test]
fn authority_intent_fixed_dispatch_is_only_a_private_nonroot_model() {
    with_mount(1, |mount, _, f| {
        let (dir, fixture) = journal(f);
        let before = inventory(&dir);
        let mut registry = modeled_registry_for_test(fixture);
        let fixture = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        let mut tasks = FixedSignedTasksRead {
            reader: fixture.reader,
        };
        let mut authority =
            bind_fixed_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host).unwrap();
        let intent = Intent::new(f);
        let raw = intent.signed(&intent.body(f));
        let mut bound = bind_fixed_intent(&mut authority, &raw, &intent.check(f, 100)).unwrap();
        bound.revalidate(&raw, &intent.check(f, 101)).unwrap();
        assert_eq!(inventory(&dir), before);
        assert!(matches!(
            bound.revalidate(&raw, &intent.check(f, 160)),
            Err(AuthorityIntentError::Projection(
                IntentProjectionError::Time
            ))
        ));
        assert!(bound.is_refused());
        assert_eq!(inventory(&dir).len(), 4);
    });
}
#[test]
fn authority_intent_independent_concurrent_chains_keep_local_refusal() {
    std::thread::scope(|scope| {
        let handles: Vec<_> = [true, false]
            .into_iter()
            .map(|fail| {
                scope.spawn(move || {
                    with_mount(1, |mount, _, f| {
                        let (_dir, mut registry) = journal(f);
                        let mut tasks =
                            open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
                        let mut authority = bind_fixture_authority(
                            &mut tasks,
                            &mut registry,
                            &f.bytes,
                            &f.trust,
                            &f.host,
                        )
                        .unwrap();
                        let intent = Intent::new(f);
                        let raw = intent.signed(&intent.body(f));
                        let mut bound =
                            bind_fixture_intent(&mut authority, &raw, &intent.check(f, 100))
                                .unwrap();
                        let result =
                            bound.revalidate(&raw, &intent.check(f, if fail { 160 } else { 101 }));
                        assert_eq!(result.is_err(), fail);
                        assert_eq!(bound.is_refused(), fail);
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
fn authority_intent_public_types_are_original_thread_only() {
    trait AmbiguousSend<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSend<()> for T {}
    impl<T: ?Sized + Send> AmbiguousSend<u8> for T {}
    let _=<FixedAuthorityIntentRead<'static,'static,'static,'static,'static,'static,'static> as AmbiguousSend<_>>::marker;
    let _ = <FixtureAuthorityIntentRead<
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
    let _=<FixedAuthorityIntentRead<'static,'static,'static,'static,'static,'static,'static> as AmbiguousSync<_>>::marker;
    let _ = <FixtureAuthorityIntentRead<
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
fn authority_intent_already_refused_constructor_cannot_check_or_publish() {
    with_mount(1, |mount, _, f| {
        let (dir, mut registry) = journal(f);
        let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        let mut authority =
            bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host).unwrap();
        let intent = Intent::new(f);
        let raw = intent.signed(&intent.body(f));
        {
            let mut bound =
                bind_fixture_intent(&mut authority, &raw, &intent.check(f, 100)).unwrap();
            assert!(bound.revalidate(&raw, &intent.check(f, 160)).is_err());
        }
        let before = inventory(&dir);
        assert!(matches!(
            bind_fixture_intent(&mut authority, b"invalid", &intent.check(f, u64::MAX)),
            Err(AuthorityIntentError::Refused)
        ));
        assert_eq!(inventory(&dir), before);
    });
}

#[test]
fn authority_intent_storage_becomes_unsafe_before_publication_keeps_projection_error() {
    with_mount(1, |mount, _, f| {
        let (dir, mut registry) = journal(f);
        let mut tasks = open_signed_fixture_tasks(mount, &f.bytes, &f.trust, &f.host).unwrap();
        let mut authority =
            bind_fixture_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host).unwrap();
        let intent = Intent::new(f);
        let raw = intent.signed(&intent.body(f));
        let mut bound = bind_fixture_intent(&mut authority, &raw, &intent.check(f, 100)).unwrap();
        // Model the change after the opening bracket, before the refusal append.
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o770)).unwrap();
        let before = inventory(&dir);
        let error = bound
            .reader
            .reject_projection_with(IntentProjectionError::Time, |_, _| {
                panic!("unsafe journal must not be published")
            });
        assert!(
            matches!(error, AuthorityIntentError::RefusalUncertain { failure, .. } if matches!(*failure, AuthorityIntentError::Projection(IntentProjectionError::Time)))
        );
        assert!(bound.is_refused());
        assert_eq!(inventory(&dir), before);
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        assert!(matches!(
            bound.revalidate(&raw, &intent.check(f, 100)),
            Err(AuthorityIntentError::Refused)
        ));
        assert_eq!(inventory(&dir), before);
    });
}
