use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use hydracache::{
    CacheInvalidation, CacheInvalidationBus, CacheInvalidationFrame, CacheInvalidationMessage,
    CacheInvalidationReceive, CacheInvalidationReceiver, ClusterGeneration,
    InMemoryFramedInvalidationBus, InMemoryTransport, InvalidationRelay, InvalidationRing,
    InvalidationTransport, PartitionId, TransportConfig, TransportError,
    CACHE_INVALIDATION_FRAME_VERSION,
};
use hydracache_transport_nats::{
    decode_nats_payload, default_nats_subject, NatsInvalidationTransport, NatsTransportConfig,
};
use testcontainers_modules::{
    nats::Nats,
    testcontainers::{runners::AsyncRunner, ContainerAsync, ImageExt},
};
use tokio::time::{sleep, timeout};

const WAIT: Duration = Duration::from_millis(500);

#[tokio::test]
async fn roundtrips_key_tag_flush_frames_over_nats(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let Some(container) = start_nats_or_skip().await else {
        return Ok(());
    };
    let url = nats_url(&container).await?;
    let subject = unique_subject("roundtrip");
    let publisher = connect_transport(&url, &subject, "publisher").await?;
    let mut subscriber = connect_transport(&url, &subject, "subscriber").await?;
    sleep(Duration::from_millis(100)).await;

    for (message_id, invalidation) in [
        (1, CacheInvalidation::key("users:42")),
        (2, CacheInvalidation::tag("users")),
        (3, CacheInvalidation::flush()),
    ] {
        let frame = frame("remote", message_id, invalidation);
        publisher.publish(&frame).await?;
        let received = receive_matching(&mut subscriber, message_id).await?;
        assert_eq!(received, frame);
    }

    Ok(())
}

#[tokio::test]
async fn reconnect_triggers_ring_resume_without_gap(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let mut subscriber = bus.subscribe();
    let ring = Arc::new(tokio::sync::Mutex::new(InvalidationRing::new(
        PartitionId::new(9),
        8,
    )));
    {
        let mut ring = ring.lock().await;
        ring.publish(CacheInvalidation::key("already-seen"));
        ring.publish(CacheInvalidation::tag("missed-on-reconnect"));
    }
    let (relay_transport, peer) = InMemoryTransport::pair(16);
    let handle = InvalidationRelay::spawn_with_metrics(
        Arc::clone(&bus),
        relay_transport,
        Some(ring),
        TransportConfig::new("orders", "node-b").channel(unique_subject("resume")),
    );

    peer.publish(
        &CacheInvalidationFrame::new(CacheInvalidationMessage::new(
            "remote",
            CacheInvalidation::key("already-seen"),
        ))
        .with_cluster_name("orders")
        .with_message_id(0),
    )
    .await?;
    let first = recv_message(&mut subscriber).await;
    assert_eq!(first.invalidation().key_value(), Some("already-seen"));

    peer.try_send_error(TransportError::Backend(
        "simulated nats reconnect".to_owned(),
    ))?;
    let replayed = recv_message(&mut subscriber).await;
    assert_eq!(
        replayed.invalidation().tag_value(),
        Some("missed-on-reconnect")
    );
    wait_until(|| {
        let snapshot = handle.snapshot();
        snapshot.transport_error_total == 1
            && snapshot.resume_requested_total == 1
            && snapshot.resume_replayed_total == 1
            && snapshot.applied_total == 2
    })
    .await;
    handle.abort();
    Ok(())
}

#[test]
fn nats_backend_uses_the_unmodified_w1_trait() {
    fn assert_transport<T: InvalidationTransport + Clone + Send + Sync + 'static>() {}
    assert_transport::<NatsInvalidationTransport>();
}

#[tokio::test]
async fn slow_nats_does_not_block_a_cache_write(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 16));
    let handle = InvalidationRelay::spawn_with_metrics(
        Arc::clone(&bus),
        StalledNatsTransport,
        None,
        TransportConfig::new("orders", "node-a")
            .channel(unique_subject("stalled"))
            .outbound_capacity(1),
    );

    let publish_result = timeout(
        Duration::from_millis(100),
        bus.publish(CacheInvalidationMessage::new(
            "node-a",
            CacheInvalidation::key("fast-path"),
        )),
    )
    .await;
    assert!(
        publish_result.is_ok(),
        "local bus publish waited on stalled NATS transport"
    );
    publish_result??;
    handle.abort();
    Ok(())
}

#[test]
fn future_frame_version_is_reported_as_unknown_version() {
    let frame = frame("remote", 1, CacheInvalidation::key("future"));
    let mut payload = frame.encode().expect("frame encodes").to_vec();
    payload[0] = (CACHE_INVALIDATION_FRAME_VERSION + 1) as u8;

    let error = decode_nats_payload(Bytes::from(payload)).expect_err("future frame is rejected");
    assert!(matches!(
        error,
        TransportError::UnknownFrameVersion {
            found: 2,
            max_supported: CACHE_INVALIDATION_FRAME_VERSION
        }
    ));
}

#[test]
fn default_subject_is_cluster_scoped() {
    assert_eq!(default_nats_subject("orders"), "hydracache.inval.orders");
    assert!(
        NatsTransportConfig::for_cluster("nats://127.0.0.1:4222", "orders", "node-a")
            .subject()
            .ends_with(".orders")
    );
}

#[derive(Debug, Clone)]
struct StalledNatsTransport;

#[async_trait]
impl InvalidationTransport for StalledNatsTransport {
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

fn frame(source: &str, message_id: u64, invalidation: CacheInvalidation) -> CacheInvalidationFrame {
    CacheInvalidationFrame::new(
        CacheInvalidationMessage::new(source, invalidation)
            .with_source_generation(ClusterGeneration::new(7)),
    )
    .with_cluster_name("orders")
    .with_message_id(message_id)
}

async fn connect_transport(
    url: &str,
    subject: &str,
    node: &str,
) -> Result<NatsInvalidationTransport, TransportError> {
    let core = TransportConfig::new("orders", node)
        .channel(subject)
        .inbound_capacity(8)
        .outbound_capacity(8)
        .reconnect_backoff(Duration::from_millis(25));
    NatsInvalidationTransport::connect(NatsTransportConfig::new(url, core)).await
}

async fn receive_matching(
    transport: &mut NatsInvalidationTransport,
    message_id: u64,
) -> Result<CacheInvalidationFrame, Box<dyn std::error::Error + Send + Sync>> {
    let deadline = Duration::from_secs(5);
    loop {
        let frame = timeout(deadline, transport.next_inbound()).await?;
        match frame {
            Some(Ok(frame)) if frame.message_id() == Some(message_id) => return Ok(frame),
            Some(Ok(_)) => continue,
            Some(Err(error)) => return Err(Box::new(error)),
            None => return Err("NATS transport closed".into()),
        }
    }
}

async fn recv_message(
    subscriber: &mut Box<dyn CacheInvalidationReceiver>,
) -> CacheInvalidationMessage {
    let CacheInvalidationReceive::Message(message) = timeout(WAIT, subscriber.recv())
        .await
        .expect("message should arrive")
    else {
        panic!("expected invalidation message");
    };
    message
}

async fn wait_until(mut condition: impl FnMut() -> bool) {
    timeout(WAIT, async {
        loop {
            if condition() {
                return;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("condition should become true");
}

async fn start_nats_or_skip() -> Option<ContainerAsync<Nats>> {
    match Nats::default().with_tag("2.10.14").start().await {
        Ok(container) => Some(container),
        Err(error) => {
            eprintln!(
                "skipping NATS testcontainers integration test because Docker is unavailable: {error}"
            );
            None
        }
    }
}

async fn nats_url(
    container: &ContainerAsync<Nats>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let host = container.get_host().await?;
    let port = container.get_host_port_ipv4(4222).await?;
    Ok(format!("nats://{host}:{port}"))
}

fn unique_subject(prefix: &str) -> String {
    format!(
        "hydracache.test.{prefix}.{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after unix epoch")
            .as_nanos()
    )
}
