#![cfg(feature = "sqlx-outbox")]

use std::error::Error;

use hydracache::{CacheOptions, HydraCache};
use hydracache_db::{
    CommitPosition, InvalidationIntentBatch, InvalidationOutbox, InvalidationOutboxWorker,
    OutboxState, SqlxInvalidationOutbox,
};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Row, SqlitePool};

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

async fn sqlite_pool() -> Result<SqlitePool, sqlx::Error> {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
}

async fn setup() -> Result<(SqlitePool, SqlxInvalidationOutbox), Box<dyn Error + Send + Sync>> {
    let pool = sqlite_pool().await?;
    sqlx::query("create table users (id integer primary key, name text not null)")
        .execute(&pool)
        .await?;
    let outbox = SqlxInvalidationOutbox::sqlite(pool.clone());
    outbox.install_schema().await?;
    outbox.check_schema().await?;
    Ok((pool, outbox))
}

#[tokio::test]
async fn applies_clean_then_idempotent_reapply() -> TestResult {
    let (_pool, outbox) = setup().await?;

    outbox.install_schema().await?;
    outbox.check_schema().await?;

    Ok(())
}

#[tokio::test]
async fn refuses_unknown_future_schema_version() -> TestResult {
    let (pool, outbox) = setup().await?;

    sqlx::query(
        "update hydracache_schema_version set version = 999 \
         where artifact = 'hydracache_invalidation_outbox'",
    )
    .execute(&pool)
    .await?;

    let error = outbox.check_schema().await.unwrap_err();

    assert!(error.to_string().contains("unknown future"));
    Ok(())
}

#[tokio::test]
async fn commit_persists_outbox_row_with_data() -> TestResult {
    let (pool, outbox) = setup().await?;
    let mut tx = pool.begin().await?;

    sqlx::query("insert into users (id, name) values (?, ?)")
        .bind(42_i64)
        .bind("Ada")
        .execute(&mut *tx)
        .await?;
    let inserted = outbox
        .enqueue_in_sqlite_tx(
            &mut tx,
            "db",
            &CommitPosition::new("sqlite:1"),
            &InvalidationIntentBatch::new("user-write").invalidate_tag("user:42"),
        )
        .await?;
    tx.commit().await?;

    let user_count: i64 = sqlx::query("select count(*) as count from users")
        .fetch_one(&pool)
        .await?
        .get("count");
    let outbox_count: i64 =
        sqlx::query("select count(*) as count from hydracache_invalidation_outbox")
            .fetch_one(&pool)
            .await?
            .get("count");

    assert_eq!(inserted, 1);
    assert_eq!(user_count, 1);
    assert_eq!(outbox_count, 1);
    Ok(())
}

#[tokio::test]
async fn rollback_removes_outbox_row() -> TestResult {
    let (pool, outbox) = setup().await?;
    let mut tx = pool.begin().await?;

    sqlx::query("insert into users (id, name) values (?, ?)")
        .bind(42_i64)
        .bind("RolledBack")
        .execute(&mut *tx)
        .await?;
    outbox
        .enqueue_in_sqlite_tx(
            &mut tx,
            "db",
            &CommitPosition::new("sqlite:1"),
            &InvalidationIntentBatch::new("user-write").invalidate_tag("user:42"),
        )
        .await?;
    tx.rollback().await?;

    let user_count: i64 = sqlx::query("select count(*) as count from users")
        .fetch_one(&pool)
        .await?
        .get("count");
    let outbox_count: i64 =
        sqlx::query("select count(*) as count from hydracache_invalidation_outbox")
            .fetch_one(&pool)
            .await?
            .get("count");

    assert_eq!(user_count, 0);
    assert_eq!(outbox_count, 0);
    Ok(())
}

#[tokio::test]
async fn crash_window_replays_durable_row() -> TestResult {
    let (_pool, outbox) = setup().await?;
    outbox
        .enqueue(
            "db",
            &CommitPosition::new("sqlite:1"),
            &InvalidationIntentBatch::new("user-write").invalidate_tag("user:42"),
        )
        .await?;

    let cache = HydraCache::local().build();
    cache
        .put("db:user:42", 42_u64, CacheOptions::new().tag("user:42"))
        .await?;
    let worker = InvalidationOutboxWorker::new(outbox.clone(), cache.clone(), "db");

    let report = worker.run_once().await?;

    assert_eq!(report.published, 1);
    assert!(cache.get::<u64>("db:user:42").await?.is_none());
    assert_eq!(outbox.status("db").await?.pending, 0);
    Ok(())
}

#[tokio::test]
async fn double_drain_is_idempotent() -> TestResult {
    let (_pool, outbox) = setup().await?;
    let batch = InvalidationIntentBatch::new("user-write").invalidate_tag("users");

    assert_eq!(
        outbox
            .enqueue("db", &CommitPosition::new("sqlite:1"), &batch)
            .await?,
        1
    );
    assert_eq!(
        outbox
            .enqueue("db", &CommitPosition::new("sqlite:1"), &batch)
            .await?,
        0
    );

    let worker = InvalidationOutboxWorker::new(outbox.clone(), HydraCache::local().build(), "db");
    let first = worker.run_once().await?;
    let second = worker.run_once().await?;
    let rows = outbox
        .claim("db", "observer", 10, std::time::Duration::ZERO)
        .await?;

    assert_eq!(first.published, 1);
    assert_eq!(second.claimed, 0);
    assert!(rows.is_empty());
    assert!(outbox.status("db").await?.last_published_at_ms.is_some());
    Ok(())
}

#[tokio::test]
async fn status_reports_pending_oldest_and_dead() -> TestResult {
    let (_pool, outbox) = setup().await?;
    outbox
        .enqueue(
            "db",
            &CommitPosition::new("sqlite:1"),
            &InvalidationIntentBatch::new("user-write").invalidate_tag("users"),
        )
        .await?;
    let claimed = outbox
        .claim("db", "worker", 1, std::time::Duration::from_secs(30))
        .await?;
    outbox
        .mark_failed(
            &claimed[0].id,
            "publish failed",
            std::time::Duration::from_secs(1),
            true,
        )
        .await?;

    let status = outbox.status("db").await?;
    let rows = outbox
        .claim("db", "worker", 1, std::time::Duration::from_secs(30))
        .await?;

    assert_eq!(status.dead_lettered, 1);
    assert_eq!(status.failed_attempts, 1);
    assert!(rows.is_empty());
    assert_eq!(claimed[0].state, OutboxState::Pending);
    Ok(())
}
