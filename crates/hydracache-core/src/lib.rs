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

    /// Return the string representation of the key.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert this key into an owned key.
    pub fn into_owned(self) -> CacheKey<'static> {
        CacheKey(Cow::Owned(self.0.into_owned()))
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

    /// Return the configured TTL, if any.
    pub fn ttl_value(&self) -> Option<Duration> {
        self.ttl
    }

    /// Return tags attached to this entry.
    pub fn tags_value(&self) -> &[String] {
        &self.tags
    }
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
