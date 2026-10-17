use super::*;
#[path = "diagnostic_original_custody_join_tests.rs"]
mod custody_tests;
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::OriginalGroupedPrepError;
use crate::diagnostic_ipc::validation_trace::{self as trace, Stage};

fn healthy(samples: usize) {
    for sample in 0..samples {
        let bundle = Bundle::new();
        with_original_at(false, None, |original, _, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let outputs = accepted(original, fence, leaf, receiver, sender, model)
                    .prepare_original_outputs_for_test(|_| Ok(()))
                    .unwrap();
                let before = records(leaf);
                let mut visited = Vec::new();
                let (mut alarm, preparation) = trace::capture(|| {
                    trace::stage(Stage::Bind);
                    outputs
                        .prepare_fixed_grouped_fixture_for_test(&bundle.path, |at, _| {
                            visited.push(at);
                            busy(root);
                        })
                        .unwrap()
                });
                assert_eq!(visited, ["opened", "loaded", "bound", "message", "armed"]);
                assert_eq!(
                    preparation.stages[Stage::Bind as usize].authentication_calls,
                    34
                );
                assert_eq!(preparation.stages[Stage::Bind as usize].signature_calls, 34);
                assert!(preparation.first_time.is_none());
                assert_eq!(
                    alarm.offsets_for_test(),
                    [300_000_000_000, 60_000_000_000, 10_000_000_000]
                );
                let origins = alarm.origins_for_test();
                println!(
                    "HC074-FIXED-GROUPED-COST sample={sample} phase=prepare report={preparation:?}"
                );
                let (_, validation) = trace::capture(|| {
                    trace::stage(Stage::Checks);
                    alarm.revalidate_fixture_for_test().unwrap();
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
                assert_eq!(&alarm.origins_for_test()[..4], &origins[..4]);
                println!("HC074-FIXED-GROUPED-COST sample={sample} phase=reference-validate report={validation:?}");
                assert!(!alarm.is_refused());
                assert_eq!(records(leaf), before);
                busy(root);
                drop(alarm);
                retained_context_refuses_for_test(root);
                println!("HC074-FIXED-GROUPED-CONTROL sample={sample} original=true unchanged=true checkpoints=5 bounds_original=true reference=true guardian=false send=false admission=false");
            });
        });
    }
}
#[test]
#[ignore = "manual nonroot grouped preparation first healthy screen; no send or execution"]
fn fixed_grouped_preparation_screen() {
    healthy(1);
}
#[test]
#[ignore = "manual nonroot grouped preparation repeated healthy gate; no send or execution"]
fn fixed_grouped_preparation_healthy() {
    healthy(3);
}

#[test]
#[ignore = "manual nonroot grouped preparation persistent drift and expiry; no send or execution"]
fn fixed_grouped_preparation_refusals() {
    for (case, cut) in [
        ("trust", "opened"),
        ("trust", "loaded"),
        ("trust", "bound"),
        ("trust", "message"),
        ("trust", "armed"),
        ("bundle-content", "loaded"),
        ("bundle-inode", "armed"),
        ("output", "armed"),
        ("projection", "armed"),
        ("state", "armed"),
        ("writer", "armed"),
        ("message", "armed"),
        ("alarm", "armed"),
        ("expiry", "armed"),
    ] {
        let bundle = Bundle::new();
        with_original_at(true, None, |original, paths, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let outputs = accepted(original, fence, leaf, receiver, sender, model)
                    .prepare_original_outputs_for_test(|_| Ok(()))
                    .unwrap();
                let before = records(leaf);
                let mut injected = false;
                let (result, event) =
                    crate::diagnostic_kernel_admission::refusal_trace::capture(|| {
                        outputs.prepare_fixed_grouped_fixture_for_test(&bundle.path, |at, held| {
                            busy(root);
                            if at == cut {
                                assert!(!injected);
                                injected = true;
                                match case {
                                    "trust" => {
                                        fs::write(&paths.worker, b"original trust drift").unwrap()
                                    }
                                    "bundle-content" => bundle.change(),
                                    "bundle-inode" => bundle.replace(),
                                    "output" => fs::write(
                                        output(leaf)
                                            .join("collector-attempt-v1/collector-stderr.bin"),
                                        b"unexpected",
                                    )
                                    .unwrap(),
                                    "expiry" => {
                                        std::thread::sleep(std::time::Duration::from_secs(10))
                                    }
                                    other => held.tamper_fixed_grouped_for_test(other),
                                }
                            }
                        })
                    });
                assert!(injected, "{case}/{cut}");
                let mut failure = result.unwrap_err();
                let reason = format!("{:?}", failure.error());
                assert!(failure.is_refused());
                if case == "expiry" {
                    let event = event.expect("original progress age refusal");
                    assert_eq!(event.site, "progress-age");
                    assert_eq!(event.clock[5], 10_000_000_000);
                    assert!(
                        reason.contains("Causal(Progress(Input(Kernel(Invalid))))"),
                        "{reason}"
                    );
                } else {
                    assert!(
                        !reason.contains("Time"),
                        "late time refusal is not {case}: {reason}"
                    );
                    match case {
                        "trust" => assert!(reason.contains("Input(Invalid)"), "{reason}"),
                        "bundle-content" | "bundle-inode" => {
                            assert!(reason.contains("Material(Build("), "{reason}")
                        }
                        "output" => assert!(reason.contains("Outputs(Invalid)"), "{reason}"),
                        "writer" => assert!(reason.contains("Invalid"), "{reason}"),
                        "projection" | "state" | "message" | "alarm" => {
                            assert!(reason.contains("Binding"), "{reason}")
                        }
                        _ => unreachable!(),
                    }
                }
                assert!(matches!(
                    failure.retry_fixed_grouped_for_test(),
                    Err(OriginalGroupedPrepError::Refused)
                ));
                busy(root);
                assert_eq!(records(leaf), before);
                drop(failure);
                retained_context_refuses_for_test(root);
                println!("HC074-FIXED-GROUPED-REFUSAL case={case} cut={cut} exact=true sticky=true held=true unchanged=true send=false");
            });
        });
    }
}

#[test]
#[ignore = "manual nonroot production revalidation refuses the fixture origin; no installation"]
fn fixed_grouped_preparation_fixture_cannot_enter_production() {
    // Exercise the actual production constructor as well: absence is a refusal,
    // never a fallback to the available signed fixture below.
    assert_eq!(
        fs::symlink_metadata(crate::diagnostic_artifacts::INSTALL_ROOT)
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::NotFound
    );
    with_original_at(true, None, |original, _, receiver, sender, model| {
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let outputs = accepted(original, fence, leaf, receiver, sender, model)
                .prepare_original_outputs_for_test(|_| Ok(()))
                .unwrap();
            let before = records(leaf);
            let mut failure = outputs.prepare_original_grouped().unwrap_err();
            let reason = format!("{:?}", failure.error());
            assert!(reason.contains("Material(Build(Io("), "{reason}");
            assert!(failure.is_refused());
            assert!(matches!(
                failure.retry_fixed_grouped_for_test(),
                Err(OriginalGroupedPrepError::Refused)
            ));
            busy(root);
            assert_eq!(records(leaf), before);
            drop(failure);
            retained_context_refuses_for_test(root);
            println!("HC074-FIXED-GROUPED-ORIGIN missing_fixed_install_refused=true fallback=false sticky=true held=true unchanged=true send=false");
        });
    });
    let bundle = Bundle::new();
    with_original_at(true, None, |original, _, receiver, sender, model| {
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let outputs = accepted(original, fence, leaf, receiver, sender, model)
                .prepare_original_outputs_for_test(|_| Ok(()))
                .unwrap();
            let before = records(leaf);
            let mut held = outputs
                .prepare_fixed_grouped_fixture_for_test(&bundle.path, |_, _| busy(root))
                .unwrap();
            let error = held.revalidate().unwrap_err();
            assert!(
                format!("{error:?}").contains("Material(Fixture)"),
                "{error:?}"
            );
            assert!(held.is_refused());
            assert!(matches!(
                held.revalidate_fixture_for_test(),
                Err(OriginalGroupedPrepError::Refused)
            ));
            busy(root);
            assert_eq!(records(leaf), before);
            drop(held);
            retained_context_refuses_for_test(root);
            println!("HC074-FIXED-GROUPED-ORIGIN fixture_refused=true sticky=true held=true unchanged=true send=false");
        });
    });
}
