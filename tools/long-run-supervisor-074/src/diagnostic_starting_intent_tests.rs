use super::*;
use crate::sha256_hex;
use crate::diagnostic_worker_policy::authority_registry::PublicationStage as PublicationStageAuthority;
use super::super::attempt_publication::publish_reserved_from_attempt;
use super::super::early_attempt::retain_fixed_attempt;
use super::super::initial_consumption::consume_fixed_initial_intent;
use super::super::starting_intent::*;
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::with_owned_empty_fence_for_test;
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::retained_context_refuses_for_test;
use crate::host_execution::{HostExecutionClaim, HostExecutionError};
use std::os::unix::fs::{MetadataExt, symlink};

macro_rules! chain {
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
                let attempt =
                    retain_fixed_attempt(&mut $original, fence, &$raw, &$i.check($f, 100)).unwrap();
                let reserved =
                    publish_reserved_from_attempt(attempt, &$raw, &$i.check($f, 100)).unwrap();
                let $read =
                    consume_fixed_initial_intent(reserved, &$raw, &$i.check($f, 100)).unwrap();
                $body
            });
        });
    };
}
fn next(state: &DiagnosticState) -> DiagnosticState {
    let mut next = state.clone();
    next.revision += 1;
    next.stage = DiagnosticStage::Starting;
    next.cell_started_monotonic_ns = Some(next.last_observed_monotonic_ns);
    next.cleanup_confirmed = false;
    next
}
#[test]
fn starting_intent_healthy_owned_departure_retains_initial_anchor() {
    chain!(i, original, read, dir, root, leaf, f, raw, {
        let _ = (&dir, root);
        let initial = objects(leaf);
        let mut starting =
            retain_fixed_starting_intent(read, &raw, &i.check(f, 100), &next(&i.state)).unwrap();
        starting.revalidate(&raw, &i.check(f, 100)).unwrap();
        assert_originals(leaf, &initial);
        busy(root);
        assert!(leaf.join("diagnostic-starting-intent-074.json").exists());
        let path = leaf.join("diagnostic-starting-intent-074.json");
        let raw_record = fs::read(&path).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&raw_record).unwrap();
        assert_eq!(value["schema_version"], STARTING_SCHEMA);
        assert_eq!(value["next_state"]["stage"], "starting");
        assert_eq!(value["next_state"]["revision"], 2);
        assert_eq!(value["next_state"]["completed_cells"], 0);
        assert_eq!(value["original_signed_intent_sha256"], sha256_hex(&raw));
        assert_eq!(
            value["initial_intent_record_sha256"],
            sha256_hex(&fs::read(leaf.join("diagnostic-intent-cell-1.json")).unwrap())
        );
        assert_eq!(
            value["previous_state_sha256"],
            sha256_hex(&initial["active-diagnostic.json"].0)
        );
        assert_eq!(fs::metadata(path).unwrap().mode() & 0o7777, 0o400);
        assert_eq!(raw_record.last(), Some(&b'\n'));
        drop(starting);
        assert!(!original.is_refused());
        retained_context_refuses_for_test(root);
    });
}

fn stamp(m: &fs::Metadata) -> [u64; 11] {
    [
        m.dev(),
        m.ino(),
        u64::from(m.uid()),
        u64::from(m.gid()),
        u64::from(m.mode()),
        m.nlink(),
        m.len(),
        m.mtime() as u64,
        m.mtime_nsec() as u64,
        m.ctime() as u64,
        m.ctime_nsec() as u64,
    ]
}
fn objects(leaf: &std::path::Path) -> std::collections::BTreeMap<String, (Vec<u8>, [u64; 11])> {
    fs::read_dir(leaf)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            (
                path.file_name().unwrap().to_str().unwrap().to_owned(),
                (
                    fs::read(&path).unwrap(),
                    stamp(&fs::metadata(&path).unwrap()),
                ),
            )
        })
        .collect()
}
fn assert_originals(
    leaf: &std::path::Path,
    originals: &std::collections::BTreeMap<String, (Vec<u8>, [u64; 11])>,
) {
    for (name, (raw, metadata)) in originals {
        let path = leaf.join(name);
        assert_eq!(&fs::read(&path).unwrap(), raw);
        assert_eq!(&stamp(&fs::metadata(path).unwrap()), metadata);
    }
}
fn busy(root: &std::path::Path) {
    assert!(matches!(
        HostExecutionClaim::acquire(root, &"a".repeat(64)),
        Err(HostExecutionError::Busy)
    ));
}

#[test]
fn starting_intent_unbound_mismatch_retains_healthy_originals_and_fence() {
    chain!(i, original, read, dir, root, leaf, f, raw, {
        let before = objects(leaf);
        let authority_before = inventory(&dir);
        let mut candidate = next(&i.state);
        candidate.revision += 1;
        let failure =
            retain_fixed_starting_intent(read, &raw, &i.check(f, 100), &candidate).unwrap_err();
        assert!(matches!(
            failure.error(),
            super::super::reservation_publication::ReservationPublicationError::Binding
        ));
        assert!(!failure.is_refused());
        busy(root);
        assert_eq!(objects(leaf), before);
        assert_eq!(inventory(&dir), authority_before);
        drop(failure);
        assert!(!original.is_refused());
        retained_context_refuses_for_test(root);
    });
}

#[test]
fn starting_intent_all_boundaries_retain_partial_objects_and_first_failure() {
    for at in super::super::starting_intent::ALL_STARTING_STAGES {
        chain!(i, original, read, dir, root, leaf, f, raw, {
            let _ = &dir;
            let originals = objects(leaf);
            let failure = super::super::starting_intent::retain_with(read, &raw, &i.check(f, 100), &next(&i.state), |observed| {
                if observed == at { Err(super::super::reservation_publication::StoragePublicationError::StartingInterrupted(at).into()) }
                else { Ok(()) }
            }).unwrap_err();
            assert!(failure.is_refused());
            assert!(
                matches!(failure.error(), super::super::reservation_publication::ReservationPublicationError::Storage(super::super::reservation_publication::StoragePublicationError::StartingInterrupted(found)) if *found==at)
            );
            busy(root);
            assert_originals(leaf, &originals);
            let path = leaf.join("diagnostic-starting-intent-074.json");
            assert_eq!(path.exists(), at != StartingStage::BeforeCreate);
            if path.exists() {
                let bytes = fs::read(&path).unwrap();
                match at {
                    StartingStage::Created => assert!(bytes.is_empty()),
                    StartingStage::PrefixWritten => {
                        assert!(serde_json::from_slice::<serde_json::Value>(&bytes).is_err())
                    }
                    _ => assert!(serde_json::from_slice::<serde_json::Value>(&bytes).is_ok()),
                }
                assert_eq!(
                    fs::metadata(path).unwrap().mode() & 0o7777,
                    if matches!(
                        at,
                        StartingStage::Created
                            | StartingStage::PrefixWritten
                            | StartingStage::BytesWritten
                    ) {
                        0o600
                    } else {
                        0o400
                    }
                );
            }
            drop(failure);
            assert!(original.is_refused());
            retained_context_refuses_for_test(root);
        });
    }
}

#[test]
fn starting_intent_owned_document_substitution_and_byte_mode_link_drift_refuse() {
    for mutation in 0..5 {
        chain!(i, original, read, dir, root, leaf, f, raw, {
            let _ = &dir;
            let mut starting =
                retain_fixed_starting_intent(read, &raw, &i.check(f, 100), &next(&i.state))
                    .unwrap();
            let path = leaf.join("diagnostic-starting-intent-074.json");
            match mutation {
                0 => fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap(),
                1 => {
                    fs::hard_link(&path, leaf.join("extra-link")).unwrap();
                }
                2 => {
                    let bytes = fs::read(&path).unwrap();
                    fs::rename(&path, leaf.join("retained-original")).unwrap();
                    fs::write(&path, bytes).unwrap();
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
                }
                3 => {
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
                    fs::write(&path, b"changed").unwrap();
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
                }
                _ => {
                    fs::rename(&path, leaf.join("retained-original")).unwrap();
                    symlink("retained-original", &path).unwrap();
                }
            }
            assert!(starting.revalidate(&raw, &i.check(f, 100)).is_err());
            let before = objects(leaf);
            assert!(starting.revalidate(&raw, &i.check(f, 100)).is_err());
            assert_eq!(objects(leaf), before);
            busy(root);
            drop(starting);
            assert!(original.is_refused());
        });
    }
}

#[test]
fn starting_intent_late_target_and_foreign_context_are_never_adopted() {
    for foreign in [false, true] {
        chain!(i, original, read, dir, root, leaf, f, raw, {
            let _ = &dir;
            let failure = super::super::starting_intent::retain_with(
                read,
                &raw,
                &i.check(f, 100),
                &next(&i.state),
                |at| {
                    if at == StartingStage::BeforeCreate {
                        fs::write(
                            leaf.join(if foreign {
                                "future-context"
                            } else {
                                "diagnostic-starting-intent-074.json"
                            }),
                            b"foreign",
                        )
                        .unwrap();
                    }
                    Ok(())
                },
            )
            .unwrap_err();
            assert!(failure.is_refused());
            busy(root);
            assert_eq!(
                fs::read(leaf.join(if foreign {
                    "future-context"
                } else {
                    "diagnostic-starting-intent-074.json"
                }))
                .unwrap(),
                b"foreign"
            );
            drop(failure);
            assert!(original.is_refused());
        });
    }
}

#[test]
fn starting_intent_original_objects_changed_after_creation_refuse() {
    for name in [
        "diagnostic-attempt-074.json",
        "active-diagnostic.json",
        "diagnostic-intent-cell-1.json",
        "diagnostic-intent-cell-1.commit",
    ] {
        chain!(i, original, read, dir, root, leaf, f, raw, {
            let _ = &dir;
            let failure = super::super::starting_intent::retain_with(
                read,
                &raw,
                &i.check(f, 100),
                &next(&i.state),
                |at| {
                    if at == StartingStage::Created {
                        let path = leaf.join(name);
                        let bytes = fs::read(&path).unwrap();
                        let mode = fs::metadata(&path).unwrap().mode() & 0o7777;
                        fs::rename(&path, leaf.join("retained-original")).unwrap();
                        fs::write(&path, bytes).unwrap();
                        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
                    }
                    Ok(())
                },
            )
            .unwrap_err();
            assert!(failure.is_refused());
            busy(root);
            assert!(leaf.join("diagnostic-starting-intent-074.json").exists());
            drop(failure);
            assert!(original.is_refused());
        });
    }
}

#[test]
fn starting_intent_expired_original_is_not_renewed_by_candidate() {
    chain!(i, original, read, dir, root, leaf, f, raw, {
        let _ = &dir;
        let failure = retain_fixed_starting_intent(read, &raw, &i.check(f, 160), &next(&i.state))
            .unwrap_err();
        assert!(failure.is_refused());
        busy(root);
        assert!(!leaf.join("diagnostic-starting-intent-074.json").exists());
        drop(failure);
        assert!(original.is_refused());
    });
}

#[test]
fn starting_intent_seeded_graph_mutations_never_complete_or_start() {
    with_mount(1, |_, _, f| {
        let i = Intent::new(f);
        let mut seed = 0x074bu64;
        for _ in 0..128 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let mut candidate = next(&i.state);
            assert!(super::super::starting_intent::valid_departure(
                &i.state, &candidate
            ));
            match seed % 8 {
                0 => candidate.revision += 1,
                1 => candidate.completed_cells += 1,
                2 => candidate.stage = DiagnosticStage::Running,
                3 => candidate.identity.lease_id = "0".repeat(64),
                4 => candidate.cgroup_inode = Some(9),
                5 => candidate.cleanup_confirmed = true,
                6 => candidate.reserved_monotonic_ns += 1,
                _ => {
                    candidate.reason =
                        Some(crate::diagnostic_lease::DiagnosticStopReason::OperatorCancelled)
                }
            }
            assert!(!super::super::starting_intent::valid_departure(
                &i.state, &candidate
            ));
        }
        println!("Starting graph seed=0x074b legal=128 rejected=128 execution=closed");
    });
}

#[test]
fn starting_intent_legacy_consumption_without_owned_attempt_cannot_depart() {
    with_mount(1, |mount, _, f| {
        let (dir, fixture) = journal(f);
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
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let reserved = super::super::reservation_publication::publish_fixed_reserved(
                &mut original,
                fence,
                &raw,
                &i.check(f, 100),
            )
            .unwrap();
            let consumed = consume_fixed_initial_intent(reserved, &raw, &i.check(f, 100)).unwrap();
            let before = objects(leaf);
            let failure =
                retain_fixed_starting_intent(consumed, &raw, &i.check(f, 100), &next(&i.state))
                    .unwrap_err();
            assert!(matches!(
                failure.error(),
                super::super::reservation_publication::ReservationPublicationError::Binding
            ));
            assert!(!failure.is_refused());
            busy(root);
            assert_eq!(objects(leaf), before);
            drop(failure);
            assert!(!original.is_refused());
            assert_eq!(inventory(&dir).len(), 2);
        });
    });
}

#[test]
fn starting_intent_unsafe_or_advanced_authority_keeps_refusal_uncertain() {
    for advanced in [false, true] {
        chain!(i, original, read, dir, root, leaf, f, raw, {
            let mut starting =
                retain_fixed_starting_intent(read, &raw, &i.check(f, 100), &next(&i.state))
                    .unwrap();
            if advanced {
                let changed = changed_checked(f, 0);
                match &mut starting.read.read.original.original.journal {
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
            assert!(matches!(starting.revalidate(&raw, &i.check(f, 100)), Err(super::super::reservation_publication::ReservationPublicationError::RefusalUncertain { .. })));
            assert_eq!(inventory(&dir), before);
            busy(root);
            fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
            assert!(matches!(
                starting.revalidate(&raw, &i.check(f, 100)),
                Err(super::super::reservation_publication::ReservationPublicationError::Refused)
            ));
            assert_eq!(inventory(&dir), before);
            drop(starting);
            assert!(original.is_refused());
        });
    }
}

#[test]
fn starting_intent_independent_concurrent_chains_do_not_share_acknowledgement() {
    std::thread::scope(|scope| {
        let threads: Vec<_> = [false, true].into_iter().map(|fail| scope.spawn(move || {
            chain!(i, original, read, dir, root, leaf, f, raw, {
                let _ = &dir;
                let originals = objects(leaf);
                let result = super::super::starting_intent::retain_with(read, &raw, &i.check(f, 100), &next(&i.state), |at| {
                    if fail && at == StartingStage::PrefixWritten { Err(super::super::reservation_publication::StoragePublicationError::StartingInterrupted(at).into()) } else { Ok(()) }
                });
                assert_eq!(result.is_err(), fail);
                busy(root);
                assert_originals(leaf, &originals);
                drop(result);
                assert_eq!(original.is_refused(), fail);
                retained_context_refuses_for_test(root);
            });
        })).collect();
        for thread in threads {
            thread.join().unwrap();
        }
    });
}

#[test]
fn starting_intent_private_engine_limits_and_second_attempt_do_not_append() {
    chain!(i, original, read, dir, root, leaf, f, raw, {
        let _ = &dir;
        let mut read = read;
        let before = objects(leaf);
        for invalid in [Vec::new(), vec![0; 4097]] {
            assert!(read.read.storage.publish_starting_with::<super::super::reservation_publication::ReservationPublicationError>(&invalid, |_| panic!("invalid bytes reached mutation")).is_err());
            assert_eq!(objects(leaf), before);
        }
        assert!(read.read.storage.revalidate_starting().is_err());
        let mut starting =
            retain_fixed_starting_intent(read, &raw, &i.check(f, 100), &next(&i.state)).unwrap();
        let before = objects(leaf);
        assert!(starting.read.read.storage.publish_starting_with::<super::super::reservation_publication::ReservationPublicationError>(b"second", |_| panic!("duplicate reached mutation")).is_err());
        assert_eq!(objects(leaf), before);
        starting.revalidate(&raw, &i.check(f, 100)).unwrap();
        busy(root);
        drop(starting);
        assert!(!original.is_refused());
    });
}

#[test]
fn starting_intent_partial_refusal_retains_first_starting_error_and_context() {
    chain!(i, original, read, dir, root, leaf, f, raw, {
        let mut starting =
            retain_fixed_starting_intent(read, &raw, &i.check(f, 100), &next(&i.state)).unwrap();
        let objects_before = objects(leaf);
        let error = super::super::reservation_publication::refuse_with(
            starting.read.read.original,
            &mut starting.read.read.storage,
            super::super::reservation_publication::StoragePublicationError::StartingInterrupted(
                StartingStage::Acknowledging,
            )
            .into(),
            |journal, reason| match journal {
                Journal::Fixed(registry) => {
                    registry.refuse_at_for_test(reason, PublicationStageAuthority::RecordCreated)
                }
                _ => unreachable!(),
            },
        );
        assert!(
            matches!(error, super::super::reservation_publication::ReservationPublicationError::RefusalUncertain { failure, .. } if matches!(*failure, super::super::reservation_publication::ReservationPublicationError::Storage(super::super::reservation_publication::StoragePublicationError::StartingInterrupted(StartingStage::Acknowledging))))
        );
        let before = inventory(&dir);
        assert!(matches!(
            starting.revalidate(&raw, &i.check(f, 100)),
            Err(super::super::reservation_publication::ReservationPublicationError::Refused)
        ));
        busy(root);
        assert_eq!(objects(leaf), objects_before);
        assert_eq!(inventory(&dir), before);
        drop(starting);
        assert!(original.is_refused());
    });
}

#[test]
fn starting_intent_original_peer_signature_and_worker_drift_refuse_before_creation() {
    for kind in 0..3 {
        chain!(i, original, read, dir, root, leaf, f, raw, {
            let _ = &dir;
            let before = objects(leaf);
            let peer = DiagnosticPeer {
                uid: 901,
                gids: vec![902],
            };
            let mut check = i.check(f, 100);
            let mut bytes = raw.clone();
            match kind {
                0 => check.peer = &peer,
                1 => {
                    let mut signed: SignedIntentProjection = serde_json::from_slice(&raw).unwrap();
                    signed.signature_hex = "00".repeat(64);
                    bytes = crate::canonical_json(&signed).unwrap();
                    bytes.push(b'\n');
                }
                _ => check.worker_bytes = b"invalid",
            }
            let failure =
                retain_fixed_starting_intent(read, &bytes, &check, &next(&i.state)).unwrap_err();
            assert!(failure.is_refused());
            assert!(matches!(
                failure.error(),
                super::super::reservation_publication::ReservationPublicationError::Intent(_)
            ));
            assert_eq!(objects(leaf), before);
            busy(root);
            drop(failure);
            assert!(original.is_refused());
        });
    }
}

#[test]
fn starting_intent_full_original_state_assertion_mismatch_is_unbound() {
    chain!(i, original, read, dir, root, leaf, f, raw, {
        let before = objects(leaf);
        let authority_before = inventory(&dir);
        let mut changed = i.state.clone();
        changed.last_observed_monotonic_ns += 1;
        let mut check = i.check(f, 100);
        check.state = &changed;
        let failure =
            retain_fixed_starting_intent(read, &raw, &check, &next(&changed)).unwrap_err();
        assert!(!failure.is_refused());
        busy(root);
        assert_eq!(objects(leaf), before);
        assert_eq!(inventory(&dir), authority_before);
        drop(failure);
        assert!(!original.is_refused());
    });
}

#[test]
fn starting_intent_clock_boot_and_cell_projection_edges_refuse() {
    with_mount(1, |_, _, f| {
        let i = Intent::new(f);
        for case in 0..6 {
            let mut candidate = next(&i.state);
            match case {
                0 => {
                    candidate.last_observed_monotonic_ns =
                        i.state.controller_monotonic_ns + 10_000_000_000;
                    candidate.cell_started_monotonic_ns =
                        Some(candidate.last_observed_monotonic_ns);
                }
                1 => candidate.identity.boot_id = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into(),
                2 => candidate.last_observed_monotonic_ns -= 1,
                3 => candidate.controller_monotonic_ns += 1,
                4 => candidate.cell_started_monotonic_ns = None,
                _ => {
                    candidate.cell_started_monotonic_ns =
                        Some(i.state.last_observed_monotonic_ns - 1)
                }
            }
            assert!(
                !super::super::starting_intent::valid_departure(&i.state, &candidate),
                "edge {case}"
            );
        }
    });
}
