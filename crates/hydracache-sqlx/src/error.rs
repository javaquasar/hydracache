use thiserror::Error;

/// Error type returned by the SQLx adapter helpers.
#[derive(Debug, Error)]
pub enum SqlxCacheError {
    /// A cached query cannot run without an explicit cache key.
    #[error("SQLx cache query `{sql}` is missing an explicit cache key")]
    MissingKey { sql: String },

    /// The underlying HydraCache operation failed.
    #[error(transparent)]
    Cache(#[from] hydracache::CacheError),
}

/// SQLx adapter result type.
pub type Result<T> = std::result::Result<T, SqlxCacheError>;
