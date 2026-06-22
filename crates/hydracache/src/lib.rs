//! User-facing HydraCache runtime.
//!
//! HydraCache is local-first: [`HydraCache::local`] has no network dependency.
//! Optional client/member builders add the first cluster API shape on top of
//! the same local cache and distributed invalidation bus.
//!
//! # Quick start
//!
//! ```rust
//! use std::time::Duration;
//!
//! use hydracache::{CacheOptions, HydraCache};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
//! struct User {
//!     id: u64,
//!     name: String,
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cache = HydraCache::local()
//!     .default_ttl(Duration::from_secs(300))
//!     .max_capacity(10_000)
//!     .build();
//!
//! let user = cache
//!     .get_or_insert_with("user:42", CacheOptions::new().tag("user:42"), || async {
//!         User {
//!             id: 42,
//!             name: "Ada".to_owned(),
//!         }
//!     })
//!     .await?;
//!
//! assert_eq!(user.id, 42);
//! cache.invalidate_tag("user:42").await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Cacheable functions
//!
//! Use [`cacheable_loader!`] when an ordinary async function or expensive operation
//! should be cached without introducing database-result-cache concepts.
//! `cacheable_loader!` wraps fallible loaders. [`cacheable_infallible!`] wraps loaders
//! that return a value directly.
//!
//! ```rust
//! use std::time::Duration;
//!
//! use hydracache::{cacheable_loader, cacheable_infallible, HydraCache};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
//! struct Report {
//!     id: u64,
//! }
//!
//! #[derive(Debug)]
//! struct LoadError;
//!
//! impl std::fmt::Display for LoadError {
//!     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!         f.write_str("load failed")
//!     }
//! }
//!
//! impl std::error::Error for LoadError {}
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cache = HydraCache::local().build();
//!
//! let report = cacheable_loader!(
//!     cache = cache,
//!     key = "report:42",
//!     tags = ["reports", "report:42"],
//!     ttl = Duration::from_secs(60),
//!     load = || async { Ok::<_, LoadError>(Report { id: 42 }) },
//! )
//! .await?;
//!
//! assert_eq!(report.id, 42);
//!
//! let total = cacheable_infallible!(
//!     cache = cache,
//!     key = "report-total:42",
//!     tags = ["reports", "report:42"],
//!     ttl_secs = 60,
//!     load = || async { 42_u64 },
//! )
//! .await?;
//!
//! assert_eq!(total, 42);
//! # Ok(())
//! # }
//! ```
//!
//! Use [`CacheKeyBuilder`] and [`TagSet`] when the key and invalidation tags are
//! generated from the same domain metadata:
//!
//! ```rust
//! use hydracache::{cacheable_loader, CacheKeyBuilder, HydraCache, TagSet};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
//! struct Profile {
//!     id: u64,
//! }
//!
//! #[derive(Debug)]
//! struct LoadError;
//!
//! impl std::fmt::Display for LoadError {
//!     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!         f.write_str("load failed")
//!     }
//! }
//!
//! impl std::error::Error for LoadError {}
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cache = HydraCache::local().build();
//! let profile_id = 42_u64;
//! let key = CacheKeyBuilder::new()
//!     .entity("profile", profile_id)
//!     .build_string();
//!
//! let profile = cacheable_loader!(
//!     cache = cache,
//!     key = key.as_str(),
//!     tags = TagSet::new().tag("profiles").entity("profile", profile_id),
//!     ttl_secs = 60,
//!     load = move || async move {
//!         Ok::<_, LoadError>(Profile { id: profile_id })
//!     },
//! )
//! .await?;
//!
//! assert_eq!(profile.id, 42);
//! cache.invalidate_tag("profile:42").await?;
//! # Ok(())
//! # }
//! ```
//!
//! Use [`cacheable`] when the cached operation is naturally an async function.
//! The cache remains an explicit argument; the generated wrapper returns
//! [`CacheResult`] because cache errors can occur outside the user loader:
//!
//! ```rust
//! use hydracache::{cacheable, HydraCache};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
//! struct Profile {
//!     id: u64,
//! }
//!
//! #[derive(Debug)]
//! struct LoadError;
//!
//! impl std::fmt::Display for LoadError {
//!     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!         f.write_str("load failed")
//!     }
//! }
//!
//! impl std::error::Error for LoadError {}
//!
//! #[cacheable(
//!     cache = cache,
//!     key_segments = ["profile", profile_id],
//!     tag_segments = [["profile", profile_id], ["profiles"]],
//!     ttl_secs = 60
//! )]
//! async fn load_profile(
//!     cache: &HydraCache,
//!     profile_id: u64,
//! ) -> Result<Profile, LoadError> {
//!     Ok(Profile { id: profile_id })
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cache = HydraCache::local().build();
//! let profile = load_profile(&cache, 42).await?;
//!
//! assert_eq!(profile.id, 42);
//! # Ok(())
//! # }
//! ```
//!
//! # Typed local cache
//!
//! ```rust
//! use hydracache::{CacheOptions, HydraCache};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
//! struct User {
//!     id: u64,
//!     name: String,
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cache = HydraCache::local().build();
//! let users = cache.typed::<User>("users");
//!
//! users
//!     .put(
//!         "42",
//!         User {
//!             id: 42,
//!             name: "Ada".to_owned(),
//!         },
//!         CacheOptions::new(),
//!     )
//!     .await?;
//!
//! let cached = users.get("42").await?;
//! assert_eq!(cached.map(|user| user.id), Some(42));
//! # Ok(())
//! # }
//! ```
//!
//! # Cache events
//!
//! Use [`HydraCache::subscribe`] when an application, actuator, or sandbox
//! wants to observe cache mutations without wrapping every call manually.
//! Access/load events are opt-in because hit/miss streams can be noisy.
//!
//! ```rust
//! use hydracache::{CacheEventKind, CacheOptions, HydraCache};
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cache = HydraCache::local().build();
//! let mut events = cache.subscribe_mutations();
//!
//! cache
//!     .put("user:42", 42_u64, CacheOptions::new().tag("users"))
//!     .await?;
//!
//! let event = events.recv().await.expect("stored event");
//! assert_eq!(event.kind(), CacheEventKind::Stored);
//! assert_eq!(event.key(), Some("user:42"));
//! assert_eq!(event.tags(), &["users".to_owned()]);
//! # Ok(())
//! # }
//! ```
//!
//! Callback listeners are adapters over the same subscription stream:
//!
//! ```rust
//! use hydracache::{CacheOptions, HydraCache};
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cache = HydraCache::local().build();
//! let listener = cache.on_mutation(|event| {
//!     println!("cache changed: {event:?}");
//! });
//!
//! cache.put("user:42", 42_u64, CacheOptions::new()).await?;
//! listener.unsubscribe();
//! # Ok(())
//! # }
//! ```
//!
//! Event publication is preflighted before HydraCache builds owned event
//! payloads. If an event kind is disabled or no active subscriber can receive
//! it, the cache operation skips the event allocation path. Access events still
//! require both a subscriber and [`HydraCacheBuilder::enable_access_events`].
//!
//! ```rust
//! use hydracache::{CacheEventKind, CacheOptions, HydraCache};
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let quiet_cache = HydraCache::local().build();
//! quiet_cache
//!     .put("user:42", "Ada", CacheOptions::new().tag("users"))
//!     .await?;
//! assert_eq!(quiet_cache.stats().events_published, 0);
//!
//! let observed_cache = HydraCache::local().build();
//! let mut events = observed_cache.subscribe_mutations();
//! observed_cache
//!     .put("user:43", "Grace", CacheOptions::new().tag("users"))
//!     .await?;
//!
//! let event = events.recv().await.expect("stored event");
//! assert_eq!(event.kind(), CacheEventKind::Stored);
//! assert_eq!(observed_cache.stats().events_published, 1);
//! # Ok(())
//! # }
//! ```
//!
//! # Distributed invalidation bus
//!
//! Use [`InMemoryInvalidationBus`] when several cache instances in one process
//! should propagate invalidation intent to each other. The bus only sends
//! `invalidate_key`, `invalidate_tag`, `remove`, and `flush` operations; cached
//! values are not replicated.
//!
//! ```rust
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! use hydracache::{CacheEventOrigin, CacheOptions, HydraCache, InMemoryInvalidationBus};
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let bus = Arc::new(InMemoryInvalidationBus::default());
//! let first = HydraCache::local()
//!     .shared_invalidation_bus(bus.clone())
//!     .invalidation_node_id("first")
//!     .build();
//! let second = HydraCache::local()
//!     .shared_invalidation_bus(bus)
//!     .invalidation_node_id("second")
//!     .build();
//!
//! first
//!     .put("user:42", 42_u64, CacheOptions::new().tag("users"))
//!     .await?;
//! second
//!     .put("user:42", 42_u64, CacheOptions::new().tag("users"))
//!     .await?;
//!
//! let mut events = second.subscribe_tag("users");
//! first.invalidate_tag("users").await?;
//!
//! // Remote invalidation is applied by a background task, so applications that
//! // need to observe it immediately should wait on events or diagnostics.
//! let event = tokio::time::timeout(Duration::from_millis(500), events.recv())
//!     .await
//!     .expect("remote invalidation event")
//!     .expect("subscription stays open");
//!
//! assert_eq!(event.origin(), CacheEventOrigin::DistributedBus);
//! assert!(!second.contains_key("user:42").await);
//!
//! // Runtime counters expose the same path for diagnostics and metrics.
//! let _publisher_stats = first.stats();
//! let _receiver_stats = second.stats();
//! # Ok(())
//! # }
//! ```
//!
//! [`InMemoryFramedInvalidationBus`] is a transport spike for cross-process
//! adapters. It serializes each message into [`CacheInvalidationFrame`] bytes
//! before delivery, so tests can exercise the same encoding boundary future
//! TCP, Redis, NATS, or Postgres adapters will need.
//!
//! Custom transports implement [`CacheInvalidationBus`] and return
//! [`CacheInvalidationReceive::Message`], [`CacheInvalidationReceive::Lagged`],
//! [`CacheInvalidationReceive::DecodeError`], or
//! [`CacheInvalidationReceive::Closed`] from their receivers. HydraCache
//! records lag, decode errors, publish failures, and closed receivers in
//! [`hydracache_core::CacheStats`] so applications can detect bus health issues
//! without parsing logs.
//!
//! # Client and member cluster mode
//!
//! [`HydraCache::client`] creates an application-side near-cache. [`HydraCache::member`]
//! creates an in-process cluster member. In v0.20 both can share an
//! [`InMemoryCluster`] for tests, demos, and embedded applications while the
//! future discovery/Raft adapters are still being designed. Custom adapters can
//! implement [`ClusterDiscovery`] for discovery/liveness and
//! [`ClusterControlPlane`] for admission/metadata decisions.
//! [`ChitchatStyleDiscovery`] is a dependency-free seed-node discovery spike
//! that records chitchat-shaped candidates and liveness events without starting
//! a network runtime.
//! [`RaftStyleMetadataControlPlane`] is a dependency-free metadata-log spike
//! that records committed membership commands and snapshots without starting a
//! Raft runtime.
//!
//! ```rust
//! use std::sync::Arc;
//!
//! use hydracache::{
//!     CacheEventOrigin, CacheOptions, ClusterGeneration, HydraCache, InMemoryCluster,
//!     InMemoryClusterDiscovery,
//! };
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cluster = Arc::new(InMemoryCluster::new("orders-prod"));
//! let discovery = Arc::new(InMemoryClusterDiscovery::new());
//!
//! let member = HydraCache::member()
//!     .cluster("orders-prod")
//!     .shared_cluster(cluster.clone())
//!     .shared_discovery(discovery.clone())
//!     .node_id("member-a")
//!     .generation(ClusterGeneration::new(1))
//!     .bind("127.0.0.1:7000")
//!     .start()
//!     .await?;
//!
//! let client = HydraCache::client()
//!     .cluster("orders-prod")
//!     .shared_cluster(cluster)
//!     .shared_discovery(discovery.clone())
//!     .node_id("api-client-a")
//!     .bootstrap("127.0.0.1:7000")
//!     .connect()
//!     .await?;
//!
//! client
//!     .put("user:42", 42_u64, CacheOptions::new().tag("user:42"))
//!     .await?;
//!
//! let mut events = client.subscribe_tag("user:42");
//! member.invalidate_tag("user:42").await?;
//!
//! let event = events.recv().await.expect("subscription stays open");
//! assert_eq!(event.origin(), CacheEventOrigin::DistributedBus);
//! assert!(!client.contains_key("user:42").await);
//!
//! let diagnostics = client.cluster_diagnostics().expect("cluster runtime");
//! assert_eq!(diagnostics.member_count, 1);
//! assert_eq!(diagnostics.client_count, 1);
//! assert!(diagnostics.lifecycle.is_running());
//! assert_eq!(discovery.candidates().len(), 2);
//!
//! client.leave_cluster().await?;
//! assert!(client.cluster_diagnostics().unwrap().lifecycle.is_stopped());
//! # Ok(())
//! # }
//! ```
//!
//! # Observability
//!
//! Use [`HydraCache::diagnostics`] for quick local smoke checks. It combines
//! lightweight stats with the approximate local backend entry count.
//!
//! ```rust
//! use hydracache::{CacheOptions, HydraCache};
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cache = HydraCache::local().build();
//!
//! let first = cache
//!     .get_or_insert_with("answer", CacheOptions::new(), || async { 42_u64 })
//!     .await?;
//! let second = cache
//!     .get_or_insert_with("answer", CacheOptions::new(), || async { 7_u64 })
//!     .await?;
//!
//! let diagnostics = cache.diagnostics().await;
//! assert_eq!((first, second), (42, 42));
//! assert_eq!(diagnostics.stats.loads, 1);
//! assert_eq!(diagnostics.stats.hits, 1);
//! assert_eq!(diagnostics.hit_ratio(), Some(0.5));
//! # Ok(())
//! # }
//! ```

extern crate self as hydracache;

mod builder;
mod cache;
mod cluster;
mod consistency;
mod entry;
mod events;
mod grid;
mod inflight;
mod invalidation_bus;
mod refresh;
mod stats;
mod tag_index;
pub mod testing;
mod typed;

pub use builder::HydraCacheBuilder;
pub use cache::HydraCache;
pub use cluster::{
    partition_for_key, validate_replica_config, ChitchatStyleDiscovery, ClusterAdmissionBridge,
    ClusterAdmissionBridgeConfig, ClusterAdmissionBridgeDiagnostics, ClusterAdmissionBridgeEvent,
    ClusterAdmissionBridgeHandle, ClusterAdmissionIgnoreReason, ClusterAdmissionRejectReason,
    ClusterCacheCounters, ClusterCandidate, ClusterComponent, ClusterComponentError,
    ClusterControlPlane, ClusterDiagnostics, ClusterDiscovery, ClusterDiscoveryDiagnostics,
    ClusterDiscoveryEvent, ClusterEndpoints, ClusterEpoch, ClusterFillCounters, ClusterGeneration,
    ClusterHealthReason, ClusterHealthState, ClusterLifecycleComponent,
    ClusterLifecycleDiagnostics, ClusterLifecycleStatus, ClusterLoadReport, ClusterMember,
    ClusterMembershipEvent, ClusterMembershipRecvError, ClusterMembershipSubscriber, ClusterNodeId,
    ClusterOwnershipDecision, ClusterOwnershipDiagnostics, ClusterOwnershipResolver,
    ClusterPeerFetch, ClusterPeerFetchDiagnostics, ClusterPeerFetchGenerationMismatch,
    ClusterPeerFetchRequest, ClusterPeerFetchResponse, ClusterPilotReadiness, ClusterPilotReport,
    ClusterReplicaConfigError, ClusterRole, ClusterStagingCounters, ClusterStagingHealth,
    HydraCacheClientBuilder, HydraCacheMemberBuilder, InMemoryCluster, InMemoryClusterDiscovery,
    InMemoryPeerFetch, MetaDataContainer, NearCacheRepairAction, PartitionId, RaftMetadataCommand,
    RaftMetadataSnapshot, RaftStyleMetadataControlPlane, RendezvousClusterOwnership, RoutingMode,
    TopologyFence, TransportPosture, CLUSTER_PEER_FETCH_BASE_URL_METADATA_KEY,
};
pub use consistency::{
    ConsistencyInvalidate, ConsistencyMode, ConsistencyOutcome, ConsistencyToken, DegradeReason,
    WriteBarrierToken,
};
pub use events::{CacheEventListenerHandle, CacheEventRecvError, CacheEventSubscriber};
pub use grid::{
    cluster_grid_metric_descriptors, diff_effective_maps, prepare_replicated_payload,
    replicated_slot_version, select_backup_promotion, AntiEntropyTask, BackupPromotion,
    ClusterGridCounters, ClusterGridDiagnostics, ClusterMetricDescriptor,
    ClusterReplicationStrategy, EffectiveReplicationMap, HotCacheDirectory,
    PartitionReplicaVersions, PromotionPhase, RebalancePlan, RebalanceTask, RebalanceTaskAck,
    RedactReplicatedValue, RepairingTask, Replicas, ReplicatedSlot, ReplicatedValueSecurityPosture,
    Replication, ReplicationConfig, ReplicationConfigError, ReplicationCryptoError,
    ReplicationKeyProvider, ReplicationPayload, SharedReplicationKeyProvider, TombstoneAdmission,
    TombstoneBudget, TombstoneTracker,
};
pub use hydracache_core::{
    CacheDiagnostics, CacheError, CacheEvent, CacheEventKind, CacheEventOptions, CacheEventOrigin,
    CacheEventScope, CacheEventValueMode, CacheKey, CacheKeyBuilder, CacheOptions, CacheStats,
    PostcardCodec, TagSet,
};
pub use hydracache_macros::{cacheable, cacheable_infallible, cacheable_loader};
pub use invalidation_bus::{
    CacheInvalidation, CacheInvalidationBus, CacheInvalidationFrame, CacheInvalidationMessage,
    CacheInvalidationReceive, CacheInvalidationReceiver, InMemoryFramedInvalidationBus,
    InMemoryInvalidationBus, CACHE_INVALIDATION_FRAME_VERSION,
};
pub use refresh::RefreshOptions;
pub use typed::TypedCache;

pub use hydracache_core::{
    CacheDiagnostics as Diagnostics, CacheOptions as Options, CacheStats as Stats,
    Result as CacheResult,
};

#[cfg(test)]
mod tests;
