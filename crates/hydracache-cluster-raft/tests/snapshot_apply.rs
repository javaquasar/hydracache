use hydracache::{
    ClusterCandidate, ClusterControlPlane, ClusterEpoch, ClusterGeneration, ClusterNodeId,
    ClusterRole, RaftMetadataCommand,
};
use hydracache_cluster_raft::{RaftMetadataCommandEnvelope, RaftMetadataRuntime};

fn assert_error_contains(error: impl ToString, expected: &[&str]) {
    let message = error.to_string();
    for needle in expected {
        assert!(
            message.contains(needle),
            "error did not include {needle:?}: {message}"
        );
    }
}

mod snapshot_apply {
    use super::*;

    #[tokio::test]
    async fn membership_tail_apply_error_after_snapshot_is_release_blocking() {
        let runtime = RaftMetadataRuntime::single_node("orders", 1).unwrap();
        runtime
            .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
            .await
            .unwrap();

        let mut snapshot = runtime.export_snapshot();
        snapshot.commands.push(RaftMetadataCommandEnvelope {
            command_id: "node-left:missing-member:2".to_owned(),
            command: RaftMetadataCommand::NodeLeft {
                node_id: ClusterNodeId::from("missing-member"),
                role: ClusterRole::Member,
                epoch: ClusterEpoch::new(2),
            },
        });

        let error = RaftMetadataRuntime::from_snapshot(snapshot).unwrap_err();

        assert_error_contains(
            error,
            &[
                "raft snapshot apply error",
                "snapshot_index=",
                "tail_index=2",
                "command_id=node-left:missing-member:2",
                "absent Member 'missing-member'",
            ],
        );
    }

    #[tokio::test]
    async fn inconsistent_snapshot_membership_indexes_are_rejected_loud() {
        let runtime = RaftMetadataRuntime::single_node("orders", 1).unwrap();
        runtime
            .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
            .await
            .unwrap();
        let mut snapshot = runtime.export_snapshot();
        snapshot.applied_index = 0;

        let error = RaftMetadataRuntime::from_snapshot(snapshot).unwrap_err();

        assert_error_contains(
            error,
            &[
                "raft snapshot apply error",
                "inconsistent snapshot membership indexes",
                "snapshot_index=0",
                "command_count=1",
                "tail_index=1",
                "command_id=member-upsert:member-a:1",
            ],
        );
    }

    #[tokio::test]
    async fn apply_error_trace_includes_snapshot_index_tail_index_and_command_id() {
        let runtime = RaftMetadataRuntime::single_node("orders", 1).unwrap();
        runtime
            .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)))
            .await
            .unwrap();
        let mut snapshot = runtime.export_snapshot();
        let snapshot_index = snapshot.applied_index;
        snapshot.commands[0].command_id = "wrong-command-id".to_owned();

        let error = RaftMetadataRuntime::from_snapshot(snapshot).unwrap_err();

        assert_error_contains(
            error,
            &[
                "raft snapshot apply error",
                &format!("snapshot_index={snapshot_index}"),
                "tail_index=1",
                "command_id=wrong-command-id",
                "expected_command_id=member-upsert:member-a:1",
            ],
        );
    }
}
