use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use hydracache_core::{CacheError, CacheOptions, Result as CacheResult};
use std::sync::atomic::AtomicUsize;

use crate::tests::common::{user, LoaderError, User};
use crate::HydraCache;

#[tokio::test]
async fn put_then_get() {
    let cache = HydraCache::local().build();

    cache
        .put("user:1", user(1), CacheOptions::new())
        .await
        .unwrap();

    let cached: Option<User> = cache.get("user:1").await.unwrap();
    assert_eq!(cached, Some(user(1)));
}

#[tokio::test]
async fn get_missing_returns_none() {
    let cache = HydraCache::local().build();
    let cached: Option<User> = cache.get("missing").await.unwrap();
    assert_eq!(cached, None);
}

#[tokio::test]
async fn get_or_load_loads_on_miss_and_uses_hit_afterward() {
    let cache = HydraCache::local().build();

    let loaded = cache
        .get_or_load("user:1", CacheOptions::new(), || async {
            Ok::<_, LoaderError>(user(1))
        })
        .await
        .unwrap();
    let hit = cache
        .get_or_load("user:1", CacheOptions::new(), || async {
            Ok::<_, LoaderError>(user(2))
        })
        .await
        .unwrap();

    assert_eq!(loaded, user(1));
    assert_eq!(hit, user(1));
    assert_eq!(cache.stats().loads, 1);
}

#[tokio::test]
async fn loader_helpers_cover_infallible_and_fallible_paths() {
    let cache = HydraCache::local().build();

    let infallible = cache
        .get_or_insert_with("user:1", CacheOptions::new(), || async { user(1) })
        .await
        .unwrap();
    let fallible = cache
        .try_get_or_insert_with("user:2", CacheOptions::new(), || async {
            Ok::<_, LoaderError>(user(2))
        })
        .await
        .unwrap();
    let error = cache
        .try_get_or_insert_with("user:error", CacheOptions::new(), || async {
            Err::<User, _>(LoaderError)
        })
        .await;

    assert_eq!(infallible, user(1));
    assert_eq!(fallible, user(2));
    assert!(matches!(error, Err(CacheError::Loader(_))));
    assert_eq!(cache.stats().loads, 3);
}

#[tokio::test]
async fn ttl_expires_entry_and_contains_key_removes_it() {
    let cache = HydraCache::local().build();

    cache
        .put(
            "user:1",
            user(1),
            CacheOptions::new().ttl(Duration::from_millis(20)),
        )
        .await
        .unwrap();

    assert!(cache.contains_key("user:1").await);
    tokio::time::sleep(Duration::from_millis(40)).await;
    assert!(!cache.contains_key("user:1").await);

    let cached: Option<User> = cache.get("user:1").await.unwrap();
    assert_eq!(cached, None);
}

#[tokio::test]
async fn remove_invalidate_tag_and_flush_clear_expected_entries() {
    let cache = HydraCache::local().build();

    cache
        .put("user:1", user(1), CacheOptions::new().tag("users"))
        .await
        .unwrap();
    cache
        .put("user:2", user(2), CacheOptions::new().tag("users"))
        .await
        .unwrap();
    cache
        .put("order:1", user(3), CacheOptions::new())
        .await
        .unwrap();

    assert!(cache.remove("order:1").await.unwrap());
    assert_eq!(cache.invalidate_tag("users").await.unwrap(), 2);

    let user_1: Option<User> = cache.get("user:1").await.unwrap();
    let order_1: Option<User> = cache.get("order:1").await.unwrap();
    assert_eq!(user_1, None);
    assert_eq!(order_1, None);

    cache
        .put("user:3", user(3), CacheOptions::new())
        .await
        .unwrap();
    cache.flush().await.unwrap();
    let user_3: Option<User> = cache.get("user:3").await.unwrap();
    assert_eq!(user_3, None);
}

#[tokio::test]
async fn overwriting_entry_removes_old_tag_mapping() {
    let cache = HydraCache::local().build();

    cache
        .put("user:1", user(1), CacheOptions::new().tag("old"))
        .await
        .unwrap();
    cache
        .put("user:1", user(2), CacheOptions::new().tag("new"))
        .await
        .unwrap();

    assert_eq!(cache.invalidate_tag("old").await.unwrap(), 0);
    assert!(cache.contains_key("user:1").await);
    assert_eq!(cache.invalidate_tag("new").await.unwrap(), 1);
}

#[tokio::test]
async fn stats_track_hits_misses_loads_invalidations() {
    let cache = HydraCache::local().build();

    let _: Option<User> = cache.get("user:1").await.unwrap();
    cache
        .get_or_load("user:1", CacheOptions::new().tag("users"), || async {
            Ok::<_, LoaderError>(user(1))
        })
        .await
        .unwrap();
    let _: Option<User> = cache.get("user:1").await.unwrap();
    cache.invalidate_tag("users").await.unwrap();

    let stats = cache.stats();
    assert_eq!(stats.misses, 2);
    assert_eq!(stats.loads, 1);
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.invalidations, 1);
}

#[tokio::test]
async fn decode_error_invalidates_bad_entry() {
    let cache = HydraCache::local().build();

    cache
        .put_bytes(
            "user:bad",
            Bytes::from_static(&[0xff, 0xff, 0xff]),
            CacheOptions::new(),
        )
        .await
        .unwrap();

    let result: CacheResult<Option<User>> = cache.get("user:bad").await;
    assert!(matches!(result, Err(CacheError::Decode(_))));

    let cached: Option<User> = cache.get("user:bad").await.unwrap();
    assert_eq!(cached, None);
}

#[tokio::test]
async fn cloned_cache_handles_share_state() {
    let cache = HydraCache::local().build();
    let clone = cache.clone();

    cache
        .put("user:1", user(1), CacheOptions::new())
        .await
        .unwrap();

    let cached: Option<User> = clone.get("user:1").await.unwrap();
    assert_eq!(cached, Some(user(1)));
}

#[tokio::test]
async fn concurrent_misses_share_one_loader_execution() {
    let cache = HydraCache::local().build();
    let calls = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::new();

    for _ in 0..8 {
        let cache = cache.clone();
        let calls = calls.clone();
        tasks.push(tokio::spawn(async move {
            cache
                .get_or_load("user:shared", CacheOptions::new(), move || {
                    let calls = calls.clone();
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        Ok::<_, LoaderError>(user(7))
                    }
                })
                .await
                .unwrap()
        }));
    }

    for task in tasks {
        assert_eq!(task.await.unwrap(), user(7));
    }

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(cache.stats().single_flight_joins, 7);
}
