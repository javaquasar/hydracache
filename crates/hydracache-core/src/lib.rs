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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CacheStats {
    /// Successful cache lookups.
    pub hits: u64,
    /// Cache lookups that did not return a usable value.
    pub misses: u64,
    /// Loader closures executed by `get_or_load`.
    pub loads: u64,
    /// Entries removed by invalidation APIs.
    pub invalidations: u64,
    /// Entries observed as evicted by the backend.
    ///
    /// v0 does not wire backend eviction listeners yet, so this remains zero.
    pub evictions: u64,
}

/// Serialization boundary for cached values.
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
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    /// Failed to encode a value before storing it.
    #[error("cache encode error: {0}")]
    Encode(String),

    /// Failed to decode a value read from the cache.
    #[error("cache decode error: {0}")]
    Decode(String),

    /// Loader returned an error.
    #[error("cache loader error: {0}")]
    Loader(#[source] Box<dyn Error + Send + Sync>),

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
        Self::Loader(Box::new(source))
    }
}
