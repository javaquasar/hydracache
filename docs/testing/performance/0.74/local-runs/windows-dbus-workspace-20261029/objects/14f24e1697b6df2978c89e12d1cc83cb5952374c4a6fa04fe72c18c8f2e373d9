//! Local diagnostic wrapper around one unchanged original closing call.
use crate::diagnostic_ipc::validation_trace as trace;
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::time_observation as time;
use std::time::Instant;
pub(super) fn run<E: std::fmt::Debug>(call: impl FnOnce() -> Result<(), E>) -> Result<(), E> {
    match std::env::var("HC074_CLOSING_DIAGNOSTICS").as_deref() {
        Err(_) => call(),
        Ok(mode @ ("facts" | "cost")) => capture(mode, call),
        Ok(_) => panic!("unknown local closing diagnostic mode"),
    }
}
fn capture<E: std::fmt::Debug>(mode: &str, call: impl FnOnce() -> Result<(), E>) -> Result<(), E> {
    let started = Instant::now();
    let (((result, causal), age), cost) = if mode == "cost" {
        let (((((result, causal), age), auth), live), kernel) = trace::kernel_walk::capture(|| {
            trace::live_walk::capture(|| {
                trace::capture(|| {
                    trace::stage(trace::Stage::Checks);
                    time::closing::capture(|| time::capture(call))
                })
            })
        });
        let stats = auth.stages[trace::Stage::Checks as usize];
        let kernel = kernel.nodes;
        (
            ((result, causal), age),
            serde_json::json!({
                "authentication_calls":stats.authentication_calls,"authentication_ns":stats.authentication_ns,
                "signature_calls":stats.signature_calls,"signature_ns":stats.signature_ns,
                "live":live.map(|s| [s.calls as u128,s.inclusive_ns,s.outside_children_ns]),
                "kernel":kernel.map(|s| [s.calls as u128,s.inclusive_ns,s.outside_children_ns]),
                "wall_not_cpu":true,"overlapping_spans":true
            }),
        )
    } else {
        (
            time::closing::capture(|| time::capture(call)),
            serde_json::Value::Null,
        )
    };
    let wall = started.elapsed().as_nanos();
    println!(
        "HC074-CLOSING {}",
        serde_json::json!({"mode":mode,"success":result.is_ok(),"wall_ns":wall,"causal":causal,"age":age,"cost":cost,"schedule_changed":false,"deadline_changed":false,"product":false})
    );
    result
}
#[test]
fn closing_validation_wrapper_calls_once_and_returns_original_error() {
    let calls = std::cell::Cell::new(0);
    let result = capture("facts", || {
        calls.set(calls.get() + 1);
        Err::<(), _>("original-error")
    });
    assert_eq!(result, Err("original-error"));
    assert_eq!(calls.get(), 1);
}
#[test]
fn closing_validation_cost_capture_keeps_nested_observers_balanced() {
    let calls = std::cell::Cell::new(0);
    capture("cost", || {
        calls.set(calls.get() + 1);
        let _s = trace::kernel_walk::span(trace::kernel_walk::Kind::Boot);
        Ok::<_, ()>(())
    })
    .unwrap();
    assert_eq!(calls.get(), 1);
}
