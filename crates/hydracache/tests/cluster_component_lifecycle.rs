use std::sync::Arc;

use hydracache::{
    ClusterGeneration, ClusterHealthReason, ClusterHealthState, HydraCache, InMemoryCluster,
};

#[tokio::test]
async fn component_lifecycle_feeds_staging_health() {
    let cluster = Arc::new(InMemoryCluster::new("orders"));
    let member = HydraCache::member()
        .shared_cluster(cluster)
        .node_id("member-a")
        .generation(ClusterGeneration::new(1))
        .start()
        .await
        .unwrap();

    assert_eq!(
        member.cluster_staging_health().unwrap().state,
        ClusterHealthState::Healthy
    );

    member.leave_cluster().await.unwrap();
    let health = member
        .cluster_staging_health()
        .expect("cluster runtime stays diagnosable after leave");

    assert!(matches!(
        health.state,
        ClusterHealthState::NotReady { ref reasons }
            if reasons.contains(&ClusterHealthReason::LifecycleNotRunning)
    ));
}
