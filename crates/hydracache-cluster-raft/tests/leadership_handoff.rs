use std::collections::BTreeMap;

use hydracache_cluster_raft::RaftWireMessage;
use hydracache_cluster_testkit::invariants::{
    assert_cluster_invariants, cluster_invariant_violations, ClusterInvariantView,
};
use hydracache_cluster_testkit::{RaftFilterAction, RaftPacketFilter, RuntimeRaftCluster};
use raft::eraftpb::{Message, MessageType};

#[tokio::test]
async fn lagging_or_ineligible_transferee_never_becomes_authoritative() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    let filters = cluster.filters();
    filters.add_filter(
        RaftPacketFilter::new()
            .from(1)
            .to(2)
            .message_type(MessageType::MsgAppend)
            .action(RaftFilterAction::Drop),
    );
    cluster.join_member(1, "handoff-prefix").await.unwrap();
    let leader_commit = cluster.node(1).snapshot().commit_index;
    assert!(cluster.node(2).snapshot().commit_index < leader_commit);

    cluster.request_leadership_transfer(1, 2).unwrap();
    cluster.tick_all(3);
    assert_ne!(cluster.leader_id(), Some(2));
    assert!(cluster.request_leadership_transfer(1, 99).is_err());

    filters.recover();
    cluster.tick_all(20);
    assert!(cluster.node(2).snapshot().commit_index >= leader_commit);
    cluster.request_leadership_transfer(1, 2).unwrap();
    cluster.tick_all(10);
    assert_eq!(cluster.leader_id(), Some(2));
    assert_cluster_invariants(&ClusterInvariantView::from_runtime_raft_cluster(&cluster));
}

#[tokio::test]
async fn leadership_handoff_preserves_committed_prefix_and_exactly_once_proposal_outcome() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.join_member(1, "before-handoff").await.unwrap();
    let prefix = cluster.node(1).export_snapshot();

    cluster.request_leadership_transfer(1, 2).unwrap();
    cluster.tick_all(10);
    let leader = cluster.leader_id().expect("handoff must elect one leader");
    cluster.join_member(leader, "raced-proposal").await.unwrap();
    cluster.tick_all(10);

    for node_id in cluster.node_ids() {
        let snapshot = cluster.node(node_id).export_snapshot();
        for command in &prefix.commands {
            assert_eq!(
                snapshot
                    .commands
                    .iter()
                    .filter(|candidate| candidate.command_id == command.command_id)
                    .count(),
                1,
                "node {node_id} lost or duplicated committed prefix {}",
                command.command_id
            );
        }
        assert_eq!(
            snapshot
                .commands
                .iter()
                .filter(|command| command.command_id == "member-upsert:raced-proposal:1")
                .count(),
            1,
            "raced proposal must be observed exactly once on node {node_id}"
        );
    }
    assert_cluster_invariants(&ClusterInvariantView::from_runtime_raft_cluster(&cluster));
}

#[tokio::test]
async fn old_term_traffic_after_handoff_cannot_regress_committed_metadata() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.join_member(1, "stable-prefix").await.unwrap();
    let old_term = cluster.node(1).snapshot().term;
    cluster.request_leadership_transfer(1, 2).unwrap();
    cluster.tick_all(10);
    let new_leader = cluster.leader_id().unwrap();
    let before = cluster.node(new_leader).snapshot();

    let mut stale = Message::default();
    stale.from = 1;
    stale.to = new_leader;
    stale.term = old_term;
    stale.commit = before.commit_index.saturating_sub(1);
    stale.set_msg_type(MessageType::MsgHeartbeat);
    cluster.drain_until_idle([RaftWireMessage::encode(&stale).unwrap()]);

    let after = cluster.node(new_leader).snapshot();
    assert!(after.term >= before.term);
    assert!(after.commit_index >= before.commit_index);
    assert!(after.applied_index >= before.applied_index);
    assert!(cluster
        .node(new_leader)
        .command_applied("member-upsert:stable-prefix:1"));
    assert_cluster_invariants(&ClusterInvariantView::from_runtime_raft_cluster(&cluster));
}

#[tokio::test]
async fn session_guarantees_survive_leadership_handoff() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.join_member(1, "session-write").await.unwrap();
    let session_watermark = cluster.node(1).snapshot().commit_index;

    cluster.request_leadership_transfer(1, 3).unwrap();
    cluster.tick_all(10);
    let leader = cluster.leader_id().unwrap();
    assert_eq!(leader, 3);
    for node_id in cluster.node_ids() {
        let snapshot = cluster.node(node_id).snapshot();
        assert!(
            snapshot.commit_index >= session_watermark,
            "node {node_id} served below the pre-handoff session watermark"
        );
        assert!(cluster
            .node(node_id)
            .command_applied("member-upsert:session-write:1"));
    }
}

#[test]
fn canary_handoff_allows_lagging_transferee_to_serve_a_regressed_view() {
    let view = ClusterInvariantView {
        leaders_by_term: BTreeMap::from([(9, vec![2])]),
        voter_sets_by_node: BTreeMap::from([
            (1, [1, 2, 3].into_iter().collect()),
            (2, [1, 2, 3].into_iter().collect()),
            (3, [1, 2, 3].into_iter().collect()),
        ]),
        member_sets_by_node: BTreeMap::new(),
        committed_command_ids: ["committed-before-handoff".to_owned()]
            .into_iter()
            .collect(),
        applied_command_ids_by_node: BTreeMap::from([
            (
                1,
                ["committed-before-handoff".to_owned()]
                    .into_iter()
                    .collect(),
            ),
            (2, Default::default()),
            (
                3,
                ["committed-before-handoff".to_owned()]
                    .into_iter()
                    .collect(),
            ),
        ]),
    };
    let violations = cluster_invariant_violations(&view);
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W29") {
        assert!(
            violations.is_empty(),
            "HC-CANARY-RED:W29 leadership handoff served a regressed committed view"
        );
    }
    assert!(violations
        .iter()
        .any(|violation| violation.contains("lost committed")));
}
