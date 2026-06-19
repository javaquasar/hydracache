use std::error::Error;

use hydracache::HydraCache;
use hydracache_sqlx::{
    DbCache, HydraCacheEntity, SqlxInvalidationOutbox, SqlxQueryExt, SqlxTransactionError,
    SqlxTransactionExt,
};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::sqlite::SqliteRow;
use sqlx::{FromRow, Row, SqlitePool};
use thiserror::Error;

type TestResult = std::result::Result<(), Box<dyn Error + Send + Sync>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, HydraCacheEntity)]
#[hydracache(entity = "sqlx-user", collection = "sqlx-users")]
struct User {
    #[hydracache(id)]
    id: i64,
    name: String,
}

impl<'r> FromRow<'r, SqliteRow> for User {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
        })
    }
}

#[derive(Debug, Error)]
enum BodyError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error("forced body failure")]
    Forced,
}

async fn sqlite_pool() -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    sqlx::query("pragma foreign_keys = on")
        .execute(&pool)
        .await?;
    Ok(pool)
}

async fn setup_state(
    namespace: &str,
    install_outbox: bool,
) -> Result<(SqlitePool, SqlxInvalidationOutbox, DbCache), Box<dyn Error + Send + Sync>> {
    let pool = sqlite_pool().await?;
    sqlx::query("create table users (id integer primary key, name text not null)")
        .execute(&pool)
        .await?;
    sqlx::query("insert into users (id, name) values (42, 'Ada'), (7, 'Grace')")
        .execute(&pool)
        .await?;

    let outbox = SqlxInvalidationOutbox::sqlite(pool.clone());
    if install_outbox {
        outbox.install_schema().await?;
    }

    let queries = DbCache::new(HydraCache::local().build(), namespace);
    Ok((pool, outbox, queries))
}

async fn user_name(pool: &SqlitePool, id: i64) -> Result<String, sqlx::Error> {
    sqlx::query_scalar("select name from users where id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
}

async fn outbox_count(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("select count(*) from hydracache_invalidation_outbox")
        .fetch_one(pool)
        .await
}

#[tokio::test]
async fn transaction_success_commits_and_enqueues() -> TestResult {
    let (pool, outbox, queries) = setup_state("db", true).await?;
    let companion = queries.sqlx_transactions().with_outbox(outbox);

    let report = companion
        .sqlite_durable(&pool, "update-user", |tx, invalidation| {
            Box::pin(async move {
                sqlx::query("update users set name = ? where id = ?")
                    .bind("Committed")
                    .bind(42_i64)
                    .execute(&mut **tx)
                    .await?;
                invalidation.cache_entity::<User>(42);
                Ok::<_, BodyError>(())
            })
        })
        .await?;

    assert_eq!(report.intent_count, 2);
    assert_eq!(report.durable_rows, 2);
    assert_eq!(user_name(&pool, 42).await?, "Committed");
    assert_eq!(outbox_count(&pool).await?, 2);
    assert_eq!(companion.diagnostics().commits, 1);
    assert_eq!(companion.diagnostics().rollbacks, 0);
    Ok(())
}

#[tokio::test]
async fn transaction_closure_error_rolls_back_no_outbox_row() -> TestResult {
    let (pool, outbox, queries) = setup_state("db", true).await?;
    let companion = queries.sqlx_transactions().with_outbox(outbox);

    let result = companion
        .sqlite_durable(&pool, "update-user", |tx, _invalidation| {
            Box::pin(async move {
                sqlx::query("update users set name = ? where id = ?")
                    .bind("RolledBack")
                    .bind(42_i64)
                    .execute(&mut **tx)
                    .await?;
                Err::<(), BodyError>(BodyError::Forced)
            })
        })
        .await;

    assert!(matches!(result, Err(SqlxTransactionError::Body(_))));
    assert_eq!(user_name(&pool, 42).await?, "Ada");
    assert_eq!(outbox_count(&pool).await?, 0);
    let diagnostics = companion.diagnostics();
    assert_eq!(diagnostics.body_errors, 1);
    assert_eq!(diagnostics.rollbacks, 1);
    Ok(())
}

#[tokio::test]
async fn transaction_enqueue_failure_rolls_back() -> TestResult {
    let (pool, outbox, queries) = setup_state("db", false).await?;
    let companion = queries.sqlx_transactions().with_outbox(outbox);

    let result = companion
        .sqlite_durable(&pool, "update-user", |tx, invalidation| {
            Box::pin(async move {
                sqlx::query("update users set name = ? where id = ?")
                    .bind("ShouldRollback")
                    .bind(42_i64)
                    .execute(&mut **tx)
                    .await?;
                invalidation.invalidate_tag("sqlx-user:42");
                Ok::<_, BodyError>(())
            })
        })
        .await;

    assert!(matches!(result, Err(SqlxTransactionError::Outbox(_))));
    assert_eq!(user_name(&pool, 42).await?, "Ada");
    let diagnostics = companion.diagnostics();
    assert_eq!(diagnostics.enqueue_failures, 1);
    assert_eq!(diagnostics.rollbacks, 1);
    Ok(())
}

#[tokio::test]
async fn transaction_commit_failure_does_not_publish() -> TestResult {
    let (pool, outbox, queries) = setup_state("db", true).await?;
    sqlx::query("create table parents (id integer primary key)")
        .execute(&pool)
        .await?;
    sqlx::query(
        "create table children (
            id integer primary key,
            parent_id integer not null,
            foreign key(parent_id) references parents(id) deferrable initially deferred
        )",
    )
    .execute(&pool)
    .await?;
    let companion = queries.sqlx_transactions().with_outbox(outbox);

    let result = companion
        .sqlite_durable(&pool, "insert-child", |tx, invalidation| {
            Box::pin(async move {
                sqlx::query("insert into children (id, parent_id) values (?, ?)")
                    .bind(1_i64)
                    .bind(404_i64)
                    .execute(&mut **tx)
                    .await?;
                invalidation.invalidate_tag("children");
                Ok::<_, BodyError>(())
            })
        })
        .await;

    assert!(matches!(result, Err(SqlxTransactionError::Sqlx(_))));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("select count(*) from children")
            .fetch_one(&pool)
            .await?,
        0
    );
    assert_eq!(outbox_count(&pool).await?, 0);
    assert_eq!(companion.diagnostics().commit_failures, 1);
    Ok(())
}

#[tokio::test]
async fn transaction_collector_supports_key_tag_entity_collection() -> TestResult {
    let (pool, outbox, queries) = setup_state("db", true).await?;
    let companion = queries.sqlx_transactions().with_outbox(outbox);

    let report = companion
        .sqlite_durable(&pool, "all-intents", |_tx, invalidation| {
            Box::pin(async move {
                invalidation
                    .invalidate_key("db:manual")
                    .invalidate_tag("tenant:7")
                    .invalidate_entity("sqlx-user", "42")
                    .invalidate_collection("sqlx-users");
                Ok::<_, BodyError>(())
            })
        })
        .await?;

    let rows = sqlx::query(
        "select intent_kind, namespace, reason from hydracache_invalidation_outbox order by rowid",
    )
    .fetch_all(&pool)
    .await?;
    let kinds = rows
        .iter()
        .map(|row| row.get::<String, _>("intent_kind"))
        .collect::<Vec<_>>();

    assert_eq!(report.intent_count, 4);
    assert_eq!(report.durable_rows, 4);
    assert_eq!(kinds, vec!["key", "tag", "entity", "collection"]);
    assert!(rows
        .iter()
        .all(|row| row.get::<String, _>("namespace") == "db"));
    assert!(rows
        .iter()
        .all(|row| row.get::<String, _>("reason") == "all-intents"));
    Ok(())
}

#[tokio::test]
async fn transaction_custom_namespace_preserved() -> TestResult {
    let (pool, outbox, queries) = setup_state("tenant-db", true).await?;
    let companion = queries.sqlx_transactions().with_outbox(outbox);

    companion
        .sqlite_durable(&pool, "tenant-write", |_tx, invalidation| {
            Box::pin(async move {
                invalidation.invalidate_tag("tenant:7");
                Ok::<_, BodyError>(())
            })
        })
        .await?;

    let namespace: String =
        sqlx::query_scalar("select namespace from hydracache_invalidation_outbox")
            .fetch_one(&pool)
            .await?;
    assert_eq!(namespace, "tenant-db");
    Ok(())
}

#[tokio::test]
async fn transaction_direct_invalidation_plan_without_outbox_table() -> TestResult {
    let (pool, _outbox, queries) = setup_state("db", false).await?;
    let companion = queries.sqlx_transactions();

    let first = queries
        .for_entity::<User>(42)
        .sqlx_one(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = ?").bind(42_i64),
        )
        .await?;
    assert_eq!(first.name, "Ada");

    let report = companion
        .sqlite_local(&pool, "local-update", |tx, invalidation| {
            Box::pin(async move {
                sqlx::query("update users set name = ? where id = ?")
                    .bind("Local")
                    .bind(42_i64)
                    .execute(&mut **tx)
                    .await?;
                invalidation.cache_entity::<User>(42);
                Ok::<_, BodyError>(())
            })
        })
        .await?;

    assert_eq!(report.intent_count, 2);
    assert_eq!(report.local_report.unwrap().tags_removed, 1);

    let reloaded = queries
        .for_entity::<User>(42)
        .sqlx_one(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = ?").bind(42_i64),
        )
        .await?;
    assert_eq!(reloaded.name, "Local");
    assert_eq!(companion.diagnostics().local_invalidations, 1);
    Ok(())
}

#[tokio::test]
async fn transaction_closure_panic_does_not_publish() -> TestResult {
    let (pool, outbox, queries) = setup_state("db", true).await?;
    let companion = queries.sqlx_transactions().with_outbox(outbox);

    let pool_for_task = pool.clone();
    let companion_for_task = companion.clone();
    let result = tokio::spawn(async move {
        companion_for_task
            .sqlite_durable(&pool_for_task, "panic", |_tx, _invalidation| {
                Box::pin(async move {
                    panic!("boom");
                    #[allow(unreachable_code)]
                    Ok::<(), BodyError>(())
                })
            })
            .await
    })
    .await;

    assert!(result.is_err());
    assert_eq!(outbox_count(&pool).await?, 0);
    assert_eq!(user_name(&pool, 42).await?, "Ada");
    Ok(())
}

#[tokio::test]
async fn transaction_retry_after_failure_succeeds() -> TestResult {
    let (pool, outbox, queries) = setup_state("db", true).await?;
    let companion = queries.sqlx_transactions().with_outbox(outbox);

    let failed = companion
        .sqlite_durable(&pool, "retry", |tx, _invalidation| {
            Box::pin(async move {
                sqlx::query("update users set name = ? where id = ?")
                    .bind("First")
                    .bind(42_i64)
                    .execute(&mut **tx)
                    .await?;
                Err::<(), BodyError>(BodyError::Forced)
            })
        })
        .await;
    assert!(failed.is_err());
    assert_eq!(user_name(&pool, 42).await?, "Ada");

    companion
        .sqlite_durable(&pool, "retry", |tx, invalidation| {
            Box::pin(async move {
                sqlx::query("update users set name = ? where id = ?")
                    .bind("Second")
                    .bind(42_i64)
                    .execute(&mut **tx)
                    .await?;
                invalidation.invalidate_tag("sqlx-user:42");
                Ok::<_, BodyError>(())
            })
        })
        .await?;

    assert_eq!(user_name(&pool, 42).await?, "Second");
    assert_eq!(outbox_count(&pool).await?, 1);
    let diagnostics = companion.diagnostics();
    assert_eq!(diagnostics.body_errors, 1);
    assert_eq!(diagnostics.commits, 1);
    Ok(())
}
