//! Original frame/signatures/channel checks, simulated kernel explicitly.
use super::*;
#[test]
#[ignore = "manual nonroot full original frame and simulated root kernel; no root enrollment or send"]
fn original_root_guardian_frame_controls() {
    for (case, kernel_fault, expected, channel_checks) in [
        ("ok", 0, 68, 4),
        ("retained-origin", 0, 0, 0),
        ("closing-origin", 0, 34, 2),
        ("inside-custody-missing", 0, 34, 3),
        ("kernel-opening", 1, 0, 0),
        ("kernel-before-send", 2, 34, 2),
        ("kernel-closing", 3, 68, 4),
        ("kernel-send-uncertain", 4, 34, 2),
    ] {
        let (_, refusal) = crate::diagnostic_kernel_admission::refusal_trace::capture(|| {
            let bundle = Bundle::new();
            with_original_at(
                true, // even healthy case deliberately attempts duplicate enrollment below
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
                        let mut controls = probe.root_controls_for_test(kernel_fault);
                        let ((result, checks), report) = trace::capture(|| {
                            trace::stage(Stage::Checks);
                            probe.root_join_for_test(&mut controls, true)
                        });
                        assert_eq!(
                            report.stages[Stage::Checks as usize].authentication_calls,
                            expected,
                            "{case}"
                        );
                        assert_eq!(
                            report.stages[Stage::Checks as usize].signature_calls,
                            expected,
                            "{case}"
                        );
                        assert_eq!(checks, channel_checks, "{case}");
                        assert!(report.first_time.is_none());
                        if case == "ok" {
                            result.unwrap();
                            let ((again, checks), report) = trace::capture(|| {
                                trace::stage(Stage::Checks);
                                probe.root_join_for_test(&mut controls, false)
                            });
                            again.unwrap();
                            assert_eq!(checks, 2);
                            assert_eq!(
                                report.stages[Stage::Checks as usize].authentication_calls,
                                34
                            );
                            assert_eq!(report.stages[Stage::Checks as usize].signature_calls, 34);
                            assert!(probe.root_join_for_test(&mut controls, true).0.is_err());
                        } else {
                            let reason = format!("{:?}", result.err().unwrap());
                            assert!(
                                !reason.contains("Expired") && !reason.contains("Time"),
                                "{case}: {reason}"
                            );
                        }
                        let (retry, calls) = probe.root_join_for_test(&mut controls, true);
                        assert!(retry.is_err());
                        assert_eq!(calls, 0);
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
                        println!("HC074-ORIGINAL-ROOT-FRAME case={case} auth={expected} channel_checks={channel_checks} original_bounds=true kernel=simulated enrollment=false original_send=false");
                    });
                },
            );
        });
        assert!(
            refusal.is_none(),
            "unexpected kernel refusal {case}: {refusal:?}"
        );
    }
}

#[path = "diagnostic_original_root_retirement_frame_tests.rs"]
mod retirement;

#[path = "diagnostic_gnu_preparation_cost_tests.rs"]
mod preparation_cost;
