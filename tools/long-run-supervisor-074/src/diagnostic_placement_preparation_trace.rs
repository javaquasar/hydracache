//! Test-only copies across the original preparation; never an admission clock.
use super::time_observation as time;
use std::any::Any;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

struct Observation {
    causal: time::Report,
    age: time::closing::Report,
}

#[path = "diagnostic_placement_preparation_cost.rs"]
pub(super) mod cost;
type Unwind<T> = Result<T, Box<dyn Any + Send>>;
fn inspect<T>(call: impl FnOnce() -> T) -> (Unwind<T>, Observation) {
    // Catch inside both captures so failed preparation facts survive unwind.
    // The original body, destructor order and panic payload are unchanged.
    let ((result, causal), age) =
        time::closing::capture(|| time::capture(|| catch_unwind(AssertUnwindSafe(call))));
    (result, Observation { causal, age })
}
pub(super) fn capture(case: &str, call: impl FnOnce()) {
    let (result, facts) = inspect(call);
    println!(
        "HC074-PLACEMENT-PREPARATION {}",
        serde_json::json!({
            "case":case,"returned":result.is_ok(),"causal":facts.causal,"age":facts.age,
            "schedule_changed":false,"deadline_changed":false,"new_clock":false,
            "original_send":false,"server":false,"product":false
        })
    );
    if let Err(payload) = result {
        resume_unwind(payload);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::sync::Arc;

    fn json(facts: Observation) -> serde_json::Value {
        serde_json::json!({"causal":facts.causal,"age":facts.age})
    }
    #[test]
    fn preparation_trace_returns_the_original_value_error_and_calls_once() {
        let calls = Cell::new(0);
        let (value, facts) = inspect(|| {
            calls.set(calls.get() + 1);
            Err::<u32, _>("original error")
        });
        assert_eq!(value.unwrap(), Err("original error"));
        assert_eq!(calls.get(), 1);
        assert_eq!(json(facts)["causal"]["observations"], 0);
        assert_eq!(inspect(|| 74).0.unwrap(), 74);
    }
    #[test]
    fn preparation_trace_retains_first_raw_refusal_and_original_panic_identity() {
        let payload = Arc::new(74u32);
        let (value, facts) = inspect(|| {
            time::closing::observe(5, 6);
            time::closing::observe(5, 10_000_000_005);
            time::closing::observe(5, 4);
            std::panic::panic_any(Arc::clone(&payload));
        });
        assert!(Arc::ptr_eq(
            &value.unwrap_err().downcast::<Arc<u32>>().unwrap(),
            &payload
        ));
        let facts = json(facts);
        let site = &facts["age"]["sites"][2];
        assert_eq!(site["calls"], 3);
        assert_eq!(site["first_refusal"]["issued_ns"], 5);
        assert_eq!(site["first_refusal"]["now_ns"], 10_000_000_005u64);
        assert_eq!(site["first_refusal"]["reason"], "expired");
        assert_eq!(site["last"]["reason"], "backward-boottime");
    }
    #[test]
    fn preparation_trace_capture_resumes_the_same_panic_without_retry() {
        let calls = Cell::new(0);
        let payload = Arc::new(75u32);
        let result = catch_unwind(AssertUnwindSafe(|| {
            capture("unit-panic", || {
                calls.set(calls.get() + 1);
                std::panic::panic_any(Arc::clone(&payload));
            });
        }));
        assert_eq!(calls.get(), 1);
        assert!(Arc::ptr_eq(
            &result.unwrap_err().downcast::<Arc<u32>>().unwrap(),
            &payload
        ));
    }
    #[test]
    fn preparation_trace_nested_and_concurrent_captures_restore_outer_facts() {
        let (_, facts) = inspect(|| {
            time::closing::observe(1, 2);
            let (_, nested) = inspect(|| time::closing::observe(2, 3));
            assert_eq!(json(nested)["age"]["sites"][2]["calls"], 1);
            let other = std::thread::spawn(|| inspect(|| ()).1).join().unwrap();
            assert_eq!(json(other)["age"]["sites"][2]["calls"], 0);
            time::closing::observe(1, 4);
        });
        let facts = json(facts);
        assert_eq!(facts["age"]["sites"][2]["calls"], 2);
        assert_eq!(facts["age"]["sites"][2]["first"]["now_ns"], 2);
        assert_eq!(facts["age"]["sites"][2]["last"]["now_ns"], 4);
        assert_eq!(json(inspect(|| ()).1)["age"]["sites"][2]["calls"], 0);
    }
    #[test]
    fn preparation_trace_copies_already_received_kernel_time_and_boot_exactly() {
        use crate::diagnostic_kernel_admission::KernelReceiver;
        use std::os::fd::{FromRawFd, OwnedFd};
        use std::os::unix::net::UnixDatagram;
        let mut fds = [-1; 2];
        assert_eq!(
            unsafe {
                libc::socketpair(
                    libc::AF_UNIX,
                    libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC,
                    0,
                    fds.as_mut_ptr(),
                )
            },
            0
        );
        let mut receiver = KernelReceiver::new(unsafe { OwnedFd::from_raw_fd(fds[0]) }).unwrap();
        let sender = unsafe { UnixDatagram::from_raw_fd(fds[1]) };
        sender.send(b"already observed time only").unwrap();
        let packet = receiver.try_receive().unwrap().unwrap();
        let point = packet.received_time();
        let (_, facts) = inspect(|| time::observe("unit-existing-point", point, point));
        let facts = json(facts);
        let last = &facts["causal"]["last"];
        assert_eq!(last["issued_ns"], point.boottime_ns());
        assert_eq!(last["now_ns"], point.boottime_ns());
        assert_eq!(last["issued_boot"], point.boot_id());
        assert_eq!(last["now_boot"], point.boot_id());
        assert_eq!(last["age_ns"], 0);
        assert_eq!(last["diagnostic_reason"], "within-window");
    }
}
