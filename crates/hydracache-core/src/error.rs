use std::error::Error;

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
