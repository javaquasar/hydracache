use hydracache::{CacheOptions, HydraCache};
use hydracache_db::{
    CommitPosition, InvalidationIntentBatch, InvalidationOutbox, InvalidationOutboxWorker,
};
use hydracache_sqlx::sqlx::sqlite::SqlitePoolOptions;
use hydracache_sqlx::sqlx::{self, SqlitePool};
use hydracache_sqlx::SqlxInvalidationOutbox;

async fn setup() -> Result<(SqlitePool, SqlxInvalidationOutbox), Box<dyn std::error::Error>> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)")
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO users (id, name) VALUES (42, 'Ada')")
        .execute(&pool)
        .await?;
    Ok((pool.clone(), SqlxInvalidationOutbox::sqlite(pool)))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (pool, outbox) = setup().await?;
    let cache = HydraCache::local().build();
    cache
        .put(
            "user:42",
            "Ada".to_owned(),
            CacheOptions::new().tag("user:42"),
        )
        .await?;

    // ANCHOR: install
    // Usually done by your migration tool; the helper is idempotent and
    // refuses to run against an unknown future schema version.
    outbox.install_schema().await?;
    outbox.check_schema().await?;
    // ANCHOR_END: install

    // ANCHOR: enqueue
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE users SET name = ? WHERE id = ?")
        .bind("Ada Lovelace")
        .bind(42_i64)
        .execute(&mut *tx)
        .await?;

    // Same transaction: the intent is durable if and only if the write commits.
    outbox
        .enqueue_in_sqlite_tx(
            &mut tx,
            "app",
            &CommitPosition::new("users:42:v2"),
            &InvalidationIntentBatch::new("rename user").invalidate_tag("user:42"),
        )
        .await?;
    tx.commit().await?;
    // ANCHOR_END: enqueue

    // ANCHOR: worker
    // A background worker drains committed rows and applies them to the cache.
    // In a service, call `run_once` in a loop or on a timer.
    let worker = InvalidationOutboxWorker::new(outbox.clone(), cache.clone(), "app");
    let report = worker.run_once().await?;

    assert_eq!(report.published, 1);
    assert!(cache.get::<String>("user:42").await?.is_none());
    assert_eq!(outbox.status("app").await?.pending, 0);
    // ANCHOR_END: worker

    // ANCHOR: rollback
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE users SET name = ? WHERE id = ?")
        .bind("Grace")
        .bind(42_i64)
        .execute(&mut *tx)
        .await?;
    outbox
        .enqueue_in_sqlite_tx(
            &mut tx,
            "app",
            &CommitPosition::new("users:42:v3"),
            &InvalidationIntentBatch::new("rename user").invalidate_tag("user:42"),
        )
        .await?;
    tx.rollback().await?;

    // Nothing was committed, so there is nothing to publish.
    assert_eq!(worker.run_once().await?.claimed, 0);
    // ANCHOR_END: rollback

    println!("outbox ok");
    Ok(())
}
