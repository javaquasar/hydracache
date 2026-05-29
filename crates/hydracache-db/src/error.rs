use thiserror::Error;

/// Error type returned by database cache adapter helpers.
#[derive(Debug, Error)]
pub enum DbCacheError {
    /// A cached database operation cannot run without an explicit cache key.
    #[error("database cached operation `{operation}` is missing an explicit cache key")]
    MissingKey { operation: String },

    /// The underlying HydraCache operation failed.
    #[error(transparent)]
    Cache(#[from] hydracache::CacheError),
}

/// Database cache adapter result type.
pub type Result<T> = std::result::Result<T, DbCacheError>;
