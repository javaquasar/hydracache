use std::sync::Arc;
use std::time::Duration;

use hydracache::{
    CacheOptions, ClusterGeneration, ConsistencyOutcome, HydraCache, InMemoryCluster,
    WriteBarrierToken,
};

#[derive(Debug)]
struct LoadError;

impl std::fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("load error")
    }
}

impl std::error::Error for LoadError {}

#[tokio::test]
async fn read_after_write_observes_own_write() {
    let cache = HydraCache::local().build();
    let token = cache.write_barrier_token();

    let outcome = cache
        .read_after_write(
            "user:1",
            token,
            Duration::ZERO,
            CacheOptions::new(),
            || async { Ok::<_, LoadError>("fresh".to_owned()) },
        )
        .await
        .unwrap();

    assert_eq!(outcome, ConsistencyOutcome::Fresh("fresh".to_owned()));
    assert_eq!(
        cache.get::<String>("user:1").await.unwrap(),
        Some("fresh".to_owned())
    );
}

#[tokio::test]
async fn barrier_falls_back_to_peer_fetch_on_timeout() {
    let cache = HydraCache::local().build();
    cache
        .put("user:1", "stale".to_owned(), CacheOptions::new())
        .await
        .unwrap();
    let unsatisfied = WriteBarrierToken::new(ClusterGeneration::default(), 99);

    let outcome = cache
        .read_after_write(
            "user:1",
            unsatisfied,
            Duration::from_millis(1),
            CacheOptions::new(),
            || async { Ok::<_, LoadError>("fresh".to_owned()) },
        )
        .await
        .unwrap();

    assert_eq!(outcome, ConsistencyOutcome::Fresh("fresh".to_owned()));
    assert_eq!(
        cache.get::<String>("user:1").await.unwrap(),
        Some("fresh".to_owned())
    );
    assert_eq!(cache.cluster_fill_counters().remote_fetch_success, 1);
    assert_eq!(cache.cluster_staging_counters().barrier_timeouts, 1);
}

#[tokio::test]
async fn barrier_respects_generation() {
    let cluster = Arc::new(InMemoryCluster::new("barrier"));
    let _member_a = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .generation(ClusterGeneration::new(2))
        .start()
        .await
        .unwrap();
    let cache = HydraCache::client()
        .shared_cluster(cluster)
        .node_id("client-a")
        .generation(ClusterGeneration::new(2))
        .connect()
        .await
        .unwrap();
    let old_generation = WriteBarrierToken::new(ClusterGeneration::new(1), 0);

    let outcome = cache
        .read_after_write(
            "user:1",
            old_generation,
            Duration::ZERO,
            CacheOptions::new(),
            || async { Ok::<_, LoadError>("fresh".to_owned()) },
        )
        .await
        .unwrap();

    assert_eq!(outcome, ConsistencyOutcome::Fresh("fresh".to_owned()));
    assert_eq!(cache.cluster_staging_counters().barrier_timeouts, 1);
    assert_eq!(cache.cluster_fill_counters().remote_fetch_success, 1);
}
