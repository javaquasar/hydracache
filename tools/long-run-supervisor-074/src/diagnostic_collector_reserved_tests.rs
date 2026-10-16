use super::*;
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::*;

const RESERVED_NAME: &str = "diagnostic-collector-reserved-074.json";
#[path = "diagnostic_collector_reserved_crash_tests.rs"]
mod crash_tests;
const STAGES: [CollectorReservedStage; 9] = [
    CollectorReservedStage::BeforeCreate,
    CollectorReservedStage::Created,
    CollectorReservedStage::CreateDirectorySynced,
    CollectorReservedStage::PrefixWritten,
    CollectorReservedStage::BytesWritten,
    CollectorReservedStage::Sealed,
    CollectorReservedStage::FileSynced,
    CollectorReservedStage::DirectorySynced,
    CollectorReservedStage::Acknowledging,
];

#[test]
fn collector_reserved_api_shape() {
    use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::OriginalObservedReservation;
    fn probe(owner: OriginalObservedReservation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
        let _ = owner.publish_reserved(b"");
    }
    let _ = probe;
}

#[test]
fn collector_reserved_publishes_frozen_state_under_same_original_fence() {
    with_original(false, |original, _| {
        let request = original.original.original.request.clone();
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let attempt = original
                .bind_common_empty_fence(fence)
                .unwrap()
                .retain_request()
                .unwrap();
            let retained = fs::read(leaf.join(NAME)).unwrap();
            let mut observed = attempt.into_observed_reservation();
            let state = observed.state_to_sign().unwrap();
            let bytes = signed(&body(&request, &state, &retained));
            let mut visited = Vec::new();
            let mut read = observed
                .publish_reserved_with(&bytes, |at| {
                    visited.push(at);
                    assert_eq!(visited, STAGES[..visited.len()]);
                    println!("collector_reserved boundary={at:?} start=false");
                    busy(root);
                    Ok(())
                })
                .unwrap();
            assert_eq!(visited, STAGES);
            let raw = fs::read(leaf.join(RESERVED_NAME)).unwrap();
            validate_retained_collector_reserved(&raw, &retained).unwrap();
            record_mutations(&raw, &retained);
            let value: Value = serde_json::from_slice(&raw).unwrap();
            assert_eq!(value["state"], serde_json::to_value(&state).unwrap());
            assert_eq!(
                value["controller_attempt_sha256"],
                crate::sha256_hex(&retained)
            );
            assert_eq!(line(&value["collector_intent"]), bytes);
            assert_eq!(
                fs::metadata(leaf.join(RESERVED_NAME)).unwrap().mode() & 0o7777,
                0o400
            );
            assert_eq!(fs::read_dir(leaf).unwrap().count(), 2);
            read.revalidate().unwrap();
            assert!(!read.is_refused());
            assert!(!format!("{read:?}").contains("request_id"));
            assert_eq!(fs::read(leaf.join(NAME)).unwrap(), retained);
            assert_eq!(fs::read(leaf.join(RESERVED_NAME)).unwrap(), raw);
            busy(root);
            drop(read);
            drop(crate::host_execution::lock_host_root(root).unwrap());
            retained_context_refuses_for_test(root);
        });
    });
}

#[test]
fn collector_reserved_seeded_all_logical_boundaries_retain_failure_and_lock() {
    let mut order = STAGES;
    let mut seed = 0x0740c09a_u64;
    for i in (1..order.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        order.swap(i, seed as usize % (i + 1));
    }
    for target in order {
        with_original(true, |original, _| {
            let request = original.original.original.request.clone();
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let attempt = original
                    .bind_common_empty_fence(fence)
                    .unwrap()
                    .retain_request()
                    .unwrap();
                let retained = fs::read(leaf.join(NAME)).unwrap();
                let mut observed = attempt.into_observed_reservation();
                let state = observed.state_to_sign().unwrap();
                let bytes = signed(&body(&request, &state, &retained));
                let mut visited = Vec::new();
                let failure = observed
                    .publish_reserved_with(&bytes, |at| {
                        visited.push(at);
                        assert_eq!(visited, STAGES[..visited.len()]);
                        if at == target {
                            Err(CollectorReservedError::Interrupted(at))
                        } else {
                            Ok(())
                        }
                    })
                    .unwrap_err();
                assert!(
                    matches!(failure.error(), CollectorReservedError::Interrupted(at) if *at == target)
                );
                assert!(failure.is_refused());
                assert_eq!(fs::read(leaf.join(NAME)).unwrap(), retained);
                assert_eq!(
                    fs::read_dir(leaf).unwrap().count(),
                    if target == CollectorReservedStage::BeforeCreate {
                        1
                    } else {
                        2
                    }
                );
                if target != CollectorReservedStage::BeforeCreate {
                    let raw = fs::read(leaf.join(RESERVED_NAME)).unwrap();
                    assert!(raw.len() <= 16384);
                    if matches!(
                        target,
                        CollectorReservedStage::Created
                            | CollectorReservedStage::CreateDirectorySynced
                    ) {
                        assert!(raw.is_empty());
                    } else if target == CollectorReservedStage::PrefixWritten {
                        assert!(!raw.is_empty());
                        assert!(validate_retained_collector_reserved(&raw, &retained).is_err());
                    } else {
                        validate_retained_collector_reserved(&raw, &retained).unwrap();
                    }
                    assert_eq!(
                        fs::metadata(leaf.join(RESERVED_NAME)).unwrap().mode() & 0o7777,
                        if matches!(
                            target,
                            CollectorReservedStage::Created
                                | CollectorReservedStage::CreateDirectorySynced
                                | CollectorReservedStage::PrefixWritten
                                | CollectorReservedStage::BytesWritten
                        ) {
                            0o600
                        } else {
                            0o400
                        }
                    );
                }
                busy(root);
                drop(failure);
                drop(crate::host_execution::lock_host_root(root).unwrap());
                retained_context_refuses_for_test(root);
            });
        });
    }
    println!("collector_reserved seed=0x0740c09a logical_faults=9 root_start=false");
}

fn record_mutations(raw: &[u8], retained: &[u8]) {
    let value: Value = serde_json::from_slice(raw).unwrap();
    for case in 0..20 {
        let mut v = value.clone();
        match case {
            0 => v["schema_version"] = "diagnostic-collector-reserved-074-v999".into(),
            1 => v["controller_attempt_sha256"] = "e".repeat(64).into(),
            2 => v["state"]["revision"] = 2.into(),
            3 => v["state"]["completed_cells"] = 1.into(),
            4 => v["state"]["reserved_monotonic_ns"] = 1.into(),
            5 => v["state"]["identity"]["lease_id"] = "e".repeat(64).into(),
            6 => v["collector_intent"]["schema_version"] = "future".into(),
            7 => v["collector_intent"]["intent"]["schema_version"] = "future".into(),
            8 => v["collector_intent"]["signature_hex"] = "G".repeat(128).into(),
            9 => v["collector_intent"]["intent"]["request_id"] = "e".repeat(64).into(),
            10 => v["collector_intent"]["intent"]["nonce_sha256"] = "e".repeat(64).into(),
            11 => {
                v["collector_intent"]["intent"]["controller"]["run_id"] = (v["collector_intent"]
                    ["intent"]["controller"]["run_id"]
                    .as_u64()
                    .unwrap()
                    + 1)
                .into()
            }
            12 => v["collector_intent"]["intent"]["expires_at_unix_seconds"] = 1.into(),
            13 => v["collector_intent"]["intent"]["worker_policy_epoch"] = 99.into(),
            14 => v["collector_intent"]["intent"]["unit_spec_sha256"] = "e".repeat(64).into(),
            15 => v["collector_intent"]["intent"]["state_sha256"] = "e".repeat(64).into(),
            16 => {
                v["collector_intent"]["intent"]["boot_id"] =
                    "00000000-0000-0000-0000-000000000000".into()
            }
            17 => v["unknown"] = true.into(),
            18 => v["collector_intent"]["intent"]["worker_policy_sha256"] = "e".repeat(64).into(),
            _ => v["collector_intent"]["unknown"] = true.into(),
        }
        assert!(
            validate_retained_collector_reserved(&line(&v), retained).is_err(),
            "case={case}"
        );
    }
    for bad in [
        vec![],
        vec![b' '; 16385],
        [raw, b"\n"].concat(),
        b"{\"schema_version\":\"future\"}\n".to_vec(),
    ] {
        assert!(validate_retained_collector_reserved(&bad, retained).is_err());
    }
    assert!(validate_retained_collector_reserved(raw, b"{}").is_err());
    // A byte inspector deliberately does not authenticate signatures or observed clocks.
    let mut v = value;
    v["collector_intent"]["signature_hex"] = "0".repeat(128).into();
    validate_retained_collector_reserved(&line(&v), retained).unwrap();
    println!(
        "collector_reserved paired_byte_mutations=20 signature_authentication_by_inspector=false"
    );
}

#[test]
fn collector_reserved_bad_signed_state_epoch_legacy_and_budget_never_create() {
    for case in 0..4 {
        with_original(true, |original, _| {
            let request = original.original.original.request.clone();
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let attempt = original
                    .bind_common_empty_fence(fence)
                    .unwrap()
                    .retain_request()
                    .unwrap();
                let retained = fs::read(leaf.join(NAME)).unwrap();
                let mut observed = attempt.into_observed_reservation();
                let state = observed.state_to_sign().unwrap();
                let mut b = body(&request, &state, &retained);
                let bytes = match case {
                    0 => { b.state_sha256 = "e".repeat(64); signed(&b) },
                    1 => { b.worker_policy_epoch += 1; signed(&b) },
                    2 => line(&crate::diagnostic_worker_policy::intent_projection::SignedIntentProjection { schema_version: crate::diagnostic_worker_policy::intent_projection::SIGNED_INTENT_SCHEMA.into(), intent: b, signature_hex: "0".repeat(128) }),
                    _ => vec![b' '; 8193],
                };
                let failure = observed
                    .publish_reserved_with(&bytes, |_| {
                        panic!("invalid bytes reached create boundary")
                    })
                    .unwrap_err();
                assert!(failure.is_refused());
                assert!(matches!(
                    failure.error(),
                    CollectorReservedError::Original(_)
                ));
                assert_eq!(fs::read_dir(leaf).unwrap().count(), 1);
                assert_eq!(fs::read(leaf.join(NAME)).unwrap(), retained);
                busy(root);
                drop(failure);
                retained_context_refuses_for_test(root);
            });
        });
    }
}

#[test]
fn collector_reserved_pre_refused_owner_has_no_retry_io_or_write() {
    with_original(true, |original, paths| {
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let attempt = original
                .bind_common_empty_fence(fence)
                .unwrap()
                .retain_request()
                .unwrap();
            let mut observed = attempt.into_observed_reservation();
            observed.state_to_sign().unwrap();
            fs::write(&paths.worker, b"public test drift").unwrap();
            assert!(observed.revalidate().is_err());
            fs::remove_file(leaf.join(NAME)).unwrap();
            let failure = observed
                .publish_reserved_with(&vec![0; 8193], |_| panic!("refused owner performed I/O"))
                .unwrap_err();
            assert!(failure.is_refused());
            assert!(matches!(
                failure.error(),
                CollectorReservedError::Original(OriginalCollectorError::Refused)
            ));
            assert_eq!(fs::read_dir(leaf).unwrap().count(), 0);
            busy(root);
        });
    });
}

#[test]
fn collector_reserved_acknowledged_object_and_source_drift_are_sticky() {
    for case in 0..6 {
        with_original(true, |original, paths| {
            let request = original.original.original.request.clone();
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let attempt = original
                    .bind_common_empty_fence(fence)
                    .unwrap()
                    .retain_request()
                    .unwrap();
                let retained = fs::read(leaf.join(NAME)).unwrap();
                let mut observed = attempt.into_observed_reservation();
                let state = observed.state_to_sign().unwrap();
                let mut read = observed
                    .publish_reserved(&signed(&body(&request, &state, &retained)))
                    .unwrap();
                let path = match case {
                    0 => leaf.join(RESERVED_NAME),
                    1 => leaf.join(NAME),
                    2 => leaf.join("foreign"),
                    3 => paths.worker.clone(),
                    4 => paths.oidc.clone(),
                    _ => leaf.to_path_buf(),
                };
                if case == 5 {
                    let moved = tempfile::tempdir().unwrap();
                    fs::rename(&path, moved.path().join("old-leaf")).unwrap();
                    fs::create_dir(&path).unwrap();
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
                    assert!(read.revalidate().is_err());
                    assert!(read.is_refused());
                    fs::remove_dir(&path).unwrap();
                    fs::rename(moved.path().join("old-leaf"), &path).unwrap();
                    fs::remove_file(leaf.join(NAME)).unwrap();
                    assert!(matches!(
                        read.revalidate(),
                        Err(CollectorReservedError::Refused)
                    ));
                    busy(root);
                    return;
                }
                let saved = if case != 2 {
                    Some(fs::read(&path).unwrap())
                } else {
                    None
                };
                let moved = tempfile::tempdir().unwrap();
                if case < 2 {
                    fs::rename(&path, moved.path().join("old")).unwrap();
                    fs::write(&path, saved.as_ref().unwrap()).unwrap();
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
                } else {
                    fs::write(&path, b"public test drift").unwrap();
                }
                assert!(read.revalidate().is_err());
                assert!(read.is_refused());
                if case < 2 {
                    fs::remove_file(&path).unwrap();
                    fs::rename(moved.path().join("old"), &path).unwrap();
                } else if let Some(raw) = saved {
                    fs::write(&path, raw).unwrap();
                } else {
                    fs::remove_file(&path).unwrap();
                }
                fs::remove_file(leaf.join(NAME)).unwrap();
                assert!(matches!(
                    read.revalidate(),
                    Err(CollectorReservedError::Refused)
                ));
                busy(root);
            });
        });
    }
}

#[test]
fn collector_reserved_occupied_name_is_not_overwritten_or_adopted() {
    with_original(true, |original, _| {
        let request = original.original.original.request.clone();
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let attempt = original
                .bind_common_empty_fence(fence)
                .unwrap()
                .retain_request()
                .unwrap();
            let retained = fs::read(leaf.join(NAME)).unwrap();
            let mut observed = attempt.into_observed_reservation();
            let state = observed.state_to_sign().unwrap();
            let bytes = signed(&body(&request, &state, &retained));
            let failure = observed
                .publish_reserved_with(&bytes, |at| {
                    assert_eq!(at, CollectorReservedStage::BeforeCreate);
                    fs::write(leaf.join(RESERVED_NAME), b"occupied test object").unwrap();
                    Ok(())
                })
                .unwrap_err();
            assert!(matches!(
                failure.error(),
                CollectorReservedError::Storage(_)
            ));
            assert!(failure.is_refused());
            assert_eq!(
                fs::read(leaf.join(RESERVED_NAME)).unwrap(),
                b"occupied test object"
            );
            assert_eq!(fs::read(leaf.join(NAME)).unwrap(), retained);
            busy(root);
        });
    });
}

#[test]
fn collector_reserved_independent_owners_do_not_share_refusal() {
    let tasks: Vec<_> = (0..2)
        .map(|case| {
            std::thread::spawn(move || {
                with_original(case == 0, |original, _| {
                    let request = original.original.original.request.clone();
                    with_owned_empty_fence_for_test(|fence, root, leaf| {
                        let attempt = original
                            .bind_common_empty_fence(fence)
                            .unwrap()
                            .retain_request()
                            .unwrap();
                        let retained = fs::read(leaf.join(NAME)).unwrap();
                        let mut observed = attempt.into_observed_reservation();
                        let state = observed.state_to_sign().unwrap();
                        let bytes = signed(&body(&request, &state, &retained));
                        if case == 0 {
                            let failure = observed.publish_reserved(b"invalid").unwrap_err();
                            assert!(failure.is_refused());
                            busy(root);
                        } else {
                            let mut read = observed.publish_reserved(&bytes).unwrap();
                            read.revalidate().unwrap();
                            assert!(!read.is_refused());
                            busy(root);
                        }
                        assert_eq!(fs::read(leaf.join(NAME)).unwrap(), retained);
                    });
                });
            })
        })
        .collect();
    for task in tasks {
        task.join().unwrap();
    }
}
