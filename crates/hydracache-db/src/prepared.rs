use std::time::Duration;

use hydracache::{CacheKeyBuilder, TagSet};

use crate::policy::collection_tag;
use crate::{CacheEntity, QueryCachePolicy};

const SHORT_LIVED_TTL: Duration = Duration::from_secs(30);
const READ_MOSTLY_TTL: Duration = Duration::from_secs(300);
const PER_ENTITY_TTL: Duration = Duration::from_secs(300);
const NEGATIVE_CACHE_TTL: Duration = Duration::from_secs(30);

/// Prepared database query cache metadata.
///
/// `PreparedQueryPolicy` stores stable query-cache metadata once and binds only
/// the dynamic part, such as an entity id, on the hot path. It remains
/// database-neutral: SQLx, Diesel, SeaORM, or a hand-written repository can all
/// turn the prepared policy into the ordinary [`QueryCachePolicy`] consumed by
/// [`DbCache`](crate::DbCache).
///
/// # Example
///
/// ```rust
/// use std::time::Duration;
///
/// use hydracache_db::{CacheEntity, PreparedQueryPolicy};
///
/// struct User;
///
/// impl CacheEntity for User {
///     type Id = i64;
///
///     const ENTITY: &'static str = "user";
///     const COLLECTION: Option<&'static str> = Some("users");
/// }
///
/// let prepared = PreparedQueryPolicy::for_cache_entity::<User>()
///     .with_name("load-user")
///     .ttl(Duration::from_secs(60));
///
/// let policy = prepared.bind_id(42);
/// assert_eq!(policy.name(), Some("load-user"));
/// assert_eq!(policy.key_value(), Some("user:42"));
/// assert_eq!(policy.tags_value(), &["users".to_owned(), "user:42".to_owned()]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedQueryPolicy {
    name: Option<String>,
    key: PreparedQueryKey,
    tags: TagSet,
    ttl: Option<Duration>,
}

impl Default for PreparedQueryPolicy {
    fn default() -> Self {
        Self {
            name: None,
            key: PreparedQueryKey::Missing,
            tags: TagSet::new(),
            ttl: None,
        }
    }
}

impl PreparedQueryPolicy {
    /// Create an empty prepared policy.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a short-lived prepared policy for burst smoothing.
    ///
    /// The preset uses a 30 second TTL and leaves key/tags to the caller.
    pub fn short_lived() -> Self {
        Self::new().ttl(SHORT_LIVED_TTL)
    }

    /// Create a read-mostly prepared policy for values that change rarely.
    ///
    /// The preset uses a 5 minute TTL. Pair it with entity or collection tags
    /// so writes can still invalidate cached results explicitly.
    pub fn read_mostly() -> Self {
        Self::new().ttl(READ_MOSTLY_TTL)
    }

    /// Create a prepared policy intended for one entity-shaped result.
    ///
    /// The preset uses a 5 minute TTL and expects the caller to add an entity
    /// prefix with [`PreparedQueryPolicy::entity`] or to start from
    /// [`PreparedQueryPolicy::for_cache_entity`].
    pub fn per_entity() -> Self {
        Self::new().ttl(PER_ENTITY_TTL)
    }

    /// Create a prepared policy for explicit-invalidation-only values.
    ///
    /// No TTL is configured. The value remains cached until invalidated,
    /// removed, flushed, or evicted due to capacity pressure.
    pub fn no_ttl_explicit_invalidation() -> Self {
        Self::new()
    }

    /// Create a prepared policy for caching negative lookups briefly.
    ///
    /// Use this for `Option<T>` or domain-specific "not found" results where
    /// repeated misses are expensive but long-lived absence would be unsafe.
    /// The preset uses a 30 second TTL.
    pub fn negative_cache() -> Self {
        Self::new().ttl(NEGATIVE_CACHE_TTL)
    }

    /// Create a prepared policy with a diagnostic operation name.
    pub fn named(name: impl Into<String>) -> Self {
        Self::new().with_name(name)
    }

    /// Create a prepared entity-id policy from one escaped entity segment.
    ///
    /// The entity segment is escaped once. Each [`PreparedQueryPolicy::bind_id`]
    /// call only escapes and appends the id segment.
    pub fn for_entity(kind: impl ToString) -> Self {
        Self::new().entity(kind)
    }

    /// Create a prepared entity-id policy from [`CacheEntity`] metadata.
    ///
    /// The entity prefix and optional collection tag are precomputed once.
    pub fn for_cache_entity<T>() -> Self
    where
        T: CacheEntity,
    {
        Self::new().cache_entity::<T>()
    }

    /// Return the optional diagnostic operation name.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Return `true` when this policy needs an id binding before it has a key.
    pub fn requires_id(&self) -> bool {
        matches!(self.key, PreparedQueryKey::EntityPrefix(_))
    }

    /// Return the static logical key, if this prepared policy has one.
    pub fn static_key_value(&self) -> Option<&str> {
        match &self.key {
            PreparedQueryKey::Static(key) => Some(key),
            PreparedQueryKey::Missing | PreparedQueryKey::EntityPrefix(_) => None,
        }
    }

    /// Return the precomputed entity key prefix, if this is an entity policy.
    pub fn entity_key_prefix(&self) -> Option<&str> {
        match &self.key {
            PreparedQueryKey::EntityPrefix(prefix) => Some(prefix),
            PreparedQueryKey::Missing | PreparedQueryKey::Static(_) => None,
        }
    }

    /// Return precomputed invalidation tags.
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

    /// Set a static logical key.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = PreparedQueryKey::Static(key.into());
        self
    }

    /// Set a static logical key from a segmented key builder.
    pub fn key_builder(self, key: CacheKeyBuilder) -> Self {
        self.key(key.build_string())
    }

    /// Set an entity-id key prefix from one escaped entity segment.
    pub fn entity(mut self, kind: impl ToString) -> Self {
        self.key = PreparedQueryKey::EntityPrefix(escaped_segment(kind));
        self
    }

    /// Set an entity-id key prefix and optional collection tag from
    /// [`CacheEntity`] metadata while preserving preset TTL/name settings.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::time::Duration;
    ///
    /// use hydracache_db::{CacheEntity, PreparedQueryPolicy};
    ///
    /// struct User;
    ///
    /// impl CacheEntity for User {
    ///     type Id = i64;
    ///
    ///     const ENTITY: &'static str = "user";
    ///     const COLLECTION: Option<&'static str> = Some("users");
    /// }
    ///
    /// let policy = PreparedQueryPolicy::per_entity().cache_entity::<User>();
    ///
    /// assert_eq!(policy.entity_key_prefix(), Some("user"));
    /// assert_eq!(policy.tags_value(), &["users".to_owned()]);
    /// assert_eq!(policy.ttl_value(), Some(Duration::from_secs(300)));
    /// ```
    pub fn cache_entity<T>(self) -> Self
    where
        T: CacheEntity,
    {
        let mut policy = self.entity(T::ENTITY);
        if let Some(tag) = T::COLLECTION {
            policy = policy.collection_tag(tag);
        }
        policy
    }

    /// Set a static collection key and add the same collection invalidation tag.
    pub fn collection(mut self, name: impl ToString) -> Self {
        let tag = collection_tag(name);
        self.key = PreparedQueryKey::Static(tag.clone());
        self.tags = self.tags.tag(tag);
        self
    }

    /// Add one precomputed invalidation tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags = self.tags.tag(tag);
        self
    }

    /// Add a precomputed collection invalidation tag from one escaped segment.
    pub fn collection_tag(mut self, name: impl ToString) -> Self {
        self.tags = self.tags.tag(collection_tag(name));
        self
    }

    /// Add several precomputed invalidation tags.
    pub fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags = self.tags.tags(tags);
        self
    }

    /// Replace precomputed invalidation tags from a reusable [`TagSet`].
    pub fn tag_set(mut self, tags: TagSet) -> Self {
        self.tags = tags;
        self
    }

    /// Set a precomputed per-entry TTL.
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Convert this prepared policy into a runtime [`QueryCachePolicy`].
    ///
    /// Entity-id policies still need [`PreparedQueryPolicy::bind_id`] to set a
    /// key. Static-key and collection policies can use this method directly.
    pub fn to_policy(&self) -> QueryCachePolicy {
        let mut policy = self.base_policy();
        if let PreparedQueryKey::Static(key) = &self.key {
            policy = policy.key(key.clone());
        }
        policy
    }

    /// Bind an id to this prepared policy and produce a runtime policy.
    ///
    /// For entity policies, this creates the final logical key and adds the
    /// entity tag. For static-key policies, the id is ignored and
    /// [`PreparedQueryPolicy::to_policy`] behavior is used.
    pub fn bind_id(&self, id: impl ToString) -> QueryCachePolicy {
        let mut policy = self.to_policy();
        if let PreparedQueryKey::EntityPrefix(prefix) = &self.key {
            let key = format!("{prefix}:{}", escaped_segment(id));
            policy = policy.key(key.clone()).tag(key);
        }
        policy
    }

    fn base_policy(&self) -> QueryCachePolicy {
        let mut policy = QueryCachePolicy::new().tag_set(self.tags.clone());
        if let Some(name) = &self.name {
            policy = policy.with_name(name.clone());
        }
        if let Some(ttl) = self.ttl {
            policy = policy.ttl(ttl);
        }
        policy
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PreparedQueryKey {
    Missing,
    Static(String),
    EntityPrefix(String),
}

fn escaped_segment(segment: impl ToString) -> String {
    CacheKeyBuilder::from_segment(segment).build_string()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use hydracache::TagSet;

    use crate::{CacheEntity, PreparedQueryPolicy};

    struct User;

    impl CacheEntity for User {
        type Id = i64;

        const ENTITY: &'static str = "user";
        const COLLECTION: Option<&'static str> = Some("users");
    }

    #[test]
    fn prepared_static_policy_builds_reusable_runtime_policy() {
        let prepared = PreparedQueryPolicy::named("list-users")
            .collection("users:active")
            .ttl(Duration::from_secs(30));

        assert!(!prepared.requires_id());
        assert_eq!(prepared.name(), Some("list-users"));
        assert_eq!(prepared.static_key_value(), Some("users%3Aactive"));
        assert_eq!(prepared.entity_key_prefix(), None);
        assert_eq!(prepared.tags_value(), &["users%3Aactive".to_owned()]);
        assert_eq!(prepared.ttl_value(), Some(Duration::from_secs(30)));

        let policy = prepared.to_policy();
        assert_eq!(policy.key_value(), Some("users%3Aactive"));
        assert_eq!(policy.tags_value(), &["users%3Aactive".to_owned()]);
        assert_eq!(policy.ttl_value(), Some(Duration::from_secs(30)));
    }

    #[test]
    fn prepared_presets_encode_common_ttl_intent() {
        assert_eq!(
            PreparedQueryPolicy::short_lived().ttl_value(),
            Some(Duration::from_secs(30))
        );
        assert_eq!(
            PreparedQueryPolicy::read_mostly().ttl_value(),
            Some(Duration::from_secs(300))
        );
        assert_eq!(
            PreparedQueryPolicy::per_entity().ttl_value(),
            Some(Duration::from_secs(300))
        );
        assert_eq!(
            PreparedQueryPolicy::no_ttl_explicit_invalidation().ttl_value(),
            None
        );
        assert_eq!(
            PreparedQueryPolicy::negative_cache().ttl_value(),
            Some(Duration::from_secs(30))
        );
    }

    #[test]
    fn prepared_presets_compose_with_bound_entity_metadata() {
        let prepared = PreparedQueryPolicy::per_entity()
            .entity("user")
            .collection_tag("users");
        let policy = prepared.bind_id(42);

        assert_eq!(policy.key_value(), Some("user:42"));
        assert_eq!(
            policy.tags_value(),
            &["users".to_owned(), "user:42".to_owned()]
        );
        assert_eq!(policy.ttl_value(), Some(Duration::from_secs(300)));
    }

    #[test]
    fn prepared_cache_entity_composes_with_presets() {
        let prepared = PreparedQueryPolicy::per_entity()
            .cache_entity::<User>()
            .with_name("load-user");

        assert_eq!(prepared.name(), Some("load-user"));
        assert_eq!(prepared.entity_key_prefix(), Some("user"));
        assert_eq!(prepared.tags_value(), &["users".to_owned()]);
        assert_eq!(prepared.ttl_value(), Some(Duration::from_secs(300)));

        let policy = prepared.bind_id(42);
        assert_eq!(policy.key_value(), Some("user:42"));
        assert_eq!(
            policy.tags_value(),
            &["users".to_owned(), "user:42".to_owned()]
        );
    }

    #[test]
    fn prepared_entity_policy_precomputes_prefix_and_binds_id() {
        let prepared = PreparedQueryPolicy::for_entity("account:user")
            .with_name("load-account-user")
            .collection_tag("users:active");

        assert!(prepared.requires_id());
        assert_eq!(prepared.static_key_value(), None);
        assert_eq!(prepared.entity_key_prefix(), Some("account%3Auser"));
        assert_eq!(prepared.tags_value(), &["users%3Aactive".to_owned()]);

        let policy = prepared.bind_id("42%beta");
        assert_eq!(policy.name(), Some("load-account-user"));
        assert_eq!(policy.key_value(), Some("account%3Auser:42%25beta"));
        assert_eq!(
            policy.tags_value(),
            &[
                "users%3Aactive".to_owned(),
                "account%3Auser:42%25beta".to_owned()
            ]
        );
    }

    #[test]
    fn prepared_cache_entity_policy_reuses_entity_metadata() {
        let prepared = PreparedQueryPolicy::for_cache_entity::<User>()
            .with_name("load-user")
            .ttl(Duration::from_secs(60));

        assert_eq!(prepared.entity_key_prefix(), Some("user"));
        assert_eq!(prepared.tags_value(), &["users".to_owned()]);

        let policy = prepared.bind_id(42);
        assert_eq!(policy.name(), Some("load-user"));
        assert_eq!(policy.key_value(), Some("user:42"));
        assert_eq!(
            policy.tags_value(),
            &["users".to_owned(), "user:42".to_owned()]
        );
        assert_eq!(policy.ttl_value(), Some(Duration::from_secs(60)));
    }

    #[test]
    fn prepared_policy_can_use_custom_static_key_and_tag_set() {
        let prepared = PreparedQueryPolicy::new()
            .key("tenant:7:users")
            .tag_set(TagSet::new().tag("tenant:7").tag("users"));

        let policy = prepared.to_policy();
        assert_eq!(policy.key_value(), Some("tenant:7:users"));
        assert_eq!(
            policy.tags_value(),
            &["tenant:7".to_owned(), "users".to_owned()]
        );
    }
}
