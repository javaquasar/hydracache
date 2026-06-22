use hydracache::{ClusterCandidate, ClusterControlPlane, ClusterGeneration, ClusterNodeId};
use hydracache_cluster_raft::{
    DurableRaftLogDirectory, RaftMetadataRuntime, RAFT_LOG_FORMAT_VERSION,
};

#[tokio::test]
async fn durable_runtime_recovers_committed_metadata_after_reopen() {
    let directory = DurableRaftLogDirectory::new();

    let runtime = RaftMetadataRuntime::durable("orders", 1, directory.clone()).unwrap();
    runtime
        .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(7)))
        .await
        .unwrap();
    assert_eq!(runtime.snapshot().commands_committed, 1);
    drop(runtime);

    let reopened = RaftMetadataRuntime::durable("orders", 1, directory).unwrap();
    assert_eq!(reopened.snapshot().commands_committed, 1);
    reopened
        .validate_generation(&ClusterNodeId::from("member-a"), ClusterGeneration::new(7))
        .await
        .unwrap();
}

#[test]
fn durable_runtime_refuses_unknown_future_format() {
    let directory = DurableRaftLogDirectory::new();
    directory.set_format_version_for_tests(RAFT_LOG_FORMAT_VERSION + 1);

    let error = RaftMetadataRuntime::durable("orders", 1, directory).unwrap_err();
    assert!(error.to_string().contains("unknown future raft log format"));
}
