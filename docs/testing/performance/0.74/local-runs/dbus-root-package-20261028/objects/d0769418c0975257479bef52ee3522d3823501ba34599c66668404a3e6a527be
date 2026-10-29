//! One fixed name, the same proc directory and the same open/fstatfs/close path.
use super::*;

#[test]
#[ignore = "manual local nonroot fresh status allocation control; not product timing"]
fn fresh_status_opener_allocation_attribution() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let origin = Origin::open().unwrap();
    let parent = child(&origin.root, &std::process::id().to_string(), true).unwrap();
    let run = |fixed: bool, iterations| {
        for _ in 0..iterations {
            let value = if fixed {
                fixed_child(&parent, c"status", false)
            } else {
                child(&parent, "status", false)
            };
            std::hint::black_box(value.unwrap());
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
            "HC074-FRESH-STATUS-ALLOC {}",
            serde_json::json!({"sample":sample,"iterations":1024,"reference":old,"candidate":new,"candidate_first":sample%2==1,"rust_requests_only":true,"server":false,"product":false})
        );
        assert_eq!(old.allocations, 1024);
        assert_eq!(old.requested_bytes, 7 * 1024);
        assert_eq!(old.reallocations, 0);
        assert_eq!(new, Counts::default());
    }
    for sample in 0..3 {
        let timed = |fixed| {
            let started = std::time::Instant::now();
            run(fixed, 4096);
            started.elapsed().as_nanos()
        };
        let (old, new) = if sample % 2 == 0 {
            (timed(false), timed(true))
        } else {
            let new = timed(true);
            (timed(false), new)
        };
        println!(
            "HC074-FRESH-STATUS-TIME {}",
            serde_json::json!({"sample":sample,"iterations":4096,"reference_wall_ns":old,"candidate_wall_ns":new,"candidate_first":sample%2==1,"allocation_counter_active":false,"server":false,"product":false})
        );
    }
    origin.revalidate().unwrap();
}
