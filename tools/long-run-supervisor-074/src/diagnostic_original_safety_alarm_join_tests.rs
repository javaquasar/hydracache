use super::*;
#[path = "diagnostic_grouped_preparation_join_tests.rs"]
mod grouped_tests;
#[path = "diagnostic_validation_leaf_cost_tests.rs"]
mod leaf_cost_tests;
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::safety_alarm::OriginalSafetyAlarmError;

#[test]
#[ignore = "manual nonroot original alarm cost attribution; a diagnostic pass is not healthy admission"]
fn original_safety_alarm_cost_attribution() {
    use crate::diagnostic_ipc::validation_trace::{self as trace, Stage};
    for sample in 0..3 {
        let bundle = Bundle::new();
        with_original_at(true, None, |original, _, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let original = accepted(original, fence, leaf, receiver, sender, model);
                let (outputs, report) = trace::capture(|| {
                    trace::stage(Stage::Bind);
                    original
                        .prepare_original_outputs_for_test(|_| Ok(()))
                        .unwrap()
                });
                println!("HC074-ALARM-ATTRIBUTION sample={sample} phase=outputs report={report:?}");
                let before = records(leaf);
                let (material, report) = trace::capture(|| {
                    trace::stage(Stage::Bind);
                    outputs
                        .prepare_original_start_material_for_test(&bundle.path, || {})
                        .unwrap()
                });
                println!(
                    "HC074-ALARM-ATTRIBUTION sample={sample} phase=material report={report:?}"
                );
                let (transport, report) = trace::capture(|| {
                    trace::stage(Stage::Bind);
                    material.prepare_original_pipe_transport_for_test().unwrap()
                });
                println!("HC074-ALARM-ATTRIBUTION sample={sample} phase=message report={report:?}");
                let (result, report) = trace::capture(|| {
                    trace::stage(Stage::Bind);
                    transport.prepare_original_safety_alarm_for_test()
                });
                println!("HC074-ALARM-ATTRIBUTION sample={sample} phase=alarm report={report:?} result={result:?}");
                match result {
                    Err(failure) => {
                        assert!(failure.is_refused());
                        assert!(
                            format!("{:?}", failure.error()).contains("Causal(Time)"),
                            "{failure:?}"
                        );
                        busy(root);
                        drop(failure);
                    }
                    Ok(mut alarm) => {
                        println!(
                            "HC074-ALARM-ORIGINS sample={sample} before={:?} offsets_ns={:?}",
                            alarm.origins_for_test(),
                            alarm.offsets_for_test()
                        );
                        let (result, report) = trace::capture(|| {
                            trace::stage(Stage::Checks);
                            alarm.revalidate_fixture_for_test()
                        });
                        println!("HC074-ALARM-ATTRIBUTION sample={sample} phase=revalidate report={report:?} result={result:?}");
                        println!("HC074-ALARM-ORIGINS sample={sample} after={:?} post_error_clock_not_first_event=true", alarm.origins_for_test());
                        {
                            let e = result.expect_err("this diagnostic characterizes the preregistered time refusal, not healthy admission");
                            assert!(format!("{e:?}").contains("Causal(Time)"), "{e:?}");
                            assert!(alarm.is_refused());
                            assert!(matches!(
                                alarm.revalidate_fixture_for_test(),
                                Err(OriginalSafetyAlarmError::Refused)
                            ));
                        }
                        busy(root);
                        drop(alarm);
                    }
                }
                assert_eq!(records(leaf), before);
                retained_context_refuses_for_test(root);
                println!("HC074-ALARM-ATTRIBUTION-CLOSED sample={sample} accepted=false renewed=false guardian=false send=false");
            });
        });
    }
}
