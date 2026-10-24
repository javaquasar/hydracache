use super::*;
#[path = "diagnostic_original_enrollment_fault_tests.rs"]
mod faults;

#[path = "diagnostic_original_manager_frame_tests.rs"]
mod manager_tests;
fn healthy(samples: usize) {
    for sample in 0..samples {
        let bundle = Bundle::new();
        with_original_at(false, None, |original, _paths, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let outputs = accepted(original, fence, leaf, receiver, sender, model)
                    .prepare_original_outputs_for_test(|_| Ok(()))
                    .unwrap();
                let before = records(leaf);
                let parent_mask = mask();
                let mut origins = None;
                let (mut joined, report) = trace::capture(|| {
                    trace::stage(Stage::Bind);
                    outputs
                        .prepare_enrollment_fixture_for_test(&bundle.path, 0, |cut, joined| {
                            if cut == "enrolled" {
                                origins = Some(joined.origins_for_test());
                            }
                        })
                        .unwrap()
                });
                call_count(&report, Stage::Bind);
                let origins = origins.unwrap();
                assert_eq!(mask(), parent_mask);
                joined.assert_original_for_test();
                assert_eq!(
                    joined.offsets_for_test(),
                    [300_000_000_000, 60_000_000_000, 10_000_000_000]
                );
                assert_eq!(&joined.origins_for_test()[..4], &origins[..4]);
                let (_, report) = trace::capture(|| {
                    trace::stage(Stage::Checks);
                    joined.revalidate().unwrap();
                });
                call_count(&report, Stage::Checks);
                joined.assert_original_for_test();
                assert_eq!(&joined.origins_for_test()[..4], &origins[..4]);
                let pins = joined.pinned_for_test();
                busy(root);
                assert_eq!(records(leaf), before);
                drop(joined);
                for pin in pins {
                    terminal(&pin);
                }
                drop(crate::host_execution::lock_host_root(root).unwrap());
                assert_eq!(records(leaf), before);
                retained_context_refuses_for_test(root);
                println!("HC074-ENROLLMENT-CONTROL sample={sample} create_ipc=34 validate_ipc=34 collector_fds=7 custody_fds=3 unchanged=true start=false");
            });
        });
    }
}
#[test]
#[ignore = "manual nonroot combined original enrollment healthy screen; no execution"]
fn original_enrollment_frame_screen() {
    healthy(1);
}
#[test]
#[ignore = "manual nonroot combined original enrollment repeated controls; no execution"]
fn original_enrollment_frame_healthy() {
    healthy(3);
}
#[test]
#[ignore = "manual nonroot combined original enrollment live refusals; no execution"]
fn original_enrollment_frame_refusals() {
    let seed = 0x0740_e118_u64;
    for case in [
        "clone-error",
        "restore-error",
        "trust",
        "output",
        "message",
        "custody-flags",
        "collector-kill",
        "custodian-kill",
        "final-expiry",
    ] {
        let bundle = Bundle::new();
        with_original_at(true, None, |original, paths, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let outputs = accepted(original, fence, leaf, receiver, sender, model)
                    .prepare_original_outputs_for_test(|_| Ok(()))
                    .unwrap();
                let before = records(leaf);
                let parent_mask = mask();
                let fault = match case {
                    "clone-error" => 1,
                    "restore-error" => 2,
                    _ => 0,
                };
                let mut failure = outputs
                    .prepare_enrollment_fixture_for_test(&bundle.path, fault, |cut, joined| {
                        if cut == "enrolled" {
                            match case {
                                "trust" => {
                                    fs::write(&paths.worker, b"enrollment trust drift").unwrap()
                                }
                                "output" => fs::write(
                                    output(leaf).join("collector-attempt-v1/collector-stderr.bin"),
                                    b"unexpected",
                                )
                                .unwrap(),
                                other => joined.tamper_for_test(other),
                            }
                        }
                    })
                    .unwrap_err();
                assert_eq!(mask(), parent_mask);
                let reason = failure.reason_for_test();
                assert!(
                    !reason.contains("Time"),
                    "late time is not {case}: {reason}"
                );
                match case {
                    "clone-error" => assert!(reason.contains("code: 11"), "{reason}"),
                    "restore-error" => assert!(reason.contains("code: 5"), "{reason}"),
                    "trust" => assert!(reason.contains("Input(Invalid)"), "{reason}"),
                    "output" => assert!(reason.contains("Outputs(Invalid)"), "{reason}"),
                    "message" => assert!(reason.contains("Binding"), "{reason}"),
                    "custody-flags" => assert!(reason.contains("Storage(Document)"), "{reason}"),
                    "collector-kill" | "custodian-kill" => assert_eq!(reason, "Child"),
                    _ => assert_eq!(reason, "Original(Original(Expired))"),
                }
                assert!(failure.refused_for_test());
                assert_eq!(
                    format!("{:?}", failure.retry_for_test().unwrap_err()),
                    "Refused"
                );
                busy(root);
                drop(failure);
                drop(crate::host_execution::lock_host_root(root).unwrap());
                assert_eq!(records(leaf), before);
                retained_context_refuses_for_test(root);
                println!("HC074-ENROLLMENT-REFUSAL seed=0x{seed:08x} case={case} reason={reason} sticky=true unchanged=true start=false");
            });
        });
    }
}
