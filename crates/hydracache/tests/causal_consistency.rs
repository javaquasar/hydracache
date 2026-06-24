use hydracache::{
    apply_causal_write, causal_apply, AppliedSet, ApplyDecision, CausalDependencyMissing,
    CausalSummary, CausalWrite, ClusterEpoch, HybridLogicalClock, PartitionId, PartitionKey,
    SessionWatermark, VersionStamp,
};

fn stamp(version: u64) -> VersionStamp {
    VersionStamp::new(
        version,
        ClusterEpoch::new(1),
        HybridLogicalClock::new(version, 0),
    )
}

fn key(partition: u32, region: &str) -> PartitionKey {
    PartitionKey::new(PartitionId::new(partition), region)
}

#[test]
fn causal_consistency_effect_not_visible_before_cause_across_regions() {
    let cause_key = key(1, "region-a");
    let effect_key = key(2, "region-b");
    let mut deps = CausalSummary::new(8);
    deps.observe(cause_key.clone(), stamp(5));
    let effect = CausalWrite::new(effect_key, stamp(6), deps);

    let region_b = AppliedSet::new();

    assert_eq!(
        causal_apply(&region_b, &effect.deps),
        ApplyDecision::Defer {
            missing: vec![CausalDependencyMissing::Exact {
                key: cause_key,
                required: stamp(5),
            }],
        }
    );
}

#[test]
fn causal_consistency_dependent_write_defers_until_dependencies_applied() {
    let cause_key = key(1, "region-a");
    let effect_key = key(2, "region-b");
    let mut deps = CausalSummary::new(8);
    deps.observe(cause_key.clone(), stamp(5));
    let effect = CausalWrite::new(effect_key.clone(), stamp(6), deps);
    let mut applied = AppliedSet::new();

    let deferred = apply_causal_write(&mut applied, effect.clone())
        .expect_err("effect cannot be visible before its cause");
    assert_eq!(deferred.missing.len(), 1);
    assert!(!applied.covers(&effect_key, stamp(6)));

    applied.mark_applied(cause_key, stamp(5));
    apply_causal_write(&mut applied, effect).expect("dependency repair unlocks the write");

    assert!(applied.covers(&effect_key, stamp(6)));
}

#[test]
fn causal_consistency_summary_overflow_degrades_conservatively_not_dropped() {
    let retained_key = key(2, "region-b");
    let mut summary = CausalSummary::new(1);
    summary.observe(key(1, "region-a"), stamp(5));
    summary.observe(retained_key.clone(), stamp(6));

    assert_eq!(summary.len(), 1);
    assert_eq!(summary.coarsened_total(), 1);
    assert_eq!(summary.coarse_floor(), Some(stamp(5)));
    assert!(summary.dependencies().contains_key(&retained_key));

    let mut applied = AppliedSet::new();
    applied.mark_applied(retained_key, stamp(6));
    assert_eq!(
        causal_apply(&applied, &summary),
        ApplyDecision::Defer {
            missing: vec![CausalDependencyMissing::CoarseFloor { required: stamp(5) }],
        }
    );

    applied.mark_stable_floor(stamp(5));
    assert_eq!(causal_apply(&applied, &summary), ApplyDecision::Apply);
}

#[test]
fn causal_consistency_metadata_is_gced_after_stability() {
    let mut summary = CausalSummary::new(1);
    summary.observe(key(1, "region-a"), stamp(5));
    summary.observe(key(2, "region-b"), stamp(6));
    assert!(!summary.is_empty());

    let mut applied = AppliedSet::new();
    applied.mark_stable_floor(stamp(6));

    assert_eq!(summary.gc_stable(&applied), 2);
    assert!(summary.is_empty());
}

#[test]
fn causal_consistency_summary_from_watermark_preserves_read_dependencies() {
    let mut watermark = SessionWatermark::new(8);
    let cause_key = key(1, "region-a");
    watermark.observe(cause_key.clone(), stamp(5));

    let summary = CausalSummary::from_watermark(&watermark);

    assert_eq!(summary.dependencies().get(&cause_key), Some(&stamp(5)));
    assert_eq!(summary.dependency_bytes(), 56);
}

#[test]
#[ignore = "fault-injection scenario for partition and message reordering"]
fn causal_consistency_holds_under_partition_and_reorder() {}
