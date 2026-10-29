//! Paired complete sender validators, not a product or latency claim.
use super::*;
use crate::diagnostic_kernel_admission::tests::document_scratch::current;
#[test]
#[ignore = "manual nonroot per-invocation document buffer allocation control"]
fn document_scratch_allocation_attribution() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let (origin, sender) = current();
    let run = |candidate: bool, iterations| {
        for _ in 0..iterations {
            std::hint::black_box(if candidate {
                sender.scratch_prototype(&origin)
            } else {
                sender.scratch_reference(&origin)
            })
            .unwrap();
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
            "HC074-DOCUMENT-SCRATCH-ALLOC {}",
            serde_json::json!({"sample":sample,"iterations":1024,"reference":old,"candidate":new,"candidate_first":sample%2==1,"rust_requests_only":true,"server":false,"product":false})
        );
        assert_eq!(old.allocations - new.allocations, 3 * 1024);
        assert_eq!(old.requested_bytes - new.requested_bytes, 3 * 65537 * 1024);
        assert_eq!(old.reallocations, new.reallocations);
    }
    for sample in 0..3 {
        let timed = |candidate| {
            let start = std::time::Instant::now();
            run(candidate, 4096);
            start.elapsed().as_nanos()
        };
        let (old, new) = if sample % 2 == 0 {
            (timed(false), timed(true))
        } else {
            let new = timed(true);
            (timed(false), new)
        };
        println!(
            "HC074-DOCUMENT-SCRATCH-TIME {}",
            serde_json::json!({"sample":sample,"iterations":4096,"reference_wall_ns":old,"candidate_wall_ns":new,"allocation_counter_active":false,"server":false,"product":false})
        );
    }
}
