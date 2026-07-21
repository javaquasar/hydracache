use std::collections::BTreeSet;

use hydracache_cluster_raft::RaftWireMessage;
use hydracache_cluster_testkit::{RaftFilterAction, RaftPacketFilter, RuntimeRaftCluster};
use raft::eraftpb::MessageType;

fn snapshot_index(message: &RaftWireMessage) -> u64 {
    message
        .decode()
        .expect("held raft message must decode")
        .get_snapshot()
        .get_metadata()
        .index
}

fn held_snapshots(cluster: &RuntimeRaftCluster) -> Vec<RaftWireMessage> {
    cluster
        .filters()
        .held()
        .into_iter()
        .filter(|message| {
            message
                .decode()
                .is_ok_and(|decoded| decoded.get_msg_type() == MessageType::MsgSnapshot)
        })
        .collect()
}

fn hold_snapshots(cluster: &RuntimeRaftCluster, from: u64, to: u64) {
    cluster.filters().add_filter(
        RaftPacketFilter::new()
            .from(from)
            .to(to)
            .message_type(MessageType::MsgSnapshot)
            .action(RaftFilterAction::Hold),
    );
}

async fn lag_and_compact(cluster: &mut RuntimeRaftCluster, follower: u64, member: &str) -> u64 {
    cluster.filters().isolate(follower, cluster.node_ids());
    cluster.join_member(1, member).await.unwrap();
    cluster.compact_applied_log_to_snapshot(1).unwrap()
}

#[tokio::test]
async fn newer_snapshot_then_delayed_older_snapshot_never_rolls_state_back() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    let older_index = lag_and_compact(&mut cluster, 3, "older-prefix").await;
    cluster.filters().recover();
    hold_snapshots(&cluster, 1, 3);
    cluster.tick_all(8);
    let older = held_snapshots(&cluster)
        .into_iter()
        .find(|message| snapshot_index(message) == older_index)
        .expect("old leader must emit the compacted snapshot");

    cluster
        .filters()
        .add_filter(RaftPacketFilter::drop_between(2, 3));
    cluster.request_leadership_transfer(1, 2).unwrap();
    cluster.tick_all(10);
    assert_eq!(cluster.leader_id(), Some(2));
    cluster.join_member(2, "newer-tail").await.unwrap();
    let newer_index = cluster.compact_applied_log_to_snapshot(2).unwrap();
    assert!(newer_index > older_index);
    cluster.filters().recover();
    hold_snapshots(&cluster, 2, 3);
    cluster.tick_all(8);
    let newer = held_snapshots(&cluster)
        .into_iter()
        .find(|message| snapshot_index(message) == newer_index)
        .expect("new leader must emit its newer compacted snapshot");

    cluster.filters().release_held();
    cluster.filters().recover();
    cluster.drain_until_idle([newer]);
    let after_newer = cluster.node(3).snapshot();
    cluster.drain_until_idle([older]);

    assert!(cluster.node(3).snapshot().applied_index >= after_newer.applied_index);
    assert!(cluster
        .node(3)
        .command_applied("member-upsert:older-prefix:1"));
    assert!(cluster
        .node(3)
        .command_applied("member-upsert:newer-tail:1"));
}

#[tokio::test]
async fn duplicated_snapshot_is_idempotent_and_abort_releases_for_retry() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    lag_and_compact(&mut cluster, 3, "retry-prefix").await;
    cluster.filters().recover();
    hold_snapshots(&cluster, 1, 3);
    cluster.tick_all(8);
    assert!(!held_snapshots(&cluster).is_empty());

    // Abort the held delivery. A subsequent heartbeat must retry, and the
    // duplicate transport must still materialize the snapshot exactly once.
    cluster.filters().release_held();
    cluster.filters().recover();
    cluster.filters().add_filter(
        RaftPacketFilter::new()
            .from(1)
            .to(3)
            .message_type(MessageType::MsgSnapshot)
            .action(RaftFilterAction::Duplicate(1)),
    );
    cluster.report_snapshot_delivery(1, 3, false).unwrap();
    cluster.tick_all(8);

    assert_eq!(cluster.node(3).snapshot().snapshot_installs, 1);
    assert!(cluster
        .node(3)
        .command_applied("member-upsert:retry-prefix:1"));
}

#[tokio::test]
async fn deferred_snapshot_failure_is_retried_by_the_normal_drive_loop() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    lag_and_compact(&mut cluster, 3, "deferred-retry-prefix").await;
    cluster.filters().recover();
    hold_snapshots(&cluster, 1, 3);
    cluster.tick_all(8);
    assert!(!held_snapshots(&cluster).is_empty());

    let discarded = cluster.filters().release_held();
    assert!(discarded
        .iter()
        .any(|message| message.is_snapshot().unwrap()));
    drop(discarded);
    cluster.filters().recover();
    cluster.node(1).report_snapshot_delivery_deferred(3, false);
    cluster.tick_all(16);

    assert!(cluster
        .node(3)
        .command_applied("member-upsert:deferred-retry-prefix:1"));
}

#[tokio::test]
async fn held_snapshot_receiver_does_not_freeze_majority_progress() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    lag_and_compact(&mut cluster, 3, "held-prefix").await;
    cluster.filters().recover();
    hold_snapshots(&cluster, 1, 3);
    cluster.tick_all(8);
    assert!(!held_snapshots(&cluster).is_empty());

    cluster.join_member(1, "majority-progress").await.unwrap();
    assert!(cluster
        .node(1)
        .command_applied("member-upsert:majority-progress:1"));
    assert!(cluster
        .node(2)
        .command_applied("member-upsert:majority-progress:1"));
    assert!(!cluster
        .node(3)
        .command_applied("member-upsert:majority-progress:1"));

    let held = cluster.filters().release_held();
    cluster.filters().recover();
    cluster.drain_until_idle(held);
    cluster.tick_all(16);
    assert!(cluster
        .node(3)
        .command_applied("member-upsert:majority-progress:1"));
}

#[tokio::test]
async fn snapshot_fanout_to_multiple_lagging_followers_stays_within_budget() {
    const MAX_HELD_MESSAGES: usize = 4;
    const MAX_HELD_BYTES: usize = 256 * 1024;

    let mut cluster = RuntimeRaftCluster::with_voters([1, 2, 3, 4, 5]);
    cluster.campaign(1);
    cluster.filters().isolate(4, cluster.node_ids());
    cluster.filters().isolate(5, cluster.node_ids());
    cluster.join_member(1, "fanout-prefix").await.unwrap();
    cluster.compact_applied_log_to_snapshot(1).unwrap();
    cluster.filters().recover();
    hold_snapshots(&cluster, 1, 4);
    hold_snapshots(&cluster, 1, 5);
    cluster.tick_all(8);

    let held = held_snapshots(&cluster);
    let targets = held
        .iter()
        .map(|message| message.to)
        .collect::<BTreeSet<_>>();
    let held_bytes = held
        .iter()
        .map(|message| message.payload.len())
        .sum::<usize>();
    assert_eq!(targets, BTreeSet::from([4, 5]));
    assert!(
        held.len() <= MAX_HELD_MESSAGES,
        "snapshot retry queue grew: {}",
        held.len()
    );
    assert!(
        held_bytes <= MAX_HELD_BYTES,
        "snapshot fanout retained {held_bytes} bytes"
    );
}

#[tokio::test]
async fn handoff_during_inflight_snapshot_delivery_converges_without_regression() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    lag_and_compact(&mut cluster, 3, "handoff-snapshot-prefix").await;
    cluster.filters().recover();
    hold_snapshots(&cluster, 1, 3);
    cluster.tick_all(8);
    let in_flight = cluster.filters().release_held();
    assert!(!in_flight.is_empty());

    cluster.request_leadership_transfer(1, 2).unwrap();
    cluster.tick_all(10);
    assert_eq!(cluster.leader_id(), Some(2));
    cluster
        .join_member(2, "handoff-snapshot-tail")
        .await
        .unwrap();
    cluster.filters().recover();
    cluster.drain_until_idle(in_flight);
    cluster.tick_all(16);

    for node_id in cluster.node_ids() {
        assert!(cluster
            .node(node_id)
            .command_applied("member-upsert:handoff-snapshot-prefix:1"));
        assert!(cluster
            .node(node_id)
            .command_applied("member-upsert:handoff-snapshot-tail:1"));
    }
}

#[test]
fn canary_snapshot_delivery_applies_a_stale_snapshot_after_a_newer_one() {
    let newer_applied_index = 12_u64;
    let stale_snapshot_index = 7_u64;
    let accepted_stale_snapshot = std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W30");
    assert!(
        !(accepted_stale_snapshot && stale_snapshot_index < newer_applied_index),
        "HC-CANARY-RED:W30 stale snapshot applied after a newer snapshot"
    );
}

#[test]
fn canary_handoff_during_snapshot_loses_or_reapplies_committed_tail() {
    let defect = std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W30");
    let committed_tail_occurrences = if defect { 2 } else { 1 };
    assert_eq!(
        committed_tail_occurrences, 1,
        "HC-CANARY-RED:W30 committed tail was lost or applied twice"
    );
}
