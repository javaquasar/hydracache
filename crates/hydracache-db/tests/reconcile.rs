use std::time::Duration;

use hydracache::HydraCache;
use hydracache_db::{
    CommitPosition, DriftReason, DriftStatus, HookDialect, HookDrift, HookSchemaVersion,
    InMemoryInvalidationOutbox, InvalidationIntentBatch, InvalidationOutbox,
    InvalidationOutboxWorker, OutboxLagPolicy, ReconciliationReport, HOOK_SCHEMA_ARTIFACT,
};

fn expected_hook() -> HookSchemaVersion {
    HookSchemaVersion {
        artifact: HOOK_SCHEMA_ARTIFACT.to_owned(),
        version: 1,
        table: "users".to_owned(),
        dialect: HookDialect::Sqlite,
    }
}

#[tokio::test]
async fn reconcile_clean_state_reports_clean() {
    let outbox = InMemoryInvalidationOutbox::new();
    let hook = expected_hook();

    let report = ReconciliationReport::from_outbox(
        &outbox,
        "db",
        HookDrift::new(hook.clone(), Some(hook)),
        OutboxLagPolicy::default(),
    )
    .await
    .unwrap();

    assert_eq!(report.status(), DriftStatus::Clean);
}

#[tokio::test]
async fn reconcile_missing_hook_version_reports_drift() {
    let outbox = InMemoryInvalidationOutbox::new();

    let report = ReconciliationReport::from_outbox(
        &outbox,
        "db",
        HookDrift::missing(expected_hook()),
        OutboxLagPolicy::default(),
    )
    .await
    .unwrap();

    let DriftStatus::Drift(reasons) = report.status() else {
        panic!("missing hook must report drift");
    };
    assert!(reasons
        .iter()
        .any(|reason| matches!(reason, DriftReason::HookSchemaMissing { .. })));
}

#[tokio::test]
async fn reconcile_outbox_backlog_reports_lag() {
    let outbox = InMemoryInvalidationOutbox::new();
    outbox
        .enqueue(
            "db",
            &CommitPosition::new("commit-1"),
            &InvalidationIntentBatch::new("write").invalidate_tag("users"),
        )
        .await
        .unwrap();
    let hook = expected_hook();

    let report = ReconciliationReport::from_outbox(
        &outbox,
        "db",
        HookDrift::new(hook.clone(), Some(hook)),
        OutboxLagPolicy::default(),
    )
    .await
    .unwrap();

    let DriftStatus::Drift(reasons) = report.status() else {
        panic!("pending outbox must report drift");
    };
    assert!(reasons
        .iter()
        .any(|reason| matches!(reason, DriftReason::OutboxPendingRows { pending: 1, .. })));
}

#[tokio::test]
async fn reconcile_manual_invalidation_clears_drift_where_applicable() {
    let outbox = InMemoryInvalidationOutbox::new();
    outbox
        .enqueue(
            "db",
            &CommitPosition::new("commit-1"),
            &InvalidationIntentBatch::new("write").invalidate_tag("users"),
        )
        .await
        .unwrap();
    let cache = HydraCache::local().build();
    let worker = InvalidationOutboxWorker::new(outbox.clone(), cache, "db")
        .claim_ttl(Duration::from_secs(1));
    worker.run_once().await.unwrap();
    let hook = expected_hook();

    let report = ReconciliationReport::from_outbox(
        &outbox,
        "db",
        HookDrift::new(hook.clone(), Some(hook)),
        OutboxLagPolicy::default(),
    )
    .await
    .unwrap();

    assert_eq!(report.status(), DriftStatus::Clean);
}
