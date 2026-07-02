use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache::{CacheError, CacheOptions, HydraCache, RefreshOptions};
use serde::{Deserialize, Serialize};
use tokio::sync::Barrier;

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

#[tokio::test]
async fn load_breaker_repeated_load_failure_trips_breaker_and_counts() {
    let cache = breaker_cache(Duration::from_millis(250));
    let calls = Arc::new(AtomicUsize::new(0));

    for _ in 0..2 {
        let calls = calls.clone();
        let error = cache
            .get_or_load("poison", CacheOptions::new(), move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Err::<Value, _>(LoaderError)
            })
            .await
            .unwrap_err();
        assert!(matches!(error, CacheError::Loader(_)));
    }

    let calls_before_fast_fail = calls.load(Ordering::SeqCst);
    let calls_for_fast_fail = calls.clone();
    let error = cache
        .get_or_load("poison", CacheOptions::new(), move || async move {
            calls_for_fast_fail.fetch_add(1, Ordering::SeqCst);
            Ok::<_, LoaderError>(Value::new(1))
        })
        .await
        .unwrap_err();

    assert!(matches!(error, CacheError::Backend(_)));
    assert_eq!(calls.load(Ordering::SeqCst), calls_before_fast_fail);
    let stats = cache.stats();
    assert_eq!(stats.loads, 2);
    assert_eq!(stats.load_breaker_open_total, 1);
    assert_eq!(stats.load_breaker_rejected_total, 1);
    assert!(stats.has_load_breaker_activity());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn load_breaker_open_breaker_fails_fast_and_does_not_stampede_the_loader() {
    let cache = breaker_cache(Duration::from_secs(1));
    let calls = Arc::new(AtomicUsize::new(0));
    open_breaker(&cache, "poison", calls.clone()).await;

    let workers = 12;
    let barrier = Arc::new(Barrier::new(workers));
    let mut tasks = Vec::new();
    for _ in 0..workers {
        let cache = cache.clone();
        let calls = calls.clone();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            cache
                .get_or_load("poison", CacheOptions::new(), move || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, LoaderError>(Value::new(99))
                })
                .await
        }));
    }

    for task in tasks {
        let error = task.await.unwrap().unwrap_err();
        assert!(matches!(error, CacheError::Backend(_)));
    }

    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(cache.stats().loads, 2);
    assert!(cache.stats().load_breaker_rejected_total >= workers as u64);
}

#[tokio::test]
async fn load_breaker_half_open_probe_recovers_or_reopens() {
    let recovered = breaker_cache(Duration::from_millis(30));
    let calls = Arc::new(AtomicUsize::new(0));
    open_breaker(&recovered, "recovering", calls.clone()).await;
    tokio::time::sleep(Duration::from_millis(45)).await;

    let calls_for_probe = calls.clone();
    let value = recovered
        .get_or_load("recovering", CacheOptions::new(), move || async move {
            calls_for_probe.fetch_add(1, Ordering::SeqCst);
            Ok::<_, LoaderError>(Value::new(7))
        })
        .await
        .unwrap();

    assert_eq!(value, Value::new(7));
    assert_eq!(recovered.stats().load_breaker_half_open_total, 1);
    assert_eq!(recovered.stats().load_breaker_recovered_total, 1);

    let reopens = breaker_cache(Duration::from_millis(30));
    let reopen_calls = Arc::new(AtomicUsize::new(0));
    open_breaker(&reopens, "still-poison", reopen_calls.clone()).await;
    tokio::time::sleep(Duration::from_millis(45)).await;

    let calls_for_probe = reopen_calls.clone();
    let error = reopens
        .get_or_load("still-poison", CacheOptions::new(), move || async move {
            calls_for_probe.fetch_add(1, Ordering::SeqCst);
            Err::<Value, _>(LoaderError)
        })
        .await
        .unwrap_err();

    assert!(matches!(error, CacheError::Loader(_)));
    assert_eq!(reopens.stats().load_breaker_half_open_total, 1);
    assert_eq!(reopens.stats().load_breaker_open_total, 2);

    let calls_before_fast_fail = reopen_calls.load(Ordering::SeqCst);
    let calls_for_fast_fail = reopen_calls.clone();
    let error = reopens
        .get_or_load("still-poison", CacheOptions::new(), move || async move {
            calls_for_fast_fail.fetch_add(1, Ordering::SeqCst);
            Ok::<_, LoaderError>(Value::new(8))
        })
        .await
        .unwrap_err();
    assert!(matches!(error, CacheError::Backend(_)));
    assert_eq!(reopen_calls.load(Ordering::SeqCst), calls_before_fast_fail);
}

#[tokio::test]
async fn load_breaker_never_serves_stale_as_fresh() {
    let cache = breaker_cache(Duration::from_millis(250));
    let calls = Arc::new(AtomicUsize::new(0));
    cache
        .put(
            "strict-stale",
            Value::new(1),
            CacheOptions::new().ttl(Duration::from_millis(20)),
        )
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    for _ in 0..2 {
        let calls = calls.clone();
        let error = cache
            .get_or_load_with_refresh(
                "strict-stale",
                CacheOptions::new().ttl(Duration::from_secs(5)),
                RefreshOptions::new(),
                move || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Err::<Value, _>(LoaderError)
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(error, CacheError::Loader(_)));
    }

    let calls_before_fast_fail = calls.load(Ordering::SeqCst);
    let calls_for_fast_fail = calls.clone();
    let error = cache
        .get_or_load_with_refresh(
            "strict-stale",
            CacheOptions::new().ttl(Duration::from_secs(5)),
            RefreshOptions::new(),
            move || async move {
                calls_for_fast_fail.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoaderError>(Value::new(2))
            },
        )
        .await
        .unwrap_err();

    assert!(matches!(error, CacheError::Backend(_)));
    assert_eq!(calls.load(Ordering::SeqCst), calls_before_fast_fail);
    assert_eq!(cache.get::<Value>("strict-stale").await.unwrap(), None);
}

fn breaker_cache(backoff: Duration) -> HydraCache {
    HydraCache::local()
        .load_breaker(2, backoff, backoff.saturating_mul(4))
        .build()
}

async fn open_breaker(cache: &HydraCache, key: &str, calls: Arc<AtomicUsize>) {
    for _ in 0..2 {
        let calls = calls.clone();
        let error = cache
            .get_or_load(key, CacheOptions::new(), move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Err::<Value, _>(LoaderError)
            })
            .await
            .unwrap_err();
        assert!(matches!(error, CacheError::Loader(_)));
    }
    assert_eq!(cache.stats().load_breaker_open_total, 1);
}
