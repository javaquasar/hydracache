use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use hydracache::HydraCache;
use hydracache_db::{
    CommitPosition, InMemoryInvalidationOutbox, InvalidationApplier, InvalidationIntent,
    InvalidationIntentBatch, InvalidationOutbox, InvalidationOutboxWorker, OutboxState,
};

#[derive(Clone, Default)]
struct RecordingApplier {
    applied: Arc<Mutex<Vec<InvalidationIntent>>>,
    fail: Arc<AtomicBool>,
}

impl RecordingApplier {
    fn fail_next(&self) {
        self.fail.store(true, Ordering::SeqCst);
    }

    fn applied(&self) -> Vec<InvalidationIntent> {
        self.applied.lock().unwrap().clone()
    }
}

impl fmt::Debug for RecordingApplier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("RecordingApplier").finish()
    }
}

#[async_trait]
impl InvalidationApplier for RecordingApplier {
    async fn apply_invalidation(&self, intent: &InvalidationIntent) -> hydracache::CacheResult<()> {
        if self.fail.swap(false, Ordering::SeqCst) {
            return Err(hydracache::CacheError::Backend("apply failed".to_owned()));
        }

        self.applied.lock().unwrap().push(intent.clone());
        Ok(())
    }
}

async fn enqueue(
    outbox: &InMemoryInvalidationOutbox,
    namespace: &str,
    commit: &str,
    batch: InvalidationIntentBatch,
) {
    outbox
        .enqueue(namespace, &CommitPosition::new(commit), &batch)
        .await
        .unwrap();
}

#[tokio::test]
async fn custom_adapter_persists_and_replays_intent() {
    let outbox = InMemoryInvalidationOutbox::new();
    let cache = HydraCache::local().build();
    cache
        .put(
            "db:user:42",
            42_u64,
            hydracache::CacheOptions::new().tag("user:42"),
        )
        .await
        .unwrap();
    enqueue(
        &outbox,
        "db",
        "commit:1",
        InvalidationIntentBatch::new("user-write").invalidate_tag("user:42"),
    )
    .await;

    let worker = InvalidationOutboxWorker::new(outbox.clone(), cache.clone(), "db");
    let report = worker.run_once().await.unwrap();

    assert_eq!(report.claimed, 1);
    assert_eq!(report.published, 1);
    assert!(cache.get::<u64>("db:user:42").await.unwrap().is_none());
    assert_eq!(outbox.status("db").await.unwrap().pending, 0);
    assert_eq!(outbox.rows()[0].state, OutboxState::Published);
}

#[tokio::test]
async fn frontier_advances_only_after_apply() {
    let outbox = InMemoryInvalidationOutbox::new();
    let applier = RecordingApplier::default();
    applier.fail_next();
    enqueue(
        &outbox,
        "db",
        "commit:1",
        InvalidationIntentBatch::new("write").invalidate_tag("users"),
    )
    .await;

    let worker = InvalidationOutboxWorker::new(outbox.clone(), applier.clone(), "db")
        .backoff(Duration::from_secs(60))
        .max_attempts(3);
    let report = worker.run_once().await.unwrap();
    let row = outbox.rows().pop().unwrap();

    assert_eq!(report.claimed, 1);
    assert_eq!(report.published, 0);
    assert_eq!(report.retried, 1);
    assert_eq!(row.state, OutboxState::Pending);
    assert_eq!(row.published_at_ms, None);
    assert_eq!(row.attempts, 1);
    assert!(applier.applied().is_empty());
}

#[tokio::test]
async fn worker_retries_failed_publish_with_backoff() {
    let outbox = InMemoryInvalidationOutbox::new();
    let applier = RecordingApplier::default();
    applier.fail_next();
    enqueue(
        &outbox,
        "db",
        "commit:1",
        InvalidationIntentBatch::new("write").invalidate_tag("users"),
    )
    .await;

    let worker = InvalidationOutboxWorker::new(outbox.clone(), applier, "db")
        .backoff(Duration::from_secs(3600))
        .max_attempts(5);

    let report = worker.run_once().await.unwrap();
    let claimed_again = outbox
        .claim("db", "second-worker", 10, Duration::from_secs(0))
        .await
        .unwrap();

    assert_eq!(report.retried, 1);
    assert!(claimed_again.is_empty());
    assert_eq!(outbox.status("db").await.unwrap().pending, 1);
}

#[tokio::test]
async fn worker_dead_letters_after_max_attempts() {
    let outbox = InMemoryInvalidationOutbox::new();
    let applier = RecordingApplier::default();
    applier.fail_next();
    enqueue(
        &outbox,
        "db",
        "commit:1",
        InvalidationIntentBatch::new("write").invalidate_tag("users"),
    )
    .await;

    let worker = InvalidationOutboxWorker::new(outbox.clone(), applier, "db").max_attempts(1);
    let report = worker.run_once().await.unwrap();
    let status = outbox.status("db").await.unwrap();

    assert_eq!(report.dead_lettered, 1);
    assert_eq!(status.dead_lettered, 1);
    assert_eq!(outbox.rows()[0].state, OutboxState::Dead);
}

#[tokio::test]
async fn reset_dead_letters_reenables_rows() {
    let outbox = InMemoryInvalidationOutbox::new();
    let applier = RecordingApplier::default();
    applier.fail_next();
    enqueue(
        &outbox,
        "db",
        "commit:1",
        InvalidationIntentBatch::new("write").invalidate_tag("users"),
    )
    .await;

    let worker =
        InvalidationOutboxWorker::new(outbox.clone(), applier.clone(), "db").max_attempts(1);
    worker.run_once().await.unwrap();

    assert_eq!(worker.reset_dead_letters().await.unwrap(), 1);
    assert_eq!(outbox.status("db").await.unwrap().dead_lettered, 0);

    let report = worker.run_once().await.unwrap();
    assert_eq!(report.published, 1);
}

#[tokio::test]
async fn claim_order_is_oldest_first() {
    let outbox = InMemoryInvalidationOutbox::new();
    enqueue(
        &outbox,
        "db",
        "commit:1",
        InvalidationIntentBatch::new("write").invalidate_tag("first"),
    )
    .await;
    tokio::time::sleep(Duration::from_millis(2)).await;
    enqueue(
        &outbox,
        "db",
        "commit:2",
        InvalidationIntentBatch::new("write").invalidate_tag("second"),
    )
    .await;

    let claimed = outbox
        .claim("db", "worker", 1, Duration::from_secs(30))
        .await
        .unwrap();

    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].intent, InvalidationIntent::tag("first"));
}

#[tokio::test]
async fn namespace_isolation() {
    let outbox = InMemoryInvalidationOutbox::new();
    let applier = RecordingApplier::default();
    enqueue(
        &outbox,
        "db-a",
        "commit:1",
        InvalidationIntentBatch::new("write").invalidate_tag("a"),
    )
    .await;
    enqueue(
        &outbox,
        "db-b",
        "commit:1",
        InvalidationIntentBatch::new("write").invalidate_tag("b"),
    )
    .await;

    let worker = InvalidationOutboxWorker::new(outbox.clone(), applier.clone(), "db-a");
    let report = worker.run_once().await.unwrap();

    assert_eq!(report.published, 1);
    assert_eq!(applier.applied(), vec![InvalidationIntent::tag("a")]);
    assert_eq!(outbox.status("db-a").await.unwrap().pending, 0);
    assert_eq!(outbox.status("db-b").await.unwrap().pending, 1);
}

#[tokio::test]
async fn status_reports_pending_oldest_and_dead() {
    let outbox = InMemoryInvalidationOutbox::new();
    let applier = RecordingApplier::default();
    applier.fail_next();
    enqueue(
        &outbox,
        "db",
        "commit:1",
        InvalidationIntentBatch::new("write").invalidate_tag("dead"),
    )
    .await;
    enqueue(
        &outbox,
        "db",
        "commit:2",
        InvalidationIntentBatch::new("write").invalidate_tag("pending"),
    )
    .await;

    let worker = InvalidationOutboxWorker::new(outbox.clone(), applier, "db")
        .batch_size(1)
        .max_attempts(1);
    worker.run_once().await.unwrap();
    let status = outbox.status("db").await.unwrap();

    assert_eq!(status.pending, 1);
    assert_eq!(status.dead_lettered, 1);
    assert_eq!(status.failed_attempts, 1);
}
