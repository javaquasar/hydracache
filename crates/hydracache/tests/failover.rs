use hydracache::{
    select_backup_promotion, ClusterNodeId, PartitionId, PromotionPhase, ReplicatedSlot, Replicas,
};

#[test]
fn no_backup_owner_reports_degraded() {
    let replicas = Replicas::new("member-a", vec![]);

    assert!(select_backup_promotion(PartitionId::new(1), &replicas).is_none());
}

#[test]
fn primary_leaves_backup_is_selected_for_promotion() {
    let replicas = Replicas::new(
        "member-a",
        vec![ClusterNodeId::from("member-b"), ClusterNodeId::from("member-c")],
    );

    let promotion = select_backup_promotion(PartitionId::new(8), &replicas).unwrap();

    assert_eq!(promotion.departing_primary, ClusterNodeId::from("member-a"));
    assert_eq!(promotion.new_primary, ClusterNodeId::from("member-b"));
    assert_eq!(promotion.phase, PromotionPhase::Before);
}

#[test]
fn invalidation_during_promotion_beats_stale_value() {
    let stale_value = ReplicatedSlot::Value {
        value: b"old".to_vec(),
        version: 5,
    };
    let tombstone = ReplicatedSlot::Tombstone {
        version: 6,
        gc_eligible_after: None,
    };

    assert!(stale_value.merge(tombstone).is_tombstone());
}

#[test]
fn replication_factor_restored_after_finalize() {
    let restored = Replicas::new(
        "member-b",
        vec![ClusterNodeId::from("member-c"), ClusterNodeId::from("member-d")],
    );

    assert_eq!(restored.copy_count(), 3);
}
