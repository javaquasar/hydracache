use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache_core::{CacheKeyBuilder, CacheOptions, TagSet};
use tokio::sync::oneshot;

use crate::tests::common::{user, LoaderError, User};
use crate::HydraCache;

#[tokio::test]
async fn typed_cache_puts_gets_and_namespaces_keys() {
    let cache = HydraCache::local().build();
    let users = cache.typed::<User>("users");
    let admins = cache.typed::<User>("admins");

    users.put("1", user(1), CacheOptions::new()).await.unwrap();
    admins.put("1", user(2), CacheOptions::new()).await.unwrap();

    assert_eq!(users.namespace(), "users");
    assert_eq!(users.key("1"), "users:1");
    assert_eq!(users.get("1").await.unwrap(), Some(user(1)));
    assert_eq!(admins.get("1").await.unwrap(), Some(user(2)));
    assert!(cache.contains_key("users:1").await);
    assert!(cache.contains_key("admins:1").await);
}

#[tokio::test]
async fn typed_cache_remove_contains_ttl_and_flush_delegate_to_shared_cache() {
    let cache = HydraCache::local().build();
    let users = cache.typed::<User>("users");
    let admins = cache.typed::<User>("admins");

    users
        .put(
            "1",
            user(1),
            CacheOptions::new().ttl(Duration::from_millis(20)),
        )
        .await
        .unwrap();
    admins.put("1", user(2), CacheOptions::new()).await.unwrap();

    assert!(users.contains_key("1").await);
    tokio::time::sleep(Duration::from_millis(40)).await;
    assert!(!users.contains_key("1").await);
    assert!(admins.remove("1").await.unwrap());
    assert!(!admins.invalidate_key("1").await.unwrap());

    users.put("2", user(2), CacheOptions::new()).await.unwrap();
    admins.put("2", user(3), CacheOptions::new()).await.unwrap();
    users.flush().await.unwrap();

    assert_eq!(users.get("2").await.unwrap(), None);
    assert_eq!(admins.get("2").await.unwrap(), None);
}

#[tokio::test]
async fn typed_cache_loader_helpers_and_errors_work() {
    let cache = HydraCache::local().build();
    let users = cache.typed::<User>("users");

    let infallible = users
        .get_or_insert_with("1", CacheOptions::new(), || async { user(1) })
        .await
        .unwrap();
    let fallible = users
        .try_get_or_insert_with("2", CacheOptions::new(), || async {
            Ok::<_, LoaderError>(user(2))
        })
        .await
        .unwrap();
    let error = users
        .try_get_or_insert_with("error", CacheOptions::new(), || async {
            Err::<User, _>(LoaderError)
        })
        .await;

    assert_eq!(infallible, user(1));
    assert_eq!(fallible, user(2));
    assert!(error.is_err());
    assert!(!users.contains_key("error").await);
}

#[tokio::test]
async fn typed_cache_single_flight_uses_namespaced_key() {
    let cache = HydraCache::local().build();
    let users = cache.typed::<User>("users");
    let calls = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::new();

    for _ in 0..8 {
        let users = users.clone();
        let calls = calls.clone();
        tasks.push(tokio::spawn(async move {
            users
                .get_or_load("1", CacheOptions::new(), move || {
                    let calls = calls.clone();
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        Ok::<_, LoaderError>(user(1))
                    }
                })
                .await
                .unwrap()
        }));
    }

    for task in tasks {
        assert_eq!(task.await.unwrap(), user(1));
    }

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(users.stats().single_flight_joins, 7);
}

#[tokio::test]
async fn typed_cache_key_builder_and_tag_set_integrate_with_runtime() {
    let cache = HydraCache::local().build();
    let users = cache.typed::<User>("users");
    let physical_key = users.key_from(CacheKeyBuilder::new().tenant(7).entity("user", 42));
    let tags = TagSet::new().tag("users").tenant(7).entity("user", 42);

    cache
        .put(&physical_key, user(42), CacheOptions::new().tag_set(tags))
        .await
        .unwrap();

    assert_eq!(physical_key, "users:tenant:7:user:42");
    assert_eq!(cache.invalidate_tag("user:42").await.unwrap(), 1);
    let cached: Option<User> = cache.get(&physical_key).await.unwrap();
    assert_eq!(cached, None);
}

#[tokio::test]
async fn typed_cache_escapes_builder_segments_and_handles_empty_builder() {
    let cache = HydraCache::local().build();
    let users = cache.typed::<User>("users");

    let escaped = users.key_from(
        CacheKeyBuilder::new()
            .tenant("tenant:7")
            .entity("user:type", "42%beta"),
    );

    assert_eq!(users.key_from(CacheKeyBuilder::new()), "users");
    assert_eq!(escaped, "users:tenant:tenant%3A7:user%3Atype:42%25beta");
}

#[tokio::test]
async fn typed_cache_invalidation_during_load_discards_stale_store() {
    let cache = HydraCache::local().build();
    let users = cache.typed::<User>("users");
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let load_users = users.clone();

    let task = tokio::spawn(async move {
        load_users
            .get_or_load("1", CacheOptions::new().tag("users"), move || async move {
                started_tx.send(()).unwrap();
                release_rx.await.unwrap();
                Ok::<_, LoaderError>(user(1))
            })
            .await
            .unwrap()
    });

    started_rx.await.unwrap();
    assert_eq!(users.invalidate_tag("users").await.unwrap(), 0);
    release_tx.send(()).unwrap();

    assert_eq!(task.await.unwrap(), user(1));
    assert_eq!(users.get("1").await.unwrap(), None);
    assert_eq!(users.stats().stale_load_discards, 1);
}

#[tokio::test]
async fn typed_cache_post_invalidation_caller_starts_fresh_load() {
    let cache = HydraCache::local().build();
    let users = cache.typed::<User>("users");
    let calls = Arc::new(AtomicUsize::new(0));
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let stale_users = users.clone();
    let stale_calls = calls.clone();

    let stale_task = tokio::spawn(async move {
        stale_users
            .get_or_load("1", CacheOptions::new().tag("users"), move || async move {
                stale_calls.fetch_add(1, Ordering::SeqCst);
                started_tx.send(()).unwrap();
                release_rx.await.unwrap();
                Ok::<_, LoaderError>(user(1))
            })
            .await
            .unwrap()
    });

    started_rx.await.unwrap();
    assert_eq!(users.invalidate_tag("users").await.unwrap(), 0);

    let fresh_calls = calls.clone();
    let fresh = users
        .get_or_load("1", CacheOptions::new().tag("users"), move || async move {
            fresh_calls.fetch_add(1, Ordering::SeqCst);
            Ok::<_, LoaderError>(user(2))
        })
        .await
        .unwrap();

    release_tx.send(()).unwrap();

    assert_eq!(fresh, user(2));
    assert_eq!(stale_task.await.unwrap(), user(1));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(users.get("1").await.unwrap(), Some(user(2)));
}
