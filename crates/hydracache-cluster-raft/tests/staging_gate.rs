use std::sync::Arc;

use hydracache::{
    ClusterCandidate, ClusterControlPlane, ClusterGeneration, ClusterNodeId, RaftMetadataCommand,
};
use hydracache_cluster_raft::{
    InMemoryRaftMetadataStore, RaftMetadataRuntime, RaftMetadataRuntimeConfig,
};

#[tokio::test]
async fn staging_gate_metadata_store_round_trips_generation() {
    let store = Arc::new(InMemoryRaftMetadataStore::new());
    let runtime = RaftMetadataRuntime::with_config_and_metadata_store(
        RaftMetadataRuntimeConfig::single_node("orders", 1),
        store.clone(),
    )
    .unwrap();

    let first = runtime
        .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(9)))
        .await
        .unwrap();
    assert_eq!(first.generation, ClusterGeneration::new(9));

    runtime
        .leave(&ClusterNodeId::from("member-a"), ClusterGeneration::new(9))
        .await
        .unwrap();

    let rejoined = runtime
        .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(10)))
        .await
        .unwrap();
    assert_eq!(rejoined.generation, ClusterGeneration::new(10));

    let stored = store.snapshot().expect("raft metadata saved");
    assert_eq!(stored.commands.len(), 3);
    assert!(matches!(
        &stored.commands.last().unwrap().command,
        RaftMetadataCommand::MemberUpsert { generation, .. }
            if *generation == ClusterGeneration::new(10)
    ));

    let recovered = RaftMetadataRuntime::with_config_and_metadata_store(
        RaftMetadataRuntimeConfig::single_node("orders", 1),
        store,
    )
    .unwrap();
    assert_eq!(recovered.snapshot().commands_committed, 3);

    let duplicate = recovered
        .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(10)))
        .await
        .unwrap();
    assert_eq!(duplicate.generation, ClusterGeneration::new(10));
    assert_eq!(recovered.snapshot().commands_committed, 3);

    let stale = recovered
        .join_member(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(9)))
        .await;
    assert!(stale.is_err());
    assert_eq!(recovered.snapshot().commands_committed, 3);
}
