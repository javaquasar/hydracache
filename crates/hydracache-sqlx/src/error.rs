use std::error::Error;

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

/// Error returned by SQLx transaction companion helpers.
#[derive(Debug, Error)]
pub enum SqlxTransactionError<E>
where
    E: Error + Send + Sync + 'static,
{
    /// The user-supplied transaction body failed.
    #[error("SQLx transaction body failed: {0}")]
    Body(E),
    /// SQLx failed while beginning, committing, or rolling back the transaction.
    #[error("SQLx transaction operation failed: {0}")]
    Sqlx(#[from] sqlx::Error),
    /// Durable outbox enqueue failed before commit.
    #[error("SQLx transaction outbox enqueue failed: {0}")]
    Outbox(#[source] DbCacheError),
    /// Local non-durable invalidation failed after commit.
    #[error("SQLx transaction local invalidation failed: {0}")]
    LocalInvalidation(#[source] hydracache::CacheError),
    /// Durable mode was requested without an outbox.
    #[error("SQLx transaction durable mode requires SqlxInvalidationOutbox")]
    MissingOutbox,
}

/// SQLx transaction companion result type.
pub type TransactionResult<T, E> = std::result::Result<T, SqlxTransactionError<E>>;
