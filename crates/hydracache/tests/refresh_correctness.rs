use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache::{CacheError, CacheOptions, HydraCache, RefreshOptions};
use serde::{Deserialize, Serialize};
use tokio::sync::{oneshot, Barrier, Notify};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Value {
    generation: u64,
}

impl Value {
    fn new(generation: u64) -> Self {
        Self { generation }
    }
}

#[derive(Debug, Clone)]
struct LoaderError;

impl fmt::Display for LoaderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("loader failed")
    }
}

impl Error for LoaderError {}

async fn wait_for_stale_discard(cache: &HydraCache) {
    for _ in 0..30 {
        if cache.stats().stale_load_discards > 0 {
            return;
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    panic!("stale refresh result was not discarded");
}

async fn wait_for_loader_call(calls: &AtomicUsize) {
    for _ in 0..30 {
        if calls.load(Ordering::SeqCst) > 0 {
            return;
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    panic!("background loader did not start");
}

#[tokio::test]
async fn background_refresh_after_tag_invalidation_does_not_restore_stale_value() {
    let cache = HydraCache::local().build();
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();

    cache
        .put(
            "correctness:refresh-race",
            Value::new(1),
            CacheOptions::new()
                .ttl(Duration::from_millis(20))
                .tag("race"),
        )
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(45)).await;

    let stale = cache
        .get_or_load_with_refresh(
            "correctness:refresh-race",
            CacheOptions::new().ttl(Duration::from_secs(5)).tag("race"),
            RefreshOptions::new().stale_while_revalidate(Duration::from_secs(5)),
            move || async move {
                started_tx.send(()).unwrap();
                release_rx.await.unwrap();
                Ok::<_, LoaderError>(Value::new(2))
            },
        )
        .await
        .unwrap();

    assert_eq!(stale, Value::new(1));
    started_rx.await.unwrap();

    assert_eq!(cache.invalidate_tag("race").await.unwrap(), 1);
    release_tx.send(()).unwrap();

    wait_for_stale_discard(&cache).await;
    let cached: Option<Value> = cache.get("correctness:refresh-race").await.unwrap();
    assert_eq!(cached, None);
    assert_eq!(cache.stats().stale_load_discards, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_stale_revalidation_shares_one_background_loader() {
    let cache = HydraCache::local().build();
    let calls = Arc::new(AtomicUsize::new(0));
    let release = Arc::new(Notify::new());
    let workers = 12;
    let barrier = Arc::new(Barrier::new(workers));

    cache
        .put(
            "correctness:shared-refresh",
            Value::new(1),
            CacheOptions::new()
                .ttl(Duration::from_millis(20))
                .tag("shared-refresh"),
        )
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(45)).await;

    let mut tasks = Vec::new();
    for _ in 0..workers {
        let cache = cache.clone();
        let calls = calls.clone();
        let release = release.clone();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            cache
                .get_or_load_with_refresh(
                    "correctness:shared-refresh",
                    CacheOptions::new()
                        .ttl(Duration::from_secs(5))
                        .tag("shared-refresh"),
                    RefreshOptions::new().stale_while_revalidate(Duration::from_secs(5)),
                    move || async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        release.notified().await;
                        Ok::<_, LoaderError>(Value::new(2))
                    },
                )
                .await
                .unwrap()
        }));
    }

    wait_for_loader_call(&calls).await;

    for task in tasks {
        assert_eq!(task.await.unwrap(), Value::new(1));
    }

    release.notify_waiters();

    for _ in 0..30 {
        if cache
            .get::<Value>("correctness:shared-refresh")
            .await
            .unwrap()
            == Some(Value::new(2))
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(cache.stats().single_flight_joins >= workers as u64 - 1);
}

#[tokio::test]
async fn stale_on_loader_error_is_bounded_by_the_configured_window() {
    let cache = HydraCache::local().build();

    cache
        .put(
            "correctness:stale-error-window",
            Value::new(1),
            CacheOptions::new().ttl(Duration::from_millis(20)),
        )
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(90)).await;

    let result = cache
        .get_or_load_with_refresh(
            "correctness:stale-error-window",
            CacheOptions::new().ttl(Duration::from_secs(5)),
            RefreshOptions::new().stale_on_loader_error(Duration::from_millis(30)),
            || async { Err::<Value, _>(LoaderError) },
        )
        .await;

    assert!(matches!(result, Err(CacheError::Loader(_))));
    assert!(!cache.contains_key("correctness:stale-error-window").await);
}
