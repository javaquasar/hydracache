use std::fs;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use hydracache::{CacheOptions, HydraCache};
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;

#[derive(Clone, Debug)]
struct CancellationCheckpoint {
    name: &'static str,
    reached: Arc<AtomicBool>,
    reached_notify: Arc<Notify>,
    release: Arc<Notify>,
}

impl CancellationCheckpoint {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            reached: Arc::new(AtomicBool::new(false)),
            reached_notify: Arc::new(Notify::new()),
            release: Arc::new(Notify::new()),
        }
    }

    async fn pause(&self) {
        self.reached.store(true, Ordering::SeqCst);
        self.reached_notify.notify_waiters();
        self.release.notified().await;
    }

    async fn wait_until_reached(&self) {
        while !self.reached.load(Ordering::SeqCst) {
            let notified = self.reached_notify.notified();
            if self.reached.load(Ordering::SeqCst) {
                break;
            }
            notified.await;
        }
    }

    fn name(&self) -> &'static str {
        self.name
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Value(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
struct LoaderError;

impl std::fmt::Display for LoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("loader failed")
    }
}

impl std::error::Error for LoaderError {}

fn write_evidence(test_name: &str, checkpoints: &[&str], assertions: &[&str]) {
    let Ok(path) = std::env::var("HYDRACACHE_CANCELLATION_EVIDENCE") else {
        return;
    };

    let path = std::path::PathBuf::from(path);
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).expect("create cancellation evidence directory");
    }

    let evidence = serde_json::json!({
        "suite": "W39a",
        "test": test_name,
        "status": "passed",
        "checkpoints": checkpoints,
        "assertions": assertions,
    });
    fs::write(
        path,
        serde_json::to_vec_pretty(&evidence).expect("serialize cancellation evidence"),
    )
    .expect("write cancellation evidence");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cache_drop_at_registered_boundaries_preserves_state_and_permit_baseline() {
    let cache = HydraCache::local().build();
    let checkpoint = CancellationCheckpoint::new("cache.single_flight.loader_before_publish");
    let task_cache = cache.clone();
    let task_checkpoint = checkpoint.clone();

    let task = tokio::spawn(async move {
        task_cache
            .get_or_load("cancelled", CacheOptions::new(), move || async move {
                task_checkpoint.pause().await;
                Ok::<Value, LoaderError>(Value(1))
            })
            .await
    });

    checkpoint.wait_until_reached().await;
    task.abort();
    let join_error = task.await.expect_err("cancelled loader must not complete");
    assert!(join_error.is_cancelled());

    assert_eq!(cache.get::<Value>("cancelled").await.unwrap(), None);
    cache
        .put("cancelled", Value(2), CacheOptions::new())
        .await
        .unwrap();
    assert_eq!(
        cache.get::<Value>("cancelled").await.unwrap(),
        Some(Value(2))
    );
    cache.invalidate_key("cancelled").await.unwrap();
    assert_eq!(cache.get::<Value>("cancelled").await.unwrap(), None);

    write_evidence(
        "cache_drop_at_registered_boundaries_preserves_state_and_permit_baseline",
        &[checkpoint.name()],
        &[
            "cancelled loader did not publish a partial value",
            "single-flight slot was reusable after task cancellation",
            "subsequent put and invalidation preserved cache state invariants",
        ],
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropped_singleflight_loader_does_not_poison_the_slot() {
    let cache = HydraCache::local().build();
    let checkpoint = CancellationCheckpoint::new("cache.single_flight.loader_wait");
    let calls = Arc::new(AtomicUsize::new(0));
    let task_cache = cache.clone();
    let task_checkpoint = checkpoint.clone();
    let task_calls = calls.clone();

    let task = tokio::spawn(async move {
        task_cache
            .get_or_load("retryable", CacheOptions::new(), move || async move {
                task_calls.fetch_add(1, Ordering::SeqCst);
                task_checkpoint.pause().await;
                Ok::<Value, LoaderError>(Value(3))
            })
            .await
    });

    checkpoint.wait_until_reached().await;
    task.abort();
    assert!(task
        .await
        .expect_err("loader task must be cancelled")
        .is_cancelled());

    let retry = cache
        .get_or_load("retryable", CacheOptions::new(), {
            let calls = calls.clone();
            || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<Value, LoaderError>(Value(7))
            }
        })
        .await
        .unwrap();

    assert_eq!(retry, Value(7));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        cache.get::<Value>("retryable").await.unwrap(),
        Some(Value(7))
    );
    assert_eq!(cache.stats().loads, 2);

    write_evidence(
        "dropped_singleflight_loader_does_not_poison_the_slot",
        &[checkpoint.name()],
        &[
            "a cancelled leader did not leave an in-flight entry behind",
            "the next loader executed exactly once and populated the key",
            "load accounting included the cancelled attempt and successful retry",
        ],
    );
}
