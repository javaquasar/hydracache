//! Three-arm isolated formatting requests. No timing or retained-memory claim.
use super::*;
use crate::diagnostic_worker_policy::authority_registry::hex_controls::{
    encode, fixture_trust, Encoder,
};
#[test]
#[ignore = "manual local nonroot authority key formatting requests; not admission or timing"]
fn authority_hex_allocation_attribution() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let trust = fixture_trust();
    let run = |mode, iterations| {
        for _ in 0..iterations {
            let value = std::hint::black_box(encode(&trust, std::hint::black_box(mode)));
            assert_eq!(value.len(), 64);
        }
    };
    let modes = [Encoder::Reference, Encoder::Proposed, Encoder::Actual];
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
        assert!(counts[0].allocations > counts[1].allocations);
        assert!(counts[2] == counts[0] || counts[2] == counts[1]);
        println!(
            "HC074-AUTHORITY-HEX {}",
            serde_json::json!({
                "sample":sample,"iterations":1024,"warmups":32,"order":order,
                "reference":counts[0],"proposed":counts[1],"actual":counts[2],
                "rust_requests_only":true,"timing_claim":false,"server":false,"product":false,
            })
        );
    }
}
