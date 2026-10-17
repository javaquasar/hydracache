use super::*;
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::OriginalPipeTransportError;

#[test]
#[ignore = "manual nonroot original unsent pipe message ownership; no bus or execution"]
fn original_pipe_transport_joined_healthy() {
    use crate::diagnostic_ipc::validation_trace::{self as trace, Stage};
    for sample in 0..3 {
        let bundle = Bundle::new();
        with_original_at(false, None, |original, _, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let material = accepted(original, fence, leaf, receiver, sender, model)
                    .prepare_original_outputs_for_test(|_| Ok(()))
                    .unwrap()
                    .prepare_original_start_material_for_test(&bundle.path, || {})
                    .unwrap();
                let before = records(leaf);
                let (mut transport, preparation) = trace::capture(|| {
                    trace::stage(Stage::Bind);
                    material.prepare_original_pipe_transport_for_test().unwrap()
                });
                let (_, validation) = trace::capture(|| {
                    trace::stage(Stage::Checks);
                    transport.revalidate_fixture_for_test().unwrap();
                });
                assert_eq!(
                    preparation.stages[Stage::Bind as usize].authentication_calls,
                    34
                );
                assert_eq!(
                    validation.stages[Stage::Checks as usize].authentication_calls,
                    34
                );
                assert!(preparation.first_time.is_none() && validation.first_time.is_none());
                println!("HC074-PIPE-TRANSPORT-COST sample={sample} phase=prepare report={preparation:?}");
                println!("HC074-PIPE-TRANSPORT-COST sample={sample} phase=validate report={validation:?}");
                assert!(!transport.is_refused());
                busy(root);
                assert_eq!(records(leaf), before);
                assert_eq!(fs::read_dir(leaf).unwrap().count(), 3);
                drop(transport);
                retained_context_refuses_for_test(root);
                println!("HC074-PIPE-TRANSPORT-CONTROL sample={sample} original=true held=true unchanged=true send=false");
            });
        });
    }
}

#[test]
#[ignore = "manual nonroot original unsent pipe message fault gate; no bus or execution"]
fn original_pipe_transport_joined_refusals() {
    for case in [
        "entry-fixture",
        "entry-build",
        "entry-refused",
        "message",
        "build",
        "output",
        "trust",
        "fixture",
        "expiry",
    ] {
        let bundle = Bundle::new();
        with_original_at(true, None, |original, paths, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let material = accepted(original, fence, leaf, receiver, sender, model)
                    .prepare_original_outputs_for_test(|_| Ok(()))
                    .unwrap()
                    .prepare_original_start_material_for_test(&bundle.path, || {})
                    .unwrap();
                let before = records(leaf);
                if case.starts_with("entry-") {
                    let mut material = material;
                    if case == "entry-refused" {
                        material.corrupt_for_test(0);
                        assert!(matches!(
                            material.revalidate_fixture_for_test(),
                            Err(OriginalStartMaterialError::Binding)
                        ));
                    }
                    let result = if case == "entry-fixture" {
                        material.prepare_original_pipe_transport()
                    } else {
                        if case == "entry-build" {
                            bundle.change();
                        }
                        material.prepare_original_pipe_transport_for_test()
                    };
                    let failure = result.unwrap_err();
                    assert!(failure.is_refused());
                    match (case, failure.error()) {
                        (
                            "entry-fixture",
                            OriginalPipeTransportError::Material(
                                OriginalStartMaterialError::Fixture,
                            ),
                        )
                        | (
                            "entry-build",
                            OriginalPipeTransportError::Material(
                                OriginalStartMaterialError::Build(_),
                            ),
                        )
                        | ("entry-refused", OriginalPipeTransportError::Refused) => (),
                        _ => panic!("wrong refusal {case}: {failure:?}"),
                    }
                    busy(root);
                    drop(failure);
                } else {
                    let mut transport =
                        material.prepare_original_pipe_transport_for_test().unwrap();
                    match case {
                        "message" => transport.remove_message_for_test(),
                        "build" => bundle.change(),
                        "output" => fs::write(
                            output(leaf).join("collector-attempt-v1/collector-stderr.bin"),
                            b"unexpected",
                        )
                        .unwrap(),
                        "trust" => fs::write(&paths.worker, b"original trust drift").unwrap(),
                        "fixture" => (),
                        "expiry" => std::thread::sleep(std::time::Duration::from_secs(10)),
                        _ => unreachable!(),
                    }
                    let (error, event) =
                        crate::diagnostic_kernel_admission::refusal_trace::capture(|| {
                            if case == "fixture" {
                                transport.revalidate().unwrap_err()
                            } else {
                                transport.revalidate_fixture_for_test().unwrap_err()
                            }
                        });
                    match (case, &error) {
                        ("message", OriginalPipeTransportError::Binding)
                        | (
                            "build",
                            OriginalPipeTransportError::Material(
                                OriginalStartMaterialError::Build(_),
                            ),
                        )
                        | (
                            "output",
                            OriginalPipeTransportError::Material(
                                OriginalStartMaterialError::Outputs(OriginalSinkError::Invalid),
                            ),
                        )
                        | (
                            "fixture",
                            OriginalPipeTransportError::Material(
                                OriginalStartMaterialError::Fixture,
                            ),
                        ) => (),
                        ("trust", _) => {
                            let reason = format!("{error:?}");
                            assert!(
                                reason.contains("Input(Invalid)")
                                    && !reason.contains("Kernel(")
                                    && !reason.contains("Time"),
                                "{reason}"
                            );
                        }
                        ("expiry", _) => {
                            let event = event.expect("original progress age refusal");
                            assert_eq!(event.site, "progress-age");
                            assert_eq!(event.clock[5], 10_000_000_000);
                            assert!(event.clock[0] > event.clock[1]);
                            assert!(
                                event.clock[2] >= event.clock[0]
                                    && event.clock[2] - event.clock[0] >= event.clock[5]
                            );
                            assert!(
                                format!("{error:?}")
                                    .contains("Causal(Progress(Input(Kernel(Invalid))))"),
                                "{error:?}"
                            );
                            println!("HC074-PIPE-TRANSPORT-EXPIRY event={event:?} renewed=false");
                        }
                        _ => panic!("wrong refusal {case}: {error:?}"),
                    }
                    assert!(transport.is_refused());
                    assert!(matches!(
                        transport.revalidate_fixture_for_test(),
                        Err(OriginalPipeTransportError::Refused)
                    ));
                    busy(root);
                    drop(transport);
                }
                assert_eq!(records(leaf), before);
                assert_eq!(fs::read_dir(leaf).unwrap().count(), 3);
                retained_context_refuses_for_test(root);
                println!("HC074-PIPE-TRANSPORT-REFUSAL case={case} exact=true sticky=true held=true unchanged=true send=false");
            });
        });
    }
}
