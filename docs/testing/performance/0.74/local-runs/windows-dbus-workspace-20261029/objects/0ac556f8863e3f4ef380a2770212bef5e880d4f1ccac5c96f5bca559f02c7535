use super::*;
#[path = "diagnostic_original_descriptor_plan_tests.rs"]
mod descriptor_plan_tests;
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::custody::OriginalCustodyError;

fn custody_healthy(samples: usize) {
    for sample in 0..samples {
        let bundle = Bundle::new();
        with_original_at(false, None, |original, _, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let prepared = accepted(original, fence, leaf, receiver, sender, model)
                    .prepare_original_outputs_for_test(|_| Ok(()))
                    .unwrap();
                let before = records(leaf);
                let origins = prepared.custody_origins_for_test();
                let mut visited = Vec::new();
                let (mut held, bind) = trace::capture(|| {
                    trace::stage(Stage::Bind);
                    prepared
                        .prepare_custodied_fixture_for_test(&bundle.path, |at, _| {
                            visited.push(at);
                            busy(root);
                        })
                        .unwrap()
                });
                assert_eq!(visited, ["opened", "retained"]);
                assert_eq!(bind.stages[Stage::Bind as usize].authentication_calls, 34);
                assert_eq!(bind.stages[Stage::Bind as usize].signature_calls, 34);
                assert!(bind.first_time.is_none());
                assert_eq!(&held.origins_for_test()[..4], &origins[..4]);
                assert_eq!(
                    held.offsets_for_test(),
                    [300_000_000_000, 60_000_000_000, 10_000_000_000]
                );
                let (_, validation) = trace::capture(|| {
                    trace::stage(Stage::Checks);
                    held.revalidate_fixture_for_test().unwrap();
                });
                assert_eq!(
                    validation.stages[Stage::Checks as usize].authentication_calls,
                    34
                );
                assert_eq!(
                    validation.stages[Stage::Checks as usize].signature_calls,
                    34
                );
                assert!(validation.first_time.is_none());
                assert_eq!(&held.origins_for_test()[..4], &origins[..4]);
                assert!(!held.is_refused());
                assert_eq!(records(leaf), before);
                inventory_at_cut(leaf, 9);
                busy(root);
                held.drop_eof_for_test();
                retained_context_refuses_for_test(root);
                println!("HC074-CUSTODY-JOIN-CONTROL sample={sample} bind_ipc=34 validate_ipc=34 original_origins=true unchanged=true eof=true send=false admission=false");
            });
        });
    }
}
#[test]
#[ignore = "manual nonroot original custody/sink full-frame healthy screen; no execution"]
fn original_custody_join_screen() {
    custody_healthy(1);
}
#[test]
#[ignore = "manual nonroot original custody/sink repeated full-frame healthy controls; no execution"]
fn original_custody_join_healthy() {
    custody_healthy(3);
}

#[test]
#[ignore = "manual nonroot original custody full-frame persistent drift and expiry; no execution"]
fn original_custody_join_refusals() {
    let seed = 0x0740_c518_u64;
    for (case, cut) in [
        ("trust", "opened"),
        ("trust", "retained"),
        ("bundle-content", "retained"),
        ("output", "retained"),
        ("message", "retained"),
        ("writer", "retained"),
        ("projection", "retained"),
        ("state", "retained"),
        ("alarm", "retained"),
        ("custody-missing", "retained"),
        ("custody-object", "retained"),
        ("custody-flags", "retained"),
        ("expiry", "retained"),
    ] {
        let bundle = Bundle::new();
        with_original_at(true, None, |original, paths, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let prepared = accepted(original, fence, leaf, receiver, sender, model)
                    .prepare_original_outputs_for_test(|_| Ok(()))
                    .unwrap();
                let before = records(leaf);
                let mut injected = false;
                let result =
                    prepared.prepare_custodied_fixture_for_test(&bundle.path, |at, held| {
                        if at == cut {
                            assert!(!injected);
                            injected = true;
                            match case {
                                "trust" => {
                                    fs::write(&paths.worker, b"original trust drift").unwrap()
                                }
                                "bundle-content" => bundle.change(),
                                "output" => fs::write(
                                    output(leaf).join("collector-attempt-v1/collector-stderr.bin"),
                                    b"unexpected",
                                )
                                .unwrap(),
                                "expiry" => std::thread::sleep(std::time::Duration::from_secs(10)),
                                other => held.tamper_for_test(other),
                            }
                        }
                    });
                assert!(injected);
                let mut failure = result.unwrap_err();
                let reason = format!("{:?}", failure.error());
                assert!(failure.is_refused());
                if case == "expiry" {
                    assert!(
                        reason.contains("Causal(Progress(Input(Kernel(Invalid))))"),
                        "{reason}"
                    );
                } else {
                    assert!(
                        !reason.contains("Time"),
                        "late time is not {case}: {reason}"
                    );
                    match case {
                        "trust" => assert!(reason.contains("Input(Invalid)"), "{reason}"),
                        "bundle-content" => assert!(reason.contains("Material(Build("), "{reason}"),
                        "output" => assert!(reason.contains("Outputs(Invalid)"), "{reason}"),
                        "custody-missing" => assert_eq!(reason, "Binding"),
                        "custody-object" | "custody-flags" => {
                            assert_eq!(reason, "Storage(Document)")
                        }
                        "writer" => assert!(reason.contains("Invalid"), "{reason}"),
                        _ => assert!(reason.contains("Binding"), "{reason}"),
                    }
                }
                assert!(matches!(
                    failure.retry_for_test(),
                    Err(OriginalCustodyError::Refused)
                ));
                busy(root);
                assert_eq!(records(leaf), before);
                drop(failure);
                retained_context_refuses_for_test(root);
                println!("HC074-CUSTODY-JOIN-REFUSAL seed=0x{seed:08x} case={case} cut={cut} sticky=true held=true unchanged=true send=false");
            });
        });
    }
}

#[test]
#[ignore = "manual nonroot original custody later live checks and fixed-origin refusal; no execution"]
fn original_custody_join_later_refusals() {
    for case in [
        "trust",
        "custody-object",
        "custody-flags",
        "custody-missing",
        "fixture-origin",
        "final-expiry",
        "production-bind",
    ] {
        let bundle = Bundle::new();
        with_original_at(true, None, |original, paths, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let prepared = accepted(original, fence, leaf, receiver, sender, model)
                    .prepare_original_outputs_for_test(|_| Ok(()))
                    .unwrap();
                let before = records(leaf);
                if case == "production-bind" {
                    let mut failure = prepared.prepare_original_custodied().unwrap_err();
                    assert!(format!("{:?}", failure.error()).contains("Material(Build(Io("));
                    assert!(failure.is_refused());
                    assert!(matches!(
                        failure.retry_for_test(),
                        Err(OriginalCustodyError::Refused)
                    ));
                    busy(root);
                    assert_eq!(records(leaf), before);
                    drop(failure);
                } else {
                    let mut held = prepared
                        .prepare_custodied_fixture_for_test(&bundle.path, |_, _| busy(root))
                        .unwrap();
                    match case {
                        "trust" => fs::write(&paths.worker, b"later original trust drift").unwrap(),
                        "fixture-origin" => (),
                        other => held.tamper_for_test(other),
                    }
                    let error = if case == "fixture-origin" {
                        held.revalidate().unwrap_err()
                    } else {
                        held.revalidate_fixture_for_test().unwrap_err()
                    };
                    let reason = format!("{error:?}");
                    assert!(!reason.contains("Time"), "{reason}");
                    match case {
                        "trust" => assert!(reason.contains("Input(Invalid)"), "{reason}"),
                        "fixture-origin" => {
                            assert!(reason.contains("Material(Fixture)"), "{reason}")
                        }
                        "custody-missing" => assert_eq!(reason, "Binding"),
                        "final-expiry" => assert_eq!(reason, "Original(Expired)"),
                        _ => assert_eq!(reason, "Storage(Document)"),
                    }
                    assert!(held.is_refused());
                    assert!(matches!(
                        held.revalidate_fixture_for_test(),
                        Err(OriginalCustodyError::Refused)
                    ));
                    busy(root);
                    assert_eq!(records(leaf), before);
                    drop(held);
                }
                retained_context_refuses_for_test(root);
                println!("HC074-CUSTODY-JOIN-LATER case={case} sticky=true held=true unchanged=true send=false");
            });
        });
    }
}
