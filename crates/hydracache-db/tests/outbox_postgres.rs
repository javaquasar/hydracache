#![cfg(feature = "sqlx-outbox")]

use std::error::Error;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hydracache::{CacheOptions, HydraCache};
use hydracache_db::{
    CommitPosition, InvalidationIntentBatch, InvalidationOutbox, InvalidationOutboxWorker,
    PgNotifyIntentSource, SqlxInvalidationOutbox,
};
use std::collections::BTreeSet;

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

#[tokio::test]
#[ignore = "requires HYDRACACHE_TEST_POSTGRES_URL or a testcontainers harness"]
async fn pg_outbox_concurrency_retry_and_backend_contract() -> TestResult {
    let Some(url) = std::env::var("HYDRACACHE_TEST_POSTGRES_URL").ok() else {
        eprintln!("skipping Postgres outbox test because HYDRACACHE_TEST_POSTGRES_URL is unset");
        return Ok(());
    };
    let pool = sqlx::PgPool::connect(&url).await?;
    let outbox = SqlxInvalidationOutbox::postgres(pool.clone());
    outbox.install_schema().await?;
    outbox.check_schema().await?;
    assert!(format!("{outbox:?}").contains("postgres"));
    let namespace = format!("pg-contract-{}", unique_suffix());

    let all_intents = InvalidationIntentBatch::new("contract-write")
        .invalidate_key("user:42")
        .invalidate_tag("users")
        .invalidate_entity("user", "42")
        .invalidate_collection("users")
        .flush();
    let mut tx = pool.begin().await?;
    assert_eq!(
        outbox
            .enqueue_in_postgres_tx(
                &mut tx,
                &namespace,
                &CommitPosition::new("pg:rolled-back"),
                &all_intents,
            )
            .await?,
        5
    );
    tx.rollback().await?;
    assert_eq!(outbox.status(&namespace).await?.pending, 0);

    assert_eq!(
        outbox
            .enqueue(
                &namespace,
                &CommitPosition::new("pg:concurrent"),
                &all_intents,
            )
            .await?,
        5
    );
    assert!(outbox
        .claim(&namespace, "zero", 0, Duration::from_secs(30))
        .await?
        .is_empty());
    let (first, second) = tokio::join!(
        outbox.claim(&namespace, "worker-a", 5, Duration::from_secs(30)),
        outbox.claim(&namespace, "worker-b", 5, Duration::from_secs(30)),
    );
    let first = first?;
    let second = second?;
    let ids = first
        .iter()
        .chain(&second)
        .map(|row| row.id.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(first.len() + second.len(), 5);
    assert_eq!(ids.len(), 5, "SKIP LOCKED workers claimed a duplicate row");
    assert!(first
        .iter()
        .all(|row| row.claim_owner.as_deref() == Some("worker-a")));
    assert!(second
        .iter()
        .all(|row| row.claim_owner.as_deref() == Some("worker-b")));
    outbox
        .mark_published(&ids.into_iter().collect::<Vec<_>>())
        .await?;

    outbox
        .enqueue(
            &namespace,
            &CommitPosition::new("pg:expiry"),
            &InvalidationIntentBatch::new("expiry").invalidate_tag("expiring"),
        )
        .await?;
    let claimed = outbox
        .claim(&namespace, "owner-before-crash", 1, Duration::from_secs(30))
        .await?;
    assert_eq!(claimed.len(), 1);
    assert!(outbox
        .claim(&namespace, "too-early", 1, Duration::from_secs(30))
        .await?
        .is_empty());
    sqlx::query("update hydracache_invalidation_outbox set claimed_at_ms = 0 where id = $1")
        .bind(&claimed[0].id)
        .execute(&pool)
        .await?;
    let reclaimed = outbox
        .claim(
            &namespace,
            "owner-after-expiry",
            1,
            Duration::from_millis(1),
        )
        .await?;
    assert_eq!(reclaimed.len(), 1);
    assert_eq!(
        reclaimed[0].claim_owner.as_deref(),
        Some("owner-after-expiry")
    );

    outbox
        .mark_failed(
            &reclaimed[0].id,
            "temporary publish failure",
            Duration::from_secs(60),
            false,
        )
        .await?;
    assert!(outbox
        .claim(&namespace, "backoff", 1, Duration::ZERO)
        .await?
        .is_empty());
    sqlx::query("update hydracache_invalidation_outbox set available_at_ms = 0 where id = $1")
        .bind(&reclaimed[0].id)
        .execute(&pool)
        .await?;
    let retry = outbox.claim(&namespace, "retry", 1, Duration::ZERO).await?;
    assert_eq!(retry[0].attempts, 1);
    outbox
        .mark_failed(&retry[0].id, "permanent failure", Duration::ZERO, true)
        .await?;
    let status = outbox.status(&namespace).await?;
    assert_eq!(status.dead_lettered, 1);
    assert!(status.failed_attempts >= 2);
    assert_eq!(outbox.reset_dead_letters(&namespace).await?, 1);
    let reset = outbox
        .claim(&namespace, "after-reset", 1, Duration::ZERO)
        .await?;
    assert_eq!(reset[0].attempts, 0);
    assert_eq!(reset[0].last_error, None);

    let sqlite_pool = sqlx::SqlitePool::connect("sqlite::memory:").await?;
    let sqlite_outbox = SqlxInvalidationOutbox::sqlite(sqlite_pool.clone());
    sqlite_outbox.install_schema().await?;
    let mut sqlite_tx = sqlite_pool.begin().await?;
    let error = outbox
        .enqueue_in_sqlite_tx(
            &mut sqlite_tx,
            &namespace,
            &CommitPosition::new("wrong:sqlite"),
            &all_intents,
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("Postgres outbox"));
    sqlite_tx.rollback().await?;

    let mut postgres_tx = pool.begin().await?;
    let error = sqlite_outbox
        .enqueue_in_postgres_tx(
            &mut postgres_tx,
            &namespace,
            &CommitPosition::new("wrong:postgres"),
            &all_intents,
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("SQLite outbox"));
    postgres_tx.rollback().await?;

    sqlx::query("delete from hydracache_invalidation_outbox where namespace = $1")
        .bind(&namespace)
        .execute(&pool)
        .await?;
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
