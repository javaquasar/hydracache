use hydracache::{
    hedge_winner, plan_hedged_read, ClusterEpoch, ClusterNodeId, HedgePolicy, NodeTopology,
    PartitionId, ReplicaObservation, ReplicaScorer, ReplicaSelection, ReplicatedValueRecord,
};

#[test]
fn locality_reads_eventual_read_prefers_local_zone() {
    let local = NodeTopology::new("eu", "az-a");
    let mut scorer = ReplicaScorer::new();
    scorer.observe(ReplicaObservation::healthy(
        "remote-fast",
        NodeTopology::new("eu", "az-b"),
        1,
        1,
        ClusterEpoch::new(1),
    ));
    scorer.observe(ReplicaObservation::healthy(
        "local-slower",
        NodeTopology::new("eu", "az-a"),
        20,
        1,
        ClusterEpoch::new(1),
    ));

    let ordered = scorer.order(
        &[
            ClusterNodeId::from("remote-fast"),
            ClusterNodeId::from("local-slower"),
        ],
        &local,
        ReplicaSelection::NearestZone,
    );

    assert_eq!(ordered[0], ClusterNodeId::from("local-slower"));
}

#[test]
fn locality_reads_eventual_read_prefers_local_zone_live() {
    let local = NodeTopology::new("eu", "az-a");
    let mut scorer = ReplicaScorer::new();
    for observation in [
        ReplicaObservation::healthy(
            "remote-fast",
            NodeTopology::new("eu", "az-b"),
            1,
            2,
            ClusterEpoch::new(4),
        ),
        ReplicaObservation::healthy(
            "local-slower",
            NodeTopology::new("eu", "az-a"),
            25,
            2,
            ClusterEpoch::new(4),
        ),
    ] {
        scorer.observe(observation);
    }

    let ordered = scorer.order(
        &[
            ClusterNodeId::from("remote-fast"),
            ClusterNodeId::from("local-slower"),
        ],
        &local,
        ReplicaSelection::NearestZone,
    );

    assert_eq!(ordered[0], ClusterNodeId::from("local-slower"));
}

#[test]
fn locality_reads_quorum_read_still_contacts_read_quorum() {
    let ordered = vec![
        ClusterNodeId::from("a"),
        ClusterNodeId::from("b"),
        ClusterNodeId::from("c"),
    ];
    let policy = HedgePolicy::new(50, 1, 5);

    let plan = plan_hedged_read(&ordered, 2, 50, &[5, 7, 9], policy);

    assert_eq!(plan.required_acks, 2);
    assert_eq!(plan.primary, Some(ClusterNodeId::from("a")));
    assert_eq!(plan.hedges, vec![ClusterNodeId::from("b")]);
}

#[test]
fn locality_reads_slow_replica_triggers_hedge_and_returns_fresh() {
    let ordered = vec![
        ClusterNodeId::from("slow"),
        ClusterNodeId::from("fresh"),
        ClusterNodeId::from("stale"),
    ];
    let plan = plan_hedged_read(&ordered, 1, 100, &[10, 20, 30], HedgePolicy::new(50, 2, 5));
    let winner = hedge_winner([
        ReplicatedValueRecord::value(PartitionId::new(1), 10, ClusterEpoch::new(1), b"slow"),
        ReplicatedValueRecord::value(PartitionId::new(1), 11, ClusterEpoch::new(1), b"fresh"),
    ])
    .expect("winner");

    assert_eq!(
        plan.hedges,
        vec![ClusterNodeId::from("fresh"), ClusterNodeId::from("stale")]
    );
    assert_eq!(winner.version, 11);
}

#[test]
fn locality_reads_slow_replica_triggers_hedge_returns_fresh_live() {
    let ordered = vec![
        ClusterNodeId::from("slow"),
        ClusterNodeId::from("fresh"),
        ClusterNodeId::from("stale"),
    ];
    let plan = plan_hedged_read(&ordered, 1, 100, &[10, 20, 30], HedgePolicy::new(50, 2, 5));
    let winner = hedge_winner([
        ReplicatedValueRecord::value(PartitionId::new(1), 10, ClusterEpoch::new(1), b"slow"),
        ReplicatedValueRecord::value(PartitionId::new(1), 12, ClusterEpoch::new(2), b"fresh"),
        ReplicatedValueRecord::value(PartitionId::new(1), 11, ClusterEpoch::new(1), b"stale"),
    ])
    .expect("winner");

    assert_eq!(
        plan.hedges,
        vec![ClusterNodeId::from("fresh"), ClusterNodeId::from("stale")]
    );
    assert_eq!(winner.version, 12);
    assert_eq!(winner.epoch, ClusterEpoch::new(2));
}

#[test]
fn locality_reads_hedge_winner_is_max_version_not_first_arrival() {
    let first_arrival =
        ReplicatedValueRecord::value(PartitionId::new(1), 1, ClusterEpoch::new(9), b"old");
    let later_fresher =
        ReplicatedValueRecord::value(PartitionId::new(1), 2, ClusterEpoch::new(9), b"new");

    let winner = hedge_winner([first_arrival, later_fresher]).expect("winner");

    assert_eq!(winner.version, 2);
}

#[test]
fn locality_reads_hedge_delay_adapts_to_rtt_distribution() {
    let policy = HedgePolicy::new(90, 1, 5);
    let low = policy.delay_ms(&[5, 6, 7, 8, 9]);
    let high = policy.delay_ms(&[5, 6, 7, 80, 90]);

    assert!(high > low);
    assert!(low >= 5);
}
