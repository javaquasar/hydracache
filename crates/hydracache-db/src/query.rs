use std::error::Error;
use std::fmt;
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
///     .entity::<User>("user", 42)
///     // Later, invalidate_tag("user:42") removes this result.
///     .collection_tag("users")
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
pub struct DbCache<C = PostcardCodec>
where
    C: CacheCodec,
{
    cache: HydraCache<C>,
    namespace: String,
}

impl<C> Clone for DbCache<C>
where
    C: CacheCodec,
{
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            namespace: self.namespace.clone(),
        }
    }
}

impl<C> fmt::Debug for DbCache<C>
where
    C: CacheCodec,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DbCache")
            .field("namespace", &self.namespace)
            .finish_non_exhaustive()
    }
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
        DbQuery {
            cache: self.cache.clone(),
            namespace: self.namespace.clone(),
            name: None,
            key: None,
            tags: TagSet::new(),
            ttl: None,
            value: PhantomData,
        }
    }

    /// Start describing an entity-shaped cached value.
    ///
    /// This is a convenience layer over [`DbCache::cached`] that sets both the
    /// logical key and the entity invalidation tag from escaped key segments.
    /// For example, `entity::<User>("user", 42)` creates key `user:42` and tag
    /// `user:42`; with namespace `db`, the physical cache key is `db:user:42`.
    pub fn entity<T>(&self, kind: impl ToString, id: impl ToString) -> DbQuery<T, C> {
        self.cached::<T>().for_entity(kind, id)
    }

    /// Start describing a collection-shaped cached value.
    ///
    /// This sets both the logical key and the collection invalidation tag to
    /// the escaped collection name. For example, `collection::<User>("users")`
    /// creates key `users` and tag `users`.
    pub fn collection<T>(&self, name: impl ToString) -> DbQuery<T, C> {
        let tag = collection_tag(name);
        self.cached::<T>().key(tag.clone()).tag(tag)
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

impl<T, C> Clone for DbQuery<T, C>
where
    C: CacheCodec,
{
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            namespace: self.namespace.clone(),
            name: self.name.clone(),
            key: self.key.clone(),
            tags: self.tags.clone(),
            ttl: self.ttl,
            value: PhantomData,
        }
    }
}

impl<T, C> fmt::Debug for DbQuery<T, C>
where
    C: CacheCodec,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DbQuery")
            .field("namespace", &self.namespace)
            .field("name", &self.name)
            .field("key", &self.key)
            .field("tags", &self.tags)
            .field("ttl", &self.ttl)
            .finish_non_exhaustive()
    }
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

    /// Set the logical key and add an entity invalidation tag.
    ///
    /// `for_entity("user", 42)` sets the key to `user:42` and adds the tag
    /// `user:42`. Both segments are escaped with [`CacheKeyBuilder`], so `:` and
    /// `%` inside one segment cannot accidentally create extra key segments.
    pub fn for_entity(mut self, kind: impl ToString, id: impl ToString) -> Self {
        let key = entity_key(kind, id);
        self.key = Some(key.clone());
        self.tags = self.tags.tag(key);
        self
    }

    /// Add one invalidation tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags = self.tags.tag(tag);
        self
    }

    /// Add a collection invalidation tag from one escaped key segment.
    ///
    /// Use this with [`DbCache::entity`] or [`DbQuery::for_entity`] when one
    /// entity result also belongs to a broader list or query group.
    pub fn collection_tag(mut self, name: impl ToString) -> Self {
        self.tags = self.tags.tag(collection_tag(name));
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
        self.fetch_value_with(loader).await
    }

    /// Fetch a cached value with an output type chosen by an adapter.
    ///
    /// Most application code should use [`DbQuery::fetch_with`]. This method is
    /// intended for adapter crates that keep the descriptor type focused on a
    /// database row while caching shapes such as `Option<T>` or `Vec<T>`.
    pub async fn fetch_value_with<U, E, F, Fut>(self, loader: F) -> Result<U>
    where
        U: Serialize + DeserializeOwned + Send + 'static,
        E: Error + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<U, E>> + Send + 'static,
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
        match &self.name {
            Some(name) => name.clone(),
            None if self.namespace.is_empty() => "unnamed".to_owned(),
            None => format!("{}:unnamed", self.namespace),
        }
    }
}

fn entity_key(kind: impl ToString, id: impl ToString) -> String {
    CacheKeyBuilder::new().entity(kind, id).build_string()
}

fn collection_tag(name: impl ToString) -> String {
    CacheKeyBuilder::from_segment(name).build_string()
}

fn physical_key(namespace: &str, key: &str) -> String {
    if namespace.is_empty() {
        key.to_owned()
    } else {
        format!("{namespace}:{key}")
    }
}
