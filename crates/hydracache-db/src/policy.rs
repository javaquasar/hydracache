use std::time::Duration;

use hydracache::{CacheKeyBuilder, CacheOptions, TagSet};

use crate::CacheEntity;

const SHORT_LIVED_TTL: Duration = Duration::from_secs(30);
const READ_MOSTLY_TTL: Duration = Duration::from_secs(300);
const PER_ENTITY_TTL: Duration = Duration::from_secs(300);
const NEGATIVE_CACHE_TTL: Duration = Duration::from_secs(30);

/// Reusable cache metadata for one database query result.
///
/// `QueryCachePolicy` contains the database-neutral parts of query result
/// caching: diagnostic name, logical key, invalidation tags, and optional TTL.
/// It is intentionally independent of SQLx, Diesel, SeaORM, or any other
/// database client.
///
/// # Example
///
/// ```rust
/// use std::time::Duration;
///
/// use hydracache_db::QueryCachePolicy;
///
/// let policy = QueryCachePolicy::named("load-user")
///     .key("user:42")
///     .tag("user:42")
///     .ttl(Duration::from_secs(60));
///
/// assert_eq!(policy.name(), Some("load-user"));
/// assert_eq!(policy.key_value(), Some("user:42"));
/// assert_eq!(policy.tags_value(), &["user:42".to_owned()]);
/// assert_eq!(policy.ttl_value(), Some(Duration::from_secs(60)));
/// ```
///
/// The [`query_cache_policy!`](crate::query_cache_policy) macro provides a
/// shorter declarative form when the policy is known at the call site.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QueryCachePolicy {
    name: Option<String>,
    key: Option<String>,
    tags: TagSet,
    ttl: Option<Duration>,
}

impl QueryCachePolicy {
    /// Create an empty cache policy.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a short-lived policy for values that should smooth brief bursts.
    ///
    /// The preset uses a 30 second TTL and leaves key/tags to the caller.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::time::Duration;
    ///
    /// use hydracache_db::QueryCachePolicy;
    ///
    /// let policy = QueryCachePolicy::short_lived().key("user:42");
    ///
    /// assert_eq!(policy.ttl_value(), Some(Duration::from_secs(30)));
    /// assert_eq!(policy.key_value(), Some("user:42"));
    /// ```
    pub fn short_lived() -> Self {
        Self::new().ttl(SHORT_LIVED_TTL)
    }

    /// Create a read-mostly policy for values that change rarely.
    ///
    /// The preset uses a 5 minute TTL. Pair it with entity or collection tags
    /// so writes can still invalidate cached results explicitly.
    pub fn read_mostly() -> Self {
        Self::new().ttl(READ_MOSTLY_TTL)
    }

    /// Create a policy intended for one entity-shaped result.
    ///
    /// The preset uses a 5 minute TTL and expects the caller to add an entity
    /// key/tag with [`QueryCachePolicy::for_entity`] or
    /// [`QueryCachePolicy::for_cache_entity`].
    pub fn per_entity() -> Self {
        Self::new().ttl(PER_ENTITY_TTL)
    }

    /// Create a policy for explicit-invalidation-only values.
    ///
    /// No TTL is configured. The value remains cached until the caller
    /// invalidates a key/tag, removes it, flushes the cache, or the backend
    /// evicts it due to capacity pressure.
    pub fn no_ttl_explicit_invalidation() -> Self {
        Self::new()
    }

    /// Create a policy for caching negative lookups briefly.
    ///
    /// Use this for `Option<T>` or domain-specific "not found" results where
    /// repeated misses are expensive but long-lived absence would be unsafe.
    /// The preset uses a 30 second TTL.
    pub fn negative_cache() -> Self {
        Self::new().ttl(NEGATIVE_CACHE_TTL)
    }

    /// Create a cache policy with a diagnostic operation name.
    pub fn named(name: impl Into<String>) -> Self {
        Self::new().with_name(name)
    }

    /// Return the optional diagnostic operation name.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Return the logical key, if one has been configured.
    pub fn key_value(&self) -> Option<&str> {
        self.key.as_deref()
    }

    /// Return configured invalidation tags.
    pub fn tags_value(&self) -> &[String] {
        self.tags.as_slice()
    }

    /// Return the optional per-entry TTL.
    pub fn ttl_value(&self) -> Option<Duration> {
        self.ttl
    }

    /// Set or replace the diagnostic operation name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the logical cache key.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Set the logical cache key from a segmented key builder.
    pub fn key_builder(self, key: CacheKeyBuilder) -> Self {
        self.key(key.build_string())
    }

    /// Set the logical key and add the same entity invalidation tag.
    pub fn for_entity(mut self, kind: impl ToString, id: impl ToString) -> Self {
        let key = entity_key(kind, id);
        self.key = Some(key.clone());
        self.tags = self.tags.tag(key);
        self
    }

    /// Set the logical key and tags from [`CacheEntity`] metadata.
    pub fn for_cache_entity<T>(mut self, id: T::Id) -> Self
    where
        T: CacheEntity,
    {
        let key = T::cache_key_for(&id);
        self.key = Some(key);
        self.tags = self.tags.tag(T::entity_tag_for(&id));
        self.tags = append_optional_tag(self.tags, T::collection_tag());
        self
    }

    /// Set the logical key and invalidation tag for a collection result.
    pub fn collection(mut self, name: impl ToString) -> Self {
        let tag = collection_tag(name);
        self.key = Some(tag.clone());
        self.tags = self.tags.tag(tag);
        self
    }

    /// Add one invalidation tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags = self.tags.tag(tag);
        self
    }

    /// Add a collection invalidation tag from one escaped key segment.
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

    /// Set a per-entry TTL.
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    pub(crate) fn cache_options(&self) -> CacheOptions {
        let mut options = CacheOptions::new().tag_set(self.tags.clone());
        if let Some(ttl) = self.ttl {
            options = options.ttl(ttl);
        }
        options
    }
}

pub(crate) fn entity_key(kind: impl ToString, id: impl ToString) -> String {
    CacheKeyBuilder::new().entity(kind, id).build_string()
}

pub(crate) fn collection_tag(name: impl ToString) -> String {
    CacheKeyBuilder::from_segment(name).build_string()
}

fn append_optional_tag(tags: TagSet, tag: Option<String>) -> TagSet {
    match tag {
        Some(tag) => tags.tag(tag),
        None => tags,
    }
}
