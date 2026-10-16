use super::super::super::super::fenced_preflight::request_attempt::collector_preflight::OriginalCollectorError;
use super::*;
use crate::diagnostic_lease::{DiagnosticStage, DiagnosticState};
use crate::diagnostic_worker_policy::intent_projection::{
    collector::*, IntentProjectionBody, IntentProjectionError,
};
use ed25519_dalek::{Signer, SigningKey};

fn state(request: &crate::diagnostic_ipc::DiagnosticRequest) -> DiagnosticState {
    DiagnosticState {
        identity: request.identity.clone(),
        revision: 1,
        stage: DiagnosticStage::Reserved,
        completed_cells: 0,
        reserved_monotonic_ns: 1,
        last_observed_monotonic_ns: 1,
        controller_monotonic_ns: 1,
        cell_started_monotonic_ns: None,
        cgroup_inode: None,
        reason: None,
        cleanup_confirmed: true,
        promotable: false,
        admission_allowed: false,
    }
}
fn body(
    request: &crate::diagnostic_ipc::DiagnosticRequest,
    state: &DiagnosticState,
    retained: &[u8],
) -> IntentProjectionBody {
    let raw: Value = serde_json::from_slice(retained).unwrap();
    let p = project_collector_reserved_state(state).unwrap();
    IntentProjectionBody {
        schema_version: COLLECTOR_INTENT_SCHEMA.into(),
        request_id: request.request_id.clone(),
        nonce_sha256: request.nonce_sha256.clone(),
        controller: request.controller.clone(),
        issued_at_unix_seconds: request.issued_at_unix_seconds,
        expires_at_unix_seconds: request.expires_at_unix_seconds,
        state_sha256: p.state_sha256,
        unit_spec_sha256: p.unit_spec_sha256,
        worker_policy_sha256: raw["worker_policy_sha256"].as_str().unwrap().into(),
        worker_policy_epoch: raw["policy_epoch"].as_u64().unwrap(),
        boot_id: request.identity.boot_id.clone(),
    }
}
fn signed(body: &IntentProjectionBody) -> Vec<u8> {
    line(&SignedCollectorIntentProjection {
        schema_version: SIGNED_COLLECTOR_INTENT_SCHEMA.into(),
        intent: body.clone(),
        signature_hex: crate::hex(
            &SigningKey::from_bytes(&[76; 32])
                .sign(&collector_intent_signing_message(body).unwrap())
                .to_bytes(),
        ),
    })
}

#[test]
fn original_collector_keeps_original_kernel_chain_lock_and_bytes() {
    with_original(false, |original, _| {
        let request = original.original.original.request.clone();
        let state = state(&request);
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let mut attempt = original
                .bind_common_empty_fence(fence)
                .unwrap()
                .retain_request()
                .unwrap();
            let retained = fs::read(leaf.join(NAME)).unwrap();
            let bytes = signed(&body(&request, &state, &retained));
            let mut guard = attempt.bind_collector_requirements(&bytes, &state).unwrap();
            guard.revalidate().unwrap();
            assert!(!guard.is_refused());
            busy(root);
            assert!(!format!("{guard:?}").contains("request_id"));
            assert_eq!(fs::read_dir(leaf).unwrap().count(), 1);
            assert_eq!(fs::read(leaf.join(NAME)).unwrap(), retained);
            drop(guard);
            attempt.revalidate().unwrap();
            busy(root);
            drop(attempt);
            retained_context_refuses_for_test(root);
        });
    });
}

#[test]
fn original_collector_seeded_signed_request_and_state_substitution_refuse() {
    let mut cases: [u8; 11] = std::array::from_fn(|i| i as u8);
    let mut seed = 0x0740c098_u64;
    for i in (1..cases.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        cases.swap(i, seed as usize % (i + 1));
    }
    for case in cases {
        with_original(true, |original, _| {
            let request = original.original.original.request.clone();
            let mut state = state(&request);
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let mut attempt = original
                    .bind_common_empty_fence(fence)
                    .unwrap()
                    .retain_request()
                    .unwrap();
                let retained = fs::read(leaf.join(NAME)).unwrap();
                let mut b = body(&request, &state, &retained);
                match case {
                    0 => b.request_id = "e".repeat(64),
                    1 => b.nonce_sha256 = "f".repeat(64),
                    2 => b.controller.run_id += 1,
                    3 => b.issued_at_unix_seconds -= 1,
                    4 => b.expires_at_unix_seconds -= 1,
                    5 => {
                        state.identity.lease_id = "e".repeat(64);
                        b = body(&request, &state, &retained);
                    }
                    6 => {
                        state.revision = 2;
                        b = body(&request, &state, &retained);
                    }
                    7 => {
                        state.completed_cells = 1;
                        b = body(&request, &state, &retained);
                    }
                    8 => b.worker_policy_epoch += 1,
                    9 => b.unit_spec_sha256 = "a".repeat(64),
                    _ => {
                        b.issued_at_unix_seconds = 1;
                        b.expires_at_unix_seconds = 2;
                    }
                }
                let failure = attempt
                    .bind_collector_requirements(&signed(&b), &state)
                    .unwrap_err();
                if case < 8 {
                    if case == 3 {
                        assert!(matches!(
                            failure,
                            OriginalCollectorError::Input(TrustInputError::Collector(
                                IntentProjectionError::Time
                            ))
                        ));
                    } else {
                        assert!(matches!(failure, OriginalCollectorError::Binding));
                    }
                } else if case == 10 {
                    assert!(matches!(
                        failure,
                        OriginalCollectorError::Input(TrustInputError::Collector(
                            IntentProjectionError::Time
                        ))
                    ));
                } else {
                    assert!(matches!(
                        failure,
                        OriginalCollectorError::Input(TrustInputError::Collector(
                            IntentProjectionError::Binding
                        ))
                    ));
                }
                assert!(attempt.is_refused());
                busy(root);
                assert_eq!(fs::read(leaf.join(NAME)).unwrap(), retained);
                // An unbounded replacement would be rejected without parsing or I/O.
                assert!(matches!(
                    attempt.bind_collector_requirements(&vec![0; 8193], &state),
                    Err(OriginalCollectorError::Refused)
                ));
                drop(attempt);
                retained_context_refuses_for_test(root);
            });
        });
    }
    println!("original_collector seed=0x0740c098 signed_mutations=11 peer_time_assertions=false output_creation=false");
}

#[test]
fn original_collector_pre_refused_attempt_does_not_reopen_missing_record() {
    with_original(true, |original, _| {
        let request = original.original.original.request.clone();
        let state = state(&request);
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let mut attempt = original
                .bind_common_empty_fence(fence)
                .unwrap()
                .retain_request()
                .unwrap();
            attempt.simulate_binding_drift_for_test();
            assert!(attempt.revalidate().is_err());
            fs::remove_file(leaf.join(NAME)).unwrap();
            assert!(matches!(
                attempt.bind_collector_requirements(&vec![0; 8193], &state),
                Err(OriginalCollectorError::Refused)
            ));
            assert!(attempt.is_refused());
            busy(root);
            assert_eq!(fs::read_dir(leaf).unwrap().count(), 0);
        });
    });
}

#[test]
fn original_collector_malformed_legacy_and_overbudget_preserve_attempt() {
    for case in 0..3 {
        with_original(true, |original, _| {
            let request = original.original.original.request.clone();
            let state = state(&request);
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let mut attempt = original
                    .bind_common_empty_fence(fence)
                    .unwrap()
                    .retain_request()
                    .unwrap();
                let retained = fs::read(leaf.join(NAME)).unwrap();
                let bytes = match case {
                    0 => vec![],
                    1 => vec![b' '; 8193],
                    _ => {
                        let mut b = body(&request, &state, &retained);
                        b.schema_version =
                            crate::diagnostic_worker_policy::intent_projection::INTENT_SCHEMA
                                .into();
                        line(&crate::diagnostic_worker_policy::intent_projection::SignedIntentProjection {schema_version:crate::diagnostic_worker_policy::intent_projection::SIGNED_INTENT_SCHEMA.into(),signature_hex:crate::hex(&SigningKey::from_bytes(&[76;32]).sign(&crate::diagnostic_worker_policy::intent_projection::intent_signing_message(&b).unwrap()).to_bytes()),intent:b})
                    }
                };
                assert!(matches!(
                    attempt.bind_collector_requirements(&bytes, &state),
                    Err(OriginalCollectorError::Input(TrustInputError::Collector(
                        IntentProjectionError::Document
                    )))
                ));
                assert!(attempt.is_refused());
                busy(root);
                assert_eq!(fs::read(leaf.join(NAME)).unwrap(), retained);
            });
        });
    }
}

#[test]
fn original_collector_late_source_and_storage_drift_are_sticky_without_retry_io() {
    for case in 0..3 {
        with_original(true, |original, paths| {
            let request = original.original.original.request.clone();
            let state = state(&request);
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let mut attempt = original
                    .bind_common_empty_fence(fence)
                    .unwrap()
                    .retain_request()
                    .unwrap();
                let retained = fs::read(leaf.join(NAME)).unwrap();
                let bytes = signed(&body(&request, &state, &retained));
                let mut guard = attempt.bind_collector_requirements(&bytes, &state).unwrap();
                let path = match case {
                    0 => paths.worker.clone(),
                    1 => paths.oidc.clone(),
                    _ => leaf.join("foreign"),
                };
                let saved = if case < 2 {
                    Some(fs::read(&path).unwrap())
                } else {
                    None
                };
                fs::write(&path, b"public test drift").unwrap();
                assert!(guard.revalidate().is_err());
                assert!(guard.is_refused());
                if let Some(raw) = saved {
                    fs::write(&path, raw).unwrap();
                } else {
                    fs::remove_file(&path).unwrap();
                }
                // Delete the named attempt: a refused observation must not try reopening it.
                fs::remove_file(leaf.join(NAME)).unwrap();
                assert!(matches!(
                    guard.revalidate(),
                    Err(OriginalCollectorError::Refused)
                ));
                busy(root);
                drop(guard);
                assert!(attempt.is_refused());
                // Restore only the disposable fixture, never through the runtime API.
                fs::write(leaf.join(NAME), &retained).unwrap();
            });
        });
    }
}

#[test]
fn original_collector_independent_chains_keep_refusal_local() {
    let tasks: Vec<_> = (0..2)
        .map(|index| {
            std::thread::spawn(move || {
                with_original(index == 0, |original, _| {
                    let request = original.original.original.request.clone();
                    let state = state(&request);
                    with_owned_empty_fence_for_test(|fence, root, leaf| {
                        let mut attempt = original
                            .bind_common_empty_fence(fence)
                            .unwrap()
                            .retain_request()
                            .unwrap();
                        let retained = fs::read(leaf.join(NAME)).unwrap();
                        let bytes = signed(&body(&request, &state, &retained));
                        if index == 0 {
                            assert!(attempt
                                .bind_collector_requirements(b"invalid", &state)
                                .is_err());
                        } else {
                            attempt
                                .bind_collector_requirements(&bytes, &state)
                                .unwrap()
                                .revalidate()
                                .unwrap();
                        }
                        assert_eq!(attempt.is_refused(), index == 0);
                        busy(root);
                        assert_eq!(fs::read(leaf.join(NAME)).unwrap(), retained);
                    });
                })
            })
        })
        .collect();
    for task in tasks {
        task.join().unwrap();
    }
}
