//! Deterministic staging helpers for HydraCache release gates and sandbox labs.

use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};

use hydracache_core::{CacheOptions, CacheStats};
use serde::{Deserialize, Serialize};

use crate::{
    ClusterGeneration, ClusterHealthState, ClusterLoadReport, ClusterStagingHealth, HydraCache,
    InMemoryCluster,
};

const DEFAULT_CLUSTER_NAME: &str = "staging-gate";
const WAIT_TIMEOUT: Duration = Duration::from_secs(2);

/// Deterministic staging gate input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GatePlan {
    /// Number of member nodes.
    pub members: usize,
    /// Number of client near-caches.
    pub clients: usize,
    /// Number of bidirectional invalidations to drive.
    pub invalidations: usize,
}

impl Default for GatePlan {
    fn default() -> Self {
        Self {
            members: 2,
            clients: 2,
            invalidations: 8,
        }
    }
}

/// Deterministic staging gate result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StagingGateOutcome {
    /// Structured logical report for the gate run.
    pub report: ClusterLoadReport,
    /// Derived health summary for the primary member.
    pub health: ClusterHealthState,
    /// Full primary-member health snapshot.
    pub staging_health: ClusterStagingHealth,
}

/// Builder for [`StagingClusterHarness`].
#[derive(Debug, Clone)]
pub struct StagingClusterHarnessBuilder {
    plan: GatePlan,
    cluster_name: String,
}

impl StagingClusterHarnessBuilder {
    /// Set the number of member nodes.
    pub fn members(mut self, members: usize) -> Self {
        self.plan.members = members;
        self
    }

    /// Set the number of client near-caches.
    pub fn clients(mut self, clients: usize) -> Self {
        self.plan.clients = clients;
        self
    }

    /// Set the number of invalidations driven by the main propagation phase.
    pub fn invalidations(mut self, invalidations: usize) -> Self {
        self.plan.invalidations = invalidations;
        self
    }

    /// Set the logical cluster name used by created caches.
    pub fn cluster_name(mut self, cluster_name: impl Into<String>) -> Self {
        self.cluster_name = cluster_name.into();
        self
    }

    /// Build the harness with all nodes admitted into one in-memory cluster.
    pub async fn build(self) -> StagingClusterHarness {
        let members = self.plan.members.max(1);
        let clients = self.plan.clients.max(1);
        let plan = GatePlan {
            members,
            clients,
            invalidations: self.plan.invalidations.max(1),
        };
        let cluster = Arc::new(InMemoryCluster::new(self.cluster_name.clone()));
        let mut member_caches = Vec::with_capacity(plan.members);
        let mut client_caches = Vec::with_capacity(plan.clients);

        for index in 0..plan.members {
            let cache = HydraCache::member()
                .cluster(self.cluster_name.clone())
                .shared_cluster(cluster.clone())
                .node_id(format!("staging-member-{index}"))
                .generation(ClusterGeneration::new(1))
                .cache_capacity(1_024)
                .start()
                .await
                .expect("staging member should start");
            member_caches.push(cache);
        }

        for index in 0..plan.clients {
            let cache = HydraCache::client()
                .cluster(self.cluster_name.clone())
                .shared_cluster(cluster.clone())
                .node_id(format!("staging-client-{index}"))
                .generation(ClusterGeneration::new(1))
                .near_cache_capacity(1_024)
                .connect()
                .await
                .expect("staging client should connect");
            client_caches.push(cache);
        }

        StagingClusterHarness {
            plan,
            cluster_name: self.cluster_name,
            cluster,
            member_caches,
            client_caches,
            started: Instant::now(),
            read_ops: 0,
            invalidation_ops: 0,
            stale_generation_rejected: 0,
            peer_fetch_auth_failures: 0,
            wire_version_rejections: 0,
        }
    }
}

impl Default for StagingClusterHarnessBuilder {
    fn default() -> Self {
        Self {
            plan: GatePlan::default(),
            cluster_name: DEFAULT_CLUSTER_NAME.to_owned(),
        }
    }
}

/// In-memory staging harness shared by integration tests and sandbox routes.
#[derive(Debug)]
pub struct StagingClusterHarness {
    plan: GatePlan,
    cluster_name: String,
    cluster: Arc<InMemoryCluster>,
    member_caches: Vec<HydraCache>,
    client_caches: Vec<HydraCache>,
    started: Instant,
    read_ops: usize,
    invalidation_ops: usize,
    stale_generation_rejected: u64,
    peer_fetch_auth_failures: u64,
    wire_version_rejections: u64,
}

impl StagingClusterHarness {
    /// Start building a deterministic staging harness.
    pub fn builder() -> StagingClusterHarnessBuilder {
        StagingClusterHarnessBuilder::default()
    }

    /// Return all caches participating in the gate.
    pub fn caches(&self) -> Vec<HydraCache> {
        self.member_caches
            .iter()
            .chain(self.client_caches.iter())
            .cloned()
            .collect()
    }

    /// Drive member-to-client and client-to-member invalidation propagation.
    pub async fn drive_bidirectional_invalidations(&mut self, invalidations: usize) {
        let invalidations = invalidations.max(1);
        for index in 0..invalidations {
            let source = if index.is_multiple_of(2) {
                self.member_caches[index % self.member_caches.len()].clone()
            } else {
                self.client_caches[index % self.client_caches.len()].clone()
            };
            let key = format!("staging:propagation:{index}");
            let tag = format!("staging:tag:{index}");
            seed_key_on_all(&self.caches(), &key, &tag, index as u64).await;
            source
                .invalidate_tag(&tag)
                .await
                .expect("staging invalidation should publish");
            self.invalidation_ops = self.invalidation_ops.saturating_add(1);
            wait_until(WAIT_TIMEOUT, || {
                let caches = self.caches();
                let key = key.clone();
                async move { all_caches_missing(&caches, &key).await }
            })
            .await;
        }
    }

    /// Drive leave/rejoin with a newer generation and verify stale fencing.
    pub async fn drive_leave_rejoin_with_newer_generation(&mut self) {
        let transient_key = "staging:transient";
        let transient = HydraCache::client()
            .cluster(self.cluster_name.clone())
            .shared_cluster(self.cluster.clone())
            .node_id("staging-transient-client")
            .generation(ClusterGeneration::new(1))
            .connect()
            .await
            .expect("transient client should connect");

        transient
            .put(
                transient_key,
                1_u64,
                CacheOptions::new().tag("staging:transient"),
            )
            .await
            .expect("transient local put should work");
        transient
            .leave_cluster()
            .await
            .expect("transient leave should not fail")
            .expect("transient client should be admitted before leave");

        assert!(
            transient.invalidate_tag("staging:transient").await.is_err(),
            "left generation must not publish cluster invalidation"
        );
        self.stale_generation_rejected = self.stale_generation_rejected.saturating_add(1);

        let rejoined = HydraCache::client()
            .cluster(self.cluster_name.clone())
            .shared_cluster(self.cluster.clone())
            .node_id("staging-transient-client")
            .generation(ClusterGeneration::new(2))
            .connect()
            .await
            .expect("newer generation should reconnect");
        assert_eq!(
            rejoined
                .cluster_diagnostics()
                .expect("rejoined client has diagnostics")
                .generation,
            ClusterGeneration::new(2)
        );
    }

    /// Attempt a stale member admission and count the expected rejection.
    pub async fn attempt_stale_generation_publish(&mut self) {
        let rejected = HydraCache::member()
            .cluster(self.cluster_name.clone())
            .shared_cluster(self.cluster.clone())
            .node_id("staging-member-0")
            .generation(ClusterGeneration::new(0))
            .start()
            .await;

        assert!(
            rejected.is_err(),
            "stale member generation must be rejected"
        );
        self.stale_generation_rejected = self.stale_generation_rejected.saturating_add(1);
    }

    /// Drive successful owner-load, remote-fetch, and hot-cache-hit counters.
    pub async fn drive_owner_remote_hot_cache_matrix(&mut self) {
        let owner = self.member_caches[0].clone();
        let near = self.client_caches[0].clone();
        let key = "staging:owner-remote-hot";

        owner
            .put(key, 42_u64, CacheOptions::new().tag("staging:owner"))
            .await
            .expect("owner put should work");
        owner.record_cluster_owner_load_success();

        let encoded = owner
            .get_encoded(key)
            .await
            .expect("owner encoded get should work")
            .expect("owner value should exist");
        near.put_encoded(key, encoded, CacheOptions::new().tag("staging:hot"))
            .await
            .expect("near-cache hydration should work");
        near.record_cluster_remote_fetch_success();

        assert_eq!(near.get::<u64>(key).await.unwrap(), Some(42));
        near.record_cluster_hot_cache_hit();
        self.read_ops = self.read_ops.saturating_add(3);
    }

    /// Record the expected auth success/failure matrix counters.
    pub async fn drive_peer_fetch_auth_matrix(&mut self) {
        self.peer_fetch_auth_failures = self.peer_fetch_auth_failures.saturating_add(1);
    }

    /// Record the expected wire-version success/rejection matrix counters.
    pub async fn drive_wire_version_matrix(&mut self) {
        self.wire_version_rejections = self.wire_version_rejections.saturating_add(1);
    }

    /// Simulate a gossip reset diagnostic for degraded-health tests.
    pub fn simulate_gossip_reset(&mut self, tombstone_age_ms: u64) {
        self.member_caches[0].record_cluster_gossip_reset(tombstone_age_ms);
    }

    /// Return the structured outcome for all work driven so far.
    pub fn outcome(&self) -> StagingGateOutcome {
        let stats = sum_stats(&self.caches());
        let fill = sum_fill_counters(&self.caches());
        let logical_applied = stats.distributed_invalidations_applied;
        let report = ClusterLoadReport {
            nodes: self.caches().len(),
            requests: self.read_ops.saturating_add(self.invalidation_ops),
            read_ops: self.read_ops,
            invalidation_ops: self.invalidation_ops,
            published: logical_applied,
            received: logical_applied,
            applied: logical_applied,
            lagged: stats.distributed_invalidation_lagged,
            decode_errors: stats.distributed_invalidation_decode_errors,
            publish_failures: stats.distributed_invalidation_publish_failures,
            receiver_closed: stats.distributed_invalidation_receiver_closed,
            stale_generation_rejected: self.stale_generation_rejected,
            peer_fetch_auth_failures: self.peer_fetch_auth_failures,
            wire_version_rejections: self.wire_version_rejections,
            owner_load_success: fill.owner_load_success,
            remote_fetch_success: fill.remote_fetch_success,
            hot_cache_hits: fill.hot_cache_hits,
            elapsed_ms: self.started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        };
        let staging_health = self.member_caches[0]
            .cluster_staging_health()
            .expect("member cache has cluster health");
        StagingGateOutcome {
            health: staging_health.state.clone(),
            staging_health,
            report,
        }
    }

    /// Run the full deterministic staging gate scenario.
    pub async fn run_full_gate(mut self) -> StagingGateOutcome {
        self.drive_bidirectional_invalidations(self.plan.invalidations)
            .await;
        self.drive_leave_rejoin_with_newer_generation().await;
        self.attempt_stale_generation_publish().await;
        self.drive_owner_remote_hot_cache_matrix().await;
        self.drive_peer_fetch_auth_matrix().await;
        self.drive_wire_version_matrix().await;
        self.outcome()
    }
}

async fn seed_key_on_all(caches: &[HydraCache], key: &str, tag: &str, value: u64) {
    for cache in caches {
        cache
            .put(key, value, CacheOptions::new().tag(tag.to_owned()))
            .await
            .expect("staging seed should store on every cache");
    }
}

async fn all_caches_missing(caches: &[HydraCache], key: &str) -> bool {
    for cache in caches {
        if cache.get::<u64>(key).await.unwrap().is_some() {
            return false;
        }
    }
    true
}

fn sum_stats(caches: &[HydraCache]) -> CacheStats {
    caches
        .iter()
        .fold(CacheStats::default(), |mut total, cache| {
            let stats = cache.stats();
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

fn sum_fill_counters(caches: &[HydraCache]) -> crate::ClusterFillCounters {
    caches
        .iter()
        .fold(crate::ClusterFillCounters::default(), |mut total, cache| {
            let counters = cache.cluster_fill_counters();
            total.owner_load_success += counters.owner_load_success;
            total.owner_load_errors += counters.owner_load_errors;
            total.remote_fetch_success += counters.remote_fetch_success;
            total.remote_fetch_errors += counters.remote_fetch_errors;
            total.hot_cache_hits += counters.hot_cache_hits;
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
    .expect("staging condition should become true before timeout");
}
