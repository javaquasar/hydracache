use hydracache_db::DbCacheError;
use thiserror::Error;

/// Error type returned by SQLx-facing cache helpers.
#[derive(Debug, Error)]
pub enum SqlxCacheError {
    /// The generic database cache adapter or underlying cache failed.
    #[error(transparent)]
    Cache(#[from] DbCacheError),
}

/// SQLx adapter result type.
pub type Result<T> = std::result::Result<T, SqlxCacheError>;
