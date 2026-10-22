//! Rust name request attribution; no timing, retained-memory or admission claim.
use super::*;
use crate::diagnostic_worker_policy::authority_registry::fixture::leaf_name_controls::{
    LeafFixture, Reading,
};
#[test]
#[ignore = "manual nonroot three-arm leaf name requests and refusal controls; not timing or admission"]
fn authority_leaf_name_allocation_attribution() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    for case in ["record", "commit", "metadata-refusal", "missing-name"] {
        let mut fixture = LeafFixture::new(case);
        let mut run = |mode, iterations| {
            for _ in 0..iterations {
                let result = std::hint::black_box(fixture.check(std::hint::black_box(mode)));
                assert_eq!(result.is_ok(), matches!(case, "record" | "commit"));
            }
        };
        let modes = [Reading::Reference, Reading::Proposed, Reading::Actual];
        for mode in modes {
            run(mode, 32);
        }
        for sample in 0..3 {
            let mut order = [0, 1, 2];
            order.rotate_left(sample);
            let mut counts = [Counts::default(); 3];
            for index in order {
                counts[index] = capture(|| run(modes[index], 1024)).1;
            }
            let name_bytes = if case == "commit" { 23 } else { 21 };
            if case == "metadata-refusal" {
                assert_eq!(counts[0], Counts::default());
                assert_eq!(counts[1], Counts::default());
            } else {
                assert_eq!(counts[0].allocations - counts[1].allocations, 1024);
                assert_eq!(
                    counts[0].requested_bytes - counts[1].requested_bytes,
                    name_bytes * 1024
                );
            }
            assert_eq!(counts[0].reallocations, counts[1].reallocations);
            assert!(counts[2] == counts[0] || counts[2] == counts[1]);
            println!(
                "HC074-AUTHORITY-LEAF-NAME {}",
                serde_json::json!({
                    "case":case,"sample":sample,"iterations":1024,"warmups":32,"order":order,
                    "reference":counts[0],"proposed":counts[1],"actual":counts[2],
                    "rust_requests_only":true,"timing_claim":false,"server":false,"product":false,
                })
            );
        }
    }
}
