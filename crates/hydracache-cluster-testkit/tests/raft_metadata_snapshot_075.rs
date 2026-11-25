use hydracache_cluster_testkit::{RaftFilterAction, RaftPacketFilter, RuntimeRaftCluster};
use raft::eraftpb::MessageType;

#[tokio::test]
async fn compacted_metadata_payload_restores_a_lagging_followers_applied_history() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.filters().isolate(3, cluster.node_ids());
    cluster.join_member(1, "snapshot-prefix").await.unwrap();
    let index = cluster.compact_applied_log_to_snapshot(1).unwrap();
    assert!(!cluster
        .node(3)
        .command_applied("member-upsert:snapshot-prefix:1"));

    cluster.filters().recover();
    cluster.filters().add_filter(
        RaftPacketFilter::new()
            .from(1)
            .to(3)
            .message_type(MessageType::MsgSnapshot)
            .action(RaftFilterAction::Hold),
    );
    cluster.tick_all(8);
    let transfer = cluster
        .filters()
        .held()
        .into_iter()
        .find(|message| {
            message
                .decode()
                .is_ok_and(|decoded| decoded.get_msg_type() == MessageType::MsgSnapshot)
        })
        .expect("compaction must force a real snapshot transfer");
    let decoded = transfer.decode().unwrap();
    let snapshot = decoded.get_snapshot();
    assert_eq!(snapshot.get_metadata().index, index);
    assert!(snapshot.get_data().starts_with(b"HCMETA01"));
    let payload: serde_json::Value = serde_json::from_slice(&snapshot.get_data()[8..]).unwrap();
    assert_eq!(payload["format_version"], 1);
    assert_eq!(payload["source_raft_node_id"], 1);
    assert_eq!(payload["applied_index"], index);
    assert!(!payload["commands"].as_array().unwrap().is_empty());

    cluster.filters().release_held();
    cluster.filters().recover();
    cluster.drain_until_idle([transfer]);
    assert!(cluster
        .node(3)
        .command_applied("member-upsert:snapshot-prefix:1"));
    assert!(cluster.node(3).snapshot().applied_index >= index);
}
