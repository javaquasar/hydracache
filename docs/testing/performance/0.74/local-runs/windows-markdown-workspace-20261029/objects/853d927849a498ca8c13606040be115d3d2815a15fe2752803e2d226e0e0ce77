//! Separate original-object controls, not a replacement or decomposition of ReservedLive.
use super::*;

#[path = "diagnostic_original_live_walk_cost_tests.rs"]
mod live_walk_tests;

#[test]
#[ignore = "manual nonroot original controller component controls; no reservation, execution or admission"]
fn original_controller_component_cost_controls() {
    let names = [
        "worker-live",
        "controller-source",
        "signed-controller",
        "builder-body-data-only",
        "build-validator",
        "oidc-source",
        "oidc-token-data-only",
        "oidc-validator",
        "authority-binding",
        "worker-time",
    ];
    for sample in 0..3 {
        with_original_at(false, None, |original, paths, _, _, _| {
            original.revalidate().unwrap();
            let request = original.original.original.request.clone();
            let worker = fs::read(&paths.worker).unwrap();
            let oidc = fs::read(&paths.oidc).unwrap();
            let mut visited = [false; 10];
            for order in 0..10 {
                let index = (order + sample * 3) % 10;
                assert!(!visited[index]);
                visited[index] = true;
                // This isolated data-only control excludes its original clock sampling.
                let token_now = if index == 6 {
                    Some(
                        original
                            .original
                            .original
                            .worker
                            .original_unix_seconds()
                            .unwrap(),
                    )
                } else {
                    None
                };
                let (_, report) = trace::capture(|| {
                    trace::stage(Stage::Checks);
                    match index {
                        0 => original.original.original.worker.revalidate().unwrap(),
                        1 => original.original.original.controller.check().unwrap(),
                        2 => original.original.original.revalidate().unwrap(),
                        3 => {
                            let build = &original.original;
                            let pinned = crate::diagnostic_reviewed_trust::load_builder_policy(
                                &build.policy_bytes,
                            )
                            .unwrap();
                            let identity = &build.original.request.identity;
                            let checked = pinned
                                .verify_receipt(&build.receipt_bytes, identity)
                                .unwrap();
                            let retained = build
                                .policy
                                .verify_receipt(&build.receipt_bytes, identity)
                                .unwrap();
                            assert_eq!(checked.identity(), build.build.identity());
                            assert_eq!(checked.statement(), build.build.statement());
                            assert_eq!(retained.identity(), checked.identity());
                            assert_eq!(retained.statement(), checked.statement());
                        }
                        4 => original.original.revalidate().unwrap(),
                        5 => original.trust.check().unwrap(),
                        6 => {
                            let digest = crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::profile::verify(
                                &original.trust.source.profile, &original.token,
                                &original.original.original.request,
                                &original.original.original.request_sha256,
                                token_now.unwrap(),
                            ).unwrap();
                            assert_eq!(digest, original.token_digest);
                        }
                        7 => original.revalidate().unwrap(),
                        8 => {
                            let _ = original
                                .original
                                .original
                                .worker
                                .controller_authority_binding()
                                .unwrap();
                        }
                        9 => {
                            let _ = original
                                .original
                                .original
                                .worker
                                .original_unix_seconds()
                                .unwrap();
                        }
                        _ => unreachable!(),
                    }
                });
                let expected = match index {
                    2 => 1,
                    4 => 2,
                    7 => 4,
                    _ => 0,
                };
                let stats = report.stages[Stage::Checks as usize];
                assert_eq!(stats.authentication_calls, expected);
                assert_eq!(stats.signature_calls, expected);
                assert_eq!(
                    report
                        .stages
                        .iter()
                        .map(|s| s.authentication_calls)
                        .sum::<u64>(),
                    expected
                );
                assert!(report.first_time.is_none() && report.time_refusals == 0);
                assert!(!original.is_refused());
                println!("HC074-COMPONENT-COST sample={sample} order={order} component={} stats={stats:?}", names[index]);
            }
            assert!(visited.into_iter().all(|seen| seen));
            original.revalidate().unwrap();
            assert_eq!(original.original.original.request, request);
            assert_eq!(fs::read(&paths.worker).unwrap(), worker);
            assert_eq!(fs::read(&paths.oidc).unwrap(), oidc);
            assert!(!original.is_refused());
            println!("HC074-COMPONENT-CONTROL sample={sample} components=10 original=true unchanged=true reservation=false production_changed=false schedule_changed=false admission=false");
        });
    }
}
