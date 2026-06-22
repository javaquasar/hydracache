use std::collections::BTreeMap;

use hydracache::{
    validate_move_preserves_zone_quorum, ClusterEpoch, ClusterNodeId, InMemoryReplicatedValueStore,
    MovePhase, NodeTopology, PartitionId, PartitionMove, Replicas, ReplicatedValueRecord,
    ReplicatedValueStore, ReshardPlan, WriteWatermark, ZoneAwareReplicaSet,
};

#[test]
fn online_reshard_write_during_move_is_shadowed_to_both_owners() {
    let movement = PartitionMove::new(PartitionId::new(7), "old-owner", "new-owner", 100);

    let targets = movement.write_targets();

    assert_eq!(
        targets,
        vec![
            ClusterNodeId::from("old-owner"),
            ClusterNodeId::from("new-owner")
        ]
    );
    assert_eq!(movement.read_owner(), ClusterNodeId::from("old-owner"));
}

#[test]
fn online_reshard_read_your_writes_holds_across_a_move() {
    let partition = PartitionId::new(3);
    let mut movement = PartitionMove::new(partition, "old-owner", "new-owner", 10);
    let watermark = WriteWatermark::new(partition, 5, ClusterEpoch::new(9));

    assert!(movement
        .write_targets()
        .contains(&ClusterNodeId::from("new-owner")));
    movement.record_backfill(10);
    movement.advance();
    assert_eq!(movement.phase, MovePhase::Backfill);
    movement.advance();
    assert_eq!(movement.phase, MovePhase::Commit);

    assert_eq!(movement.read_owner(), ClusterNodeId::from("new-owner"));
    assert_eq!(watermark.partition, partition);
    assert_eq!(watermark.version, 5);
}

#[test]
fn online_reshard_coordinator_crash_resumes_move_from_progress() {
    let partition = PartitionId::new(11);
    let mut plan = ReshardPlan::new(
        ClusterEpoch::new(2),
        vec![PartitionMove::new(partition, "a", "b", 1_000)],
        1,
    );
    plan.record_backfill(partition, 400);

    let resumed = ReshardPlan::resume_from(plan.snapshot());

    assert_eq!(resumed.moves[0].backfilled_bytes, 400);
    assert_eq!(resumed.moves[0].progress_ratio(), 0.4);
}

#[test]
fn online_reshard_tombstone_not_resurrected_during_backfill() {
    let partition = PartitionId::new(4);
    let mut source = InMemoryReplicatedValueStore::default();
    let mut target = InMemoryReplicatedValueStore::default();
    source
        .upsert(
            "user:1",
            ReplicatedValueRecord::value(partition, 10, ClusterEpoch::new(1), b"old".to_vec()),
        )
        .unwrap();
    source
        .tombstone("user:1", partition, 11, ClusterEpoch::new(1))
        .unwrap();

    let backfilled = source.get("user:1").unwrap().expect("record");
    target.upsert("user:1", backfilled).unwrap();

    assert!(target
        .get("user:1")
        .unwrap()
        .expect("target")
        .is_tombstone());
}

#[test]
fn online_reshard_move_respecting_zone_spread_is_rejected_if_it_colocates_quorum() {
    let mut topology = BTreeMap::new();
    topology.insert(ClusterNodeId::from("a"), NodeTopology::new("eu", "az-a"));
    topology.insert(ClusterNodeId::from("b"), NodeTopology::new("eu", "az-a"));
    topology.insert(ClusterNodeId::from("c"), NodeTopology::new("eu", "az-a"));
    let candidate = ZoneAwareReplicaSet {
        replicas: Replicas::new(
            "a",
            vec![ClusterNodeId::from("b"), ClusterNodeId::from("c")],
        ),
        topology,
        placement_zone_underspread: true,
    };

    let error = validate_move_preserves_zone_quorum(&candidate, 2).unwrap_err();

    assert!(error.to_string().contains("zone-spread write quorum"));
}

#[test]
fn online_reshard_drain_node_moves_all_partitions_then_leaves_cleanly() {
    let plan = ReshardPlan::drain_node(
        ClusterEpoch::new(3),
        "node-a",
        [
            (PartitionId::new(1), ClusterNodeId::from("node-b"), 10),
            (PartitionId::new(2), ClusterNodeId::from("node-c"), 20),
        ],
        1,
    );

    assert_eq!(plan.moves.len(), 2);
    assert_eq!(plan.active_moves().len(), 1);
    assert!(plan
        .moves
        .iter()
        .all(|movement| movement.from == ClusterNodeId::from("node-a")));
}
