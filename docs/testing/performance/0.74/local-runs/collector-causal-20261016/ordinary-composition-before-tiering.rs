use super::*;
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::CollectorProgressError;
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::*;

#[test]
fn collector_causal_real_response_prequeued_refusal_and_entropy_failure_keep_fence() {
    for case in 0..4 {
        with_progress_original(true, |original, _, receiver, sender, model| {
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
                let reservation = observed
                    .publish_reserved(&signed(&body(&request, &state, &request_raw)))
                    .unwrap();
                let reserved_raw = fs::read(leaf.join(RESERVED_NAME)).unwrap();
                let mut next = request.clone();
                next.operation = DiagnosticOperation::Heartbeat;
                next.expected_state_revision = 1;
                next.request_id = "f".repeat(64);
                next.nonce_sha256 = "0".repeat(64);
                if case == 1 {
                    // Actually enqueue a valid old-controller signature before nonce generation.
                    let bytes = model.sign(next.clone(), &model.key);
                    assert_eq!(
                        unsafe {
                            libc::send(
                                sender.as_raw_fd(),
                                bytes.as_ptr().cast(),
                                bytes.len(),
                                libc::MSG_NOSIGNAL,
                            )
                        },
                        bytes.len() as isize
                    );
                }
                if case == 2 {
                    let failure = reservation
                        .issue_progress_challenge_with(Err(CollectorCausalError::Entropy))
                        .unwrap_err();
                    assert!(matches!(failure.error(), CollectorCausalError::Entropy));
                    assert!(failure.is_refused());
                    busy(root);
                    drop(failure);
                } else {
                    let challenge = reservation.issue_progress_challenge().unwrap();
                    let raw = challenge.challenge_to_sign().to_vec();
                    let nonce = collector_challenge_nonce(&raw).unwrap();
                    let doc: Value = serde_json::from_slice(&raw).unwrap();
                    assert_eq!(
                        doc["controller_attempt_sha256"],
                        crate::sha256_hex(&request_raw)
                    );
                    assert_eq!(
                        doc["collector_reserved_sha256"],
                        crate::sha256_hex(&reserved_raw)
                    );
                    assert_eq!(
                        doc["reserved_state_sha256"],
                        crate::sha256_hex(&crate::canonical_json(&state).unwrap())
                    );
                    assert_eq!(
                        doc["identity"],
                        serde_json::to_value(&state.identity).unwrap()
                    );
                    let mut copy = raw.clone();
                    copy[0] ^= 1;
                    assert_ne!(copy, challenge.challenge_to_sign());
                    assert_eq!(challenge.challenge_to_sign(), raw);
                    assert!(!format!("{challenge:?}").contains("random_hex"));
                    busy(root);
                    if case == 3 {
                        // Data remains readable after expiry but cannot become fresh authority.
                        std::thread::sleep(std::time::Duration::from_secs(
                            crate::diagnostic_lease::CONTROLLER_LOSS_SECONDS,
                        ));
                        assert_eq!(
                            collector_challenge_nonce(challenge.challenge_to_sign()).unwrap(),
                            nonce
                        );
                    }
                    let packet = if case == 1 {
                        // Dequeue only after issuance; original receive-order checks alone pass.
                        assert_ne!(next.nonce_sha256, nonce);
                        receiver.try_receive().unwrap().unwrap()
                    } else {
                        next.nonce_sha256 = nonce;
                        receive(receiver, sender, &model.sign(next, &model.key))
                    };
                    let result = challenge.accept_response(packet);
                    if case == 1 || case == 3 {
                        let failure = result.unwrap_err();
                        if case == 1 {
                            assert!(matches!(failure.error(), CollectorCausalError::Binding));
                        } else {
                            assert!(matches!(failure.error(), CollectorCausalError::Time));
                        }
                        assert!(failure.is_refused());
                        busy(root);
                        drop(failure);
                    } else {
                        let mut causal = result.unwrap();
                        assert!(!causal.is_refused());
                        assert!(!format!("{causal:?}").contains("random_hex"));
                        assert_eq!(fs::read(leaf.join(NAME)).unwrap(), request_raw);
                        assert_eq!(fs::read(leaf.join(RESERVED_NAME)).unwrap(), reserved_raw);
                        busy(root);
                        fs::remove_file(leaf.join(RESERVED_NAME)).unwrap();
                        assert!(matches!(
                            causal.revalidate(),
                            Err(CollectorCausalError::Progress(
                                CollectorProgressError::Reservation(
                                    CollectorReservedError::Storage(_)
                                )
                            ))
                        ));
                        assert!(causal.is_refused());
                        assert!(matches!(
                            causal.revalidate(),
                            Err(CollectorCausalError::Refused)
                        ));
                        busy(root);
                        drop(causal);
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
                println!("collector_causal live_case={case} original_fence=true journal_writes=false starting=false");
            });
        });
    }
}
#[test]
fn collector_causal_api_shape() {
    fn probe(owner: OriginalCollectorReservation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
        let _ = owner.issue_progress_challenge();
    }
    let _ = probe;
}
