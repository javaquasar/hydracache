//! Paired fresh proc opens; not an API, libc allocation or latency claim.
use super::*;
use crate::diagnostic_kernel_admission::origin::pid_path::PidName;

#[test]
#[ignore = "manual local nonroot uncached PID pathname control; no execution or admission"]
fn pid_path_allocation_attribution() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let root = Origin::open().unwrap();
    let pid = std::process::id() as i32;
    let digits = pid.to_string().len();
    let run = |stack: bool, iterations| {
        for _ in 0..iterations {
            let pid = std::hint::black_box(pid);
            let result = if stack {
                fixed_child(&root.root, PidName::new(pid).as_c_str(), true)
            } else {
                child(&root.root, &pid.to_string(), true)
            };
            std::hint::black_box(result.unwrap());
        }
    };
    run(false, 32);
    run(true, 32);
    for sample in 0..3 {
        let (old, new) = if sample % 2 == 0 {
            (
                capture(|| run(false, 1024)).1,
                capture(|| run(true, 1024)).1,
            )
        } else {
            let new = capture(|| run(true, 1024)).1;
            (capture(|| run(false, 1024)).1, new)
        };
        println!(
            "HC074-PID-ALLOC {}",
            serde_json::json!({"sample":sample,"iterations":1024,"pid":pid,"digits":digits,"reference":old,"candidate":new,"candidate_first":sample%2==1,"rust_requests_only":true,"server":false,"product":false})
        );
        assert_eq!(old.allocations, 2048);
        assert_eq!(old.reallocations, 0);
        assert!(old.requested_bytes > 0);
        assert_eq!(new, Counts::default());
    }
    for sample in 0..3 {
        let timed = |stack| {
            let start = std::time::Instant::now();
            run(stack, 4096);
            start.elapsed().as_nanos()
        };
        let (old, new) = if sample % 2 == 0 {
            (timed(false), timed(true))
        } else {
            let new = timed(true);
            (timed(false), new)
        };
        println!(
            "HC074-PID-TIME {}",
            serde_json::json!({"sample":sample,"iterations":4096,"reference_wall_ns":old,"candidate_wall_ns":new,"candidate_first":sample%2==1,"allocation_counter_active":false,"server":false,"product":false})
        );
    }
    root.revalidate().unwrap();
}
