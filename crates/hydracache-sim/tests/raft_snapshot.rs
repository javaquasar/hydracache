use hydracache_sim::{SimConfig, SimWorld, VerdictView, SIM_SNAPSHOT_SCHEMA_VERSION};

#[test]
fn raft_backend_reports_election_source_raft() {
    let mut world = raft_world(0x5310);
    world.run(80);

    let snapshot = world.snapshot();

    assert_eq!(snapshot.election_source, "raft");
    assert!(snapshot.election_disclosure.contains("real raft-rs"));
    assert!(snapshot.election_disclosure.contains("simulator network"));
}

#[test]
fn node_views_reflect_real_raft_term_and_leader() {
    let mut world = raft_world(0x5311);
    world.run(80);

    let election = world.election_snapshot();
    let snapshot = world.snapshot();
    let leader = election.leader.expect("raft leader elected");
    let leader_view = snapshot
        .nodes
        .iter()
        .find(|node| node.id == leader.as_str())
        .expect("leader visible in node views");

    assert_eq!(leader_view.vote_state, "leader");
    assert_eq!(leader_view.role, "leader");
    assert_eq!(leader_view.term, election.term);
    assert!(leader_view.votes_received >= 2);
}

#[test]
fn c3_invariants_hold_against_real_raft() {
    let mut world = raft_world(0x5312);
    world.run(80);
    let leader = world
        .snapshot()
        .nodes
        .iter()
        .find(|node| node.vote_state == "leader")
        .expect("leader visible")
        .id
        .clone();

    assert!(world.isolate_node(leader));
    world.run(80);

    assert!(
        world.invariant_report().is_ok(),
        "raft-backed C3 invariants should hold: {:?}",
        world.invariant_report().violations
    );
    assert_eq!(world.snapshot().verdict, VerdictView::Holding);
}

#[test]
fn raft_snapshot_uses_existing_schema_version() {
    let mut world = raft_world(0x5313);
    world.run(20);
    let snapshot = world.snapshot();

    assert_eq!(SIM_SNAPSHOT_SCHEMA_VERSION, 6);
    assert_eq!(snapshot.schema_version, SIM_SNAPSHOT_SCHEMA_VERSION);
    assert_eq!(snapshot.election_source, "raft");
}

fn raft_world(seed: u64) -> SimWorld {
    let mut world = SimWorld::with_raft_election(seed, SimConfig::default());
    world.set_workload_enabled(false);
    world
}
