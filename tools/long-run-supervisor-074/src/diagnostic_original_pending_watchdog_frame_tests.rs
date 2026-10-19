//! Same original frame and real Pending children; fixed-channel test probe only.
use super::*;

#[test]
#[ignore = "manual nonroot original Pending custody; no enrollment or delivery"]
fn original_pending_watchdog_frame_controls() {
    for case in [
        "ok",
        "clone-first",
        "clone-second",
        "restore-first",
        "restore-second",
        "guardian-first-dead",
        "guardian-second-dead",
        "retained-origin",
        "closing-origin",
        "inside-custody-missing",
    ] {
        let (_, refusal) = crate::diagnostic_kernel_admission::refusal_trace::capture(|| {
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
                        let mut probe = frame.pending_probe_for_test(case);
                        let ((result, checks), report) = trace::capture(|| {
                            trace::stage(Stage::Checks);
                            probe.prepare_for_test()
                        });
                        let expected = if case == "ok" {
                            68
                        } else if case == "retained-origin" {
                            0
                        } else {
                            34
                        };
                        let expected_checks = if case == "ok" {
                            4
                        } else if case == "retained-origin" {
                            0
                        } else if case == "inside-custody-missing" {
                            3
                        } else {
                            2
                        };
                        assert_eq!(
                            report.stages[Stage::Checks as usize].authentication_calls,
                            expected
                        );
                        assert_eq!(
                            report.stages[Stage::Checks as usize].signature_calls,
                            expected
                        );
                        assert_eq!(checks, expected_checks);
                        assert!(report.first_time.is_none());
                        if case == "ok" {
                            let pending = result.unwrap();
                            let pids = pending.pids_for_test();
                            assert!(pids[0] > 0 && pids[1] > 0 && pids[0] != pids[1]);
                            let ((again, checks), report) = trace::capture(|| {
                                trace::stage(Stage::Checks);
                                probe.revalidate_for_test(&pending)
                            });
                            again.unwrap();
                            assert_eq!(checks, 2);
                            assert_eq!(
                                report.stages[Stage::Checks as usize].authentication_calls,
                                34
                            );
                            assert_eq!(report.stages[Stage::Checks as usize].signature_calls, 34);
                            busy(root);
                            drop(pending);
                            busy(root);
                        } else {
                            assert!(result.is_err(), "{case}");
                            let reason = format!("{:?}", result.err().unwrap());
                            assert!(
                                !reason.contains("Expired") && !reason.contains("Time"),
                                "unexpected time failure in {case}: {reason}"
                            );
                            let (retry, checks) = probe.retry_for_test();
                            assert!(retry.is_err());
                            assert_eq!(checks, 0);
                        }
                        drop(probe);
                        assert_eq!(&frame.origins_for_test()[..4], &origins[..4]);
                        assert_eq!(
                            frame.offsets_for_test(),
                            [300_000_000_000, 60_000_000_000, 10_000_000_000]
                        );
                        busy(root);
                        drop(frame);
                        assert_eq!(records(leaf), before);
                        retained_context_refuses_for_test(root);
                        println!("HC074-ORIGINAL-PENDING case={case} frame_auth={expected} channel_checks={checks} original_bounds=true test_channel_only=true enroll=false send=false");
                    });
                },
            );
        });
        assert!(
            refusal.is_none(),
            "unexpected kernel refusal in {case}: {refusal:?}"
        );
    }
}
