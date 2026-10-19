//! Same original full frame with a distinct metadata-only test channel probe.
use super::*;

#[test]
#[ignore = "manual nonroot original watchdog context frame; no watchdog spawn or delivery"]
fn original_watchdog_context_frame_controls() {
    for case in [
        "ok",
        "retained-owner",
        "retained-identity",
        "retained-origin",
        "retained-bounds",
        "retained-deadline",
        "retained-fence",
        "retained-alarm",
        "retained-alias",
        "closing-context-fence",
        "closing-context-deadline",
        "closing-context-origin",
        "names-after",
        "inside-custody-missing",
    ] {
        let (_, first_refusal) = crate::diagnostic_kernel_admission::refusal_trace::capture(|| {
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
                        let mut probe = frame.watchdog_probe_for_test();
                        if case.starts_with("retained-") {
                            let (result, checks) = frame.watchdog_round_for_test(&mut probe, "ok");
                            result.unwrap();
                            assert_eq!(checks, 2);
                        }
                        let ((result, checks), report) = trace::capture(|| {
                            trace::stage(Stage::Checks);
                            frame.watchdog_round_for_test(&mut probe, case)
                        });
                        let complete = case == "ok"
                            || case.starts_with("closing-context-")
                            || case == "names-after";
                        let expected = if complete { 34 } else { 0 };
                        assert_eq!(
                            report.stages[Stage::Checks as usize].authentication_calls,
                            expected
                        );
                        assert_eq!(
                            report.stages[Stage::Checks as usize].signature_calls,
                            expected
                        );
                        assert!(report.first_time.is_none());
                        if case == "ok" {
                            result.unwrap();
                            assert_eq!(checks, 2);
                            let ((again, checks), again_report) = trace::capture(|| {
                                trace::stage(Stage::Checks);
                                frame.watchdog_round_for_test(&mut probe, "ok")
                            });
                            again.unwrap();
                            assert_eq!(checks, 2);
                            assert_eq!(
                                again_report.stages[Stage::Checks as usize].authentication_calls,
                                34
                            );
                            assert_eq!(
                                again_report.stages[Stage::Checks as usize].signature_calls,
                                34
                            );
                            frame.assert_original_for_test();
                        } else {
                            let reason = format!("{:?}", result.unwrap_err());
                            assert!(
                                !reason.contains("Time") && !reason.contains("Expired"),
                                "unexpected time failure {case}: {reason}"
                            );
                            assert_eq!(
                                checks,
                                if complete {
                                    2
                                } else if case.starts_with("retained-") {
                                    0
                                } else {
                                    1
                                }
                            );
                            let (retry, checks) = frame.watchdog_round_for_test(&mut probe, "ok");
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
                        println!("HC074-ORIGINAL-WATCHDOG case={case} frame_auth={expected} channel_checks={checks} original_bounds=true test_channel_only=true spawn=false send=false");
                    });
                },
            );
        });
        assert!(
            first_refusal.is_none(),
            "unexpected kernel refusal in {case}: {first_refusal:?}"
        );
    }
}

#[path = "diagnostic_original_pending_watchdog_frame_tests.rs"]
mod pending;
