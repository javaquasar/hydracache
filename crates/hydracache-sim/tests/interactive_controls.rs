use hydracache_sim::{LinkStateView, SimConfig, SimWorld};

#[test]
fn partition_and_heal_link_update_snapshot_state() {
    let mut world = SimWorld::new(50, SimConfig::default());

    assert!(world.partition_link("node-0", "node-1"));
    let partitioned = world.snapshot();
    let link = partitioned
        .links
        .iter()
        .find(|link| link.from == "node-0" && link.to == "node-1")
        .expect("directed link exists");
    assert_eq!(link.state, LinkStateView::Partitioned);

    assert!(world.heal_link("node-0", "node-1"));
    let healed = world.snapshot();
    let link = healed
        .links
        .iter()
        .find(|link| link.from == "node-0" && link.to == "node-1")
        .expect("directed link exists");
    assert_eq!(link.state, LinkStateView::Up);
}

// The election model sees the same cuts as the network: once a link is cut both
// ways, no modeled election signal is shown crossing it, and the leader keeps a
// 2-of-3 quorum over the remaining links.
#[test]
fn election_signals_do_not_cross_a_partitioned_link() {
    let election_kinds = [
        "vote_request",
        "vote_response",
        "leader_heartbeat",
        "heartbeat_ack",
    ];
    let mut world = SimWorld::new(80, SimConfig::default());
    // Let the formation settle so the leader is in steady-state heartbeats.
    world.run(12);
    let leader = world
        .snapshot()
        .nodes
        .iter()
        .find(|node| node.role == "leader")
        .map(|node| node.id.clone())
        .expect("seed 80 elects a leader within 12 steps");
    let follower = world
        .snapshot()
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .find(|id| *id != leader)
        .expect("a follower");

    assert!(world.partition_link(leader.as_str(), follower.as_str()));
    assert!(world.partition_link(follower.as_str(), leader.as_str()));
    for _ in 0..8 {
        world.step();
        let snapshot = world.snapshot();
        let crossing = snapshot
            .in_flight
            .iter()
            .filter(|message| election_kinds.contains(&message.kind.as_str()))
            .find(|message| {
                (message.from == leader && message.to == follower)
                    || (message.from == follower && message.to == leader)
            });
        assert!(
            crossing.is_none(),
            "election signal crossed the cut: {crossing:?}"
        );
        assert!(
            snapshot
                .nodes
                .iter()
                .any(|node| node.id == leader && node.role == "leader"),
            "the leader keeps a 2-of-3 quorum"
        );
    }
}

#[test]
fn crash_and_restart_node_update_snapshot_state() {
    let mut world = SimWorld::new(51, SimConfig::default());

    assert!(world.crash_node("node-2"));
    let crashed = world.snapshot();
    let node = crashed
        .nodes
        .iter()
        .find(|node| node.id == "node-2")
        .expect("node exists");
    assert!(node.crashed);
    assert!(!node.up);

    assert!(world.restart_node("node-2"));
    let restarted = world.snapshot();
    let node = restarted
        .nodes
        .iter()
        .find(|node| node.id == "node-2")
        .expect("node exists");
    assert!(!node.crashed);
    assert!(node.up);
}

#[test]
fn workload_toggle_stops_client_operation_generation() {
    let mut world = SimWorld::new(52, SimConfig::default());
    world.set_workload_enabled(false);
    world.run(5);
    assert_eq!(world.outcome().accepted_ops, 0);

    world.set_workload_enabled(true);
    world.run(1);
    assert_eq!(world.outcome().accepted_ops, 1);
}
