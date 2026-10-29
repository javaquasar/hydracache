//! Test-only overlapping wall attribution, never a healthy gate or clock source.
use crate::diagnostic_ipc::validation_trace as trace;
use std::time::Instant;
const STAGES: [trace::Stage; 8] = [
    trace::Stage::Unassigned,
    trace::Stage::Setup,
    trace::Stage::Fence,
    trace::Stage::Retain,
    trace::Stage::Observe,
    trace::Stage::Publish,
    trace::Stage::Bind,
    trace::Stage::Checks,
];
const NAMES: [&str; 8] = [
    "enroll-join-cleanup",
    "cold-setup-original-admission",
    "fence-reserve-publish-challenge",
    "response-consent",
    "outputs",
    "enrollment-frame",
    "fixed-manager-message",
    "owned-root-preparation",
];
pub(crate) fn phase(index: usize) {
    // The existing disabled stage recorder does not read Instant or a guard clock.
    trace::stage(STAGES[index]);
}
fn observe<T>(call: impl FnOnce() -> T) -> (super::Unwind<T>, serde_json::Value) {
    let start = Instant::now();
    let ((((result, facts), auth), live), kernel) = trace::kernel_walk::capture(|| {
        trace::live_walk::capture(|| {
            trace::capture(|| {
                phase(1);
                super::inspect(call)
            })
        })
    });
    let wall = start.elapsed().as_nanos();
    let phases: Vec<_> = auth.stages.iter().enumerate().map(|(i,s)| serde_json::json!({
        "phase":NAMES[i],"wall_ns":s.wall_ns,"authentication_calls":s.authentication_calls,"authentication_ns":s.authentication_ns,
        "signature_calls":s.signature_calls,"signature_ns":s.signature_ns,"worker_policy_calls":s.worker_policy_calls,"worker_policy_ns":s.worker_policy_ns
    })).collect();
    let report = serde_json::json!({"returned":result.is_ok(),"wall_ns":wall,"phases":phases,
        "causal":facts.causal,"age":facts.age,
        "live":live.map(|s| [s.calls as u128,s.inclusive_ns,s.outside_children_ns]),
        "kernel":kernel.nodes.map(|s| [s.calls as u128,s.inclusive_ns,s.outside_children_ns]),
        "wall_not_cpu":true,"overlapping_spans":true,"instrumented":true,
        "guard_clock_added":false,"schedule_changed":false,"deadline_changed":false,"healthy_gate":false,
        "original_send":false,"server":false,"product":false});
    (result, report)
}
pub(crate) fn capture(case: &str, call: impl FnOnce()) {
    let (result, mut report) = observe(call);
    report["case"] = case.into();
    println!("HC074-PLACEMENT-COST {report}");
    if let Err(payload) = result {
        super::resume_unwind(payload);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn placement_cost_preserves_value_once_and_balanced_spans() {
        let calls = std::cell::Cell::new(0);
        phase(0); // Disabled recorder; no capture to activate a timer.
        let (result, report) = observe(|| {
            calls.set(calls.get() + 1);
            phase(3);
            let _span = trace::kernel_walk::span(trace::kernel_walk::Kind::Boot);
            let _live = trace::live_walk::span(trace::live_walk::Kind::KernelContext);
            Err::<(), _>("original-error")
        });
        assert_eq!(result.unwrap(), Err("original-error"));
        assert_eq!(calls.get(), 1);
        assert_eq!(report["phases"].as_array().unwrap().len(), 8);
        assert_eq!(report["kernel"][5][0], 1);
        assert_eq!(report["live"][4][0], 1);
        assert_eq!(report["healthy_gate"], false);
    }
    #[test]
    fn placement_cost_keeps_refusal_and_original_panic_payload() {
        let payload = std::sync::Arc::new(74u32);
        let (result, report) = observe(|| {
            phase(7);
            super::super::time::closing::observe(5, 10_000_000_005);
            std::panic::panic_any(std::sync::Arc::clone(&payload));
        });
        assert!(std::sync::Arc::ptr_eq(
            &result
                .unwrap_err()
                .downcast::<std::sync::Arc<u32>>()
                .unwrap(),
            &payload
        ));
        assert_eq!(
            report["age"]["sites"][2]["first_refusal"]["reason"],
            "expired"
        );
        assert_eq!(report["returned"], false);
        assert_eq!(observe(|| ()).1["kernel"][5][0], 0);
    }
}
