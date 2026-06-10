use std::sync::Arc;
use std::time::Duration;

use hydracache_core::{CacheEventOptions, CacheEventOrigin, CacheOptions};
use tokio::time::{sleep, timeout};

use crate::tests::common::{user, User};
use crate::{
    ClusterDiscoveryEvent, ClusterGeneration, ClusterRole, HydraCache, InMemoryCluster,
    InMemoryClusterDiscovery,
};

async fn wait_until_absent(cache: &HydraCache, key: &str) {
    timeout(Duration::from_secs(2), async {
        loop {
            if cache.get::<User>(key).await.unwrap().is_none() {
                return;
            }

            sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("cache entry should be removed by cluster invalidation");
}

#[tokio::test]
async fn local_cache_has_no_cluster_diagnostics() {
    let cache = HydraCache::local().build();

    assert!(cache.cluster_diagnostics().is_none());
}

#[tokio::test]
async fn member_and_client_builders_connect_to_shared_cluster() {
    let cluster = Arc::new(InMemoryCluster::new("orders-prod"));
    let discovery = Arc::new(InMemoryClusterDiscovery::new());

    let member = HydraCache::member()
        .cluster("orders-prod")
        .shared_cluster(cluster.clone())
        .shared_discovery(discovery.clone())
        .node_id("member-a")
        .generation(ClusterGeneration::new(1))
        .bind("127.0.0.1:7000")
        .diagnostics_endpoint("http://127.0.0.1:3000")
        .start()
        .await
        .unwrap();

    let client = HydraCache::client()
        .cluster("orders-prod")
        .shared_cluster(cluster)
        .shared_discovery(discovery.clone())
        .node_id("client-a")
        .generation(ClusterGeneration::new(4))
        .bootstrap("127.0.0.1:7000")
        .near_cache_capacity(128)
        .connect()
        .await
        .unwrap();

    let member_diag = member.cluster_diagnostics().unwrap();
    assert_eq!(member_diag.cluster_name, "orders-prod");
    assert_eq!(member_diag.role, ClusterRole::Member);
    assert_eq!(member_diag.member_count, 1);
    assert_eq!(member_diag.client_count, 1);
    assert_eq!(member_diag.epoch.value(), 1);

    let client_diag = client.cluster_diagnostics().unwrap();
    assert_eq!(client_diag.role, ClusterRole::Client);
    assert_eq!(client_diag.bootstrap, vec!["127.0.0.1:7000".to_owned()]);
    assert_eq!(client.invalidation_node_id(), "client-a");
    assert!(client_diag.invalidation_subscribers >= 2);

    let discovered = discovery.candidates();
    assert_eq!(discovered.len(), 2);
    assert!(discovered
        .iter()
        .any(|candidate| candidate.node_id.as_str() == "member-a"));
    assert!(discovered
        .iter()
        .any(|candidate| candidate.node_id.as_str() == "client-a"));
    assert_eq!(
        discovery
            .events()
            .iter()
            .filter(|event| matches!(event, ClusterDiscoveryEvent::CandidateSeen(_)))
            .count(),
        2
    );
}

#[tokio::test]
async fn member_invalidation_reaches_client_near_cache() {
    let cluster = Arc::new(InMemoryCluster::new("orders"));

    let member = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .start()
        .await
        .unwrap();
    let client = HydraCache::client()
        .shared_cluster(cluster)
        .node_id("client-a")
        .connect()
        .await
        .unwrap();

    client
        .put("user:42", user(42), CacheOptions::new().tag("user:42"))
        .await
        .unwrap();
    assert!(client.get::<User>("user:42").await.unwrap().is_some());

    let mut client_events =
        client.subscribe(CacheEventOptions::mutations().origin(CacheEventOrigin::DistributedBus));
    member.invalidate_tag("user:42").await.unwrap();

    let event = timeout(Duration::from_secs(2), client_events.recv())
        .await
        .expect("client should observe distributed invalidation")
        .expect("client event stream should remain open");

    assert_eq!(event.origin(), CacheEventOrigin::DistributedBus);
    wait_until_absent(&client, "user:42").await;
    assert_eq!(member.stats().distributed_invalidations_published, 1);
    assert_eq!(client.stats().distributed_invalidations_applied, 1);
}

#[tokio::test]
async fn client_invalidation_reaches_member_cache() {
    let cluster = Arc::new(InMemoryCluster::new("orders"));

    let member = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .start()
        .await
        .unwrap();
    let client = HydraCache::client()
        .shared_cluster(cluster)
        .node_id("client-a")
        .connect()
        .await
        .unwrap();

    member
        .put("user:7", user(7), CacheOptions::new().tag("user:7"))
        .await
        .unwrap();
    assert!(member.get::<User>("user:7").await.unwrap().is_some());

    client.invalidate_key("user:7").await.unwrap();

    wait_until_absent(&member, "user:7").await;
    assert_eq!(client.stats().distributed_invalidations_published, 1);
    assert_eq!(member.stats().distributed_invalidations_applied, 1);
}

#[tokio::test]
async fn cluster_rejects_stale_generation_for_same_node() {
    let cluster = Arc::new(InMemoryCluster::new("orders"));

    HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .generation(ClusterGeneration::new(2))
        .start()
        .await
        .unwrap();

    let error = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .generation(ClusterGeneration::new(1))
        .start()
        .await
        .unwrap_err();

    assert!(error.to_string().contains("stale cluster generation"));
    assert_eq!(cluster.members().len(), 1);
    assert_eq!(cluster.members()[0].generation.value(), 2);
}

#[tokio::test]
async fn client_builder_can_create_isolated_cluster_runtime() {
    let client = HydraCache::client()
        .cluster("isolated")
        .node_id("client-a")
        .bootstrap("127.0.0.1:7000")
        .connect()
        .await
        .unwrap();

    let diagnostics = client.cluster_diagnostics().unwrap();
    assert_eq!(diagnostics.cluster_name, "isolated");
    assert_eq!(diagnostics.role, ClusterRole::Client);
    assert_eq!(diagnostics.member_count, 0);
    assert_eq!(diagnostics.client_count, 1);
    assert_eq!(diagnostics.bootstrap, vec!["127.0.0.1:7000".to_owned()]);
}
