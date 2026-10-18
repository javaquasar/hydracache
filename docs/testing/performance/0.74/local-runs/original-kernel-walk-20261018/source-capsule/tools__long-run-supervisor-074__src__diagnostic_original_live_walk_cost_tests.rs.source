//! Actual retained validator before reservation, not a substitute for ReservedLive.
use super::*;
use crate::diagnostic_ipc::validation_trace::live_walk;

#[path = "diagnostic_original_kernel_walk_cost_tests.rs"]
mod kernel_walk_tests;

#[test]
#[ignore = "manual nonroot original live worker walk attribution; no reservation or execution"]
fn original_live_worker_walk_attribution() {
    let names = [
        "kernel-worker",
        "worker-policy",
        "worker-source",
        "active-authority",
        "kernel-context",
        "local-context",
    ];
    for sample in 0..3 {
        with_original_at(false, None, |original, paths, _, _, _| {
            original.revalidate().unwrap();
            let request = original.original.original.request.clone();
            let worker = fs::read(&paths.worker).unwrap();
            let oidc = fs::read(&paths.oidc).unwrap();
            let (stats, ipc) = trace::capture(|| {
                let (_, stats) = live_walk::capture(|| {
                    for _ in 0..32 {
                        original.original.original.worker.revalidate().unwrap();
                    }
                });
                stats
            });
            assert_eq!(stats.map(|s| s.calls), [32, 64, 128, 128, 64, 32]);
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
            for (kind, stats) in names.into_iter().zip(stats) {
                assert!(stats.outside_children_ns <= stats.inclusive_ns);
                println!("HC074-LIVE-WALK sample={sample} kind={kind} stats={stats:?}");
            }
            println!("HC074-LIVE-WALK-CONTROL sample={sample} iterations=32 original=true unchanged=true reservation=false schedule_changed=false admission=false");
        });
    }
}
