use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache_core::{CacheCodec, PostcardCodec};
use moka::future::Cache;
use tokio::sync::watch;

use crate::cache::{HydraCache, HydraCacheInner};
use crate::cluster::{ClusterRuntime, RoutingMode, TransportPosture};
use crate::entry::CacheEntry;
use crate::events::EventBus;
use crate::inflight::InFlightMap;
use crate::invalidation_bus::CacheInvalidationBus;
use crate::stats::StatsCounters;
use crate::tag_index::TagIndex;

static NEXT_INVALIDATION_NODE_ID: AtomicU64 = AtomicU64::new(1);

fn next_invalidation_node_id() -> String {
    let id = NEXT_INVALIDATION_NODE_ID.fetch_add(1, Ordering::Relaxed);
    format!("hydracache-node-{id}")
}

/// Builder for a local [`HydraCache`] instance.
///
/// Use [`HydraCache::local`] to create a builder with sensible defaults.
///
/// # Example
///
/// ```rust
/// use std::time::Duration;
///
/// use hydracache::HydraCache;
///
/// let cache = HydraCache::local()
///     .max_capacity(50_000)
///     .default_ttl(Duration::from_secs(60))
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct HydraCacheBuilder<C = PostcardCodec>
where
    C: CacheCodec,
{
    max_capacity: u64,
    max_entry_bytes: usize,
    default_ttl: Duration,
    event_buffer_capacity: usize,
    access_events: bool,
    invalidation_bus: Option<Arc<dyn CacheInvalidationBus>>,
    invalidation_node_id: Option<String>,
    cluster_runtime: Option<ClusterRuntime>,
    transport_posture: TransportPosture,
    routing_mode: RoutingMode,
    read_through_enabled: bool,
    codec: C,
}

impl<C> HydraCacheBuilder<C>
where
    C: CacheCodec,
{
    /// Set the maximum weighted capacity used by the Moka backend.
    ///
    /// Entry weight is based on encoded byte length and is capped by
    /// `max_entry_bytes`.
    pub fn max_capacity(mut self, max_capacity: u64) -> Self {
        self.max_capacity = max_capacity.max(1);
        self
    }

    /// Set the maximum accepted encoded entry size in bytes.
    pub fn max_entry_bytes(mut self, max_entry_bytes: usize) -> Self {
        self.max_entry_bytes = max_entry_bytes.max(1);
        self
    }

    /// Set the default TTL used when [`hydracache_core::CacheOptions`] does not specify one.
    pub fn default_ttl(mut self, default_ttl: Duration) -> Self {
        self.default_ttl = default_ttl;
        self
    }

    /// Set the bounded cache event buffer capacity.
    ///
    /// Slow subscribers may observe lag when more than this many events are
    /// published before they receive them.
    pub fn event_buffer_capacity(mut self, capacity: usize) -> Self {
        self.event_buffer_capacity = capacity.max(1);
        self
    }

    /// Enable high-volume hit/miss/load events.
    ///
    /// Mutation and invalidation events are always published when subscribers
    /// exist. Access events are opt-in because they can be very noisy.
    pub fn enable_access_events(mut self, enabled: bool) -> Self {
        self.access_events = enabled;
        self
    }

    /// Attach a shared invalidation bus to this cache.
    ///
    /// Caches that share the same bus propagate `invalidate_key`,
    /// `invalidate_tag`, `remove`, and `flush` operations to each other. Values
    /// are not replicated. Building a cache with a bus requires an active Tokio
    /// runtime because HydraCache starts a lightweight background receiver task.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use hydracache::{HydraCache, InMemoryInvalidationBus};
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let bus = Arc::new(InMemoryInvalidationBus::default());
    /// let first = HydraCache::local()
    ///     .shared_invalidation_bus(bus.clone())
    ///     .invalidation_node_id("first")
    ///     .build();
    /// let second = HydraCache::local()
    ///     .shared_invalidation_bus(bus)
    ///     .invalidation_node_id("second")
    ///     .build();
    /// # let _ = (first, second);
    /// # }
    /// ```
    pub fn shared_invalidation_bus(mut self, bus: Arc<dyn CacheInvalidationBus>) -> Self {
        self.invalidation_bus = Some(bus);
        self
    }

    /// Attach an owned invalidation bus to this cache.
    ///
    /// Use [`shared_invalidation_bus`](Self::shared_invalidation_bus) when two
    /// or more caches should communicate through the same bus instance.
    pub fn invalidation_bus<B>(self, bus: B) -> Self
    where
        B: CacheInvalidationBus,
    {
        self.shared_invalidation_bus(Arc::new(bus))
    }

    /// Set a stable node id used to suppress self-originated invalidations.
    ///
    /// A generated id is used by default. Supplying an explicit id is useful for
    /// tests, sandbox demos, and future external transports where observability
    /// should show human-readable node names.
    pub fn invalidation_node_id(mut self, node_id: impl Into<String>) -> Self {
        self.invalidation_node_id = Some(node_id.into());
        self
    }

    /// Replace the default codec.
    ///
    /// Most applications can use the default [`PostcardCodec`].
    pub fn codec<Next>(self, codec: Next) -> HydraCacheBuilder<Next>
    where
        Next: CacheCodec,
    {
        HydraCacheBuilder {
            max_capacity: self.max_capacity,
            max_entry_bytes: self.max_entry_bytes,
            default_ttl: self.default_ttl,
            event_buffer_capacity: self.event_buffer_capacity,
            access_events: self.access_events,
            invalidation_bus: self.invalidation_bus,
            invalidation_node_id: self.invalidation_node_id,
            cluster_runtime: self.cluster_runtime,
            transport_posture: self.transport_posture,
            routing_mode: self.routing_mode,
            read_through_enabled: self.read_through_enabled,
            codec,
        }
    }

    pub(crate) fn cluster_runtime(mut self, runtime: ClusterRuntime) -> Self {
        self.cluster_runtime = Some(runtime);
        self
    }

    /// Declare whether HydraCache transport auth is configured for this cache.
    pub fn transport_auth_configured(mut self, enabled: bool) -> Self {
        self.transport_posture.auth = enabled;
        self
    }

    /// Declare whether strict current wire compatibility is configured.
    pub fn strict_wire_compatibility(mut self, enabled: bool) -> Self {
        self.transport_posture.wire_strict = enabled;
        self
    }

    /// Declare that an external mesh/mTLS boundary handles transport identity.
    pub fn declare_mesh_boundary(mut self, enabled: bool) -> Self {
        self.transport_posture.mesh_declared = enabled;
        self
    }

    /// Set the pilot routing mode used for diagnostics and routed reads.
    pub fn routing_mode(mut self, routing_mode: RoutingMode) -> Self {
        self.routing_mode = routing_mode;
        self
    }

    /// Enable or disable cluster read-through/remote peer-fetch paths.
    pub fn read_through_enabled(mut self, enabled: bool) -> Self {
        self.read_through_enabled = enabled;
        self
    }

    /// Build the local cache.
    pub fn build(self) -> HydraCache<C> {
        let max_entry_bytes = self.max_entry_bytes;
        let store = Cache::builder()
            .max_capacity(self.max_capacity)
            .weigher(move |_key, entry: &CacheEntry| {
                entry.value.len().min(max_entry_bytes).max(1) as u32
            })
            .build();

        let invalidation_node_id = self
            .invalidation_node_id
            .unwrap_or_else(next_invalidation_node_id);
        let (bus_shutdown, bus_shutdown_rx) = self
            .invalidation_bus
            .as_ref()
            .map(|_| watch::channel(false))
            .map_or((None, None), |(sender, receiver)| {
                (Some(sender), Some(receiver))
            });

        let cache = HydraCache {
            inner: Arc::new(HydraCacheInner {
                store,
                tag_index: TagIndex::default(),
                in_flight: InFlightMap::default(),
                codec: self.codec,
                default_ttl: self.default_ttl,
                max_entry_bytes,
                stats: Arc::new(StatsCounters::default()),
                events: EventBus::new(self.event_buffer_capacity, self.access_events),
                invalidation_bus: self.invalidation_bus,
                invalidation_node_id,
                consistency_generation: AtomicU64::new(0),
                bus_shutdown,
                cluster_runtime: self.cluster_runtime,
                transport_posture: self.transport_posture,
                routing_mode: self.routing_mode,
                read_through_enabled: self.read_through_enabled,
            }),
        };

        if let Some(shutdown) = bus_shutdown_rx {
            cache.spawn_invalidation_listener(shutdown);
        }

        cache
    }
}

impl Default for HydraCacheBuilder<PostcardCodec> {
    fn default() -> Self {
        Self {
            max_capacity: 10_000,
            max_entry_bytes: 16 * 1024 * 1024,
            default_ttl: Duration::from_secs(300),
            event_buffer_capacity: 1024,
            access_events: false,
            invalidation_bus: None,
            invalidation_node_id: None,
            cluster_runtime: None,
            transport_posture: TransportPosture::default(),
            routing_mode: RoutingMode::default(),
            read_through_enabled: true,
            codec: PostcardCodec,
        }
    }
}
