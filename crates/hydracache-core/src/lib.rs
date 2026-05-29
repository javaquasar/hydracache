//! Core types for HydraCache.
//!
//! This crate intentionally contains no database adapter and no distributed runtime.
//! It defines the small set of types shared by the v0 local cache.

use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::time::Duration;

use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

/// HydraCache result type.
pub type Result<T> = std::result::Result<T, CacheError>;

/// A logical cache key.
///
/// v0 treats keys as application-provided strings. Query adapters may later derive
/// these keys from SQL text and typed arguments.
///
/// # Example
///
/// ```rust
/// use hydracache_core::CacheKey;
///
/// let key = CacheKey::new("users:42");
/// assert_eq!(key.as_str(), "users:42");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey<'a>(Cow<'a, str>);

impl<'a> CacheKey<'a> {
    /// Create a new cache key.
    pub fn new(value: impl Into<Cow<'a, str>>) -> Self {
        Self(value.into())
    }

    /// Start building an owned cache key from escaped segments.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache_core::CacheKey;
    ///
    /// let key = CacheKey::builder()
    ///     .segment("tenant:7")
    ///     .segment("users")
    ///     .segment(42)
    ///     .build();
    ///
    /// assert_eq!(key.as_str(), "tenant%3A7:users:42");
    /// ```
    pub fn builder() -> CacheKeyBuilder {
        CacheKeyBuilder::new()
    }

    /// Return the string representation of the key.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert this key into an owned key.
    pub fn into_owned(self) -> CacheKey<'static> {
        CacheKey(Cow::Owned(self.0.into_owned()))
    }
}

/// Builder for cache keys made of escaped `:`-separated segments.
///
/// `segment` escapes `:` and `%`, which keeps a single logical segment from
/// being confused with multiple key segments.
///
/// # Example
///
/// ```rust
/// use hydracache_core::CacheKeyBuilder;
///
/// let key = CacheKeyBuilder::new()
///     .segment("tenant")
///     .segment(7)
///     .entity("user", 42)
///     .build_string();
///
/// assert_eq!(key, "tenant:7:user:42");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CacheKeyBuilder {
    segments: Vec<String>,
}

impl CacheKeyBuilder {
    /// Create an empty key builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a key builder with one initial segment.
    pub fn from_segment(segment: impl ToString) -> Self {
        Self::new().segment(segment)
    }

    /// Append one escaped key segment.
    pub fn segment(mut self, segment: impl ToString) -> Self {
        self.segments.push(escape_segment(&segment.to_string()));
        self
    }

    /// Append multiple escaped key segments.
    pub fn segments<I, S>(mut self, segments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: ToString,
    {
        self.segments.extend(
            segments
                .into_iter()
                .map(|segment| escape_segment(&segment.to_string())),
        );
        self
    }

    /// Append an escaped entity kind and id pair.
    pub fn entity(self, kind: impl ToString, id: impl ToString) -> Self {
        self.segment(kind).segment(id)
    }

    /// Append a `tenant:{id}` prefix.
    pub fn tenant(self, id: impl ToString) -> Self {
        self.segment("tenant").segment(id)
    }

    /// Return whether no segments have been added.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Build an owned [`CacheKey`].
    pub fn build(self) -> CacheKey<'static> {
        CacheKey::new(self.build_string())
    }

    /// Build an owned key string.
    pub fn build_string(self) -> String {
        self.segments.join(":")
    }
}

impl<'a> From<&'a str> for CacheKey<'a> {
    fn from(value: &'a str) -> Self {
        Self::new(value)
    }
}

impl From<String> for CacheKey<'static> {
    fn from(value: String) -> Self {
        Self::new(Cow::Owned(value))
    }
}

impl fmt::Display for CacheKey<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A reusable set of cache invalidation tags.
///
/// # Example
///
/// ```rust
/// use hydracache_core::{CacheOptions, TagSet};
///
/// let tags = TagSet::new()
///     .tag("users")
///     .entity("user", 42)
///     .tenant(7);
///
/// let options = CacheOptions::new().tag_set(tags);
/// assert_eq!(options.tags_value(), &["users".to_owned(), "user:42".to_owned(), "tenant:7".to_owned()]);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TagSet {
    tags: Vec<String>,
}

impl TagSet {
    /// Create an empty tag set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a tag set with one initial tag.
    pub fn from_tag(tag: impl Into<String>) -> Self {
        Self::new().tag(tag)
    }

    /// Add one tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags.
    pub fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags.extend(tags.into_iter().map(Into::into));
        self
    }

    /// Add an entity tag such as `user:42`.
    pub fn entity(self, kind: impl ToString, id: impl ToString) -> Self {
        self.tag(
            CacheKeyBuilder::new()
                .segment(kind)
                .segment(id)
                .build_string(),
        )
    }

    /// Add a tenant tag such as `tenant:7`.
    pub fn tenant(self, id: impl ToString) -> Self {
        self.entity("tenant", id)
    }

    /// Return whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }

    /// Borrow tags as strings.
    pub fn as_slice(&self) -> &[String] {
        &self.tags
    }

    /// Convert into a vector of tags.
    pub fn into_vec(self) -> Vec<String> {
        self.tags
    }
}

impl IntoIterator for TagSet {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.tags.into_iter()
    }
}

/// Per-entry cache behavior.
///
/// Options are passed to `put`, `get_or_load`, and loader helper methods.
///
/// # Example
///
/// ```rust
/// use std::time::Duration;
///
/// use hydracache_core::CacheOptions;
///
/// let options = CacheOptions::new()
///     .ttl(Duration::from_secs(60))
///     .tags(["users", "user:42"]);
///
/// assert_eq!(options.ttl_value(), Some(Duration::from_secs(60)));
/// assert_eq!(options.tags_value(), &["users".to_owned(), "user:42".to_owned()]);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CacheOptions {
    ttl: Option<Duration>,
    tags: Vec<String>,
}

impl CacheOptions {
    /// Create empty cache options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a per-entry TTL.
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Attach one tag used by `invalidate_tag`.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Attach tags used by `invalidate_tag`.
    pub fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    /// Replace tags from a [`TagSet`].
    pub fn tag_set(mut self, tags: TagSet) -> Self {
        self.tags = tags.into_vec();
        self
    }

    /// Return the configured TTL, if any.
    pub fn ttl_value(&self) -> Option<Duration> {
        self.ttl
    }

    /// Return tags attached to this entry.
    pub fn tags_value(&self) -> &[String] {
        &self.tags
    }
}

fn escape_segment(segment: &str) -> String {
    let mut escaped = String::with_capacity(segment.len());
    for ch in segment.chars() {
        match ch {
            '%' => escaped.push_str("%25"),
            ':' => escaped.push_str("%3A"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

/// Snapshot of lightweight cache counters.
///
/// The counters are intentionally lightweight and approximate enough for local
/// observability. They are not intended to be a durable metrics store.
///
/// # Example
///
/// ```rust
/// use hydracache_core::CacheStats;
///
/// let stats = CacheStats::default();
/// assert_eq!(stats.hits, 0);
/// assert_eq!(stats.single_flight_joins, 0);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CacheStats {
    /// Successful cache lookups.
    pub hits: u64,
    /// Cache lookups that did not return a usable value.
    pub misses: u64,
    /// Loader closures executed by `get_or_load`.
    pub loads: u64,
    /// Calls that joined an already running single-flight load.
    pub single_flight_joins: u64,
    /// Loader results skipped because their invalidation generation became stale.
    pub stale_load_discards: u64,
    /// Entries removed by invalidation APIs.
    pub invalidations: u64,
    /// Entries observed as evicted by the backend.
    ///
    /// v0 does not wire backend eviction listeners yet, so this remains zero.
    pub evictions: u64,
}

/// Serialization boundary for cached values.
///
/// Implement this trait to replace the default [`PostcardCodec`].
///
/// # Example
///
/// ```rust
/// use hydracache_core::{CacheCodec, PostcardCodec};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
/// struct User {
///     id: u64,
/// }
///
/// let codec = PostcardCodec;
/// let bytes = codec.encode(&User { id: 1 }).unwrap();
/// let decoded: User = codec.decode(&bytes).unwrap();
///
/// assert_eq!(decoded, User { id: 1 });
/// ```
pub trait CacheCodec: Clone + Send + Sync + 'static {
    /// Encode a typed value into bytes.
    fn encode<T>(&self, value: &T) -> Result<Bytes>
    where
        T: Serialize;

    /// Decode bytes back into a typed value.
    fn decode<T>(&self, bytes: &Bytes) -> Result<T>
    where
        T: DeserializeOwned;
}

/// Default compact binary codec for v0.
///
/// `PostcardCodec` is compact and works well for local cache values that derive
/// `serde::Serialize` and `serde::Deserialize`.
#[derive(Debug, Clone, Copy, Default)]
pub struct PostcardCodec;

impl CacheCodec for PostcardCodec {
    fn encode<T>(&self, value: &T) -> Result<Bytes>
    where
        T: Serialize,
    {
        postcard::to_allocvec(value)
            .map(Bytes::from)
            .map_err(|source| CacheError::Encode(source.to_string()))
    }

    fn decode<T>(&self, bytes: &Bytes) -> Result<T>
    where
        T: DeserializeOwned,
    {
        postcard::from_bytes(bytes).map_err(|source| CacheError::Decode(source.to_string()))
    }
}

/// Errors returned by HydraCache.
///
/// # Example
///
/// ```rust
/// use hydracache_core::CacheError;
///
/// let error = CacheError::Backend("store unavailable".to_owned());
/// assert_eq!(error.to_string(), "cache backend error: store unavailable");
/// ```
#[derive(Debug, Clone, thiserror::Error)]
pub enum CacheError {
    /// Failed to encode a value before storing it.
    #[error("cache encode error: {0}")]
    Encode(String),

    /// Failed to decode a value read from the cache.
    #[error("cache decode error: {0}")]
    Decode(String),

    /// Loader returned an error.
    #[error("cache loader error: {0}")]
    Loader(String),

    /// Backend or internal error.
    #[error("cache backend error: {0}")]
    Backend(String),
}

impl CacheError {
    /// Wrap a loader error.
    pub fn loader<E>(source: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self::Loader(source.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_builder_new_is_empty() {
        let builder = CacheKeyBuilder::new();

        assert!(builder.is_empty());
        assert_eq!(builder.clone().build_string(), "");
        assert_eq!(builder.build().as_str(), "");
    }

    #[test]
    fn key_builder_from_segment_adds_initial_segment() {
        let key = CacheKeyBuilder::from_segment("users").build_string();

        assert_eq!(key, "users");
    }

    #[test]
    fn key_builder_segment_escapes_colon_and_percent() {
        let key = CacheKeyBuilder::new()
            .segment("tenant:7")
            .segment("percent%value")
            .build_string();

        assert_eq!(key, "tenant%3A7:percent%25value");
    }

    #[test]
    fn key_builder_segments_preserve_order() {
        let key = CacheKeyBuilder::new()
            .segments(["tenant", "7", "users"])
            .build_string();

        assert_eq!(key, "tenant:7:users");
    }

    #[test]
    fn key_builder_entity_and_tenant_append_pairs() {
        let key = CacheKeyBuilder::new()
            .tenant(7)
            .entity("user", 42)
            .build_string();

        assert_eq!(key, "tenant:7:user:42");
    }

    #[test]
    fn cache_key_builder_constructor_matches_direct_builder() {
        let key = CacheKey::builder().entity("user", 42).build();

        assert_eq!(key.as_str(), "user:42");
        assert_eq!(key.to_string(), "user:42");
    }

    #[test]
    fn tag_set_new_is_empty() {
        let tags = TagSet::new();

        assert!(tags.is_empty());
        assert!(tags.as_slice().is_empty());
        assert!(tags.into_vec().is_empty());
    }

    #[test]
    fn tag_set_from_tag_adds_initial_tag() {
        let tags = TagSet::from_tag("users");

        assert_eq!(tags.as_slice(), &["users".to_owned()]);
    }

    #[test]
    fn tag_set_tags_entity_and_tenant_preserve_order() {
        let tags = TagSet::new()
            .tags(["users", "active"])
            .entity("user", 42)
            .tenant(7);

        assert_eq!(
            tags.as_slice(),
            &[
                "users".to_owned(),
                "active".to_owned(),
                "user:42".to_owned(),
                "tenant:7".to_owned()
            ]
        );
    }

    #[test]
    fn tag_set_entity_escapes_segments() {
        let tags = TagSet::new().entity("user:type", "42%beta");

        assert_eq!(tags.as_slice(), &["user%3Atype:42%25beta".to_owned()]);
    }

    #[test]
    fn tag_set_into_iterator_yields_owned_tags() {
        let tags: Vec<_> = TagSet::new()
            .tag("users")
            .tag("admins")
            .into_iter()
            .collect();

        assert_eq!(tags, vec!["users".to_owned(), "admins".to_owned()]);
    }

    #[test]
    fn cache_options_tag_set_replaces_existing_tags() {
        let options = CacheOptions::new()
            .tag("old")
            .tag_set(TagSet::new().tag("new").entity("user", 42));

        assert_eq!(
            options.tags_value(),
            &["new".to_owned(), "user:42".to_owned()]
        );
    }
}
