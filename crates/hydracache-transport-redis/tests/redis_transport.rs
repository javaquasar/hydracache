use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use hydracache::{
    CacheInvalidation, CacheInvalidationBus, CacheInvalidationFrame, CacheInvalidationMessage,
    ClusterGeneration, InMemoryFramedInvalidationBus, InvalidationRelay, InvalidationTransport,
    TransportConfig, TransportError, CACHE_INVALIDATION_FRAME_VERSION,
};
use hydracache_transport_redis::{
    decode_redis_payload, RedisInvalidationTransport, RedisTransportConfig,
};
use testcontainers_modules::{
    redis::{Redis, REDIS_PORT},
    testcontainers::{runners::AsyncRunner, ContainerAsync, ImageExt},
};
use tokio::time::{sleep, timeout};

#[tokio::test]
async fn roundtrips_key_tag_flush_frames_over_redis(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let Some(container) = start_redis_or_skip().await else {
        return Ok(());
    };
    let url = redis_url(&container).await?;
    let channel = unique_channel("roundtrip");
    let publisher = connect_transport(&url, &channel, "publisher").await?;
    let mut subscriber = connect_transport(&url, &channel, "subscriber").await?;
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
async fn undecodable_payload_is_loud_and_does_not_stop_subscriber(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let Some(container) = start_redis_or_skip().await else {
        return Ok(());
    };
    let url = redis_url(&container).await?;
    let channel = unique_channel("decode");
    let publisher = connect_transport(&url, &channel, "publisher").await?;
    let mut subscriber = connect_transport(&url, &channel, "subscriber").await?;
    sleep(Duration::from_millis(100)).await;

    publisher
        .publish_encoded(Bytes::from_static(b"not-a-hydracache-frame"))
        .await?;
    let first = timeout(Duration::from_secs(5), subscriber.next_inbound()).await?;
    assert!(matches!(first, Some(Err(TransportError::Decode(_)))));

    let frame = frame("remote", 10, CacheInvalidation::key("users:99"));
    publisher.publish(&frame).await?;
    let received = receive_matching(&mut subscriber, 10).await?;
    assert_eq!(received.invalidation().key_value(), Some("users:99"));

    Ok(())
}

#[tokio::test]
async fn channel_scoping_keeps_unrelated_clusters_apart(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let Some(container) = start_redis_or_skip().await else {
        return Ok(());
    };
    let url = redis_url(&container).await?;
    let orders_channel = unique_channel("orders");
    let payments_channel = unique_channel("payments");
    let publisher = connect_transport(&url, &orders_channel, "publisher").await?;
    let mut payments = connect_transport(&url, &payments_channel, "payments").await?;
    sleep(Duration::from_millis(100)).await;

    publisher
        .publish(&frame("remote", 42, CacheInvalidation::tag("orders")))
        .await?;
    let leaked = timeout(Duration::from_millis(250), payments.next_inbound()).await;
    assert!(leaked.is_err(), "unrelated Redis channel received a frame");

    Ok(())
}

#[tokio::test]
async fn relay_filters_foreign_cluster_frames_from_redis(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let Some(container) = start_redis_or_skip().await else {
        return Ok(());
    };
    let url = redis_url(&container).await?;
    let channel = unique_channel("cluster-scope");
    let publisher = connect_transport(&url, &channel, "publisher").await?;
    let receiver = connect_transport(&url, &channel, "node-b").await?;
    let bus = Arc::new(InMemoryFramedInvalidationBus::new(16));
    let handle = InvalidationRelay::spawn_with_metrics(
        Arc::clone(&bus),
        receiver,
        None,
        TransportConfig::new("orders", "node-b").channel(&channel),
    );
    sleep(Duration::from_millis(100)).await;

    let foreign = CacheInvalidationFrame::new(CacheInvalidationMessage::new(
        "remote",
        CacheInvalidation::key("wrong-cluster"),
    ))
    .with_cluster_name("payments")
    .with_message_id(77);
    publisher.publish(&foreign).await?;

    let snapshot = eventually_foreign_cluster_drop(&handle).await;
    assert_eq!(snapshot.foreign_cluster_dropped_total, 1);
    assert_eq!(snapshot.applied_total, 0);
    handle.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn stalled_redis_publish_does_not_block_cache_write_path(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bad_core = TransportConfig::new("orders", "node-a")
        .channel(unique_channel("stalled"))
        .outbound_capacity(1)
        .reconnect_backoff(Duration::from_millis(10));
    let bad_config = RedisTransportConfig::new("redis://127.0.0.1:1/", bad_core.clone());
    let bad_transport = RedisInvalidationTransport::connect(bad_config).await?;
    let bus = Arc::new(InMemoryFramedInvalidationBus::new(16));
    let handle =
        InvalidationRelay::spawn_with_metrics(Arc::clone(&bus), bad_transport, None, bad_core);

    let publish_result = timeout(
        Duration::from_millis(100),
        bus.publish(CacheInvalidationMessage::new(
            "node-a",
            CacheInvalidation::key("fast-path"),
        )),
    )
    .await;
    assert!(publish_result.is_ok(), "local bus publish waited on Redis");
    publish_result??;
    handle.shutdown().await?;
    Ok(())
}

#[test]
fn future_frame_version_is_reported_as_unknown_version() {
    let frame = frame("remote", 1, CacheInvalidation::key("future"));
    let mut payload = frame.encode().expect("frame encodes").to_vec();
    payload[0] = (CACHE_INVALIDATION_FRAME_VERSION + 1) as u8;

    let error = decode_redis_payload(payload).expect_err("future frame is rejected");
    assert!(matches!(
        error,
        TransportError::UnknownFrameVersion {
            found: 2,
            max_supported: CACHE_INVALIDATION_FRAME_VERSION
        }
    ));
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
    channel: &str,
    node: &str,
) -> Result<RedisInvalidationTransport, TransportError> {
    let core = TransportConfig::new("orders", node)
        .channel(channel)
        .inbound_capacity(8)
        .outbound_capacity(8)
        .reconnect_backoff(Duration::from_millis(25));
    RedisInvalidationTransport::connect(RedisTransportConfig::new(url, core)).await
}

async fn receive_matching(
    transport: &mut RedisInvalidationTransport,
    message_id: u64,
) -> Result<CacheInvalidationFrame, Box<dyn std::error::Error + Send + Sync>> {
    let deadline = Duration::from_secs(5);
    loop {
        let frame = timeout(deadline, transport.next_inbound()).await?;
        match frame {
            Some(Ok(frame)) if frame.message_id() == Some(message_id) => return Ok(frame),
            Some(Ok(_)) => continue,
            Some(Err(error)) => return Err(Box::new(error)),
            None => return Err("Redis transport closed".into()),
        }
    }
}

async fn eventually_foreign_cluster_drop(
    handle: &hydracache::InvalidationRelayHandle,
) -> hydracache::TransportMetricsSnapshot {
    for _ in 0..20 {
        let snapshot = handle.snapshot();
        if snapshot.foreign_cluster_dropped_total > 0 {
            return snapshot;
        }
        sleep(Duration::from_millis(50)).await;
    }
    handle.snapshot()
}

async fn start_redis_or_skip() -> Option<ContainerAsync<Redis>> {
    match Redis::default().with_tag("7-alpine").start().await {
        Ok(container) => Some(container),
        Err(error) => {
            eprintln!(
                "skipping Redis testcontainers integration test because Docker is unavailable: {error}"
            );
            None
        }
    }
}

async fn redis_url(
    container: &ContainerAsync<Redis>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let host = container.get_host().await?;
    let port = container.get_host_port_ipv4(REDIS_PORT).await?;
    Ok(format!("redis://{host}:{port}/"))
}

fn unique_channel(prefix: &str) -> String {
    format!(
        "hydracache:test:{prefix}:{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after unix epoch")
            .as_nanos()
    )
}
