#![cfg(feature = "test-failpoints")]

use hydracache_cluster_testkit::RuntimeRaftCluster;
use raft::eraftpb::MessageType;

#[tokio::test]
async fn rejoined_lagging_runtime_is_caught_up_via_installsnapshot_after_log_compaction() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.filters().isolate(3, [1, 2, 3]);

    cluster.join_member(1, "member-a").await.unwrap();
    cluster.join_member(1, "member-b").await.unwrap();
    assert!(!cluster.node(3).command_applied("member-upsert:member-a:1"));

    let compacted_index = cluster
        .node(1)
        .compact_applied_log_to_snapshot_for_tests()
        .unwrap();
    assert!(compacted_index >= 3);

    cluster.filters().recover();
    cluster.join_member(1, "member-tail").await.unwrap();
    cluster.tick_all(12);

    assert!(
        cluster.delivered().iter().any(|message| {
            message.to == 3
                && message
                    .decode()
                    .is_ok_and(|decoded| decoded.get_msg_type() == MessageType::MsgSnapshot)
        }),
        "lagging follower should be caught up through MsgSnapshot after leader compaction"
    );
    assert!(
        cluster.node(3).snapshot().snapshot_installs > 0,
        "follower should record metadata snapshot install"
    );
    for member in ["member-a", "member-b", "member-tail"] {
        assert!(
            cluster
                .node(3)
                .command_applied(&format!("member-upsert:{member}:1")),
            "rejoined follower missed {member} after snapshot plus tail"
        );
    }
}

#[tokio::test]
async fn rejoin_after_compaction_survives_tail_commit_midway() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    cluster.filters().isolate(3, [1, 2, 3]);

    cluster
        .join_member(1, "member-before-restart")
        .await
        .unwrap();
    cluster
        .node(1)
        .compact_applied_log_to_snapshot_for_tests()
        .unwrap();

    cluster.filters().recover();
    cluster.tick_all(6);
    cluster
        .join_member(1, "member-after-restart")
        .await
        .unwrap();
    cluster.tick_all(12);

    assert!(cluster.delivered().iter().any(|message| {
        message.to == 3
            && message
                .decode()
                .is_ok_and(|decoded| decoded.get_msg_type() == MessageType::MsgSnapshot)
    }));
    assert!(cluster
        .node(3)
        .command_applied("member-upsert:member-before-restart:1"));
    assert!(cluster
        .node(3)
        .command_applied("member-upsert:member-after-restart:1"));
}

#[test]
fn canary_rejoin_serves_stale_local_membership_after_snapshot() {
    let snapshot_installed = true;
    let stale_local_membership_served =
        std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W10");
    assert!(
        !(snapshot_installed && stale_local_membership_served),
        "HC-CANARY-RED:W10 stale membership served after snapshot catch-up"
    );
}
