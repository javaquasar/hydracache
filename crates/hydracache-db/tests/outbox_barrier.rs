use std::time::Duration;

use hydracache::HydraCache;
use hydracache_db::{
    CommitPosition, ConsistencyMode, InMemoryInvalidationOutbox, InvalidationIntentBatch,
    InvalidationOutbox, InvalidationOutboxWorker, InvalidationReceipt, InvalidationWait,
};

#[tokio::test]
async fn local_barrier_succeeds_after_worker_drains() {
    let outbox = InMemoryInvalidationOutbox::new();
    let commit = CommitPosition::new("commit:1");
    let batch = InvalidationIntentBatch::new("write").invalidate_tag("users");
    outbox.enqueue("db", &commit, &batch).await.unwrap();
    let receipt = InvalidationReceipt::new("db", commit);
    let worker = InvalidationOutboxWorker::new(outbox.clone(), HydraCache::local().build(), "db");
    let worker_task = worker.clone();

    let drain = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(5)).await;
        worker_task.run_once().await.unwrap()
    });
    let wait =
        InvalidationWait::local(Duration::from_millis(200)).poll_interval(Duration::from_millis(1));

    let outcome = wait.wait(&outbox, &receipt).await.unwrap();
    let report = drain.await.unwrap();

    assert_eq!(outcome.mode, ConsistencyMode::Local);
    assert!(outcome.satisfied);
    assert!(!outcome.degraded);
    assert!(!outcome.timed_out);
    assert_eq!(outcome.pending, 0);
    assert_eq!(report.published, 1);
    assert_eq!(worker.diagnostics().iterations, 1);
    assert_eq!(worker.diagnostics().published, 1);
    assert_eq!(wait.diagnostics().waits, 1);
    assert_eq!(wait.diagnostics().satisfied, 1);
}

#[tokio::test]
async fn best_effort_timeout_returns_degraded_outcome() {
    let outbox = InMemoryInvalidationOutbox::new();
    let commit = CommitPosition::new("commit:1");
    let batch = InvalidationIntentBatch::new("write").invalidate_tag("users");
    outbox.enqueue("db", &commit, &batch).await.unwrap();
    let receipt = InvalidationReceipt::new("db", commit);
    let wait = InvalidationWait::best_effort(Duration::from_millis(5))
        .poll_interval(Duration::from_millis(1));

    let outcome = wait.wait(&outbox, &receipt).await.unwrap();

    assert_eq!(outcome.mode, ConsistencyMode::BestEffort);
    assert!(!outcome.satisfied);
    assert!(outcome.degraded);
    assert!(outcome.timed_out);
    assert_eq!(outcome.pending, 1);
    assert_eq!(wait.diagnostics().waits, 1);
    assert_eq!(wait.diagnostics().timed_out, 1);
    assert_eq!(wait.diagnostics().degraded, 1);
}

#[tokio::test]
async fn no_wait_preserves_backward_compatible_behavior() {
    let outbox = InMemoryInvalidationOutbox::new();
    let commit = CommitPosition::new("commit:1");
    let batch = InvalidationIntentBatch::new("write").invalidate_tag("users");
    outbox.enqueue("db", &commit, &batch).await.unwrap();
    let receipt = InvalidationReceipt::new("db", commit);
    let wait = InvalidationWait::no_wait();

    let outcome = wait.wait(&outbox, &receipt).await.unwrap();

    assert_eq!(outcome.mode, ConsistencyMode::NoWait);
    assert!(outcome.satisfied);
    assert!(!outcome.degraded);
    assert_eq!(outcome.pending, 0);
    assert_eq!(outbox.status("db").await.unwrap().pending, 1);
    assert_eq!(wait.diagnostics().waits, 1);
    assert_eq!(wait.diagnostics().satisfied, 1);
}
