#![cfg(feature = "sqlx-outbox")]

use std::error::Error;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use hydracache::{CacheOptions, HydraCache};
use hydracache_db::{
    ConsistencyMode, InvalidationIntentBatch, InvalidationOutbox, InvalidationOutboxWorker,
    InvalidationReceipt, InvalidationWait, SqlxInvalidationOutbox,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use tokio::task::JoinSet;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct UserRow {
    id: i64,
    name: String,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires HYDRACACHE_TEST_POSTGRES_URL (provided by migration-conformance-postgres-069)"]
async fn postgres_cached_reads_match_direct_queries_through_the_real_outbox() -> TestResult {
    run_postgres_cached_reads_match_direct_queries().await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires PostgreSQL 18 via migration-conformance-postgres-069"]
async fn postgres_18_cached_reads_match_direct_queries_through_the_real_outbox() -> TestResult {
    run_postgres_cached_reads_match_direct_queries().await
}

async fn run_postgres_cached_reads_match_direct_queries() -> TestResult {
    let url = std::env::var("HYDRACACHE_TEST_POSTGRES_URL")
        .expect("HYDRACACHE_TEST_POSTGRES_URL is mandatory when this gate is selected");
    let pool = PgPool::connect(&url).await?;
    assert_postgres_series(&pool).await?;
    let outbox = SqlxInvalidationOutbox::postgres(pool.clone());
    outbox.install_schema().await?;
    outbox.check_schema().await?;

    for mode in [
        ConsistencyMode::NoWait,
        ConsistencyMode::Local,
        ConsistencyMode::BestEffort,
    ] {
        run_mode(&pool, &outbox, mode, false).await?;
    }
    for seed in [0x69_2026_u64, 0x69_2027, 0x69_2028] {
        run_concurrent_seed(&pool, &outbox, seed).await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires HYDRACACHE_TEST_POSTGRES_URL (provided by migration-conformance-postgres-069)"]
async fn canary_postgres_differential_rejects_a_dropped_invalidation() -> TestResult {
    let url = std::env::var("HYDRACACHE_TEST_POSTGRES_URL")
        .expect("HYDRACACHE_TEST_POSTGRES_URL is mandatory when this gate is selected");
    let pool = PgPool::connect(&url).await?;
    assert_postgres_series(&pool).await?;
    let outbox = SqlxInvalidationOutbox::postgres(pool.clone());
    outbox.install_schema().await?;
    let inject = std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W4_PG_DROP");
    run_mode(&pool, &outbox, ConsistencyMode::Local, inject).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires HYDRACACHE_TEST_POSTGRES_URL (provided by migration-conformance-postgres-069)"]
async fn postgres_commit_scoped_wait_soak_stays_within_budget() -> TestResult {
    const SEEDS: u64 = 24;
    const BUDGET: Duration = Duration::from_secs(120);
    let url = std::env::var("HYDRACACHE_TEST_POSTGRES_URL")
        .expect("HYDRACACHE_TEST_POSTGRES_URL is mandatory when this gate is selected");
    let pool = PgPool::connect(&url).await?;
    assert_postgres_series(&pool).await?;
    let outbox = SqlxInvalidationOutbox::postgres(pool.clone());
    outbox.install_schema().await?;
    outbox.check_schema().await?;

    let started = Instant::now();
    tokio::time::timeout(BUDGET, async {
        for index in 0..SEEDS {
            run_concurrent_seed(&pool, &outbox, 0x69_5000_u64 + index).await?;
        }
        Ok::<(), Box<dyn Error + Send + Sync>>(())
    })
    .await
    .map_err(|_| "HC69_POSTGRES_SOAK_BUDGET_EXCEEDED")??;
    println!(
        "HC69_POSTGRES_SOAK_OK\tseeds={SEEDS}\twriters_per_seed=12\tduration_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

async fn assert_postgres_series(pool: &PgPool) -> TestResult {
    let version: String = sqlx::query_scalar("select version()")
        .fetch_one(pool)
        .await?;
    let server_version_num: String = sqlx::query_scalar("show server_version_num")
        .fetch_one(pool)
        .await?;
    let actual_series = server_version_num
        .get(..2)
        .ok_or("PostgreSQL server_version_num is malformed")?;
    let expected_series = std::env::var("HYDRACACHE_POSTGRES_SERIES")
        .expect("HYDRACACHE_POSTGRES_SERIES is mandatory for release evidence");
    assert_eq!(
        actual_series, expected_series,
        "PostgreSQL service does not match the declared evidence series"
    );
    println!("HC69_POSTGRES_VERSION\tseries={actual_series}\t{version}");
    Ok(())
}

async fn run_mode(
    pool: &PgPool,
    outbox: &SqlxInvalidationOutbox,
    mode: ConsistencyMode,
    inject_drop: bool,
) -> TestResult {
    let suffix = unique_suffix();
    let table = format!("hc69_users_{suffix}");
    let namespace = format!("db-pg-differential-{suffix}");
    let cache_key = format!("db:{table}:all");
    sqlx::query(&format!(
        "CREATE TABLE {table} (id BIGINT PRIMARY KEY, name TEXT NOT NULL)"
    ))
    .execute(pool)
    .await?;

    let cache = HydraCache::local().build();
    cache
        .put(&cache_key, Vec::<UserRow>::new(), CacheOptions::new())
        .await?;
    let worker = InvalidationOutboxWorker::new(outbox.clone(), cache.clone(), namespace.clone());

    for (id, name) in [(1_i64, "Ada"), (2, "Grace"), (3, "Linus")] {
        let mut tx = pool.begin().await?;
        sqlx::query(&format!("INSERT INTO {table} (id, name) VALUES ($1, $2)"))
            .bind(id)
            .bind(name)
            .execute(&mut *tx)
            .await?;
        let position = outbox.postgres_commit_position(&mut tx).await?;
        let drop_this_invalidation = inject_drop && id == 2;
        if !drop_this_invalidation {
            assert_eq!(
                outbox
                    .enqueue_in_postgres_tx(
                        &mut tx,
                        &namespace,
                        &position,
                        &InvalidationIntentBatch::new("users-write").invalidate_key(&cache_key),
                    )
                    .await?,
                1
            );
        }
        tx.commit().await?;

        let receipt = InvalidationReceipt::new(namespace.clone(), position);
        let wait = invalidation_wait(mode);
        let outcome = if mode == ConsistencyMode::NoWait {
            wait.wait(outbox, &receipt).await?
        } else {
            let waiter = tokio::spawn({
                let wait = wait.clone();
                let outbox = outbox.clone();
                async move { wait.wait(&outbox, &receipt).await }
            });
            tokio::task::yield_now().await;
            assert_eq!(
                worker.run_once().await?.published,
                usize::from(!drop_this_invalidation)
            );
            waiter.await??
        };
        assert!(outcome.satisfied && !outcome.degraded && !outcome.timed_out);

        if mode == ConsistencyMode::NoWait {
            assert_eq!(
                worker.run_once().await?.published,
                usize::from(!drop_this_invalidation)
            );
        }
        let direct = direct_rows(pool, &table).await?;
        let cached = cached_rows(&cache, pool, &table, &cache_key).await?;
        assert_eq!(
            cached, direct,
            "HC-CANARY-RED:W4-PG PostgreSQL cached/direct divergence after committed row {id} in {mode:?}"
        );
    }

    assert_eq!(outbox.status(&namespace).await?.pending, 0);
    sqlx::query("DELETE FROM hydracache_invalidation_outbox WHERE namespace = $1")
        .bind(&namespace)
        .execute(pool)
        .await?;
    sqlx::query(&format!("DROP TABLE {table}"))
        .execute(pool)
        .await?;
    Ok(())
}

async fn run_concurrent_seed(
    pool: &PgPool,
    outbox: &SqlxInvalidationOutbox,
    mut seed: u64,
) -> TestResult {
    let suffix = unique_suffix();
    let table = format!("hc69_concurrent_{suffix}");
    let namespace = format!("db-pg-concurrent-{suffix}");
    let cache_key = format!("db:{table}:all");
    sqlx::query(&format!(
        "CREATE TABLE {table} (id BIGINT PRIMARY KEY, name TEXT NOT NULL)"
    ))
    .execute(pool)
    .await?;
    let cache = HydraCache::local().build();
    cache
        .put(&cache_key, Vec::<UserRow>::new(), CacheOptions::new())
        .await?;

    let mut ids = (1_i64..=12).collect::<Vec<_>>();
    for index in (1..ids.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        ids.swap(index, seed as usize % (index + 1));
    }
    let mut writers = JoinSet::new();
    for id in ids {
        let pool = pool.clone();
        let outbox = outbox.clone();
        let table = table.clone();
        let namespace = namespace.clone();
        let cache_key = cache_key.clone();
        writers.spawn(async move {
            let mut tx = pool.begin().await?;
            sqlx::query(&format!("INSERT INTO {table} (id, name) VALUES ($1, $2)"))
                .bind(id)
                .bind(format!("user-{id}"))
                .execute(&mut *tx)
                .await?;
            let position = outbox.postgres_commit_position(&mut tx).await?;
            outbox
                .enqueue_in_postgres_tx(
                    &mut tx,
                    &namespace,
                    &position,
                    &InvalidationIntentBatch::new("concurrent-users-write")
                        .invalidate_key(cache_key),
                )
                .await?;
            tx.commit().await?;
            Ok::<(), Box<dyn Error + Send + Sync>>(())
        });
    }
    while let Some(result) = writers.join_next().await {
        result??;
    }

    assert_eq!(outbox.status(&namespace).await?.pending, 12);
    let worker = InvalidationOutboxWorker::new(outbox.clone(), cache.clone(), &namespace);
    assert_eq!(worker.run_once().await?.published, 12);
    assert_eq!(
        cached_rows(&cache, pool, &table, &cache_key).await?,
        direct_rows(pool, &table).await?,
        "PostgreSQL multi-writer divergence for seed {seed:#x}"
    );
    sqlx::query("DELETE FROM hydracache_invalidation_outbox WHERE namespace = $1")
        .bind(&namespace)
        .execute(pool)
        .await?;
    sqlx::query(&format!("DROP TABLE {table}"))
        .execute(pool)
        .await?;
    Ok(())
}

async fn direct_rows(pool: &PgPool, table: &str) -> Result<Vec<UserRow>, sqlx::Error> {
    Ok(
        sqlx::query(&format!("SELECT id, name FROM {table} ORDER BY id"))
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|row| UserRow {
                id: row.get("id"),
                name: row.get("name"),
            })
            .collect(),
    )
}

async fn cached_rows(
    cache: &HydraCache,
    pool: &PgPool,
    table: &str,
    cache_key: &str,
) -> Result<Vec<UserRow>, Box<dyn Error + Send + Sync>> {
    if let Some(rows) = cache.get::<Vec<UserRow>>(cache_key).await? {
        return Ok(rows);
    }
    let rows = direct_rows(pool, table).await?;
    cache
        .put(cache_key, rows.clone(), CacheOptions::new())
        .await?;
    Ok(rows)
}

fn invalidation_wait(mode: ConsistencyMode) -> InvalidationWait {
    match mode {
        ConsistencyMode::NoWait => InvalidationWait::no_wait(),
        ConsistencyMode::Local => InvalidationWait::local(Duration::from_secs(5)),
        ConsistencyMode::BestEffort => InvalidationWait::best_effort(Duration::from_secs(5)),
        _ => panic!("unknown consistency mode in PostgreSQL differential test"),
    }
    .poll_interval(Duration::from_millis(2))
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
