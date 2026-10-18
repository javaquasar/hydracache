//! Full original nonroot frame with a distinct test channel, never a fixed owner.
use super::*;
#[test]
#[ignore = "manual nonroot original manager full frame controls; no fixed owner or execution"]
fn original_manager_join_frame_controls() {
    for sample in 0..3 {
        let bundle = Bundle::new();
        with_original_at(false, None, |original, _paths, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let outputs = accepted(original, fence, leaf, receiver, sender, model)
                    .prepare_original_outputs_for_test(|_| Ok(()))
                    .unwrap();
                let mut frame = outputs
                    .prepare_enrollment_fixture_for_test(&bundle.path, 0, |_, _| {})
                    .unwrap();
                let before = records(leaf);
                let origins = frame.origins_for_test();
                let pins = frame.pinned_for_test();
                let ((result, checks), report) = trace::capture(|| {
                    trace::stage(Stage::Checks);
                    frame.manager_round_for_test("ok")
                });
                result.unwrap();
                assert_eq!(checks, 2);
                call_count(&report, Stage::Checks);
                frame.assert_original_for_test();
                assert_eq!(&frame.origins_for_test()[..4], &origins[..4]);
                assert_eq!(
                    frame.offsets_for_test(),
                    [300_000_000_000, 60_000_000_000, 10_000_000_000]
                );
                busy(root);
                assert_eq!(records(leaf), before);
                drop(frame);
                for pin in pins {
                    terminal(&pin);
                }
                assert_eq!(records(leaf), before);
                retained_context_refuses_for_test(root);
                println!("HC074-ORIGINAL-MANAGER-CONTROL sample={sample} full_frame_ipc=34 channel_checks=2 original_bounds=true test_channel_only=true start=false");
            });
        });
    }
}
#[test]
#[ignore = "manual nonroot original manager live refusals; no fixed owner or execution"]
fn original_manager_join_frame_refusals() {
    for case in [
        "channel-before",
        "channel-after",
        "context-before",
        "context-after",
        "trust",
        "message",
        "output",
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
                let mut frame = outputs
                    .prepare_enrollment_fixture_for_test(&bundle.path, 0, |_, _| {})
                    .unwrap();
                let before = records(leaf);
                let origins = frame.origins_for_test();
                match case {
                    "trust" => fs::write(&paths.worker, b"manager trust drift").unwrap(),
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
                let (result, _) = frame.manager_round_for_test(case);
                let reason = format!("{:?}", result.unwrap_err());
                assert!(
                    !reason.contains("Time"),
                    "unexpected expiry for {case}: {reason}"
                );
                assert_eq!(&frame.origins_for_test()[..4], &origins[..4]);
                let (retry, checks) = frame.manager_round_for_test("ok");
                assert!(retry.is_err());
                assert_eq!(checks, 0);
                busy(root);
                drop(frame);
                assert_eq!(records(leaf), before);
                retained_context_refuses_for_test(root);
                println!("HC074-ORIGINAL-MANAGER-REFUSAL case={case} reason={reason} sticky=true test_channel_only=true start=false");
            });
        });
    }
}
#[test]
#[ignore = "manual nonroot consuming fixed-channel namespace refusal retains full owner"]
fn original_manager_join_nonroot_fixed_refusal() {
    let bundle = Bundle::new();
    with_original_at(true, None, |original, _paths, receiver, sender, model| {
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let outputs = accepted(original, fence, leaf, receiver, sender, model)
                .prepare_original_outputs_for_test(|_| Ok(()))
                .unwrap();
            let frame = outputs
                .prepare_enrollment_fixture_for_test(&bundle.path, 0, |_, _| {})
                .unwrap();
            let before = records(leaf);
            let mut failure = frame.bind_fixed_manager().unwrap_err();
            assert_eq!(format!("{failure:?}"), "Channel");
            failure.assert_retained_for_test();
            busy(root);
            drop(failure);
            assert_eq!(records(leaf), before);
            retained_context_refuses_for_test(root);
            println!("HC074-ORIGINAL-MANAGER nonroot=true fixed_refused=true original_owner_retained=true start=false");
        });
    });
}
