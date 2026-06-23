use hydracache::{
    promote_region_home, rejoining_region_authority, ClusterEpoch, ClusterNodeId,
    ControlPlaneSnapshot, InMemoryReplicatedValueStore, InMemorySnapshotSink, NodeTopology,
    PartitionId, PromotionFreezeWindow, PromotionPhase, RegionObservation, RegionRestore,
    RegionState, RegionStateDetector, RejoiningRegionDecision, Replicas, ReplicatedValueRecord,
    ReplicatedValueStore, SnapshotSink, CONTROL_PLANE_SNAPSHOT_FORMAT_VERSION,
};

fn base_snapshot() -> ControlPlaneSnapshot {
    let mut snapshot = ControlPlaneSnapshot::new(ClusterEpoch::new(7));
    snapshot
        .topology
        .insert(ClusterNodeId::from("eu-a"), NodeTopology::new("eu", "az-a"));
    snapshot
        .topology
        .insert(ClusterNodeId::from("eu-b"), NodeTopology::new("eu", "az-b"));
    snapshot
        .topology
        .insert(ClusterNodeId::from("us-a"), NodeTopology::new("us", "az-a"));
    snapshot
        .topology
        .insert(ClusterNodeId::from("us-b"), NodeTopology::new("us", "az-b"));
    snapshot.ownership.insert(
        PartitionId::new(1),
        Replicas::new("eu-a", vec![ClusterNodeId::from("us-a")]),
    );
    snapshot.ownership.insert(
        PartitionId::new(2),
        Replicas::new("eu-b", vec![ClusterNodeId::from("us-b")]),
    );
    snapshot
}

fn bounded_freeze() -> PromotionFreezeWindow {
    PromotionFreezeWindow {
        observed_ms: 12,
        bound_ms: 50,
    }
}

#[test]
fn region_failover_down_promotes_surviving_home() {
    let snapshot = base_snapshot();
    let detector = RegionStateDetector::default();
    let mut observation = RegionObservation::healthy("eu");
    observation.operator_declared_down = true;
    observation.quorum_reachable = false;
    let state = detector.classify(&observation);

    let report =
        promote_region_home(&snapshot, "eu".into(), "us".into(), state, bounded_freeze()).unwrap();

    assert_eq!(state, RegionState::Down);
    assert_eq!(
        report.promotion.partitions,
        vec![PartitionId::new(1), PartitionId::new(2)]
    );
    assert_eq!(report.promotion.epoch, ClusterEpoch::new(8));
    assert_eq!(report.promotion.phase, PromotionPhase::Finalize);
    assert!(report.is_fully_replicated());
    for partition in [PartitionId::new(1), PartitionId::new(2)] {
        let replicas = report.snapshot.ownership.get(&partition).unwrap();
        let topology = report.snapshot.topology.get(&replicas.primary).unwrap();
        assert_eq!(topology.region.as_str(), "us");
    }
}

#[test]
fn region_failover_flapping_region_does_not_double_promote() {
    let snapshot = base_snapshot();
    let detector = RegionStateDetector::new(2);
    let observation = RegionObservation {
        region: "eu".into(),
        missed_heartbeats: 10,
        quorum_reachable: false,
        operator_declared_down: false,
        split_brain_risk: false,
    };

    let state = detector.classify(&observation);
    let error = promote_region_home(&snapshot, "eu".into(), "us".into(), state, bounded_freeze())
        .unwrap_err();

    assert_eq!(state, RegionState::Suspect);
    assert!(error.to_string().contains("explicitly declared down"));

    let mut split_brain = observation;
    split_brain.operator_declared_down = true;
    split_brain.split_brain_risk = true;
    assert_eq!(detector.classify(&split_brain), RegionState::Suspect);
}

#[test]
fn region_failover_rejoining_lower_epoch_region_loses_authority() {
    let decision = rejoining_region_authority(ClusterEpoch::new(8), ClusterEpoch::new(7));

    assert_eq!(
        decision,
        RejoiningRegionDecision::FenceLowerEpoch {
            current_epoch: ClusterEpoch::new(8),
            rejoining_epoch: ClusterEpoch::new(7),
        }
    );
    assert_eq!(
        rejoining_region_authority(ClusterEpoch::new(8), ClusterEpoch::new(8)),
        RejoiningRegionDecision::AcceptAuthority
    );
}

#[test]
fn region_failover_restore_rebuilds_from_snapshot() {
    let snapshot = base_snapshot();
    let mut sink = InMemorySnapshotSink::new();
    sink.put(snapshot.clone()).unwrap();
    let snapshot = sink.latest().unwrap().expect("snapshot");
    let mut store = InMemoryReplicatedValueStore::default();
    store
        .upsert(
            "user:1",
            ReplicatedValueRecord::value(
                PartitionId::new(1),
                1,
                ClusterEpoch::new(7),
                b"ada".to_vec(),
            ),
        )
        .unwrap();
    let mut restore = RegionRestore::new(snapshot.clone(), store);

    let backfilled = restore
        .backfill_from(vec![(
            "user:2".to_owned(),
            ReplicatedValueRecord::value(
                PartitionId::new(2),
                2,
                ClusterEpoch::new(7),
                b"grace".to_vec(),
            ),
        )])
        .unwrap();
    let outcome = restore.restore().unwrap();

    assert_eq!(backfilled, 1);
    assert_eq!(outcome.authority.epoch(), ClusterEpoch::new(7));
    assert_eq!(outcome.authority.committed_map(), snapshot.topology);
    assert_eq!(outcome.report.topology_node_count, 4);
    assert_eq!(outcome.report.partition_count, 2);
    assert_eq!(outcome.report.restored_value_count, 2);
    assert!(outcome.values.get("user:2").unwrap().is_some());
}

#[test]
fn region_failover_restore_rejects_future_snapshot_format() {
    let mut snapshot = base_snapshot();
    snapshot.format_version = CONTROL_PLANE_SNAPSHOT_FORMAT_VERSION + 1;
    let restore = RegionRestore::new(snapshot, InMemoryReplicatedValueStore::default());

    let error = restore.restore().unwrap_err();

    assert!(error.to_string().contains("newer than this binary"));
}

#[test]
fn region_failover_promotion_reports_degraded_when_target_missing() {
    let mut snapshot = base_snapshot();
    snapshot
        .topology
        .retain(|_, topology| topology.region.as_str() != "us");

    let report = promote_region_home(
        &snapshot,
        "eu".into(),
        "us".into(),
        RegionState::Down,
        bounded_freeze(),
    )
    .unwrap();

    assert_eq!(
        report.degraded_partitions,
        vec![PartitionId::new(1), PartitionId::new(2)]
    );
    assert!(!report.is_fully_replicated());
}

#[test]
#[ignore = "chaos gate: whole-region loss self-heals to target RF"]
fn region_failover_whole_region_loss_self_heals_to_target_rf() {
    let snapshot = base_snapshot();
    let report = promote_region_home(
        &snapshot,
        "eu".into(),
        "us".into(),
        RegionState::Down,
        bounded_freeze(),
    )
    .unwrap();

    assert!(report.is_fully_replicated());
    assert_eq!(report.promotion.partitions.len(), 2);
}
