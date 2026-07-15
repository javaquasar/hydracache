#![cfg(feature = "sqlx-outbox")]

use std::error::Error;
use std::time::Duration;

use hydracache_db::{
    CommitPosition, InvalidationIntentBatch, InvalidationOutbox, SqlxInvalidationOutbox,
};

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

#[tokio::test]
async fn sqlite_claim_returns_the_committed_owner_and_claim_timestamp() -> TestResult {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await?;
    let outbox = SqlxInvalidationOutbox::sqlite(pool);
    outbox.install_schema().await?;
    outbox
        .enqueue(
            "claim-contract",
            &CommitPosition::new("sqlite:claim"),
            &InvalidationIntentBatch::new("claim-test").invalidate_key("user:42"),
        )
        .await?;

    let claimed = outbox
        .claim("claim-contract", "worker-a", 1, Duration::from_secs(30))
        .await?;
    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].claim_owner.as_deref(), Some("worker-a"));
    assert!(claimed[0].claimed_at_ms.is_some());
    Ok(())
}
