use hydracache::{cacheable_infallible, CacheEventKind, CacheOptions, HydraCache};

#[tokio::main]
async fn main() -> hydracache::CacheResult<()> {
    // ANCHOR: diagnostics
    let cache = HydraCache::local().build();

    let first = cacheable_infallible!(
        cache = cache,
        key = "expensive:42",
        tags = ["expensive"],
        ttl_secs = 60,
        load = || async { 42_u64 },
    )
    .await?;

    let second = cacheable_infallible!(
        cache = cache,
        key = "expensive:42",
        tags = ["expensive"],
        ttl_secs = 60,
        load = || async { 7_u64 },
    )
    .await?;

    let diagnostics = cache.diagnostics().await;

    assert_eq!((first, second), (42, 42));
    assert_eq!(diagnostics.stats.loads, 1);
    assert_eq!(diagnostics.stats.hits, 1);
    assert_eq!(diagnostics.total_requests(), 2);
    assert_eq!(diagnostics.hit_ratio(), Some(0.5));
    assert!(!diagnostics.is_empty());
    // ANCHOR_END: diagnostics

    // ANCHOR: tag-events
    let cache = HydraCache::local().build();
    let mut events = cache.subscribe_tag("users");

    cache
        .put("user:42", 42_u64, CacheOptions::new().tag("users"))
        .await?;

    let event = events.recv().await.expect("stored event");
    assert_eq!(event.kind(), CacheEventKind::Stored);
    assert_eq!(event.key(), Some("user:42"));

    cache.invalidate_tag("users").await?;
    let invalidation = events.recv().await.expect("tag invalidation");
    assert_eq!(invalidation.kind(), CacheEventKind::TagInvalidated);
    // ANCHOR_END: tag-events

    // ANCHOR: access-events
    let cache = HydraCache::local()
        .enable_access_events(true)
        .event_buffer_capacity(256)
        .build();
    let mut events = cache.subscribe_access();

    let answer = cache
        .get_or_insert_with("answer", CacheOptions::new(), || async { 42_u64 })
        .await?;

    assert_eq!(answer, 42);
    let event = events.next_event().await.expect("access event");
    assert_eq!(event.kind(), CacheEventKind::Miss);
    // ANCHOR_END: access-events

    // ANCHOR: callbacks
    let cache = HydraCache::local().build();
    let listener = cache.on_mutation(|event| {
        println!("cache changed: {event:?}");
    });

    cache.put("user:42", 42_u64, CacheOptions::new()).await?;
    listener.unsubscribe();
    // ANCHOR_END: callbacks

    Ok(())
}
