//! Attribution through the existing callback, never a different validation schedule.
use super::*;
use crate::diagnostic_ipc::validation_trace::{self as trace, Stage};

#[test]
#[ignore = "manual nonroot original leaf attribution; no Starting, outputs, bundle, alarm or execution"]
fn original_validation_leaf_cost_attribution() {
    let channels = [
        Stage::Unassigned,
        Stage::Setup,
        Stage::Fence,
        Stage::Retain,
        Stage::Observe,
        Stage::Publish,
        Stage::Bind,
        Stage::Checks,
    ];
    for sample in 0..3 {
        with_original_at(false, None, |original, _, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let mut frame = accepted(original, fence, leaf, receiver, sender, model);
                let names = [NAME, RESERVED_NAME];
                let before: Vec<_> = names
                    .iter()
                    .map(|n| fs::read(leaf.join(n)).unwrap())
                    .collect();
                let mut visited = Vec::with_capacity(8);
                let (result, report) = trace::capture(|| {
                    frame.revalidate_with_for_test(|step| {
                        let index = visited.len();
                        // Ordinals of the frozen eight-step enum, without
                        // widening its private module visibility for a probe.
                        assert_eq!(step as usize, index);
                        trace::stage(channels[index]);
                        visited.push(step);
                    })
                });
                println!("HC074-LEAF-RESULT sample={sample} result={result:?} report={report:?}");
                result.unwrap();
                assert_eq!(visited.len(), 8);
                assert_eq!(
                    report
                        .stages
                        .iter()
                        .map(|s| s.authentication_calls)
                        .sum::<u64>(),
                    34
                );
                assert_eq!(
                    report.stages.iter().map(|s| s.signature_calls).sum::<u64>(),
                    34
                );
                assert!(report.first_time.is_none() && report.time_refusals == 0);
                for index in 0..8 {
                    println!(
                        "HC074-LEAF-COST sample={sample} bucket={index} opening={:?} includes_whole_closing_round={} stats={:?}",
                        visited[index], index == 7, report.stages[channels[index] as usize]
                    );
                }
                let after: Vec<_> = names
                    .iter()
                    .map(|n| fs::read(leaf.join(n)).unwrap())
                    .collect();
                assert_eq!(after, before);
                assert_eq!(fs::read_dir(leaf).unwrap().count(), 2);
                assert!(!frame.is_refused());
                busy(root);
                drop(frame);
                retained_context_refuses_for_test(root);
                println!("HC074-LEAF-CONTROL sample={sample} opening=8 closing=8 authentications=34 held=true unchanged=true starting=false outputs=false alarm=false schedule_changed=false");
            });
        });
    }
}
