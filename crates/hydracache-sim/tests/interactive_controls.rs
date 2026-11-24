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

fn applied_writes(world: &SimWorld) -> std::collections::BTreeMap<String, u64> {
    world
        .snapshot()
        .nodes
        .into_iter()
        .map(|node| (node.id, node.applied_writes))
        .collect()
}

// Schema 9: each node reports the writes it applied itself. Live nodes agree;
// a crashed node falls behind while the cluster writes and catches up when it
// restarts and copies the leader's state. commit_index stays cluster-wide.
#[test]
fn applied_writes_track_each_node_and_catch_up_on_restart() {
    let mut world = SimWorld::new(80, SimConfig::default());
    world.run(16);
    // The writer applies first; followers apply when ReplicatePut arrives on
    // the next step. Pause the workload and let deliveries finish to compare.
    let in_flight = applied_writes(&world);
    let writer = *in_flight.values().max().expect("nodes");
    assert!(
        in_flight.values().all(|count| *count + 1 >= writer),
        "{in_flight:?}"
    );
    world.set_workload_enabled(false);
    world.run(2);
    let healthy = applied_writes(&world);
    let first = *healthy.values().next().expect("nodes");
    assert!(first > 0, "the workload wrote something: {healthy:?}");
    assert!(
        healthy.values().all(|count| *count == first),
        "live nodes agree: {healthy:?}"
    );
    world.set_workload_enabled(true);

    assert!(world.crash_node("node-2"));
    world.run(12);
    let crashed = applied_writes(&world);
    assert!(
        crashed["node-2"] < crashed["node-0"],
        "a crashed node misses writes: {crashed:?}"
    );
    let snapshot = world.snapshot();
    let commit = |id: &str| {
        snapshot
            .nodes
            .iter()
            .find(|node| node.id == id)
            .unwrap()
            .commit_index
    };
    assert_eq!(
        commit("node-2"),
        commit("node-0"),
        "commit_index is still cluster-wide"
    );

    assert!(world.restart_node("node-2"));
    let restarted = applied_writes(&world);
    let source = restarted
        .iter()
        .filter(|(id, _)| id.as_str() != "node-2")
        .map(|(_, count)| *count)
        .max()
        .unwrap();
    assert_eq!(
        restarted["node-2"], source,
        "catch-up copies the source's progress"
    );
}

// A follower cut off from the writer by partitioned links stops applying
// replicated writes: the cluster has no log to replay them later.
#[test]
fn applied_writes_fall_behind_behind_a_partition() {
    let mut world = SimWorld::new(80, SimConfig::default());
    world.run(16);
    let leader = world
        .snapshot()
        .nodes
        .iter()
        .find(|node| node.role == "leader")
        .map(|node| node.id.clone())
        .expect("leader");
    let follower = world
        .snapshot()
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .find(|id| *id != leader)
        .expect("follower");
    assert!(world.partition_link(leader.as_str(), follower.as_str()));
    assert!(world.partition_link(follower.as_str(), leader.as_str()));
    world.run(12);
    let counts = applied_writes(&world);
    assert!(counts[&follower] < counts[&leader], "{counts:?}");
    let json = world.snapshot_json();
    assert!(
        json.contains("\"applied_writes\""),
        "the field is in the snapshot JSON"
    );
}
