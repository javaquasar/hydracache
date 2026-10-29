use super::*;
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::OriginalCausalProgress;
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::preconsent::PreConsentStep;

fn with_causal(
    refused: bool,
    run: impl FnOnce(
        OriginalCausalProgress<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
        &Paths,
        &std::path::Path,
        &std::path::Path,
        &mut KernelReceiver,
        &OwnedFd,
        &Model,
    ),
) {
    with_progress_original(refused, |original, paths, receiver, sender, model| {
        let request = original.original.original.request.clone();
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let attempt = original
                .bind_common_empty_fence(fence)
                .unwrap()
                .retain_request()
                .unwrap();
            let raw = fs::read(leaf.join(NAME)).unwrap();
            let mut observed = attempt.into_observed_reservation();
            let state = observed.state_to_sign().unwrap();
            let reserved = observed
                .publish_reserved(&signed(&body(&request, &state, &raw)))
                .unwrap();
            let challenge = reserved.issue_progress_challenge().unwrap();
            let nonce = crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::collector_challenge_nonce(challenge.challenge_to_sign()).unwrap();
            let mut response = request;
            response.operation = DiagnosticOperation::Heartbeat;
            response.expected_state_revision = 1;
            response.request_id = "f".repeat(64);
            response.nonce_sha256 = nonce;
            let causal = challenge
                .accept_response(receive(receiver, sender, &model.sign(response, &model.key)))
                .unwrap();
            run(causal, paths, root, leaf, receiver, sender, model);
            assert_eq!(fs::read(leaf.join(NAME)).unwrap(), raw);
            drop(crate::host_execution::lock_host_root(root).unwrap());
            retained_context_refuses_for_test(root);
        });
    });
}
fn drift(kind: usize, paths: &Paths, leaf: &std::path::Path, model: &Model) {
    match kind {
        0 => fs::write(&paths.worker, b"changed preconsent worker trust").unwrap(),
        1 => fs::write(&paths.oidc, b"changed preconsent OIDC trust").unwrap(),
        2 => {
            let bytes = fs::read(model.path()).unwrap();
            fs::write(model.path(), bytes).unwrap();
        }
        3 => fs::write(
            paths.authority.join("unexpected-preconsent-context"),
            b"invalid authority",
        )
        .unwrap(),
        4 => fs::remove_file(leaf.join(RESERVED_NAME)).unwrap(),
        _ => panic!("unknown private fault"),
    }
}
#[test]
#[ignore = "manual nonroot preconsent constant-depth live/reference controls; no publication"]
fn preconsent_frame_live_mutations_reference_and_consent_refusal() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    for cut in 0..7 {
        for kind in 0..5 {
            with_causal(true, |causal, paths, root, leaf, _, _, model| {
                let mut prepared = causal.prepare_framed_consent("1".repeat(64)).unwrap();
                let frozen = crate::canonical_json(prepared.consent_to_sign()).unwrap();
                let mut visited = 0;
                let result = prepared.revalidate_with_for_test(|_| {
                    if visited == cut {
                        drift(kind, paths, leaf, model);
                    }
                    visited += 1;
                });
                assert!(visited > cut && result.is_err() && prepared.is_refused());
                assert!(matches!(
                    prepared.revalidate(),
                    Err(OriginalStartConsentError::Refused)
                ));
                assert_eq!(
                    crate::canonical_json(prepared.consent_to_sign()).unwrap(),
                    frozen
                );
                busy(root);
                drop(prepared);
                println!("HC074-PRECONSENT-DRIFT cut={cut} kind={kind} frozen=true refused=true fence_retained=true");
            });
        }
    }
    for kind in 0..5 {
        with_causal(true, |causal, paths, root, leaf, _, _, model| {
            drift(kind, paths, leaf, model);
            let failure = causal.prepare_start_consent("1".repeat(64)).unwrap_err();
            assert!(failure.is_refused());
            busy(root);
            drop(failure);
            println!("HC074-PRECONSENT-REFERENCE kind={kind} refused=true");
        });
    }
    // Correctly signed but different body, invalid signature and a foreign receiver.
    for kind in 0..3 {
        with_causal(true, |causal, _, root, _, receiver, sender, model| {
            let prepared = causal.prepare_framed_consent("1".repeat(64)).unwrap();
            let mut body = prepared.consent_to_sign().clone();
            if kind == 0 {
                body.collector_reserved_sha256 = "0".repeat(64);
            }
            let mut bytes = consent(body, &model.key);
            if kind == 1 {
                let mut value: Value = serde_json::from_slice(&bytes).unwrap();
                value["signature_hex"] = "0".repeat(128).into();
                bytes = line(&value);
            }
            let packet = if kind == 2 {
                let (mut foreign, other) = pair();
                receive(&mut foreign, &other, &bytes)
            } else {
                receive(receiver, sender, &bytes)
            };
            let (failure, _) = crate::diagnostic_kernel_admission::refusal_trace::capture(|| {
                prepared.accept_consent(packet).unwrap_err()
            });
            assert!(failure.is_refused());
            busy(root);
            drop(failure);
            println!("HC074-PRECONSENT-CONSENT kind={kind} refused=true retained=true");
        });
    }
    with_causal(true, |mut causal, _, root, leaf, _, _, _| {
        fs::remove_file(leaf.join(RESERVED_NAME)).unwrap();
        assert!(causal.revalidate().is_err());
        let failure = causal.prepare_framed_consent("1".repeat(64)).unwrap_err();
        assert!(matches!(
            failure.error(),
            OriginalStartConsentError::Refused
        ));
        assert!(failure.is_refused());
        busy(root);
        drop(failure);
        println!("HC074-PRECONSENT-CONSTRUCTION pre_refused=true fence_retained=true");
    });
    for id in ["bad".to_owned(), "f".repeat(64)] {
        with_causal(true, |causal, _, root, _, _, _, _| {
            let failure = causal.prepare_framed_consent(id).unwrap_err();
            assert!(matches!(
                failure.error(),
                OriginalStartConsentError::Binding
            ));
            assert!(failure.is_refused());
            busy(root);
            drop(failure);
            println!("HC074-PRECONSENT-ID refused=true retained=true");
        });
    }
    with_causal(false, |causal, _, root, _, receiver, sender, model| {
        let mut prepared = causal.prepare_framed_consent("1".repeat(64)).unwrap();
        let frozen = crate::canonical_json(prepared.consent_to_sign()).unwrap();
        let mut order = Vec::new();
        prepared
            .revalidate_with_for_test(|at| order.push(format!("{at:?}")))
            .unwrap();
        assert_eq!(
            order,
            [
                "StorageBefore",
                "ReservedLive",
                "Progress",
                "Causal",
                "Prepared",
                "StorageAfter",
                "FinalClocks"
            ]
        );
        assert_eq!(
            crate::canonical_json(prepared.consent_to_sign()).unwrap(),
            frozen
        );
        let bytes = consent(prepared.consent_to_sign().clone(), &model.key);
        let mut framed = prepared
            .accept_consent(receive(receiver, sender, &bytes))
            .unwrap();
        framed.revalidate().unwrap();
        busy(root);
        drop(framed);
        println!("HC074-PRECONSENT-HEALTHY frozen=true ordered=true same_receiver=true");
    });
    with_causal(true, |causal, paths, root, _, _, _, _| {
        let mut prepared = causal.prepare_framed_consent("1".repeat(64)).unwrap();
        let path = paths.oidc.clone();
        let (go, wait) = std::sync::mpsc::channel();
        let (done, finished) = std::sync::mpsc::channel();
        let writer = std::thread::spawn(move || {
            if wait.recv().is_ok() {
                fs::write(path, b"concurrent closing drift").unwrap();
                done.send(()).unwrap();
            }
        });
        let mut go = Some(go);
        let mut visited = 0;
        let result = prepared.revalidate_with_for_test(|at| {
            visited += 1;
            if at == PreConsentStep::StorageAfter {
                go.take().unwrap().send(()).unwrap();
                finished.recv().unwrap();
            }
        });
        drop(go);
        writer.join().unwrap();
        assert_eq!(visited, 7);
        assert!(result.is_err() && prepared.is_refused());
        busy(root);
        drop(prepared);
        println!("HC074-PRECONSENT-CONCURRENT closing_round_refused=true fence_retained=true");
    });
}
