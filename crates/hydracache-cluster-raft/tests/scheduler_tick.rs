use std::collections::{BTreeMap, BTreeSet};

use hydracache_cluster_raft::RaftRuntimeRole;
use hydracache_cluster_testkit::RuntimeRaftCluster;

#[test]
fn process_pause_and_uneven_ticks_never_create_two_leaders_per_term() {
    let mut cluster = RuntimeRaftCluster::three_node();
    let mut leaders_by_term = BTreeMap::<u64, BTreeSet<u64>>::new();
    cluster.campaign(1);
    record_leaders(&cluster, &mut leaders_by_term);

    // Isolation models an OS-paused process more faithfully than ticking its
    // in-process runtime: it cannot send or consume heartbeats while the two
    // live peers continue at deliberately uneven tick cadences.
    cluster.filters().isolate(1, [2, 3]);
    for step in 0..24 {
        cluster.tick_node(2);
        if step % 3 == 0 {
            cluster.tick_node(3);
        }
        record_leaders(&cluster, &mut leaders_by_term);
        if connected_leader(&cluster, [2, 3]).is_some() {
            break;
        }
    }
    let replacement = connected_leader(&cluster, [2, 3])
        .expect("connected majority must elect a replacement for the paused leader");
    assert_ne!(replacement, 1);

    for step in 0..16 {
        cluster.tick_node(replacement);
        if step % 4 == 0 {
            let peer = if replacement == 2 { 3 } else { 2 };
            cluster.tick_node(peer);
        }
        record_leaders(&cluster, &mut leaders_by_term);
    }

    cluster.filters().recover();
    cluster.tick_all(12);
    record_leaders(&cluster, &mut leaders_by_term);

    for (term, leaders) in leaders_by_term {
        assert!(
            leaders.len() <= 1,
            "pause/uneven-tick schedule observed two leaders in term {term}: {leaders:?}"
        );
    }
}

#[tokio::test]
async fn resumed_demoted_process_never_reports_authoritative_membership() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.join_member(1, "before-pause").await.unwrap();

    cluster.filters().isolate(1, [2, 3]);
    let replacement = elect_connected_majority(&mut cluster);
    cluster
        .join_member(replacement, "committed-while-paused")
        .await
        .unwrap();
    let command_id = "member-upsert:committed-while-paused:1";
    assert!(!cluster.node(1).command_applied(command_id));
    assert!(cluster.node(replacement).command_applied(command_id));

    cluster.filters().recover();
    cluster.tick_all(16);

    let resumed = cluster.node(1).snapshot();
    assert_ne!(
        resumed.role,
        RaftRuntimeRole::Leader,
        "the resumed former leader must demote before its membership can be authoritative"
    );
    assert!(
        cluster.node(1).command_applied(command_id),
        "the resumed process must catch up before exposing the converged membership"
    );
    let expected = member_ids(&cluster, replacement);
    assert_eq!(member_ids(&cluster, 1), expected);
}

#[test]
fn canary_resumed_demoted_process_is_accepted_as_authoritative() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.filters().isolate(1, [2, 3]);
    let replacement = elect_connected_majority(&mut cluster);
    let stale_before_resume = cluster.node(1).snapshot();
    assert_eq!(stale_before_resume.role, RaftRuntimeRole::Leader);

    cluster.filters().recover();
    cluster.tick_all(12);
    let resumed = cluster.node(1).snapshot();
    assert_ne!(resumed.role, RaftRuntimeRole::Leader);
    assert!(resumed.term >= cluster.node(replacement).snapshot().term);

    let accepted_as_authoritative = std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref()
        == Ok("W10")
        && stale_before_resume.role == RaftRuntimeRole::Leader;
    assert!(
        !accepted_as_authoritative,
        "HC-CANARY-RED:W10 resumed demoted process was accepted from its stale leader view"
    );
}

fn elect_connected_majority(cluster: &mut RuntimeRaftCluster) -> u64 {
    for step in 0..32 {
        cluster.tick_node(2);
        if step % 2 == 0 {
            cluster.tick_node(3);
        }
        if let Some(leader) = connected_leader(cluster, [2, 3]) {
            return leader;
        }
    }
    panic!("connected majority did not elect a leader under uneven ticks");
}

fn connected_leader(
    cluster: &RuntimeRaftCluster,
    nodes: impl IntoIterator<Item = u64>,
) -> Option<u64> {
    nodes
        .into_iter()
        .find(|node_id| cluster.node(*node_id).snapshot().role == RaftRuntimeRole::Leader)
}

fn record_leaders(
    cluster: &RuntimeRaftCluster,
    leaders_by_term: &mut BTreeMap<u64, BTreeSet<u64>>,
) {
    for node_id in cluster.node_ids() {
        let snapshot = cluster.node(node_id).snapshot();
        if snapshot.role == RaftRuntimeRole::Leader {
            leaders_by_term
                .entry(snapshot.term)
                .or_default()
                .insert(node_id);
        }
    }
}

fn member_ids(cluster: &RuntimeRaftCluster, node_id: u64) -> BTreeSet<String> {
    cluster
        .node(node_id)
        .members()
        .into_iter()
        .map(|member| member.node_id.as_str().to_owned())
        .collect()
}
