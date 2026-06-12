use std::time::Duration;

use hydracache::HydraCache;
use hydracache_sqlx::{HydraCacheEntity, PreparedQueryPolicy, SqlxCache, SqlxQueryExt};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, HydraCacheEntity)]
#[hydracache(entity = "sqlite-user", collection = "sqlite-users", id = i64)]
struct User {
    id: i64,
    name: String,
}

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

async fn sqlite_users() -> Result<SqlitePool, Box<dyn std::error::Error + Send + Sync>> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;

    sqlx::query("create table users (id integer primary key, name text not null)")
        .execute(&pool)
        .await?;
    sqlx::query("insert into users (id, name) values (?, ?)")
        .bind(42_i64)
        .bind("Ada")
        .execute(&pool)
        .await?;
    sqlx::query("insert into users (id, name) values (?, ?)")
        .bind(7_i64)
        .bind("Linus")
        .execute(&pool)
        .await?;

    Ok(pool)
}

#[tokio::test]
async fn prepared_sqlite_queries_cache_real_in_memory_database_results(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pool = sqlite_users().await?;

    let queries = SqlxCache::new(HydraCache::local().build(), "sqlite");
    let prepared_user = queries.prepare::<(i64, String)>(
        PreparedQueryPolicy::for_cache_entity::<User>().with_name("sqlite-load-user"),
    );

    let first = prepared_user
        .for_id(42)
        .sqlx_one(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = ?").bind(42_i64),
        )
        .await?;
    assert_eq!(first, (42, "Ada".to_owned()));

    sqlx::query("update users set name = ? where id = ?")
        .bind("Grace")
        .bind(42_i64)
        .execute(&pool)
        .await?;

    let cached = prepared_user
        .for_id(42)
        .sqlx_one(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = ?").bind(42_i64),
        )
        .await?;
    assert_eq!(cached, (42, "Ada".to_owned()));

    assert_eq!(queries.cache().invalidate_tag("sqlite-users").await?, 1);

    let reloaded = prepared_user
        .for_id(42)
        .sqlx_one(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = ?").bind(42_i64),
        )
        .await?;
    assert_eq!(reloaded, (42, "Grace".to_owned()));

    let prepared_list = queries.prepare::<(i64, String)>(
        PreparedQueryPolicy::named("sqlite-list-users").collection("sqlite-users:all"),
    );

    let listed = prepared_list
        .to_query()
        .sqlx_all(
            pool.clone(),
            sqlx::query_as("select id, name from users order by id"),
        )
        .await?;
    assert_eq!(
        listed,
        vec![(7, "Linus".to_owned()), (42, "Grace".to_owned())]
    );

    sqlx::query("insert into users (id, name) values (?, ?)")
        .bind(99_i64)
        .bind("New")
        .execute(&pool)
        .await?;

    let listed_cached = prepared_list
        .to_query()
        .sqlx_all(
            pool,
            sqlx::query_as("select id, name from users order by id"),
        )
        .await?;
    assert_eq!(
        listed_cached,
        vec![(7, "Linus".to_owned()), (42, "Grace".to_owned())]
    );

    Ok(())
}

#[tokio::test]
async fn sqlite_fetch_optional_caches_some_and_none_results() -> TestResult {
    let pool = sqlite_users().await?;
    let queries = SqlxCache::new(HydraCache::local().build(), "sqlite");
    let prepared_user = queries.prepare::<(i64, String)>(
        PreparedQueryPolicy::for_cache_entity::<User>().with_name("sqlite-optional-user"),
    );

    let first = prepared_user
        .for_id(7)
        .sqlx_optional(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = ?").bind(7_i64),
        )
        .await?;
    assert_eq!(first, Some((7, "Linus".to_owned())));

    sqlx::query("update users set name = ? where id = ?")
        .bind("Torvalds")
        .bind(7_i64)
        .execute(&pool)
        .await?;

    let cached_some = prepared_user
        .for_id(7)
        .sqlx_optional(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = ?").bind(7_i64),
        )
        .await?;
    assert_eq!(cached_some, Some((7, "Linus".to_owned())));

    let missing = prepared_user
        .for_id(999)
        .sqlx_optional(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = ?").bind(999_i64),
        )
        .await?;
    assert_eq!(missing, None);

    sqlx::query("insert into users (id, name) values (?, ?)")
        .bind(999_i64)
        .bind("Later")
        .execute(&pool)
        .await?;

    let cached_none = prepared_user
        .for_id(999)
        .sqlx_optional(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = ?").bind(999_i64),
        )
        .await?;
    assert_eq!(cached_none, None);
    assert_eq!(queries.cache().invalidate_tag("sqlite-user:999").await?, 1);

    let reloaded = prepared_user
        .for_id(999)
        .sqlx_optional(
            pool,
            sqlx::query_as("select id, name from users where id = ?").bind(999_i64),
        )
        .await?;
    assert_eq!(reloaded, Some((999, "Later".to_owned())));

    Ok(())
}

#[tokio::test]
async fn sqlite_fetch_all_reloads_after_collection_invalidation() -> TestResult {
    let pool = sqlite_users().await?;
    let queries = SqlxCache::new(HydraCache::local().build(), "sqlite");
    let prepared_list = queries.prepare::<(i64, String)>(
        PreparedQueryPolicy::named("sqlite-list-users").collection("sqlite-users-all"),
    );

    let first = prepared_list
        .to_query()
        .sqlx_all(
            pool.clone(),
            sqlx::query_as("select id, name from users order by id"),
        )
        .await?;

    sqlx::query("insert into users (id, name) values (?, ?)")
        .bind(99_i64)
        .bind("New")
        .execute(&pool)
        .await?;

    let cached = prepared_list
        .to_query()
        .sqlx_all(
            pool.clone(),
            sqlx::query_as("select id, name from users order by id"),
        )
        .await?;

    assert_eq!(first.len(), 2);
    assert_eq!(cached.len(), 2);
    assert_eq!(queries.cache().invalidate_tag("sqlite-users-all").await?, 1);

    let reloaded = prepared_list
        .to_query()
        .sqlx_all(
            pool,
            sqlx::query_as("select id, name from users order by id"),
        )
        .await?;

    assert_eq!(reloaded.len(), 3);
    assert_eq!(reloaded[2], (99, "New".to_owned()));

    Ok(())
}

#[tokio::test]
async fn sqlite_sqlx_one_reloads_after_ttl_expiration() -> TestResult {
    let pool = sqlite_users().await?;
    let queries = SqlxCache::new(HydraCache::local().build(), "sqlite");

    let first = queries
        .entity::<(i64, String)>("sqlite-user", 42)
        .ttl(Duration::from_millis(20))
        .sqlx_one(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = ?").bind(42_i64),
        )
        .await?;

    sqlx::query("update users set name = ? where id = ?")
        .bind("AfterTtl")
        .bind(42_i64)
        .execute(&pool)
        .await?;

    tokio::time::sleep(Duration::from_millis(40)).await;

    let reloaded = queries
        .entity::<(i64, String)>("sqlite-user", 42)
        .ttl(Duration::from_millis(20))
        .sqlx_one(
            pool,
            sqlx::query_as("select id, name from users where id = ?").bind(42_i64),
        )
        .await?;

    assert_eq!(first, (42, "Ada".to_owned()));
    assert_eq!(reloaded, (42, "AfterTtl".to_owned()));

    Ok(())
}

#[tokio::test]
async fn sqlite_sqlx_all_caches_empty_collections() -> TestResult {
    let pool = sqlite_users().await?;
    let queries = SqlxCache::new(HydraCache::local().build(), "sqlite");

    let first = queries
        .collection::<(i64, String)>("sqlite-users-empty")
        .sqlx_all(
            pool.clone(),
            sqlx::query_as("select id, name from users where id < ? order by id").bind(0_i64),
        )
        .await?;

    sqlx::query("insert into users (id, name) values (?, ?)")
        .bind(-1_i64)
        .bind("Negative")
        .execute(&pool)
        .await?;

    let cached = queries
        .collection::<(i64, String)>("sqlite-users-empty")
        .sqlx_all(
            pool,
            sqlx::query_as("select id, name from users where id < ? order by id").bind(0_i64),
        )
        .await?;

    assert!(first.is_empty());
    assert!(cached.is_empty());

    Ok(())
}
