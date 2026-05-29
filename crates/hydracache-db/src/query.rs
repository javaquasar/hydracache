use std::error::Error;
use std::future::Future;
use std::marker::PhantomData;
use std::time::Duration;

use hydracache::{CacheKeyBuilder, CacheOptions, HydraCache, PostcardCodec, TagSet};
use hydracache_core::CacheCodec;
use serde::{de::DeserializeOwned, Serialize};

use crate::{DbCacheError, Result};

/// A database-oriented view over a [`HydraCache`] instance.
///
/// `DbCache` groups query result keys under a namespace while keeping all
/// cache storage, single-flight, tags, TTL, and stats in the shared local cache.
///
/// # Example
///
/// ```rust
/// use std::time::Duration;
///
/// use hydracache::HydraCache;
/// use hydracache_db::DbCache;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// struct User {
///     id: i64,
///     name: String,
/// }
///
/// # #[tokio::main]
/// # async fn main() -> hydracache_db::Result<()> {
/// let local = HydraCache::local().build();
/// let queries = DbCache::new(local, "db");
///
/// let user = queries
///     .cached::<User>()
///     // Physical cache key: "db:user:42".
///     .key("user:42")
///     // Later, invalidate_tag("user:42") removes this result.
///     .tag("user:42")
///     .ttl(Duration::from_secs(60))
///     .fetch_with(|| async {
///         // Replace this block with code from sqlx, diesel, sea-orm, or any
///         // other database client. It is called only when the cache does not
///         // already contain "db:user:42" or when the cached value has expired.
///         Ok::<_, std::io::Error>(User {
///             id: 42,
///             name: "Ada".to_owned(),
///         })
///     })
///     .await?;
///
/// assert_eq!(user.id, 42);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct DbCache<C = PostcardCodec>
where
    C: CacheCodec,
{
    cache: HydraCache<C>,
    namespace: String,
}

impl<C> DbCache<C>
where
    C: CacheCodec,
{
    /// Create a database query cache adapter over an existing local cache.
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

    /// Start describing a cacheable database-loaded value.
    ///
    /// This is the preferred entry point when the query is already visible
    /// inside the `fetch_with` loader through a database client, ORM, or
    /// repository method.
    pub fn cached<T>(&self) -> DbQuery<T, C> {
        self.named("unnamed")
    }

    /// Start describing a cacheable database-loaded value with a diagnostic name.
    pub fn named<T>(&self, name: impl Into<String>) -> DbQuery<T, C> {
        DbQuery {
            cache: self.cache.clone(),
            namespace: self.namespace.clone(),
            name: Some(name.into()),
            key: None,
            tags: TagSet::new(),
            ttl: None,
            value: PhantomData,
        }
    }

    /// Start describing a cacheable SQL query result.
    ///
    /// Prefer [`DbCache::cached`] or [`DbCache::named`] when writing new code.
    /// This method remains useful if you want the SQL text itself to be the
    /// diagnostic label for errors and logs.
    pub fn query_as<T>(&self, sql: impl Into<String>) -> DbQuery<T, C> {
        self.named(sql)
    }
}

/// A cacheable database query descriptor.
///
/// The descriptor is deliberately explicit: callers choose the key, tags, and
/// TTL that match their freshness model. An operation name is optional and used
/// only for diagnostics. `fetch_with` executes the supplied loader only on a
/// cache miss.
#[derive(Debug, Clone)]
pub struct DbQuery<T, C = PostcardCodec>
where
    C: CacheCodec,
{
    cache: HydraCache<C>,
    namespace: String,
    name: Option<String>,
    key: Option<String>,
    tags: TagSet,
    ttl: Option<Duration>,
    value: PhantomData<fn() -> T>,
}

impl<T, C> DbQuery<T, C>
where
    C: CacheCodec,
{
    /// Return the optional diagnostic operation name.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Set or replace the diagnostic operation name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
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

    /// Fetch a cached value or run the supplied database loader on miss.
    ///
    /// The loader is intentionally caller-supplied so the database library
    /// remains responsible for pools, transactions, compile-time checked
    /// queries, and row mapping. HydraCache owns only the cache boundary.
    pub async fn fetch_with<E, F, Fut>(self, loader: F) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Error + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<T, E>> + Send + 'static,
    {
        let Some(key) = self.physical_key() else {
            return Err(DbCacheError::MissingKey {
                operation: self.operation_label(),
            });
        };

        self.cache
            .get_or_load(&key, self.options(), loader)
            .await
            .map_err(DbCacheError::from)
    }

    fn options(&self) -> CacheOptions {
        let mut options = CacheOptions::new().tag_set(self.tags.clone());
        if let Some(ttl) = self.ttl {
            options = options.ttl(ttl);
        }
        options
    }

    fn operation_label(&self) -> String {
        match (&self.name, &self.key) {
            (Some(name), _) => name.clone(),
            (None, Some(key)) if self.namespace.is_empty() => key.clone(),
            (None, Some(key)) => physical_key(&self.namespace, key),
            (None, None) if self.namespace.is_empty() => "unnamed".to_owned(),
            (None, None) => format!("{}:unnamed", self.namespace),
        }
    }
}

fn physical_key(namespace: &str, key: &str) -> String {
    if namespace.is_empty() {
        key.to_owned()
    } else {
        format!("{namespace}:{key}")
    }
}
