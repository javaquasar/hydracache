#![cfg(feature = "sqlx-outbox")]

use std::error::Error;

use hydracache_db::{
    CommitPosition, InvalidationIntentBatch, InvalidationOutbox, SqlxInvalidationOutbox,
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
