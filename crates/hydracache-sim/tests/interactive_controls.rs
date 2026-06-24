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
