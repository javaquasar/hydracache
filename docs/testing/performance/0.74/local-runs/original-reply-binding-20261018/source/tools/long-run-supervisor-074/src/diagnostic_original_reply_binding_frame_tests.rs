//! Distinct test-channel frame probe; no production channel or dispatch.
use super::*;
#[test]
#[ignore = "manual nonroot original reply binding full-frame faults; no dispatch"]
fn original_reply_binding_frame_controls_and_refusals() {
    for case in [
        "ok",
        "reply-missing",
        "reply-manager",
        "reply-client",
        "reply-serial",
        "closing-reply-missing",
        "closing-reply-manager",
        "closing-reply-client",
        "closing-reply-serial",
        "names-before",
        "names-after",
    ] {
        let bundle = Bundle::new();
        with_original_at(
            case != "ok",
            None,
            |original, _paths, receiver, sender, model| {
                with_owned_empty_fence_for_test(|fence, root, leaf| {
                    let mut frame = accepted(original, fence, leaf, receiver, sender, model)
                        .prepare_original_outputs_for_test(|_| Ok(()))
                        .unwrap()
                        .prepare_enrollment_fixture_for_test(&bundle.path, 0, |_, _| {})
                        .unwrap();
                    let before = records(leaf);
                    let origins = frame.origins_for_test();
                    let mut probe = frame.message_probe_for_test();
                    let ((result, checks), report) = trace::capture(|| {
                        trace::stage(Stage::Checks);
                        frame.message_round_for_test(&mut probe, case)
                    });
                    let closing =
                        case == "ok" || case.starts_with("closing-") || case == "names-after";
                    let expected_auth = if closing { 34 } else { 0 };
                    assert_eq!(
                        report.stages[Stage::Checks as usize].authentication_calls,
                        expected_auth
                    );
                    assert_eq!(
                        report.stages[Stage::Checks as usize].signature_calls,
                        expected_auth
                    );
                    assert!(report.first_time.is_none());
                    if case == "ok" {
                        result.unwrap();
                        assert_eq!(checks, 2);
                    } else {
                        let reason = format!("{:?}", result.unwrap_err());
                        assert!(
                            !reason.contains("Time"),
                            "unexpected expiry {case}: {reason}"
                        );
                        assert_eq!(
                            checks,
                            if case.starts_with("closing-") || case == "names-after" {
                                2
                            } else {
                                1
                            }
                        );
                        let (retry, checks) = frame.message_round_for_test(&mut probe, "ok");
                        assert!(retry.is_err());
                        assert_eq!(checks, 0);
                    }
                    assert_eq!(&frame.origins_for_test()[..4], &origins[..4]);
                    assert_eq!(
                        frame.offsets_for_test(),
                        [300_000_000_000, 60_000_000_000, 10_000_000_000]
                    );
                    busy(root);
                    drop(probe);
                    busy(root);
                    drop(frame);
                    assert_eq!(records(leaf), before);
                    retained_context_refuses_for_test(root);
                    println!("HC074-ORIGINAL-REPLY case={case} frame_auth={expected_auth} channel_checks={checks} original_bounds=true test_channel_only=true start=false");
                });
            },
        );
    }
}
