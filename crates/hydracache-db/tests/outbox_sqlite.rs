#![cfg(feature = "sqlx-outbox")]

use std::{error::Error, time::Duration};

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

#[tokio::test]
async fn claim_zero_limit_is_noop() -> TestResult {
    let (_pool, outbox) = setup().await?;
    outbox
        .enqueue(
            "db",
            &CommitPosition::new("sqlite:1"),
            &InvalidationIntentBatch::new("user-write").invalidate_key("user:42"),
        )
        .await?;

    let none = outbox
        .claim("db", "worker", 0, Duration::from_secs(30))
        .await?;
    let claimed = outbox
        .claim("db", "worker", 1, Duration::from_secs(30))
        .await?;
    let claimed_again = outbox
        .claim("db", "other-worker", 1, Duration::from_secs(30))
        .await?;

    assert!(none.is_empty());
    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].intent.value(), Some("user:42"));
    assert!(claimed_again.is_empty());
    Ok(())
}

#[tokio::test]
async fn malformed_durable_row_fails_loud() -> TestResult {
    let (pool, outbox) = setup().await?;
    sqlx::query(
        "insert into hydracache_invalidation_outbox (
            id, namespace, commit_position, target_hash, intent_kind,
            reason, created_at_ms, available_at_ms, state
        ) values (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind("bad-key-row")
    .bind("db")
    .bind("sqlite:bad")
    .bind("bad-target")
    .bind("key")
    .bind("manual-corruption")
    .bind(0_i64)
    .bind(0_i64)
    .bind("pending")
    .execute(&pool)
    .await?;

    let error = outbox
        .claim("db", "worker", 1, Duration::from_secs(30))
        .await
        .unwrap_err();

    assert!(
        error.to_string().contains("missing cache_key"),
        "malformed row error should name the bad field: {error}"
    );
    Ok(())
}

#[tokio::test]
async fn retry_backoff_blocks_then_dead_letter_reset_reclaims() -> TestResult {
    let (pool, outbox) = setup().await?;
    outbox
        .enqueue(
            "db",
            &CommitPosition::new("sqlite:1"),
            &InvalidationIntentBatch::new("user-write").invalidate_tag("users"),
        )
        .await?;
    let claimed = outbox
        .claim("db", "worker-a", 1, Duration::from_secs(30))
        .await?;

    outbox
        .mark_failed(
            &claimed[0].id,
            "temporary publish error",
            Duration::from_secs(60),
            false,
        )
        .await?;
    let blocked = outbox
        .claim("db", "worker-b", 1, Duration::from_secs(30))
        .await?;
    assert!(blocked.is_empty(), "retry backoff should delay reclaim");
    let status = outbox.status("db").await?;
    assert_eq!(status.pending, 1);
    assert_eq!(status.failed_attempts, 1);

    sqlx::query(
        "update hydracache_invalidation_outbox \
         set available_at_ms = 0 \
         where id = ?",
    )
    .bind(&claimed[0].id)
    .execute(&pool)
    .await?;
    let retry = outbox.claim("db", "worker-c", 1, Duration::ZERO).await?;
    assert_eq!(retry.len(), 1);
    assert_eq!(retry[0].attempts, 1);
    assert_eq!(
        retry[0].last_error.as_deref(),
        Some("temporary publish error")
    );

    outbox
        .mark_failed(&retry[0].id, "permanent error", Duration::ZERO, true)
        .await?;
    assert_eq!(outbox.reset_dead_letters("db").await?, 1);
    let reset = outbox.claim("db", "worker-d", 1, Duration::ZERO).await?;
    assert_eq!(reset.len(), 1);
    assert_eq!(reset[0].attempts, 0);
    assert_eq!(reset[0].last_error, None);
    Ok(())
}

#[tokio::test]
async fn status_reports_oldest_pending_lag() -> TestResult {
    let (pool, outbox) = setup().await?;
    outbox
        .enqueue(
            "db",
            &CommitPosition::new("sqlite:1"),
            &InvalidationIntentBatch::new("user-write").invalidate_tag("users"),
        )
        .await?;
    sqlx::query(
        "update hydracache_invalidation_outbox \
         set created_at_ms = 0, available_at_ms = 0 \
         where namespace = ?",
    )
    .bind("db")
    .execute(&pool)
    .await?;

    let status = outbox.status("db").await?;

    assert_eq!(status.pending, 1);
    assert!(
        status.oldest_pending_age_ms > 1_000,
        "oldest pending age should expose durable outbox lag: {status:?}"
    );
    Ok(())
}
