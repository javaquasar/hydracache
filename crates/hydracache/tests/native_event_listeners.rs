use std::time::Duration;

use hydracache::{
    CacheEvent, CacheEventKind, CacheEventOptions, CacheEventOrigin, CacheEventRecvError,
    CacheEventSubscriber, CacheOptions, HydraCache,
};
use tokio::sync::mpsc;

const EVENT_TIMEOUT: Duration = Duration::from_secs(1);
const QUIET_WINDOW: Duration = Duration::from_millis(100);

async fn recv_event(subscriber: &mut CacheEventSubscriber) -> CacheEvent {
    tokio::time::timeout(EVENT_TIMEOUT, subscriber.recv())
        .await
        .expect("native API event should arrive before the timeout")
        .expect("native API event subscription should remain open")
}

async fn assert_no_event(subscriber: &mut CacheEventSubscriber) {
    assert!(
        tokio::time::timeout(QUIET_WINDOW, subscriber.recv())
            .await
            .is_err(),
        "native API subscriber received an event outside its public filter"
    );
}

#[tokio::test]
async fn public_subscription_delivers_filtered_local_mutations() {
    let cache = HydraCache::local().build();
    let options = CacheEventOptions::new()
        .include_kind(CacheEventKind::Stored)
        .key_prefix("users:")
        .tag("people")
        .origin(CacheEventOrigin::LocalApi);
    let mut events = cache.subscribe(options.clone());

    assert_eq!(events.options(), &options);

    cache
        .put("orders:1", 1_u64, CacheOptions::new().tag("orders"))
        .await
        .expect("unrelated native put should succeed");
    cache
        .put("users:42", 42_u64, CacheOptions::new().tag("people"))
        .await
        .expect("matching native put should succeed");

    let event = recv_event(&mut events).await;
    assert_eq!(event.kind(), CacheEventKind::Stored);
    assert_eq!(event.key(), Some("users:42"));
    assert_eq!(event.tags(), &["people".to_owned()]);
    assert_eq!(event.origin(), CacheEventOrigin::LocalApi);
    assert_eq!(event.affected_keys(), Some(1));

    cache
        .remove("users:42")
        .await
        .expect("native remove should succeed");
    assert_no_event(&mut events).await;
}

#[tokio::test]
async fn public_typed_listener_is_namespaced_and_unsubscribes() {
    let cache = HydraCache::local().build();
    let users = cache.typed::<u64>("users");
    let orders = cache.typed::<u64>("orders");
    let mut subscription = users.subscribe_mutations();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let listener = users.on_mutation(move |event| {
        let _ = tx.send((event.kind(), event.key().map(str::to_owned)));
    });

    orders
        .put("1", 1, CacheOptions::new())
        .await
        .expect("unrelated typed put should succeed");
    users
        .put("42", 42, CacheOptions::new())
        .await
        .expect("matching typed put should succeed");

    let event = recv_event(&mut subscription).await;
    assert_eq!(event.kind(), CacheEventKind::Stored);
    assert_eq!(event.key(), Some("users:42"));

    let callback = tokio::time::timeout(EVENT_TIMEOUT, rx.recv())
        .await
        .expect("typed callback should run before the timeout")
        .expect("typed callback channel should remain open");
    assert_eq!(
        callback,
        (CacheEventKind::Stored, Some("users:42".to_owned()))
    );
    assert!(!listener.is_finished());

    listener.unsubscribe();
    tokio::task::yield_now().await;
    users
        .put("43", 43, CacheOptions::new())
        .await
        .expect("typed put after unsubscribe should succeed");

    let later = tokio::time::timeout(QUIET_WINDOW, rx.recv()).await;
    assert!(
        !matches!(later, Ok(Some(_))),
        "explicit unsubscribe must stop callback delivery"
    );
}

#[tokio::test]
async fn public_access_listener_requires_explicit_enablement() {
    let default_cache = HydraCache::local().build();
    let mut disabled_events = default_cache.subscribe_access();

    let missing: Option<u64> = default_cache
        .get("missing")
        .await
        .expect("native get should succeed");
    assert_eq!(missing, None);
    assert_no_event(&mut disabled_events).await;
    assert_eq!(default_cache.stats().events_published, 0);

    let enabled_cache = HydraCache::local().enable_access_events(true).build();
    let mut enabled_events = enabled_cache.subscribe_access();

    let missing: Option<u64> = enabled_cache
        .get("answer")
        .await
        .expect("native miss should succeed");
    assert_eq!(missing, None);
    assert_eq!(
        recv_event(&mut enabled_events).await.kind(),
        CacheEventKind::Miss
    );

    enabled_cache
        .put("answer", 42_u64, CacheOptions::new())
        .await
        .expect("native put should succeed");
    assert_eq!(
        enabled_cache
            .get::<u64>("answer")
            .await
            .expect("native hit should succeed"),
        Some(42)
    );
    assert_eq!(
        recv_event(&mut enabled_events).await.kind(),
        CacheEventKind::Hit
    );
}

#[tokio::test]
async fn public_slow_subscriber_observes_gap_and_can_resume() {
    let cache = HydraCache::local().event_buffer_capacity(1).build();
    let mut events = cache.subscribe_mutations();

    for index in 0..3_u64 {
        cache
            .put(format!("key:{index}").as_str(), index, CacheOptions::new())
            .await
            .expect("native put must not wait for the slow subscriber");
    }

    let error = tokio::time::timeout(EVENT_TIMEOUT, events.recv())
        .await
        .expect("bounded subscriber should report lag")
        .expect_err("the first receive should expose the skipped event gap");
    assert!(matches!(error, CacheEventRecvError::Lagged(skipped) if skipped > 0));
    assert!(cache.stats().event_subscriber_lagged > 0);

    let latest = tokio::time::timeout(EVENT_TIMEOUT, events.next_event())
        .await
        .expect("subscriber should resume after reporting lag")
        .expect("event bus should remain open");
    assert_eq!(latest.kind(), CacheEventKind::Stored);
    assert_eq!(latest.key(), Some("key:2"));
}
