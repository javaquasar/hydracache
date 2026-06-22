use std::collections::BTreeMap;

use hydracache::{
    restore_topology_from_snapshot, AutoRepairPolicy, ClusterEpoch, ClusterNodeId, CompatVersion,
    ControlPlaneSnapshot, InMemorySnapshotSink, NodeTopology, RepairAction, RepairMode,
    SnapshotSink, UpgradeGuard, UpgradeStep, CACHE_INVALIDATION_FRAME_VERSION,
    CONTROL_PLANE_SNAPSHOT_FORMAT_VERSION, REPLICATED_VALUE_RECORD_FORMAT_VERSION,
};

#[test]
fn self_heal_debt_over_threshold_triggers_bounded_repair() {
    let policy = AutoRepairPolicy::new(RepairMode::Active, 10, 100, 1);

    let decision = policy.evaluate(11, 0);

    assert_eq!(decision.scheduled, vec![RepairAction::AntiEntropy]);
    assert_eq!(decision.scheduled.len(), 1);
    assert_eq!(decision.capped_at, 1);
}

#[test]
fn self_heal_advisory_mode_suggests_but_does_not_act() {
    let policy = AutoRepairPolicy::new(RepairMode::Advisory, 10, 100, 1);

    let decision = policy.evaluate(0, 500);

    assert_eq!(decision.recommended, vec![RepairAction::AntiEntropy]);
    assert!(decision.scheduled.is_empty());
}

#[test]
fn self_heal_repair_never_starves_hot_path() {
    let policy = AutoRepairPolicy::new(RepairMode::Active, 0, 0, 1);
    let decision = policy.evaluate(10, 10);
    let hot_path_budget_ms = 20;
    let simulated_repair_cost_ms = decision.scheduled.len() as u64 * 5;

    assert!(decision.scheduled.len() <= decision.capped_at);
    assert!(simulated_repair_cost_ms < hot_path_budget_ms);
}

#[test]
fn self_heal_control_plane_restore_rebuilds_topology() {
    let mut snapshot = ControlPlaneSnapshot::new(ClusterEpoch::new(7));
    snapshot.topology.insert(
        ClusterNodeId::from("node-a"),
        NodeTopology::new("eu", "az-a"),
    );
    snapshot.topology.insert(
        ClusterNodeId::from("node-b"),
        NodeTopology::new("eu", "az-b"),
    );
    snapshot.tombstone_versions = BTreeMap::from([("user:1".to_owned(), 9)]);

    let mut sink = InMemorySnapshotSink::new();
    sink.put(snapshot.clone()).unwrap();
    let latest = sink.latest().unwrap().expect("snapshot");
    let restored = restore_topology_from_snapshot(&latest).unwrap();

    assert_eq!(restored.epoch(), ClusterEpoch::new(7));
    assert_eq!(restored.committed_map(), snapshot.topology);
}

#[test]
fn self_heal_control_plane_snapshot_current_format_round_trips() {
    assert_eq!(CONTROL_PLANE_SNAPSHOT_FORMAT_VERSION, 1);

    let mut snapshot = ControlPlaneSnapshot::new(ClusterEpoch::new(8));
    snapshot.topology.insert(
        ClusterNodeId::from("node-current"),
        NodeTopology::new("eu", "az-a"),
    );
    let mut sink = InMemorySnapshotSink::new();

    sink.put(snapshot.clone()).unwrap();

    assert_eq!(sink.latest().unwrap(), Some(snapshot));
}

#[test]
fn self_heal_control_plane_snapshot_future_format_fails_loud() {
    let mut snapshot = ControlPlaneSnapshot::new(ClusterEpoch::new(8));
    snapshot.format_version = CONTROL_PLANE_SNAPSHOT_FORMAT_VERSION + 1;
    snapshot.topology.insert(
        ClusterNodeId::from("node-future"),
        NodeTopology::new("eu", "az-a"),
    );
    let mut sink = InMemorySnapshotSink::new();

    let put_error = sink.put(snapshot.clone()).unwrap_err();
    let restore_error = restore_topology_from_snapshot(&snapshot).unwrap_err();

    assert!(put_error.to_string().contains("newer than this binary"));
    assert!(restore_error.to_string().contains("newer than this binary"));
}

#[test]
fn self_heal_upgrade_guard_accepts_042_to_043_registered_formats() {
    let guard = UpgradeGuard::current();
    let step = UpgradeStep {
        from: CompatVersion::new(0, 42, 0),
        to: CompatVersion::new(0, 43, 0),
        raft_log_format: 1,
        value_record_format: REPLICATED_VALUE_RECORD_FORMAT_VERSION,
        wire_frame_version: CACHE_INVALIDATION_FRAME_VERSION,
    };

    guard.check(step).unwrap();
}

#[test]
fn self_heal_upgrade_guard_refuses_incompatible_step() {
    let guard = UpgradeGuard::current();
    let step = UpgradeStep {
        from: CompatVersion::new(0, 43, 0),
        to: CompatVersion::new(0, 44, 0),
        raft_log_format: 1,
        value_record_format: 1,
        wire_frame_version: 1,
    };

    let error = guard.check(step).unwrap_err();

    assert!(error.to_string().contains("compatibility window"));
}

#[test]
#[ignore = "chaos gate: zone-loss self-heal"]
fn self_heal_zone_loss_self_heals_to_target_rf() {
    let policy = AutoRepairPolicy::new(RepairMode::Active, 1, 1, 2);
    let decision = policy.evaluate(2, 5);

    assert!(!decision.scheduled.is_empty());
    assert!(decision.scheduled.len() <= 2);
}
