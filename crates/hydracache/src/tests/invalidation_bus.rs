use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use hydracache_core::{CacheEventKind, CacheEventOptions, CacheEventOrigin, CacheOptions, Result};
use tokio::sync::watch;

use crate::tests::common::{user, User};
use crate::{
    CacheInvalidation, CacheInvalidationBus, CacheInvalidationMessage, CacheInvalidationReceiver,
    HydraCache, InMemoryInvalidationBus,
};

#[derive(Debug, Clone)]
struct ClosingBus;

#[async_trait]
impl CacheInvalidationBus for ClosingBus {
    async fn publish(&self, _message: CacheInvalidationMessage) -> Result<()> {
        Ok(())
    }

    fn subscribe(&self) -> Box<dyn CacheInvalidationReceiver> {
        Box::new(ClosingReceiver)
    }
}

struct ClosingReceiver;

#[async_trait]
impl CacheInvalidationReceiver for ClosingReceiver {
    async fn recv(&mut self) -> Option<CacheInvalidationMessage> {
        None
    }
}

async fn wait_until<F>(mut condition: F)
where
    F: FnMut() -> bool,
{
    tokio::time::timeout(Duration::from_millis(500), async {
        loop {
            if condition() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("condition should become true before timeout");
}

async fn wait_until_absent(cache: &HydraCache, key: &str) {
    tokio::time::timeout(Duration::from_millis(500), async {
        loop {
            if !cache.contains_key(key).await {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("key should become absent before timeout");
}

#[tokio::test]
async fn shared_bus_propagates_tag_invalidations_between_cache_instances() {
    let bus = Arc::new(InMemoryInvalidationBus::new(16));
    let source = HydraCache::local()
        .shared_invalidation_bus(bus.clone())
        .invalidation_node_id("source")
        .build();
    let target = HydraCache::local()
        .shared_invalidation_bus(bus)
        .invalidation_node_id("target")
        .build();
    let mut target_events =
        target.subscribe(CacheEventOptions::mutations().origin(CacheEventOrigin::DistributedBus));

    source
        .put("user:42", user(42), CacheOptions::new().tag("users"))
        .await
        .unwrap();
    target
        .put("user:42", user(42), CacheOptions::new().tag("users"))
        .await
        .unwrap();

    assert_eq!(source.invalidate_tag("users").await.unwrap(), 1);
    wait_until_absent(&target, "user:42").await;

    let event = tokio::time::timeout(Duration::from_millis(500), target_events.recv())
        .await
        .expect("remote event should arrive")
        .expect("subscription should stay open");
    assert_eq!(event.kind(), CacheEventKind::TagInvalidated);
    assert_eq!(event.origin(), CacheEventOrigin::DistributedBus);
    assert_eq!(event.tag(), Some("users"));
    assert_eq!(event.affected_keys(), Some(1));

    assert_eq!(source.stats().distributed_invalidations_published, 1);
    assert_eq!(source.stats().distributed_invalidations_received, 0);
    assert_eq!(target.stats().distributed_invalidations_received, 1);
    assert_eq!(target.stats().distributed_invalidations_applied, 1);
}

#[tokio::test]
async fn key_invalidation_is_published_even_when_source_does_not_hold_key() {
    let bus = Arc::new(InMemoryInvalidationBus::new(16));
    let source = HydraCache::local()
        .shared_invalidation_bus(bus.clone())
        .invalidation_node_id("source")
        .build();
    let target = HydraCache::local()
        .shared_invalidation_bus(bus)
        .invalidation_node_id("target")
        .build();
    let mut target_events = target.subscribe(
        CacheEventOptions::mutations()
            .include_kind(CacheEventKind::KeyInvalidated)
            .origin(CacheEventOrigin::DistributedBus),
    );

    target
        .put("user:7", user(7), CacheOptions::new())
        .await
        .unwrap();

    assert!(!source.invalidate_key("user:7").await.unwrap());
    wait_until_absent(&target, "user:7").await;

    let event = tokio::time::timeout(Duration::from_millis(500), target_events.recv())
        .await
        .expect("remote key invalidation should arrive")
        .expect("subscription should stay open");
    assert_eq!(event.key(), Some("user:7"));
    assert_eq!(source.stats().distributed_invalidations_published, 1);
    assert_eq!(target.stats().distributed_invalidations_applied, 1);
}

#[tokio::test]
async fn shared_bus_propagates_flush_without_echoing_back_to_source() {
    let bus = Arc::new(InMemoryInvalidationBus::new(16));
    let source = HydraCache::local()
        .shared_invalidation_bus(bus.clone())
        .invalidation_node_id("source")
        .build();
    let target = HydraCache::local()
        .shared_invalidation_bus(bus)
        .invalidation_node_id("target")
        .build();

    source
        .put("source-only", user(1), CacheOptions::new())
        .await
        .unwrap();
    target
        .put("target-only", user(2), CacheOptions::new())
        .await
        .unwrap();

    source.flush().await.unwrap();
    wait_until_absent(&target, "target-only").await;
    wait_until(|| source.stats().distributed_invalidations_published == 1).await;

    assert!(!source.contains_key("source-only").await);
    assert_eq!(source.stats().distributed_invalidations_received, 0);
    assert_eq!(target.stats().distributed_invalidations_received, 1);
    assert_eq!(target.stats().distributed_invalidations_applied, 1);
}

#[tokio::test]
async fn typed_cache_invalidation_uses_physical_keys_on_the_shared_bus() {
    let bus = Arc::new(InMemoryInvalidationBus::new(16));
    let source = HydraCache::local()
        .shared_invalidation_bus(bus.clone())
        .invalidation_node_id("source")
        .build();
    let target = HydraCache::local()
        .shared_invalidation_bus(bus)
        .invalidation_node_id("target")
        .build();
    let source_users = source.typed::<User>("users");
    let target_users = target.typed::<User>("users");

    target_users
        .put("42", user(42), CacheOptions::new())
        .await
        .unwrap();

    assert!(!source_users.invalidate_key("42").await.unwrap());
    wait_until_absent(&target, "users:42").await;

    assert_eq!(target_users.get("42").await.unwrap(), None);
    assert_eq!(source.stats().distributed_invalidations_published, 1);
    assert_eq!(target.stats().distributed_invalidations_applied, 1);
}

#[tokio::test]
async fn owned_bus_builder_generates_observable_node_id_and_handles_closed_receiver() {
    let cache = HydraCache::local().invalidation_bus(ClosingBus).build();

    assert!(cache.invalidation_node_id().starts_with("hydracache-node-"));

    tokio::task::yield_now().await;

    assert_eq!(cache.stats().distributed_invalidations_received, 0);
    assert_eq!(cache.stats().distributed_invalidations_applied, 0);
}

#[tokio::test]
async fn listener_spawn_without_bus_is_a_noop() {
    let cache = HydraCache::local().build();
    let (_shutdown, receiver) = watch::channel(false);

    cache.spawn_invalidation_listener(receiver);

    assert!(cache.invalidation_node_id().starts_with("hydracache-node-"));
}

#[tokio::test]
async fn manual_listener_can_shutdown_without_processing_messages() {
    let bus = Arc::new(InMemoryInvalidationBus::new(16));
    let cache = HydraCache::local()
        .shared_invalidation_bus(bus.clone())
        .invalidation_node_id("manual")
        .build();
    let (shutdown, receiver) = watch::channel(false);

    cache.spawn_invalidation_listener(receiver);
    tokio::task::yield_now().await;
    shutdown.send(true).unwrap();
    tokio::task::yield_now().await;

    assert_eq!(cache.stats().distributed_invalidations_received, 0);
}

#[tokio::test]
async fn listener_exits_when_cache_inner_is_dropped() {
    let bus = Arc::new(InMemoryInvalidationBus::new(16));
    let cache = HydraCache::local()
        .shared_invalidation_bus(bus.clone())
        .invalidation_node_id("transient")
        .build();
    let (_shutdown, receiver) = watch::channel(false);

    cache.spawn_invalidation_listener(receiver);
    drop(cache);

    bus.publish(CacheInvalidationMessage::new(
        "remote",
        CacheInvalidation::flush(),
    ))
    .await
    .unwrap();
    tokio::task::yield_now().await;
}

#[tokio::test]
async fn self_originated_bus_messages_are_ignored_by_listener() {
    let cache = HydraCache::local()
        .invalidation_bus(InMemoryInvalidationBus::new(16))
        .invalidation_node_id("self")
        .build();

    cache
        .put("self-key", user(9), CacheOptions::new())
        .await
        .unwrap();
    assert!(cache.remove("self-key").await.unwrap());
    tokio::time::sleep(Duration::from_millis(20)).await;

    assert_eq!(cache.stats().distributed_invalidations_published, 1);
    assert_eq!(cache.stats().distributed_invalidations_received, 0);
    assert_eq!(cache.stats().distributed_invalidations_applied, 0);
}
