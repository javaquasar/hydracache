use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use hydracache::{
    transport_metric_descriptors, CacheInvalidation, CacheInvalidationBus, CacheInvalidationFrame,
    CacheInvalidationFrameSink, CacheInvalidationMessage, CacheInvalidationReceive,
    CacheInvalidationReceiver, CacheResult, ClusterGeneration, InMemoryFramedInvalidationBus,
    InMemoryTransport, InvalidationRelay, InvalidationRing, InvalidationTransport, PartitionId,
    TransportConfig, TransportError,
};

const WAIT: Duration = Duration::from_millis(500);
const QUIET: Duration = Duration::from_millis(50);

fn config() -> TransportConfig {
    TransportConfig::new("orders", "local")
        .outbound_capacity(8)
        .inbound_capacity(8)
        .dedup_window(32)
}

fn shared_ring(capacity: usize) -> Arc<tokio::sync::Mutex<InvalidationRing>> {
    Arc::new(tokio::sync::Mutex::new(InvalidationRing::new(
        PartitionId::new(7),
        capacity,
    )))
}

async fn wait_until(mut condition: impl FnMut() -> bool) {
    tokio::time::timeout(WAIT, async {
        loop {
            if condition() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("condition should become true");
}

async fn recv_message(
    subscriber: &mut Box<dyn CacheInvalidationReceiver>,
) -> CacheInvalidationMessage {
    let CacheInvalidationReceive::Message(message) = tokio::time::timeout(WAIT, subscriber.recv())
        .await
        .expect("message should arrive")
    else {
        panic!("expected invalidation message");
    };
    message
}

#[tokio::test]
async fn resume_range_after_gap_closes_from_watermark() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let mut subscriber = bus.subscribe();
    let ring = shared_ring(8);
    {
        let mut ring = ring.lock().await;
        ring.publish(CacheInvalidation::key("already-seen"));
        ring.publish(CacheInvalidation::tag("users"));
        ring.publish(CacheInvalidation::flush());
    }
    let (relay_transport, _peer) = InMemoryTransport::pair(16);
    let handle = InvalidationRelay::spawn_with_metrics(bus, relay_transport, Some(ring), config());

    handle.request_resume(0).unwrap();

    let first = recv_message(&mut subscriber).await;
    let second = recv_message(&mut subscriber).await;
    assert_eq!(first.invalidation().tag_value(), Some("users"));
    assert!(second.invalidation().is_flush());
    wait_until(|| {
        let snapshot = handle.snapshot();
        snapshot.resume_replayed_total == 2 && snapshot.applied_total == 2
    })
    .await;
    handle.abort();
}

#[tokio::test]
async fn resume_fell_behind_emits_clear_partition() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let mut subscriber = bus.subscribe();
    let ring = shared_ring(1);
    {
        let mut ring = ring.lock().await;
        ring.publish(CacheInvalidation::key("old-0"));
        ring.publish(CacheInvalidation::key("old-1"));
        ring.publish(CacheInvalidation::key("retained"));
    }
    let (relay_transport, _peer) = InMemoryTransport::pair(16);
    let handle = InvalidationRelay::spawn_with_metrics(bus, relay_transport, Some(ring), config());

    handle.request_resume(0).unwrap();

    let message = recv_message(&mut subscriber).await;
    assert!(
        message.invalidation().is_flush(),
        "clear-partition is represented as conservative local flush until the frame schema grows"
    );
    wait_until(|| {
        let snapshot = handle.snapshot();
        snapshot.resume_fell_behind_total == 1
            && snapshot.resume_clear_partition_total == 1
            && snapshot.last_clear_partition == Some(PartitionId::new(7))
    })
    .await;
    handle.abort();
}

#[tokio::test]
async fn resume_replayed_events_are_still_fenced_and_deduped() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let mut subscriber = bus.subscribe();
    let ring = shared_ring(8);
    {
        let mut ring = ring.lock().await;
        ring.publish(CacheInvalidation::key("already-seen"));
        ring.publish_with_generation(
            CacheInvalidation::tag("newer"),
            Some(ClusterGeneration::new(3)),
        );
        ring.publish_with_generation(CacheInvalidation::flush(), Some(ClusterGeneration::new(2)));
    }
    let (relay_transport, _peer) = InMemoryTransport::pair(16);
    let handle = InvalidationRelay::spawn_with_metrics(bus, relay_transport, Some(ring), config());

    handle.request_resume(0).unwrap();

    let message = recv_message(&mut subscriber).await;
    assert_eq!(message.invalidation().tag_value(), Some("newer"));
    wait_until(|| {
        let snapshot = handle.snapshot();
        snapshot.resume_replayed_total == 2
            && snapshot.applied_total == 1
            && snapshot.fenced_total == 1
    })
    .await;
    assert!(
        tokio::time::timeout(QUIET, subscriber.recv())
            .await
            .is_err(),
        "stale replayed generation should be fenced"
    );
    handle.abort();
}

#[derive(Debug, Clone)]
struct StalledTransport;

#[async_trait]
impl InvalidationTransport for StalledTransport {
    async fn publish(
        &self,
        _frame: &CacheInvalidationFrame,
    ) -> std::result::Result<(), TransportError> {
        std::future::pending::<()>().await;
        Ok(())
    }

    async fn next_inbound(
        &mut self,
    ) -> Option<std::result::Result<CacheInvalidationFrame, TransportError>> {
        std::future::pending::<Option<std::result::Result<CacheInvalidationFrame, TransportError>>>(
        )
        .await
    }
}

#[tokio::test]
async fn full_outbound_queue_drops_oldest_counts_and_marks_resume() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let ring = shared_ring(8);
    {
        let mut ring = ring.lock().await;
        ring.publish(CacheInvalidation::key("already-seen"));
        ring.publish(CacheInvalidation::key("dropped-gap"));
        ring.publish(CacheInvalidation::tag("resume-gap"));
    }
    let handle = InvalidationRelay::spawn_with_metrics(
        bus.clone(),
        StalledTransport,
        Some(ring),
        config().outbound_capacity(1),
    );

    for key in ["one", "two", "three"] {
        bus.publish(CacheInvalidationMessage::new(
            "local",
            CacheInvalidation::key(key),
        ))
        .await
        .unwrap();
    }

    wait_until(|| {
        let snapshot = handle.snapshot();
        snapshot.outbound_dropped_full_total > 0
            && snapshot.resume_marked_total > 0
            && snapshot.resume_requested_total > 0
            && snapshot.resume_replayed_total > 0
    })
    .await;
    handle.abort();
}

#[derive(Debug, Clone, Default)]
struct SlowApplyBus {
    applied: Arc<AtomicU64>,
}

impl CacheInvalidationFrameSink for SlowApplyBus {
    fn publish_encoded_frame(&self, _bytes: Bytes) -> CacheResult<()> {
        self.applied.fetch_add(1, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(100));
        Ok(())
    }
}

#[async_trait]
impl CacheInvalidationBus for SlowApplyBus {
    async fn publish(&self, _message: CacheInvalidationMessage) -> CacheResult<()> {
        Ok(())
    }

    fn subscribe(&self) -> Box<dyn CacheInvalidationReceiver> {
        Box::new(PendingReceiver)
    }
}

struct PendingReceiver;

#[async_trait]
impl CacheInvalidationReceiver for PendingReceiver {
    async fn recv(&mut self) -> CacheInvalidationReceive {
        std::future::pending::<CacheInvalidationReceive>().await
    }
}

#[tokio::test]
async fn full_inbound_queue_drops_with_counter_not_unbounded() {
    let bus = Arc::new(SlowApplyBus::default());
    let (relay_transport, peer) = InMemoryTransport::pair(16);
    let handle = InvalidationRelay::spawn_with_metrics(
        bus,
        relay_transport,
        None,
        config().inbound_capacity(1),
    );

    for id in 0..6 {
        peer.publish(
            &CacheInvalidationFrame::new(CacheInvalidationMessage::new(
                "remote",
                CacheInvalidation::key(format!("k{id}")),
            ))
            .with_message_id(id),
        )
        .await
        .unwrap();
    }

    wait_until(|| handle.snapshot().inbound_dropped_full_total > 0).await;
    handle.abort();
}

#[tokio::test]
async fn per_source_rate_limit_bounds_flood_and_counts() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let mut subscriber = bus.subscribe();
    let (relay_transport, peer) = InMemoryTransport::pair(16);
    let handle = InvalidationRelay::spawn_with_metrics(
        bus,
        relay_transport,
        None,
        config().inbound_rate_limit_per_source(Some(1)),
    );

    for id in 0..3 {
        peer.publish(
            &CacheInvalidationFrame::new(CacheInvalidationMessage::new(
                "remote",
                CacheInvalidation::key(format!("k{id}")),
            ))
            .with_message_id(id),
        )
        .await
        .unwrap();
    }

    let message = recv_message(&mut subscriber).await;
    assert_eq!(message.invalidation().key_value(), Some("k0"));
    wait_until(|| handle.snapshot().rate_limited_total == 2).await;
    assert!(
        tokio::time::timeout(QUIET, subscriber.recv())
            .await
            .is_err(),
        "rate-limited frames should not apply"
    );
    handle.abort();
}

#[test]
fn metrics_are_bounded_label() {
    for descriptor in transport_metric_descriptors() {
        assert!(!descriptor.labels.contains(&"key"));
        assert!(!descriptor.labels.contains(&"source"));
        assert!(!descriptor.labels.contains(&"cluster_name"));
        assert!(descriptor
            .labels
            .iter()
            .all(|label| matches!(*label, "kind" | "direction")));
    }
}

#[tokio::test]
async fn publish_never_blocks_the_cache_write() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let handle = InvalidationRelay::spawn_with_metrics(
        bus.clone(),
        StalledTransport,
        None,
        config().outbound_capacity(1),
    );

    for id in 0..16 {
        tokio::time::timeout(
            Duration::from_millis(100),
            bus.publish(CacheInvalidationMessage::new(
                "local",
                CacheInvalidation::key(format!("k{id}")),
            )),
        )
        .await
        .expect("local bus publish should not wait for stalled transport")
        .unwrap();
    }

    wait_until(|| handle.snapshot().outbound_dropped_full_total > 0).await;
    handle.abort();
}
