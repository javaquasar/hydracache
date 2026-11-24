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

fn drops(world: &SimWorld, from: &str, to: &str) -> hydracache_sim::LinkDropsView {
    world
        .snapshot()
        .links
        .into_iter()
        .find(|link| link.from == from && link.to == to)
        .expect("directed link exists")
        .drops
}

// Schema 9: a link reports the messages it dropped, by cause, instead of
// refusing traffic without a trace.
#[test]
fn links_count_dropped_messages_by_cause() {
    let mut world = SimWorld::new(80, SimConfig::default());
    world.run(12);
    assert_eq!(
        drops(&world, "node-0", "node-1"),
        Default::default(),
        "a healthy link drops nothing"
    );

    // Cut one direction: everything node-0 sends to node-1 is refused at send;
    // the reverse direction is untouched.
    assert!(world.partition_link("node-0", "node-1"));
    world.run(6);
    let cut = drops(&world, "node-0", "node-1");
    assert!(cut.cut_at_send > 0, "{cut:?}");
    assert_eq!(drops(&world, "node-1", "node-0").cut_at_send, 0);
    assert!(world.heal_link("node-0", "node-1"));
    let healed = drops(&world, "node-0", "node-1").cut_at_send;
    world.run(4);
    assert_eq!(
        drops(&world, "node-0", "node-1").cut_at_send,
        healed,
        "a healed link stops refusing"
    );

    // An injected drop discards exactly the next message on that link.
    world
        .apply_control_action(hydracache_sim::ControlActionV1::DropNext {
            at_step: world.outcome().steps,
            from: "node-1".to_owned(),
            to: "node-2".to_owned(),
        })
        .expect("drop_next applies");
    world.run(2);
    assert_eq!(drops(&world, "node-1", "node-2").injected, 1);

    // Messages delivered to a crashed node are counted against its links.
    assert!(world.crash_node("node-2"));
    world.run(3);
    assert!(drops(&world, "node-0", "node-2").target_down > 0);
}

// A message in flight when its link is cut is dropped on delivery.
#[test]
fn messages_in_flight_when_a_link_is_cut_count_as_dropped_on_delivery() {
    let mut world = SimWorld::new(80, SimConfig::default());
    world.run(12);
    world
        .apply_control_action(hydracache_sim::ControlActionV1::DelayNext {
            at_step: world.outcome().steps,
            from: "node-0".to_owned(),
            to: "node-1".to_owned(),
            millis: 50,
        })
        .expect("delay_next applies");
    world.run(1);
    assert!(world.partition_link("node-0", "node-1"));
    world.run(60);
    assert!(drops(&world, "node-0", "node-1").cut_at_delivery > 0);
}
