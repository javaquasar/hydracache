use hydracache_cluster_testkit::value_plane_dedup_075::{
    DedupAcceptance, DedupError, DedupLifecycle, RetainedOutcome,
};

fn outcome(generation: u64, digest: u64, value: u64, sequence: u64) -> RetainedOutcome {
    RetainedOutcome {
        namespace_generation: generation,
        digest,
        outcome: value,
        sequence,
    }
}

#[test]
fn replay_transfer_and_safe_eviction_preserve_exact_outcomes() {
    let mut source = DedupLifecycle::new(4).unwrap();
    assert_eq!(
        source.record(1, outcome(7, 11, 19, 3)).unwrap(),
        DedupAcceptance::Recorded
    );
    assert_eq!(
        source.record(1, outcome(7, 11, 999, 9)).unwrap(),
        DedupAcceptance::Replayed(19)
    );
    let mut target = DedupLifecycle::new(4).unwrap();
    target.import(source.export()).unwrap();
    assert!(!target.namespace_reclaimable(7));
    target.advance_safe_watermark(3).unwrap();
    assert_eq!(target.evict_through(3).unwrap(), 1);
    assert!(target.namespace_reclaimable(7));
    assert!(target.is_empty());
}

#[test]
fn conflicting_identity_capacity_and_unsafe_eviction_fail_loud() {
    assert!(DedupLifecycle::new(0).is_err());
    let mut model = DedupLifecycle::new(1).unwrap();
    model.record(1, outcome(1, 10, 20, 1)).unwrap();
    assert_eq!(
        model.record(1, outcome(1, 11, 20, 1)),
        Err(DedupError::IdentityConflict)
    );
    assert_eq!(
        model.record(2, outcome(1, 12, 21, 2)),
        Err(DedupError::Capacity)
    );
    assert_eq!(
        model.evict_through(1),
        Err(DedupError::EvictionBeyondSafeWatermark)
    );
    assert_eq!(model.len(), 1);
}
