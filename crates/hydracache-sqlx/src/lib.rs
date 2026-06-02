//! SQLx-facing integration crate for HydraCache database result caching.
//!
//! The database-neutral query cache API lives in `hydracache-db`. This crate
//! keeps SQLx users on a convenient import path while avoiding a hard conceptual
//! dependency between the generic adapter and SQLx itself.
//!
//! # Example
//!
//! ```no_run
//! use hydracache::HydraCache;
//! use hydracache_sqlx::{DbCache, SqlxQueryExt};
//!
//! # async fn example(pool: sqlx::PgPool) -> hydracache_sqlx::Result<()> {
//! let local = HydraCache::local().build();
//!
//! // SQLx users may import DbCache from this crate, but the type itself is
//! // database-neutral and comes from hydracache-db.
//! let queries = DbCache::new(local, "db");
//!
//! let (id, name): (i64, String) = queries
//!     .entity::<(i64, String)>("user", 42)
//!     .collection_tag("users")
//!     .fetch_one(
//!         pool.clone(),
//!         sqlx::query_as("select id, name from users where id = $1").bind(42_i64),
//!     )
//!     .await?;
//!
//! assert_eq!(id, 42);
//! assert!(!name.is_empty());
//! # Ok(())
//! # }
//! ```
//!
//! Use [`DbQuery::fetch_with`] when you need SQLx macros, transactions, or a
//! repository function instead of a pool-like executor.

mod error;
mod query_ext;

pub use error::{Result, SqlxCacheError};
pub use hydracache_db::{CacheEntity, DbCache, DbCacheError, DbQuery, Result as DbResult};
pub use query_ext::SqlxQueryExt;

/// SQLx-specific compatibility name for [`DbCache`].
pub type SqlxCache<C = hydracache::PostcardCodec> = DbCache<C>;

/// SQLx-specific compatibility name for [`DbQuery`].
pub type SqlxQuery<T, C = hydracache::PostcardCodec> = DbQuery<T, C>;

/// Re-export the SQLx crate used by this adapter.
///
/// This lets downstream users keep one adapter-aligned SQLx version in examples
/// and integration code without hiding SQLx behind HydraCache abstractions.
pub use sqlx;

#[cfg(test)]
mod tests {
    use hydracache::HydraCache;
    use serde::{Deserialize, Serialize};
    use sqlx::postgres::PgPoolOptions;

    use crate::{DbCache, SqlxCache, SqlxQueryExt};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct User {
        id: u64,
    }

    #[tokio::test]
    async fn sqlx_cache_alias_matches_database_cache_api() {
        let query = SqlxCache::new(HydraCache::local().build(), "sqlx")
            .cached::<User>()
            .key("user:1");

        assert_eq!(query.physical_key(), Some("sqlx:user:1".to_owned()));
    }

    #[tokio::test]
    async fn db_cache_reexport_is_available_from_sqlx_crate() {
        let query = DbCache::new(HydraCache::local().build(), "db")
            .cached::<User>()
            .key("user:1");

        assert_eq!(query.physical_key(), Some("db:user:1".to_owned()));
    }

    #[tokio::test]
    async fn sqlx_helper_missing_key_returns_sqlx_cache_error() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost/postgres")
            .unwrap();

        let result = DbCache::new(HydraCache::local().build(), "db")
            .cached::<(i64,)>()
            .fetch_one(pool, sqlx::query_as("select 1"))
            .await;

        let error = result.unwrap_err();
        assert_eq!(
            error.to_string(),
            "database cached operation `db:unnamed` is missing an explicit cache key"
        );
    }

    #[tokio::test]
    async fn sqlx_cache_error_wraps_db_cache_errors() {
        let error = hydracache_db::DbCacheError::MissingKey {
            operation: "load-user".to_owned(),
        };
        let error = crate::SqlxCacheError::from(error);

        assert_eq!(
            error.to_string(),
            "database cached operation `load-user` is missing an explicit cache key"
        );
    }
}
