use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache_core::CacheOptions;
use tokio::sync::oneshot;

use crate::tests::common::{user, LoaderError, User};
use crate::HydraCache;

#[tokio::test]
async fn invalidating_tag_during_load_discards_stale_store() {
    let cache = HydraCache::local().build();
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let load_cache = cache.clone();

    let task = tokio::spawn(async move {
        load_cache
            .get_or_load(
                "user:stale",
                CacheOptions::new().tag("users"),
                move || async move {
                    started_tx.send(()).unwrap();
                    release_rx.await.unwrap();
                    Ok::<_, LoaderError>(user(1))
                },
            )
            .await
            .unwrap()
    });

    started_rx.await.unwrap();
    assert_eq!(cache.invalidate_tag("users").await.unwrap(), 0);
    release_tx.send(()).unwrap();

    assert_eq!(task.await.unwrap(), user(1));
    let cached: Option<User> = cache.get("user:stale").await.unwrap();
    assert_eq!(cached, None);
    assert_eq!(cache.stats().stale_load_discards, 1);
}

#[tokio::test]
async fn post_invalidation_caller_starts_fresh_load() {
    let cache = HydraCache::local().build();
    let calls = Arc::new(AtomicUsize::new(0));
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let stale_cache = cache.clone();
    let stale_calls = calls.clone();

    let stale_task = tokio::spawn(async move {
        stale_cache
            .get_or_load(
                "user:race",
                CacheOptions::new().tag("users"),
                move || async move {
                    stale_calls.fetch_add(1, Ordering::SeqCst);
                    started_tx.send(()).unwrap();
                    release_rx.await.unwrap();
                    Ok::<_, LoaderError>(user(1))
                },
            )
            .await
            .unwrap()
    });

    started_rx.await.unwrap();
    assert_eq!(cache.invalidate_tag("users").await.unwrap(), 0);

    let fresh_calls = calls.clone();
    let fresh = cache
        .get_or_load(
            "user:race",
            CacheOptions::new().tag("users"),
            move || async move {
                fresh_calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, LoaderError>(user(2))
            },
        )
        .await
        .unwrap();

    release_tx.send(()).unwrap();

    assert_eq!(fresh, user(2));
    assert_eq!(stale_task.await.unwrap(), user(1));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let cached: Option<User> = cache.get("user:race").await.unwrap();
    assert_eq!(cached, Some(user(2)));
}

#[tokio::test]
async fn flush_and_multi_tag_invalidation_discard_stale_loads() {
    let cache = HydraCache::local().build();
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let load_cache = cache.clone();

    let task = tokio::spawn(async move {
        load_cache
            .get_or_load(
                "user:multi-tag",
                CacheOptions::new().tags(["users", "tenant:1"]),
                move || async move {
                    started_tx.send(()).unwrap();
                    release_rx.await.unwrap();
                    Ok::<_, LoaderError>(user(1))
                },
            )
            .await
            .unwrap()
    });

    started_rx.await.unwrap();
    assert_eq!(cache.invalidate_tag("tenant:1").await.unwrap(), 0);
    release_tx.send(()).unwrap();
    assert_eq!(task.await.unwrap(), user(1));

    let cached: Option<User> = cache.get("user:multi-tag").await.unwrap();
    assert_eq!(cached, None);

    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let flush_cache = cache.clone();
    let flush_task = tokio::spawn(async move {
        flush_cache
            .get_or_load(
                "user:flush",
                CacheOptions::new().tag("users"),
                move || async move {
                    started_tx.send(()).unwrap();
                    release_rx.await.unwrap();
                    Ok::<_, LoaderError>(user(2))
                },
            )
            .await
            .unwrap()
    });

    started_rx.await.unwrap();
    cache.flush().await.unwrap();
    release_tx.send(()).unwrap();
    assert_eq!(flush_task.await.unwrap(), user(2));
    let cached: Option<User> = cache.get("user:flush").await.unwrap();
    assert_eq!(cached, None);
}

#[tokio::test]
async fn untagged_load_is_not_guarded_by_tag_generation() {
    let cache = HydraCache::local().build();
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let load_cache = cache.clone();

    let task = tokio::spawn(async move {
        load_cache
            .get_or_load("user:untagged", CacheOptions::new(), move || async move {
                started_tx.send(()).unwrap();
                release_rx.await.unwrap();
                Ok::<_, LoaderError>(user(1))
            })
            .await
            .unwrap()
    });

    started_rx.await.unwrap();
    assert_eq!(cache.invalidate_tag("users").await.unwrap(), 0);
    release_tx.send(()).unwrap();

    assert_eq!(task.await.unwrap(), user(1));
    let cached: Option<User> = cache.get("user:untagged").await.unwrap();
    assert_eq!(cached, Some(user(1)));
}

#[tokio::test]
async fn stale_load_does_not_overwrite_fresh_value() {
    let cache = HydraCache::local().build();
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let stale_cache = cache.clone();

    let stale_task = tokio::spawn(async move {
        stale_cache
            .get_or_load(
                "user:overwrite",
                CacheOptions::new().tag("users"),
                move || async move {
                    started_tx.send(()).unwrap();
                    release_rx.await.unwrap();
                    Ok::<_, LoaderError>(user(1))
                },
            )
            .await
            .unwrap()
    });

    started_rx.await.unwrap();
    assert_eq!(cache.invalidate_tag("users").await.unwrap(), 0);

    let fresh = cache
        .get_or_load(
            "user:overwrite",
            CacheOptions::new().tag("users"),
            || async { Ok::<_, LoaderError>(user(2)) },
        )
        .await
        .unwrap();

    release_tx.send(()).unwrap();
    assert_eq!(fresh, user(2));
    assert_eq!(stale_task.await.unwrap(), user(1));

    let cached: Option<User> = cache.get("user:overwrite").await.unwrap();
    assert_eq!(cached, Some(user(2)));
}

#[tokio::test]
async fn invalidation_without_in_flight_loader_does_not_increment_stale_discards() {
    let cache = HydraCache::local().build();

    cache
        .put("user:stats", user(1), CacheOptions::new().tag("users"))
        .await
        .unwrap();

    assert_eq!(cache.invalidate_tag("users").await.unwrap(), 1);

    let stats = cache.stats();
    assert_eq!(stats.invalidations, 1);
    assert_eq!(stats.stale_load_discards, 0);
}

#[tokio::test]
async fn per_entry_ttl_overrides_default_ttl() {
    let cache = HydraCache::local()
        .default_ttl(Duration::from_millis(20))
        .build();

    cache
        .put(
            "user:1",
            user(1),
            CacheOptions::new().ttl(Duration::from_millis(120)),
        )
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(cache.contains_key("user:1").await);
}
