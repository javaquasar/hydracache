use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache_core::CacheOptions;

use crate::tests::common::{user, LoaderError, User};
use crate::HydraCache;

#[tokio::test]
async fn concurrent_loader_errors_are_shared_and_retry_is_possible() {
    let cache = HydraCache::local().build();
    let calls = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::new();

    for _ in 0..6 {
        let cache = cache.clone();
        let calls = calls.clone();
        tasks.push(tokio::spawn(async move {
            cache
                .get_or_load("user:error", CacheOptions::new(), move || {
                    let calls = calls.clone();
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        Err::<User, _>(LoaderError)
                    }
                })
                .await
        }));
    }

    for task in tasks {
        assert!(task.await.unwrap().is_err());
    }

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(cache.stats().single_flight_joins, 5);

    let retry_calls = calls.clone();
    let retry = cache
        .get_or_load("user:error", CacheOptions::new(), move || async move {
            retry_calls.fetch_add(1, Ordering::SeqCst);
            Ok::<_, LoaderError>(user(9))
        })
        .await
        .unwrap();

    assert_eq!(retry, user(9));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn stress_concurrent_single_flight_same_key() {
    let cache = HydraCache::local().build();
    let calls = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::new();

    for _ in 0..64 {
        let cache = cache.clone();
        let calls = calls.clone();
        tasks.push(tokio::spawn(async move {
            for _ in 0..8 {
                let value = cache
                    .get_or_load("user:stress", CacheOptions::new().tag("users"), {
                        let calls = calls.clone();
                        move || async move {
                            calls.fetch_add(1, Ordering::SeqCst);
                            tokio::time::sleep(Duration::from_millis(1)).await;
                            Ok::<_, LoaderError>(user(42))
                        }
                    })
                    .await
                    .unwrap();
                assert_eq!(value, user(42));
            }
        }));
    }

    for task in tasks {
        task.await.unwrap();
    }

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(cache.stats().single_flight_joins > 0);
}

#[tokio::test]
async fn stress_concurrent_loads_and_invalidations_stay_usable() {
    let cache = HydraCache::local().build();
    let loads = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::new();

    for worker in 0..24 {
        let cache = cache.clone();
        let loads = loads.clone();
        tasks.push(tokio::spawn(async move {
            for step in 0..20 {
                let key = format!("user:{}", step % 4);
                let expected = user((worker * 100 + step) as u64);

                let loaded = cache
                    .get_or_load(
                        &key,
                        CacheOptions::new().tags(["users", "tenant:stress"]),
                        {
                            let loads = loads.clone();
                            let expected = expected.clone();
                            move || async move {
                                loads.fetch_add(1, Ordering::SeqCst);
                                tokio::task::yield_now().await;
                                Ok::<_, LoaderError>(expected)
                            }
                        },
                    )
                    .await
                    .unwrap();

                assert!(loaded.name.starts_with("user-"));

                if step % 5 == 0 {
                    cache.invalidate_tag("users").await.unwrap();
                }

                if step % 11 == 0 {
                    cache.invalidate_tag("tenant:stress").await.unwrap();
                }
            }
        }));
    }

    for task in tasks {
        task.await.unwrap();
    }

    assert!(loads.load(Ordering::SeqCst) > 0);
    assert!(cache.stats().invalidations > 0);
}

#[tokio::test]
async fn stress_concurrent_put_remove_flush_and_load_stays_usable() {
    let cache = HydraCache::local()
        .default_ttl(Duration::from_millis(250))
        .build();
    let mut tasks = Vec::new();

    for worker in 0..16 {
        let cache = cache.clone();
        tasks.push(tokio::spawn(async move {
            for step in 0..40 {
                let key = format!("mixed:{}", step % 8);
                match (worker + step) % 5 {
                    0 => {
                        cache
                            .put(
                                &key,
                                user((worker * 1000 + step) as u64),
                                CacheOptions::new().tag("mixed"),
                            )
                            .await
                            .unwrap();
                    }
                    1 => {
                        let _: Option<User> = cache.get(&key).await.unwrap();
                    }
                    2 => {
                        cache.remove(&key).await.unwrap();
                    }
                    3 => {
                        let value = user((worker * 1000 + step) as u64);
                        let loaded = cache
                            .get_or_load(&key, CacheOptions::new().tag("mixed"), move || {
                                let value = value.clone();
                                async move {
                                    tokio::task::yield_now().await;
                                    Ok::<_, LoaderError>(value)
                                }
                            })
                            .await
                            .unwrap();
                        assert!(loaded.name.starts_with("user-"));
                    }
                    _ => {
                        if step % 2 == 0 {
                            cache.invalidate_tag("mixed").await.unwrap();
                        } else {
                            cache.flush().await.unwrap();
                        }
                    }
                }
            }
        }));
    }

    for task in tasks {
        task.await.unwrap();
    }

    cache
        .put("mixed:final", user(999), CacheOptions::new().tag("mixed"))
        .await
        .unwrap();
    let cached: Option<User> = cache.get("mixed:final").await.unwrap();
    assert_eq!(cached, Some(user(999)));
}
