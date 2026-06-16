use std::fmt;

use thiserror::Error;

/// Database adapter kind attached to operation diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbAdapterKind {
    /// Database-neutral repository or custom loader path.
    Generic,
    /// SQLx-facing adapter helper.
    Sqlx,
    /// Diesel-facing adapter helper.
    Diesel,
    /// SeaORM-facing adapter helper.
    SeaOrm,
}

impl fmt::Display for DbAdapterKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Generic => "generic",
            Self::Sqlx => "sqlx",
            Self::Diesel => "diesel",
            Self::SeaOrm => "seaorm",
        })
    }
}

/// Cached database result shape attached to operation diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbResultShape {
    /// Exactly one row or value.
    One,
    /// Optional row or value.
    Optional,
    /// Collection result.
    All,
    /// Repository/custom result shape selected by the caller.
    Custom,
}

impl fmt::Display for DbResultShape {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::One => "one",
            Self::Optional => "optional",
            Self::All => "all",
            Self::Custom => "custom",
        })
    }
}

/// Diagnostic context for one database cache operation.
///
/// The context is deliberately database-neutral. It describes the cache-side
/// operation and adapter helper while the database library keeps ownership of
/// typed database errors, transactions, query construction, and row mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbOperationContext {
    /// Adapter helper that initiated the operation.
    pub adapter: DbAdapterKind,
    /// Diagnostic operation name.
    pub operation: String,
    /// Cache namespace used to build physical keys.
    pub namespace: String,
    /// Physical cache key when one was available.
    pub physical_key: Option<String>,
    /// Result shape requested by the caller or adapter helper.
    pub result_shape: DbResultShape,
}

impl fmt::Display for DbOperationContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let namespace = if self.namespace.is_empty() {
            "<empty>"
        } else {
            &self.namespace
        };
        let physical_key = self.physical_key.as_deref().unwrap_or("<missing>");

        write!(
            formatter,
            "adapter={}, namespace={}, key={}, result_shape={}",
            self.adapter, namespace, physical_key, self.result_shape
        )
    }
}

/// Error type returned by database cache adapter helpers.
#[derive(Debug, Error)]
pub enum DbCacheError {
    /// A cached database operation cannot run without an explicit cache key.
    #[error(
        "database cached operation `{operation}` is missing an explicit cache key \
         (adapter={adapter}, namespace={namespace}, result_shape={result_shape})"
    )]
    MissingKey {
        /// Diagnostic operation name.
        operation: String,
        /// Adapter helper that initiated the operation.
        adapter: DbAdapterKind,
        /// Cache namespace used for physical cache keys.
        namespace: String,
        /// Result shape requested by the caller or adapter helper.
        result_shape: DbResultShape,
    },

    /// The underlying cache operation failed with database-cache context.
    #[error("database cached operation `{operation}` failed ({context}): {source}")]
    Operation {
        /// Diagnostic operation name.
        operation: String,
        /// Database cache operation context.
        context: DbOperationContext,
        /// Underlying cache-layer error.
        #[source]
        source: hydracache::CacheError,
    },

    /// The underlying HydraCache operation failed.
    #[error(transparent)]
    Cache(#[from] hydracache::CacheError),
}

impl DbCacheError {
    pub(crate) fn operation(context: DbOperationContext, source: hydracache::CacheError) -> Self {
        Self::Operation {
            operation: context.operation.clone(),
            context,
            source,
        }
    }
}

/// Database cache adapter result type.
pub type Result<T> = std::result::Result<T, DbCacheError>;
