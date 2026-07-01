#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use futures_util::StreamExt;
use hydracache::{
    decode_transport_frame, CacheInvalidationFrame, InvalidationTransport, TransportConfig,
    TransportError,
};
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;

/// Redis pub/sub configuration for HydraCache invalidation frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisTransportConfig {
    /// Shared HydraCache relay configuration.
    pub core: TransportConfig,
    /// Redis connection URL. Authentication and TLS are configured by the URL.
    pub url: String,
    /// Internal subscriber queue bound used before the relay consumes frames.
    pub subscriber_capacity: usize,
}

impl RedisTransportConfig {
    /// Build Redis transport configuration from a Redis URL and relay config.
    pub fn new(url: impl Into<String>, core: TransportConfig) -> Self {
        let subscriber_capacity = core.inbound_capacity.max(1);
        Self {
            core,
            url: url.into(),
            subscriber_capacity,
        }
    }

    /// Return the configured Redis pub/sub channel.
    pub fn channel(&self) -> &str {
        &self.core.channel
    }

    /// Return whether the URL asks Redis to use TLS.
    pub fn uses_tls(&self) -> bool {
        self.url.starts_with("rediss://")
    }
}

/// Redis pub/sub implementation of [`InvalidationTransport`].
#[derive(Debug, Clone)]
pub struct RedisInvalidationTransport {
    client: redis::Client,
    config: Arc<RedisTransportConfig>,
    inbound:
        Arc<Mutex<mpsc::Receiver<std::result::Result<CacheInvalidationFrame, TransportError>>>>,
}

impl RedisInvalidationTransport {
    /// Connect to Redis and start the background subscription loop.
    pub async fn connect(
        config: RedisTransportConfig,
    ) -> std::result::Result<Self, TransportError> {
        let client = redis::Client::open(config.url.as_str())
            .map_err(|error| redis_backend_error("open client", error))?;
        let (sender, receiver) = mpsc::channel(config.subscriber_capacity.max(1));
        let config = Arc::new(config);
        tokio::spawn(run_subscriber(client.clone(), Arc::clone(&config), sender));
        Ok(Self {
            client,
            config,
            inbound: Arc::new(Mutex::new(receiver)),
        })
    }

    /// Publish already encoded frame bytes to Redis.
    ///
    /// This is mostly useful for diagnostics and tests that verify malformed or
    /// future-version payload handling.
    pub async fn publish_encoded(
        &self,
        payload: Bytes,
    ) -> std::result::Result<u64, TransportError> {
        let mut connection = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| redis_backend_error("connect publisher", error))?;
        redis::cmd("PUBLISH")
            .arg(self.config.channel())
            .arg(payload.as_ref())
            .query_async::<u64>(&mut connection)
            .await
            .map_err(|error| redis_backend_error("publish", error))
    }
}

#[async_trait]
impl InvalidationTransport for RedisInvalidationTransport {
    async fn publish(
        &self,
        frame: &CacheInvalidationFrame,
    ) -> std::result::Result<(), TransportError> {
        let encoded = frame
            .encode()
            .map_err(|error| TransportError::Decode(error.to_string()))?;
        self.publish_encoded(encoded).await.map(|_| ())
    }

    async fn next_inbound(
        &mut self,
    ) -> Option<std::result::Result<CacheInvalidationFrame, TransportError>> {
        self.inbound.lock().await.recv().await
    }
}

/// Decode one Redis payload into a HydraCache invalidation frame.
pub fn decode_redis_payload(
    payload: Vec<u8>,
) -> std::result::Result<CacheInvalidationFrame, TransportError> {
    decode_transport_frame(&Bytes::from(payload))
}

async fn run_subscriber(
    client: redis::Client,
    config: Arc<RedisTransportConfig>,
    sender: mpsc::Sender<std::result::Result<CacheInvalidationFrame, TransportError>>,
) {
    let backoff = Duration::from_millis(config.core.reconnect_backoff_ms.max(1));
    while !sender.is_closed() {
        if let Err(error) = subscribe_once(&client, &config, &sender).await {
            if sender.send(Err(error)).await.is_err() {
                break;
            }
            sleep(backoff).await;
        }
    }
}

async fn subscribe_once(
    client: &redis::Client,
    config: &RedisTransportConfig,
    sender: &mpsc::Sender<std::result::Result<CacheInvalidationFrame, TransportError>>,
) -> std::result::Result<(), TransportError> {
    let mut pubsub = client
        .get_async_pubsub()
        .await
        .map_err(|error| redis_backend_error("connect subscriber", error))?;
    pubsub
        .subscribe(config.channel())
        .await
        .map_err(|error| redis_backend_error("subscribe", error))?;
    let mut messages = pubsub.on_message();
    while let Some(message) = messages.next().await {
        let payload = match message.get_payload::<Vec<u8>>() {
            Ok(payload) => payload,
            Err(error) => {
                if sender
                    .send(Err(redis_backend_error("read payload", error)))
                    .await
                    .is_err()
                {
                    return Ok(());
                }
                continue;
            }
        };
        if sender.send(decode_redis_payload(payload)).await.is_err() {
            return Ok(());
        }
    }
    Err(TransportError::Backend(
        "redis pub/sub stream ended".to_owned(),
    ))
}

fn redis_backend_error(action: &str, error: redis::RedisError) -> TransportError {
    TransportError::Backend(format!("redis {action}: {error}"))
}
