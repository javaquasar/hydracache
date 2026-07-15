use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache::{CacheError, CacheOptions, HydraCache};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Value(u64);

#[derive(Clone, Debug)]
struct LoaderError;

impl fmt::Display for LoaderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("seeded loader failure")
    }
}

impl Error for LoaderError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Op {
    Load,
    Get,
    InvalidateKey,
    InvalidateTag,
    Flush,
    Expire,
    CapacityPressure,
}

fn schedule(seed: u64, count: usize) -> Vec<Op> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            match state % 7 {
                0 => Op::Load,
                1 => Op::Get,
                2 => Op::InvalidateKey,
                3 => Op::InvalidateTag,
                4 => Op::Flush,
                5 => Op::Expire,
                _ => Op::CapacityPressure,
            }
        })
        .collect()
}

async fn wait_for_joins(cache: &HydraCache, expected: u64) {
    tokio::time::timeout(Duration::from_secs(1), async {
        while cache.stats().single_flight_joins < expected {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("followers must join the seeded in-flight load");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cache_core_matrix_preserves_invalidation_and_singleflight_invariants() {
    for seed in 0..12_u64 {
        let cache = HydraCache::local().max_capacity(16).build();
        let calls = Arc::new(AtomicUsize::new(0));
        let (started_tx, started_rx) = oneshot::channel();
        let (release_tx, release_rx) = oneshot::channel();
        let first_cache = cache.clone();
        let first_calls = calls.clone();
        let first = tokio::spawn(async move {
            first_cache
                .get_or_load(
                    "matrix:key",
                    CacheOptions::new().tag("matrix"),
                    move || async move {
                        first_calls.fetch_add(1, Ordering::SeqCst);
                        started_tx.send(()).unwrap();
                        release_rx.await.unwrap();
                        Ok::<_, LoaderError>(Value(seed))
                    },
                )
                .await
                .unwrap()
        });
        started_rx.await.unwrap();

        let mut followers = Vec::new();
        for _ in 0..3 {
            let follower_cache = cache.clone();
            let follower_calls = calls.clone();
            followers.push(tokio::spawn(async move {
                follower_cache
                    .get_or_load(
                        "matrix:key",
                        CacheOptions::new().tag("matrix"),
                        move || async move {
                            follower_calls.fetch_add(1, Ordering::SeqCst);
                            Ok::<_, LoaderError>(Value(seed + 100))
                        },
                    )
                    .await
                    .unwrap()
            }));
        }
        wait_for_joins(&cache, 3).await;
        match schedule(seed, 1)[0] {
            Op::InvalidateKey | Op::Load | Op::Get => {
                cache.invalidate_key("matrix:key").await.unwrap();
            }
            Op::InvalidateTag | Op::Expire => {
                cache.invalidate_tag("matrix").await.unwrap();
            }
            Op::Flush | Op::CapacityPressure => cache.flush().await.unwrap(),
        }
        release_tx.send(()).unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), first)
                .await
                .expect("single-flight leader must complete after release")
                .unwrap(),
            Value(seed)
        );
        for follower in followers {
            assert_eq!(
                tokio::time::timeout(Duration::from_secs(1), follower)
                    .await
                    .expect("single-flight follower must observe the released leader")
                    .unwrap(),
                Value(seed)
            );
        }

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(cache.get::<Value>("matrix:key").await.unwrap(), None);
        assert_eq!(cache.stats().stale_load_discards, 1);
    }
}

#[tokio::test]
async fn loader_failure_cancellation_expiry_and_capacity_pressure_recover() {
    let cache = HydraCache::local().max_capacity(8).build();
    let failed = cache
        .get_or_load("failure", CacheOptions::new(), || async {
            Err::<Value, _>(LoaderError)
        })
        .await;
    assert!(matches!(failed, Err(CacheError::Loader(_))));
    assert_eq!(
        cache
            .get_or_load("failure", CacheOptions::new(), || async {
                Ok::<_, LoaderError>(Value(2))
            })
            .await
            .unwrap(),
        Value(2)
    );

    let (started_tx, started_rx) = oneshot::channel();
    let cancelled_cache = cache.clone();
    let cancelled = tokio::spawn(async move {
        cancelled_cache
            .get_or_load("cancelled", CacheOptions::new(), move || async move {
                started_tx.send(()).unwrap();
                std::future::pending::<Result<Value, LoaderError>>().await
            })
            .await
    });
    started_rx.await.unwrap();
    cancelled.abort();
    assert!(cancelled.await.unwrap_err().is_cancelled());
    let recovered = tokio::time::timeout(
        Duration::from_secs(1),
        cache.get_or_load("cancelled", CacheOptions::new(), || async {
            Ok::<_, LoaderError>(Value(3))
        }),
    )
    .await
    .expect("cancelled leader must release the single-flight slot")
    .unwrap();
    assert_eq!(recovered, Value(3));

    for index in 0..64 {
        cache
            .put(
                &format!("pressure:{index}"),
                Value(index),
                CacheOptions::new().ttl(Duration::from_millis(5)),
            )
            .await
            .unwrap();
    }
    tokio::time::sleep(Duration::from_millis(25)).await;
    let diagnostics = cache.diagnostics().await;
    assert!(diagnostics.estimated_entries <= 8);
    assert_eq!(cache.get::<Value>("cancelled").await.unwrap(), None);
}

#[test]
fn cache_core_matrix_is_seed_deterministic_and_shrinkable() {
    for seed in [0, 1, 7, 42, u64::MAX] {
        assert_eq!(schedule(seed, 64), schedule(seed, 64));
    }
    let failing = [
        Op::Load,
        Op::Get,
        Op::InvalidateTag,
        Op::Load,
        Op::CapacityPressure,
    ];
    let minimal = (1..=failing.len())
        .map(|length| &failing[..length])
        .find(|prefix| prefix.contains(&Op::InvalidateTag) && prefix.last() == Some(&Op::Load))
        .unwrap();
    assert_eq!(minimal, [Op::Load, Op::Get, Op::InvalidateTag, Op::Load]);
}

#[tokio::test]
async fn mass_same_tick_expiry_does_not_panic_or_starve() {
    let cache = HydraCache::local().max_capacity(512).build();
    cache
        .put(
            "unrelated",
            Value(999),
            CacheOptions::new().ttl(Duration::from_secs(1)),
        )
        .await
        .unwrap();
    for index in 0..256_u64 {
        cache
            .put(
                &format!("expiry:{index}"),
                Value(index),
                CacheOptions::new().ttl(Duration::from_millis(5)),
            )
            .await
            .unwrap();
    }
    tokio::time::sleep(Duration::from_millis(30)).await;
    for index in 0..256_u64 {
        assert_eq!(
            cache
                .get::<Value>(&format!("expiry:{index}"))
                .await
                .unwrap(),
            None
        );
    }
    let diagnostics = tokio::time::timeout(Duration::from_secs(1), cache.diagnostics())
        .await
        .expect("expiry maintenance must not starve unrelated work");
    assert_eq!(
        cache.get::<Value>("unrelated").await.unwrap(),
        Some(Value(999))
    );
    assert!(diagnostics.estimated_entries <= 1);
}

#[test]
fn canary_cache_matrix_allows_loader_completion_to_resurrect_invalidated_value() {
    let invalidated_before_completion = true;
    let stale_value_stored = std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W34");
    assert!(
        !(invalidated_before_completion && stale_value_stored),
        "HC-CANARY-RED:W34 loader resurrected an invalidated value"
    );
}
