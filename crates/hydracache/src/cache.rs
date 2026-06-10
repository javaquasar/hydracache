use std::error::Error;
use std::future::Future;
use std::sync::atomic::Ordering;
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
    ClusterDiagnostics, ClusterRuntime, HydraCacheClientBuilder, HydraCacheMemberBuilder,
};
use crate::entry::CacheEntry;
use crate::events::{CacheEventListenerHandle, CacheEventSubscriber, EventBus};
use crate::inflight::{InFlightMap, SharedLoadFuture};
use crate::invalidation_bus::{
    CacheInvalidation, CacheInvalidationBus, CacheInvalidationMessage, CacheInvalidationReceive,
};
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
    pub(crate) stats: Arc<StatsCounters>,
    pub(crate) events: EventBus,
    pub(crate) invalidation_bus: Option<Arc<dyn CacheInvalidationBus>>,
    pub(crate) invalidation_node_id: String,
    pub(crate) bus_shutdown: Option<watch::Sender<bool>>,
    pub(crate) cluster_runtime: Option<ClusterRuntime>,
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
    pub fn cluster_diagnostics(&self) -> Option<ClusterDiagnostics> {
        self.inner
            .cluster_runtime
            .as_ref()
            .map(ClusterRuntime::diagnostics)
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
                self.publish_key_event(
                    CacheEventKind::Miss,
                    key,
                    CacheEventOrigin::LocalApi,
                    entry.tags.clone(),
                );
                Ok(None)
            }
            Some(entry) => match self.inner.codec.decode::<T>(&entry.value) {
                Ok(value) => {
                    self.inner.stats.hits.fetch_add(1, Ordering::Relaxed);
                    self.publish_key_event(
                        CacheEventKind::Hit,
                        key,
                        CacheEventOrigin::LocalApi,
                        entry.tags.clone(),
                    );
                    Ok(Some(value))
                }
                Err(error) => {
                    self.remove_entry(key, &entry).await;
                    self.inner.stats.misses.fetch_add(1, Ordering::Relaxed);
                    self.publish_key_event(
                        CacheEventKind::Miss,
                        key,
                        CacheEventOrigin::LocalApi,
                        entry.tags.clone(),
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

        self.publish_event(CacheEvent::for_tag(
            CacheEventKind::TagInvalidated,
            tag,
            removed,
            origin,
        ));

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
        self.publish_event(CacheEvent::for_cache(
            CacheEventKind::Flushed,
            Some(estimated_entries),
            origin,
        ));
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
        if !self.inner.tag_index.is_current(generation).await {
            self.inner
                .stats
                .stale_load_discards
                .fetch_add(1, Ordering::Relaxed);
            self.publish_key_event(
                CacheEventKind::StaleLoadDiscarded,
                key,
                CacheEventOrigin::Loader,
                options.tags_value().to_vec(),
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
        let event_tags = options.tags_value().to_vec();
        let late_join_event_tags = event_tags.clone();

        if let Some(shared) = self.inner.in_flight.get_current(key, &generation).await {
            self.inner
                .stats
                .single_flight_joins
                .fetch_add(1, Ordering::Relaxed);
            self.publish_key_event(
                CacheEventKind::SingleFlightJoined,
                key,
                CacheEventOrigin::SingleFlight,
                event_tags,
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
        let load_event_tags = event_tags.clone();
        let shared = async move {
            let result = async {
                cache.publish_key_event(
                    CacheEventKind::LoadStarted,
                    &load_key,
                    CacheEventOrigin::Loader,
                    load_event_tags.clone(),
                );
                let bytes = loader(cache.clone()).await?;
                let accepted = cache
                    .put_bytes_if_fresh(&load_key, bytes.clone(), options, &load_generation)
                    .await?;
                if accepted {
                    cache.publish_key_event(
                        CacheEventKind::LoadCompleted,
                        &load_key,
                        CacheEventOrigin::Loader,
                        load_event_tags.clone(),
                    );
                }
                Ok(bytes)
            }
            .await;

            if result.is_err() {
                cache.publish_key_event(
                    CacheEventKind::LoadFailed,
                    &load_key,
                    CacheEventOrigin::Loader,
                    load_event_tags,
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
            self.publish_key_event(
                CacheEventKind::SingleFlightJoined,
                key,
                CacheEventOrigin::SingleFlight,
                late_join_event_tags,
            );
        }

        shared
    }

    async fn remove_expired(&self, key: &str, entry: &CacheEntry) {
        self.remove_entry(key, entry).await;
        self.publish_key_event(
            CacheEventKind::Expired,
            key,
            CacheEventOrigin::Backend,
            entry.tags.clone(),
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
        self.publish_key_event(kind, key, origin, entry.tags.clone());
        Ok(true)
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
        self.publish_event(CacheEvent::for_key(kind, key, origin, tags));
    }

    fn publish_event(&self, event: CacheEvent) {
        self.inner.events.publish(event, &self.inner.stats);
    }

    async fn publish_invalidation(&self, invalidation: CacheInvalidation) -> Result<()> {
        let Some(bus) = &self.inner.invalidation_bus else {
            return Ok(());
        };

        if let Err(error) = bus
            .publish(CacheInvalidationMessage::new(
                self.inner.invalidation_node_id.clone(),
                invalidation,
            ))
            .await
        {
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
