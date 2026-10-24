//! Name-format request attribution, not allocation latency or admission.
use super::*;
use crate::diagnostic_worker_policy::authority_registry::fixture::name_capacity_controls::{
    render, Reading,
};
#[test]
#[ignore = "manual local three-arm name capacity requests; not timing or admission"]
fn authority_name_capacity_allocation_attribution() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let long = "x".repeat(256);
    for (case, revision, suffix) in [
        ("json-1", 1, "json"),
        ("commit-1", 1, "commit"),
        ("json-7", 7, "json"),
        ("commit-7", 7, "commit"),
        ("json-128", 128, "json"),
        ("commit-128", 128, "commit"),
        ("long-128", 128, long.as_str()),
    ] {
        let expected = render(Reading::Reference, revision, suffix);
        let run = |mode, iterations| {
            for _ in 0..iterations {
                let output = render(
                    std::hint::black_box(mode),
                    std::hint::black_box(revision),
                    std::hint::black_box(suffix),
                );
                assert_eq!(std::hint::black_box(output).as_bytes(), expected.as_bytes());
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
            for arm in order {
                counts[arm] = capture(|| run(modes[arm], 1024)).1;
            }
            assert_eq!(counts[0].allocations, counts[1].allocations);
            assert!(counts[1].reallocations <= counts[0].reallocations);
            assert!(counts[1].requested_bytes < counts[0].requested_bytes);
            assert!(counts[2] == counts[0] || counts[2] == counts[1]);
            println!(
                "HC074-AUTHORITY-NAME-CAPACITY {}",
                serde_json::json!({
                    "case":case,"revision":revision,"suffix_bytes":suffix.len(),"sample":sample,
                    "order":order,"iterations":1024,"warmups":32,"reference":counts[0],
                    "proposed":counts[1],"actual":counts[2],"rust_requests_only":true,
                    "timing_claim":false,"server":false,"product":false,
                })
            );
        }
    }
}
