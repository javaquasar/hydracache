use std::collections::BTreeSet;

use hydracache::ClusterNodeId;
use hydracache_sim::{
    ElectionDriver, ElectionDriverSnapshot, ElectionSource, FormationPhase, NodeFsmState,
    SimConfig, SimWorld,
};

#[test]
fn cold_start_elects_single_leader_deterministically() {
    let mut world = SimWorld::new(0x53_01, SimConfig::default());
    world.set_workload_enabled(false);

    world.run(8);

    let snapshot = world.election_snapshot();
    let leaders = snapshot.leaders();
    assert_eq!(snapshot.source, ElectionSource::SimModel);
    assert_eq!(snapshot.phase, FormationPhase::Formed);
    assert_eq!(leaders.len(), 1, "{snapshot:?}");
    assert_eq!(snapshot.leader.as_ref(), Some(&leaders[0].node_id));
    assert!(leaders[0].votes_received >= 2);
    assert!(snapshot.nodes.iter().all(|node| node.term == snapshot.term));
}

#[test]
fn election_run_is_replayable_from_seed() {
    let left = run_driver(0x53_02, 12);
    let right = run_driver(0x53_02, 12);

    assert_eq!(left, right);
    assert!(left.leader.is_some());
    assert!(left
        .trace
        .iter()
        .any(|event| event.contains("source:sim-model")));
}

#[test]
fn election_determinism_holds_over_1000_seeds() {
    for seed in 0..1000 {
        let left = run_driver(seed, 8);
        let right = run_driver(seed, 8);

        assert_eq!(left, right, "seed {seed} diverged");
        assert_eq!(left.source, ElectionSource::SimModel);
        assert_eq!(
            left.leaders().len(),
            1,
            "seed {seed} did not elect one leader"
        );
        assert_eq!(left.phase, FormationPhase::Formed);
    }
}

#[test]
fn sim_model_is_labelled_and_makes_no_product_claim() {
    let source = ElectionSource::SimModel;

    assert_eq!(source.as_str(), "sim-model");
    assert!(!source.carries_product_consensus_claim());
    assert!(source
        .disclosure()
        .contains("not a product consensus claim"));
}

fn run_driver(seed: u64, steps: u64) -> ElectionDriverSnapshot {
    let nodes = ["node-0", "node-1", "node-2"]
        .into_iter()
        .map(ClusterNodeId::from)
        .collect::<Vec<_>>();
    let live_nodes = nodes.iter().cloned().collect::<BTreeSet<_>>();
    let mut driver = ElectionDriver::new(seed, nodes);

    for step in 1..=steps {
        driver.step(step, &live_nodes);
    }

    let snapshot = driver.snapshot();
    assert!(snapshot
        .nodes
        .iter()
        .all(|node| node.state == NodeFsmState::Leader || node.state == NodeFsmState::Follower));
    snapshot
}
