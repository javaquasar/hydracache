use super::*;
#[path = "diagnostic_original_collector_tests.rs"]
mod collector_tests;
#[path = "diagnostic_controller_attempt_crash_tests.rs"]
mod process_crash_tests;
use super::super::super::fenced_preflight::request_attempt::{
    validate_retained_controller_attempt, AttemptStage, ControllerAttemptError, StorageAttemptError,
};
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::{
    attempt_storage::AttemptStorage, retained_context_refuses_for_test,
};

const NAME: &str = "diagnostic-controller-attempt-074.json";

#[test]
fn original_collector_preflight_api_shape() {
    use super::super::super::fenced_preflight::request_attempt::ControllerRequestAttempt;
    fn probe(
        attempt: &mut ControllerRequestAttempt<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
        state: &crate::diagnostic_lease::DiagnosticState,
    ) {
        let _ = attempt.bind_collector_requirements(b"", state);
    }
    let _ = probe;
}
const STAGES: [AttemptStage; 8] = [
    AttemptStage::BeforeCreate,
    AttemptStage::Created,
    AttemptStage::PrefixWritten,
    AttemptStage::BytesWritten,
    AttemptStage::Sealed,
    AttemptStage::FileSynced,
    AttemptStage::DirectorySynced,
    AttemptStage::Acknowledging,
];

#[test]
fn controller_attempt_original_capture_mismatch_preserves_retained_bytes() {
    with_original(true, |original, _| {
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let mut attempt = original
                .bind_common_empty_fence(fence)
                .unwrap()
                .retain_request()
                .unwrap();
            let raw = fs::read(leaf.join(NAME)).unwrap();
            attempt.simulate_binding_drift_for_test();
            assert!(matches!(
                attempt.revalidate(),
                Err(ControllerAttemptError::Binding)
            ));
            assert!(attempt.is_refused());
            busy(root);
            assert_eq!(fs::read(leaf.join(NAME)).unwrap(), raw);
            assert!(matches!(
                attempt.revalidate(),
                Err(ControllerAttemptError::Refused)
            ));
        });
    });
}

#[test]
fn controller_attempt_signed_nonreserve_refuses_before_first_write() {
    for operation in [
        crate::diagnostic_ipc::DiagnosticOperation::Status,
        crate::diagnostic_ipc::DiagnosticOperation::Heartbeat,
        crate::diagnostic_ipc::DiagnosticOperation::Cancel,
    ] {
        with_original_operation(operation, true, |original, _| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let preflight = original.bind_common_empty_fence(fence).unwrap();
                let failure = preflight
                    .retain_request_with(|_| panic!("unsupported operation reached mutation"))
                    .unwrap_err();
                assert!(matches!(failure.error(), ControllerAttemptError::Document));
                assert!(failure.is_refused());
                busy(root);
                assert_eq!(fs::read_dir(leaf).unwrap().count(), 0);
            });
        });
    }
}

#[test]
fn controller_attempt_retains_original_request_and_lock() {
    with_original(false, |original, _| {
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let preflight = original.bind_common_empty_fence(fence).unwrap();
            let mut attempt = preflight.retain_request().unwrap();
            attempt.revalidate().unwrap();
            busy(root);
            assert_eq!(fs::read_dir(leaf).unwrap().count(), 1);
            let file = leaf.join(NAME);
            let raw = fs::read(&file).unwrap();
            validate_retained_controller_attempt(&raw).unwrap();
            assert_eq!(raw.last(), Some(&b'\n'));
            assert_eq!(
                fs::metadata(&file).unwrap().permissions().mode() & 0o7777,
                0o400
            );
            let document: Value = serde_json::from_slice(&raw).unwrap();
            assert_eq!(
                document["schema_version"],
                "diagnostic-controller-attempt-074-v1"
            );
            assert_eq!(document["authority_revision"], 1);
            assert_eq!(document["policy_epoch"], 7);
            assert!(document.get("token").is_none());
            assert!(document.get("signature_hex").is_none());
            assert_eq!(
                document["request_sha256"],
                crate::sha256_hex(&crate::canonical_json(&document["request"]).unwrap())
            );
            assert_eq!(document["build_receipt_sha256"], crate::sha256_hex(RECEIPT));
            assert!(!format!("{attempt:?}").contains("request_id"));
            drop(attempt);
            original.revalidate().unwrap();
            drop(crate::host_execution::lock_host_root(root).unwrap());
            retained_context_refuses_for_test(root);
        });
    });
}

#[test]
fn controller_attempt_seeded_all_logical_write_faults_keep_lock_and_partial_context() {
    let mut order = STAGES;
    let mut seed = 0x0740_c094_u64;
    for i in (1..order.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        order.swap(i, seed as usize % (i + 1));
    }
    for target in order {
        with_original(true, |original, _| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let preflight = original.bind_common_empty_fence(fence).unwrap();
                let mut visited = Vec::new();
                let failure = preflight
                    .retain_request_with(|at| {
                        visited.push(at);
                        if at == target {
                            Err(ControllerAttemptError::Storage(
                                StorageAttemptError::Interrupted(at),
                            ))
                        } else {
                            Ok(())
                        }
                    })
                    .unwrap_err();
                assert!(
                    matches!(failure.error(), ControllerAttemptError::Storage(StorageAttemptError::Interrupted(at)) if *at == target)
                );
                assert_eq!(visited.last(), Some(&target));
                assert_eq!(visited, STAGES[..visited.len()]);
                assert!(failure.is_refused() && original.is_refused());
                busy(root);
                let file = leaf.join(NAME);
                if target == AttemptStage::BeforeCreate {
                    assert!(!file.exists());
                } else {
                    let raw = fs::read(&file).unwrap();
                    match target {
                        AttemptStage::Created => assert!(raw.is_empty()),
                        AttemptStage::PrefixWritten => {
                            assert!(validate_retained_controller_attempt(&raw).is_err())
                        }
                        _ => validate_retained_controller_attempt(&raw).unwrap(),
                    }
                    assert_eq!(fs::read_dir(leaf).unwrap().count(), 1);
                }
                drop(failure);
                if target != AttemptStage::BeforeCreate {
                    retained_context_refuses_for_test(root);
                }
                assert!(original.revalidate().is_err());
            });
        });
    }
    println!("controller_attempt_seed=0x0740c094 logical_boundaries=8 sigkill_proof=false");
}

#[test]
fn controller_attempt_trust_drift_at_each_write_boundary_cross_refuses() {
    for target in STAGES {
        with_original(true, |original, paths| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let preflight = original.bind_common_empty_fence(fence).unwrap();
                let saved = fs::read(&paths.oidc).unwrap();
                let mut visited = Vec::new();
                let failure = preflight
                    .retain_request_with(|at| {
                        visited.push(at);
                        if at == target {
                            fs::write(&paths.oidc, b"changed configured source\n").unwrap();
                        }
                        Ok(())
                    })
                    .unwrap_err();
                assert!(matches!(
                    failure.error(),
                    ControllerAttemptError::Controller(_)
                ));
                assert_eq!(visited.last(), Some(&target));
                assert!(failure.is_refused());
                busy(root);
                fs::write(&paths.oidc, saved).unwrap();
                drop(failure);
                assert!(original.revalidate().is_err());
                if target != AttemptStage::BeforeCreate {
                    assert!(leaf.join(NAME).exists());
                    retained_context_refuses_for_test(root);
                }
            });
        });
    }
}

#[test]
fn controller_attempt_late_owned_object_and_namespace_drift_stay_refused() {
    for case in 0..8 {
        with_original(true, |original, paths| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let preflight = original.bind_common_empty_fence(fence).unwrap();
                let mut attempt = preflight.retain_request().unwrap();
                let file = leaf.join(NAME);
                let raw = fs::read(&file).unwrap();
                match case {
                    0 => {
                        fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
                        fs::write(&file, b"drift").unwrap();
                        fs::set_permissions(&file, fs::Permissions::from_mode(0o400)).unwrap();
                    }
                    1 => fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap(),
                    2 => {
                        fs::rename(&file, file.with_extension("old")).unwrap();
                        fs::write(&file, &raw).unwrap();
                        fs::set_permissions(&file, fs::Permissions::from_mode(0o400)).unwrap();
                    }
                    3 => fs::hard_link(&file, leaf.join("alias")).unwrap(),
                    4 => fs::write(leaf.join("foreign"), b"context").unwrap(),
                    5 => fs::write(root.join("active-campaign"), b"context").unwrap(),
                    6 => fs::write(&paths.worker, b"changed policy input").unwrap(),
                    _ => fs::set_permissions(&paths.authority, fs::Permissions::from_mode(0o750))
                        .unwrap(),
                }
                assert!(attempt.revalidate().is_err());
                assert!(attempt.is_refused());
                busy(root);
                // Even inaccessible objects after refusal must not be inspected again.
                fs::rename(leaf, leaf.with_extension("retained")).unwrap();
                assert!(matches!(
                    attempt.revalidate(),
                    Err(ControllerAttemptError::Refused)
                ));
                drop(attempt);
                assert!(original.is_refused());
                fs::set_permissions(&paths.authority, fs::Permissions::from_mode(0o700)).unwrap();
            });
        });
    }
}

#[test]
fn controller_attempt_canonical_reader_refuses_mutations_not_admission() {
    with_original(false, |original, _| {
        with_owned_empty_fence_for_test(|fence, _, leaf| {
            let attempt = original
                .bind_common_empty_fence(fence)
                .unwrap()
                .retain_request()
                .unwrap();
            let raw = fs::read(leaf.join(NAME)).unwrap();
            let document: Value = serde_json::from_slice(&raw).unwrap();
            let mut cases = vec![
                Vec::new(),
                vec![b'x'; 16_385],
                raw[..raw.len() - 1].to_vec(),
                [raw.as_slice(), b"\n"].concat(),
            ];
            for (field, value) in [
                (
                    "schema_version",
                    json!("diagnostic-controller-attempt-074-v2"),
                ),
                ("policy_epoch", json!(0)),
                ("authority_revision", json!(0)),
                ("authority_revision", json!(129)),
                ("request_sha256", json!("0".repeat(64))),
                ("token_sha256", json!("bad")),
                ("worker_policy_sha256", json!("bad")),
                ("builder_policy_sha256", json!("bad")),
                ("worker_trust_sha256", json!("bad")),
                ("controller_trust_sha256", json!("bad")),
                ("oidc_trust_sha256", json!("bad")),
                ("build_receipt_sha256", json!("0".repeat(64))),
                ("extra", json!(true)),
            ] {
                let mut changed = document.clone();
                changed[field] = value;
                cases.push(line(&changed));
            }
            let mut changed = document.clone();
            changed["request"]["operation"] = json!("status");
            changed["request"]["expected_state_revision"] = json!(1);
            changed["request_sha256"] = json!(crate::sha256_hex(
                &crate::canonical_json(&changed["request"]).unwrap()
            ));
            cases.push(line(&changed));
            let mut duplicate = raw.clone();
            duplicate.pop();
            duplicate.pop();
            duplicate.extend_from_slice(b",\"policy_epoch\":7}\n");
            cases.push(duplicate);
            for bad in cases {
                assert!(validate_retained_controller_attempt(&bad).is_err());
            }
            // Valid bytes still cannot return the runtime guard; this validator returns ().
            validate_retained_controller_attempt(&raw).unwrap();
            drop(attempt);
        });
    });
}

#[test]
fn controller_attempt_engine_cannot_convert_to_legacy_intent_publication() {
    with_owned_empty_fence_for_test(|fence, root, leaf| {
        let mut storage = AttemptStorage::for_controller(fence);
        storage
            .retain_with::<StorageAttemptError>(b"test-only-bytes", |_| Ok(()))
            .unwrap();
        let failed = storage
            .into_publication()
            .err()
            .expect("controller document must not become legacy intent");
        busy(root);
        assert!(!failed.is_refused());
        assert_eq!(fs::read(leaf.join(NAME)).unwrap(), b"test-only-bytes");
        drop(failed);
        retained_context_refuses_for_test(root);
    });
}

#[test]
fn controller_attempt_pre_refused_original_does_no_write_or_checkpoint() {
    with_original(true, |original, _| {
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let preflight = original.bind_common_empty_fence(fence).unwrap();
            fs::write(root.join("active-campaign"), b"late context").unwrap();
            let failure = preflight
                .retain_request_with(|_| panic!("refused input observed write checkpoint"))
                .unwrap_err();
            assert!(matches!(
                failure.error(),
                ControllerAttemptError::Preflight(_)
            ));
            assert!(failure.is_refused());
            busy(root);
            assert_eq!(fs::read_dir(leaf).unwrap().count(), 0);
        });
    });
}

#[test]
fn controller_attempt_independent_concurrent_originals_remain_isolated() {
    let handles: Vec<_> = (0..4)
        .map(|i| {
            std::thread::spawn(move || {
                let refused = i % 2 == 0;
                with_original(refused, |original, _| {
                    with_owned_empty_fence_for_test(|fence, root, leaf| {
                        let preflight = original.bind_common_empty_fence(fence).unwrap();
                        let mut attempt = preflight.retain_request().unwrap();
                        if refused {
                            fs::write(leaf.join("foreign"), b"fault").unwrap();
                        }
                        assert_eq!(attempt.revalidate().is_err(), refused);
                        busy(root);
                    });
                });
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join()).collect();
    for result in results {
        result.unwrap();
    }
}
