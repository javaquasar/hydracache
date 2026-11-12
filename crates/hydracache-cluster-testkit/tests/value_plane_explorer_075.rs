use hydracache_cluster_testkit::value_plane_explorer_075::{
    composite_invariant_violations, shrink_failing_schedule, CompositeBounds, CompositeExplorer,
    CompositeSnapshot, CompositeTransferPhase,
};

#[test]
fn composite_explorer_covers_transfer_expiry_failover_and_lifecycle() {
    let report = CompositeExplorer::new(CompositeBounds {
        max_depth: 9,
        max_states: 50_000,
    })
    .unwrap()
    .explore();
    assert!(report.passed(), "{report:#?}");
    assert!(report.explored_states > 5_000);
    assert!(report.explored_transitions >= report.explored_states);
    assert_eq!(report.max_depth_reached, 9);
    assert_eq!(report.action_coverage.len(), 19);
}

#[test]
fn invariant_canaries_detect_early_target_and_stale_generation() {
    let early_target = CompositeSnapshot {
        transfer: CompositeTransferPhase::Ready,
        target_serving: true,
        ..CompositeSnapshot::default()
    };
    assert!(composite_invariant_violations(&early_target).contains(&"target_serves_before_cutover"));

    let stale_generation = CompositeSnapshot {
        live_version: 3,
        acknowledged_version: 3,
        namespace_generation: 2,
        record_generation: Some(1),
        ..CompositeSnapshot::default()
    };
    assert!(composite_invariant_violations(&stale_generation)
        .contains(&"namespace_generation_crossing"));
}

#[test]
fn failing_schedules_are_minimized_deterministically() {
    let original = [1_u8, 2, 7, 3, 7, 4];
    let minimized = shrink_failing_schedule(&original, |candidate| candidate.contains(&7));
    assert_eq!(minimized, vec![7]);
    assert_eq!(
        shrink_failing_schedule(&original, |candidate| candidate.contains(&7)),
        minimized
    );
}
