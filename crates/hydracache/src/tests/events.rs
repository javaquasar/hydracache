use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydracache_core::{
    CacheEvent, CacheEventKind, CacheEventOptions, CacheEventOrigin, CacheOptions,
};
use tokio::sync::{mpsc, oneshot};

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
async fn subscriber_options_and_closed_errors_are_observable() {
    assert_eq!(
        CacheEventRecvError::Closed.to_string(),
        "cache event subscription closed"
    );
    assert_eq!(
        CacheEventRecvError::Lagged(2).to_string(),
        "cache event subscriber lagged by 2 events"
    );

    let options = CacheEventOptions::mutations().tag("users");
    let mut events = {
        let cache = HydraCache::local().build();
        let events = cache.subscribe(options.clone());
        assert_eq!(events.options(), &options);
        events
    };

    let result = tokio::time::timeout(Duration::from_millis(500), events.recv())
        .await
        .expect("closed event bus should wake subscriber");
    assert_eq!(result.unwrap_err(), CacheEventRecvError::Closed);
    assert_eq!(events.next_event().await, None);
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

#[tokio::test]
async fn shorthand_subscriptions_cover_mutations_access_key_and_tag_filters() {
    let cache = HydraCache::local().enable_access_events(true).build();
    let mut mutations = cache.subscribe_mutations();
    let mut access = cache.subscribe_access();
    let mut key_events = cache.subscribe_key("user:2");
    let mut tag_events = cache.subscribe_tag("users");

    let missing: Option<User> = cache.get("missing").await.unwrap();
    assert_eq!(missing, None);
    assert_eq!(recv_event(&mut access).await.kind(), CacheEventKind::Miss);

    cache
        .put("user:1", user(1), CacheOptions::new().tag("users"))
        .await
        .unwrap();
    let stored = recv_event(&mut mutations).await;
    assert_eq!(stored.kind(), CacheEventKind::Stored);
    assert_eq!(stored.key(), Some("user:1"));

    let tagged = recv_event(&mut tag_events).await;
    assert_eq!(tagged.kind(), CacheEventKind::Stored);
    assert_eq!(tagged.tags(), &["users".to_owned()]);

    cache
        .put("user:2", user(2), CacheOptions::new().tag("users"))
        .await
        .unwrap();
    let key_event = recv_event(&mut key_events).await;
    assert_eq!(key_event.kind(), CacheEventKind::Stored);
    assert_eq!(key_event.key(), Some("user:2"));
}

#[tokio::test]
async fn next_event_skips_lag_and_returns_latest_matching_event() {
    let cache = HydraCache::local().event_buffer_capacity(1).build();
    let mut events = cache.subscribe_mutations();

    for index in 0..3 {
        cache
            .put(format!("key:{index}").as_str(), index, CacheOptions::new())
            .await
            .unwrap();
    }

    let event = tokio::time::timeout(Duration::from_millis(500), events.next_event())
        .await
        .expect("latest event should arrive")
        .expect("subscription should stay open after lag");

    assert_eq!(event.kind(), CacheEventKind::Stored);
    assert_eq!(event.key(), Some("key:2"));
    assert!(cache.stats().event_subscriber_lagged > 0);
}

#[tokio::test]
async fn callback_listener_receives_events_and_unsubscribe_stops_delivery() {
    let cache = HydraCache::local().build();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let handle = cache.on_mutation(move |event| {
        let _ = tx.send(event.kind());
    });

    cache
        .put("user:1", user(1), CacheOptions::new())
        .await
        .unwrap();

    let kind = tokio::time::timeout(Duration::from_millis(500), rx.recv())
        .await
        .expect("callback should receive event")
        .expect("callback channel should stay open");
    assert_eq!(kind, CacheEventKind::Stored);
    assert!(!handle.is_finished());

    handle.unsubscribe();

    cache
        .put("user:2", user(2), CacheOptions::new())
        .await
        .unwrap();
    let later = tokio::time::timeout(Duration::from_millis(50), rx.recv()).await;
    assert!(
        !matches!(later, Ok(Some(_))),
        "unsubscribed callback should not receive later events"
    );
}

#[tokio::test]
async fn access_callback_receives_events_when_access_events_are_enabled() {
    let cache = HydraCache::local().enable_access_events(true).build();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let handle = cache.on_access(move |event| {
        let _ = tx.send(event.kind());
    });

    let cached: Option<User> = cache.get("missing").await.unwrap();
    assert_eq!(cached, None);

    let kind = tokio::time::timeout(Duration::from_millis(500), rx.recv())
        .await
        .expect("access callback should receive event")
        .expect("access callback channel should stay open");
    assert_eq!(kind, CacheEventKind::Miss);

    handle.unsubscribe();
}

#[tokio::test]
async fn typed_cache_helpers_scope_namespace_key_tag_and_callbacks() {
    let cache = HydraCache::local().build();
    let users = cache.typed::<User>("users");
    let admins = cache.typed::<User>("admins");
    let mut namespace_events = users.subscribe_namespace();
    let mut mutation_events = users.subscribe_mutations();
    let mut key_events = users.subscribe_key("2");
    let mut tag_events = users.subscribe_tag("users");
    let (tx, mut rx) = mpsc::unbounded_channel();
    let handle = users.on_mutation(move |event| {
        let _ = tx.send(event.key().map(str::to_owned));
    });

    admins.put("1", user(1), CacheOptions::new()).await.unwrap();
    users
        .put("1", user(1), CacheOptions::new().tag("users"))
        .await
        .unwrap();

    let namespace_event = recv_event(&mut namespace_events).await;
    assert_eq!(namespace_event.key(), Some("users:1"));

    let mutation_event = recv_event(&mut mutation_events).await;
    assert_eq!(mutation_event.key(), Some("users:1"));

    let tagged = recv_event(&mut tag_events).await;
    assert_eq!(tagged.tags(), &["users".to_owned()]);

    let callback_key = tokio::time::timeout(Duration::from_millis(500), rx.recv())
        .await
        .expect("typed callback should receive event")
        .expect("callback channel should stay open");
    assert_eq!(callback_key.as_deref(), Some("users:1"));

    users.put("2", user(2), CacheOptions::new()).await.unwrap();
    let key_event = recv_event(&mut key_events).await;
    assert_eq!(key_event.key(), Some("users:2"));

    handle.unsubscribe();
}

#[tokio::test]
async fn typed_access_subscription_filters_to_namespace() {
    let cache = HydraCache::local().enable_access_events(true).build();
    let users = cache.typed::<User>("users");
    let admins = cache.typed::<User>("admins");
    let mut events = users.subscribe_access();

    let admin: Option<User> = admins.get("1").await.unwrap();
    let user: Option<User> = users.get("1").await.unwrap();

    assert_eq!(admin, None);
    assert_eq!(user, None);

    let event = recv_event(&mut events).await;
    assert_eq!(event.kind(), CacheEventKind::Miss);
    assert_eq!(event.key(), Some("users:1"));
}
