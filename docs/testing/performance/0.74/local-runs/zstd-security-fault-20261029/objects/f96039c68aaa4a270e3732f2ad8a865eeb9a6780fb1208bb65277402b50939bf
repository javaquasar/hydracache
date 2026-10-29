//! Pure fd-path ownership requests; actual runtime is measured by full inspect.
use super::*;
use crate::diagnostic_worker_policy::authority_registry::fixture::fd_path_controls::{
    borrowed, proposed, reference,
};
#[test]
#[ignore = "manual local fd-path requests; not timing or admission"]
fn authority_fd_path_allocation_attribution() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    for fd in [0, 9, 10, 99, 100, 12, i32::MIN, i32::MAX] {
        let expected = reference(fd);
        let run = |arm, iterations| {
            for _ in 0..iterations {
                let fd = std::hint::black_box(fd);
                if arm == 0 {
                    let path = reference(fd);
                    assert_eq!(std::hint::black_box(path.as_str()), expected);
                } else {
                    let path = proposed(fd);
                    assert_eq!(std::hint::black_box(borrowed(&path)), expected);
                }
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
            assert_eq!(counts[1], Counts::default());
            println!(
                "HC074-AUTHORITY-FD-PATH {}",
                serde_json::json!({
                    "case":fd.to_string(),"fd":fd,"sample":sample,"order":order,
                    "iterations":1024,"warmups":32,"reference":counts[0],"proposed":counts[1],
                    "rust_requests_only":true,"timing_claim":false,"server":false,"product":false,
                })
            );
        }
    }
}
