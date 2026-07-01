use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use hydracache::{
    CacheInvalidation, CacheInvalidationBus, CacheInvalidationFrame, CacheInvalidationFrameSink,
    CacheInvalidationMessage, CacheInvalidationReceive, CacheInvalidationReceiver, CacheResult,
    ClusterGeneration, InMemoryFramedInvalidationBus, InMemoryTransport, InvalidationRelay,
    InvalidationRing, InvalidationTransport, PartitionId, TransportConfig, TransportError,
};

const WAIT: Duration = Duration::from_millis(750);
const QUIET: Duration = Duration::from_millis(50);

fn config(local_node_id: &str) -> TransportConfig {
    TransportConfig::new("orders", local_node_id)
        .channel("fault.matrix.orders")
        .outbound_capacity(8)
        .inbound_capacity(8)
        .dedup_window(256)
}

fn shared_ring(capacity: usize) -> Arc<tokio::sync::Mutex<InvalidationRing>> {
    Arc::new(tokio::sync::Mutex::new(InvalidationRing::new(
        PartitionId::new(13),
        capacity,
    )))
}

fn frame(
    source: &str,
    message_id: u64,
    generation: u64,
    invalidation: CacheInvalidation,
) -> CacheInvalidationFrame {
    CacheInvalidationFrame::new(
        CacheInvalidationMessage::new(source, invalidation)
            .with_source_generation(ClusterGeneration::new(generation)),
    )
    .with_cluster_name("orders")
    .with_message_id(message_id)
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

async fn assert_quiet(subscriber: &mut Box<dyn CacheInvalidationReceiver>, reason: &str) {
    assert!(
        tokio::time::timeout(QUIET, subscriber.recv())
            .await
            .is_err(),
        "{reason}"
    );
}

#[tokio::test]
async fn duplicates_are_deduped_and_apply_once() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let mut subscriber = bus.subscribe();
    let (relay_transport, peer) = InMemoryTransport::pair(16);
    let handle = InvalidationRelay::spawn_with_metrics(bus, relay_transport, None, config("local"));

    let duplicate = frame("remote", 7, 1, CacheInvalidation::key("duplicate-once"));
    peer.publish(&duplicate).await.unwrap();
    peer.publish(&duplicate).await.unwrap();

    let message = recv_message(&mut subscriber).await;
    assert_eq!(message.invalidation().key_value(), Some("duplicate-once"));
    wait_until(|| {
        let snapshot = handle.snapshot();
        snapshot.applied_total == 1 && snapshot.deduped_total == 1
    })
    .await;
    assert_quiet(&mut subscriber, "duplicate frame should not apply twice").await;
    handle.abort();
}

#[tokio::test]
async fn reordering_does_not_double_apply_or_resurrect() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let mut subscriber = bus.subscribe();
    let (relay_transport, peer) = InMemoryTransport::pair(16);
    let handle = InvalidationRelay::spawn_with_metrics(bus, relay_transport, None, config("local"));

    peer.publish(&frame(
        "remote",
        1,
        5,
        CacheInvalidation::tag("new-generation"),
    ))
    .await
    .unwrap();
    peer.publish(&frame(
        "remote",
        2,
        4,
        CacheInvalidation::key("stale-resurrection"),
    ))
    .await
    .unwrap();

    let message = recv_message(&mut subscriber).await;
    assert_eq!(message.invalidation().tag_value(), Some("new-generation"));
    wait_until(|| {
        let snapshot = handle.snapshot();
        snapshot.applied_total == 1 && snapshot.fenced_total == 1
    })
    .await;
    assert_quiet(
        &mut subscriber,
        "stale reordered generation must be fenced before apply",
    )
    .await;
    handle.abort();
}

#[tokio::test]
async fn dropped_then_resumed_closes_gap_via_ring() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let mut subscriber = bus.subscribe();
    let ring = shared_ring(8);
    {
        let mut ring = ring.lock().await;
        ring.publish(CacheInvalidation::key("already-seen"));
        ring.publish(CacheInvalidation::tag("gap-closed"));
    }
    let (relay_transport, peer) = InMemoryTransport::pair(16);
    let handle =
        InvalidationRelay::spawn_with_metrics(bus, relay_transport, Some(ring), config("local"));

    peer.publish(&frame(
        "remote",
        0,
        1,
        CacheInvalidation::key("already-seen"),
    ))
    .await
    .unwrap();
    let first = recv_message(&mut subscriber).await;
    assert_eq!(first.invalidation().key_value(), Some("already-seen"));

    peer.try_send_error(TransportError::Backend("fault: dropped link".to_owned()))
        .unwrap();
    let replayed = recv_message(&mut subscriber).await;
    assert_eq!(replayed.invalidation().tag_value(), Some("gap-closed"));
    wait_until(|| {
        let snapshot = handle.snapshot();
        snapshot.resume_requested_total == 1 && snapshot.resume_replayed_total == 1
    })
    .await;
    handle.abort();
}

#[tokio::test]
async fn dropped_beyond_retention_clears_partition() {
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
    let handle =
        InvalidationRelay::spawn_with_metrics(bus, relay_transport, Some(ring), config("local"));

    handle.request_resume(0).unwrap();
    let message = recv_message(&mut subscriber).await;
    assert!(message.invalidation().is_flush());
    wait_until(|| {
        let snapshot = handle.snapshot();
        snapshot.resume_fell_behind_total == 1
            && snapshot.resume_clear_partition_total == 1
            && snapshot.last_clear_partition == Some(PartitionId::new(13))
    })
    .await;
    handle.abort();
}

#[tokio::test]
async fn echo_storm_does_not_amplify() {
    let bus_a = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let bus_b = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let mut subscriber_b = bus_b.subscribe();
    let (transport_a, transport_b) = InMemoryTransport::pair(16);
    let handle_a =
        InvalidationRelay::spawn_with_metrics(bus_a.clone(), transport_a, None, config("node-a"));
    let handle_b =
        InvalidationRelay::spawn_with_metrics(bus_b.clone(), transport_b, None, config("node-b"));

    bus_a
        .publish(CacheInvalidationMessage::new(
            "node-a",
            CacheInvalidation::key("echo-once"),
        ))
        .await
        .unwrap();

    let message = recv_message(&mut subscriber_b).await;
    assert_eq!(message.invalidation().key_value(), Some("echo-once"));
    wait_until(|| {
        let a = handle_a.snapshot();
        let b = handle_b.snapshot();
        a.published_total == 1
            && a.applied_total == 0
            && b.applied_total == 1
            && b.remote_source_suppressed_total == 1
    })
    .await;
    assert_quiet(
        &mut subscriber_b,
        "anti-storm should not amplify a remote inbound frame",
    )
    .await;
    handle_a.abort();
    handle_b.abort();
}

#[tokio::test]
async fn two_transports_deliver_same_message_id_apply_once() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let mut subscriber = bus.subscribe();
    let (left_transport, left_peer) = InMemoryTransport::pair(16);
    let (right_transport, right_peer) = InMemoryTransport::pair(16);
    let handle = InvalidationRelay::spawn_with_metrics(
        bus,
        FanInTransport {
            left: left_transport,
            right: right_transport,
        },
        None,
        config("local"),
    );

    let duplicate = frame("remote", 99, 3, CacheInvalidation::tag("fan-in"));
    left_peer.publish(&duplicate).await.unwrap();
    right_peer.publish(&duplicate).await.unwrap();

    let message = recv_message(&mut subscriber).await;
    assert_eq!(message.invalidation().tag_value(), Some("fan-in"));
    wait_until(|| {
        let snapshot = handle.snapshot();
        snapshot.applied_total == 1 && snapshot.deduped_total == 1
    })
    .await;
    handle.abort();
}

#[tokio::test]
async fn foreign_cluster_isolation_holds_under_fault_injection() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let mut subscriber = bus.subscribe();
    let (relay_transport, peer) = InMemoryTransport::pair(16);
    let handle = InvalidationRelay::spawn_with_metrics(bus, relay_transport, None, config("local"));

    for id in 0..4 {
        let foreign = CacheInvalidationFrame::new(CacheInvalidationMessage::new(
            "remote",
            CacheInvalidation::key(format!("foreign-{id}")),
        ))
        .with_cluster_name("payments")
        .with_message_id(id);
        peer.publish(&foreign).await.unwrap();
    }
    peer.publish(&frame(
        "remote",
        10,
        1,
        CacheInvalidation::key("orders-only"),
    ))
    .await
    .unwrap();

    let message = recv_message(&mut subscriber).await;
    assert_eq!(message.invalidation().key_value(), Some("orders-only"));
    wait_until(|| {
        let snapshot = handle.snapshot();
        snapshot.foreign_cluster_dropped_total == 4 && snapshot.applied_total == 1
    })
    .await;
    handle.abort();
}

#[tokio::test]
async fn flood_is_bounded_and_counted() {
    let bus = Arc::new(SlowApplyBus::default());
    let (relay_transport, peer) = InMemoryTransport::pair(64);
    let handle = InvalidationRelay::spawn_with_metrics(
        bus.clone(),
        relay_transport,
        None,
        config("local").inbound_capacity(1),
    );

    for id in 0..32 {
        peer.publish(&frame(
            "remote",
            id,
            1,
            CacheInvalidation::key(format!("burst-{id}")),
        ))
        .await
        .unwrap();
    }

    wait_until(|| {
        let snapshot = handle.snapshot();
        snapshot.inbound_dropped_full_total > 0 && bus.applied.load(Ordering::Relaxed) > 0
    })
    .await;
    let snapshot = handle.snapshot();
    assert!(
        snapshot.inbound_dropped_full_total < 32,
        "bounded queue should drop some, not invent an unbounded flood"
    );
    handle.abort();
}

#[tokio::test]
async fn soak_1000_seeds_no_resurrection_no_unbounded_growth() {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 2048));
    let mut subscriber = bus.subscribe();
    let (relay_transport, peer) = InMemoryTransport::pair(4096);
    let handle = InvalidationRelay::spawn_with_metrics(
        bus,
        relay_transport,
        None,
        config("local")
            .inbound_capacity(256)
            .dedup_window(4096)
            .inbound_rate_limit_per_source(Some(4)),
    );

    let mut newest_first_seeds = HashSet::new();
    for seed in 0..1000_u64 {
        let source = format!("remote-{seed}");
        let base_id = seed.saturating_mul(4);
        let newest_first = lcg(seed).is_multiple_of(2);
        let newer = frame(
            &source,
            base_id,
            2,
            CacheInvalidation::key(format!("seed-{seed}-newer")),
        );
        let older = frame(
            &source,
            base_id + 1,
            1,
            CacheInvalidation::key(format!("seed-{seed}-older")),
        );
        if newest_first {
            newest_first_seeds.insert(seed);
            peer.publish(&newer).await.unwrap();
            peer.publish(&older).await.unwrap();
        } else {
            peer.publish(&older).await.unwrap();
            peer.publish(&newer).await.unwrap();
        }
        if lcg(seed ^ 0x5eed).is_multiple_of(3) {
            peer.publish(&newer).await.unwrap();
        }
    }

    let mut applied_keys = HashSet::new();
    for _ in 0..2000 {
        let receive = tokio::time::timeout(QUIET, subscriber.recv()).await;
        let Ok(CacheInvalidationReceive::Message(message)) = receive else {
            break;
        };
        if let Some(key) = message.invalidation().key_value() {
            applied_keys.insert(key.to_owned());
        }
    }

    for seed in newest_first_seeds {
        assert!(
            !applied_keys.contains(&format!("seed-{seed}-older")),
            "seed {seed} applied a stale frame after a newer generation"
        );
    }
    wait_until(|| {
        let snapshot = handle.snapshot();
        snapshot.received_total >= 2000
            && snapshot.fenced_total > 0
            && snapshot.inbound_dropped_full_total <= snapshot.received_total
    })
    .await;
    handle.abort();
}

fn lcg(seed: u64) -> u64 {
    seed.wrapping_mul(6364136223846793005).wrapping_add(1)
}

#[derive(Debug, Clone)]
struct FanInTransport {
    left: InMemoryTransport,
    right: InMemoryTransport,
}

#[async_trait]
impl InvalidationTransport for FanInTransport {
    async fn publish(
        &self,
        _frame: &CacheInvalidationFrame,
    ) -> std::result::Result<(), TransportError> {
        Ok(())
    }

    async fn next_inbound(
        &mut self,
    ) -> Option<std::result::Result<CacheInvalidationFrame, TransportError>> {
        tokio::select! {
            left = self.left.next_inbound() => left,
            right = self.right.next_inbound() => right,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct SlowApplyBus {
    applied: Arc<AtomicU64>,
}

impl CacheInvalidationFrameSink for SlowApplyBus {
    fn publish_encoded_frame(&self, _bytes: Bytes) -> CacheResult<()> {
        self.applied.fetch_add(1, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(25));
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
