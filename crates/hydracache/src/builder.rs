use std::sync::Arc;
use std::time::Duration;

use hydracache_core::{CacheCodec, PostcardCodec};
use moka::future::Cache;

use crate::cache::{HydraCache, HydraCacheInner};
use crate::entry::CacheEntry;
use crate::events::EventBus;
use crate::inflight::InFlightMap;
use crate::stats::StatsCounters;
use crate::tag_index::TagIndex;

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
            codec,
        }
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

        HydraCache {
            inner: Arc::new(HydraCacheInner {
                store,
                tag_index: TagIndex::default(),
                in_flight: InFlightMap::default(),
                codec: self.codec,
                default_ttl: self.default_ttl,
                stats: Arc::new(StatsCounters::default()),
                events: EventBus::new(self.event_buffer_capacity, self.access_events),
            }),
        }
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
            codec: PostcardCodec,
        }
    }
}
