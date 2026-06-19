#![cfg(feature = "sqlx-outbox")]

use std::error::Error;

use hydracache_db::{
    sqlite_hook_drift, DriftReason, DriftStatus, HookInvalidationTarget, HookPlan, OutboxLagPolicy,
    ReconciliationReport, SqlxInvalidationOutbox,
};
use sqlx::sqlite::SqlitePoolOptions;

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

async fn sqlite_pool() -> Result<sqlx::SqlitePool, sqlx::Error> {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
}

#[tokio::test]
async fn reconcile_sqlite_hook_schema_matches_installed_plan() -> TestResult {
    let pool = sqlite_pool().await?;
    sqlx::query("create table users (id integer primary key)")
        .execute(&pool)
        .await?;
    let outbox = SqlxInvalidationOutbox::sqlite(pool.clone());
    outbox.install_schema().await?;
    let hook_plan = HookPlan::sqlite("users").on_insert(HookInvalidationTarget::tag("users"));
    hook_plan.install_sqlite(&pool).await?;

    let hook_drift = sqlite_hook_drift(&pool, hook_plan.schema_version()).await?;
    let report =
        ReconciliationReport::from_outbox(&outbox, "db", hook_drift, OutboxLagPolicy::default())
            .await?;

    assert_eq!(report.status(), DriftStatus::Clean);
    Ok(())
}

#[tokio::test]
async fn reconcile_sqlite_missing_hook_schema_reports_drift() -> TestResult {
    let pool = sqlite_pool().await?;
    let outbox = SqlxInvalidationOutbox::sqlite(pool.clone());
    outbox.install_schema().await?;
    let expected = HookPlan::sqlite("users")
        .on_insert(HookInvalidationTarget::tag("users"))
        .schema_version();

    let hook_drift = sqlite_hook_drift(&pool, expected).await?;
    let report =
        ReconciliationReport::from_outbox(&outbox, "db", hook_drift, OutboxLagPolicy::default())
            .await?;

    let DriftStatus::Drift(reasons) = report.status() else {
        panic!("missing sqlite hook schema must report drift");
    };
    assert!(reasons
        .iter()
        .any(|reason| matches!(reason, DriftReason::HookSchemaMissing { .. })));
    Ok(())
}
