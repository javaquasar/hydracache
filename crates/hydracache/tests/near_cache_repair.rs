use hydracache::{ClusterGeneration, MetaDataContainer, NearCacheRepairAction, RepairingTask};

#[test]
fn sequence_gap_triggers_conservative_invalidation() {
    let mut metadata = MetaDataContainer::default();
    assert_eq!(
        metadata.on_watermark(Some(ClusterGeneration::new(1)), Some(1)),
        NearCacheRepairAction::ClearPartition
    );

    assert_eq!(
        metadata.on_watermark(Some(ClusterGeneration::new(1)), Some(3)),
        NearCacheRepairAction::InvalidateConservatively
    );
}

#[test]
fn generation_change_clears_partition() {
    let mut metadata = MetaDataContainer::default();
    metadata.on_watermark(Some(ClusterGeneration::new(1)), Some(1));

    assert_eq!(
        metadata.on_watermark(Some(ClusterGeneration::new(2)), Some(1)),
        NearCacheRepairAction::ClearPartition
    );
}

#[test]
fn duplicate_or_reordered_frame_does_not_break_watermark() {
    let mut metadata = MetaDataContainer::default();
    metadata.on_watermark(Some(ClusterGeneration::new(1)), Some(10));

    assert_eq!(
        metadata.on_watermark(Some(ClusterGeneration::new(1)), Some(9)),
        NearCacheRepairAction::Apply
    );
    assert_eq!(metadata.last_seq(), 10);
}

#[test]
fn lost_frame_is_recovered_by_periodic_task() {
    let task = RepairingTask::new(std::time::Duration::from_millis(10));
    let mut metadata = MetaDataContainer::default();
    metadata.on_watermark(Some(ClusterGeneration::new(1)), Some(1));

    let action = metadata.on_watermark(Some(ClusterGeneration::new(1)), Some(4));

    assert_eq!(task.interval, std::time::Duration::from_millis(10));
    assert_eq!(action, NearCacheRepairAction::InvalidateConservatively);
}

#[test]
fn reorder_and_restart_simultaneously_resolves_to_clear() {
    let mut metadata = MetaDataContainer::default();
    metadata.on_watermark(Some(ClusterGeneration::new(1)), Some(100));

    assert_eq!(
        metadata.on_watermark(Some(ClusterGeneration::new(2)), Some(1)),
        NearCacheRepairAction::ClearPartition
    );
    assert_eq!(metadata.last_seq(), 1);
}
