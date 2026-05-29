//! SQLx-facing integration crate for HydraCache database result caching.
//!
//! The database-neutral query cache API lives in `hydracache-db`. This crate
//! keeps SQLx users on a convenient import path while avoiding a hard conceptual
//! dependency between the generic adapter and SQLx itself.
//!
//! # Example
//!
//! ```rust
//! use hydracache::HydraCache;
//! use hydracache_sqlx::DbCache;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
//! struct User {
//!     id: i64,
//!     name: String,
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache_sqlx::Result<()> {
//! let local = HydraCache::local().build();
//!
//! // SQLx users may import DbCache from this crate, but the type itself is
//! // database-neutral and comes from hydracache-db.
//! let queries = DbCache::new(local, "db");
//!
//! let user = queries
//!     .cached::<User>()
//!     .key("user:42")
//!     .tag("user:42")
//!     .fetch_with(|| async {
//!         // This loader runs only on a cache miss. On a cache hit, HydraCache
//!         // returns the cached User and this SQLx code is not executed.
//!         Ok::<_, std::io::Error>(User {
//!             id: 42,
//!             name: "Ada".to_owned(),
//!         })
//!     })
//!     .await?;
//!
//! assert_eq!(user.id, 42);
//! # Ok(())
//! # }
//! ```

pub use hydracache_db::{DbCache, DbCacheError, DbQuery, Result};

/// SQLx-specific compatibility name for [`DbCache`].
pub type SqlxCache<C = hydracache::PostcardCodec> = DbCache<C>;

/// SQLx-specific compatibility name for [`DbQuery`].
pub type SqlxQuery<T, C = hydracache::PostcardCodec> = DbQuery<T, C>;

/// SQLx-specific compatibility name for [`DbCacheError`].
pub type SqlxCacheError = DbCacheError;

/// Re-export the SQLx crate used by this adapter.
///
/// This lets downstream users keep one adapter-aligned SQLx version in examples
/// and integration code without hiding SQLx behind HydraCache abstractions.
pub use sqlx;

#[cfg(test)]
mod tests {
    use hydracache::HydraCache;
    use serde::{Deserialize, Serialize};

    use crate::{DbCache, SqlxCache};

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
}
