use std::collections::BTreeMap;

use hydracache::{
    admit_autoscaler_intent, evaluate_capacity, scale_in_removal_allowed,
    scale_out_counts_toward_quorum, AutoscalerAdmissionPolicy, AutoscalerIntent, CapacitySample,
    CapacityThresholds, ClusterEpoch, ClusterNodeId, CompatVersion, MovePhase, NodeTopology,
    PartitionId, Replicas, ReplicationConfig, ScaleAction, ScaleRecommendation, UpgradeGuard,
    UpgradeStep, ZoneAwareReplicaSet, CACHE_INVALIDATION_FRAME_VERSION,
    REPLICATED_VALUE_RECORD_FORMAT_VERSION,
};

fn compat() -> UpgradeStep {
    UpgradeStep {
        from: CompatVersion::new(0, 42, 0),
        to: CompatVersion::new(0, 43, 0),
        raft_log_format: 1,
        value_record_format: REPLICATED_VALUE_RECORD_FORMAT_VERSION,
        wire_frame_version: CACHE_INVALIDATION_FRAME_VERSION,
    }
}

fn replication() -> ReplicationConfig {
    ReplicationConfig {
        replication_factor: 3,
        read_quorum: 2,
        write_quorum: 2,
        sync_backups: 1,
        async_backups: 1,
        max_replicated_entry_bytes: 1024,
        replicate_values: true,
    }
}

fn policy() -> AutoscalerAdmissionPolicy {
    AutoscalerAdmissionPolicy::new(
        ClusterEpoch::new(4),
        replication(),
        UpgradeGuard::current(),
        2,
    )
}

fn healthy_candidate() -> ZoneAwareReplicaSet {
    let mut topology = BTreeMap::new();
    topology.insert(
        ClusterNodeId::from("node-a"),
        NodeTopology::new("eu", "az-a"),
    );
    topology.insert(
        ClusterNodeId::from("node-b"),
        NodeTopology::new("eu", "az-b"),
    );
    topology.insert(
        ClusterNodeId::from("node-c"),
        NodeTopology::new("eu", "az-c"),
    );
    ZoneAwareReplicaSet {
        replicas: Replicas::new(
            "node-a",
            vec![ClusterNodeId::from("node-b"), ClusterNodeId::from("node-c")],
        ),
        topology,
        placement_zone_underspread: false,
    }
}

#[test]
fn capacity_autoscale_signal_recommends_scale_out_under_memory_pressure() {
    let mut sample = CapacitySample::new("eu");
    sample.memory_pressure = 0.91;

    let signal = evaluate_capacity(sample, CapacityThresholds::default());

    assert_eq!(
        signal.recommendation,
        ScaleRecommendation::ScaleOut { suggested: 1 }
    );
}

#[test]
fn capacity_autoscale_signal_recommends_rebalance_for_hot_partition_skew() {
    let mut sample = CapacitySample::new("eu");
    sample.hot_partition_skew = 3.5;

    let signal = evaluate_capacity(sample, CapacityThresholds::default());

    assert_eq!(signal.recommendation, ScaleRecommendation::Rebalance);
}

#[test]
fn capacity_autoscale_signal_recommends_scale_in_after_dwell() {
    let mut sample = CapacitySample::new("eu");
    sample.memory_pressure = 0.10;
    sample.scale_in_candidates = vec![ClusterNodeId::from("node-c")];
    sample.seconds_since_last_scale = 10_000;

    let signal = evaluate_capacity(sample, CapacityThresholds::default());

    assert_eq!(
        signal.recommendation,
        ScaleRecommendation::ScaleIn {
            drain: vec![ClusterNodeId::from("node-c")]
        }
    );
}

#[test]
fn capacity_autoscale_dwell_window_holds_even_under_pressure() {
    let mut sample = CapacitySample::new("eu");
    sample.memory_pressure = 0.99;
    sample.seconds_since_last_scale = 1;

    let signal = evaluate_capacity(sample, CapacityThresholds::default());

    assert_eq!(signal.recommendation, ScaleRecommendation::Hold);
}

#[test]
fn capacity_autoscale_scale_out_counts_toward_quorum_only_after_backfill() {
    let admission = admit_autoscaler_intent(
        AutoscalerIntent::ScaleOut {
            node: ClusterNodeId::from("node-c"),
            topology: NodeTopology::new("eu", "az-c"),
            candidate: healthy_candidate(),
            backfill_sources: vec![
                (PartitionId::new(1), ClusterNodeId::from("node-a"), 10),
                (PartitionId::new(2), ClusterNodeId::from("node-b"), 20),
            ],
            compat: compat(),
        },
        policy(),
    )
    .unwrap();
    let mut plan = admission.plan;

    assert_eq!(admission.action, ScaleAction::ScaleOut);
    assert!(!admission.quorum_eligible);
    assert!(!scale_out_counts_toward_quorum(&plan));

    for movement in &mut plan.moves {
        movement.record_backfill(movement.total_bytes);
        movement.advance();
        movement.advance();
    }

    assert!(plan
        .moves
        .iter()
        .all(|movement| movement.phase == MovePhase::Commit));
    assert!(scale_out_counts_toward_quorum(&plan));
}

#[test]
fn capacity_autoscale_scale_in_drains_before_removal() {
    let admission = admit_autoscaler_intent(
        AutoscalerIntent::ScaleIn {
            drain: ClusterNodeId::from("node-a"),
            remaining_voters: 2,
            drain_targets: vec![
                (PartitionId::new(1), ClusterNodeId::from("node-b"), 10),
                (PartitionId::new(2), ClusterNodeId::from("node-c"), 20),
            ],
            compat: compat(),
        },
        policy(),
    )
    .unwrap();
    let mut plan = admission.plan;

    assert_eq!(admission.action, ScaleAction::ScaleIn);
    assert!(!admission.removal_allowed);
    assert!(!scale_in_removal_allowed(&plan));
    assert!(plan
        .moves
        .iter()
        .all(|movement| movement.from == ClusterNodeId::from("node-a")));

    for movement in &mut plan.moves {
        movement.record_backfill(movement.total_bytes);
        movement.advance();
        movement.advance();
        movement.advance();
    }

    assert!(scale_in_removal_allowed(&plan));
}

#[test]
fn capacity_autoscale_intent_violating_zone_spread_is_rejected() {
    let mut candidate = healthy_candidate();
    candidate.placement_zone_underspread = true;

    let error = admit_autoscaler_intent(
        AutoscalerIntent::ScaleOut {
            node: ClusterNodeId::from("node-c"),
            topology: NodeTopology::new("eu", "az-c"),
            candidate,
            backfill_sources: vec![(PartitionId::new(1), ClusterNodeId::from("node-a"), 10)],
            compat: compat(),
        },
        policy(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("zone-spread write quorum"));
}

#[test]
fn capacity_autoscale_intent_outside_compat_window_is_refused() {
    let mut step = compat();
    step.from = CompatVersion::new(0, 41, 0);
    step.to = CompatVersion::new(0, 44, 0);

    let error = admit_autoscaler_intent(
        AutoscalerIntent::ScaleIn {
            drain: ClusterNodeId::from("node-a"),
            remaining_voters: 2,
            drain_targets: vec![(PartitionId::new(1), ClusterNodeId::from("node-b"), 10)],
            compat: step,
        },
        policy(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("compatibility window"));
}

#[test]
fn capacity_autoscale_intent_breaking_quorum_is_rejected() {
    let error = admit_autoscaler_intent(
        AutoscalerIntent::ScaleIn {
            drain: ClusterNodeId::from("node-a"),
            remaining_voters: 1,
            drain_targets: vec![(PartitionId::new(1), ClusterNodeId::from("node-b"), 10)],
            compat: compat(),
        },
        policy(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("write quorum"));
}
