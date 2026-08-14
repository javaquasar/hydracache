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

#[tokio::test]
async fn receipt_wait_is_not_blocked_by_a_later_pending_commit() {
    let outbox = InMemoryInvalidationOutbox::new();
    let completed = CommitPosition::new("commit:completed");
    let later = CommitPosition::new("commit:later");
    let batch = InvalidationIntentBatch::new("write").invalidate_tag("users");
    outbox.enqueue("db", &completed, &batch).await.unwrap();
    let completed_rows = outbox
        .claim("db", "worker", 1, Duration::from_secs(30))
        .await
        .unwrap();
    outbox
        .mark_published(&[completed_rows[0].id.clone()])
        .await
        .unwrap();
    outbox.enqueue("db", &later, &batch).await.unwrap();

    let receipt = InvalidationReceipt::new("db", completed.clone());
    let wait =
        InvalidationWait::local(Duration::from_millis(20)).poll_interval(Duration::from_millis(1));
    let outcome = wait.wait(&outbox, &receipt).await.unwrap();

    assert!(outcome.satisfied);
    assert!(!outcome.timed_out);
    assert_eq!(outbox.status("db").await.unwrap().pending, 1);
    assert_eq!(
        outbox
            .status_for_commit("db", &completed)
            .await
            .unwrap()
            .pending,
        0
    );
    assert_eq!(
        outbox
            .status_for_commit("db", &later)
            .await
            .unwrap()
            .pending,
        1
    );
}

#[tokio::test]
async fn dead_lettered_receipt_is_degraded_never_satisfied() {
    let outbox = InMemoryInvalidationOutbox::new();
    let position = CommitPosition::new("commit-dead");
    outbox
        .enqueue(
            "db",
            &position,
            &InvalidationIntentBatch::new("write").invalidate_key("users"),
        )
        .await
        .unwrap();
    let claimed = outbox
        .claim("db", "worker", 1, Duration::from_secs(30))
        .await
        .unwrap();
    outbox
        .mark_failed(&claimed[0].id, "permanent", Duration::ZERO, true)
        .await
        .unwrap();

    let wait = InvalidationWait::local(Duration::from_secs(1));
    let outcome = wait
        .wait(&outbox, &InvalidationReceipt::new("db", position.clone()))
        .await
        .unwrap();

    assert!(!outcome.satisfied);
    assert!(outcome.degraded);
    assert!(!outcome.timed_out);
    assert_eq!(outcome.pending, 0);
    assert_eq!(
        outbox
            .status_for_commit("db", &position)
            .await
            .unwrap()
            .dead_lettered,
        1
    );
    assert_eq!(wait.diagnostics().degraded, 1);
    assert_eq!(wait.diagnostics().satisfied, 0);
}
