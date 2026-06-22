use std::error::Error;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use futures_util::FutureExt;
use hydracache_core::{
    CacheCodec, CacheDiagnostics, CacheError, CacheEvent, CacheEventKind, CacheEventOptions,
    CacheEventOrigin, CacheOptions, CacheStats, PostcardCodec, Result,
};
use moka::future::Cache;
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::watch;

use crate::builder::HydraCacheBuilder;
use crate::cluster::{
    ClusterCacheCounters, ClusterDiagnostics, ClusterDiscoveryDiagnostics, ClusterFillCounters,
    ClusterMembershipEvent, ClusterMembershipSubscriber, ClusterNodeId,
    ClusterOwnershipDiagnostics, ClusterPilotReadiness, ClusterPilotReport, ClusterRuntime,
    ClusterStagingCounters, ClusterStagingHealth, HydraCacheClientBuilder, HydraCacheMemberBuilder,
    RoutingMode, TopologyFence, TransportPosture,
};
use crate::entry::CacheEntry;
use crate::events::{CacheEventListenerHandle, CacheEventSubscriber, EventBus};
use crate::grid::{ClusterGridCounters, ReplicatedValueSecurityPosture, ReplicationConfig};
use crate::inflight::{InFlightMap, SharedLoadFuture};
use crate::invalidation_bus::{
    CacheInvalidation, CacheInvalidationBus, CacheInvalidationMessage, CacheInvalidationReceive,
};
use crate::refresh::RefreshOptions;
use crate::stats::StatsCounters;
use crate::tag_index::{LoadGenerationSnapshot, TagIndex};
use crate::typed::TypedCache;

/// Local async cache runtime.
///
/// `HydraCache` stores encoded values in a local Moka-backed cache and exposes
/// async helpers for loader-based caching, TTLs, tags, explicit invalidation,
/// local single-flight, and lightweight stats.
///
/// # Example
///
/// ```rust
/// use hydracache::{CacheOptions, HydraCache};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// struct User {
///     id: u64,
/// }
///
/// # #[tokio::main]
/// # async fn main() -> hydracache::CacheResult<()> {
/// let cache = HydraCache::local().build();
///
/// cache.put("user:1", User { id: 1 }, CacheOptions::new()).await?;
/// let cached: Option<User> = cache.get("user:1").await?;
///
/// assert_eq!(cached, Some(User { id: 1 }));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct HydraCache<C = PostcardCodec>
where
    C: CacheCodec,
{
    pub(crate) inner: Arc<HydraCacheInner<C>>,
}

#[derive(Debug)]
pub(crate) struct HydraCacheInner<C>
where
    C: CacheCodec,
{
    pub(crate) store: Cache<String, CacheEntry>,
    pub(crate) tag_index: TagIndex,
    pub(crate) in_flight: InFlightMap,
    pub(crate) codec: C,
    pub(crate) default_ttl: std::time::Duration,
    pub(crate) max_entry_bytes: usize,
    pub(crate) stats: Arc<StatsCounters>,
    pub(crate) events: EventBus,
    pub(crate) invalidation_bus: Option<Arc<dyn CacheInvalidationBus>>,
    pub(crate) invalidation_node_id: String,
    pub(crate) consistency_generation: AtomicU64,
    pub(crate) bus_shutdown: Option<watch::Sender<bool>>,
    pub(crate) cluster_runtime: Option<ClusterRuntime>,
    pub(crate) transport_posture: TransportPosture,
    pub(crate) routing_mode: RoutingMode,
    pub(crate) read_through_enabled: bool,
    pub(crate) replication_config: ReplicationConfig,
    pub(crate) replicated_value_security: ReplicatedValueSecurityPosture,
}

impl<C> Drop for HydraCacheInner<C>
where
    C: CacheCodec,
{
    fn drop(&mut self) {
        if let Some(shutdown) = &self.bus_shutdown {
            let _ = shutdown.send(true);
        }
    }
}

impl HydraCache<PostcardCodec> {
    /// Start building a local cache.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::HydraCache;
    ///
    /// let cache = HydraCache::local().build();
    /// ```
    pub fn local() -> HydraCacheBuilder<PostcardCodec> {
        HydraCacheBuilder::default()
    }

    /// Start building a client near-cache connected to a HydraCache cluster.
    ///
    /// v0.20 provides an in-process cluster model for tests and demos. Network
    /// discovery and Raft-backed membership are intentionally future adapters.
    pub fn client() -> HydraCacheClientBuilder<PostcardCodec> {
        HydraCacheClientBuilder::default()
    }

    /// Start building an in-process cluster member.
    ///
    /// Members participate in the in-memory invalidation bus today and provide
    /// the API shape for future chitchat/Raft-backed cluster runtimes.
    pub fn member() -> HydraCacheMemberBuilder<PostcardCodec> {
        HydraCacheMemberBuilder::default()
    }
}

impl<C> HydraCache<C>
where
    C: CacheCodec,
{
    /// Create a typed, namespaced view over this cache.
    ///
    /// The typed view prefixes physical keys as `namespace:key` while sharing
    /// the same storage, stats, single-flight map, tags, and invalidation
    /// generations.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::{CacheOptions, HydraCache};
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    /// struct User {
    ///     id: u64,
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() -> hydracache::CacheResult<()> {
    /// let cache = HydraCache::local().build();
    /// let users = cache.typed::<User>("users");
    ///
    /// users.put("1", User { id: 1 }, CacheOptions::new()).await?;
    /// assert_eq!(users.get("1").await?, Some(User { id: 1 }));
    /// # Ok(())
    /// # }
    /// ```
    pub fn typed<T>(&self, namespace: impl Into<String>) -> TypedCache<T, C> {
        TypedCache::new(self.clone(), namespace.into())
    }

    /// Return this cache instance's invalidation node id.
    ///
    /// The id is included in bus messages and lets each cache ignore messages it
    /// originally published.
    pub fn invalidation_node_id(&self) -> &str {
        &self.inner.invalidation_node_id
    }

    /// Return cluster diagnostics when this cache was built as a client or member.
    ///
    /// Local caches return `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use hydracache::{HydraCache, InMemoryCluster};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> hydracache::CacheResult<()> {
    /// let cluster = Arc::new(InMemoryCluster::new("orders"));
    /// let client = HydraCache::client()
    ///     .shared_cluster(cluster)
    ///     .node_id("client-a")
    ///     .connect()
    ///     .await?;
    ///
    /// let diagnostics = client.cluster_diagnostics().expect("cluster runtime");
    /// assert_eq!(diagnostics.client_count, 1);
    /// assert!(diagnostics.lifecycle.is_running());
    ///
    /// client.leave_cluster().await?;
    /// assert!(client.cluster_diagnostics().unwrap().lifecycle.is_stopped());
    /// # Ok(())
    /// # }
    /// ```
    pub fn cluster_diagnostics(&self) -> Option<ClusterDiagnostics> {
        self.inner
            .cluster_runtime
            .as_ref()
            .map(ClusterRuntime::diagnostics)
    }

    /// Return ownership diagnostics when this cache was built as a client or member.
    ///
    /// Local caches return `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use hydracache::{HydraCache, InMemoryCluster};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> hydracache::CacheResult<()> {
    /// let cluster = Arc::new(InMemoryCluster::new("orders"));
    /// let member = HydraCache::member()
    ///     .shared_cluster(cluster.clone())
    ///     .node_id("member-a")
    ///     .start()
    ///     .await?;
    ///
    /// let _owner = cluster.owner_for_key("user:42");
    /// let ownership = member
    ///     .cluster_ownership_diagnostics()
    ///     .expect("cluster runtime");
    ///
    /// assert_eq!(ownership.resolver, "rendezvous");
    /// assert_eq!(ownership.resolutions, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn cluster_ownership_diagnostics(&self) -> Option<ClusterOwnershipDiagnostics> {
        self.inner
            .cluster_runtime
            .as_ref()
            .map(ClusterRuntime::ownership_diagnostics)
    }

    /// Return discovery diagnostics when this cache was built with discovery.
    ///
    /// Local caches and client/member caches without a discovery adapter return
    /// `None`.
    pub fn cluster_discovery_diagnostics(&self) -> Option<ClusterDiscoveryDiagnostics> {
        self.inner
            .cluster_runtime
            .as_ref()
            .and_then(ClusterRuntime::discovery_diagnostics)
    }

    /// Return owner-load, remote-fetch, and hot-cache hit counters.
    pub fn cluster_fill_counters(&self) -> ClusterFillCounters {
        ClusterFillCounters {
            owner_load_success: self
                .inner
                .stats
                .cluster_owner_load_success
                .load(Ordering::Relaxed),
            owner_load_errors: self
                .inner
                .stats
                .cluster_owner_load_errors
                .load(Ordering::Relaxed),
            remote_fetch_success: self
                .inner
                .stats
                .cluster_remote_fetch_success
                .load(Ordering::Relaxed),
            remote_fetch_errors: self
                .inner
                .stats
                .cluster_remote_fetch_errors
                .load(Ordering::Relaxed),
            hot_cache_hits: self
                .inner
                .stats
                .cluster_hot_cache_hits
                .load(Ordering::Relaxed),
        }
    }

    /// Return cluster staging counters that are not part of local cache stats.
    pub fn cluster_staging_counters(&self) -> ClusterStagingCounters {
        ClusterStagingCounters {
            peer_fetch_auth_failures: self
                .inner
                .stats
                .cluster_peer_fetch_auth_failures
                .load(Ordering::Relaxed),
            wire_version_rejections: self
                .inner
                .stats
                .cluster_wire_version_rejections
                .load(Ordering::Relaxed),
            stale_generation_rejected: self
                .inner
                .stats
                .cluster_stale_generation_rejected
                .load(Ordering::Relaxed),
            tombstone_age_ms: self
                .inner
                .stats
                .cluster_gossip_tombstone_age_ms
                .load(Ordering::Relaxed),
            gossip_reset_count: self
                .inner
                .stats
                .cluster_gossip_reset_count
                .load(Ordering::Relaxed),
            barrier_timeouts: self
                .inner
                .stats
                .cluster_barrier_timeouts
                .load(Ordering::Relaxed),
            near_cache_conservative_invalidations: self
                .inner
                .stats
                .cluster_near_cache_conservative_invalidations
                .load(Ordering::Relaxed),
            lifecycle_stop_count: self
                .inner
                .stats
                .cluster_lifecycle_stop_count
                .load(Ordering::Relaxed),
            lifecycle_restart_count: self
                .inner
                .stats
                .cluster_lifecycle_restart_count
                .load(Ordering::Relaxed),
        }
    }

    /// Return aggregate 0.41 distributed-grid counters.
    pub fn cluster_grid_counters(&self) -> ClusterGridCounters {
        ClusterGridCounters {
            replication_success_total: self
                .inner
                .stats
                .cluster_replication_success_total
                .load(Ordering::Relaxed),
            replication_failure_total: self
                .inner
                .stats
                .cluster_replication_failure_total
                .load(Ordering::Relaxed),
            bytes_replicated_total: self
                .inner
                .stats
                .cluster_bytes_replicated_total
                .load(Ordering::Relaxed),
            replication_backpressure_total: self
                .inner
                .stats
                .cluster_replication_backpressure_total
                .load(Ordering::Relaxed),
            replication_oversized_rejected_total: self
                .inner
                .stats
                .cluster_replication_oversized_rejected_total
                .load(Ordering::Relaxed),
            replication_decrypt_failure_total: self
                .inner
                .stats
                .cluster_replication_decrypt_failure_total
                .load(Ordering::Relaxed),
            under_replicated_keys: self
                .inner
                .stats
                .cluster_under_replicated_keys
                .load(Ordering::Relaxed),
            failover_total: self
                .inner
                .stats
                .cluster_failover_total
                .load(Ordering::Relaxed),
            repair_task_total: self
                .inner
                .stats
                .cluster_repair_task_total
                .load(Ordering::Relaxed),
            repair_failure_total: self
                .inner
                .stats
                .cluster_repair_failure_total
                .load(Ordering::Relaxed),
            rebalance_plan_total: self
                .inner
                .stats
                .cluster_rebalance_plan_total
                .load(Ordering::Relaxed),
            rebalance_task_ack_total: self
                .inner
                .stats
                .cluster_rebalance_task_ack_total
                .load(Ordering::Relaxed),
            topology_fence_rejected_total: self
                .inner
                .stats
                .cluster_topology_fence_rejected_total
                .load(Ordering::Relaxed),
            tombstone_repair_debt: self
                .inner
                .stats
                .cluster_tombstone_repair_debt
                .load(Ordering::Relaxed),
            replicated_value_rejected_total: 0,
            split_brain_detected_total: 0,
            merge_discarded_entries_total: 0,
            merge_unresolved_conflicts_total: 0,
            cluster_auth_rejected_total: 0,
            repair_debt_degraded_mode: 0,
        }
    }

    /// Return the configured value-replication shape.
    pub fn replication_config(&self) -> ReplicationConfig {
        self.inner.replication_config
    }

    /// Return the replicated-value confidentiality posture.
    pub fn replicated_value_security_posture(&self) -> ReplicatedValueSecurityPosture {
        self.inner.replicated_value_security
    }

    /// Return the declared transport-security posture.
    pub fn transport_posture(&self) -> TransportPosture {
        self.inner.transport_posture
    }

    /// Return the configured client routing mode.
    pub fn routing_mode(&self) -> RoutingMode {
        self.inner.routing_mode
    }

    /// Return whether cluster read-through / remote peer-fetch is enabled.
    pub fn read_through_enabled(&self) -> bool {
        self.inner.read_through_enabled
    }

    /// Return a topology fence from the latest cluster diagnostics.
    pub fn cluster_topology_fence(&self) -> TopologyFence {
        let committed_epoch = self
            .cluster_diagnostics()
            .map(|diagnostics| diagnostics.epoch)
            .unwrap_or_default();
        TopologyFence::new(committed_epoch)
    }

    /// Return the boolean pilot readiness contract.
    pub fn cluster_pilot_readiness(&self) -> ClusterPilotReadiness {
        let diagnostics = self.cluster_diagnostics();
        let stats = self.stats();
        let member_count = diagnostics
            .as_ref()
            .map(|diagnostics| diagnostics.member_count)
            .unwrap_or_default();
        let lifecycle_operational = diagnostics
            .as_ref()
            .map(|diagnostics| diagnostics.lifecycle.is_running())
            .unwrap_or(false);
        let topology_committed = diagnostics
            .as_ref()
            .map(|diagnostics| diagnostics.epoch > Default::default())
            .unwrap_or(false);

        ClusterPilotReadiness {
            transport_posture: self.transport_posture(),
            has_members: member_count > 0,
            member_count,
            within_supported_size: (2..=5).contains(&member_count),
            strict_wire_compatibility: self.transport_posture().wire_strict,
            diagnostics_clean: stats.distributed_invalidation_decode_errors == 0
                && stats.distributed_invalidation_publish_failures == 0
                && stats.distributed_invalidation_receiver_closed == 0,
            lifecycle_operational,
            topology_committed,
        }
    }

    /// Return a dashboard-ready pilot report.
    pub fn cluster_pilot_report(&self) -> ClusterPilotReport {
        let readiness = self.cluster_pilot_readiness();
        let diagnostics = self.cluster_diagnostics();
        let ownership = self.cluster_ownership_diagnostics();
        let stats = self.stats();
        let fill = self.cluster_fill_counters();
        let staging = self.cluster_staging_counters();
        let transport_posture = self.transport_posture();

        ClusterPilotReport {
            readiness,
            counters: ClusterCacheCounters::from(fill),
            epoch: diagnostics
                .as_ref()
                .map(|diagnostics| diagnostics.epoch.value())
                .unwrap_or_default(),
            generation: diagnostics
                .as_ref()
                .map(|diagnostics| diagnostics.generation.value())
                .unwrap_or_default(),
            stamp: ownership
                .as_ref()
                .map(|diagnostics| diagnostics.stamp)
                .unwrap_or_default(),
            invalidations_published: stats.distributed_invalidations_published,
            invalidations_received: stats.distributed_invalidations_received,
            invalidations_applied: stats.distributed_invalidations_applied,
            invalidation_lagged: stats.distributed_invalidation_lagged,
            decode_errors: stats.distributed_invalidation_decode_errors,
            publish_failures: stats.distributed_invalidation_publish_failures,
            receiver_closed: stats.distributed_invalidation_receiver_closed,
            owner_load_success: fill.owner_load_success,
            owner_load_errors: fill.owner_load_errors,
            remote_fetch_success: fill.remote_fetch_success,
            remote_fetch_errors: fill.remote_fetch_errors,
            auth_failures: staging.peer_fetch_auth_failures,
            wire_version_failures: staging.wire_version_rejections,
            stale_generation_rejections: staging.stale_generation_rejected,
            barrier_timeouts: staging.barrier_timeouts,
            near_cache_conservative_invalidations: staging.near_cache_conservative_invalidations,
            lifecycle_stop_count: staging.lifecycle_stop_count,
            lifecycle_restart_count: staging.lifecycle_restart_count,
            transport_posture,
            highlights: transport_posture
                .highlight()
                .into_iter()
                .chain(self.replicated_value_security_posture().highlight())
                .map(str::to_owned)
                .collect(),
        }
    }

    /// Return a staging-focused cluster health summary.
    ///
    /// Local caches return `None`; client/member caches return a derived
    /// machine-readable health state and all counters used to compute it.
    pub fn cluster_staging_health(&self) -> Option<ClusterStagingHealth> {
        let diagnostics = self.cluster_diagnostics()?;
        Some(ClusterStagingHealth::from_parts(
            diagnostics,
            self.stats(),
            self.cluster_fill_counters(),
            self.cluster_staging_counters(),
        ))
    }

    /// Record a successful owner-side origin load for staging diagnostics.
    pub fn record_cluster_owner_load_success(&self) {
        self.inner
            .stats
            .cluster_owner_load_success
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failed owner-side origin load for staging diagnostics.
    pub fn record_cluster_owner_load_error(&self) {
        self.inner
            .stats
            .cluster_owner_load_errors
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a successful remote peer-fetch for staging diagnostics.
    pub fn record_cluster_remote_fetch_success(&self) {
        self.inner
            .stats
            .cluster_remote_fetch_success
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failed remote peer-fetch for staging diagnostics.
    pub fn record_cluster_remote_fetch_error(&self) {
        self.inner
            .stats
            .cluster_remote_fetch_errors
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a hot near-cache hit for a non-owned value.
    pub fn record_cluster_hot_cache_hit(&self) {
        self.inner
            .stats
            .cluster_hot_cache_hits
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a peer-fetch or owner-load auth failure.
    pub fn record_cluster_peer_fetch_auth_failure(&self) {
        self.inner
            .stats
            .cluster_peer_fetch_auth_failures
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a peer-fetch or owner-load wire-version rejection.
    pub fn record_cluster_wire_version_rejection(&self) {
        self.inner
            .stats
            .cluster_wire_version_rejections
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a stale-generation rejection observed by staging fencing checks.
    pub fn record_cluster_stale_generation_rejected(&self) {
        self.inner
            .stats
            .cluster_stale_generation_rejected
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a gossip reset/tombstone diagnostic.
    pub fn record_cluster_gossip_reset(&self, tombstone_age_ms: u64) {
        self.inner
            .stats
            .cluster_gossip_tombstone_age_ms
            .store(tombstone_age_ms, Ordering::Relaxed);
        self.inner
            .stats
            .cluster_gossip_reset_count
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a read-after-write/quorum barrier timeout.
    pub fn record_cluster_barrier_timeout(&self) {
        self.inner
            .stats
            .cluster_barrier_timeouts
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a conservative invalidation caused by near-cache repair.
    pub fn record_cluster_near_cache_conservative_invalidation(&self) {
        self.inner
            .stats
            .cluster_near_cache_conservative_invalidations
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a successful replicated value/tombstone send.
    pub fn record_cluster_replication_success(&self, bytes: u64) {
        self.inner
            .stats
            .cluster_replication_success_total
            .fetch_add(1, Ordering::Relaxed);
        self.inner
            .stats
            .cluster_bytes_replicated_total
            .fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record a failed replicated value/tombstone send.
    pub fn record_cluster_replication_failure(&self) {
        self.inner
            .stats
            .cluster_replication_failure_total
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record replication queue backpressure.
    pub fn record_cluster_replication_backpressure(&self) {
        self.inner
            .stats
            .cluster_replication_backpressure_total
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a value rejected by the replication byte cap.
    pub fn record_cluster_replication_oversized_rejected(&self) {
        self.inner
            .stats
            .cluster_replication_oversized_rejected_total
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a replicated payload decrypt/open failure.
    pub fn record_cluster_replication_decrypt_failure(&self) {
        self.inner
            .stats
            .cluster_replication_decrypt_failure_total
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Set the aggregate under-replicated key gauge.
    pub fn set_cluster_under_replicated_keys(&self, value: u64) {
        self.inner
            .stats
            .cluster_under_replicated_keys
            .store(value, Ordering::Relaxed);
    }

    /// Record a topology-fence rejection.
    pub fn record_cluster_topology_fence_rejected(&self) {
        self.inner
            .stats
            .cluster_topology_fence_rejected_total
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Set the aggregate tombstone repair-debt gauge.
    pub fn set_cluster_tombstone_repair_debt(&self, value: u64) {
        self.inner
            .stats
            .cluster_tombstone_repair_debt
            .store(value, Ordering::Relaxed);
    }

    /// Record a lifecycle stop observed by a pilot probe.
    pub fn record_cluster_lifecycle_stop(&self) {
        self.inner
            .stats
            .cluster_lifecycle_stop_count
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a lifecycle restart/rejoin observed by a pilot probe.
    pub fn record_cluster_lifecycle_restart(&self) {
        self.inner
            .stats
            .cluster_lifecycle_restart_count
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a synthetic direct-routing remote fetch for pilot routing tests.
    pub fn record_cluster_direct_remote_fetch(&self) -> Result<()> {
        if !self.read_through_enabled() {
            return Ok(());
        }
        match self.routing_mode() {
            RoutingMode::Direct => self.record_cluster_remote_fetch_success(),
            RoutingMode::SingleEndpoint => self.record_cluster_hot_cache_hit(),
        }
        Ok(())
    }

    /// Subscribe to membership events for this cache's attached cluster runtime.
    ///
    /// Local caches return `None`. Client/member caches return a bounded stream
    /// of events emitted after subscription; slow subscribers may receive lag
    /// errors.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use hydracache::{ClusterMembershipEvent, HydraCache, InMemoryCluster};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> hydracache::CacheResult<()> {
    /// let cluster = Arc::new(InMemoryCluster::new("orders"));
    /// let member = HydraCache::member()
    ///     .shared_cluster(cluster.clone())
    ///     .node_id("member-a")
    ///     .start()
    ///     .await?;
    /// let mut events = member
    ///     .subscribe_cluster_membership()
    ///     .expect("member has a cluster runtime");
    ///
    /// let _client = HydraCache::client()
    ///     .shared_cluster(cluster)
    ///     .node_id("client-a")
    ///     .connect()
    ///     .await?;
    ///
    /// assert!(matches!(
    ///     events.recv().await.expect("membership event"),
    ///     ClusterMembershipEvent::ClientConnected(_)
    /// ));
    /// # Ok(())
    /// # }
    /// ```
    pub fn subscribe_cluster_membership(&self) -> Option<ClusterMembershipSubscriber> {
        self.inner
            .cluster_runtime
            .as_ref()
            .map(ClusterRuntime::subscribe_membership)
    }

    /// Leave the attached cluster runtime when this cache is a client or member.
    ///
    /// Local caches return `Ok(None)`. A client/member cache returns the
    /// control-plane leave event when the node was still admitted.
    ///
    /// This removes cluster membership metadata, but it does not clear the
    /// local cache contents.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use hydracache::{ClusterMembershipEvent, ClusterRole, HydraCache, InMemoryCluster};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> hydracache::CacheResult<()> {
    /// let cluster = Arc::new(InMemoryCluster::new("orders"));
    /// let client = HydraCache::client()
    ///     .shared_cluster(cluster)
    ///     .node_id("client-a")
    ///     .connect()
    ///     .await?;
    ///
    /// let left = client.leave_cluster().await?.expect("client was admitted");
    /// assert!(matches!(
    ///     left,
    ///     ClusterMembershipEvent::NodeLeft {
    ///         role: ClusterRole::Client,
    ///         ..
    ///     }
    /// ));
    /// assert!(client.cluster_diagnostics().unwrap().lifecycle.is_stopped());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn leave_cluster(&self) -> Result<Option<ClusterMembershipEvent>> {
        match &self.inner.cluster_runtime {
            Some(runtime) => runtime.leave().await,
            None => Ok(None),
        }
    }

    /// Subscribe to cache events matching the provided filters.
    ///
    /// Dropping the returned subscriber unregisters it. Access/load events are
    /// only published when the cache was built with
    /// [`HydraCacheBuilder::enable_access_events`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::{CacheEventKind, CacheEventOptions, CacheOptions, HydraCache};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> hydracache::CacheResult<()> {
    /// let cache = HydraCache::local().build();
    /// let mut events = cache.subscribe(
    ///     CacheEventOptions::mutations().include_kind(CacheEventKind::Stored),
    /// );
    ///
    /// cache.put("answer", 42_u64, CacheOptions::new()).await?;
    ///
    /// let event = events.recv().await.expect("stored event");
    /// assert_eq!(event.kind(), CacheEventKind::Stored);
    /// assert_eq!(event.key(), Some("answer"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn subscribe(&self, options: CacheEventOptions) -> CacheEventSubscriber {
        self.inner
            .events
            .subscribe(options, self.inner.stats.clone())
    }

    /// Subscribe to mutation and invalidation events.
    pub fn subscribe_mutations(&self) -> CacheEventSubscriber {
        self.subscribe(CacheEventOptions::mutations())
    }

    /// Subscribe to access and loader events.
    ///
    /// These events are published only when the cache is built with
    /// [`HydraCacheBuilder::enable_access_events`].
    pub fn subscribe_access(&self) -> CacheEventSubscriber {
        self.subscribe(CacheEventOptions::access())
    }

    /// Subscribe to events for one exact physical key.
    pub fn subscribe_key(&self, key: impl Into<String>) -> CacheEventSubscriber {
        self.subscribe(CacheEventOptions::new().key(key))
    }

    /// Subscribe to events associated with one tag.
    pub fn subscribe_tag(&self, tag: impl Into<String>) -> CacheEventSubscriber {
        self.subscribe(CacheEventOptions::new().tag(tag))
    }

    /// Run a callback for events matching the provided filters.
    ///
    /// The callback runs in a background task over a normal event subscription;
    /// it is never executed directly by cache operations.
    pub fn add_listener<F>(
        &self,
        options: CacheEventOptions,
        listener: F,
    ) -> CacheEventListenerHandle
    where
        F: Fn(CacheEvent) + Send + 'static,
    {
        CacheEventListenerHandle::spawn(self.subscribe(options), listener)
    }

    /// Run a callback for mutation and invalidation events.
    pub fn on_mutation<F>(&self, listener: F) -> CacheEventListenerHandle
    where
        F: Fn(CacheEvent) + Send + 'static,
    {
        self.add_listener(CacheEventOptions::mutations(), listener)
    }

    /// Run a callback for access and loader events.
    ///
    /// These events are published only when the cache is built with
    /// [`HydraCacheBuilder::enable_access_events`].
    pub fn on_access<F>(&self, listener: F) -> CacheEventListenerHandle
    where
        F: Fn(CacheEvent) + Send + 'static,
    {
        self.add_listener(CacheEventOptions::access(), listener)
    }

    /// Get and decode a cached value.
    ///
    /// Returns `Ok(None)` when the key is missing or expired.
    pub async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        match self.inner.store.get(key).await {
            Some(entry) if entry.is_expired() => {
                self.remove_expired(key, &entry).await;
                self.inner.stats.misses.fetch_add(1, Ordering::Relaxed);
                self.publish_key_event_with_tags(
                    CacheEventKind::Miss,
                    key,
                    CacheEventOrigin::LocalApi,
                    || entry.tags.clone(),
                );
                Ok(None)
            }
            Some(entry) => match self.inner.codec.decode::<T>(&entry.value) {
                Ok(value) => {
                    self.inner.stats.hits.fetch_add(1, Ordering::Relaxed);
                    self.publish_key_event_with_tags(
                        CacheEventKind::Hit,
                        key,
                        CacheEventOrigin::LocalApi,
                        || entry.tags.clone(),
                    );
                    Ok(Some(value))
                }
                Err(error) => {
                    self.remove_entry(key, &entry).await;
                    self.inner.stats.misses.fetch_add(1, Ordering::Relaxed);
                    self.publish_key_event_with_tags(
                        CacheEventKind::Miss,
                        key,
                        CacheEventOrigin::LocalApi,
                        || entry.tags.clone(),
                    );
                    Err(error)
                }
            },
            None => {
                self.inner.stats.misses.fetch_add(1, Ordering::Relaxed);
                self.publish_key_event(
                    CacheEventKind::Miss,
                    key,
                    CacheEventOrigin::LocalApi,
                    Vec::<String>::new(),
                );
                Ok(None)
            }
        }
    }

    /// Get the encoded bytes stored for a key.
    ///
    /// This is mainly intended for transport adapters that need to move an
    /// already-encoded value between cache members without knowing the
    /// application type. Most application code should prefer [`get`](Self::get)
    /// or [`get_or_insert_with`](Self::get_or_insert_with).
    ///
    /// Expiration, hit/miss counters, and access events follow the same rules
    /// as [`get`](Self::get).
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::{CacheOptions, HydraCache};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> hydracache::CacheResult<()> {
    /// let cache = HydraCache::local().build();
    ///
    /// cache.put("answer", 42_u64, CacheOptions::new()).await?;
    ///
    /// let encoded = cache.get_encoded("answer").await?;
    /// assert!(encoded.is_some());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_encoded(&self, key: &str) -> Result<Option<Bytes>> {
        match self.inner.store.get(key).await {
            Some(entry) if entry.is_expired() => {
                self.remove_expired(key, &entry).await;
                self.inner.stats.misses.fetch_add(1, Ordering::Relaxed);
                self.publish_key_event_with_tags(
                    CacheEventKind::Miss,
                    key,
                    CacheEventOrigin::LocalApi,
                    || entry.tags.clone(),
                );
                Ok(None)
            }
            Some(entry) => {
                self.inner.stats.hits.fetch_add(1, Ordering::Relaxed);
                self.publish_key_event_with_tags(
                    CacheEventKind::Hit,
                    key,
                    CacheEventOrigin::LocalApi,
                    || entry.tags.clone(),
                );
                Ok(Some(entry.value))
            }
            None => {
                self.inner.stats.misses.fetch_add(1, Ordering::Relaxed);
                self.publish_key_event(
                    CacheEventKind::Miss,
                    key,
                    CacheEventOrigin::LocalApi,
                    Vec::<String>::new(),
                );
                Ok(None)
            }
        }
    }

    /// Store already-encoded bytes for a key.
    ///
    /// This is mainly intended for transport adapters and cluster near-cache
    /// hydration. The bytes must have been produced by a compatible
    /// [`CacheCodec`]; `HydraCache` stores them as-is and will decode them on a
    /// later typed [`get`](Self::get).
    ///
    /// TTLs, tags, tag-index updates, store events, and diagnostics follow the
    /// same rules as [`put`](Self::put).
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::{CacheOptions, HydraCache};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> hydracache::CacheResult<()> {
    /// let source = HydraCache::local().build();
    /// let target = HydraCache::local().build();
    ///
    /// source.put("answer", 42_u64, CacheOptions::new()).await?;
    /// let encoded = source
    ///     .get_encoded("answer")
    ///     .await?
    ///     .expect("source value is cached");
    ///
    /// target
    ///     .put_encoded("answer", encoded, CacheOptions::new().tag("answers"))
    ///     .await?;
    ///
    /// assert_eq!(target.get::<u64>("answer").await?, Some(42));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn put_encoded(
        &self,
        key: &str,
        value: impl Into<Bytes>,
        options: CacheOptions,
    ) -> Result<()> {
        self.put_bytes(key, value.into(), options).await
    }

    /// Encode and store a value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::{CacheOptions, HydraCache};
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    /// struct User {
    ///     id: u64,
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() -> hydracache::CacheResult<()> {
    /// let cache = HydraCache::local().build();
    ///
    /// cache.put("user:1", User { id: 1 }, CacheOptions::new()).await?;
    /// assert_eq!(cache.get::<User>("user:1").await?, Some(User { id: 1 }));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn put<T>(&self, key: &str, value: T, options: CacheOptions) -> Result<()>
    where
        T: Serialize,
    {
        let bytes = self.inner.codec.encode(&value)?;
        self.put_bytes(key, bytes, options).await
    }

    /// Get a value, or run the loader and cache its result on miss.
    ///
    /// Concurrent misses for the same key share one loader execution.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::{CacheOptions, HydraCache};
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    /// struct User {
    ///     id: u64,
    /// }
    ///
    /// #[derive(Debug)]
    /// struct LoaderError;
    ///
    /// impl std::fmt::Display for LoaderError {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         f.write_str("loader failed")
    ///     }
    /// }
    ///
    /// impl std::error::Error for LoaderError {}
    ///
    /// # #[tokio::main]
    /// # async fn main() -> hydracache::CacheResult<()> {
    /// let cache = HydraCache::local().build();
    ///
    /// let user = cache
    ///     .get_or_load("user:1", CacheOptions::new(), || async {
    ///         Ok::<_, LoaderError>(User { id: 1 })
    ///     })
    ///     .await?;
    ///
    /// assert_eq!(user, User { id: 1 });
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_or_load<T, E, F, Fut>(
        &self,
        key: &str,
        options: CacheOptions,
        loader: F,
    ) -> Result<T>
    where
        T: Serialize + DeserializeOwned,
        E: Error + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<T, E>> + Send + 'static,
    {
        if let Some(value) = self.get(key).await? {
            return Ok(value);
        }

        let shared = self
            .shared_load(key, options, move |cache| async move {
                cache.inner.stats.loads.fetch_add(1, Ordering::Relaxed);
                let value = loader().await.map_err(CacheError::loader)?;
                let bytes = cache.inner.codec.encode(&value)?;
                Ok(bytes)
            })
            .await;

        let bytes = shared.await.map_err(|error| (*error).clone())?;
        self.inner.codec.decode(&bytes)
    }

    /// Get a value with explicit refresh/stale behavior.
    ///
    /// This is the production-oriented sibling of [`HydraCache::get_or_load`].
    /// It keeps the same single-flight and invalidation-safety semantics, but
    /// can return a stale value while refreshing it in the background.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::time::Duration;
    ///
    /// use hydracache::{CacheOptions, HydraCache, RefreshOptions};
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    /// struct User {
    ///     id: u64,
    /// }
    ///
    /// #[derive(Debug)]
    /// struct LoaderError;
    ///
    /// impl std::fmt::Display for LoaderError {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         f.write_str("loader failed")
    ///     }
    /// }
    ///
    /// impl std::error::Error for LoaderError {}
    ///
    /// # #[tokio::main]
    /// # async fn main() -> hydracache::CacheResult<()> {
    /// let cache = HydraCache::local().build();
    ///
    /// let user = cache
    ///     .get_or_load_with_refresh(
    ///         "user:1",
    ///         CacheOptions::new().ttl(Duration::from_secs(60)).tag("user:1"),
    ///         RefreshOptions::new()
    ///             .refresh_ahead(Duration::from_secs(10))
    ///             .stale_while_revalidate(Duration::from_secs(300))
    ///             .serve_stale_on_loader_error(true),
    ///         || async { Ok::<_, LoaderError>(User { id: 1 }) },
    ///     )
    ///     .await?;
    ///
    /// assert_eq!(user, User { id: 1 });
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_or_load_with_refresh<T, E, F, Fut>(
        &self,
        key: &str,
        options: CacheOptions,
        refresh_options: RefreshOptions,
        loader: F,
    ) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Error + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<T, E>> + Send + 'static,
    {
        let mut loader = Some(loader);

        if let Some(entry) = self.inner.store.get(key).await {
            if !entry.is_expired() {
                let value = self.decode_cached_hit(key, &entry).await?;
                if refresh_options
                    .refresh_ahead_value()
                    .is_some_and(|threshold| entry.refresh_ahead_due(threshold))
                {
                    self.spawn_background_refresh(
                        key.to_owned(),
                        options,
                        take_loader(&mut loader),
                    );
                }
                return Ok(value);
            }

            let in_stale_while_revalidate_window =
                entry_in_stale_window(&entry, refresh_options.stale_while_revalidate_value());

            if in_stale_while_revalidate_window {
                match self.decode_cached_hit(key, &entry).await {
                    Ok(value) => {
                        self.spawn_background_refresh(
                            key.to_owned(),
                            options,
                            take_loader(&mut loader),
                        );
                        return Ok(value);
                    }
                    Err(_) => {
                        self.remove_entry(key, &entry).await;
                    }
                }
            } else {
                self.remove_expired(key, &entry).await;
            }

            self.record_miss(key, || entry.tags.clone());

            let fallback = refresh_options
                .serve_stale_on_loader_error_value()
                .then(|| {
                    entry_in_stale_window(&entry, refresh_options.stale_on_loader_error_window())
                        .then(|| entry.clone())
                })
                .flatten();

            return self
                .load_and_decode_with_stale_fallback(
                    key,
                    options,
                    take_loader(&mut loader),
                    fallback,
                )
                .await;
        }

        self.record_miss(key, Vec::<String>::new);
        self.load_and_decode_with_stale_fallback(key, options, take_loader(&mut loader), None)
            .await
    }

    /// Get a value, or compute and cache it with an infallible async loader.
    ///
    /// This is the most ergonomic local-cache spelling for loaders that cannot
    /// fail in application terms. Fallible loaders should use `try_get_or_insert_with`
    /// or `get_or_load`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::{CacheOptions, HydraCache};
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    /// struct User {
    ///     id: u64,
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() -> hydracache::CacheResult<()> {
    /// let cache = HydraCache::local().build();
    ///
    /// let user = cache
    ///     .get_or_insert_with("user:1", CacheOptions::new(), || async { User { id: 1 } })
    ///     .await?;
    ///
    /// assert_eq!(user, User { id: 1 });
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_or_insert_with<T, F, Fut>(
        &self,
        key: &str,
        options: CacheOptions,
        loader: F,
    ) -> Result<T>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        self.get_or_load(key, options, move || async move {
            Ok::<_, std::convert::Infallible>(loader().await)
        })
        .await
    }

    /// Get a value, or run a fallible async loader and cache its result on miss.
    ///
    /// This is an alias for `get_or_load` with a name that mirrors common
    /// cache-map APIs.
    pub async fn try_get_or_insert_with<T, E, F, Fut>(
        &self,
        key: &str,
        options: CacheOptions,
        loader: F,
    ) -> Result<T>
    where
        T: Serialize + DeserializeOwned,
        E: Error + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<T, E>> + Send + 'static,
    {
        self.get_or_load(key, options, loader).await
    }

    /// Remove one key from the cache.
    pub async fn invalidate_key(&self, key: &str) -> Result<bool> {
        let removed = self
            .remove_with_event(
                key,
                CacheEventKind::KeyInvalidated,
                CacheEventOrigin::LocalApi,
            )
            .await?;
        self.publish_invalidation(CacheInvalidation::key(key))
            .await?;
        Ok(removed)
    }

    /// Remove one key from the cache.
    ///
    /// This is an alias for `invalidate_key` with a shorter name for local-cache use.
    pub async fn remove(&self, key: &str) -> Result<bool> {
        let removed = self
            .remove_with_event(key, CacheEventKind::Removed, CacheEventOrigin::LocalApi)
            .await?;
        self.publish_invalidation(CacheInvalidation::key(key))
            .await?;
        Ok(removed)
    }

    /// Return whether the key currently maps to a usable value.
    ///
    /// Expired entries are removed and reported as absent.
    pub async fn contains_key(&self, key: &str) -> bool {
        match self.inner.store.get(key).await {
            Some(entry) if entry.is_expired() => {
                self.remove_expired(key, &entry).await;
                false
            }
            Some(_) => true,
            None => false,
        }
    }

    /// Remove all entries currently associated with a tag.
    ///
    /// Tag invalidation also advances the tag generation. Tagged loaders that
    /// started before the invalidation will return to their caller but skip
    /// storing stale values back into the cache.
    pub async fn invalidate_tag(&self, tag: &str) -> Result<u64> {
        let removed = self
            .invalidate_tag_with_origin(tag, CacheEventOrigin::LocalApi)
            .await?;
        self.publish_invalidation(CacheInvalidation::tag(tag))
            .await?;
        Ok(removed)
    }

    async fn invalidate_tag_with_origin(&self, tag: &str, origin: CacheEventOrigin) -> Result<u64> {
        let keys = self.inner.tag_index.take_tag(tag).await;
        let mut removed = 0;

        for key in keys {
            if let Some(entry) = self.inner.store.get(&key).await {
                self.remove_entry(&key, &entry).await;
                removed += 1;
            }
        }

        if removed > 0 {
            self.inner
                .stats
                .invalidations
                .fetch_add(removed, Ordering::Relaxed);
        }

        self.publish_tag_event(CacheEventKind::TagInvalidated, tag, removed, origin);

        Ok(removed)
    }

    /// Remove all cached entries and tag mappings.
    pub async fn flush(&self) -> Result<()> {
        self.flush_with_origin(CacheEventOrigin::LocalApi).await?;
        self.publish_invalidation(CacheInvalidation::flush()).await
    }

    async fn flush_with_origin(&self, origin: CacheEventOrigin) -> Result<()> {
        let estimated_entries = self.inner.store.entry_count();
        self.inner.store.invalidate_all();
        self.inner.tag_index.clear().await;
        self.publish_cache_event(CacheEventKind::Flushed, Some(estimated_entries), origin);
        Ok(())
    }

    /// Return a snapshot of lightweight cache counters.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::{CacheOptions, HydraCache};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> hydracache::CacheResult<()> {
    /// let cache = HydraCache::local().build();
    ///
    /// let first = cache
    ///     .get_or_insert_with("answer", CacheOptions::new(), || async { 42_u64 })
    ///     .await?;
    /// let second = cache
    ///     .get_or_insert_with("answer", CacheOptions::new(), || async { 7_u64 })
    ///     .await?;
    ///
    /// let stats = cache.stats();
    /// assert_eq!((first, second), (42, 42));
    /// assert_eq!(stats.loads, 1);
    /// assert_eq!(stats.hits, 1);
    /// assert_eq!(stats.hit_ratio(), Some(0.5));
    /// # Ok(())
    /// # }
    /// ```
    pub fn stats(&self) -> CacheStats {
        self.inner.stats.snapshot()
    }

    /// Return a diagnostic snapshot for quick application-level smoke checks.
    ///
    /// `diagnostics` includes [`CacheStats`] plus the local backend's
    /// approximate entry count. Use it to answer questions like "did this call
    /// hit the cache on the second run?" without wiring a metrics system yet.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::{CacheOptions, HydraCache};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> hydracache::CacheResult<()> {
    /// let cache = HydraCache::local().build();
    ///
    /// cache
    ///     .get_or_insert_with("report:daily", CacheOptions::new(), || async { 1_u64 })
    ///     .await?;
    /// cache
    ///     .get_or_insert_with("report:daily", CacheOptions::new(), || async { 2_u64 })
    ///     .await?;
    ///
    /// let diagnostics = cache.diagnostics().await;
    /// assert_eq!(diagnostics.stats.loads, 1);
    /// assert_eq!(diagnostics.stats.hits, 1);
    /// assert_eq!(diagnostics.total_requests(), 2);
    /// assert!(!diagnostics.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn diagnostics(&self) -> CacheDiagnostics {
        self.inner.store.run_pending_tasks().await;
        CacheDiagnostics {
            stats: self.stats(),
            estimated_entries: self.inner.store.entry_count(),
        }
    }

    async fn decode_cached_hit<T>(&self, key: &str, entry: &CacheEntry) -> Result<T>
    where
        T: DeserializeOwned,
    {
        match self.inner.codec.decode::<T>(&entry.value) {
            Ok(value) => {
                self.inner.stats.hits.fetch_add(1, Ordering::Relaxed);
                self.publish_key_event_with_tags(
                    CacheEventKind::Hit,
                    key,
                    CacheEventOrigin::LocalApi,
                    || entry.tags.clone(),
                );
                Ok(value)
            }
            Err(error) => {
                self.remove_entry(key, entry).await;
                self.record_miss(key, || entry.tags.clone());
                Err(error)
            }
        }
    }

    fn record_miss<I, S>(&self, key: &str, tags: I)
    where
        I: FnOnce() -> S,
        S: IntoIterator,
        S::Item: Into<String>,
    {
        self.inner.stats.misses.fetch_add(1, Ordering::Relaxed);
        self.publish_key_event_with_tags(
            CacheEventKind::Miss,
            key,
            CacheEventOrigin::LocalApi,
            tags,
        );
    }

    fn spawn_background_refresh<T, E, F, Fut>(&self, key: String, options: CacheOptions, loader: F)
    where
        T: Serialize + Send + 'static,
        E: Error + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<T, E>> + Send + 'static,
    {
        let cache = self.clone();
        tokio::spawn(async move {
            let shared = cache
                .shared_load(&key, options, move |cache| async move {
                    cache.inner.stats.loads.fetch_add(1, Ordering::Relaxed);
                    let value = loader().await.map_err(CacheError::loader)?;
                    let bytes = cache.inner.codec.encode(&value)?;
                    Ok(bytes)
                })
                .await;
            let _ = shared.await;
        });
    }

    async fn load_and_decode_with_stale_fallback<T, E, F, Fut>(
        &self,
        key: &str,
        options: CacheOptions,
        loader: F,
        stale_entry: Option<CacheEntry>,
    ) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Error + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<T, E>> + Send + 'static,
    {
        let shared = self
            .shared_load(key, options, move |cache| async move {
                cache.inner.stats.loads.fetch_add(1, Ordering::Relaxed);
                let value = loader().await.map_err(CacheError::loader)?;
                let bytes = cache.inner.codec.encode(&value)?;
                Ok(bytes)
            })
            .await;

        match shared.await {
            Ok(bytes) => self.inner.codec.decode(&bytes),
            Err(error) => {
                if let Some(entry) = stale_entry {
                    match self.decode_cached_hit(key, &entry).await {
                        Ok(value) => return Ok(value),
                        Err(_) => {
                            self.remove_entry(key, &entry).await;
                        }
                    }
                }
                Err((*error).clone())
            }
        }
    }

    pub(crate) async fn put_bytes(
        &self,
        key: &str,
        value: Bytes,
        options: CacheOptions,
    ) -> Result<()> {
        self.put_bytes_unchecked(key, value, options, CacheEventOrigin::LocalApi)
            .await
    }

    async fn put_bytes_unchecked(
        &self,
        key: &str,
        value: Bytes,
        options: CacheOptions,
        origin: CacheEventOrigin,
    ) -> Result<()> {
        if self.exceeds_max_entry_bytes(value.len()) {
            return Err(self.oversize_rejection_error(value.len()));
        }

        let ttl = options.ttl_value().unwrap_or(self.inner.default_ttl);
        let tags = options.tags_value().to_vec();
        let entry = CacheEntry {
            value,
            tags: tags.clone(),
            expires_at: Instant::now().checked_add(ttl),
        };

        if let Some(old_entry) = self.inner.store.get(key).await {
            self.inner.tag_index.unregister(key, &old_entry.tags).await;
        }

        self.inner.store.insert(key.to_owned(), entry).await;
        self.inner.tag_index.register(key, &tags).await;
        self.publish_key_event(CacheEventKind::Stored, key, origin, tags);
        Ok(())
    }

    async fn put_bytes_if_fresh(
        &self,
        key: &str,
        value: Bytes,
        options: CacheOptions,
        generation: &LoadGenerationSnapshot,
    ) -> Result<bool> {
        if self.exceeds_max_entry_bytes(value.len()) {
            self.record_oversize_rejection();
            return Ok(false);
        }

        if !self.inner.tag_index.is_current(generation).await {
            self.inner
                .stats
                .stale_load_discards
                .fetch_add(1, Ordering::Relaxed);
            self.publish_key_event_with_tags(
                CacheEventKind::StaleLoadDiscarded,
                key,
                CacheEventOrigin::Loader,
                || options.tags_value().to_vec(),
            );
            return Ok(false);
        }

        self.put_bytes_unchecked(key, value, options, CacheEventOrigin::Loader)
            .await?;
        Ok(true)
    }

    async fn shared_load<F, Fut>(
        &self,
        key: &str,
        options: CacheOptions,
        loader: F,
    ) -> SharedLoadFuture
    where
        F: FnOnce(Self) -> Fut + Send + 'static,
        Fut: Future<Output = Result<Bytes>> + Send + 'static,
    {
        let generation = self.inner.tag_index.snapshot(options.tags_value()).await;

        if let Some(shared) = self.inner.in_flight.get_current(key, &generation).await {
            self.inner
                .stats
                .single_flight_joins
                .fetch_add(1, Ordering::Relaxed);
            self.publish_key_event_with_tags(
                CacheEventKind::SingleFlightJoined,
                key,
                CacheEventOrigin::SingleFlight,
                || options.tags_value().to_vec(),
            );
            return shared;
        }

        // Coverage builds get one cooperative scheduling point here so tests can
        // deterministically exercise the defensive "insert_or_get_current lost
        // the race" branch below. Normal builds do not yield on this path.
        #[cfg(coverage)]
        tokio::task::yield_now().await;

        let key_owned = key.to_owned();
        let cache = self.clone();
        let load_key = key_owned.clone();
        let load_generation = generation.clone();
        let late_join_event_tags = self
            .event_tags_if_observed(CacheEventKind::SingleFlightJoined, || {
                options.tags_value().to_vec()
            });
        let shared = async move {
            let load_event_tags = cache.event_tags_if_observed(CacheEventKind::LoadStarted, || {
                options.tags_value().to_vec()
            });
            let load_completed_event_tags = cache
                .event_tags_if_observed(CacheEventKind::LoadCompleted, || {
                    options.tags_value().to_vec()
                });
            let load_failed_event_tags = cache
                .event_tags_if_observed(CacheEventKind::LoadFailed, || {
                    options.tags_value().to_vec()
                });

            let result = async {
                cache.publish_key_event_with_prepared_tags(
                    CacheEventKind::LoadStarted,
                    &load_key,
                    CacheEventOrigin::Loader,
                    load_event_tags,
                );
                let bytes = loader(cache.clone()).await?;
                let accepted = cache
                    .put_bytes_if_fresh(&load_key, bytes.clone(), options, &load_generation)
                    .await?;
                if accepted {
                    cache.publish_key_event_with_prepared_tags(
                        CacheEventKind::LoadCompleted,
                        &load_key,
                        CacheEventOrigin::Loader,
                        load_completed_event_tags,
                    );
                }
                Ok(bytes)
            }
            .await;

            if result.is_err() {
                cache.publish_key_event_with_prepared_tags(
                    CacheEventKind::LoadFailed,
                    &load_key,
                    CacheEventOrigin::Loader,
                    load_failed_event_tags,
                );
            }

            let result = result.map_err(Arc::new);

            cache
                .inner
                .in_flight
                .remove_if_generation_matches(&load_key, &load_generation)
                .await;
            result
        }
        .boxed()
        .shared();

        let (shared, inserted) = self
            .inner
            .in_flight
            .insert_or_get_current(key_owned, shared, generation)
            .await;
        if !inserted {
            self.inner
                .stats
                .single_flight_joins
                .fetch_add(1, Ordering::Relaxed);
            self.publish_key_event_with_prepared_tags(
                CacheEventKind::SingleFlightJoined,
                key,
                CacheEventOrigin::SingleFlight,
                late_join_event_tags,
            );
        }

        shared
    }

    fn exceeds_max_entry_bytes(&self, encoded_len: usize) -> bool {
        encoded_len > self.inner.max_entry_bytes
    }

    fn record_oversize_rejection(&self) {
        self.inner
            .stats
            .oversize_rejections
            .fetch_add(1, Ordering::Relaxed);
    }

    fn oversize_rejection_error(&self, encoded_len: usize) -> CacheError {
        self.record_oversize_rejection();
        CacheError::Backend(format!(
            "encoded cache entry is {encoded_len} bytes, exceeding max_entry_bytes={}",
            self.inner.max_entry_bytes
        ))
    }

    async fn remove_expired(&self, key: &str, entry: &CacheEntry) {
        self.remove_entry(key, entry).await;
        self.publish_key_event_with_tags(
            CacheEventKind::Expired,
            key,
            CacheEventOrigin::Backend,
            || entry.tags.clone(),
        );
    }

    async fn remove_entry(&self, key: &str, entry: &CacheEntry) {
        self.inner.store.invalidate(key).await;
        self.inner.tag_index.unregister(key, &entry.tags).await;
    }

    async fn remove_with_event(
        &self,
        key: &str,
        kind: CacheEventKind,
        origin: CacheEventOrigin,
    ) -> Result<bool> {
        let Some(entry) = self.inner.store.get(key).await else {
            return Ok(false);
        };

        self.remove_entry(key, &entry).await;
        self.inner
            .stats
            .invalidations
            .fetch_add(1, Ordering::Relaxed);
        self.publish_key_event_with_tags(kind, key, origin, || entry.tags.clone());
        Ok(true)
    }

    fn may_publish_event(&self, kind: CacheEventKind) -> bool {
        self.inner.events.may_publish(kind)
    }

    fn event_tags_if_observed<F>(&self, kind: CacheEventKind, tags: F) -> Option<Vec<String>>
    where
        F: FnOnce() -> Vec<String>,
    {
        self.may_publish_event(kind).then(tags)
    }

    fn publish_key_event<I, S>(
        &self,
        kind: CacheEventKind,
        key: &str,
        origin: CacheEventOrigin,
        tags: I,
    ) where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.publish_key_event_with_tags(kind, key, origin, || tags);
    }

    fn publish_key_event_with_tags<F, I, S>(
        &self,
        kind: CacheEventKind,
        key: &str,
        origin: CacheEventOrigin,
        tags: F,
    ) where
        F: FnOnce() -> I,
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        if !self.may_publish_event(kind) {
            return;
        }

        self.publish_event(CacheEvent::for_key(kind, key, origin, tags()));
    }

    fn publish_key_event_with_prepared_tags(
        &self,
        kind: CacheEventKind,
        key: &str,
        origin: CacheEventOrigin,
        tags: Option<Vec<String>>,
    ) {
        if let Some(tags) = tags {
            self.publish_key_event(kind, key, origin, tags);
        }
    }

    fn publish_tag_event(
        &self,
        kind: CacheEventKind,
        tag: &str,
        affected_keys: u64,
        origin: CacheEventOrigin,
    ) {
        if !self.may_publish_event(kind) {
            return;
        }

        self.publish_event(CacheEvent::for_tag(kind, tag, affected_keys, origin));
    }

    fn publish_cache_event(
        &self,
        kind: CacheEventKind,
        affected_keys: Option<u64>,
        origin: CacheEventOrigin,
    ) {
        if !self.may_publish_event(kind) {
            return;
        }

        self.publish_event(CacheEvent::for_cache(kind, affected_keys, origin));
    }

    fn publish_event(&self, event: CacheEvent) {
        self.inner.events.publish(event, &self.inner.stats);
    }

    async fn publish_invalidation(&self, invalidation: CacheInvalidation) -> Result<()> {
        let Some(bus) = &self.inner.invalidation_bus else {
            return Ok(());
        };

        let mut message =
            CacheInvalidationMessage::new(self.inner.invalidation_node_id.clone(), invalidation);
        if let Some(runtime) = &self.inner.cluster_runtime {
            if let Err(error) = runtime.validate_generation().await {
                self.inner
                    .stats
                    .distributed_invalidation_publish_failures
                    .fetch_add(1, Ordering::Relaxed);
                return Err(error);
            }
            message = message.with_source_generation(runtime.generation());
        }

        if let Err(error) = bus.publish(message).await {
            self.inner
                .stats
                .distributed_invalidation_publish_failures
                .fetch_add(1, Ordering::Relaxed);
            return Err(error);
        }
        self.inner
            .stats
            .distributed_invalidations_published
            .fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub(crate) fn spawn_invalidation_listener(&self, mut shutdown: watch::Receiver<bool>) {
        let Some(bus) = self.inner.invalidation_bus.clone() else {
            return;
        };
        let mut receiver = bus.subscribe();
        let node_id = self.inner.invalidation_node_id.clone();
        let weak_inner = Arc::downgrade(&self.inner);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown.changed() => break,
                    received = receiver.recv() => {
                        let Some(inner) = weak_inner.upgrade() else {
                            break;
                        };
                        match received {
                            CacheInvalidationReceive::Message(message) => {
                                if message.source_id() == node_id {
                                    continue;
                                }

                                let cache = HydraCache { inner };
                                let _ = cache.apply_remote_invalidation(message).await;
                            }
                            CacheInvalidationReceive::Lagged(count) => {
                                inner
                                    .stats
                                    .distributed_invalidation_lagged
                                    .fetch_add(count, Ordering::Relaxed);
                            }
                            CacheInvalidationReceive::DecodeError(_) => {
                                inner
                                    .stats
                                    .distributed_invalidation_decode_errors
                                    .fetch_add(1, Ordering::Relaxed);
                            }
                            CacheInvalidationReceive::Closed => {
                                inner
                                    .stats
                                    .distributed_invalidation_receiver_closed
                                    .fetch_add(1, Ordering::Relaxed);
                                break;
                            }
                        }
                    }
                }
            }
        });
    }

    async fn apply_remote_invalidation(&self, message: CacheInvalidationMessage) -> Result<()> {
        if let (Some(runtime), Some(generation)) =
            (&self.inner.cluster_runtime, message.source_generation())
        {
            let source_id = ClusterNodeId::from(message.source_id().to_owned());
            runtime
                .validate_remote_generation(&source_id, generation)
                .await?;
        }

        let (_, invalidation) = message.into_parts();
        self.inner
            .stats
            .distributed_invalidations_received
            .fetch_add(1, Ordering::Relaxed);

        match invalidation {
            CacheInvalidation::Key { key } => {
                self.remove_with_event(
                    &key,
                    CacheEventKind::KeyInvalidated,
                    CacheEventOrigin::DistributedBus,
                )
                .await?;
            }
            CacheInvalidation::Tag { tag } => {
                self.invalidate_tag_with_origin(&tag, CacheEventOrigin::DistributedBus)
                    .await?;
            }
            CacheInvalidation::Flush => {
                self.flush_with_origin(CacheEventOrigin::DistributedBus)
                    .await?;
            }
        }

        self.inner
            .stats
            .distributed_invalidations_applied
            .fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

fn entry_in_stale_window(entry: &CacheEntry, window: Option<std::time::Duration>) -> bool {
    window
        .map(|window| entry.stale_window_contains_now(window))
        .unwrap_or(false)
}

fn take_loader<F>(loader: &mut Option<F>) -> F {
    loader
        .take()
        .expect("refresh loader is consumed exactly once per cache call")
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use hydracache_core::{CacheEventKind, CacheEventOrigin};

    use super::HydraCache;

    #[test]
    fn lazy_key_event_tags_are_not_built_without_subscribers() {
        let cache = HydraCache::local().build();
        let tag_builds = AtomicUsize::new(0);

        cache.publish_key_event_with_tags(
            CacheEventKind::Stored,
            "user:42",
            CacheEventOrigin::LocalApi,
            || {
                tag_builds.fetch_add(1, Ordering::Relaxed);
                vec!["users".to_owned()]
            },
        );

        assert_eq!(tag_builds.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn lazy_key_event_tags_are_built_for_observed_mutations() {
        let cache = HydraCache::local().build();
        let _events = cache.subscribe_mutations();
        let tag_builds = AtomicUsize::new(0);

        cache.publish_key_event_with_tags(
            CacheEventKind::Stored,
            "user:42",
            CacheEventOrigin::LocalApi,
            || {
                tag_builds.fetch_add(1, Ordering::Relaxed);
                vec!["users".to_owned()]
            },
        );

        assert_eq!(tag_builds.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn lazy_key_event_tags_respect_disabled_access_events() {
        let cache = HydraCache::local().build();
        let _events = cache.subscribe_access();
        let tag_builds = AtomicUsize::new(0);

        cache.publish_key_event_with_tags(
            CacheEventKind::Hit,
            "user:42",
            CacheEventOrigin::LocalApi,
            || {
                tag_builds.fetch_add(1, Ordering::Relaxed);
                vec!["users".to_owned()]
            },
        );

        assert_eq!(tag_builds.load(Ordering::Relaxed), 0);
    }
}
