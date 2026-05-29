use std::error::Error;
use std::future::Future;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use futures_util::FutureExt;
use hydracache_core::{CacheCodec, CacheError, CacheOptions, CacheStats, PostcardCodec, Result};
use moka::future::Cache;
use serde::{de::DeserializeOwned, Serialize};

use crate::builder::HydraCacheBuilder;
use crate::entry::CacheEntry;
use crate::inflight::{InFlightMap, SharedLoadFuture};
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
    pub(crate) stats: StatsCounters,
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
        self.remove(key).await
    }

    /// Remove one key from the cache.
    ///
    /// This is an alias for `invalidate_key` with a shorter name for local-cache use.
    pub async fn remove(&self, key: &str) -> Result<bool> {
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

    /// Return whether the key currently maps to a usable value.
    ///
    /// Expired entries are removed and reported as absent.
    pub async fn contains_key(&self, key: &str) -> bool {
        match self.inner.store.get(key).await {
            Some(entry) if entry.is_expired() => {
                self.remove_entry(key, &entry).await;
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

    pub(crate) async fn put_bytes(
        &self,
        key: &str,
        value: Bytes,
        options: CacheOptions,
    ) -> Result<()> {
        self.put_bytes_unchecked(key, value, options).await
    }

    async fn put_bytes_unchecked(
        &self,
        key: &str,
        value: Bytes,
        options: CacheOptions,
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
            return Ok(false);
        }

        self.put_bytes_unchecked(key, value, options).await?;
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
            return shared;
        }

        let key_owned = key.to_owned();
        let cache = self.clone();
        let load_key = key_owned.clone();
        let load_generation = generation.clone();
        let shared = async move {
            let result = async {
                let bytes = loader(cache.clone()).await?;
                cache
                    .put_bytes_if_fresh(&load_key, bytes.clone(), options, &load_generation)
                    .await?;
                Ok(bytes)
            }
            .await
            .map_err(Arc::new);

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
        }

        shared
    }

    async fn remove_expired(&self, key: &str, entry: &CacheEntry) {
        self.remove_entry(key, entry).await;
    }

    async fn remove_entry(&self, key: &str, entry: &CacheEntry) {
        self.inner.store.invalidate(key).await;
        self.inner.tag_index.unregister(key, &entry.tags).await;
    }
}
