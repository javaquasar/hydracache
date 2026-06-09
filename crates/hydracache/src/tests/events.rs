use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache_core::{
    CacheEvent, CacheEventKind, CacheEventOptions, CacheEventOrigin, CacheOptions,
};
use tokio::sync::oneshot;

use crate::events::CacheEventRecvError;
use crate::tests::common::{user, LoaderError, User};
use crate::{CacheEventSubscriber, HydraCache};

async fn recv_event(subscriber: &mut CacheEventSubscriber) -> CacheEvent {
    tokio::time::timeout(Duration::from_millis(500), subscriber.recv())
        .await
        .expect("event should arrive before timeout")
        .expect("subscription should stay open")
}

async fn assert_no_event(subscriber: &mut CacheEventSubscriber) {
    assert!(
        tokio::time::timeout(Duration::from_millis(50), subscriber.recv())
            .await
            .is_err(),
        "no matching event should be delivered"
    );
}

#[tokio::test]
async fn mutation_subscriber_receives_store_remove_invalidate_and_flush_events() {
    let cache = HydraCache::local().build();
    let mut events = cache.subscribe(CacheEventOptions::mutations());

    cache
        .put("user:1", user(1), CacheOptions::new().tag("users"))
        .await
        .unwrap();
    let stored = recv_event(&mut events).await;
    assert_eq!(stored.kind(), CacheEventKind::Stored);
    assert_eq!(stored.key(), Some("user:1"));
    assert_eq!(stored.tags(), &["users".to_owned()]);

    assert!(cache.remove("user:1").await.unwrap());
    let removed = recv_event(&mut events).await;
    assert_eq!(removed.kind(), CacheEventKind::Removed);
    assert_eq!(removed.key(), Some("user:1"));

    cache
        .put("user:2", user(2), CacheOptions::new().tag("users"))
        .await
        .unwrap();
    let _ = recv_event(&mut events).await;

    assert!(cache.invalidate_key("user:2").await.unwrap());
    let invalidated = recv_event(&mut events).await;
    assert_eq!(invalidated.kind(), CacheEventKind::KeyInvalidated);
    assert_eq!(invalidated.key(), Some("user:2"));

    cache
        .put("user:3", user(3), CacheOptions::new().tag("users"))
        .await
        .unwrap();
    let _ = recv_event(&mut events).await;

    assert_eq!(cache.invalidate_tag("users").await.unwrap(), 1);
    let tag_invalidated = recv_event(&mut events).await;
    assert_eq!(tag_invalidated.kind(), CacheEventKind::TagInvalidated);
    assert_eq!(tag_invalidated.tag(), Some("users"));
    assert_eq!(tag_invalidated.affected_keys(), Some(1));

    cache
        .put("user:4", user(4), CacheOptions::new())
        .await
        .unwrap();
    let _ = recv_event(&mut events).await;

    cache.flush().await.unwrap();
    let flushed = recv_event(&mut events).await;
    assert_eq!(flushed.kind(), CacheEventKind::Flushed);
    assert!(flushed.affected_keys().is_some());
}

#[tokio::test]
async fn access_events_are_disabled_by_default() {
    let cache = HydraCache::local().build();
    let mut events = cache.subscribe(CacheEventOptions::access());

    let cached: Option<User> = cache.get("missing").await.unwrap();

    assert_eq!(cached, None);
    assert_no_event(&mut events).await;
    assert_eq!(cache.stats().events_published, 0);
}

#[tokio::test]
async fn access_events_can_be_enabled_for_miss_load_store_and_hit_flow() {
    let cache = HydraCache::local().enable_access_events(true).build();
    let mut events = cache.subscribe(CacheEventOptions::new());

    let first = cache
        .get_or_insert_with("answer", CacheOptions::new().tag("answers"), || async {
            42_u64
        })
        .await
        .unwrap();
    let second = cache
        .get_or_insert_with("answer", CacheOptions::new().tag("answers"), || async {
            7_u64
        })
        .await
        .unwrap();

    assert_eq!((first, second), (42, 42));

    let expected = [
        CacheEventKind::Miss,
        CacheEventKind::LoadStarted,
        CacheEventKind::Stored,
        CacheEventKind::LoadCompleted,
        CacheEventKind::Hit,
    ];

    for kind in expected {
        let event = recv_event(&mut events).await;
        assert_eq!(event.kind(), kind);
        assert_eq!(event.key(), Some("answer"));
    }

    assert_eq!(cache.stats().events_published, expected.len() as u64);
}

#[tokio::test]
async fn event_subscriber_filters_by_kind_key_prefix_tag_and_origin() {
    let cache = HydraCache::local().build();
    let mut events = cache.subscribe(
        CacheEventOptions::mutations()
            .include_kind(CacheEventKind::Stored)
            .key_prefix("users:")
            .tag("users")
            .origin(CacheEventOrigin::LocalApi),
    );

    cache
        .put("orders:1", user(1), CacheOptions::new().tag("orders"))
        .await
        .unwrap();
    cache
        .put("users:1", user(1), CacheOptions::new().tag("users"))
        .await
        .unwrap();

    let event = recv_event(&mut events).await;
    assert_eq!(event.kind(), CacheEventKind::Stored);
    assert_eq!(event.key(), Some("users:1"));
    assert_eq!(event.tags(), &["users".to_owned()]);
}

#[tokio::test]
async fn slow_subscribers_report_lag_without_blocking_cache_operations() {
    let cache = HydraCache::local().event_buffer_capacity(1).build();
    let mut events = cache.subscribe(CacheEventOptions::mutations());

    for index in 0..3 {
        cache
            .put(format!("key:{index}").as_str(), index, CacheOptions::new())
            .await
            .unwrap();
    }

    let result = tokio::time::timeout(Duration::from_millis(500), events.recv())
        .await
        .expect("lag should be reported before timeout");

    assert!(matches!(result, Err(CacheEventRecvError::Lagged(_))));
    assert!(cache.stats().event_subscriber_lagged > 0);
    assert_eq!(cache.stats().events_published, 3);
}

#[tokio::test]
async fn typed_cache_subscription_observes_physical_namespaced_keys() {
    let cache = HydraCache::local().build();
    let users = cache.typed::<User>("users");
    let mut events = users.subscribe(CacheEventOptions::mutations().key_prefix("users:"));

    users.put("1", user(1), CacheOptions::new()).await.unwrap();

    let event = recv_event(&mut events).await;
    assert_eq!(event.kind(), CacheEventKind::Stored);
    assert_eq!(event.key(), Some("users:1"));
}

#[tokio::test]
async fn single_flight_join_event_is_emitted_when_access_events_are_enabled() {
    let cache = HydraCache::local().enable_access_events(true).build();
    let calls = Arc::new(AtomicUsize::new(0));
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let mut events =
        cache.subscribe(CacheEventOptions::new().include_kind(CacheEventKind::SingleFlightJoined));

    let owner_cache = cache.clone();
    let owner_calls = calls.clone();
    let owner = tokio::spawn(async move {
        owner_cache
            .get_or_load("shared", CacheOptions::new(), move || async move {
                owner_calls.fetch_add(1, Ordering::SeqCst);
                started_tx.send(()).unwrap();
                release_rx.await.unwrap();
                Ok::<_, LoaderError>(user(1))
            })
            .await
            .unwrap()
    });

    started_rx.await.unwrap();

    let joiner_cache = cache.clone();
    let joiner = tokio::spawn(async move {
        joiner_cache
            .get_or_load("shared", CacheOptions::new(), || async {
                Ok::<_, LoaderError>(user(2))
            })
            .await
            .unwrap()
    });

    let event = recv_event(&mut events).await;
    assert_eq!(event.kind(), CacheEventKind::SingleFlightJoined);
    assert_eq!(event.key(), Some("shared"));

    release_tx.send(()).unwrap();
    assert_eq!(owner.await.unwrap(), user(1));
    assert_eq!(joiner.await.unwrap(), user(1));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn stale_load_discard_emits_event() {
    let cache = HydraCache::local().build();
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let mut events = cache.subscribe(CacheEventOptions::mutations());
    let load_cache = cache.clone();

    let task = tokio::spawn(async move {
        load_cache
            .get_or_load(
                "user:1",
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
    let tag_event = recv_event(&mut events).await;
    assert_eq!(tag_event.kind(), CacheEventKind::TagInvalidated);

    release_tx.send(()).unwrap();
    assert_eq!(task.await.unwrap(), user(1));

    let stale_event = recv_event(&mut events).await;
    assert_eq!(stale_event.kind(), CacheEventKind::StaleLoadDiscarded);
    assert_eq!(stale_event.key(), Some("user:1"));
}

#[tokio::test]
async fn load_failed_event_is_emitted_when_access_events_are_enabled() {
    let cache = HydraCache::local().enable_access_events(true).build();
    let mut events =
        cache.subscribe(CacheEventOptions::new().include_kind(CacheEventKind::LoadFailed));

    let error = cache
        .try_get_or_insert_with("user:error", CacheOptions::new(), || async {
            Err::<User, _>(LoaderError)
        })
        .await;

    assert!(error.is_err());
    let event = recv_event(&mut events).await;
    assert_eq!(event.kind(), CacheEventKind::LoadFailed);
    assert_eq!(event.key(), Some("user:error"));
}
