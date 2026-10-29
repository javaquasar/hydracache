//! Same retained validators and unchanged worker batch, with numeric nested edges.
use super::*;
use crate::diagnostic_ipc::validation_trace::kernel_walk::{self as kernel, COUNT};

const EDGES: [(usize, usize, u64); 13] = [
    (0, 1, 128),
    (0, 8, 64),
    (0, 7, 64),
    (1, 4, 256),
    (1, 2, 256),
    (1, 3, 256),
    (1, 9, 256),
    (4, 5, 1024),
    (4, 6, 2048),
    (2, 9, 256),
    (5, 9, 1024),
    (7, 4, 128),
    (8, 4, 128),
];
#[test]
#[ignore = "manual nonroot original kernel walk attribution; no reservation or execution"]
fn original_kernel_context_walk_attribution() {
    for sample in 0..3 {
        with_original_at(false, None, |original, paths, _, _, _| {
            original.revalidate().unwrap();
            let request = original.original.original.request.clone();
            let worker = fs::read(&paths.worker).unwrap();
            let oidc = fs::read(&paths.oidc).unwrap();
            let ((walks, report), ipc) = trace::capture(|| {
                let (report, walks) = live_walk::capture(|| {
                    let (_, report) = kernel::capture(|| {
                        for _ in 0..32 {
                            original.original.original.worker.revalidate().unwrap();
                        }
                    });
                    report
                });
                (walks, report)
            });
            assert_eq!(walks.map(|s| s.calls), [32, 64, 128, 128, 64, 32]);
            assert_eq!(
                report.nodes.map(|s| s.calls),
                [64, 128, 256, 256, 512, 1024, 2048, 64, 64, 1536]
            );
            assert_eq!(report.roots, [64, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            for parent in 0..COUNT {
                for child in 0..COUNT {
                    let expected = EDGES
                        .iter()
                        .find(|&&(p, c, _)| p == parent && c == child)
                        .map_or(0, |&(_, _, count)| count);
                    assert_eq!(report.edges[parent][child].calls, expected);
                }
                assert_eq!(
                    report.nodes[parent].calls,
                    report.roots[parent]
                        + report.edges.iter().map(|r| r[parent].calls).sum::<u64>()
                );
                assert_eq!(
                    report.nodes[parent].inclusive_ns - report.nodes[parent].outside_children_ns,
                    report.edges[parent]
                        .iter()
                        .map(|e| e.inclusive_ns)
                        .sum::<u128>()
                );
            }
            for stage in ipc.stages {
                assert_eq!(stage.authentication_calls, 0);
                assert_eq!(stage.signature_calls, 0);
            }
            assert!(ipc.first_time.is_none() && ipc.time_refusals == 0);
            original.revalidate().unwrap();
            assert_eq!(original.original.original.request, request);
            assert_eq!(fs::read(&paths.worker).unwrap(), worker);
            assert_eq!(fs::read(&paths.oidc).unwrap(), oidc);
            assert!(!original.is_refused());
            for (index, stats) in walks.into_iter().enumerate() {
                println!("HC074-KERNEL-WORKER sample={sample} index={index} stats={stats:?}");
            }
            for (index, stats) in report.nodes.into_iter().enumerate() {
                println!("HC074-KERNEL-NODE sample={sample} index={index} stats={stats:?}");
            }
            for (parent, child, _) in EDGES {
                let edge = report.edges[parent][child];
                println!("HC074-KERNEL-EDGE sample={sample} parent={parent} child={child} calls={} inclusive_ns={}", edge.calls, edge.inclusive_ns);
            }
            println!("HC074-KERNEL-CONTROL sample={sample} iterations=32 roots=64 original=true unchanged=true reservation=false schedule_changed=false admission=false");
        });
    }
}
