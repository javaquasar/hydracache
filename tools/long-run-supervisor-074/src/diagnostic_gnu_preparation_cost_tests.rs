//! Original preparation, not a replacement validation schedule or healthy gate.
use super::*;
use crate::diagnostic_ipc::validation_trace::{kernel_walk as kernel, live_walk};
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::time_observation as clock;

fn phase<T>(name: &str, run: impl FnOnce() -> T) -> T {
    let (((value, facts), nodes), walks, report) = {
        let ((value, walks), report) = trace::capture(|| {
            trace::stage(Stage::Bind);
            live_walk::capture(|| kernel::capture(|| clock::capture(run)))
        });
        (value, walks, report)
    };
    let stats = report.stages[Stage::Bind as usize];
    let node_json: Vec<_> = nodes.nodes.iter().enumerate().map(|(index, s)| serde_json::json!({
        "index":index,"calls":s.calls,"inclusive_ns":s.inclusive_ns,"outside_children_ns":s.outside_children_ns
    })).collect();
    let walk_json: Vec<_> = walks.iter().enumerate().map(|(index, s)| serde_json::json!({
        "index":index,"calls":s.calls,"inclusive_ns":s.inclusive_ns,"outside_children_ns":s.outside_children_ns
    })).collect();
    println!(
        "HC074-PREPARATION {}",
        serde_json::json!({
            "phase":name,"wall_ns":stats.wall_ns,"auth_calls":stats.authentication_calls,
            "auth_ns":stats.authentication_ns,"signature_calls":stats.signature_calls,
            "signature_ns":stats.signature_ns,"worker_policy_calls":stats.worker_policy_calls,
            "worker_policy_ns":stats.worker_policy_ns,"clock":facts,"kernel":node_json,"live":walk_json
        })
    );
    value
}

#[test]
#[ignore = "manual nonroot original GNU preparation attribution; refusal is not healthy admission"]
fn gnu_preparation_cost_attribution() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let bundle = Bundle::new();
    with_original_at(true, None, |original, _, receiver, sender, model| {
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let frame = phase("consent", || {
                accepted(original, fence, leaf, receiver, sender, model)
            });
            let result = phase("outputs", || {
                frame.prepare_original_outputs_for_test(|_| Ok(()))
            });
            let outputs = match result {
                Ok(outputs) => outputs,
                Err(failure) => {
                    assert!(failure.is_refused());
                    println!(
                        "HC074-PREPARATION-OUTCOME phase=outputs healthy=false error={:?}",
                        failure.error()
                    );
                    busy(root);
                    drop(failure);
                    retained_context_refuses_for_test(root);
                    return;
                }
            };
            let before = records(leaf);
            let result = phase("enrollment", || {
                outputs.prepare_enrollment_fixture_for_test(&bundle.path, 0, |_, _| {})
            });
            match result {
                Ok(mut frame) => {
                    assert_eq!(
                        frame.offsets_for_test(),
                        [300_000_000_000, 60_000_000_000, 10_000_000_000]
                    );
                    let mut probe = frame.pending_probe_for_test("ok");
                    let mut controls = probe.root_controls_for_test(0);
                    probe.root_retire_for_test(&mut controls);
                    drop(probe);
                    busy(root);
                    drop(frame);
                    println!("HC074-PREPARATION-OUTCOME phase=enrollment healthy=true kernel=simulated original_constructor=false send=false");
                }
                Err(failure) => {
                    // The outer original fixture asserts sticky refusal after Drop.
                    println!(
                        "HC074-PREPARATION-OUTCOME phase=enrollment healthy=false error={:?}",
                        failure
                    );
                    busy(root);
                    drop(failure);
                }
            }
            assert_eq!(records(leaf), before);
            retained_context_refuses_for_test(root);
        });
    });
    println!("HC074-PREPARATION-CLOSED admission=false renewal=false send=false server=false product=false");
}
