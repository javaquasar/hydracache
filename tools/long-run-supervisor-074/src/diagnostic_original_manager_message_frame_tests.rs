use super::*;
#[path = "diagnostic_original_preparation_budget_tests.rs"]
mod budget;
#[path = "diagnostic_original_reply_binding_frame_tests.rs"]
mod reply_binding;
#[test]
#[ignore = "manual nonroot original addressed message full-frame and owned rights controls"]
fn original_grouped_addressed_frame_controls() {
    for sample in 0..3 {
        let bundle = Bundle::new();
        with_original_at(false, None, |original, _paths, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let mut frame = accepted(original, fence, leaf, receiver, sender, model)
                    .prepare_original_outputs_for_test(|_| Ok(()))
                    .unwrap()
                    .prepare_enrollment_fixture_for_test(&bundle.path, 0, |_, _| {})
                    .unwrap();
                let before = records(leaf);
                let origins = frame.origins_for_test();
                let mut probe = frame.message_probe_for_test();
                for pass in 0..2 {
                    let age_before = frame.origins_for_test();
                    let ((result, checks), report) = trace::capture(|| {
                        trace::stage(Stage::Checks);
                        frame.message_round_for_test(
                            &mut probe,
                            if pass == 0 { "ok" } else { "delivery" },
                        )
                    });
                    let age_after = frame.origins_for_test();
                    println!("HC074-GROUPED-AGE sample={sample} pass={pass} issued_ns={} age_before_ns={} age_after_ns={} auth={} signatures={} channel_checks={checks} result={result:?}",
                        age_before[1], age_before[5] - age_before[1], age_after[5] - age_after[1],
                        report.stages[Stage::Checks as usize].authentication_calls,
                        report.stages[Stage::Checks as usize].signature_calls);
                    result.unwrap();
                    assert_eq!(checks, 2);
                    assert_eq!(
                        report.stages[Stage::Checks as usize].authentication_calls,
                        34
                    );
                    assert_eq!(report.stages[Stage::Checks as usize].signature_calls, 34);
                    assert!(report.first_time.is_none());
                    frame.assert_original_for_test();
                    assert_eq!(&frame.origins_for_test()[..4], &origins[..4]);
                    assert_eq!(
                        frame.offsets_for_test(),
                        [300_000_000_000, 60_000_000_000, 10_000_000_000]
                    );
                    assert_eq!(records(leaf), before);
                    println!("HC074-GROUPED-ADDRESSED-CONTROL sample={sample} pass={pass} frame_ipc=34 channel_checks=2 test_channel_only=true start=false");
                }
                busy(root);
                drop(probe);
                busy(root);
                drop(frame);
                assert_eq!(records(leaf), before);
                retained_context_refuses_for_test(root);
                println!("HC074-GROUPED-ADDRESSED-TRANSFER sample={sample} original_writers=2 rights=true same_process_socket=true system_bus_delivery=false start=false");
            });
        });
    }
}
#[test]
#[ignore = "manual nonroot original addressed message live refusals; no dispatch"]
fn original_grouped_addressed_frame_refusals() {
    for case in [
        "alias",
        "client",
        "flags-fd",
        "body",
        "swapped",
        "serial",
        "missing",
        "channel-before",
        "channel-after",
        "context-before",
        "context-after",
        "message",
        "output",
        "custodian-kill",
        "inside-message",
        "inside-collector-kill",
        "inside-final-expiry",
        "trust",
        "collector-kill",
        "final-expiry",
    ] {
        let bundle = Bundle::new();
        with_original_at(true, None, |original, paths, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let mut frame = accepted(original, fence, leaf, receiver, sender, model)
                    .prepare_original_outputs_for_test(|_| Ok(()))
                    .unwrap()
                    .prepare_enrollment_fixture_for_test(&bundle.path, 0, |_, _| {})
                    .unwrap();
                let before = records(leaf);
                let mut probe = frame.message_probe_for_test();
                match case {
                    "trust" => fs::write(&paths.worker, b"addressed trust drift").unwrap(),
                    "output" => fs::write(
                        output(leaf).join("collector-attempt-v1/collector-stderr.bin"),
                        b"unexpected",
                    )
                    .unwrap(),
                    "message" | "collector-kill" | "custodian-kill" | "final-expiry" => {
                        frame.tamper_for_test(case)
                    }
                    _ => (),
                }
                let (result, _) = frame.message_round_for_test(&mut probe, case);
                let reason = format!("{:?}", result.unwrap_err());
                assert!(
                    !reason.contains("Time"),
                    "unexpected expiry {case}: {reason}"
                );
                let (retry, checks) = frame.message_round_for_test(&mut probe, "ok");
                assert!(retry.is_err());
                assert_eq!(checks, 0);
                busy(root);
                drop(probe);
                busy(root);
                drop(frame);
                assert_eq!(records(leaf), before);
                retained_context_refuses_for_test(root);
                println!(
                    "HC074-GROUPED-ADDRESSED-REFUSAL case={case} reason={reason} sticky=true start=false"
                );
            });
        });
    }
}

#[test]
#[ignore = "manual nonroot original grouped message fixed-channel refusal retains full owner"]
fn original_grouped_addressed_nonroot_fixed_refusal() {
    let bundle = Bundle::new();
    with_original_at(true, None, |original, _paths, receiver, sender, model| {
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let frame = accepted(original, fence, leaf, receiver, sender, model)
                .prepare_original_outputs_for_test(|_| Ok(()))
                .unwrap()
                .prepare_enrollment_fixture_for_test(&bundle.path, 0, |_, _| {})
                .unwrap();
            let before = records(leaf);
            let mut failure = frame.prepare_fixed_manager_message().unwrap_err();
            assert_eq!(format!("{failure:?}"), "Frame(Channel)");
            failure.assert_retained_for_test();
            busy(root);
            drop(failure);
            assert_eq!(records(leaf), before);
            retained_context_refuses_for_test(root);
            println!("HC074-GROUPED-ADDRESSED nonroot=true fixed_refused=true original_owner_retained=true start=false");
        });
    });
}
