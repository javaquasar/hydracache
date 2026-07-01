use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use hydracache::{
    CacheInvalidation, CacheInvalidationBus, CacheInvalidationFrame, CacheInvalidationFrameSink,
    CacheInvalidationMessage, CacheInvalidationReceive, CacheInvalidationReceiver, CacheResult,
    ClusterGeneration, InMemoryFramedInvalidationBus, InMemoryTransport, InvalidationRelay,
    InvalidationTransport, TransportConfig, TransportError, CACHE_INVALIDATION_FRAME_VERSION,
};

const WAIT: Duration = Duration::from_millis(500);
const QUIET: Duration = Duration::from_millis(50);

fn config() -> TransportConfig {
    TransportConfig::new("orders", "local")
        .outbound_capacity(8)
        .inbound_capacity(8)
        .dedup_window(16)
}

fn frame(
    source_id: &str,
    cluster_name: Option<&str>,
    message_id: Option<u64>,
    generation: Option<u64>,
    invalidation: CacheInvalidation,
) -> CacheInvalidationFrame {
    let mut message = CacheInvalidationMessage::new(source_id, invalidation);
    if let Some(generation) = generation {
        message = message.with_source_generation(ClusterGeneration::new(generation));
    }

    let mut frame = CacheInvalidationFrame::new(message);
    if let Some(cluster_name) = cluster_name {
        frame = frame.with_cluster_name(cluster_name);
    }
    if let Some(message_id) = message_id {
        frame = frame.with_message_id(message_id);
    }
    frame
}

async fn recv_frame(transport: &mut InMemoryTransport) -> CacheInvalidationFrame {
    tokio::time::timeout(WAIT, transport.next_inbound())
        .await
        .expect("frame should arrive")
        .expect("transport should stay open")
        .expect("transport should yield a valid frame")
}

async fn assert_no_frame(transport: &mut InMemoryTransport) {
    assert!(
        tokio::time::timeout(QUIET, transport.next_inbound())
            .await
            .is_err(),
        "relay should not publish an extra frame"
    );
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

#[tokio::test]
async fn outbound_key_tag_flush_are_published_to_transport() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let (relay_transport, mut peer) = InMemoryTransport::pair(16);
    let handle =
        InvalidationRelay::spawn_with_metrics(bus.clone(), relay_transport, None, config());

    assert_eq!(bus.receiver_count(), 1);

    bus.publish(CacheInvalidationMessage::new(
        "local",
        CacheInvalidation::key("user:42"),
    ))
    .await
    .unwrap();
    bus.publish(CacheInvalidationMessage::new(
        "local",
        CacheInvalidation::tag("users"),
    ))
    .await
    .unwrap();
    bus.publish(CacheInvalidationMessage::new(
        "local",
        CacheInvalidation::flush(),
    ))
    .await
    .unwrap();

    let key = recv_frame(&mut peer).await;
    let tag = recv_frame(&mut peer).await;
    let flush = recv_frame(&mut peer).await;

    assert_eq!(key.cluster_name(), Some("orders"));
    assert_eq!(key.source_id(), "local");
    assert_eq!(key.message_id(), Some(1));
    assert_eq!(key.invalidation().key_value(), Some("user:42"));
    assert_eq!(tag.invalidation().tag_value(), Some("users"));
    assert_eq!(tag.message_id(), Some(2));
    assert!(flush.invalidation().is_flush());
    assert_eq!(flush.message_id(), Some(3));

    wait_until(|| handle.snapshot().published_total == 3).await;
    handle.abort();
}

#[tokio::test]
async fn inbound_is_applied_locally_and_not_republished() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let mut subscriber = bus.subscribe();
    let (relay_transport, mut peer) = InMemoryTransport::pair(16);
    let handle = InvalidationRelay::spawn_with_metrics(bus, relay_transport, None, config());

    let inbound = frame(
        "remote",
        Some("orders"),
        Some(42),
        Some(1),
        CacheInvalidation::tag("users"),
    );
    peer.publish(&inbound).await.unwrap();

    let CacheInvalidationReceive::Message(message) = tokio::time::timeout(WAIT, subscriber.recv())
        .await
        .expect("local bus should receive inbound frame")
    else {
        panic!("expected local bus message");
    };
    assert_eq!(message.source_id(), "remote");
    assert_eq!(message.invalidation().tag_value(), Some("users"));

    wait_until(|| {
        let snapshot = handle.snapshot();
        snapshot.applied_total == 1 && snapshot.remote_source_suppressed_total == 1
    })
    .await;
    assert_no_frame(&mut peer).await;
    handle.abort();
}

#[tokio::test]
async fn own_inbound_frames_are_dropped_before_local_apply() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let mut subscriber = bus.subscribe();
    let (relay_transport, peer) = InMemoryTransport::pair(16);
    let handle = InvalidationRelay::spawn_with_metrics(bus, relay_transport, None, config());

    let own = frame(
        "local",
        Some("orders"),
        Some(1),
        Some(1),
        CacheInvalidation::flush(),
    );
    peer.publish(&own).await.unwrap();

    wait_until(|| handle.snapshot().own_frame_dropped_total == 1).await;
    assert!(
        tokio::time::timeout(QUIET, subscriber.recv())
            .await
            .is_err(),
        "own frame should not reach the local bus"
    );
    handle.abort();
}

#[tokio::test]
async fn duplicate_message_ids_are_deduped_with_a_bounded_window() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let mut subscriber = bus.subscribe();
    let (relay_transport, peer) = InMemoryTransport::pair(16);
    let handle = InvalidationRelay::spawn_with_metrics(bus, relay_transport, None, config());
    let duplicate = frame(
        "remote",
        Some("orders"),
        Some(99),
        Some(1),
        CacheInvalidation::key("user:42"),
    );

    peer.publish(&duplicate).await.unwrap();
    peer.publish(&duplicate).await.unwrap();

    let CacheInvalidationReceive::Message(message) = tokio::time::timeout(WAIT, subscriber.recv())
        .await
        .expect("first frame should apply")
    else {
        panic!("expected first frame");
    };
    assert_eq!(message.invalidation().key_value(), Some("user:42"));
    wait_until(|| handle.snapshot().deduped_total == 1).await;
    assert!(
        tokio::time::timeout(QUIET, subscriber.recv())
            .await
            .is_err(),
        "duplicate should be dropped"
    );
    handle.abort();
}

#[tokio::test]
async fn reordered_older_generation_is_fenced_and_not_applied() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let mut subscriber = bus.subscribe();
    let (relay_transport, peer) = InMemoryTransport::pair(16);
    let handle = InvalidationRelay::spawn_with_metrics(bus, relay_transport, None, config());

    peer.publish(&frame(
        "remote",
        Some("orders"),
        Some(10),
        Some(3),
        CacheInvalidation::tag("users"),
    ))
    .await
    .unwrap();
    peer.publish(&frame(
        "remote",
        Some("orders"),
        Some(11),
        Some(2),
        CacheInvalidation::flush(),
    ))
    .await
    .unwrap();

    let CacheInvalidationReceive::Message(message) = tokio::time::timeout(WAIT, subscriber.recv())
        .await
        .expect("newer generation should apply")
    else {
        panic!("expected newer generation");
    };
    assert_eq!(message.source_generation(), Some(ClusterGeneration::new(3)));

    wait_until(|| handle.snapshot().fenced_total == 1).await;
    assert!(
        tokio::time::timeout(QUIET, subscriber.recv())
            .await
            .is_err(),
        "older generation should be fenced"
    );
    handle.abort();
}

#[tokio::test]
async fn foreign_cluster_frames_are_dropped_and_counted() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let mut subscriber = bus.subscribe();
    let (relay_transport, peer) = InMemoryTransport::pair(16);
    let handle = InvalidationRelay::spawn_with_metrics(bus, relay_transport, None, config());

    peer.publish(&frame(
        "remote",
        Some("payments"),
        Some(1),
        Some(1),
        CacheInvalidation::key("user:42"),
    ))
    .await
    .unwrap();

    wait_until(|| handle.snapshot().foreign_cluster_dropped_total == 1).await;
    assert!(
        tokio::time::timeout(QUIET, subscriber.recv())
            .await
            .is_err(),
        "foreign cluster frame should not reach the local bus"
    );
    handle.abort();
}

#[tokio::test]
async fn unknown_and_malformed_inbound_frames_are_loud_and_nonfatal() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let mut subscriber = bus.subscribe();
    let (relay_transport, peer) = InMemoryTransport::pair(16);
    let handle = InvalidationRelay::spawn_with_metrics(bus, relay_transport, None, config());
    let sender = peer.clone();

    peer.try_send_error(TransportError::UnknownFrameVersion {
        found: CACHE_INVALIDATION_FRAME_VERSION + 1,
        max_supported: CACHE_INVALIDATION_FRAME_VERSION,
    })
    .unwrap();
    peer.try_send_error(TransportError::Decode("corrupted frame".to_owned()))
        .unwrap();
    sender
        .publish(&frame(
            "remote",
            Some("orders"),
            Some(7),
            Some(1),
            CacheInvalidation::tag("users"),
        ))
        .await
        .unwrap();

    wait_until(|| {
        let snapshot = handle.snapshot();
        snapshot.unknown_version_total == 1 && snapshot.decode_error_total == 1
    })
    .await;

    let CacheInvalidationReceive::Message(message) = tokio::time::timeout(WAIT, subscriber.recv())
        .await
        .expect("valid frame after errors should still apply")
    else {
        panic!("expected valid frame after transport errors");
    };
    assert_eq!(message.invalidation().tag_value(), Some("users"));
    handle.abort();
}

#[tokio::test]
async fn outbound_relay_ignores_peer_bus_messages_but_publishes_local_ones() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let (relay_transport, mut peer) = InMemoryTransport::pair(16);
    let handle =
        InvalidationRelay::spawn_with_metrics(bus.clone(), relay_transport, None, config());

    bus.publish(CacheInvalidationMessage::new(
        "peer",
        CacheInvalidation::tag("from-peer"),
    ))
    .await
    .unwrap();
    bus.publish(CacheInvalidationMessage::new(
        "local",
        CacheInvalidation::tag("from-local"),
    ))
    .await
    .unwrap();

    let outbound = recv_frame(&mut peer).await;
    assert_eq!(outbound.source_id(), "local");
    assert_eq!(outbound.invalidation().tag_value(), Some("from-local"));
    wait_until(|| handle.snapshot().remote_source_suppressed_total == 1).await;
    assert_no_frame(&mut peer).await;
    handle.abort();
}

#[derive(Debug, Clone, Default)]
struct LaggingBus;

#[async_trait]
impl CacheInvalidationBus for LaggingBus {
    async fn publish(&self, _message: CacheInvalidationMessage) -> CacheResult<()> {
        Ok(())
    }

    fn subscribe(&self) -> Box<dyn CacheInvalidationReceiver> {
        Box::new(LaggingReceiver {
            returned_lag: false,
        })
    }
}

impl CacheInvalidationFrameSink for LaggingBus {
    fn publish_encoded_frame(&self, _bytes: Bytes) -> CacheResult<()> {
        Ok(())
    }
}

struct LaggingReceiver {
    returned_lag: bool,
}

#[async_trait]
impl CacheInvalidationReceiver for LaggingReceiver {
    async fn recv(&mut self) -> CacheInvalidationReceive {
        if self.returned_lag {
            CacheInvalidationReceive::Closed
        } else {
            self.returned_lag = true;
            CacheInvalidationReceive::Lagged(7)
        }
    }
}

#[tokio::test]
async fn bus_lag_is_counted_by_the_outbound_relay() {
    let bus = Arc::new(LaggingBus);
    let (relay_transport, _peer) = InMemoryTransport::pair(16);
    let handle = InvalidationRelay::spawn_with_metrics(bus, relay_transport, None, config());

    wait_until(|| handle.snapshot().bus_lag_total == 7).await;
    handle.abort();
}
