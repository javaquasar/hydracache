use std::error::Error;
use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::time::Duration;

use hydracache::{CacheKeyBuilder, CacheOptions, HydraCache, PostcardCodec, TagSet};
use hydracache_core::CacheCodec;
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    CacheEntity, DbAdapterKind, DbCacheError, DbOperationContext, DbResultShape,
    PreparedQueryPolicy, QueryCachePolicy, Result,
};

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
            policy: QueryCachePolicy::new(),
            adapter: DbAdapterKind::Generic,
            result_shape: DbResultShape::Custom,
            value: PhantomData,
        }
    }

    /// Start describing a cacheable database-loaded value with a reusable
    /// [`QueryCachePolicy`].
    ///
    /// This is useful when the same key/tag/TTL pattern is shared by a
    /// repository method, a SQLx call site, and a future ORM adapter.
    pub fn cached_with<T>(&self, policy: QueryCachePolicy) -> DbQuery<T, C> {
        self.cached::<T>().with_policy(policy)
    }

    /// Prepare a reusable database query descriptor from stable metadata.
    ///
    /// Use this when a repository method runs many times and only a small part
    /// of the cache metadata changes per call, such as the entity id.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::time::Duration;
    ///
    /// use hydracache::HydraCache;
    /// use hydracache_db::{CacheEntity, DbCache, PreparedQueryPolicy};
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct User {
    ///     id: i64,
    /// }
    ///
    /// impl CacheEntity for User {
    ///     type Id = i64;
    ///
    ///     const ENTITY: &'static str = "user";
    ///     const COLLECTION: Option<&'static str> = Some("users");
    /// }
    ///
    /// let queries = DbCache::new(HydraCache::local().build(), "db");
    /// let load_user = queries.prepare::<User>(
    ///     PreparedQueryPolicy::for_cache_entity::<User>()
    ///         .with_name("load-user")
    ///         .ttl(Duration::from_secs(60)),
    /// );
    ///
    /// let query = load_user.for_id(42);
    /// assert_eq!(query.physical_key(), Some("db:user:42".to_owned()));
    /// assert_eq!(query.tags_value(), &["users".to_owned(), "user:42".to_owned()]);
    /// ```
    pub fn prepare<T>(&self, policy: PreparedQueryPolicy) -> PreparedDbQuery<T, C> {
        PreparedDbQuery {
            cache: self.cache.clone(),
            namespace: self.namespace.clone(),
            policy,
            value: PhantomData,
        }
    }

    /// Prepare a reusable entity-id descriptor from [`CacheEntity`] metadata.
    pub fn prepare_entity<T>(&self) -> PreparedDbQuery<T, C>
    where
        T: CacheEntity,
    {
        self.prepare(PreparedQueryPolicy::for_cache_entity::<T>())
    }

    /// Start describing an entity-shaped cached value.
    ///
    /// This is a convenience layer over [`DbCache::cached`] that sets both the
    /// logical key and the entity invalidation tag from escaped key segments.
    /// For example, `entity::<User>("user", 42)` creates key `user:42` and tag
    /// `user:42`; with namespace `db`, the physical cache key is `db:user:42`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::HydraCache;
    /// use hydracache_db::DbCache;
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct User {
    ///     id: i64,
    /// }
    ///
    /// let queries = DbCache::new(HydraCache::local().build(), "db");
    /// let query = queries.entity::<User>("user", 42);
    ///
    /// assert_eq!(query.key_value(), Some("user:42"));
    /// assert_eq!(query.tags_value(), &["user:42".to_owned()]);
    /// assert_eq!(query.physical_key(), Some("db:user:42".to_owned()));
    /// ```
    pub fn entity<T>(&self, kind: impl ToString, id: impl ToString) -> DbQuery<T, C> {
        self.cached::<T>().for_entity(kind, id)
    }

    /// Start describing an entity-shaped cached value from [`CacheEntity`]
    /// metadata.
    ///
    /// This helper removes repeated entity and collection literals from call
    /// sites. It sets the logical key, entity tag, and optional collection tag
    /// defined by `T`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::HydraCache;
    /// use hydracache_db::{CacheEntity, DbCache};
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct User {
    ///     id: i64,
    /// }
    ///
    /// impl CacheEntity for User {
    ///     type Id = i64;
    ///
    ///     const ENTITY: &'static str = "user";
    ///     const COLLECTION: Option<&'static str> = Some("users");
    /// }
    ///
    /// let queries = DbCache::new(HydraCache::local().build(), "db");
    /// let query = queries.for_entity::<User>(42);
    ///
    /// assert_eq!(query.key_value(), Some("user:42"));
    /// assert_eq!(
    ///     query.tags_value(),
    ///     &["user:42".to_owned(), "users".to_owned()]
    /// );
    /// ```
    pub fn for_entity<T>(&self, id: T::Id) -> DbQuery<T, C>
    where
        T: CacheEntity,
    {
        self.cached::<T>().for_cache_entity(id)
    }

    /// Start describing a collection-shaped cached value.
    ///
    /// This sets both the logical key and the collection invalidation tag to
    /// the escaped collection name. For example, `collection::<User>("users")`
    /// creates key `users` and tag `users`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::HydraCache;
    /// use hydracache_db::DbCache;
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct User {
    ///     id: i64,
    /// }
    ///
    /// let queries = DbCache::new(HydraCache::local().build(), "db");
    /// let query = queries.collection::<User>("users:active");
    ///
    /// assert_eq!(query.key_value(), Some("users%3Aactive"));
    /// assert_eq!(query.tags_value(), &["users%3Aactive".to_owned()]);
    /// assert_eq!(query.physical_key(), Some("db:users%3Aactive".to_owned()));
    /// ```
    pub fn collection<T>(&self, name: impl ToString) -> DbQuery<T, C> {
        self.cached::<T>().collection(name)
    }

    /// Start describing a cacheable database-loaded value with a diagnostic name.
    pub fn named<T>(&self, name: impl Into<String>) -> DbQuery<T, C> {
        DbQuery {
            cache: self.cache.clone(),
            namespace: self.namespace.clone(),
            policy: QueryCachePolicy::named(name),
            adapter: DbAdapterKind::Generic,
            result_shape: DbResultShape::Custom,
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
    policy: QueryCachePolicy,
    adapter: DbAdapterKind,
    result_shape: DbResultShape,
    value: PhantomData<fn() -> T>,
}

/// A prepared database query descriptor.
///
/// `PreparedDbQuery` keeps adapter state and stable query-cache metadata close
/// together. It can cheaply create ordinary [`DbQuery`] values for each call,
/// or execute loaders directly through `load_id`/`fetch_with_id`.
pub struct PreparedDbQuery<T, C = PostcardCodec>
where
    C: CacheCodec,
{
    cache: HydraCache<C>,
    namespace: String,
    policy: PreparedQueryPolicy,
    value: PhantomData<fn() -> T>,
}

impl<T, C> Clone for PreparedDbQuery<T, C>
where
    C: CacheCodec,
{
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            namespace: self.namespace.clone(),
            policy: self.policy.clone(),
            value: PhantomData,
        }
    }
}

impl<T, C> fmt::Debug for PreparedDbQuery<T, C>
where
    C: CacheCodec,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedDbQuery")
            .field("namespace", &self.namespace)
            .field("policy", &self.policy)
            .finish_non_exhaustive()
    }
}

impl<T, C> PreparedDbQuery<T, C>
where
    C: CacheCodec,
{
    /// Return the namespace used for physical cache keys.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Return the underlying local cache.
    pub fn cache(&self) -> &HydraCache<C> {
        &self.cache
    }

    /// Return the prepared policy backing this descriptor.
    pub fn prepared_policy(&self) -> &PreparedQueryPolicy {
        &self.policy
    }

    /// Return the optional diagnostic operation name.
    pub fn name(&self) -> Option<&str> {
        self.policy.name()
    }

    /// Return whether this descriptor needs an id binding before execution.
    pub fn requires_id(&self) -> bool {
        self.policy.requires_id()
    }

    /// Return the static logical key, if this descriptor has one.
    pub fn static_key_value(&self) -> Option<&str> {
        self.policy.static_key_value()
    }

    /// Return the precomputed entity key prefix, if this is an entity policy.
    pub fn entity_key_prefix(&self) -> Option<&str> {
        self.policy.entity_key_prefix()
    }

    /// Return the precomputed tags.
    pub fn tags_value(&self) -> &[String] {
        self.policy.tags_value()
    }

    /// Return the configured per-entry TTL.
    pub fn ttl_value(&self) -> Option<Duration> {
        self.policy.ttl_value()
    }

    /// Return the configured refresh/stale policy.
    pub fn refresh_policy_value(&self) -> Option<hydracache::RefreshOptions> {
        self.policy.refresh_policy_value()
    }

    /// Replace refresh/stale behavior for this prepared descriptor.
    pub fn refresh_policy(mut self, refresh: hydracache::RefreshOptions) -> Self {
        self.policy = self.policy.refresh_policy(refresh);
        self
    }

    /// Create a runtime query from a static prepared policy.
    ///
    /// Entity-id policies should usually use [`PreparedDbQuery::for_id`] so the
    /// dynamic id becomes part of the key and entity invalidation tag.
    pub fn to_query(&self) -> DbQuery<T, C> {
        self.query_from_policy(self.policy.to_policy())
    }

    /// Bind an id and create a runtime query.
    pub fn for_id(&self, id: impl ToString) -> DbQuery<T, C> {
        self.query_from_policy(self.policy.bind_id(id))
    }

    /// Fetch a cached value from a static prepared policy or run the loader.
    pub async fn load<E, F, Fut>(&self, loader: F) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Error + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<T, E>> + Send + 'static,
    {
        self.to_query().load(loader).await
    }

    /// Bind an id, fetch a cached value, or run the loader.
    pub async fn load_id<E, F, Fut>(&self, id: impl ToString, loader: F) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Error + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<T, E>> + Send + 'static,
    {
        self.for_id(id).load(loader).await
    }

    /// Fetch a static prepared value with an output type chosen by an adapter.
    pub async fn fetch_value_with<U, E, F, Fut>(&self, loader: F) -> Result<U>
    where
        U: Serialize + DeserializeOwned + Send + 'static,
        E: Error + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<U, E>> + Send + 'static,
    {
        self.to_query().fetch_value_with(loader).await
    }

    /// Bind an id and fetch a value with an output type chosen by an adapter.
    pub async fn fetch_value_with_id<U, E, F, Fut>(&self, id: impl ToString, loader: F) -> Result<U>
    where
        U: Serialize + DeserializeOwned + Send + 'static,
        E: Error + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<U, E>> + Send + 'static,
    {
        self.for_id(id).fetch_value_with(loader).await
    }

    fn query_from_policy(&self, policy: QueryCachePolicy) -> DbQuery<T, C> {
        DbQuery {
            cache: self.cache.clone(),
            namespace: self.namespace.clone(),
            policy,
            adapter: DbAdapterKind::Generic,
            result_shape: DbResultShape::Custom,
            value: PhantomData,
        }
    }
}

impl<T, C> Clone for DbQuery<T, C>
where
    C: CacheCodec,
{
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            namespace: self.namespace.clone(),
            policy: self.policy.clone(),
            adapter: self.adapter,
            result_shape: self.result_shape,
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
            .field("policy", &self.policy)
            .field("adapter", &self.adapter)
            .field("result_shape", &self.result_shape)
            .finish_non_exhaustive()
    }
}

impl<T, C> DbQuery<T, C>
where
    C: CacheCodec,
{
    /// Return the optional diagnostic operation name.
    pub fn name(&self) -> Option<&str> {
        self.policy.name()
    }

    /// Set or replace the diagnostic operation name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.policy = self.policy.with_name(name);
        self
    }

    /// Return the reusable cache policy backing this descriptor.
    pub fn cache_policy(&self) -> &QueryCachePolicy {
        &self.policy
    }

    /// Replace the current cache policy.
    ///
    /// This is the lowest-friction way to reuse one policy across SQLx,
    /// Diesel, SeaORM, or repository-style call sites while keeping the loader
    /// itself fully caller-controlled.
    pub fn with_policy(mut self, policy: QueryCachePolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Return the namespace used for physical cache keys.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Return the logical key, if one has been configured.
    pub fn key_value(&self) -> Option<&str> {
        self.policy.key_value()
    }

    /// Return the physical cache key, including the adapter namespace.
    pub fn physical_key(&self) -> Option<String> {
        let key = self.key_value()?;
        Some(physical_key(&self.namespace, key))
    }

    /// Return the configured tags.
    pub fn tags_value(&self) -> &[String] {
        self.policy.tags_value()
    }

    /// Return the configured per-entry TTL.
    pub fn ttl_value(&self) -> Option<Duration> {
        self.policy.ttl_value()
    }

    /// Return the configured refresh/stale policy.
    pub fn refresh_policy_value(&self) -> Option<hydracache::RefreshOptions> {
        self.policy.refresh_policy_value()
    }

    /// Return the database adapter kind used for operation diagnostics.
    pub fn adapter_kind(&self) -> DbAdapterKind {
        self.adapter
    }

    /// Return the result shape used for operation diagnostics.
    pub fn result_shape(&self) -> DbResultShape {
        self.result_shape
    }

    /// Set database adapter and result-shape context for diagnostics.
    ///
    /// Most users do not need to call this directly. Adapter crates use it to
    /// label errors from helpers such as `sqlx_one`, `diesel_optional`, or
    /// `sea_all` without changing the cache key, tags, TTL, or loader.
    pub fn adapter_context(mut self, adapter: DbAdapterKind, result_shape: DbResultShape) -> Self {
        self.adapter = adapter;
        self.result_shape = result_shape;
        self
    }

    /// Set the logical cache key for this query result.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.policy = self.policy.key(key);
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
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::HydraCache;
    /// use hydracache_db::DbCache;
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct User {
    ///     id: i64,
    /// }
    ///
    /// let queries = DbCache::new(HydraCache::local().build(), "db");
    /// let query = queries
    ///     .cached::<User>()
    ///     .tag("users")
    ///     .for_entity("user", 42);
    ///
    /// assert_eq!(query.key_value(), Some("user:42"));
    /// assert_eq!(
    ///     query.tags_value(),
    ///     &["users".to_owned(), "user:42".to_owned()]
    /// );
    /// ```
    pub fn for_entity(mut self, kind: impl ToString, id: impl ToString) -> Self {
        self.policy = self.policy.for_entity(kind, id);
        self
    }

    /// Set the logical key and tags from [`CacheEntity`] metadata.
    ///
    /// This is the metadata-driven equivalent of [`DbQuery::for_entity`]. It
    /// preserves any existing tags, then adds the entity tag and optional
    /// collection tag defined by `T`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::HydraCache;
    /// use hydracache_db::{CacheEntity, DbCache};
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct User {
    ///     id: i64,
    /// }
    ///
    /// impl CacheEntity for User {
    ///     type Id = i64;
    ///
    ///     const ENTITY: &'static str = "user";
    ///     const COLLECTION: Option<&'static str> = Some("users");
    /// }
    ///
    /// let queries = DbCache::new(HydraCache::local().build(), "db");
    /// let query = queries
    ///     .cached::<User>()
    ///     .tag("tenant:7")
    ///     .for_cache_entity(42);
    ///
    /// assert_eq!(query.key_value(), Some("user:42"));
    /// assert_eq!(
    ///     query.tags_value(),
    ///     &[
    ///         "tenant:7".to_owned(),
    ///         "user:42".to_owned(),
    ///         "users".to_owned()
    ///     ]
    /// );
    /// ```
    pub fn for_cache_entity(mut self, id: T::Id) -> Self
    where
        T: CacheEntity,
    {
        self.policy = self.policy.for_cache_entity::<T>(id);
        self
    }

    /// Set the logical key and invalidation tag for a collection result.
    pub fn collection(mut self, name: impl ToString) -> Self {
        self.policy = self.policy.collection(name);
        self
    }

    /// Add one invalidation tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.policy = self.policy.tag(tag);
        self
    }

    /// Add a collection invalidation tag from one escaped key segment.
    ///
    /// Use this with [`DbCache::entity`] or [`DbQuery::for_entity`] when one
    /// entity result also belongs to a broader list or query group.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache::HydraCache;
    /// use hydracache_db::DbCache;
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct User {
    ///     id: i64,
    /// }
    ///
    /// let queries = DbCache::new(HydraCache::local().build(), "db");
    /// let query = queries
    ///     .entity::<User>("user", 42)
    ///     .collection_tag("users:active");
    ///
    /// assert_eq!(
    ///     query.tags_value(),
    ///     &["user:42".to_owned(), "users%3Aactive".to_owned()]
    /// );
    /// ```
    pub fn collection_tag(mut self, name: impl ToString) -> Self {
        self.policy = self.policy.collection_tag(name);
        self
    }

    /// Add several invalidation tags.
    pub fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.policy = self.policy.tags(tags);
        self
    }

    /// Replace invalidation tags from a reusable [`TagSet`].
    pub fn tag_set(mut self, tags: TagSet) -> Self {
        self.policy = self.policy.tag_set(tags);
        self
    }

    /// Set a per-entry TTL for this query result.
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.policy = self.policy.ttl(ttl);
        self
    }

    /// Set refresh/stale behavior for this query result.
    pub fn refresh_policy(mut self, refresh: hydracache::RefreshOptions) -> Self {
        self.policy = self.policy.refresh_policy(refresh);
        self
    }

    /// Fetch a cached value or run the supplied repository/database loader on
    /// miss.
    ///
    /// This is a short alias for [`DbQuery::fetch_with`]. It reads more
    /// naturally when a call site is wrapping a repository method rather than a
    /// raw SQL query.
    pub async fn load<E, F, Fut>(self, loader: F) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Error + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<T, E>> + Send + 'static,
    {
        self.fetch_with(loader).await
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
        let key = self.required_physical_key()?;
        let options = self.options();
        let context = self.operation_context(Some(key.clone()));

        match self.policy.refresh_policy_value() {
            Some(refresh) => self
                .cache
                .get_or_load_with_refresh(&key, options, refresh, loader)
                .await
                .map_err(|source| DbCacheError::operation(context, source)),
            None => self
                .cache
                .get_or_load(&key, options, loader)
                .await
                .map_err(|source| DbCacheError::operation(context, source)),
        }
    }

    fn options(&self) -> CacheOptions {
        self.policy.cache_options()
    }

    fn required_physical_key(&self) -> Result<String> {
        self.physical_key().ok_or_else(|| {
            let context = self.operation_context(None);
            DbCacheError::MissingKey {
                operation: context.operation,
                adapter: context.adapter,
                namespace: context.namespace,
                result_shape: context.result_shape,
            }
        })
    }

    fn operation_context(&self, physical_key: Option<String>) -> DbOperationContext {
        DbOperationContext {
            adapter: self.adapter,
            operation: self.operation_label(),
            namespace: self.namespace.clone(),
            physical_key,
            result_shape: self.result_shape,
        }
    }

    fn operation_label(&self) -> String {
        self.name()
            .map(str::to_owned)
            .unwrap_or_else(|| default_operation_label(&self.namespace))
    }
}

fn physical_key(namespace: &str, key: &str) -> String {
    if namespace.is_empty() {
        key.to_owned()
    } else {
        format!("{namespace}:{key}")
    }
}

fn default_operation_label(namespace: &str) -> String {
    if namespace.is_empty() {
        "unnamed".to_owned()
    } else {
        format!("{namespace}:unnamed")
    }
}
