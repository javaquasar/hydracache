use std::collections::BTreeSet;

use hydracache_cluster_raft::RaftRuntimeRole;
use hydracache_cluster_testkit::{RaftFilterAction, RaftPacketFilter, RuntimeRaftCluster};

const LOAD_COMMANDS: usize = 24;

#[tokio::test]
async fn membership_change_under_partition_loses_no_committed_metadata_command() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.join_member(1, "load-baseline").await.unwrap();

    // Drop only follower-to-leader traffic from node 3. Nodes 1 and 2 retain a
    // quorum while the third voter observes an asymmetric, stale view.
    cluster.filters().add_filter(
        RaftPacketFilter::new()
            .from(3)
            .to(1)
            .action(RaftFilterAction::Drop),
    );

    let mut committed_ids = Vec::new();
    for index in 0..LOAD_COMMANDS / 2 {
        let node_id = format!("load-before-change-{index:02}");
        cluster.join_member(1, &node_id).await.unwrap();
        committed_ids.push(format!("member-upsert:{node_id}:1"));
    }

    cluster.propose_remove_voter(1, 3).unwrap();
    assert_eq!(voters(&cluster, 1), BTreeSet::from([1, 2]));
    assert_eq!(voters(&cluster, 2), BTreeSet::from([1, 2]));

    for index in LOAD_COMMANDS / 2..LOAD_COMMANDS {
        let node_id = format!("load-after-change-{index:02}");
        cluster.join_member(1, &node_id).await.unwrap();
        committed_ids.push(format!("member-upsert:{node_id}:1"));
    }

    cluster.filters().recover();
    cluster.tick_all(12);

    let authoritative = command_ids(&cluster, 1);
    assert_eq!(authoritative, command_ids(&cluster, 2));
    for command_id in committed_ids {
        assert!(
            authoritative.contains(&command_id),
            "majority lost committed metadata command {command_id} across voter removal"
        );
    }
}

#[tokio::test]
async fn stable_command_id_retry_storm_is_idempotent_across_membership_change() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.join_member(1, "stable-retry").await.unwrap();
    let stable_id = "member-upsert:stable-retry:1";

    cluster.filters().add_filter(
        RaftPacketFilter::new()
            .from(3)
            .to(1)
            .action(RaftFilterAction::Drop),
    );
    cluster.propose_remove_voter(1, 3).unwrap();

    for _ in 0..32 {
        cluster.join_member(1, "stable-retry").await.unwrap();
    }
    cluster.filters().recover();
    cluster.tick_all(8);

    for node_id in [1, 2] {
        let occurrences = command_ids(&cluster, node_id)
            .into_iter()
            .filter(|command_id| command_id == stable_id)
            .count();
        assert_eq!(
            occurrences, 1,
            "stable command id was materialized more than once on voter {node_id}"
        );
    }
    assert_eq!(
        cluster.node(1).snapshot().duplicate_commands,
        32,
        "every stable-id retry should be coalesced without another committed command"
    );
}

#[tokio::test]
async fn minority_side_never_reports_an_authoritative_committed_membership() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.join_member(1, "minority-baseline").await.unwrap();

    cluster.filters().isolate(3, [1, 2]);
    for _ in 0..12 {
        cluster.tick_node(3);
    }
    assert_ne!(
        cluster.node(3).snapshot().role,
        RaftRuntimeRole::Leader,
        "pre-vote must keep the isolated minority from claiming authority"
    );

    cluster
        .join_member(1, "majority-only-committed")
        .await
        .unwrap();
    let committed_id = "member-upsert:majority-only-committed:1";
    assert!(cluster.node(1).command_applied(committed_id));
    assert!(cluster.node(2).command_applied(committed_id));
    assert!(
        !cluster.node(3).command_applied(committed_id),
        "isolated minority fixture must actually be stale before the authority assertion"
    );
    assert_ne!(
        cluster.node(3).snapshot().role,
        RaftRuntimeRole::Leader,
        "a stale minority membership view must never be reported as authoritative"
    );

    cluster.filters().recover();
    cluster.tick_all(12);
    assert!(
        cluster.node(3).command_applied(committed_id),
        "healed voter must catch up to the committed majority history"
    );
}

#[tokio::test]
async fn canary_membership_load_double_applies_a_stable_command_id() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.join_member(1, "canary-stable").await.unwrap();
    cluster.propose_remove_voter(1, 3).unwrap();
    cluster.join_member(1, "canary-stable").await.unwrap();

    let stable_id = "member-upsert:canary-stable:1";
    let committed_occurrences = command_ids(&cluster, 1)
        .into_iter()
        .filter(|command_id| command_id == stable_id)
        .count();
    let observed_applies = committed_occurrences
        + usize::from(std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W3"));

    assert_eq!(
        observed_applies, 1,
        "HC-CANARY-RED:W3 stable command id was applied twice across membership change"
    );
}

fn command_ids(cluster: &RuntimeRaftCluster, node_id: u64) -> BTreeSet<String> {
    cluster
        .node(node_id)
        .command_envelopes()
        .into_iter()
        .map(|envelope| envelope.command_id)
        .collect()
}

fn voters(cluster: &RuntimeRaftCluster, node_id: u64) -> BTreeSet<u64> {
    cluster
        .node(node_id)
        .voter_ids()
        .unwrap()
        .into_iter()
        .collect()
}
