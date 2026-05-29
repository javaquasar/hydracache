use std::error::Error;
use std::future::Future;
use std::marker::PhantomData;
use std::time::Duration;

use hydracache::{CacheKeyBuilder, CacheOptions, HydraCache, PostcardCodec, TagSet};
use hydracache_core::CacheCodec;
use serde::{de::DeserializeOwned, Serialize};

use crate::{Result, SqlxCacheError};

/// A SQLx-oriented view over a [`HydraCache`] instance.
///
/// `SqlxCache` groups query result keys under a namespace while keeping all
/// cache storage, single-flight, tags, TTL, and stats in the shared local cache.
#[derive(Debug, Clone)]
pub struct SqlxCache<C = PostcardCodec>
where
    C: CacheCodec,
{
    cache: HydraCache<C>,
    namespace: String,
}

impl<C> SqlxCache<C>
where
    C: CacheCodec,
{
    /// Create a SQLx query cache adapter over an existing local cache.
    pub fn new(cache: HydraCache<C>, namespace: impl Into<String>) -> Self {
        Self {
            cache,
            namespace: namespace.into(),
        }
    }

    /// Return the namespace used for physical cache keys.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Return the underlying local cache.
    pub fn cache(&self) -> &HydraCache<C> {
        &self.cache
    }

    /// Start describing a cacheable SQL query result.
    pub fn query_as<T>(&self, sql: impl Into<String>) -> SqlxQuery<T, C> {
        SqlxQuery {
            cache: self.cache.clone(),
            namespace: self.namespace.clone(),
            sql: sql.into(),
            key: None,
            tags: TagSet::new(),
            ttl: None,
            value: PhantomData,
        }
    }
}

/// A cacheable SQL query descriptor.
///
/// The descriptor is deliberately explicit: callers provide the SQL text for
/// diagnostics, then choose the key, tags, and TTL that match their freshness
/// model. `fetch_with` executes the supplied SQLx loader only on a cache miss.
#[derive(Debug, Clone)]
pub struct SqlxQuery<T, C = PostcardCodec>
where
    C: CacheCodec,
{
    cache: HydraCache<C>,
    namespace: String,
    sql: String,
    key: Option<String>,
    tags: TagSet,
    ttl: Option<Duration>,
    value: PhantomData<fn() -> T>,
}

impl<T, C> SqlxQuery<T, C>
where
    C: CacheCodec,
{
    /// Return the SQL text associated with this cached query.
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// Return the namespace used for physical cache keys.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Return the logical key, if one has been configured.
    pub fn key_value(&self) -> Option<&str> {
        self.key.as_deref()
    }

    /// Return the physical cache key, including the adapter namespace.
    pub fn physical_key(&self) -> Option<String> {
        self.key
            .as_deref()
            .map(|key| physical_key(&self.namespace, key))
    }

    /// Return the configured tags.
    pub fn tags_value(&self) -> &[String] {
        self.tags.as_slice()
    }

    /// Return the configured per-entry TTL.
    pub fn ttl_value(&self) -> Option<Duration> {
        self.ttl
    }

    /// Set the logical cache key for this query result.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Set the logical cache key from a segmented key builder.
    pub fn key_builder(self, key: CacheKeyBuilder) -> Self {
        self.key(key.build_string())
    }

    /// Add one invalidation tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags = self.tags.tag(tag);
        self
    }

    /// Add several invalidation tags.
    pub fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags = self.tags.tags(tags);
        self
    }

    /// Replace invalidation tags from a reusable [`TagSet`].
    pub fn tag_set(mut self, tags: TagSet) -> Self {
        self.tags = tags;
        self
    }

    /// Set a per-entry TTL for this query result.
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Fetch a cached value or run the supplied SQLx loader on miss.
    ///
    /// The loader is intentionally caller-supplied so SQLx remains responsible
    /// for connection pools, transactions, compile-time checked macros, and row
    /// mapping. HydraCache owns only the cache boundary.
    pub async fn fetch_with<E, F, Fut>(self, loader: F) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Error + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<T, E>> + Send + 'static,
    {
        let Some(key) = self.physical_key() else {
            return Err(SqlxCacheError::MissingKey { sql: self.sql });
        };

        self.cache
            .get_or_load(&key, self.options(), loader)
            .await
            .map_err(SqlxCacheError::from)
    }

    fn options(&self) -> CacheOptions {
        let mut options = CacheOptions::new().tag_set(self.tags.clone());
        if let Some(ttl) = self.ttl {
            options = options.ttl(ttl);
        }
        options
    }
}

fn physical_key(namespace: &str, key: &str) -> String {
    if namespace.is_empty() {
        key.to_owned()
    } else {
        format!("{namespace}:{key}")
    }
}
