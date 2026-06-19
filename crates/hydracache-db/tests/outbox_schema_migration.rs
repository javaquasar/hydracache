#![cfg(feature = "sqlx-outbox")]

use std::error::Error;

use hydracache_db::{SqlxInvalidationOutbox, OUTBOX_SCHEMA_VERSION};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Row, SqlitePool};

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

const OUTBOX_ARTIFACT: &str = "hydracache_invalidation_outbox";

async fn sqlite_pool() -> Result<SqlitePool, sqlx::Error> {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
}

#[tokio::test]
async fn schema_migration_installs_version_row_and_is_idempotent() -> TestResult {
    let pool = sqlite_pool().await?;
    let outbox = SqlxInvalidationOutbox::sqlite(pool.clone());

    outbox.install_schema().await?;
    outbox.install_schema().await?;
    outbox.check_schema().await?;

    let version: i64 =
        sqlx::query("select version from hydracache_schema_version where artifact = ?")
            .bind(OUTBOX_ARTIFACT)
            .fetch_one(&pool)
            .await?
            .get("version");

    assert_eq!(version, OUTBOX_SCHEMA_VERSION);
    Ok(())
}

#[tokio::test]
async fn schema_check_fails_closed_when_version_row_is_missing() -> TestResult {
    let pool = sqlite_pool().await?;
    let outbox = SqlxInvalidationOutbox::sqlite(pool.clone());
    outbox.install_schema().await?;

    sqlx::query("delete from hydracache_schema_version where artifact = ?")
        .bind(OUTBOX_ARTIFACT)
        .execute(&pool)
        .await?;

    let error = outbox.check_schema().await.unwrap_err();

    assert!(error.to_string().contains("missing"));
    Ok(())
}

#[tokio::test]
async fn schema_check_fails_closed_on_future_version() -> TestResult {
    let pool = sqlite_pool().await?;
    let outbox = SqlxInvalidationOutbox::sqlite(pool.clone());
    outbox.install_schema().await?;

    sqlx::query("update hydracache_schema_version set version = ? where artifact = ?")
        .bind(OUTBOX_SCHEMA_VERSION + 1)
        .bind(OUTBOX_ARTIFACT)
        .execute(&pool)
        .await?;

    let error = outbox.check_schema().await.unwrap_err();

    assert!(error.to_string().contains("unknown future"));
    Ok(())
}
