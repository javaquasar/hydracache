//! Diesel-facing integration crate for HydraCache database result caching.
//!
//! The database-neutral query cache API lives in `hydracache-db`. This crate
//! keeps Diesel users on a convenient import path while avoiding a Diesel
//! dependency in the core cache adapter.
//!
//! Diesel is synchronous, so these helpers run the supplied loader on
//! `tokio::task::spawn_blocking`. The loader should own or acquire its
//! connection inside the closure.
//!
//! # Example
//!
//! ```rust
//! use hydracache::HydraCache;
//! use hydracache_diesel::{DieselCache, DieselQueryExt};
//!
//! # async fn example() -> hydracache_diesel::Result<()> {
//! let queries = DieselCache::new(HydraCache::local().build(), "diesel");
//!
//! let user_name = queries
//!     .entity::<String>("user", 42)
//!     .collection_tag("users")
//!     .diesel_one(move || Ok::<_, hydracache_diesel::diesel::result::Error>("Ada".to_owned()))
//!     .await?;
//!
//! assert_eq!(user_name, "Ada");
//! # Ok(())
//! # }
//! ```
//!
//! Use [`DbQuery::fetch_with`] when you need custom repository code,
//! transactions, or an async Diesel wrapper.
//!
//! [`DbQuery::fetch_with`]: hydracache_db::DbQuery::fetch_with

extern crate self as hydracache_diesel;

use async_trait::async_trait;
use diesel::result::Error as DieselError;
use hydracache_core::CacheCodec;
use hydracache_db::DbQuery;
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

pub use hydracache_db::{
    query_cache_policy, CacheEntity, DbCache, DbCacheError, DbQuery as GenericDbQuery,
    HydraCacheEntity, PreparedDbQuery, PreparedQueryPolicy, QueryCachePolicy, Result as DbResult,
};

/// Diesel-specific compatibility name for [`DbCache`].
pub type DieselCache<C = hydracache::PostcardCodec> = DbCache<C>;

/// Diesel-specific compatibility name for [`DbQuery`].
pub type DieselQuery<T, C = hydracache::PostcardCodec> = DbQuery<T, C>;

/// Re-export the Diesel crate used by this adapter.
pub use diesel;

/// Error type returned by Diesel-facing cache helpers.
#[derive(Debug, Error)]
pub enum DieselCacheError {
    /// The generic database cache adapter or underlying cache failed.
    #[error(transparent)]
    Cache(#[from] DbCacheError),
}

/// Diesel adapter result type.
pub type Result<T> = std::result::Result<T, DieselCacheError>;

#[derive(Debug, Error)]
enum DieselLoaderError {
    #[error(transparent)]
    Query(#[from] DieselError),
    #[error("blocking Diesel worker failed: {0}")]
    Worker(#[from] tokio::task::JoinError),
}

/// Convenience Diesel execution methods for [`DbQuery`].
///
/// These helpers keep Diesel responsible for query construction and row
/// mapping, while HydraCache owns keying, tags, TTL, serialization, and local
/// single-flight. The loader is executed with `spawn_blocking`, so it should
/// acquire a connection from a pool or otherwise own the connection source.
#[async_trait]
pub trait DieselQueryExt<T, C>
where
    C: CacheCodec,
{
    /// Execute a blocking Diesel loader on miss and cache exactly one row.
    async fn diesel_one<F>(self, loader: F) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> diesel::QueryResult<T> + Send + 'static;

    /// Execute a blocking Diesel loader on miss and cache either one row or
    /// `None` when Diesel reports `NotFound`.
    async fn diesel_optional<F>(self, loader: F) -> Result<Option<T>>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> diesel::QueryResult<T> + Send + 'static;

    /// Execute a blocking Diesel loader on miss and cache all returned rows.
    async fn diesel_all<F>(self, loader: F) -> Result<Vec<T>>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> diesel::QueryResult<Vec<T>> + Send + 'static;
}

#[async_trait]
impl<T, C> DieselQueryExt<T, C> for DbQuery<T, C>
where
    C: CacheCodec,
{
    async fn diesel_one<F>(self, loader: F) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> diesel::QueryResult<T> + Send + 'static,
    {
        self.fetch_value_with(move || async move { run_blocking(loader).await })
            .await
            .map_err(Into::into)
    }

    async fn diesel_optional<F>(self, loader: F) -> Result<Option<T>>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> diesel::QueryResult<T> + Send + 'static,
    {
        self.fetch_value_with(move || async move {
            match run_blocking(loader).await {
                Ok(value) => Ok(Some(value)),
                Err(DieselLoaderError::Query(DieselError::NotFound)) => Ok(None),
                Err(error) => Err(error),
            }
        })
        .await
        .map_err(Into::into)
    }

    async fn diesel_all<F>(self, loader: F) -> Result<Vec<T>>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> diesel::QueryResult<Vec<T>> + Send + 'static,
    {
        self.fetch_value_with(move || async move { run_blocking(loader).await })
            .await
            .map_err(Into::into)
    }
}

async fn run_blocking<T, F>(loader: F) -> std::result::Result<T, DieselLoaderError>
where
    T: Send + 'static,
    F: FnOnce() -> diesel::QueryResult<T> + Send + 'static,
{
    tokio::task::spawn_blocking(loader)
        .await
        .map_err(DieselLoaderError::from)?
        .map_err(DieselLoaderError::from)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use diesel::prelude::*;
    use diesel::result::Error as DieselError;
    use diesel::sqlite::SqliteConnection;
    use hydracache::HydraCache;
    use serde::{Deserialize, Serialize};

    use super::{
        query_cache_policy, CacheEntity, DieselCache, DieselQueryExt, HydraCacheEntity,
        PreparedQueryPolicy, QueryCachePolicy,
    };

    diesel::table! {
        users (id) {
            id -> BigInt,
            name -> Text,
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Queryable, Serialize, Deserialize, HydraCacheEntity)]
    #[hydracache(entity = "diesel-user", collection = "diesel-users", id = i64)]
    struct User {
        id: i64,
        name: String,
    }

    fn sqlite_users() -> Arc<Mutex<SqliteConnection>> {
        let mut connection = SqliteConnection::establish(":memory:").unwrap();
        diesel::sql_query("create table users (id bigint primary key, name text not null)")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query("insert into users (id, name) values (42, 'Ada'), (7, 'Grace')")
            .execute(&mut connection)
            .unwrap();
        Arc::new(Mutex::new(connection))
    }

    fn diesel_user_loader(
        connection: Arc<Mutex<SqliteConnection>>,
        calls: Arc<AtomicUsize>,
        user_id: i64,
    ) -> impl FnOnce() -> diesel::QueryResult<User> + Send + 'static {
        move || {
            calls.fetch_add(1, Ordering::SeqCst);
            let mut connection = connection.lock().expect("sqlite connection poisoned");
            users::table.find(user_id).first::<User>(&mut *connection)
        }
    }

    #[tokio::test]
    async fn diesel_one_caches_real_sqlite_query_until_invalidation() {
        let connection = sqlite_users();
        let calls = Arc::new(AtomicUsize::new(0));
        let queries = DieselCache::new(HydraCache::local().build(), "diesel");

        let first = queries
            .entity::<User>("diesel-user", 42)
            .collection_tag("diesel-users")
            .diesel_one(diesel_user_loader(connection.clone(), calls.clone(), 42))
            .await
            .unwrap();

        assert_eq!(first.name, "Ada");
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        diesel::sql_query("update users set name = 'Updated' where id = 42")
            .execute(&mut *connection.lock().unwrap())
            .unwrap();

        let cached = queries
            .entity::<User>("diesel-user", 42)
            .collection_tag("diesel-users")
            .diesel_one(diesel_user_loader(connection.clone(), calls.clone(), 42))
            .await
            .unwrap();

        assert_eq!(cached.name, "Ada");
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        assert_eq!(
            queries
                .cache()
                .invalidate_tag("diesel-users")
                .await
                .unwrap(),
            1
        );

        let reloaded = queries
            .entity::<User>("diesel-user", 42)
            .collection_tag("diesel-users")
            .diesel_one(diesel_user_loader(connection, calls.clone(), 42))
            .await
            .unwrap();

        assert_eq!(reloaded.name, "Updated");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn diesel_optional_caches_not_found_without_reloading() {
        let connection = sqlite_users();
        let calls = Arc::new(AtomicUsize::new(0));
        let queries = DieselCache::new(HydraCache::local().build(), "diesel");

        let missing = queries
            .entity::<User>("diesel-user", 404)
            .diesel_optional(diesel_user_loader(connection.clone(), calls.clone(), 404))
            .await
            .unwrap();
        let cached_missing = queries
            .entity::<User>("diesel-user", 404)
            .diesel_optional(diesel_user_loader(connection, calls.clone(), 404))
            .await
            .unwrap();

        assert_eq!(missing, None);
        assert_eq!(cached_missing, None);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn missing_key_fails_before_running_diesel_loader() {
        let calls = Arc::new(AtomicUsize::new(0));
        let queries = DieselCache::new(HydraCache::local().build(), "diesel");

        let result = queries
            .cached::<User>()
            .diesel_one({
                let calls = calls.clone();
                move || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Err(DieselError::NotFound)
                }
            })
            .await;

        let error = result.expect_err("query without a key should fail");
        assert!(error.to_string().contains("missing an explicit cache key"));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn diesel_one_loader_errors_are_not_cached_and_can_retry() {
        let calls = Arc::new(AtomicUsize::new(0));
        let queries = DieselCache::new(HydraCache::local().build(), "diesel");

        let failed = queries
            .entity::<User>("diesel-user", 500)
            .diesel_one({
                let calls = calls.clone();
                move || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Err(DieselError::NotFound)
                }
            })
            .await;
        assert!(failed.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let recovered = queries
            .entity::<User>("diesel-user", 500)
            .diesel_one({
                let calls = calls.clone();
                move || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, DieselError>(User {
                        id: 500,
                        name: "Recovered".to_owned(),
                    })
                }
            })
            .await
            .unwrap();
        assert_eq!(recovered.name, "Recovered");
        assert_eq!(calls.load(Ordering::SeqCst), 2);

        let cached = queries
            .entity::<User>("diesel-user", 500)
            .diesel_one({
                let calls = calls.clone();
                move || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Err(DieselError::NotFound)
                }
            })
            .await
            .unwrap();
        assert_eq!(cached.name, "Recovered");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn diesel_optional_found_value_is_cached_until_invalidated() {
        let connection = sqlite_users();
        let calls = Arc::new(AtomicUsize::new(0));
        let queries = DieselCache::new(HydraCache::local().build(), "diesel");

        let first = queries
            .entity::<User>("diesel-user", 42)
            .diesel_optional(diesel_user_loader(connection.clone(), calls.clone(), 42))
            .await
            .unwrap()
            .expect("seeded user should exist");

        diesel::sql_query("update users set name = 'Updated' where id = 42")
            .execute(&mut *connection.lock().unwrap())
            .unwrap();

        let cached = queries
            .entity::<User>("diesel-user", 42)
            .diesel_optional(diesel_user_loader(connection, calls.clone(), 42))
            .await
            .unwrap()
            .expect("cached user should still exist");

        assert_eq!(first.name, "Ada");
        assert_eq!(cached.name, "Ada");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn diesel_all_caches_collection_results() {
        let connection = sqlite_users();
        let calls = Arc::new(AtomicUsize::new(0));
        let queries = DieselCache::new(HydraCache::local().build(), "diesel");

        let first = queries
            .collection::<User>("diesel-users:all")
            .diesel_all({
                let connection = connection.clone();
                let calls = calls.clone();
                move || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    let mut connection = connection.lock().expect("sqlite connection poisoned");
                    users::table
                        .order(users::id.asc())
                        .load::<User>(&mut *connection)
                }
            })
            .await
            .unwrap();

        diesel::sql_query("insert into users (id, name) values (100, 'Lin')")
            .execute(&mut *connection.lock().unwrap())
            .unwrap();

        let cached = queries
            .collection::<User>("diesel-users:all")
            .diesel_all({
                let connection = connection.clone();
                let calls = calls.clone();
                move || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    users::table
                        .order(users::id.asc())
                        .load::<User>(&mut *connection.lock().unwrap())
                }
            })
            .await
            .unwrap();

        assert_eq!(first.len(), 2);
        assert_eq!(cached.len(), 2);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn diesel_all_reloads_after_collection_tag_invalidation() {
        let connection = sqlite_users();
        let calls = Arc::new(AtomicUsize::new(0));
        let queries = DieselCache::new(HydraCache::local().build(), "diesel");

        let load_all = || {
            let connection = connection.clone();
            let calls = calls.clone();
            move || {
                calls.fetch_add(1, Ordering::SeqCst);
                let mut connection = connection.lock().expect("sqlite connection poisoned");
                users::table
                    .order(users::id.asc())
                    .load::<User>(&mut *connection)
            }
        };

        let first = queries
            .collection::<User>("diesel-users-all")
            .diesel_all(load_all())
            .await
            .unwrap();

        diesel::sql_query("insert into users (id, name) values (100, 'Lin')")
            .execute(&mut *connection.lock().unwrap())
            .unwrap();

        let cached = queries
            .collection::<User>("diesel-users-all")
            .diesel_all(load_all())
            .await
            .unwrap();

        assert_eq!(first.len(), 2);
        assert_eq!(cached.len(), 2);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            queries
                .cache()
                .invalidate_tag("diesel-users-all")
                .await
                .unwrap(),
            1
        );

        let reloaded = queries
            .collection::<User>("diesel-users-all")
            .diesel_all(load_all())
            .await
            .unwrap();

        assert_eq!(reloaded.len(), 3);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn reexports_match_database_neutral_metadata_api() {
        assert_eq!(User::entity_tag_for(&42), "diesel-user:42");
        assert_eq!(User::collection_tag(), Some("diesel-users".to_owned()));

        let policy = query_cache_policy!(
            name = "load-diesel-user",
            entity = User,
            id = 42_i64,
            ttl_secs = 60,
        );
        assert_eq!(policy.name(), Some("load-diesel-user"));
        assert_eq!(policy.key_value(), Some("diesel-user:42"));

        let prepared = PreparedQueryPolicy::for_cache_entity::<User>().with_name("prepared");
        let bound = prepared.bind_id(7);
        assert_eq!(bound.key_value(), Some("diesel-user:7"));

        let manual = QueryCachePolicy::new().for_cache_entity::<User>(42);
        assert_eq!(manual.key_value(), Some("diesel-user:42"));
    }
}
