use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use hydracache::{ClusterCandidate, ClusterControlPlane, ClusterGeneration};
use hydracache_cluster_raft::{RaftMetadataRuntime, RaftRuntimeRole};
use hydracache_cluster_testkit::{RaftFilterAction, RaftPacketFilter, RuntimeRaftCluster};
use tokio::task::JoinHandle;
use tokio::time::timeout;

const LOAD_COMMANDS: usize = 24;

#[tokio::test]
async fn membership_change_under_partition_loses_no_committed_metadata_command() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.join_member(1, "load-baseline").await.unwrap();

    // Isolate the old leader, then elect a leader in the only two-node quorum.
    // Node 1 remains a genuine stale minority leader until the partition heals.
    cluster.filters().isolate(1, [1, 2, 3]);
    cluster.campaign(2);
    assert_quorum_view(&cluster, [2, 3], 2, &BTreeSet::from([1, 2, 3]));
    assert_eq!(cluster.node(1).snapshot().role, RaftRuntimeRole::Leader);

    let before_remove = (0..LOAD_COMMANDS / 3)
        .map(|index| format!("load-before-remove-{index:02}"))
        .collect::<Vec<_>>();
    let leader = cluster.node(2);
    let pending_before_remove = spawn_pending_join_batch(&leader, &before_remove).await;
    let proposal_messages = leader.take_outbound_messages();
    assert!(!proposal_messages.is_empty());
    let remove_messages = leader.propose_remove_voter(1).unwrap();
    cluster.drain_until_idle(proposal_messages.into_iter().chain(remove_messages));
    await_join_batch(pending_before_remove).await;

    assert_quorum_view(&cluster, [2, 3], 2, &BTreeSet::from([2, 3]));
    assert_eq!(
        voters(&cluster, 1),
        BTreeSet::from([1, 2, 3]),
        "isolated old leader must retain a stale pre-transition voter view"
    );

    let before_add = (LOAD_COMMANDS / 3..(LOAD_COMMANDS * 2) / 3)
        .map(|index| format!("load-before-add-{index:02}"))
        .collect::<Vec<_>>();
    let pending_before_add = spawn_pending_join_batch(&leader, &before_add).await;
    let proposal_messages = leader.take_outbound_messages();
    assert!(!proposal_messages.is_empty());
    let add_messages = leader.propose_add_voter(1).unwrap();
    cluster.drain_until_idle(proposal_messages.into_iter().chain(add_messages));
    await_join_batch(pending_before_add).await;

    assert_quorum_view(&cluster, [2, 3], 2, &BTreeSet::from([1, 2, 3]));

    let before_drain = ((LOAD_COMMANDS * 2) / 3..LOAD_COMMANDS)
        .map(|index| format!("load-before-drain-{index:02}"))
        .collect::<Vec<_>>();
    let pending_before_drain = spawn_pending_join_batch(&leader, &before_drain).await;
    let proposal_messages = leader.take_outbound_messages();
    assert!(!proposal_messages.is_empty());
    let drain_messages = cluster.node(3).request_remove_voter(3).unwrap();
    cluster.drain_until_idle(proposal_messages.into_iter().chain(drain_messages));
    await_join_batch(pending_before_drain).await;

    for node_id in [2, 3] {
        assert_eq!(
            voters(&cluster, node_id),
            BTreeSet::from([1, 2]),
            "node {node_id} did not apply follower 3's drain"
        );
        assert_eq!(cluster.node(node_id).leader_id(), Some(2));
    }
    let authoritative = command_ids(&cluster, 2);
    assert_eq!(authoritative, command_ids(&cluster, 3));
    assert_eq!(authoritative.len(), LOAD_COMMANDS + 1);
    for node_id in before_remove.iter().chain(&before_add).chain(&before_drain) {
        let command_id = format!("member-upsert:{node_id}:1");
        assert_eq!(
            occurrences(&authoritative, &command_id),
            1,
            "quorum history must preserve exactly one copy of {command_id}"
        );
    }
    assert!(
        command_ids(&cluster, 1).len() < authoritative.len(),
        "isolated old leader must be measurably stale before healing"
    );

    cluster.filters().recover();
    cluster.tick_all(12);

    for node_id in [1, 2, 3] {
        assert_eq!(voters(&cluster, node_id), BTreeSet::from([1, 2]));
        assert_eq!(
            command_ids(&cluster, node_id),
            authoritative,
            "healed node {node_id} did not converge to the exact quorum command history"
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

    let leader = cluster.node(1);
    let mut retries = Vec::new();
    for _ in 0..32 {
        let leader = leader.clone();
        retries.push(tokio::spawn(async move {
            leader
                .join_member(member("stable-retry"))
                .await
                .expect("stable-id retry should resolve")
        }));
    }
    for retry in retries {
        let retried = retry.await.expect("stable-id retry task should not panic");
        assert_eq!(retried.node_id.as_str(), "stable-retry");
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

    cluster.filters().isolate(1, [1, 2, 3]);
    cluster.campaign(2);
    assert_eq!(
        cluster.node(1).snapshot().role,
        RaftRuntimeRole::Leader,
        "fixture must exercise a stale isolated old leader, not merely a follower"
    );
    assert_quorum_view(&cluster, [2, 3], 2, &BTreeSet::from([1, 2, 3]));

    cluster
        .join_member(2, "majority-only-committed")
        .await
        .unwrap();
    let committed_id = "member-upsert:majority-only-committed:1";
    assert!(cluster.node(2).command_applied(committed_id));
    assert!(cluster.node(3).command_applied(committed_id));
    assert_eq!(command_ids(&cluster, 2), command_ids(&cluster, 3));
    assert!(!cluster.node(1).command_applied(committed_id));
    assert!(
        cluster.node(1).snapshot().commit_index < cluster.node(2).snapshot().commit_index,
        "isolated old leader must not advance to the authoritative quorum commit index"
    );

    cluster.filters().recover();
    cluster.tick_all(12);
    assert!(
        cluster.node(1).command_applied(committed_id),
        "healed old leader must catch up to the committed majority history"
    );
    assert_eq!(cluster.node(1).snapshot().role, RaftRuntimeRole::Follower);
    assert_eq!(cluster.node(1).leader_id(), Some(2));
    assert_eq!(command_ids(&cluster, 1), command_ids(&cluster, 2));
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

fn member(node_id: impl Into<String>) -> ClusterCandidate {
    ClusterCandidate::member(node_id.into()).generation(ClusterGeneration::new(1))
}

async fn spawn_pending_join_batch(
    leader: &Arc<RaftMetadataRuntime>,
    node_ids: &[String],
) -> Vec<JoinHandle<()>> {
    let mut joins = Vec::with_capacity(node_ids.len());
    for node_id in node_ids {
        let candidate = node_id.clone();
        let command_id = format!("member-upsert:{candidate}:1");
        let runtime = leader.clone();
        let join = tokio::spawn(async move {
            let joined = runtime
                .join_member(member(candidate.clone()))
                .await
                .unwrap_or_else(|error| panic!("pending join {candidate} failed: {error}"));
            assert_eq!(joined.node_id.as_str(), candidate);
        });
        timeout(Duration::from_secs(2), async {
            loop {
                if leader
                    .snapshot()
                    .last_result
                    .as_ref()
                    .is_some_and(|result| result.command_id == command_id)
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("join {node_id} was never proposed"));
        assert!(
            !join.is_finished(),
            "join {node_id} completed before the network was drained"
        );
        joins.push(join);
    }
    assert!(
        joins.iter().all(|join| !join.is_finished()),
        "membership load must overlap pending proposals"
    );
    joins
}

async fn await_join_batch(joins: Vec<JoinHandle<()>>) {
    for join in joins {
        join.await.expect("pending join task should not panic");
    }
}

fn assert_quorum_view<const N: usize>(
    cluster: &RuntimeRaftCluster,
    quorum_nodes: [u64; N],
    leader_id: u64,
    expected_voters: &BTreeSet<u64>,
) {
    let quorum = quorum_nodes.into_iter().collect::<BTreeSet<_>>();
    assert!(
        quorum.is_subset(expected_voters) && quorum.len() > expected_voters.len() / 2,
        "asserted nodes {quorum:?} are not a quorum of voters {expected_voters:?}"
    );
    let leader_term = cluster.node(leader_id).snapshot().term;
    for node_id in quorum {
        let snapshot = cluster.node(node_id).snapshot();
        assert_eq!(
            cluster.node(node_id).leader_id(),
            Some(leader_id),
            "quorum node {node_id} disagrees on the authoritative leader"
        );
        assert_eq!(
            snapshot.term, leader_term,
            "quorum node {node_id} disagrees on the authoritative term"
        );
        assert_eq!(
            voters(cluster, node_id),
            *expected_voters,
            "quorum node {node_id} disagrees on the voter configuration"
        );
    }
    assert_eq!(
        cluster.node(leader_id).snapshot().role,
        RaftRuntimeRole::Leader
    );
}

fn command_ids(cluster: &RuntimeRaftCluster, node_id: u64) -> Vec<String> {
    cluster
        .node(node_id)
        .command_envelopes()
        .into_iter()
        .map(|envelope| envelope.command_id)
        .collect()
}

fn occurrences(command_ids: &[String], expected: &str) -> usize {
    command_ids
        .iter()
        .filter(|command_id| command_id.as_str() == expected)
        .count()
}

fn voters(cluster: &RuntimeRaftCluster, node_id: u64) -> BTreeSet<u64> {
    cluster
        .node(node_id)
        .voter_ids()
        .unwrap()
        .into_iter()
        .collect()
}
