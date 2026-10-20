//! Same decoder bytes, paired Rust requests and separately uncounted timing.
use super::*;
use crate::diagnostic_kernel_admission::{projection, tests::projection_prefix::reference};

#[test]
fn projection_prefix_allocation_and_timing_attribution() {
    let credentials = libc::ucred {
        pid: 23,
        uid: 24,
        gid: 25,
    };
    let raw = b"Pid: 23\nTgid: 23\nUid: 24 24 24 24\nGid: 25 25 25 25\nGroups: 26 27\n";
    let run = |candidate: bool, iterations: usize| {
        for _ in 0..iterations {
            let bytes = std::hint::black_box(raw.as_slice());
            let value = if candidate {
                projection(bytes, &credentials)
            } else {
                reference(bytes, &credentials)
            };
            std::hint::black_box(value.unwrap());
        }
    };
    run(false, 32);
    run(true, 32);
    let mut improved = true;
    for sample in 0..3 {
        let (old, new) = if sample % 2 == 0 {
            (
                capture(|| run(false, 4096)).1,
                capture(|| run(true, 4096)).1,
            )
        } else {
            let new = capture(|| run(true, 4096)).1;
            (capture(|| run(false, 4096)).1, new)
        };
        println!(
            "HC074-PROJECTION-PREFIX-ALLOC {}",
            serde_json::json!({"sample":sample,"iterations":4096,"reference":old,"candidate":new,"candidate_first":sample%2==1,"rust_requests_only":true,"product":false})
        );
        improved &= new.allocations < old.allocations
            && new.reallocations <= old.reallocations
            && new.requested_bytes < old.requested_bytes;
    }
    for sample in 0..3 {
        let timed = |candidate| {
            let started = std::time::Instant::now();
            run(candidate, 32768);
            started.elapsed().as_nanos()
        };
        let (old, new) = if sample % 2 == 0 {
            (timed(false), timed(true))
        } else {
            let new = timed(true);
            (timed(false), new)
        };
        println!(
            "HC074-PROJECTION-PREFIX-TIME {}",
            serde_json::json!({"sample":sample,"iterations":32768,"reference_wall_ns":old,"candidate_wall_ns":new,"candidate_first":sample%2==1,"allocation_counter_active":false,"product":false})
        );
    }
    assert!(
        improved,
        "fixed prefixes must remove repeated allocation requests"
    );
}
