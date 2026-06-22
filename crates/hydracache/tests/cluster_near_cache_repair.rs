use hydracache::{ClusterGeneration, HydraCache, MetaDataContainer, NearCacheRepairAction};

#[test]
fn sequence_gap_triggers_conservative_invalidate() {
    let mut metadata = MetaDataContainer::default();

    assert_eq!(
        metadata.on_watermark(Some(ClusterGeneration::new(1)), Some(1)),
        NearCacheRepairAction::ClearPartition
    );
    assert_eq!(
        metadata.on_watermark(Some(ClusterGeneration::new(1)), Some(3)),
        NearCacheRepairAction::InvalidateConservatively
    );
    assert_eq!(metadata.last_seq(), 3);
}

#[test]
fn generation_change_clears_partition() {
    let mut metadata = MetaDataContainer::default();
    metadata.on_watermark(Some(ClusterGeneration::new(1)), Some(7));

    let action = metadata.on_watermark(Some(ClusterGeneration::new(2)), Some(1));

    assert_eq!(action, NearCacheRepairAction::ClearPartition);
    assert_eq!(metadata.last_uuid(), Some(ClusterGeneration::new(2)));
    assert_eq!(metadata.last_seq(), 1);
}

#[test]
fn duplicate_or_reordered_frame_does_not_break_watermark() {
    let mut metadata = MetaDataContainer::default();
    metadata.on_watermark(Some(ClusterGeneration::new(1)), Some(1));
    metadata.on_watermark(Some(ClusterGeneration::new(1)), Some(2));

    assert_eq!(
        metadata.on_watermark(Some(ClusterGeneration::new(1)), Some(2)),
        NearCacheRepairAction::Apply
    );
    assert_eq!(
        metadata.on_watermark(Some(ClusterGeneration::new(1)), Some(1)),
        NearCacheRepairAction::Apply
    );
    assert_eq!(metadata.last_seq(), 2);
}

#[test]
fn reorder_plus_restart_resolves_to_clear() {
    let mut metadata = MetaDataContainer::default();
    metadata.on_watermark(Some(ClusterGeneration::new(1)), Some(5));

    assert_eq!(
        metadata.on_watermark(Some(ClusterGeneration::new(2)), Some(1)),
        NearCacheRepairAction::ClearPartition
    );
}

#[test]
fn conservative_invalidate_counter_is_reported() {
    let cache = HydraCache::local().build();

    cache.record_cluster_near_cache_conservative_invalidation();

    let report = cache.cluster_pilot_report();
    assert_eq!(report.near_cache_conservative_invalidations, 1);
}
