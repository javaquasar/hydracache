//! User-facing HydraCache local runtime.
//!
//! v0 is intentionally local-only: no SQLx adapter, no distributed coordination,
//! and no single-flight. The goal is a small async cache with TTL, tags, and
//! pleasant `get_or_load` ergonomics.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use hydracache_core::{CacheCodec, CacheOptions, CacheStats, Result};
pub use hydracache_core::{CacheError, CacheKey, PostcardCodec};
use moka::future::Cache;
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::RwLock;

/// Local async cache runtime.
#[derive(Debug, Clone)]
pub struct HydraCache<C = PostcardCodec>
where
    C: CacheCodec,
{
    inner: Arc<HydraCacheInner<C>>,
}

#[derive(Debug)]
struct HydraCacheInner<C>
where
    C: CacheCodec,
{
    store: Cache<String, CacheEntry>,
    tag_index: TagIndex,
    codec: C,
    default_ttl: Duration,
    stats: StatsCounters,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    value: Bytes,
    tags: Vec<String>,
    expires_at: Option<Instant>,
}

/// Builder for a local HydraCache instance.
#[derive(Debug, Clone)]
pub struct HydraCacheBuilder<C = PostcardCodec>
where
    C: CacheCodec,
{
    max_capacity: u64,
    max_entry_bytes: usize,
    default_ttl: Duration,
    codec: C,
}

impl HydraCache<PostcardCodec> {
    /// Start building a local cache.
    pub fn local() -> HydraCacheBuilder<PostcardCodec> {
        HydraCacheBuilder::default()
    }
}

impl<C> HydraCacheBuilder<C>
where
    C: CacheCodec,
{
    /// Set the maximum weighted capacity used by the Moka backend.
    pub fn max_capacity(mut self, max_capacity: u64) -> Self {
        self.max_capacity = max_capacity.max(1);
        self
    }

    /// Set the maximum accepted encoded entry size in bytes.
    pub fn max_entry_bytes(mut self, max_entry_bytes: usize) -> Self {
        self.max_entry_bytes = max_entry_bytes.max(1);
        self
    }

    /// Set the default TTL used when `CacheOptions` does not specify one.
    pub fn default_ttl(mut self, default_ttl: Duration) -> Self {
        self.default_ttl = default_ttl;
        self
    }

    /// Replace the default codec.
    pub fn codec<Next>(self, codec: Next) -> HydraCacheBuilder<Next>
    where
        Next: CacheCodec,
    {
        HydraCacheBuilder {
            max_capacity: self.max_capacity,
            max_entry_bytes: self.max_entry_bytes,
            default_ttl: self.default_ttl,
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
                codec: self.codec,
                default_ttl: self.default_ttl,
                stats: StatsCounters::default(),
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
            codec: PostcardCodec,
        }
    }
}

impl<C> HydraCache<C>
where
    C: CacheCodec,
{
    /// Get and decode a cached value.
    pub async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        match self.inner.store.get(key).await {
            Some(entry) if entry.is_expired() => {
                self.remove_expired(key, &entry).await;
                self.inner.stats.misses.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            }
            Some(entry) => match self.inner.codec.decode::<T>(&entry.value) {
                Ok(value) => {
                    self.inner.stats.hits.fetch_add(1, Ordering::Relaxed);
                    Ok(Some(value))
                }
                Err(error) => {
                    self.remove_entry(key, &entry).await;
                    self.inner.stats.misses.fetch_add(1, Ordering::Relaxed);
                    Err(error)
                }
            },
            None => {
                self.inner.stats.misses.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            }
        }
    }

    /// Encode and store a value.
    pub async fn put<T>(&self, key: &str, value: T, options: CacheOptions) -> Result<()>
    where
        T: Serialize,
    {
        let bytes = self.inner.codec.encode(&value)?;
        self.put_bytes(key, bytes, options).await
    }

    /// Get a value, or run the loader and cache its result on miss.
    ///
    /// v0 does not deduplicate concurrent misses. If multiple callers miss the
    /// same key at the same time, each caller may run its own loader.
    pub async fn get_or_load<T, E, F, Fut>(
        &self,
        key: &str,
        options: CacheOptions,
        loader: F,
    ) -> Result<T>
    where
        T: Serialize + DeserializeOwned,
        E: Error + Send + Sync + 'static,
        F: FnOnce() -> Fut,
        Fut: Future<Output = std::result::Result<T, E>>,
    {
        if let Some(value) = self.get(key).await? {
            return Ok(value);
        }

        self.inner.stats.loads.fetch_add(1, Ordering::Relaxed);
        let value = loader().await.map_err(CacheError::loader)?;
        self.put(key, &value, options).await?;
        Ok(value)
    }

    /// Remove one key from the cache.
    pub async fn invalidate_key(&self, key: &str) -> Result<bool> {
        let Some(entry) = self.inner.store.get(key).await else {
            return Ok(false);
        };

        self.remove_entry(key, &entry).await;
        self.inner
            .stats
            .invalidations
            .fetch_add(1, Ordering::Relaxed);
        Ok(true)
    }

    /// Remove all entries currently associated with a tag.
    pub async fn invalidate_tag(&self, tag: &str) -> Result<u64> {
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

        Ok(removed)
    }

    /// Remove all cached entries and tag mappings.
    pub async fn flush(&self) -> Result<()> {
        self.inner.store.invalidate_all();
        self.inner.tag_index.clear().await;
        Ok(())
    }

    /// Return a snapshot of lightweight cache counters.
    pub fn stats(&self) -> CacheStats {
        self.inner.stats.snapshot()
    }

    async fn put_bytes(&self, key: &str, value: Bytes, options: CacheOptions) -> Result<()> {
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
        Ok(())
    }

    async fn remove_expired(&self, key: &str, entry: &CacheEntry) {
        self.remove_entry(key, entry).await;
    }

    async fn remove_entry(&self, key: &str, entry: &CacheEntry) {
        self.inner.store.invalidate(key).await;
        self.inner.tag_index.unregister(key, &entry.tags).await;
    }
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.expires_at
            .map(|expires_at| Instant::now() >= expires_at)
            .unwrap_or(false)
    }
}

#[derive(Debug, Default)]
struct TagIndex {
    tags: RwLock<HashMap<String, HashSet<String>>>,
}

impl TagIndex {
    async fn register(&self, key: &str, tags: &[String]) {
        if tags.is_empty() {
            return;
        }

        let mut guard = self.tags.write().await;
        for tag in tags {
            guard.entry(tag.clone()).or_default().insert(key.to_owned());
        }
    }

    async fn unregister(&self, key: &str, tags: &[String]) {
        if tags.is_empty() {
            return;
        }

        let mut guard = self.tags.write().await;
        for tag in tags {
            if let Some(keys) = guard.get_mut(tag) {
                keys.remove(key);
                if keys.is_empty() {
                    guard.remove(tag);
                }
            }
        }
    }

    async fn take_tag(&self, tag: &str) -> Vec<String> {
        self.tags
            .write()
            .await
            .remove(tag)
            .map(|keys| keys.into_iter().collect())
            .unwrap_or_default()
    }

    async fn clear(&self) {
        self.tags.write().await.clear();
    }
}

#[derive(Debug, Default)]
struct StatsCounters {
    hits: AtomicU64,
    misses: AtomicU64,
    loads: AtomicU64,
    invalidations: AtomicU64,
    evictions: AtomicU64,
}

impl StatsCounters {
    fn snapshot(&self) -> CacheStats {
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            loads: self.loads.load(Ordering::Relaxed),
            invalidations: self.invalidations.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
        }
    }
}

pub use hydracache_core::{CacheOptions as Options, CacheStats as Stats, Result as CacheResult};

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::fmt;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct User {
        id: u64,
        name: String,
    }

    #[derive(Debug)]
    struct LoaderError;

    impl fmt::Display for LoaderError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("loader failed")
        }
    }

    impl Error for LoaderError {}

    fn user(id: u64) -> User {
        User {
            id,
            name: format!("user-{id}"),
        }
    }

    #[tokio::test]
    async fn put_then_get() {
        let cache = HydraCache::local().build();

        cache
            .put("user:1", user(1), CacheOptions::new())
            .await
            .unwrap();

        let cached: Option<User> = cache.get("user:1").await.unwrap();
        assert_eq!(cached, Some(user(1)));
    }

    #[tokio::test]
    async fn get_missing_returns_none() {
        let cache = HydraCache::local().build();
        let cached: Option<User> = cache.get("missing").await.unwrap();
        assert_eq!(cached, None);
    }

    #[tokio::test]
    async fn get_or_load_loads_on_miss() {
        let cache = HydraCache::local().build();

        let loaded = cache
            .get_or_load("user:1", CacheOptions::new(), || async {
                Ok::<_, LoaderError>(user(1))
            })
            .await
            .unwrap();

        assert_eq!(loaded, user(1));
        assert_eq!(cache.stats().loads, 1);
    }

    #[tokio::test]
    async fn get_or_load_uses_cached_value_on_hit() {
        let cache = HydraCache::local().build();

        cache
            .put("user:1", user(1), CacheOptions::new())
            .await
            .unwrap();

        let loaded = cache
            .get_or_load("user:1", CacheOptions::new(), || async {
                Ok::<_, LoaderError>(user(2))
            })
            .await
            .unwrap();

        assert_eq!(loaded, user(1));
        assert_eq!(cache.stats().loads, 0);
    }

    #[tokio::test]
    async fn ttl_expires_entry() {
        let cache = HydraCache::local()
            .default_ttl(Duration::from_millis(20))
            .build();

        cache
            .put("user:1", user(1), CacheOptions::new())
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(40)).await;

        let cached: Option<User> = cache.get("user:1").await.unwrap();
        assert_eq!(cached, None);
    }

    #[tokio::test]
    async fn invalidate_key_removes_one() {
        let cache = HydraCache::local().build();
        cache
            .put("user:1", user(1), CacheOptions::new())
            .await
            .unwrap();

        assert!(cache.invalidate_key("user:1").await.unwrap());
        let cached: Option<User> = cache.get("user:1").await.unwrap();
        assert_eq!(cached, None);
    }

    #[tokio::test]
    async fn invalidate_tag_removes_all_tagged() {
        let cache = HydraCache::local().build();
        let tagged = CacheOptions::new().tags(["users"]);

        cache.put("user:1", user(1), tagged.clone()).await.unwrap();
        cache.put("user:2", user(2), tagged).await.unwrap();
        cache
            .put("order:1", user(3), CacheOptions::new())
            .await
            .unwrap();

        assert_eq!(cache.invalidate_tag("users").await.unwrap(), 2);

        let user_1: Option<User> = cache.get("user:1").await.unwrap();
        let user_2: Option<User> = cache.get("user:2").await.unwrap();
        let order_1: Option<User> = cache.get("order:1").await.unwrap();
        assert_eq!(user_1, None);
        assert_eq!(user_2, None);
        assert_eq!(order_1, Some(user(3)));
    }

    #[tokio::test]
    async fn flush_clears_all() {
        let cache = HydraCache::local().build();
        cache
            .put("user:1", user(1), CacheOptions::new())
            .await
            .unwrap();
        cache.flush().await.unwrap();

        let cached: Option<User> = cache.get("user:1").await.unwrap();
        assert_eq!(cached, None);
    }

    #[tokio::test]
    async fn stats_track_hits_misses_loads_invalidations() {
        let cache = HydraCache::local().build();

        let _: Option<User> = cache.get("user:1").await.unwrap();
        cache
            .get_or_load("user:1", CacheOptions::new().tags(["users"]), || async {
                Ok::<_, LoaderError>(user(1))
            })
            .await
            .unwrap();
        let _: Option<User> = cache.get("user:1").await.unwrap();
        cache.invalidate_tag("users").await.unwrap();

        let stats = cache.stats();
        assert_eq!(stats.misses, 2);
        assert_eq!(stats.loads, 1);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.invalidations, 1);
    }

    #[tokio::test]
    async fn loader_error_is_returned() {
        let cache = HydraCache::local().build();

        let result = cache
            .get_or_load("user:1", CacheOptions::new(), || async {
                Err::<User, _>(LoaderError)
            })
            .await;

        assert!(matches!(result, Err(CacheError::Loader(_))));
    }

    #[tokio::test]
    async fn decode_error_invalidates_bad_entry() {
        let cache = HydraCache::local().build();

        cache
            .put_bytes(
                "user:1",
                Bytes::from_static(&[0xff, 0xff, 0xff]),
                CacheOptions::new(),
            )
            .await
            .unwrap();

        let result: CacheResult<Option<User>> = cache.get("user:1").await;
        assert!(matches!(result, Err(CacheError::Decode(_))));

        let cached: Option<User> = cache.get("user:1").await.unwrap();
        assert_eq!(cached, None);
    }
}
