use super::*;
use crate::diagnostic_start_consent::{start_consent_signing_message, SignedStartConsent, ENVELOPE_SCHEMA};
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::*;
use ed25519_dalek::Signer;
#[path = "diagnostic_consent_budget_tests.rs"]
mod budget_tests;

fn consent(
    body: crate::diagnostic_start_consent::StartConsentBody,
    key: &ed25519_dalek::SigningKey,
) -> Vec<u8> {
    let signature_hex = crate::hex(
        &key.sign(&start_consent_signing_message(&body).unwrap())
            .to_bytes(),
    );
    let mut raw = crate::canonical_json(&SignedStartConsent {
        schema_version: ENVELOPE_SCHEMA.into(),
        consent: body,
        signature_hex,
    })
    .unwrap();
    raw.push(b'\n');
    raw
}
#[test]
#[ignore = "manual nonroot original kernel start consent; no execution"]
fn original_start_consent_live_bindings_and_refusal_keep_fence() {
    for case in 0..4 {
        with_progress_original(true, |original, paths, receiver, sender, model| {
            let request = original.original.original.request.clone();
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let attempt = original
                    .bind_common_empty_fence(fence)
                    .unwrap()
                    .retain_request()
                    .unwrap();
                let request_raw = fs::read(leaf.join(NAME)).unwrap();
                let mut observed = attempt.into_observed_reservation();
                let state = observed.state_to_sign().unwrap();
                let reserved = observed
                    .publish_reserved(&signed(&body(&request, &state, &request_raw)))
                    .unwrap();
                let reserved_raw = fs::read(leaf.join(RESERVED_NAME)).unwrap();
                let challenge = reserved.issue_progress_challenge().unwrap();
                let raw_challenge = challenge.challenge_to_sign().to_vec();
                let nonce=crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::collector_challenge_nonce(&raw_challenge).unwrap();
                let mut response = request.clone();
                response.operation = DiagnosticOperation::Heartbeat;
                response.expected_state_revision = 1;
                response.request_id = "f".repeat(64);
                response.nonce_sha256 = nonce.clone();
                let response_raw = model.sign(response.clone(), &model.key);
                let causal = challenge
                    .accept_response(receive(receiver, sender, &response_raw))
                    .unwrap();
                let id = if case == 2 {
                    response.request_id
                } else {
                    "1".repeat(64)
                };
                let prepared = causal.prepare_start_consent(id);
                if case == 2 {
                    let failure = prepared.unwrap_err();
                    assert!(matches!(
                        failure.error(),
                        OriginalStartConsentError::Binding
                    ));
                    assert!(failure.is_refused());
                    busy(root);
                    drop(failure);
                } else {
                    let prepared = prepared.unwrap();
                    let mut b = prepared.consent_to_sign().clone();
                    assert_eq!(b.reservation_request_id, request.request_id);
                    assert_eq!(b.controller, request.controller);
                    assert_eq!(b.controller_attempt_sha256, crate::sha256_hex(&request_raw));
                    assert_eq!(
                        b.collector_reserved_sha256,
                        crate::sha256_hex(&reserved_raw)
                    );
                    assert_eq!(
                        b.reserved_state_sha256,
                        crate::sha256_hex(&crate::canonical_json(&state).unwrap())
                    );
                    assert_eq!(b.challenge_nonce_sha256, nonce);
                    assert_eq!(
                        b.authenticated_response_sha256,
                        crate::sha256_hex(&response_raw)
                    );
                    assert_eq!(b.collector_unit_sha256,crate::sha256_hex(&crate::diagnostic_worker_policy::intent_projection::collector::collector_unit_projection(&state).unwrap()));
                    let retained: Value = serde_json::from_slice(&request_raw).unwrap();
                    assert_eq!(b.worker_policy_sha256, retained["worker_policy_sha256"]);
                    assert_eq!(b.worker_policy_epoch, retained["policy_epoch"]);
                    assert_eq!(
                        b.starting_state.reserved_monotonic_ns,
                        state.reserved_monotonic_ns
                    );
                    assert_eq!(
                        b.starting_state.controller_monotonic_ns,
                        b.starting_state.last_observed_monotonic_ns
                    );
                    if case == 1 {
                        b.collector_reserved_sha256 = "0".repeat(64);
                    }
                    if case == 3 {
                        fs::write(&paths.worker, b"changed original trust").unwrap();
                    }
                    let packet = receive(receiver, sender, &consent(b, &model.key));
                    let result = prepared.accept_consent(packet);
                    if case == 0 {
                        let mut accepted = result.unwrap();
                        accepted.revalidate().unwrap();
                        assert!(!accepted.is_refused());
                        busy(root);
                        assert!(!format!("{accepted:?}").contains("consent_request_id"));
                        assert_eq!(fs::read_dir(leaf).unwrap().count(), 2);
                        fs::remove_file(leaf.join(RESERVED_NAME)).unwrap();
                        assert!(accepted.revalidate().is_err());
                        assert!(accepted.is_refused());
                        assert!(matches!(
                            accepted.revalidate(),
                            Err(OriginalStartConsentError::Refused)
                        ));
                        busy(root);
                        drop(accepted);
                    } else {
                        let failure = result.unwrap_err();
                        if case == 1 {
                            assert!(matches!(
                                failure.error(),
                                OriginalStartConsentError::Binding
                            ));
                        }
                        assert!(failure.is_refused());
                        busy(root);
                        drop(failure);
                    }
                }
                assert_eq!(fs::read(leaf.join(NAME)).unwrap(), request_raw);
                if case != 0 {
                    assert_eq!(fs::read(leaf.join(RESERVED_NAME)).unwrap(), reserved_raw);
                }
                assert_eq!(
                    fs::read_dir(leaf).unwrap().count(),
                    if case == 0 { 1 } else { 2 }
                );
                drop(crate::host_execution::lock_host_root(root).unwrap());
                retained_context_refuses_for_test(root);
                println!("original_start_consent live_case={case} original_fence=true durable_starting=false process_start=false");
            });
        });
    }
}
