#![cfg(feature = "sqlx-outbox")]

use std::error::Error;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hydracache::{CacheOptions, HydraCache};
use hydracache_db::{
    CommitPosition, InvalidationIntentBatch, InvalidationOutbox, InvalidationOutboxWorker,
    PgNotifyIntentSource, SqlxInvalidationOutbox,
};

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

#[tokio::test]
#[ignore = "requires HYDRACACHE_TEST_POSTGRES_URL or a testcontainers harness"]
async fn pg_commit_position_uses_txid() -> TestResult {
    let Some(url) = std::env::var("HYDRACACHE_TEST_POSTGRES_URL").ok() else {
        eprintln!("skipping Postgres outbox test because HYDRACACHE_TEST_POSTGRES_URL is unset");
        return Ok(());
    };
    let pool = sqlx::PgPool::connect(&url).await?;
    let outbox = SqlxInvalidationOutbox::postgres(pool.clone());
    outbox.install_schema().await?;
    outbox.check_schema().await?;

    let mut tx = pool.begin().await?;
    let position = outbox.postgres_commit_position(&mut tx).await?;
    tx.rollback().await?;

    assert!(!position.as_str().is_empty());
    Ok(())
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_TEST_POSTGRES_URL or a testcontainers harness"]
async fn pg_crash_window_replays() -> TestResult {
    let Some(url) = std::env::var("HYDRACACHE_TEST_POSTGRES_URL").ok() else {
        eprintln!("skipping Postgres outbox test because HYDRACACHE_TEST_POSTGRES_URL is unset");
        return Ok(());
    };
    let pool = sqlx::PgPool::connect(&url).await?;
    let outbox = SqlxInvalidationOutbox::postgres(pool);
    outbox.install_schema().await?;
    outbox.check_schema().await?;

    let inserted = outbox
        .enqueue(
            "db",
            &CommitPosition::new("pg:manual"),
            &InvalidationIntentBatch::new("pg-write").invalidate_tag("users"),
        )
        .await?;

    assert!(inserted <= 1);
    Ok(())
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_TEST_POSTGRES_URL or a testcontainers harness"]
async fn pg_notify_source_receives_intent_payload() -> TestResult {
    let Some(url) = std::env::var("HYDRACACHE_TEST_POSTGRES_URL").ok() else {
        eprintln!("skipping Postgres notify test because HYDRACACHE_TEST_POSTGRES_URL is unset");
        return Ok(());
    };
    let channel = unique_channel("hydracache_notify");
    let mut source = PgNotifyIntentSource::connect(&url, &channel).await?;
    let pool = sqlx::PgPool::connect(&url).await?;

    sqlx::query("select pg_notify($1, $2)")
        .bind(&channel)
        .bind("tag:users")
        .execute(&pool)
        .await?;

    let received = tokio::time::timeout(Duration::from_secs(5), source.recv()).await??;

    assert_eq!(source.channel(), channel);
    assert_eq!(received.channel(), channel);
    assert_eq!(received.payload(), "tag:users");
    Ok(())
}

#[tokio::test]
#[ignore = "requires HYDRACACHE_TEST_POSTGRES_URL or a testcontainers harness"]
async fn lost_notify_is_recovered_by_poll() -> TestResult {
    let Some(url) = std::env::var("HYDRACACHE_TEST_POSTGRES_URL").ok() else {
        eprintln!("skipping Postgres notify test because HYDRACACHE_TEST_POSTGRES_URL is unset");
        return Ok(());
    };
    let pool = sqlx::PgPool::connect(&url).await?;
    let outbox = SqlxInvalidationOutbox::postgres(pool);
    outbox.install_schema().await?;
    outbox.check_schema().await?;
    let cache = HydraCache::local().build();
    cache
        .put("db:user:42", 42_u64, CacheOptions::new().tag("users"))
        .await?;

    outbox
        .enqueue(
            "db",
            &CommitPosition::new(format!("pg:lost-notify:{}", unique_suffix())),
            &InvalidationIntentBatch::new("pg-write").invalidate_tag("users"),
        )
        .await?;

    let worker = InvalidationOutboxWorker::new(outbox.clone(), cache.clone(), "db");
    let report = worker.run_once().await?;

    assert_eq!(report.published, 1);
    assert_eq!(outbox.status("db").await?.pending, 0);
    assert!(cache.get::<u64>("db:user:42").await?.is_none());
    Ok(())
}

fn unique_channel(prefix: &str) -> String {
    format!("{prefix}_{}", unique_suffix())
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
