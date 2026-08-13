#![cfg(feature = "sqlx-outbox")]

use hydracache::{CacheOptions, HydraCache};
use hydracache_db::{
    CommitPosition, ConsistencyMode, InvalidationIntentBatch, InvalidationOutbox,
    InvalidationOutboxWorker, InvalidationReceipt, InvalidationWait, SqlxInvalidationOutbox,
};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;

const CACHE_KEY: &str = "db:users:all";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct UserRow {
    id: i64,
    name: String,
}

#[derive(Clone, Copy, Debug)]
enum Write {
    Insert(i64, &'static str),
    Update(i64, &'static str),
    Delete(i64),
}

struct Committed {
    position: u64,
    receipt: InvalidationReceipt,
    acknowledge: oneshot::Sender<()>,
}

struct WriteRequest {
    write: Write,
    drop_invalidation: bool,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cached_reads_match_direct_queries_per_consistency_mode_under_concurrent_writes() {
    for mode in [
        ConsistencyMode::NoWait,
        ConsistencyMode::Local,
        ConsistencyMode::BestEffort,
    ] {
        run_seeded_schedule(mode, false).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn post_quiescence_cache_and_source_are_exactly_equal() {
    run_seeded_schedule(ConsistencyMode::NoWait, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stale_read_beyond_the_documented_bound_is_red_not_tolerated() {
    run_seeded_schedule(ConsistencyMode::BestEffort, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn canary_db_differential_accepts_a_dropped_invalidation() {
    let inject = std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W4_DROP");
    run_seeded_schedule(ConsistencyMode::Local, inject).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn multi_writer_seed_matrix_converges_through_the_outbox() {
    for seed in [0x69_2026_u64, 0x69_2027, 0x69_2028] {
        run_concurrent_seed(seed).await;
    }
}

async fn run_concurrent_seed(mut seed: u64) {
    let pool = sqlite_pool().await;
    let outbox = SqlxInvalidationOutbox::sqlite(pool.clone());
    outbox.install_schema().await.unwrap();
    let namespace = format!("db-concurrent-{seed}-{}", unique_suffix());
    let cache = HydraCache::local().build();
    cache
        .put(CACHE_KEY, Vec::<UserRow>::new(), CacheOptions::new())
        .await
        .unwrap();

    let mut ids = (1_i64..=12).collect::<Vec<_>>();
    for index in (1..ids.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        ids.swap(index, seed as usize % (index + 1));
    }
    let mut writers = JoinSet::new();
    for (position, id) in ids.into_iter().enumerate() {
        let pool = pool.clone();
        let outbox = outbox.clone();
        let namespace = namespace.clone();
        writers.spawn(async move {
            let mut transaction = pool.begin().await.unwrap();
            sqlx::query("INSERT INTO users (id, name) VALUES (?, ?)")
                .bind(id)
                .bind(format!("user-{id}"))
                .execute(&mut *transaction)
                .await
                .unwrap();
            outbox
                .enqueue_in_sqlite_tx(
                    &mut transaction,
                    &namespace,
                    &CommitPosition::new(format!("sqlite:concurrent:{position}")),
                    &InvalidationIntentBatch::new("concurrent-users-write")
                        .invalidate_key(CACHE_KEY),
                )
                .await
                .unwrap();
            transaction.commit().await.unwrap();
        });
    }
    while let Some(result) = writers.join_next().await {
        result.unwrap();
    }

    assert_eq!(outbox.status(&namespace).await.unwrap().pending, 12);
    let worker = InvalidationOutboxWorker::new(outbox.clone(), cache.clone(), &namespace);
    let report = worker.run_once().await.unwrap();
    assert_eq!(report.published, 12);
    assert_eq!(
        cached_rows(&cache, &pool).await,
        direct_rows(&pool).await,
        "multi-writer cached/direct divergence for seed {seed:#x}"
    );
}

async fn run_seeded_schedule(mode: ConsistencyMode, inject_drop: bool) {
    let pool = sqlite_pool().await;
    let outbox = SqlxInvalidationOutbox::sqlite(pool.clone());
    outbox.install_schema().await.unwrap();
    outbox.check_schema().await.unwrap();
    let namespace = format!("db-differential-{}", unique_suffix());
    let cache = HydraCache::local().build();
    let worker = InvalidationOutboxWorker::new(outbox.clone(), cache.clone(), namespace.clone());
    let initial = direct_rows(&pool).await;
    cache
        .put(CACHE_KEY, initial, CacheOptions::new())
        .await
        .unwrap();

    let schedule = seeded_schedule(0x69_2026);
    let (write_tx, mut write_rx) = mpsc::channel::<WriteRequest>(1);
    let (commit_tx, mut commit_rx) = mpsc::channel::<Committed>(1);
    let writer_pool = pool.clone();
    let writer_outbox = outbox.clone();
    let writer_namespace = namespace.clone();
    let writer = tokio::spawn(async move {
        let mut position = 0_u64;
        while let Some(request) = write_rx.recv().await {
            position += 1;
            let commit_position = CommitPosition::new(format!("sqlite:{position}"));
            apply_write(
                &writer_pool,
                &writer_outbox,
                &writer_namespace,
                &commit_position,
                request.write,
                request.drop_invalidation,
            )
            .await;
            let (acknowledge, acknowledged) = oneshot::channel();
            commit_tx
                .send(Committed {
                    position,
                    receipt: InvalidationReceipt::new(writer_namespace.clone(), commit_position),
                    acknowledge,
                })
                .await
                .unwrap();
            acknowledged.await.unwrap();
        }
    });

    for (index, write) in schedule.into_iter().enumerate() {
        let expected_position = index as u64 + 1;
        let drop_this_invalidation = inject_drop && expected_position == 2;
        write_tx
            .send(WriteRequest {
                write,
                drop_invalidation: drop_this_invalidation,
            })
            .await
            .unwrap();
        let committed = commit_rx.recv().await.unwrap();
        assert_eq!(committed.position, expected_position);

        let direct_at_commit = direct_rows(&pool).await;
        let wait = invalidation_wait(mode);
        let outcome = if mode == ConsistencyMode::NoWait {
            let outcome = wait.wait(&outbox, &committed.receipt).await.unwrap();
            assert!(outcome.satisfied && !outcome.degraded && !outcome.timed_out);
            outcome
        } else {
            let waiter = tokio::spawn({
                let wait = wait.clone();
                let outbox = outbox.clone();
                let receipt = committed.receipt.clone();
                async move { wait.wait(&outbox, &receipt).await.unwrap() }
            });
            tokio::task::yield_now().await;
            let report = worker.run_once().await.unwrap();
            if !drop_this_invalidation {
                assert_eq!(
                    report.published, 1,
                    "one committed intent must be published"
                );
            }
            waiter.await.unwrap()
        };

        let cached_at_commit = cached_rows(&cache, &pool).await;
        if mode != ConsistencyMode::NoWait {
            assert!(
                outcome.satisfied && !outcome.degraded && !outcome.timed_out,
                "invalidation wait must observe a drained durable outbox: {outcome:?}"
            );
            assert_eq!(
                cached_at_commit, direct_at_commit,
                "HC-CANARY-RED:W4 cached result diverged at logical commit {} in {mode:?}",
                committed.position
            );
        }

        if mode == ConsistencyMode::NoWait {
            let report = worker.run_once().await.unwrap();
            if !drop_this_invalidation {
                assert_eq!(
                    report.published, 1,
                    "one committed intent must be published"
                );
            }
        }
        let converged = cached_rows(&cache, &pool).await;
        assert_eq!(
            converged, direct_at_commit,
            "HC-CANARY-RED:W4 post-drain result diverged at logical commit {}",
            committed.position
        );
        committed.acknowledge.send(()).unwrap();
    }

    drop(write_tx);
    writer.await.unwrap();
    assert_eq!(outbox.status(&namespace).await.unwrap().pending, 0);
    assert_eq!(cached_rows(&cache, &pool).await, direct_rows(&pool).await);
}

async fn sqlite_pool() -> SqlitePool {
    let url = format!(
        "sqlite:file:hc69_{}_{}?mode=memory&cache=shared",
        std::process::id(),
        unique_suffix()
    );
    let options = url
        .parse::<SqliteConnectOptions>()
        .unwrap()
        .busy_timeout(std::time::Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)")
        .execute(&pool)
        .await
        .unwrap();
    pool
}

async fn apply_write(
    pool: &SqlitePool,
    outbox: &SqlxInvalidationOutbox,
    namespace: &str,
    commit_position: &CommitPosition,
    write: Write,
    drop_invalidation: bool,
) {
    let mut transaction = pool.begin().await.unwrap();
    match write {
        Write::Insert(id, name) => {
            sqlx::query("INSERT INTO users (id, name) VALUES (?, ?)")
                .bind(id)
                .bind(name)
                .execute(&mut *transaction)
                .await
                .unwrap();
        }
        Write::Update(id, name) => {
            sqlx::query("UPDATE users SET name = ? WHERE id = ?")
                .bind(name)
                .bind(id)
                .execute(&mut *transaction)
                .await
                .unwrap();
        }
        Write::Delete(id) => {
            sqlx::query("DELETE FROM users WHERE id = ?")
                .bind(id)
                .execute(&mut *transaction)
                .await
                .unwrap();
        }
    }
    if !drop_invalidation {
        let inserted = outbox
            .enqueue_in_sqlite_tx(
                &mut transaction,
                namespace,
                commit_position,
                &InvalidationIntentBatch::new("users-write").invalidate_key(CACHE_KEY),
            )
            .await
            .unwrap();
        assert_eq!(inserted, 1);
    }
    transaction.commit().await.unwrap();
}

fn invalidation_wait(mode: ConsistencyMode) -> InvalidationWait {
    match mode {
        ConsistencyMode::NoWait => InvalidationWait::no_wait(),
        ConsistencyMode::Local => InvalidationWait::local(std::time::Duration::from_secs(2)),
        ConsistencyMode::BestEffort => {
            InvalidationWait::best_effort(std::time::Duration::from_secs(2))
        }
        _ => panic!("unknown consistency mode in differential test"),
    }
    .poll_interval(std::time::Duration::from_millis(1))
}

async fn direct_rows(pool: &SqlitePool) -> Vec<UserRow> {
    sqlx::query("SELECT id, name FROM users ORDER BY id")
        .fetch_all(pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| UserRow {
            id: row.get("id"),
            name: row.get("name"),
        })
        .collect()
}

async fn cached_rows(cache: &HydraCache, pool: &SqlitePool) -> Vec<UserRow> {
    if let Some(rows) = cache.get::<Vec<UserRow>>(CACHE_KEY).await.unwrap() {
        return rows;
    }
    let rows = direct_rows(pool).await;
    cache
        .put(CACHE_KEY, rows.clone(), CacheOptions::new())
        .await
        .unwrap();
    rows
}

fn seeded_schedule(mut seed: u64) -> Vec<Write> {
    let mut inserts = vec![
        Write::Insert(1, "Ada"),
        Write::Insert(2, "Grace"),
        Write::Insert(3, "Linus"),
    ];
    for index in (1..inserts.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        inserts.swap(index, seed as usize % (index + 1));
    }
    inserts.extend([Write::Update(2, "Hopper"), Write::Delete(1)]);
    inserts
}

fn unique_suffix() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[test]
fn schedule_is_seed_deterministic() {
    let first = seeded_schedule(0x69_2026)
        .into_iter()
        .map(|write| format!("{write:?}"))
        .collect::<Vec<_>>();
    let second = seeded_schedule(0x69_2026)
        .into_iter()
        .map(|write| format!("{write:?}"))
        .collect::<Vec<_>>();
    assert_eq!(first, second);
}
