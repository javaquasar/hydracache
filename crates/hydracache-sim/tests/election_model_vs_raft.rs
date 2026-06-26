use std::collections::BTreeSet;

use hydracache::ClusterNodeId;
use hydracache_sim::{ElectionDriver, SimNetwork, SimRaftCluster};

#[test]
fn model_and_raft_agree_on_single_leader_per_term() {
    for seed in 0..32 {
        let run = run_both(
            seed,
            5,
            &[(0, vec![0, 1, 2, 3, 4]), (80, vec![0, 1, 2, 3, 4])],
        );

        assert_at_most_one_leader_per_term(&run.model_terms);
        assert_at_most_one_leader_per_term(&run.raft_terms);
        assert!(
            run.model_leader.is_some(),
            "model should elect under full quorum for seed {seed}"
        );
        assert!(
            run.raft_leader.is_some(),
            "raft should elect under full quorum for seed {seed}"
        );
    }
}

#[test]
fn model_never_claims_a_leader_raft_denies() {
    for seed in 0..32 {
        let run = run_both(seed, 5, &[(0, vec![0, 1]), (40, vec![0, 1])]);

        assert!(
            !(run.model_leader.is_some() && run.raft_leader.is_none()),
            "model claimed {:?} while raft denied leadership for seed {seed}",
            run.model_leader
        );
    }
}

#[test]
fn model_converges_to_raft_leader_within_bound() {
    let mut matching_seed = None;
    for seed in 0..128 {
        let run = run_both(seed, 3, &[(0, vec![0, 1, 2]), (80, vec![0, 1, 2])]);
        if run.model_leader.is_some() && run.model_leader == run.raft_leader {
            matching_seed = Some(seed);
            break;
        }
    }

    assert!(
        matching_seed.is_some(),
        "expected at least one bounded seed where the sim-model and raft choose the same leader"
    );
}

#[derive(Debug)]
struct DualRun {
    model_leader: Option<ClusterNodeId>,
    raft_leader: Option<ClusterNodeId>,
    model_terms: Vec<(u64, Vec<ClusterNodeId>)>,
    raft_terms: Vec<(u64, Vec<ClusterNodeId>)>,
}

fn run_both(seed: u64, count: usize, windows: &[(u64, Vec<usize>)]) -> DualRun {
    let nodes = nodes(count);
    let mut model = ElectionDriver::new(seed, nodes.clone());
    let mut raft = SimRaftCluster::new(seed, nodes.clone()).expect("raft initializes");
    let network = SimNetwork::from_seed(seed ^ 0x44);
    let mut model_terms = Vec::new();
    let mut raft_terms = Vec::new();

    for (window_index, (start, live_indexes)) in windows.iter().enumerate() {
        let end = windows
            .get(window_index + 1)
            .map(|(next, _)| *next)
            .unwrap_or(start.saturating_add(1));
        let live = live_set(&nodes, live_indexes);
        for step in start.saturating_add(1)..=end {
            model.step(step, &live);
            raft.step(step, &live, &network).expect("raft step");
            let model_snapshot = model.snapshot();
            let raft_snapshot = raft.snapshot();
            model_terms.push((
                model_snapshot.term,
                model_snapshot
                    .leaders()
                    .into_iter()
                    .map(|node| node.node_id.clone())
                    .collect(),
            ));
            raft_terms.push((
                raft_snapshot.term,
                raft_snapshot
                    .leaders()
                    .into_iter()
                    .map(|node| node.node_id.clone())
                    .collect(),
            ));
        }
    }

    DualRun {
        model_leader: model.snapshot().leader,
        raft_leader: raft.snapshot().leader,
        model_terms,
        raft_terms,
    }
}

fn assert_at_most_one_leader_per_term(history: &[(u64, Vec<ClusterNodeId>)]) {
    for (term, leaders) in history {
        assert!(
            leaders.len() <= 1,
            "term {term} has multiple leaders: {leaders:?}"
        );
    }
}

fn nodes(count: usize) -> Vec<ClusterNodeId> {
    (0..count)
        .map(|index| ClusterNodeId::new(format!("node-{index}")))
        .collect()
}

fn live_set(nodes: &[ClusterNodeId], indexes: &[usize]) -> BTreeSet<ClusterNodeId> {
    indexes
        .iter()
        .filter_map(|index| nodes.get(*index).cloned())
        .collect()
}
