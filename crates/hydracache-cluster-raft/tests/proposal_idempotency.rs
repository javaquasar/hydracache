use std::collections::BTreeSet;

use hydracache::{ClusterCandidate, ClusterControlPlane, ClusterGeneration};
use hydracache_cluster_raft::RaftMetadataRuntime;
use hydracache_cluster_testkit::{RaftFilterAction, RaftPacketFilter, RuntimeRaftCluster};
use raft::eraftpb::MessageType;

fn voter_set(cluster: &RuntimeRaftCluster, node_id: u64) -> BTreeSet<u64> {
    cluster
        .node(node_id)
        .voter_ids()
        .unwrap()
        .into_iter()
        .collect()
}

fn member(id: &'static str) -> ClusterCandidate {
    ClusterCandidate::member(id).generation(ClusterGeneration::new(1))
}

#[test]
fn retried_confchange_is_not_double_applied_across_snapshot_and_restart() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.filters().add_filter(
        RaftPacketFilter::new()
            .from(1)
            .message_type(MessageType::MsgAppend)
            .allow(1)
            .action(RaftFilterAction::Duplicate(1)),
    );

    cluster.propose_add_voter(1, 4).unwrap();
    let snapshot_index = cluster.save_snapshot_for_node(1).unwrap();
    assert!(
        snapshot_index > 0,
        "restart boundary must persist a real raft snapshot"
    );
    cluster.restart_node(1).unwrap();
    cluster.campaign(1);

    let retry = cluster.propose_add_voter(1, 4);
    assert!(
        retry.is_ok() || retry.unwrap_err().to_string().contains("already exists"),
        "retrying an already-applied ConfChange should commit idempotently or fail loud"
    );

    for node_id in [1, 2, 3] {
        assert_eq!(voter_set(&cluster, node_id), BTreeSet::from([1, 2, 3, 4]));
    }
}

#[tokio::test]
async fn duplicate_reordered_proposal_is_idempotent_after_snapshot() {
    let runtime = RaftMetadataRuntime::single_node("orders", 1).unwrap();
    runtime.join_member(member("member-a")).await.unwrap();

    let restored = RaftMetadataRuntime::from_snapshot(runtime.export_snapshot()).unwrap();
    let before = restored.snapshot();
    let duplicate = restored.join_member(member("member-a")).await.unwrap();
    let after = restored.snapshot();

    assert_eq!(duplicate.node_id.as_str(), "member-a");
    assert_eq!(
        after.commands_committed, before.commands_committed,
        "same command id must not append a second metadata command after restart"
    );
    assert_eq!(after.duplicate_commands, 1);
    assert_eq!(restored.members().len(), 1);

    let restored_again = RaftMetadataRuntime::from_snapshot(restored.export_snapshot()).unwrap();
    assert_eq!(restored_again.members().len(), 1);
    assert_eq!(
        restored_again.snapshot().commands_committed,
        before.commands_committed
    );
}

#[test]
fn canary_confchange_double_applies_on_retry() {
    let duplicated_voter_log = [1, 2, 3, 4, 4];
    let unique_voters = duplicated_voter_log.into_iter().collect::<BTreeSet<_>>();
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W13") {
        assert_eq!(
            duplicated_voter_log.len(),
            unique_voters.len(),
            "HC-CANARY-RED:W13 ConfChange applied twice on retry"
        );
    }
    assert_ne!(
        duplicated_voter_log.len(),
        unique_voters.len(),
        "canary fixture must model the forbidden duplicate-voter outcome"
    );
}
