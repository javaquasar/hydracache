//! SeaORM-facing integration crate for HydraCache database result caching.
//!
//! The database-neutral query cache API lives in `hydracache-db`. This crate
//! keeps SeaORM users on a convenient import path while avoiding a SeaORM
//! dependency in the core cache adapter.
//!
//! # Example
//!
//! ```rust
//! use hydracache::HydraCache;
//! use hydracache_seaorm::{SeaOrmCache, SeaOrmQueryExt};
//!
//! # async fn example() -> hydracache_seaorm::Result<()> {
//! let queries = SeaOrmCache::new(HydraCache::local().build(), "seaorm");
//!
//! let user_name = queries
//!     .entity::<String>("user", 42)
//!     .collection_tag("users")
//!     .sea_one(|| async { Ok::<_, hydracache_seaorm::sea_orm::DbErr>(Some("Ada".to_owned())) })
//!     .await?;
//!
//! assert_eq!(user_name, Some("Ada".to_owned()));
//! # Ok(())
//! # }
//! ```
//!
//! Use [`DbQuery::fetch_with`] when you need custom repository code,
//! transactions, or a database client that does not fit the convenience helper
//! shapes.
//!
//! [`DbQuery::fetch_with`]: hydracache_db::DbQuery::fetch_with

extern crate self as hydracache_seaorm;

use std::future::Future;

use async_trait::async_trait;
use hydracache_core::CacheCodec;
use hydracache_db::DbQuery;
use sea_orm::DbErr;
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

pub use hydracache_db::{
    query_cache_policy, CacheEntity, DbCache, DbCacheError, DbQuery as GenericDbQuery,
    HydraCacheEntity, PreparedDbQuery, PreparedQueryPolicy, QueryCachePolicy, Result as DbResult,
};

/// SeaORM-specific compatibility name for [`DbCache`].
pub type SeaOrmCache<C = hydracache::PostcardCodec> = DbCache<C>;

/// SeaORM-specific compatibility name for [`DbQuery`].
pub type SeaOrmQuery<T, C = hydracache::PostcardCodec> = DbQuery<T, C>;

/// Re-export the SeaORM crate used by this adapter.
pub use sea_orm;

/// Error type returned by SeaORM-facing cache helpers.
#[derive(Debug, Error)]
pub enum SeaOrmCacheError {
    /// The generic database cache adapter or underlying cache failed.
    #[error(transparent)]
    Cache(#[from] DbCacheError),
}

/// SeaORM adapter result type.
pub type Result<T> = std::result::Result<T, SeaOrmCacheError>;

/// Convenience SeaORM execution methods for [`DbQuery`].
///
/// These helpers keep SeaORM responsible for query construction and row
/// mapping, while HydraCache owns keying, tags, TTL, serialization, and local
/// single-flight.
#[async_trait]
pub trait SeaOrmQueryExt<T, C>
where
    C: CacheCodec,
{
    /// Execute an async SeaORM loader on miss and cache either one row or
    /// `None`.
    async fn sea_one<F, Fut>(self, loader: F) -> Result<Option<T>>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<Option<T>, DbErr>> + Send + 'static;

    /// Execute an async SeaORM loader on miss and cache exactly one value.
    async fn sea_value<F, Fut>(self, loader: F) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<T, DbErr>> + Send + 'static;

    /// Execute an async SeaORM loader on miss and cache all returned rows.
    async fn sea_all<F, Fut>(self, loader: F) -> Result<Vec<T>>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<Vec<T>, DbErr>> + Send + 'static;
}

#[async_trait]
impl<T, C> SeaOrmQueryExt<T, C> for DbQuery<T, C>
where
    C: CacheCodec,
{
    async fn sea_one<F, Fut>(self, loader: F) -> Result<Option<T>>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<Option<T>, DbErr>> + Send + 'static,
    {
        self.fetch_value_with(loader).await.map_err(Into::into)
    }

    async fn sea_value<F, Fut>(self, loader: F) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<T, DbErr>> + Send + 'static,
    {
        self.fetch_value_with(loader).await.map_err(Into::into)
    }

    async fn sea_all<F, Fut>(self, loader: F) -> Result<Vec<T>>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = std::result::Result<Vec<T>, DbErr>> + Send + 'static,
    {
        self.fetch_value_with(loader).await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use hydracache::HydraCache;
    use sea_orm::entity::prelude::*;
    use sea_orm::{ConnectionTrait, Database, DatabaseBackend, EntityTrait, QueryOrder, Set};
    use serde::{Deserialize, Serialize};

    use super::{
        query_cache_policy, CacheEntity, HydraCacheEntity, PreparedQueryPolicy, QueryCachePolicy,
        SeaOrmCache, SeaOrmQueryExt,
    };

    mod user {
        use super::*;

        #[derive(
            Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize, HydraCacheEntity,
        )]
        #[sea_orm(table_name = "users")]
        #[hydracache(entity = "seaorm-user", collection = "seaorm-users", id = i64)]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i64,
            pub name: String,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }

    async fn sqlite_users() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.execute(sea_orm::Statement::from_string(
            DatabaseBackend::Sqlite,
            "create table users (id integer primary key, name text not null)".to_owned(),
        ))
        .await
        .unwrap();

        user::Entity::insert(user::ActiveModel {
            id: Set(42),
            name: Set("Ada".to_owned()),
        })
        .exec(&db)
        .await
        .unwrap();
        user::Entity::insert(user::ActiveModel {
            id: Set(7),
            name: Set("Grace".to_owned()),
        })
        .exec(&db)
        .await
        .unwrap();

        db
    }

    #[tokio::test]
    async fn sea_one_caches_real_sqlite_query_until_invalidation() {
        let db = sqlite_users().await;
        let calls = Arc::new(AtomicUsize::new(0));
        let queries = SeaOrmCache::new(HydraCache::local().build(), "seaorm");

        let first = queries
            .entity::<user::Model>("seaorm-user", 42)
            .collection_tag("seaorm-users")
            .sea_one({
                let db = db.clone();
                let calls = calls.clone();
                move || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    user::Entity::find_by_id(42).one(&db).await
                }
            })
            .await
            .unwrap()
            .expect("user should exist");

        assert_eq!(first.name, "Ada");
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        db.execute(sea_orm::Statement::from_string(
            DatabaseBackend::Sqlite,
            "update users set name = 'Updated' where id = 42".to_owned(),
        ))
        .await
        .unwrap();

        let cached = queries
            .entity::<user::Model>("seaorm-user", 42)
            .collection_tag("seaorm-users")
            .sea_one({
                let db = db.clone();
                let calls = calls.clone();
                move || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    user::Entity::find_by_id(42).one(&db).await
                }
            })
            .await
            .unwrap()
            .expect("cached user should exist");

        assert_eq!(cached.name, "Ada");
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        assert_eq!(
            queries
                .cache()
                .invalidate_tag("seaorm-users")
                .await
                .unwrap(),
            1
        );

        let reloaded = queries
            .entity::<user::Model>("seaorm-user", 42)
            .collection_tag("seaorm-users")
            .sea_one({
                let db = db.clone();
                let calls = calls.clone();
                move || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    user::Entity::find_by_id(42).one(&db).await
                }
            })
            .await
            .unwrap()
            .expect("reloaded user should exist");

        assert_eq!(reloaded.name, "Updated");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn sea_one_caches_none_without_reloading() {
        let db = sqlite_users().await;
        let calls = Arc::new(AtomicUsize::new(0));
        let queries = SeaOrmCache::new(HydraCache::local().build(), "seaorm");

        let missing = queries
            .entity::<user::Model>("seaorm-user", 404)
            .sea_one({
                let db = db.clone();
                let calls = calls.clone();
                move || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    user::Entity::find_by_id(404).one(&db).await
                }
            })
            .await
            .unwrap();
        let cached_missing = queries
            .entity::<user::Model>("seaorm-user", 404)
            .sea_one({
                let db = db.clone();
                let calls = calls.clone();
                move || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    user::Entity::find_by_id(404).one(&db).await
                }
            })
            .await
            .unwrap();

        assert_eq!(missing, None);
        assert_eq!(cached_missing, None);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn missing_key_fails_before_running_seaorm_loader() {
        let calls = Arc::new(AtomicUsize::new(0));
        let queries = SeaOrmCache::new(HydraCache::local().build(), "seaorm");

        let result = queries
            .cached::<user::Model>()
            .sea_value({
                let calls = calls.clone();
                move || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, DbErr>(user::Model {
                        id: 42,
                        name: "Ada".to_owned(),
                    })
                }
            })
            .await;

        let error = result.expect_err("query without a key should fail");
        assert!(error.to_string().contains("missing an explicit cache key"));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn sea_value_loader_errors_are_not_cached_and_can_retry() {
        let calls = Arc::new(AtomicUsize::new(0));
        let queries = SeaOrmCache::new(HydraCache::local().build(), "seaorm");

        let failed = queries
            .entity::<user::Model>("seaorm-user", 500)
            .sea_value({
                let calls = calls.clone();
                move || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Err::<user::Model, _>(DbErr::Custom("boom".to_owned()))
                }
            })
            .await;
        assert!(failed.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let recovered = queries
            .entity::<user::Model>("seaorm-user", 500)
            .sea_value({
                let calls = calls.clone();
                move || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, DbErr>(user::Model {
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
            .entity::<user::Model>("seaorm-user", 500)
            .sea_value({
                let calls = calls.clone();
                move || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Err::<user::Model, _>(DbErr::Custom("should not run".to_owned()))
                }
            })
            .await
            .unwrap();
        assert_eq!(cached.name, "Recovered");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn sea_one_found_value_is_cached_until_invalidated() {
        let db = sqlite_users().await;
        let calls = Arc::new(AtomicUsize::new(0));
        let queries = SeaOrmCache::new(HydraCache::local().build(), "seaorm");

        let first = queries
            .entity::<user::Model>("seaorm-user", 42)
            .sea_one({
                let db = db.clone();
                let calls = calls.clone();
                move || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    user::Entity::find_by_id(42).one(&db).await
                }
            })
            .await
            .unwrap()
            .expect("seeded user should exist");

        user::Entity::update(user::ActiveModel {
            id: Set(42),
            name: Set("Updated".to_owned()),
        })
        .exec(&db)
        .await
        .unwrap();

        let cached = queries
            .entity::<user::Model>("seaorm-user", 42)
            .sea_one({
                let db = db.clone();
                let calls = calls.clone();
                move || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    user::Entity::find_by_id(42).one(&db).await
                }
            })
            .await
            .unwrap()
            .expect("cached user should still exist");

        assert_eq!(first.name, "Ada");
        assert_eq!(cached.name, "Ada");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn sea_all_and_sea_value_cache_collection_shapes() {
        let db = sqlite_users().await;
        let calls = Arc::new(AtomicUsize::new(0));
        let queries = SeaOrmCache::new(HydraCache::local().build(), "seaorm");

        let first = queries
            .collection::<user::Model>("seaorm-users:all")
            .sea_all({
                let db = db.clone();
                let calls = calls.clone();
                move || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    user::Entity::find()
                        .order_by_asc(user::Column::Id)
                        .all(&db)
                        .await
                }
            })
            .await
            .unwrap();

        user::Entity::insert(user::ActiveModel {
            id: Set(100),
            name: Set("Lin".to_owned()),
        })
        .exec(&db)
        .await
        .unwrap();

        let cached = queries
            .collection::<user::Model>("seaorm-users:all")
            .sea_all({
                let db = db.clone();
                let calls = calls.clone();
                move || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    user::Entity::find()
                        .order_by_asc(user::Column::Id)
                        .all(&db)
                        .await
                }
            })
            .await
            .unwrap();

        let scalar = queries
            .cached::<String>()
            .key("seaorm:scalar")
            .sea_value(|| async { Ok::<_, DbErr>("cached-value".to_owned()) })
            .await
            .unwrap();

        assert_eq!(first.len(), 2);
        assert_eq!(cached.len(), 2);
        assert_eq!(scalar, "cached-value");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn sea_all_reloads_after_collection_tag_invalidation() {
        let db = sqlite_users().await;
        let calls = Arc::new(AtomicUsize::new(0));
        let queries = SeaOrmCache::new(HydraCache::local().build(), "seaorm");

        let load_all = || {
            let db = db.clone();
            let calls = calls.clone();
            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                user::Entity::find()
                    .order_by_asc(user::Column::Id)
                    .all(&db)
                    .await
            }
        };

        let first = queries
            .collection::<user::Model>("seaorm-users-all")
            .sea_all(load_all())
            .await
            .unwrap();

        user::Entity::insert(user::ActiveModel {
            id: Set(100),
            name: Set("Lin".to_owned()),
        })
        .exec(&db)
        .await
        .unwrap();

        let cached = queries
            .collection::<user::Model>("seaorm-users-all")
            .sea_all(load_all())
            .await
            .unwrap();

        assert_eq!(first.len(), 2);
        assert_eq!(cached.len(), 2);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            queries
                .cache()
                .invalidate_tag("seaorm-users-all")
                .await
                .unwrap(),
            1
        );

        let reloaded = queries
            .collection::<user::Model>("seaorm-users-all")
            .sea_all(load_all())
            .await
            .unwrap();

        assert_eq!(reloaded.len(), 3);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn reexports_match_database_neutral_metadata_api() {
        assert_eq!(user::Model::entity_tag_for(&42), "seaorm-user:42");
        assert_eq!(
            user::Model::collection_tag(),
            Some("seaorm-users".to_owned())
        );

        let policy = query_cache_policy!(
            name = "load-seaorm-user",
            entity = user::Model,
            id = 42_i64,
            ttl_secs = 60,
        );
        assert_eq!(policy.name(), Some("load-seaorm-user"));
        assert_eq!(policy.key_value(), Some("seaorm-user:42"));

        let prepared = PreparedQueryPolicy::for_cache_entity::<user::Model>().with_name("prepared");
        let bound = prepared.bind_id(7);
        assert_eq!(bound.key_value(), Some("seaorm-user:7"));

        let manual = QueryCachePolicy::new().for_cache_entity::<user::Model>(42);
        assert_eq!(manual.key_value(), Some("seaorm-user:42"));
    }
}
