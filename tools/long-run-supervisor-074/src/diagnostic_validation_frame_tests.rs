use super::*;
#[path = "diagnostic_validation_frame_budget_tests.rs"]
mod budget;

#[test]
fn validation_frame_consuming_adapter_exists() {
    fn probe(owner: OriginalStartConsent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
        let _ = owner.into_validation_frame();
    }
    let _ = probe;
}

fn with_accepted(
    run: impl FnOnce(
        OriginalStartConsent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
        &Paths,
        &std::path::Path,
        &std::path::Path,
        &Model,
    ),
) {
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
            let prepared = causal.prepare_start_consent("1".repeat(64)).unwrap();
            let bytes = consent(prepared.consent_to_sign().clone(), &model.key);
            let accepted = prepared
                .accept_consent(receive(receiver, sender, &bytes))
                .unwrap();
            run(accepted, paths, root, leaf, model);
            assert_eq!(fs::read(leaf.join(NAME)).unwrap(), request_raw);
            drop(crate::host_execution::lock_host_root(root).unwrap());
            retained_context_refuses_for_test(root);
        });
    });
}
fn drift(kind: usize, paths: &Paths, leaf: &std::path::Path, model: &Model) {
    match kind {
        0 => fs::write(&paths.worker, b"changed worker trust").unwrap(),
        1 => fs::write(&paths.oidc, b"changed OIDC trust").unwrap(),
        2 => {
            let original = fs::read(model.path()).unwrap();
            fs::write(model.path(), original).unwrap();
        }
        3 => fs::write(
            paths.authority.join("unexpected-frame-context"),
            b"not authority",
        )
        .unwrap(),
        4 => fs::remove_file(leaf.join(RESERVED_NAME)).unwrap(),
        _ => panic!("unregistered fault"),
    }
}
#[test]
#[ignore = "manual nonroot framed original consent mutation/reference controls; no execution"]
fn validation_frame_live_drift_at_every_cut_and_reference_refusal() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    for cut in 0..8 {
        for kind in 0..5 {
            with_accepted(|accepted, paths, root, leaf, model| {
                let mut framed = accepted.into_validation_frame().unwrap();
                let mut visited = 0;
                let result = framed.revalidate_with_for_test(|_| {
                    if visited == cut {
                        drift(kind, paths, leaf, model);
                    }
                    visited += 1;
                });
                assert!(visited > cut);
                assert!(result.is_err());
                assert!(framed.is_refused());
                assert!(matches!(
                    framed.revalidate(),
                    Err(OriginalStartConsentError::Refused)
                ));
                busy(root);
                drop(framed);
                println!(
                    "HC074-FRAME-DRIFT cut={cut} kind={kind} refused=true fence_retained=true"
                );
            });
        }
    }
    // Independently fresh old-reference owners, never reuse a failed frame.
    for kind in 0..5 {
        with_accepted(|mut reference, paths, root, leaf, model| {
            drift(kind, paths, leaf, model);
            assert!(reference.revalidate().is_err());
            assert!(reference.is_refused());
            busy(root);
            drop(reference);
            println!("HC074-FRAME-REFERENCE kind={kind} refused=true");
        });
    }
    with_accepted(|accepted, _, root, leaf, _| {
        let mut framed = accepted.into_validation_frame().unwrap();
        let mut order = Vec::new();
        framed
            .revalidate_with_for_test(|step| order.push(format!("{step:?}")))
            .unwrap();
        assert_eq!(
            order,
            [
                "StorageBefore",
                "ReservedLive",
                "Progress",
                "Causal",
                "Prepared",
                "Consent",
                "StorageAfter",
                "FinalClocks"
            ]
        );
        assert!(!framed.is_refused());
        assert!(!format!("{framed:?}").contains("consent_request_id"));
        fs::remove_file(leaf.join(RESERVED_NAME)).unwrap();
        assert!(framed.revalidate().is_err());
        busy(root);
        drop(framed);
        println!("HC074-FRAME-CONTROL healthy_before_fault=true ordered=true");
    });
    with_accepted(|accepted, _, root, leaf, _| {
        fs::remove_file(leaf.join(RESERVED_NAME)).unwrap();
        let failure = accepted.into_validation_frame().unwrap_err();
        assert!(failure.is_refused());
        busy(root);
        drop(failure);
        println!("HC074-FRAME-CONSTRUCTION refused=true retained=true");
    });
    with_accepted(|accepted, paths, root, _, _| {
        let mut framed = accepted.into_validation_frame().unwrap();
        let path = paths.worker.clone();
        let (go, wait) = std::sync::mpsc::channel();
        let (done, finished) = std::sync::mpsc::channel();
        let writer = std::thread::spawn(move || {
            if wait.recv().is_ok() {
                fs::write(path, b"concurrent original trust drift").unwrap();
                done.send(()).unwrap();
            }
        });
        let mut go = Some(go);
        let mut visited = 0;
        let result = framed.revalidate_with_for_test(|_| {
            if visited == 5 {
                go.take().unwrap().send(()).unwrap();
                finished.recv().unwrap();
            }
            visited += 1;
        });
        drop(go);
        writer.join().unwrap();
        assert!(visited > 5);
        assert!(result.is_err());
        assert!(framed.is_refused());
        busy(root);
        drop(framed);
        println!("HC074-FRAME-CONCURRENT refused=true original_owners=true");
    });
}
