use std::error::Error;
use std::future::Future;
use std::marker::PhantomData;

use hydracache_core::{
    CacheCodec, CacheDiagnostics, CacheEvent, CacheEventOptions, CacheKeyBuilder, CacheOptions,
    CacheStats, PostcardCodec, Result,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::cache::HydraCache;
use crate::events::{CacheEventListenerHandle, CacheEventSubscriber};

/// A typed, namespaced view over a [`HydraCache`].
///
/// `TypedCache` does not create a separate storage backend. It prefixes keys and
/// delegates to the same underlying cache, preserving single-flight, TTL, tags,
/// stats, and invalidation safety.
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
///
/// assert_eq!(users.key("1"), "users:1");
/// assert_eq!(users.get("1").await?, Some(User { id: 1 }));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct TypedCache<T, C = PostcardCodec>
where
    C: CacheCodec,
{
    cache: HydraCache<C>,
    namespace: String,
    _value: PhantomData<fn() -> T>,
}

impl<T, C> TypedCache<T, C>
where
    C: CacheCodec,
{
    pub(crate) fn new(cache: HydraCache<C>, namespace: String) -> Self {
        Self {
            cache,
            namespace,
            _value: PhantomData,
        }
    }

    /// Return this typed cache namespace.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Subscribe to shared cache events.
    ///
    /// The subscription observes the underlying physical keys. Use
    /// [`CacheEventOptions::key_prefix`] with this typed cache namespace when
    /// you want events for only this typed view.
    pub fn subscribe(&self, options: CacheEventOptions) -> CacheEventSubscriber {
        self.cache.subscribe(options)
    }

    /// Subscribe to mutation and invalidation events for this namespace.
    pub fn subscribe_mutations(&self) -> CacheEventSubscriber {
        self.subscribe(CacheEventOptions::mutations().key_prefix(self.namespace_prefix()))
    }

    /// Subscribe to access and loader events for this namespace.
    ///
    /// These events are published only when the shared cache is built with
    /// `enable_access_events(true)`.
    pub fn subscribe_access(&self) -> CacheEventSubscriber {
        self.subscribe(CacheEventOptions::access().key_prefix(self.namespace_prefix()))
    }

    /// Subscribe to key-scoped events for this typed namespace.
    pub fn subscribe_namespace(&self) -> CacheEventSubscriber {
        self.subscribe(CacheEventOptions::new().key_prefix(self.namespace_prefix()))
    }

    /// Subscribe to one logical typed key.
    pub fn subscribe_key(&self, key: &str) -> CacheEventSubscriber {
        self.subscribe(CacheEventOptions::new().key(self.key(key)))
    }

    /// Subscribe to events associated with one tag.
    pub fn subscribe_tag(&self, tag: impl Into<String>) -> CacheEventSubscriber {
        self.cache.subscribe_tag(tag)
    }

    /// Run a callback for events matching the provided filters.
    pub fn add_listener<F>(
        &self,
        options: CacheEventOptions,
        listener: F,
    ) -> CacheEventListenerHandle
    where
        F: Fn(CacheEvent) + Send + 'static,
    {
        self.cache.add_listener(options, listener)
    }

    /// Run a callback for mutation events scoped to this namespace.
    pub fn on_mutation<F>(&self, listener: F) -> CacheEventListenerHandle
    where
        F: Fn(CacheEvent) + Send + 'static,
    {
        self.add_listener(
            CacheEventOptions::mutations().key_prefix(self.namespace_prefix()),
            listener,
        )
    }

    /// Build the physical key used by the shared underlying cache.
    pub fn key(&self, key: &str) -> String {
        format!("{}:{key}", self.namespace)
    }

    fn namespace_prefix(&self) -> String {
        format!("{}:", self.namespace)
    }

    /// Build a physical key from escaped key segments inside this namespace.
    pub fn key_from(&self, builder: CacheKeyBuilder) -> String {
        let key = builder.build_string();
        if key.is_empty() {
            self.namespace.clone()
        } else {
            format!("{}:{key}", self.namespace)
        }
    }
}

impl<T, C> TypedCache<T, C>
where
    T: Serialize + DeserializeOwned,
    C: CacheCodec,
{
    /// Get and decode a typed cached value.
    pub async fn get(&self, key: &str) -> Result<Option<T>> {
        self.cache.get(&self.key(key)).await
    }

    /// Encode and store a typed value.
    pub async fn put(&self, key: &str, value: T, options: CacheOptions) -> Result<()> {
        self.cache.put(&self.key(key), value, options).await
    }

    /// Get a typed value, or run the loader and cache its result on miss.
    pub async fn get_or_load<E, F, Fut>(
        &self,
        key: &str,
        options: CacheOptions,
        loader: F,
    ) -> Result<T>
    where
        E: Error + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<T, E>> + Send + 'static,
    {
        self.cache
            .get_or_load(&self.key(key), options, loader)
            .await
    }

    /// Get a typed value, or compute and cache it with an infallible async loader.
    pub async fn get_or_insert_with<F, Fut>(
        &self,
        key: &str,
        options: CacheOptions,
        loader: F,
    ) -> Result<T>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        self.cache
            .get_or_insert_with(&self.key(key), options, loader)
            .await
    }

    /// Get a typed value, or run a fallible async loader and cache its result on miss.
    pub async fn try_get_or_insert_with<E, F, Fut>(
        &self,
        key: &str,
        options: CacheOptions,
        loader: F,
    ) -> Result<T>
    where
        E: Error + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<T, E>> + Send + 'static,
    {
        self.cache
            .try_get_or_insert_with(&self.key(key), options, loader)
            .await
    }

    /// Remove one typed key from the cache.
    pub async fn remove(&self, key: &str) -> Result<bool> {
        self.cache.remove(&self.key(key)).await
    }

    /// Remove one typed key from the cache.
    pub async fn invalidate_key(&self, key: &str) -> Result<bool> {
        self.remove(key).await
    }

    /// Return whether the typed key currently maps to a usable value.
    pub async fn contains_key(&self, key: &str) -> bool {
        self.cache.contains_key(&self.key(key)).await
    }

    /// Remove all entries currently associated with a tag.
    pub async fn invalidate_tag(&self, tag: &str) -> Result<u64> {
        self.cache.invalidate_tag(tag).await
    }

    /// Remove all cached entries and tag mappings from the shared cache.
    pub async fn flush(&self) -> Result<()> {
        self.cache.flush().await
    }

    /// Return a snapshot of shared cache counters.
    pub fn stats(&self) -> CacheStats {
        self.cache.stats()
    }

    /// Return shared cache diagnostics for this typed cache view.
    pub async fn diagnostics(&self) -> CacheDiagnostics {
        self.cache.diagnostics().await
    }
}
