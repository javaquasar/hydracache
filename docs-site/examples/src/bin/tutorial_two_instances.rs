use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache::{CacheEventOrigin, HydraCache, InMemoryInvalidationBus};
use hydracache_sqlx::sqlx::sqlite::SqlitePoolOptions;
use hydracache_sqlx::sqlx::{self, SqlitePool};
use hydracache_sqlx::{DbCache, HydraCacheEntity, InvalidationPlan};
use serde::{Deserialize, Serialize};

// ANCHOR: entity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users")]
struct User {
    #[hydracache(id)]
    id: i64,
    name: String,
}
// ANCHOR_END: entity

// ANCHOR: repository
/// One service instance: a database pool plus its own in-process cache.
struct UserRepository {
    pool: SqlitePool,
    cache: HydraCache,
    queries: DbCache,
    db_reads: Arc<AtomicUsize>,
}

impl UserRepository {
    fn new(pool: SqlitePool, cache: HydraCache, db_reads: Arc<AtomicUsize>) -> Self {
        let queries = DbCache::new(cache.clone(), "users-service");
        Self {
            pool,
            cache,
            queries,
            db_reads,
        }
    }

    /// Read path: served from the cache; the loader hits SQL only on a miss.
    async fn get_user(&self, id: i64) -> Result<User, Box<dyn std::error::Error>> {
        let pool = self.pool.clone();
        let db_reads = self.db_reads.clone();
        let user = self
            .queries
            .for_entity::<User>(id)
            .fetch_with(move || async move {
                db_reads.fetch_add(1, Ordering::SeqCst);
                let (id, name): (i64, String) =
                    sqlx::query_as("SELECT id, name FROM users WHERE id = ?")
                        .bind(id)
                        .fetch_one(&pool)
                        .await?;
                Ok::<_, sqlx::Error>(User { id, name })
            })
            .await?;
        Ok(user)
    }

    /// Write path: stage invalidations, commit, then apply them.
    async fn rename_user(&self, id: i64, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let pending = InvalidationPlan::new().cache_entity::<User>(id);

        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE users SET name = ? WHERE id = ?")
            .bind(name)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        // Only reached after a successful commit. On error or rollback the
        // plan is dropped and the cache is left untouched.
        pending.execute(&self.cache).await?;
        Ok(())
    }
}
// ANCHOR_END: repository

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: database
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
    // ANCHOR_END: database

    // ANCHOR: instances
    // In production each instance is a separate process and the bus is
    // Redis or NATS. Here both share an in-memory bus.
    let bus = Arc::new(InMemoryInvalidationBus::default());
    let db_reads = Arc::new(AtomicUsize::new(0));

    let instance_a = UserRepository::new(
        pool.clone(),
        HydraCache::local()
            .shared_invalidation_bus(bus.clone())
            .invalidation_node_id("instance-a")
            .build(),
        db_reads.clone(),
    );
    let instance_b = UserRepository::new(
        pool.clone(),
        HydraCache::local()
            .shared_invalidation_bus(bus)
            .invalidation_node_id("instance-b")
            .build(),
        db_reads.clone(),
    );
    // ANCHOR_END: instances

    // ANCHOR: read
    // First read on each instance misses and loads from SQL...
    assert_eq!(instance_a.get_user(42).await?.name, "Ada");
    assert_eq!(instance_b.get_user(42).await?.name, "Ada");
    assert_eq!(db_reads.load(Ordering::SeqCst), 2);

    // ...repeated reads are served from each instance's own cache.
    for _ in 0..10 {
        instance_a.get_user(42).await?;
        instance_b.get_user(42).await?;
    }
    assert_eq!(db_reads.load(Ordering::SeqCst), 2);
    // ANCHOR_END: read

    // ANCHOR: write
    let mut b_events = instance_b.cache.subscribe_tag("user:42");

    // Instance A writes. Its post-commit plan invalidates locally and
    // publishes the same intent to instance B.
    instance_a.rename_user(42, "Ada Lovelace").await?;

    let event = tokio::time::timeout(Duration::from_millis(500), b_events.recv())
        .await?
        .expect("subscription stays open");
    assert_eq!(event.origin(), CacheEventOrigin::DistributedBus);

    // Both instances now miss once and see the new row.
    assert_eq!(instance_a.get_user(42).await?.name, "Ada Lovelace");
    assert_eq!(instance_b.get_user(42).await?.name, "Ada Lovelace");
    assert_eq!(db_reads.load(Ordering::SeqCst), 4);
    // ANCHOR_END: write

    println!(
        "tutorial ok: {} database reads",
        db_reads.load(Ordering::SeqCst)
    );
    Ok(())
}
