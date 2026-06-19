#![cfg(feature = "sqlx-outbox")]

use std::error::Error;

use hydracache::{CacheOptions, HydraCache};
use hydracache_db::{
    HookInvalidationTarget, HookPlan, InvalidationOutbox, InvalidationOutboxWorker,
    SqlxInvalidationOutbox,
};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Row, SqlitePool};

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

async fn sqlite_pool() -> Result<SqlitePool, sqlx::Error> {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
}

async fn setup() -> TestResult<(SqlitePool, SqlxInvalidationOutbox)> {
    let pool = sqlite_pool().await?;
    sqlx::query(
        "create table users (
            id integer primary key,
            tenant_id integer not null,
            name text not null
        )",
    )
    .execute(&pool)
    .await?;
    let outbox = SqlxInvalidationOutbox::sqlite(pool.clone());
    outbox.install_schema().await?;
    Ok((pool, outbox))
}

#[tokio::test]
async fn trigger_writes_outbox_on_insert() -> TestResult {
    let (pool, _outbox) = setup().await?;
    HookPlan::sqlite("users")
        .on_insert(HookInvalidationTarget::entity("user", "id"))
        .install_sqlite(&pool)
        .await?;

    sqlx::query("insert into users (id, tenant_id, name) values (1, 7, 'Ada')")
        .execute(&pool)
        .await?;
    let row = sqlx::query(
        "select intent_kind, entity_name, cache_key from hydracache_invalidation_outbox",
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(row.get::<String, _>("intent_kind"), "entity");
    assert_eq!(row.get::<String, _>("entity_name"), "user");
    assert_eq!(row.get::<String, _>("cache_key"), "1");
    Ok(())
}

#[tokio::test]
async fn trigger_writes_outbox_on_update() -> TestResult {
    let (pool, _outbox) = setup().await?;
    HookPlan::sqlite("users")
        .on_update(HookInvalidationTarget::tag("users"))
        .install_sqlite(&pool)
        .await?;

    sqlx::query("insert into users (id, tenant_id, name) values (1, 7, 'Ada')")
        .execute(&pool)
        .await?;
    sqlx::query("update users set name = 'Grace' where id = 1")
        .execute(&pool)
        .await?;
    let row = sqlx::query("select intent_kind, cache_tag from hydracache_invalidation_outbox")
        .fetch_one(&pool)
        .await?;

    assert_eq!(row.get::<String, _>("intent_kind"), "tag");
    assert_eq!(row.get::<String, _>("cache_tag"), "users");
    Ok(())
}

#[tokio::test]
async fn trigger_writes_outbox_on_delete() -> TestResult {
    let (pool, _outbox) = setup().await?;
    HookPlan::sqlite("users")
        .on_delete(HookInvalidationTarget::collection("users"))
        .install_sqlite(&pool)
        .await?;

    sqlx::query("insert into users (id, tenant_id, name) values (1, 7, 'Ada')")
        .execute(&pool)
        .await?;
    sqlx::query("delete from users where id = 1")
        .execute(&pool)
        .await?;
    let row =
        sqlx::query("select intent_kind, collection_name from hydracache_invalidation_outbox")
            .fetch_one(&pool)
            .await?;

    assert_eq!(row.get::<String, _>("intent_kind"), "collection");
    assert_eq!(row.get::<String, _>("collection_name"), "users");
    Ok(())
}

#[tokio::test]
async fn worker_publishes_trigger_row_and_invalidates() -> TestResult {
    let (pool, outbox) = setup().await?;
    let cache = HydraCache::local().build();
    cache
        .put("profile:1", "stale", CacheOptions::new().tag("user:1"))
        .await?;
    HookPlan::sqlite("users")
        .on_insert(HookInvalidationTarget::entity("user", "id"))
        .install_sqlite(&pool)
        .await?;

    sqlx::query("insert into users (id, tenant_id, name) values (1, 7, 'Ada')")
        .execute(&pool)
        .await?;
    let worker = InvalidationOutboxWorker::new(outbox.clone(), cache.clone(), "db");
    let report = worker.run_once().await?;

    assert_eq!(report.published, 1);
    assert_eq!(cache.get::<String>("profile:1").await?, None);
    Ok(())
}

#[tokio::test]
async fn duplicate_trigger_rows_are_idempotent() -> TestResult {
    let (_pool, outbox) = setup().await?;
    sqlx::query(
        "insert or ignore into hydracache_invalidation_outbox (
            id, namespace, commit_position, target_hash, intent_kind,
            cache_key, cache_tag, entity_name, collection_name, reason,
            created_at_ms, available_at_ms
        ) values
        ('same', 'db', 'commit:1', 'tag:users', 'tag', null, 'users', null, null, 'test', 1, 1),
        ('same-again', 'db', 'commit:1', 'tag:users', 'tag', null, 'users', null, null, 'test', 1, 1)",
    )
    .execute(&_pool)
    .await?;

    let rows = outbox
        .claim("db", "worker", 10, std::time::Duration::ZERO)
        .await?;

    assert_eq!(rows.len(), 1);
    Ok(())
}

#[tokio::test]
async fn namespace_isolation_with_generated_triggers() -> TestResult {
    let (pool, outbox) = setup().await?;
    HookPlan::sqlite("users")
        .namespace("tenant-a")
        .on_insert(HookInvalidationTarget::entity("user", "id"))
        .install_sqlite(&pool)
        .await?;

    sqlx::query("insert into users (id, tenant_id, name) values (1, 7, 'Ada')")
        .execute(&pool)
        .await?;

    assert_eq!(
        outbox
            .claim("db", "worker", 10, std::time::Duration::ZERO)
            .await?
            .len(),
        0
    );
    assert_eq!(
        outbox
            .claim("tenant-a", "worker", 10, std::time::Duration::ZERO)
            .await?
            .len(),
        1
    );
    Ok(())
}
