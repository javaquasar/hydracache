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
//! use hydracache_sqlx::{DbCache, HydraCacheEntity, PreparedQueryPolicy, SqlxQueryExt};
//!
//! #[derive(serde::Serialize, serde::Deserialize, HydraCacheEntity)]
//! #[hydracache(entity = "user", collection = "users")]
//! struct User {
//!     #[hydracache(id)]
//!     id: i64,
//!     name: String,
//! }
//!
//! # async fn example(pool: sqlx::PgPool) -> hydracache_sqlx::Result<()> {
//! let local = HydraCache::local().build();
//!
//! // SQLx users may import DbCache from this crate, but the type itself is
//! // database-neutral and comes from hydracache-db.
//! let queries = DbCache::new(local, "db");
//!
//! let user: User = queries
//!     .for_entity::<User>(42)
//!     .fetch_with(move || async move {
//!         let (id, name): (i64, String) =
//!             sqlx::query_as("select id, name from users where id = $1")
//!                 .bind(42_i64)
//!                 .fetch_one(&pool)
//!                 .await?;
//!
//!         Ok::<_, sqlx::Error>(User { id, name })
//!     })
//!     .await?;
//!
//! assert_eq!(user.id, 42);
//! assert!(!user.name.is_empty());
//! # Ok(())
//! # }
//! ```
//!
//! Prepared policies keep repeated repository methods cheap while still using
//! ordinary SQLx query execution on cache misses:
//!
//! ```no_run
//! use hydracache::HydraCache;
//! use hydracache_sqlx::{DbCache, HydraCacheEntity, PreparedQueryPolicy, SqlxQueryExt};
//!
//! #[derive(serde::Serialize, serde::Deserialize, HydraCacheEntity)]
//! #[hydracache(entity = "user", collection = "users")]
//! struct User {
//!     #[hydracache(id)]
//!     id: i64,
//!     name: String,
//! }
//!
//! # async fn example(pool: sqlx::PgPool) -> hydracache_sqlx::Result<()> {
//! let queries = DbCache::new(HydraCache::local().build(), "db");
//! let load_user = queries.prepare::<(i64, String)>(
//!     PreparedQueryPolicy::for_cache_entity::<User>().with_name("load-user"),
//! );
//!
//! let (id, name) = load_user
//!     .for_id(42)
//!     .sqlx_one(
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
//!
//! [`QueryCachePolicy`] and [`PreparedQueryPolicy`] are also re-exported for
//! SQLx users, but the policy types are database-neutral and live in
//! `hydracache-db`.
//! [`query_cache_policy!`] is re-exported for the same convenience.

extern crate self as hydracache_sqlx;

mod error;
mod query_ext;

pub use error::{Result, SqlxCacheError};
pub use hydracache_db::{
    prepared_query_policy, query_cache_policy, CacheEntity, CacheKeyBuilder, DbAdapterKind,
    DbCache, DbCacheError, DbOperationContext, DbQuery, DbResultShape, HydraCacheEntity,
    PreparedDbQuery, PreparedQueryPolicy, QueryCachePolicy, RefreshPolicy, Result as DbResult,
};
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

    use crate::{
        prepared_query_policy, query_cache_policy, CacheKeyBuilder, DbCache, PreparedQueryPolicy,
        QueryCachePolicy, RefreshPolicy, SqlxCache, SqlxQueryExt,
    };

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
    async fn query_cache_policy_reexport_is_available_from_sqlx_crate() {
        let refresh =
            RefreshPolicy::new().stale_while_revalidate(std::time::Duration::from_secs(5));
        let policy = QueryCachePolicy::new()
            .key("user:1")
            .tag("user:1")
            .refresh_policy(refresh);
        let query = DbCache::new(HydraCache::local().build(), "db").cached_with::<User>(policy);

        assert_eq!(query.physical_key(), Some("db:user:1".to_owned()));
        assert_eq!(query.tags_value(), &["user:1".to_owned()]);
        assert_eq!(query.refresh_policy_value(), Some(refresh));
    }

    #[test]
    fn query_policy_segment_macro_reexport_uses_sqlx_crate_paths() {
        let policy = query_cache_policy!(
            name = "search-users",
            key_segments = ["tenant", 7_u64, "q", "ada:lovelace"],
            tag_segments = [["tenant", 7_u64], ["users"]],
            ttl_secs = 30,
        );
        let expected_key = CacheKeyBuilder::new()
            .segment("tenant")
            .segment(7_u64)
            .segment("q")
            .segment("ada:lovelace")
            .build_string();

        assert_eq!(policy.name(), Some("search-users"));
        assert_eq!(policy.key_value(), Some(expected_key.as_str()));
        assert_eq!(
            policy.tags_value(),
            &["tenant:7".to_owned(), "users".to_owned()]
        );
    }

    #[tokio::test]
    async fn prepared_query_policy_reexport_is_available_from_sqlx_crate() {
        let policy = prepared_query_policy!(
            entity = "user",
            name = "load-user",
            collection_tag = "users",
        );
        let expected = PreparedQueryPolicy::for_entity("user")
            .with_name("load-user")
            .collection_tag("users");
        assert_eq!(policy, expected);

        let prepared = DbCache::new(HydraCache::local().build(), "db").prepare::<User>(policy);

        let query = prepared.for_id(1);
        assert_eq!(query.name(), Some("load-user"));
        assert_eq!(query.physical_key(), Some("db:user:1".to_owned()));
        assert_eq!(
            query.tags_value(),
            &["users".to_owned(), "user:1".to_owned()]
        );
    }

    #[tokio::test]
    async fn sqlx_helper_missing_key_returns_sqlx_cache_error() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost/postgres")
            .unwrap();

        let result = DbCache::new(HydraCache::local().build(), "db")
            .cached::<(i64,)>()
            .sqlx_one(pool, sqlx::query_as("select 1"))
            .await;

        let error = result.unwrap_err();
        assert_eq!(
            error.to_string(),
            "database cached operation `db:unnamed` is missing an explicit cache key (adapter=sqlx, namespace=db, result_shape=one)"
        );
    }

    #[tokio::test]
    async fn sqlx_cache_error_wraps_db_cache_errors() {
        let error = hydracache_db::DbCacheError::MissingKey {
            operation: "load-user".to_owned(),
            adapter: hydracache_db::DbAdapterKind::Sqlx,
            namespace: "db".to_owned(),
            result_shape: hydracache_db::DbResultShape::One,
        };
        let error = crate::SqlxCacheError::from(error);

        assert_eq!(
            error.to_string(),
            "database cached operation `load-user` is missing an explicit cache key (adapter=sqlx, namespace=db, result_shape=one)"
        );
    }
}
