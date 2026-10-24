//! Three-arm binding requests, including prefix refusals. Not latency or admission.
use super::*;
use crate::diagnostic_worker_policy::authority_registry::context_controls::{
    Binding, BindingFixture,
};
#[test]
#[ignore = "manual nonroot context ownership and prefix request attribution; no timing or admission"]
fn authority_context_allocation_attribution() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    for case in [
        "healthy",
        "issuer-mismatch",
        "epoch-mismatch",
        "context-mismatch",
    ] {
        let fixture = BindingFixture::new(case);
        let run = |mode, iterations| {
            for _ in 0..iterations {
                let result = std::hint::black_box(fixture.check(std::hint::black_box(mode)));
                assert_eq!(result.is_ok(), case == "healthy");
            }
        };
        let modes = [Binding::Reference, Binding::Proposed, Binding::Actual];
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
            assert_eq!(counts[1].allocations, 1024);
            assert_eq!(counts[1].reallocations, 0);
            assert_eq!(counts[1].requested_bytes, 64 * 1024);
            if matches!(case, "issuer-mismatch" | "epoch-mismatch") {
                assert_eq!(counts[0], counts[1]);
            } else {
                assert!(counts[0].allocations > counts[1].allocations);
            }
            assert!(counts[2] == counts[0] || counts[2] == counts[1]);
            println!(
                "HC074-AUTHORITY-CONTEXT {}",
                serde_json::json!({
                    "case":case,"sample":sample,"iterations":1024,"warmups":32,"order":order,
                    "reference":counts[0],"proposed":counts[1],"actual":counts[2],
                    "rust_requests_only":true,"timing_claim":false,"server":false,"product":false,
                })
            );
        }
    }
}
