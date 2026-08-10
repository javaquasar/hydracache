use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use hydracache::{
    CacheEventOrigin, CacheInvalidationBus, CacheInvalidationMessage, CacheInvalidationReceive,
    CacheInvalidationReceiver, CacheOptions, CacheResult, HydraCache,
    InMemoryFramedInvalidationBus, InMemoryInvalidationBus,
};

#[tokio::main]
async fn main() -> hydracache::CacheResult<()> {
    // ANCHOR: invalidation-bus
    let bus = Arc::new(InMemoryInvalidationBus::default());
    let first = HydraCache::local()
        .shared_invalidation_bus(bus.clone())
        .invalidation_node_id("first")
        .build();
    let second = HydraCache::local()
        .shared_invalidation_bus(bus)
        .invalidation_node_id("second")
        .build();

    first
        .put("user:42", 42_u64, CacheOptions::new().tag("users"))
        .await?;
    second
        .put("user:42", 42_u64, CacheOptions::new().tag("users"))
        .await?;

    let mut events = second.subscribe_tag("users");
    first.invalidate_tag("users").await?;

    let event = tokio::time::timeout(Duration::from_millis(500), events.recv())
        .await
        .expect("remote invalidation event")
        .expect("subscription stays open");

    assert_eq!(event.origin(), CacheEventOrigin::DistributedBus);
    assert!(!second.contains_key("user:42").await);
    assert_eq!(first.stats().distributed_invalidations_published, 1);
    assert_eq!(second.stats().distributed_invalidations_applied, 1);
    // ANCHOR_END: invalidation-bus

    // ANCHOR: framed-bus
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 128));
    let first = HydraCache::local()
        .shared_invalidation_bus(bus.clone())
        .invalidation_node_id("first")
        .build();
    let second = HydraCache::local()
        .shared_invalidation_bus(bus)
        .invalidation_node_id("second")
        .build();

    let _ = (first, second);
    // ANCHOR_END: framed-bus

    let _custom = MyBus;

    Ok(())
}

// ANCHOR: custom-bus
#[derive(Debug, Clone)]
struct MyBus;

#[async_trait]
impl CacheInvalidationBus for MyBus {
    async fn publish(&self, message: CacheInvalidationMessage) -> CacheResult<()> {
        // Send `message` through Redis, NATS, Postgres LISTEN/NOTIFY, etc.
        let _ = message;
        Ok(())
    }

    fn subscribe(&self) -> Box<dyn CacheInvalidationReceiver> {
        Box::new(MyReceiver)
    }
}

struct MyReceiver;

#[async_trait]
impl CacheInvalidationReceiver for MyReceiver {
    async fn recv(&mut self) -> CacheInvalidationReceive {
        // Return Message(...) for normal delivery, Lagged(n) for skipped messages,
        // and Closed when the stream is no longer usable.
        CacheInvalidationReceive::Closed
    }
}
// ANCHOR_END: custom-bus
