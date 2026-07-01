#![forbid(unsafe_code)]

use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use futures_util::StreamExt;
use hydracache::{
    decode_transport_frame, CacheInvalidationFrame, InvalidationTransport, TransportConfig,
    TransportError,
};
use tokio::sync::{mpsc, Mutex};

/// NATS subject configuration for HydraCache invalidation frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NatsTransportConfig {
    /// Shared HydraCache relay configuration.
    pub core: TransportConfig,
    /// NATS connection URL.
    pub url: String,
    /// Internal subscriber queue bound used before the relay consumes frames.
    pub subscriber_capacity: usize,
}

impl NatsTransportConfig {
    /// Build NATS transport configuration from a URL and relay config.
    pub fn new(url: impl Into<String>, core: TransportConfig) -> Self {
        let subscriber_capacity = core.inbound_capacity.max(1);
        Self {
            core,
            url: url.into(),
            subscriber_capacity,
        }
    }

    /// Build config with the standard NATS subject for a HydraCache cluster.
    pub fn for_cluster(
        url: impl Into<String>,
        cluster_name: impl Into<String>,
        local_node_id: impl Into<String>,
    ) -> Self {
        let cluster_name = cluster_name.into();
        let core = TransportConfig::new(cluster_name.clone(), local_node_id)
            .channel(default_nats_subject(&cluster_name));
        Self::new(url, core)
    }

    /// Return the configured NATS subject.
    pub fn subject(&self) -> &str {
        &self.core.channel
    }

    /// Return whether the URL asks NATS to use TLS.
    pub fn uses_tls(&self) -> bool {
        self.url.starts_with("tls://") || self.url.starts_with("nats+tls://")
    }
}

/// Return the default NATS invalidation subject for a cluster.
pub fn default_nats_subject(cluster_name: &str) -> String {
    format!("hydracache.inval.{cluster_name}")
}

/// NATS implementation of [`InvalidationTransport`].
#[derive(Debug, Clone)]
pub struct NatsInvalidationTransport {
    client: async_nats::Client,
    config: Arc<NatsTransportConfig>,
    inbound:
        Arc<Mutex<mpsc::Receiver<std::result::Result<CacheInvalidationFrame, TransportError>>>>,
}

impl NatsInvalidationTransport {
    /// Connect to NATS and start the background subscription loop.
    pub async fn connect(config: NatsTransportConfig) -> std::result::Result<Self, TransportError> {
        let client = async_nats::connect(config.url.clone())
            .await
            .map_err(|error| nats_backend_error("connect", error))?;
        let subscriber = client
            .subscribe(config.subject().to_owned())
            .await
            .map_err(|error| nats_backend_error("subscribe", error))?;
        let (sender, receiver) = mpsc::channel(config.subscriber_capacity.max(1));
        let config = Arc::new(config);
        tokio::spawn(run_subscriber(subscriber, sender));
        Ok(Self {
            client,
            config,
            inbound: Arc::new(Mutex::new(receiver)),
        })
    }

    /// Publish already encoded frame bytes to NATS.
    pub async fn publish_encoded(&self, payload: Bytes) -> std::result::Result<(), TransportError> {
        self.client
            .publish(self.config.subject().to_owned(), payload)
            .await
            .map_err(|error| nats_backend_error("publish", error))?;
        self.client
            .flush()
            .await
            .map_err(|error| nats_backend_error("flush", error))
    }
}

#[async_trait]
impl InvalidationTransport for NatsInvalidationTransport {
    async fn publish(
        &self,
        frame: &CacheInvalidationFrame,
    ) -> std::result::Result<(), TransportError> {
        let encoded = frame
            .encode()
            .map_err(|error| TransportError::Decode(error.to_string()))?;
        self.publish_encoded(encoded).await
    }

    async fn next_inbound(
        &mut self,
    ) -> Option<std::result::Result<CacheInvalidationFrame, TransportError>> {
        self.inbound.lock().await.recv().await
    }
}

/// Decode one NATS payload into a HydraCache invalidation frame.
pub fn decode_nats_payload(
    payload: Bytes,
) -> std::result::Result<CacheInvalidationFrame, TransportError> {
    decode_transport_frame(&payload)
}

async fn run_subscriber(
    mut subscriber: async_nats::Subscriber,
    sender: mpsc::Sender<std::result::Result<CacheInvalidationFrame, TransportError>>,
) {
    while let Some(message) = subscriber.next().await {
        if sender
            .send(decode_nats_payload(message.payload))
            .await
            .is_err()
        {
            return;
        }
    }
    let _ = sender
        .send(Err(TransportError::Backend(
            "nats subscriber stream ended".to_owned(),
        )))
        .await;
}

fn nats_backend_error(action: &str, error: impl std::fmt::Display) -> TransportError {
    TransportError::Backend(format!("nats {action}: {error}"))
}
