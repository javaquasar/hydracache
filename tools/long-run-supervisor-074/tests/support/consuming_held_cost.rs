//! Observer regression controls; no guard clock, owner or admission capability.
use crate::diagnostic_ipc::validation_trace as trace;
use std::time::Instant;

fn cpu() -> Option<u128> {
    let mut value: libc::timespec = unsafe { std::mem::zeroed() };
    if unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut value) } != 0
        || value.tv_sec < 0
        || !(0..1_000_000_000).contains(&value.tv_nsec)
    {
        return None;
    }
    Some(value.tv_sec as u128 * 1_000_000_000 + value.tv_nsec as u128)
}
fn observe_with_cpu<T>(
    call: impl FnOnce() -> T,
    mut cpu: impl FnMut() -> Option<u128>,
) -> (std::thread::Result<T>, serde_json::Value) {
    let wall = Instant::now();
    let before = cpu();
    let (((result, auth), live), kernel) = trace::kernel_walk::capture(|| {
        trace::live_walk::capture(|| {
            trace::capture(|| std::panic::catch_unwind(std::panic::AssertUnwindSafe(call)))
        })
    });
    let process_cpu_ns = before.zip(cpu()).and_then(|(a, b)| b.checked_sub(a));
    let wall_ns = wall.elapsed().as_nanos();
    let kernel: Vec<_> = kernel.nodes.iter().map(|s| serde_json::json!({"calls":s.calls,"inclusive_ns":s.inclusive_ns,"outside_children_ns":s.outside_children_ns})).collect();
    let live: Vec<_> = live.iter().map(|s| serde_json::json!({"calls":s.calls,"inclusive_ns":s.inclusive_ns,"outside_children_ns":s.outside_children_ns})).collect();
    let report = serde_json::json!({
        "wall_ns":wall_ns,"process_cpu_ns":process_cpu_ns,"returned":result.is_ok(),
        "authentication_calls":auth.stages.iter().map(|s|s.authentication_calls).sum::<u64>(),
        "authentication_ns":auth.stages.iter().map(|s|s.authentication_ns).sum::<u128>(),
        "signature_calls":auth.stages.iter().map(|s|s.signature_calls).sum::<u64>(),
        "signature_ns":auth.stages.iter().map(|s|s.signature_ns).sum::<u128>(),
        "kernel":kernel,"live":live,"overlapping_spans":true,"instrumented":true,
        "includes_child_cpu":false,"guard_clock_added":false,"deadline_changed":false,
        "healthy_gate":false,"server":false,"product":false
    });
    (result, report)
}
pub(super) fn profiled<T>(phase: &str, call: impl FnOnce() -> T) -> T {
    let (result, mut report) = observe_with_cpu(call, cpu);
    report["phase"] = phase.into();
    println!("HC074-HELD-PHASE {report}");
    match result {
        Ok(value) => value,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

#[test]
fn preparation_cost_observer_preserves_value_once_and_balanced_counters() {
    let calls = std::cell::Cell::new(0);
    let (result, report) = observe_with_cpu(
        || {
            calls.set(calls.get() + 1);
            let _kernel = trace::kernel_walk::span(trace::kernel_walk::Kind::Boot);
            let _live = trace::live_walk::span(trace::live_walk::Kind::KernelContext);
            let _auth = trace::authentication();
            Err::<(), _>(74)
        },
        || Some(100),
    );
    assert_eq!(result.unwrap(), Err(74));
    assert_eq!(calls.get(), 1);
    assert_eq!(report["kernel"][5]["calls"], 1);
    assert_eq!(report["live"][4]["calls"], 1);
    assert_eq!(report["authentication_calls"], 1);
    assert_eq!(report["process_cpu_ns"], 0);
    assert_eq!(report["healthy_gate"], false);
    assert_eq!(report["guard_clock_added"], false);
    assert_eq!(report["includes_child_cpu"], false);
}

#[test]
fn preparation_cost_observer_failure_does_not_mask_original_panic_or_reuse_counts() {
    let marker = std::sync::Arc::new(74u32);
    let calls = std::cell::Cell::new(0);
    let (result, report) = observe_with_cpu(
        || {
            calls.set(calls.get() + 1);
            let _kernel = trace::kernel_walk::span(trace::kernel_walk::Kind::Origin);
            std::panic::panic_any(std::sync::Arc::clone(&marker));
        },
        || None,
    );
    assert_eq!(calls.get(), 1);
    assert!(std::sync::Arc::ptr_eq(
        &result
            .unwrap_err()
            .downcast::<std::sync::Arc<u32>>()
            .unwrap(),
        &marker
    ));
    assert_eq!(report["returned"], false);
    assert!(report["process_cpu_ns"].is_null());
    assert_eq!(report["kernel"][4]["calls"], 1);
    let (_, next) = observe_with_cpu(|| (), || Some(100));
    assert_eq!(next["kernel"][4]["calls"], 0);
    assert_eq!(next["authentication_calls"], 0);
}

#[test]
fn preparation_cost_observer_restores_outer_frames_and_refuses_cpu_underflow() {
    let (_, outer) = trace::kernel_walk::capture(|| {
        let _outer = trace::kernel_walk::span(trace::kernel_walk::Kind::Sample);
        let mut cpu = [2, 1].into_iter();
        let (_, report) = observe_with_cpu(|| 74, || cpu.next());
        assert!(report["process_cpu_ns"].is_null());
    });
    assert_eq!(outer.nodes[7].calls, 1);
    assert_eq!(outer.nodes.iter().map(|s| s.calls).sum::<u64>(), 1);
}
