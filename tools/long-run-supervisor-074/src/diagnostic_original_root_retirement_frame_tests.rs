//! Real original predicates, same retirement function; simulated kernel only.
use super::*;
#[test]
#[ignore = "manual nonroot original retirement frame; no original root constructor or send"]
fn original_root_retirement_frame_control() {
    let (_, refusal) = crate::diagnostic_kernel_admission::refusal_trace::capture(|| {
        let bundle = Bundle::new();
        with_original_at(true, None, |original, _paths, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let mut frame = accepted(original, fence, leaf, receiver, sender, model)
                    .prepare_original_outputs_for_test(|_| Ok(()))
                    .unwrap()
                    .prepare_enrollment_fixture_for_test(&bundle.path, 0, |_, _| {})
                    .unwrap();
                let before = records(leaf);
                let origins = frame.origins_for_test();
                let mut probe = frame.pending_probe_for_test("ok");
                let mut controls = probe.root_controls_for_test(0);
                let ((accepted, calls), report) = trace::capture(|| {
                    trace::stage(Stage::Checks);
                    probe.root_join_for_test(&mut controls, true)
                });
                accepted.unwrap();
                assert_eq!(calls, 4);
                assert_eq!(
                    report.stages[Stage::Checks as usize].authentication_calls,
                    68
                );
                assert_eq!(report.stages[Stage::Checks as usize].signature_calls, 68);
                let ((retry, calls), report) = trace::capture(|| {
                    trace::stage(Stage::Checks);
                    probe.root_retire_for_test(&mut controls);
                    // A new capsule cannot hide behind the old one's refusal.
                    let mut fresh = probe.root_controls_for_test(0);
                    probe.root_join_for_test(&mut fresh, true)
                });
                assert!(retry.is_err());
                assert_eq!(calls, 0);
                assert_eq!(
                    report.stages[Stage::Checks as usize].authentication_calls,
                    0
                );
                assert_eq!(report.stages[Stage::Checks as usize].signature_calls, 0);
                assert!(report.first_time.is_none());
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
                println!("HC074-ROOT-RETIREMENT frame=true auth=68 channel_checks=4 retired_retry_auth=0 retired_retry_channel=0 original_bounds=true kernel=simulated original_constructor=false send=false");
            });
        });
    });
    assert!(refusal.is_none(), "unexpected kernel refusal: {refusal:?}");
}
