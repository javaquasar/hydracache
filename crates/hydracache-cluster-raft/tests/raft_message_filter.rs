use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};

use hydracache_cluster_raft::RaftWireMessage;
use hydracache_cluster_testkit::{RaftFilterAction, RaftPacketFilter, RuntimeRaftCluster};
use raft::eraftpb::{Message, MessageType};

fn message_type(message: &RaftWireMessage) -> Option<MessageType> {
    message.decode().ok().map(|decoded| decoded.get_msg_type())
}

fn voter_set(cluster: &RuntimeRaftCluster, node_id: u64) -> BTreeSet<u64> {
    cluster
        .node(node_id)
        .voter_ids()
        .unwrap()
        .into_iter()
        .collect()
}

#[test]
fn prevote_isolated_node_rejoin_does_not_depose_leader() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    let leader_term = cluster.node(1).snapshot().term;

    cluster.filters().isolate(2, [1, 2, 3]);
    for _ in 0..20 {
        cluster.tick_node(2);
    }

    assert_eq!(cluster.leader_id(), Some(1));
    assert_eq!(cluster.node(1).snapshot().term, leader_term);

    cluster.filters().recover();
    cluster.tick_all(5);

    assert_eq!(cluster.leader_id(), Some(1));
    assert_eq!(cluster.node(1).snapshot().term, leader_term);
}

#[test]
fn retired_peer_messages_are_rejected_after_drain_epoch_advances() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    let leader_term = cluster.node(1).snapshot().term;

    cluster.filters().isolate(3, [1, 2, 3]);
    for _ in 0..20 {
        cluster.tick_node(3);
    }
    let stale_from_retired_peer = cluster
        .filters()
        .dropped()
        .into_iter()
        .filter(|message| message.from == 3)
        .collect::<Vec<_>>();

    assert!(
        stale_from_retired_peer.iter().any(|message| matches!(
            message_type(message),
            Some(MessageType::MsgRequestPreVote | MessageType::MsgRequestVote)
        )),
        "isolated peer should have reserved stale election traffic"
    );

    cluster.propose_remove_voter(1, 3).unwrap();
    assert_eq!(voter_set(&cluster, 1), BTreeSet::from([1, 2]));
    assert_eq!(voter_set(&cluster, 2), BTreeSet::from([1, 2]));

    cluster.filters().recover();
    cluster.drain_until_idle(stale_from_retired_peer);
    cluster.tick_all(5);

    assert_eq!(voter_set(&cluster, 1), BTreeSet::from([1, 2]));
    assert_eq!(voter_set(&cluster, 2), BTreeSet::from([1, 2]));
    assert_ne!(
        cluster.leader_id(),
        Some(3),
        "retired peer must not become leader after stale messages replay"
    );
    assert_eq!(
        cluster.node(1).snapshot().term,
        leader_term,
        "stale retired-peer traffic must not inflate the surviving leader term"
    );
}

#[test]
fn leader_promotion_does_not_resurrect_draining_member() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.filters().add_filter(
        RaftPacketFilter::new()
            .from(1)
            .to(3)
            .message_type(MessageType::MsgAppend)
            .action(RaftFilterAction::Delay(50)),
    );

    cluster.propose_remove_voter(1, 3).unwrap();
    assert_eq!(voter_set(&cluster, 1), BTreeSet::from([1, 2]));
    assert_eq!(voter_set(&cluster, 2), BTreeSet::from([1, 2]));

    for _ in 0..10 {
        cluster.tick_node(1);
        cluster.tick_node(2);
    }

    cluster.filters().recover();
    cluster.tick_all(10);

    assert_eq!(voter_set(&cluster, 1), BTreeSet::from([1, 2]));
    assert_eq!(voter_set(&cluster, 2), BTreeSet::from([1, 2]));
    assert_ne!(
        cluster.leader_id(),
        Some(3),
        "delayed drain traffic must not resurrect the removed voter as leader"
    );
}

#[test]
fn mixed_prevote_cluster_still_elects() {
    let mut cluster =
        RuntimeRaftCluster::with_prevote_overrides([1, 2, 3], BTreeMap::from([(2, false)]));

    cluster.campaign(1);

    assert_eq!(cluster.leader_id(), Some(1));
    assert_eq!(cluster.node(2).leader_id(), Some(1));
    assert_eq!(cluster.node(3).leader_id(), Some(1));
}

#[test]
fn asymmetric_partition_leader_keeps_leadership_when_only_one_direction_drops() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    let leader_term = cluster.node(1).snapshot().term;

    cluster
        .filters()
        .add_filter(RaftPacketFilter::drop_between(2, 1));
    for _ in 0..20 {
        cluster.tick_all(1);
    }

    assert_eq!(cluster.leader_id(), Some(1));
    assert_eq!(cluster.node(1).snapshot().term, leader_term);
}

#[test]
fn minority_partition_cannot_commit_but_majority_can() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.filters().isolate(1, [1, 2, 3]);

    cluster.propose_add_voter(1, 4).unwrap();

    assert_eq!(voter_set(&cluster, 1), BTreeSet::from([1, 2, 3]));
    assert_eq!(voter_set(&cluster, 2), BTreeSet::from([1, 2, 3]));
    assert_eq!(voter_set(&cluster, 3), BTreeSet::from([1, 2, 3]));

    cluster.filters().recover();
    cluster.campaign(2);
    cluster.propose_add_voter(2, 4).unwrap();

    assert!(voter_set(&cluster, 2).contains(&4));
    assert!(voter_set(&cluster, 3).contains(&4));
}

#[test]
fn duplicate_confchange_delivery_is_idempotent() {
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

    for node_id in [1, 2, 3] {
        assert_eq!(voter_set(&cluster, node_id), BTreeSet::from([1, 2, 3, 4]));
    }
}

#[tokio::test]
async fn reordered_appends_do_not_corrupt_committed_prefix() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.filters().add_filter(
        RaftPacketFilter::new()
            .from(1)
            .message_type(MessageType::MsgAppend)
            .allow(1)
            .action(RaftFilterAction::Delay(3)),
    );

    cluster.join_member(1, "member-a").await.unwrap();
    cluster.join_member(1, "member-b").await.unwrap();
    cluster.tick_all(5);

    for node_id in [1, 2, 3] {
        assert!(cluster
            .node(node_id)
            .command_applied("member-upsert:member-a:1"));
        assert!(cluster
            .node(node_id)
            .command_applied("member-upsert:member-b:1"));
    }
}

#[test]
fn message_filter_replays_identically_for_same_seed() {
    fn run() -> Vec<hydracache_cluster_testkit::RaftMessageTraceEvent> {
        let mut cluster = RuntimeRaftCluster::three_node();
        cluster.filters().add_filter(
            RaftPacketFilter::new()
                .from(1)
                .message_type(MessageType::MsgAppend)
                .allow(2)
                .action(RaftFilterAction::Delay(2)),
        );
        cluster.campaign(1);
        cluster.propose_add_voter(1, 4).unwrap();
        cluster.filters().trace()
    }

    assert_eq!(run(), run());
}

#[test]
fn inbound_snapshot_message_is_applied_or_rejected_loud() {
    let cluster = RuntimeRaftCluster::three_node();
    let mut message = Message {
        from: 2,
        to: 1,
        term: 1,
        ..Message::default()
    };
    message.set_msg_type(MessageType::MsgSnapshot);
    let wire = RaftWireMessage::encode(&message).unwrap();

    let result = catch_unwind(AssertUnwindSafe(|| cluster.node(1).step(wire)));

    assert!(result.is_ok(), "snapshot step must not panic");
    if let Ok(Err(error)) = result {
        assert!(
            error.to_string().contains("raft") || error.to_string().contains("decode"),
            "snapshot rejection should be loud: {error}"
        );
    }
}
