//! Pure expected-name comparison requests; actual runtime measured separately.
use super::*;
use crate::diagnostic_worker_policy::authority_registry::fixture::expected_name_controls::{
    proposed, reference,
};
use crate::diagnostic_worker_policy::authority_registry::fixture::inventory_controls::observed;
#[test]
#[ignore = "manual local expected inventory requests; not timing or admission"]
fn authority_expected_name_allocation_attribution() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    for (case, generations) in [
        ("empty", 0),
        ("healthy-1", 1),
        ("healthy-7", 7),
        ("healthy-128", 128),
        ("invalid-7", 7),
    ] {
        let mut seen = observed(generations);
        if case == "invalid-7" {
            seen.remove("0000000000000001.commit");
            seen.insert("0000000000000000.commit".into());
        }
        let run = |arm, iterations| {
            for _ in 0..iterations {
                let input = std::hint::black_box(&seen);
                let count = std::hint::black_box(generations);
                let matched = if arm == 0 {
                    reference(input, count)
                } else {
                    proposed(input, count)
                };
                assert_eq!(std::hint::black_box(matched), case != "invalid-7");
            }
        };
        run(0, 32);
        run(1, 32);
        for sample in 0..3 {
            let order = if sample % 2 == 0 { [0, 1] } else { [1, 0] };
            let mut counts = [Counts::default(); 2];
            for arm in order {
                counts[arm] = capture(|| run(arm, 1024)).1;
            }
            assert!(counts[1].allocations <= counts[0].allocations);
            assert!(counts[1].requested_bytes <= counts[0].requested_bytes);
            println!(
                "HC074-AUTHORITY-EXPECTED-NAME {}",
                serde_json::json!({
                    "case":case,"generations":generations,"sample":sample,"order":order,
                    "iterations":1024,"warmups":32,"reference":counts[0],"proposed":counts[1],
                    "rust_requests_only":true,"timing_claim":false,"server":false,"product":false,
                })
            );
        }
    }
}
