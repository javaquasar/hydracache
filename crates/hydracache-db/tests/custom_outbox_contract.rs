use std::time::Duration;

use async_trait::async_trait;
use hydracache_db::{
    CommitPosition, InMemoryInvalidationOutbox, InvalidationIntentBatch, InvalidationOutbox,
    InvalidationReceipt, InvalidationWait, OutboxRow, OutboxStatus, Result,
};

/// A downstream-style adapter that deliberately implements the public trait
/// outside the hydracache-db crate. This fixture is the executable migration
/// example for the 0.69 `status_for_commit` addition.
#[derive(Clone, Debug, Default)]
struct CustomInvalidationOutbox {
    storage: InMemoryInvalidationOutbox,
}

#[async_trait]
impl InvalidationOutbox for CustomInvalidationOutbox {
    async fn enqueue(
        &self,
        namespace: &str,
        commit_position: &CommitPosition,
        batch: &InvalidationIntentBatch,
    ) -> Result<usize> {
        self.storage
            .enqueue(namespace, commit_position, batch)
            .await
    }

    async fn claim(
        &self,
        namespace: &str,
        owner: &str,
        limit: usize,
        claim_ttl: Duration,
    ) -> Result<Vec<OutboxRow>> {
        self.storage.claim(namespace, owner, limit, claim_ttl).await
    }

    async fn mark_published(&self, ids: &[String]) -> Result<()> {
        self.storage.mark_published(ids).await
    }

    async fn mark_failed(
        &self,
        id: &str,
        error: &str,
        backoff: Duration,
        dead: bool,
    ) -> Result<()> {
        self.storage.mark_failed(id, error, backoff, dead).await
    }

    async fn reset_dead_letters(&self, namespace: &str) -> Result<u64> {
        self.storage.reset_dead_letters(namespace).await
    }

    async fn status(&self, namespace: &str) -> Result<OutboxStatus> {
        self.storage.status(namespace).await
    }

    async fn status_for_commit(
        &self,
        namespace: &str,
        commit_position: &CommitPosition,
    ) -> Result<OutboxStatus> {
        // Custom adapters must filter by both values. Delegating to a
        // namespace-wide status method would compile but violate 0.69.
        self.storage
            .status_for_commit(namespace, commit_position)
            .await
    }
}

#[tokio::test]
async fn external_adapter_compiles_and_waits_for_only_the_receipt_commit() {
    let outbox = CustomInvalidationOutbox::default();
    let completed = CommitPosition::new("external:completed");
    let later = CommitPosition::new("external:later");
    let batch = InvalidationIntentBatch::new("external-write").invalidate_tag("users");
    outbox.enqueue("db", &completed, &batch).await.unwrap();
    let completed_rows = outbox
        .claim("db", "external-worker", 1, Duration::from_secs(30))
        .await
        .unwrap();
    outbox
        .mark_published(&[completed_rows[0].id.clone()])
        .await
        .unwrap();
    outbox.enqueue("db", &later, &batch).await.unwrap();

    let wait =
        InvalidationWait::local(Duration::from_millis(20)).poll_interval(Duration::from_millis(1));
    let outcome = wait
        .wait(&outbox, &InvalidationReceipt::new("db", completed.clone()))
        .await
        .unwrap();

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
