use std::collections::BTreeSet;

use hydracache::ClusterNodeId;
use hydracache_sim::{ElectionSource, PartitionSymmetry, SimNetwork, SimRaftCluster};

#[test]
fn sim_raft_elects_single_leader_deterministically() {
    let mut cluster = cluster(0x5301, 3);
    let network = SimNetwork::from_seed(0x44);
    let live = node_set(3);

    run_until_leader(&mut cluster, &live, &network, 1, 80);

    let snapshot = cluster.snapshot();
    assert_eq!(snapshot.source, ElectionSource::Raft);
    assert_eq!(snapshot.leaders().len(), 1, "{snapshot:?}");
    assert_eq!(
        snapshot.leader.as_ref(),
        Some(&snapshot.leaders()[0].node_id)
    );
    assert!(snapshot.term > 0);
}

#[test]
fn election_determinism_holds_over_1000_seeds() {
    for seed in 0..1000 {
        let left = raft_history(seed);
        let right = raft_history(seed);

        assert_eq!(left, right, "seed {seed} diverged");
        assert!(
            left.iter().any(|(leader, _, _, _)| leader.is_some()),
            "seed {seed} never elected a leader"
        );
    }
}

#[test]
fn leader_loss_triggers_real_reelection() {
    let mut cluster = cluster(0x5302, 3);
    let network = SimNetwork::from_seed(0x45);
    let mut live = node_set(3);

    let leader = run_until_leader(&mut cluster, &live, &network, 1, 80);
    let old_term = cluster.term();
    live.remove(&leader);

    let new_leader = run_until_leader(&mut cluster, &live, &network, 81, 180);

    assert_ne!(new_leader, leader);
    assert!(cluster.term() > old_term);
}

#[test]
fn partition_minority_cannot_elect() {
    let mut cluster = cluster(0x5303, 5);
    let mut network = SimNetwork::from_seed(0x46);
    let live = node_set(5);

    let old_leader = run_until_leader(&mut cluster, &live, &network, 1, 100);
    let mut minority = vec![old_leader.clone()];
    minority.push(
        live.iter()
            .find(|node| **node != old_leader)
            .cloned()
            .expect("second minority node"),
    );
    let majority = live
        .iter()
        .filter(|node| !minority.contains(node))
        .cloned()
        .collect::<Vec<_>>();
    network.partition((&minority, &majority), PartitionSymmetry::Symmetric);

    for step in 101..220 {
        cluster.step(step, &live, &network).expect("raft step");
    }

    let snapshot = cluster.snapshot();
    let leaders = snapshot.leaders();
    assert_eq!(leaders.len(), 1, "{snapshot:?}");
    assert!(
        majority.contains(&leaders[0].node_id),
        "minority unexpectedly retained/elected leader: {snapshot:?}"
    );
}

#[test]
fn conf_change_add_then_remove_node_is_deterministic() {
    let left = conf_change_history(0x5304);
    let right = conf_change_history(0x5304);

    assert_eq!(left, right);
    let final_snapshot = left.last().expect("history has final snapshot");
    assert!(
        final_snapshot
            .3
            .iter()
            .any(|(node, state)| node == "node-3" && *state == "disconnected"),
        "removed node should be visible as disconnected in the final snapshot"
    );
}

type HistoryRow = (
    Option<String>,
    u64,
    Vec<(u64, u64, u64, u64)>,
    Vec<(String, String)>,
);

fn raft_history(seed: u64) -> Vec<HistoryRow> {
    let mut cluster = cluster(seed, 3);
    let network = SimNetwork::from_seed(seed ^ 0x44);
    let live = node_set(3);
    let mut history = Vec::new();
    for step in 1..=80 {
        cluster.step(step, &live, &network).expect("raft step");
        let snapshot = cluster.snapshot();
        history.push((
            snapshot.leader.map(|leader| leader.to_string()),
            snapshot.term,
            cluster.inflight_order(),
            snapshot
                .nodes
                .into_iter()
                .map(|node| (node.node_id.to_string(), node.state.to_string()))
                .collect(),
        ));
    }
    history
}

fn conf_change_history(seed: u64) -> Vec<HistoryRow> {
    let mut cluster = cluster(seed, 3);
    let network = SimNetwork::from_seed(seed ^ 0x45);
    let mut live = node_set(3);
    run_until_leader(&mut cluster, &live, &network, 1, 100);

    let added = ClusterNodeId::from("node-3");
    cluster
        .add_node(added.clone(), 101)
        .expect("add-node proposal");
    live.insert(added.clone());
    for step in 101..180 {
        cluster.step(step, &live, &network).expect("raft step");
    }

    cluster
        .remove_node(&added, 180)
        .expect("remove-node proposal");
    live.remove(&added);
    let mut history = Vec::new();
    for step in 180..240 {
        cluster.step(step, &live, &network).expect("raft step");
        let snapshot = cluster.snapshot();
        history.push((
            snapshot.leader.map(|leader| leader.to_string()),
            snapshot.term,
            cluster.inflight_order(),
            snapshot
                .nodes
                .into_iter()
                .map(|node| (node.node_id.to_string(), node.state.to_string()))
                .collect(),
        ));
    }
    history
}

fn run_until_leader(
    cluster: &mut SimRaftCluster,
    live: &BTreeSet<ClusterNodeId>,
    network: &SimNetwork,
    start: u64,
    end: u64,
) -> ClusterNodeId {
    for step in start..=end {
        cluster.step(step, live, network).expect("raft step");
        if let Some(leader) = cluster.leader() {
            return leader;
        }
    }
    panic!("raft did not elect a leader by step {end}");
}

fn cluster(seed: u64, count: usize) -> SimRaftCluster {
    SimRaftCluster::new(seed, nodes(count)).expect("raft cluster initializes")
}

fn nodes(count: usize) -> Vec<ClusterNodeId> {
    (0..count)
        .map(|index| ClusterNodeId::new(format!("node-{index}")))
        .collect()
}

fn node_set(count: usize) -> BTreeSet<ClusterNodeId> {
    nodes(count).into_iter().collect()
}
