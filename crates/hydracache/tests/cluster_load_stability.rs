use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use hydracache::{CacheOptions, CacheStats, ClusterGeneration, HydraCache, InMemoryCluster};
use serde::{Deserialize, Serialize};
use tokio::sync::Barrier;

const CLUSTER_NAME: &str = "load-stability";
const GLOBAL_TAG: &str = "cluster-load";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ClusterLoadValue {
    id: u64,
    payload: String,
}

impl ClusterLoadValue {
    fn new(id: u64) -> Self {
        Self {
            id,
            payload: format!("cluster-load-payload-{id:04}"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ClusterLoadConfig {
    members: usize,
    clients: usize,
    requests: u64,
    concurrency: usize,
    unique_keys: u64,
    invalidate_every: u64,
    loader_delay: Duration,
}

impl ClusterLoadConfig {
    fn smoke() -> Self {
        Self {
            members: 2,
            clients: 3,
            requests: 240,
            concurrency: 12,
            unique_keys: 24,
            invalidate_every: 19,
            loader_delay: Duration::from_millis(1),
        }
    }

    fn manual_from_env() -> Self {
        Self {
            members: env_usize("HYDRACACHE_CLUSTER_LOAD_MEMBERS", 3),
            clients: env_usize("HYDRACACHE_CLUSTER_LOAD_CLIENTS", 6),
            requests: env_u64("HYDRACACHE_CLUSTER_LOAD_REQUESTS", 2_000),
            concurrency: env_usize("HYDRACACHE_CLUSTER_LOAD_CONCURRENCY", 48),
            unique_keys: env_u64("HYDRACACHE_CLUSTER_LOAD_UNIQUE_KEYS", 128),
            invalidate_every: env_u64("HYDRACACHE_CLUSTER_LOAD_INVALIDATE_EVERY", 37),
            loader_delay: Duration::from_millis(env_u64(
                "HYDRACACHE_CLUSTER_LOAD_LOADER_DELAY_MS",
                1,
            )),
        }
    }

    fn node_count(self) -> usize {
        self.members + self.clients
    }

    fn validate(self) {
        assert!(self.members > 0, "load test needs at least one member");
        assert!(self.clients > 0, "load test needs at least one client");
        assert!(self.requests > 0, "load test needs at least one request");
        assert!(self.concurrency > 0, "load test needs at least one worker");
        assert!(
            self.unique_keys > 0,
            "load test needs at least one unique key"
        );
        assert!(
            self.invalidate_every > 1,
            "invalidate_every must leave room for read operations"
        );
        assert!(
            self.node_count() > 1,
            "load test needs at least two cluster nodes"
        );
    }
}

#[derive(Debug)]
struct ClusterLoadReport {
    config: ClusterLoadConfig,
    elapsed: Duration,
    read_ops: u64,
    invalidation_ops: u64,
    loader_calls: u64,
    published: u64,
    received: u64,
    applied: u64,
    lagged: u64,
    decode_errors: u64,
    publish_failures: u64,
    receiver_closed: u64,
}

impl ClusterLoadReport {
    fn emit(&self, name: &str) {
        eprintln!(
            "cluster-load {name}: nodes={nodes}, requests={requests}, concurrency={concurrency}, unique_keys={unique_keys}, read_ops={read_ops}, invalidation_ops={invalidation_ops}, loader_calls={loader_calls}, published={published}, received={received}, applied={applied}, health_issues={health_issues}, elapsed_ms={elapsed_ms}, approx_ops_per_sec={ops_per_sec:.0}",
            nodes = self.config.node_count(),
            requests = self.config.requests,
            concurrency = self.config.concurrency,
            unique_keys = self.config.unique_keys,
            read_ops = self.read_ops,
            invalidation_ops = self.invalidation_ops,
            loader_calls = self.loader_calls,
            published = self.published,
            received = self.received,
            applied = self.applied,
            health_issues = self.lagged + self.decode_errors + self.publish_failures + self.receiver_closed,
            elapsed_ms = self.elapsed.as_millis(),
            ops_per_sec = requests_per_second(self.read_ops + self.invalidation_ops, self.elapsed),
        );
    }
}

fn requests_per_second(requests: u64, elapsed: Duration) -> f64 {
    if elapsed.is_zero() {
        requests as f64
    } else {
        requests as f64 / elapsed.as_secs_f64()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn in_memory_cluster_stays_consistent_under_smoke_load() {
    let report = run_cluster_load_workload(ClusterLoadConfig::smoke()).await;

    report.emit("smoke");
    assert_eq!(
        report.read_ops + report.invalidation_ops,
        report.config.requests
    );
    assert!(report.loader_calls > 0);
    assert!(report.loader_calls <= report.read_ops);
    assert!(report.published >= report.invalidation_ops);
    assert!(report.received > 0);
    assert!(report.applied > 0);
    assert_eq!(report.lagged, 0);
    assert_eq!(report.decode_errors, 0);
    assert_eq!(report.publish_failures, 0);
    assert_eq!(report.receiver_closed, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore = "manual cluster load test; run with --ignored --nocapture"]
async fn in_memory_cluster_survives_manual_load_test() {
    let report = run_cluster_load_workload(ClusterLoadConfig::manual_from_env()).await;

    report.emit("manual");
    assert_eq!(
        report.read_ops + report.invalidation_ops,
        report.config.requests
    );
    assert!(report.loader_calls > 0);
    assert!(report.loader_calls <= report.read_ops);
    assert!(report.published >= report.invalidation_ops);
    assert!(report.received >= report.invalidation_ops);
    assert!(report.applied >= report.invalidation_ops);
    assert_eq!(report.lagged, 0);
    assert_eq!(report.decode_errors, 0);
    assert_eq!(report.publish_failures, 0);
    assert_eq!(report.receiver_closed, 0);
}

async fn run_cluster_load_workload(config: ClusterLoadConfig) -> ClusterLoadReport {
    config.validate();

    let cluster = Arc::new(InMemoryCluster::new(CLUSTER_NAME));
    let caches = build_cluster_caches(cluster.clone(), config).await;
    let loader_calls = Arc::new(AtomicU64::new(0));
    let read_ops = Arc::new(AtomicU64::new(0));
    let invalidation_ops = Arc::new(AtomicU64::new(0));
    let started = Instant::now();

    run_mixed_read_invalidation_phase(
        &caches,
        config,
        loader_calls.clone(),
        read_ops.clone(),
        invalidation_ops.clone(),
    )
    .await;

    assert_tag_invalidation_reaches_every_node(&caches).await;
    assert_key_invalidation_reaches_every_node(&caches).await;
    assert_leave_and_rejoin_remain_generation_safe(cluster).await;
    wait_for_bus_activity(&caches, invalidation_ops.load(Ordering::SeqCst)).await;

    let elapsed = started.elapsed();
    let stats = sum_stats(&caches);

    ClusterLoadReport {
        config,
        elapsed,
        read_ops: read_ops.load(Ordering::SeqCst),
        invalidation_ops: invalidation_ops.load(Ordering::SeqCst),
        loader_calls: loader_calls.load(Ordering::SeqCst),
        published: stats.distributed_invalidations_published,
        received: stats.distributed_invalidations_received,
        applied: stats.distributed_invalidations_applied,
        lagged: stats.distributed_invalidation_lagged,
        decode_errors: stats.distributed_invalidation_decode_errors,
        publish_failures: stats.distributed_invalidation_publish_failures,
        receiver_closed: stats.distributed_invalidation_receiver_closed,
    }
}

async fn build_cluster_caches(
    cluster: Arc<InMemoryCluster>,
    config: ClusterLoadConfig,
) -> Vec<HydraCache> {
    let mut caches = Vec::with_capacity(config.node_count());

    for id in 0..config.members {
        let cache = HydraCache::member()
            .cluster(CLUSTER_NAME)
            .shared_cluster(cluster.clone())
            .node_id(format!("load-member-{id}"))
            .generation(ClusterGeneration::new(1))
            .cache_capacity((config.unique_keys * config.node_count() as u64 * 4).max(1_024))
            .start()
            .await
            .unwrap();
        caches.push(cache);
    }

    for id in 0..config.clients {
        let cache = HydraCache::client()
            .cluster(CLUSTER_NAME)
            .shared_cluster(cluster.clone())
            .node_id(format!("load-client-{id}"))
            .generation(ClusterGeneration::new(1))
            .near_cache_capacity((config.unique_keys * config.node_count() as u64 * 4).max(1_024))
            .connect()
            .await
            .unwrap();
        caches.push(cache);
    }

    assert_eq!(cluster.members().len(), config.members);
    assert_eq!(cluster.clients().len(), config.clients);
    wait_until(Duration::from_secs(2), || {
        let caches = caches.clone();
        async move {
            caches
                .iter()
                .all(|cache| cache.cluster_diagnostics().is_some_and(|d| d.connected))
        }
    })
    .await;

    caches
}

async fn run_mixed_read_invalidation_phase(
    caches: &[HydraCache],
    config: ClusterLoadConfig,
    loader_calls: Arc<AtomicU64>,
    read_ops: Arc<AtomicU64>,
    invalidation_ops: Arc<AtomicU64>,
) {
    let workers = config.concurrency.min(config.requests as usize).max(1);
    let barrier = Arc::new(Barrier::new(workers));
    let mut tasks = Vec::with_capacity(workers);

    for worker in 0..workers {
        let caches = caches.to_vec();
        let barrier = barrier.clone();
        let loader_calls = loader_calls.clone();
        let read_ops = read_ops.clone();
        let invalidation_ops = invalidation_ops.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            let mut request = worker as u64;
            while request < config.requests {
                let cache = caches[(request as usize + worker) % caches.len()].clone();
                let key_id = ((request * 31) + worker as u64) % config.unique_keys;
                let key = cluster_key(key_id);
                let key_tag = cluster_key_tag(key_id);

                if request.is_multiple_of(config.invalidate_every) {
                    cache.invalidate_tag(&key_tag).await.unwrap();
                    invalidation_ops.fetch_add(1, Ordering::SeqCst);
                } else {
                    let calls = loader_calls.clone();
                    let delay = config.loader_delay;
                    let value = cache
                        .get_or_insert_with(
                            &key,
                            CacheOptions::new().tags([GLOBAL_TAG.to_owned(), key_tag]),
                            move || async move {
                                calls.fetch_add(1, Ordering::SeqCst);
                                if !delay.is_zero() {
                                    tokio::time::sleep(delay).await;
                                }
                                ClusterLoadValue::new(key_id)
                            },
                        )
                        .await
                        .unwrap();
                    assert_eq!(value.id, key_id);
                    read_ops.fetch_add(1, Ordering::SeqCst);
                }

                request += workers as u64;
            }
        }));
    }

    for task in tasks {
        task.await.unwrap();
    }
}

async fn assert_tag_invalidation_reaches_every_node(caches: &[HydraCache]) {
    let key = "cluster-load:propagation:tag";
    let tag = "cluster-load:propagation:tag";
    seed_key_on_all(caches, key, tag, ClusterLoadValue::new(10_001)).await;

    caches[0].invalidate_tag(tag).await.unwrap();
    wait_until(Duration::from_secs(2), || {
        let caches = caches.to_vec();
        async move { all_caches_missing(&caches, key).await }
    })
    .await;
}

async fn assert_key_invalidation_reaches_every_node(caches: &[HydraCache]) {
    let key = "cluster-load:propagation:key";
    let tag = "cluster-load:propagation:key";
    seed_key_on_all(caches, key, tag, ClusterLoadValue::new(10_002)).await;

    caches[0].invalidate_key(key).await.unwrap();
    wait_until(Duration::from_secs(2), || {
        let caches = caches.to_vec();
        async move { all_caches_missing(&caches, key).await }
    })
    .await;
}

async fn assert_leave_and_rejoin_remain_generation_safe(cluster: Arc<InMemoryCluster>) {
    let retained_key = "cluster-load:retained-after-leave";
    let transient = HydraCache::client()
        .cluster(CLUSTER_NAME)
        .shared_cluster(cluster.clone())
        .node_id("load-transient-client")
        .generation(ClusterGeneration::new(1))
        .connect()
        .await
        .unwrap();

    transient
        .put(
            retained_key,
            ClusterLoadValue::new(20_001),
            CacheOptions::new().tag("cluster-load:retained"),
        )
        .await
        .unwrap();
    assert!(transient.leave_cluster().await.unwrap().is_some());
    assert_eq!(
        transient
            .get::<ClusterLoadValue>(retained_key)
            .await
            .unwrap(),
        Some(ClusterLoadValue::new(20_001)),
        "leaving the cluster must not flush local near-cache contents",
    );
    assert!(
        transient
            .invalidate_tag("cluster-load:retained")
            .await
            .is_err(),
        "a left generation must not be allowed to publish cluster invalidations",
    );

    let rejoined = HydraCache::client()
        .cluster(CLUSTER_NAME)
        .shared_cluster(cluster)
        .node_id("load-transient-client")
        .generation(ClusterGeneration::new(2))
        .connect()
        .await
        .unwrap();
    let diagnostics = rejoined
        .cluster_diagnostics()
        .expect("rejoined client has cluster diagnostics");
    assert_eq!(diagnostics.generation, ClusterGeneration::new(2));
}

async fn seed_key_on_all(caches: &[HydraCache], key: &str, tag: &str, value: ClusterLoadValue) {
    for cache in caches {
        cache
            .put(
                key,
                value.clone(),
                CacheOptions::new().tags([GLOBAL_TAG.to_owned(), tag.to_owned()]),
            )
            .await
            .unwrap();
    }
}

async fn all_caches_missing(caches: &[HydraCache], key: &str) -> bool {
    for cache in caches {
        if cache.get::<ClusterLoadValue>(key).await.unwrap().is_some() {
            return false;
        }
    }
    true
}

async fn wait_for_bus_activity(caches: &[HydraCache], invalidation_ops: u64) {
    wait_until(Duration::from_secs(2), || {
        let caches = caches.to_vec();
        async move {
            let stats = sum_stats(&caches);
            stats.distributed_invalidations_published >= invalidation_ops + 2
                && stats.distributed_invalidations_received > 0
                && stats.distributed_invalidations_applied > 0
        }
    })
    .await;
}

fn sum_stats(caches: &[HydraCache]) -> CacheStats {
    caches
        .iter()
        .fold(CacheStats::default(), |mut total, cache| {
            let stats = cache.stats();
            total.hits += stats.hits;
            total.misses += stats.misses;
            total.loads += stats.loads;
            total.single_flight_joins += stats.single_flight_joins;
            total.stale_load_discards += stats.stale_load_discards;
            total.invalidations += stats.invalidations;
            total.evictions += stats.evictions;
            total.oversize_rejections += stats.oversize_rejections;
            total.events_published += stats.events_published;
            total.event_subscriber_lagged += stats.event_subscriber_lagged;
            total.distributed_invalidations_published += stats.distributed_invalidations_published;
            total.distributed_invalidations_received += stats.distributed_invalidations_received;
            total.distributed_invalidations_applied += stats.distributed_invalidations_applied;
            total.distributed_invalidation_lagged += stats.distributed_invalidation_lagged;
            total.distributed_invalidation_decode_errors +=
                stats.distributed_invalidation_decode_errors;
            total.distributed_invalidation_publish_failures +=
                stats.distributed_invalidation_publish_failures;
            total.distributed_invalidation_receiver_closed +=
                stats.distributed_invalidation_receiver_closed;
            total
        })
}

async fn wait_until<F, Fut>(timeout_after: Duration, mut condition: F)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    tokio::time::timeout(timeout_after, async {
        loop {
            if condition().await {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("cluster load condition should become true before timeout");
}

fn cluster_key(id: u64) -> String {
    format!("cluster-load:key:{id}")
}

fn cluster_key_tag(id: u64) -> String {
    format!("cluster-load:key-tag:{id}")
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
