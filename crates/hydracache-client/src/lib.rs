//! Remote HydraCache client SDK over the W1 external client protocol.
//!
//! The SDK intentionally depends on `hydracache-client-protocol` rather than
//! internal member transport APIs. It is the reference implementation used by
//! the language-agnostic conformance manifest.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use hydracache_client_protocol::{
    ClientContext, ClientErrorCode, ClientErrorEnvelope, ClientFrame, ClientProtocolError,
    ClientRequest, ClientRequestEnvelope, ClientResponse, ClientResponseEnvelope,
    ClientWireMessage, Namespace, RepairAction, StructuredKey, SubscriptionWatermarkTracker,
    VersionHandshake, PROTOCOL_VERSION,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Stable retryability mapping for SDK-facing errors.
pub fn stable_error_retryable(code: ClientErrorCode) -> bool {
    matches!(
        code,
        ClientErrorCode::TenantQuota
            | ClientErrorCode::RateLimited
            | ClientErrorCode::DeadlineExceeded
            | ClientErrorCode::BackendUnavailable
    )
}

/// Client identity supplied to the public client surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientIdentity {
    client_id: String,
    tenant: String,
}

impl ClientIdentity {
    /// Create a client identity.
    pub fn new(
        client_id: impl Into<String>,
        tenant: impl Into<String>,
    ) -> Result<Self, ClientError> {
        let client_id = client_id.into();
        let tenant = tenant.into();
        if client_id.trim().is_empty() {
            return Err(ClientError::InvalidConfig("client_id"));
        }
        if tenant.trim().is_empty() {
            return Err(ClientError::InvalidConfig("tenant"));
        }
        Ok(Self { client_id, tenant })
    }

    /// Client id.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Tenant id.
    pub fn tenant(&self) -> &str {
        &self.tenant
    }
}

/// Retry policy for retryable W1 errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum attempts, including the first attempt.
    pub max_attempts: usize,
    /// Deterministic backoff between attempts.
    pub backoff_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 2,
            backoff_ms: 0,
        }
    }
}

/// Remote client configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HydraClientConfig {
    /// Client identity.
    pub identity: ClientIdentity,
    /// Supported client protocol range.
    pub supported_versions: VersionHandshake,
    /// Maximum response frame size.
    pub max_frame_bytes: usize,
    /// Retry policy.
    pub retry: RetryPolicy,
}

impl HydraClientConfig {
    /// Build a config for protocol v1.
    pub fn new(identity: ClientIdentity) -> Self {
        Self {
            identity,
            supported_versions: VersionHandshake::default(),
            max_frame_bytes: 1024 * 1024,
            retry: RetryPolicy::default(),
        }
    }

    /// Override retry policy.
    pub fn with_retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }
}

/// Transport abstraction used by the SDK.
#[async_trait]
pub trait ClientTransport: Send + Sync {
    /// Send one encoded W1 frame and return the encoded W1 response frame.
    async fn send_frame(
        &self,
        identity: &ClientIdentity,
        frame: Bytes,
    ) -> Result<Bytes, ClientError>;
}

/// HTTP transport over `/client/v1/data`.
#[derive(Debug, Clone)]
pub struct HttpClientTransport {
    data_url: String,
    http: reqwest::Client,
}

impl HttpClientTransport {
    /// Create a transport from a server base URL.
    pub fn new(base_url: impl Into<String>) -> Self {
        let base_url = base_url.into();
        let data_url = format!("{}/client/v1/data", base_url.trim_end_matches('/'));
        Self {
            data_url,
            http: reqwest::Client::new(),
        }
    }

    /// Return the data route URL.
    pub fn data_url(&self) -> &str {
        &self.data_url
    }
}

#[async_trait]
impl ClientTransport for HttpClientTransport {
    async fn send_frame(
        &self,
        identity: &ClientIdentity,
        frame: Bytes,
    ) -> Result<Bytes, ClientError> {
        let response = self
            .http
            .post(&self.data_url)
            .header("x-hydracache-client-id", identity.client_id())
            .header("x-hydracache-tenant", identity.tenant())
            .body(frame)
            .send()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ClientError::Transport(format!(
                "client surface returned HTTP {status}"
            )));
        }

        response
            .bytes()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))
    }
}

/// Request options.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestOptions {
    /// Request context.
    pub context: ClientContext,
    /// Logical deadline in milliseconds; `0` is expired in deterministic tests.
    pub deadline_ms: Option<u64>,
    /// Optional idempotency key.
    pub idempotency_key: Option<String>,
}

impl RequestOptions {
    /// Attach an idempotency key.
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Attach a deterministic deadline.
    pub fn with_deadline_ms(mut self, deadline_ms: u64) -> Self {
        self.deadline_ms = Some(deadline_ms);
        self
    }
}

/// Remote HydraCache client.
#[derive(Debug)]
pub struct HydraClient<T> {
    transport: T,
    config: HydraClientConfig,
    negotiated_version: u16,
    request_seq: AtomicU64,
    metrics: Arc<ClientMetrics>,
}

impl<T> HydraClient<T>
where
    T: ClientTransport,
{
    /// Connect and negotiate the highest common protocol version.
    pub async fn connect(transport: T, config: HydraClientConfig) -> Result<Self, ClientError> {
        let frame =
            ClientFrame::from_message(&ClientWireMessage::Handshake(config.supported_versions))?
                .encode()?;
        let response = transport.send_frame(&config.identity, frame).await?;
        let response = decode_message(&response, config.max_frame_bytes)?;
        let ClientWireMessage::Handshake(server) = response else {
            return Err(ClientError::UnexpectedMessage("handshake"));
        };
        let negotiated_version = config.supported_versions.negotiate(server)?;

        let client = Self {
            transport,
            config,
            negotiated_version,
            request_seq: AtomicU64::new(1),
            metrics: Arc::new(ClientMetrics::default()),
        };
        client
            .metrics
            .sessions_active
            .fetch_add(1, Ordering::SeqCst);
        Ok(client)
    }

    /// Negotiated protocol version.
    pub fn negotiated_version(&self) -> u16 {
        self.negotiated_version
    }

    /// Snapshot client metrics.
    pub fn metrics(&self) -> ClientMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Build a near-cache repair helper sharing this client's metrics.
    pub fn near_cache(&self) -> RemoteNearCache {
        RemoteNearCache::with_metrics(Arc::clone(&self.metrics))
    }

    /// Read a value.
    pub async fn get(
        &self,
        ns: Namespace,
        key: StructuredKey,
    ) -> Result<Option<Bytes>, ClientError> {
        let response = self
            .request(ClientRequest::Get { ns, key }, RequestOptions::default())
            .await?;
        let ClientResponse::Value { value } = response else {
            return Err(ClientError::UnexpectedResponse("value"));
        };
        Ok(value.map(Bytes::from))
    }

    /// Store a value.
    pub async fn put(
        &self,
        ns: Namespace,
        key: StructuredKey,
        value: Bytes,
        ttl: Option<Duration>,
    ) -> Result<(), ClientError> {
        let ttl_ms = ttl.map(duration_millis_saturating);
        let response = self
            .request(
                ClientRequest::Put {
                    ns,
                    key,
                    value: value.to_vec(),
                    ttl_ms,
                    dimensions: Vec::new(),
                },
                RequestOptions::default(),
            )
            .await?;
        match response {
            ClientResponse::Stored => Ok(()),
            _ => Err(ClientError::UnexpectedResponse("stored")),
        }
    }

    /// Invalidate a value.
    pub async fn invalidate(&self, ns: Namespace, key: StructuredKey) -> Result<(), ClientError> {
        let response = self
            .request(
                ClientRequest::Invalidate { ns, key },
                RequestOptions::default(),
            )
            .await?;
        match response {
            ClientResponse::Invalidated => Ok(()),
            _ => Err(ClientError::UnexpectedResponse("invalidated")),
        }
    }

    /// Send a raw protocol request with explicit options.
    pub async fn request(
        &self,
        request: ClientRequest,
        options: RequestOptions,
    ) -> Result<ClientResponse, ClientError> {
        let attempts = self.config.retry.max_attempts.max(1);
        let mut last_error = None;

        for attempt in 1..=attempts {
            match self.request_once(request.clone(), options.clone()).await {
                Ok(response) => return Ok(response),
                Err(ClientError::Server(error))
                    if error.retryable || stable_error_retryable(error.code) =>
                {
                    last_error = Some(ClientError::Server(error));
                    if attempt < attempts && self.config.retry.backoff_ms > 0 {
                        tokio::time::sleep(Duration::from_millis(self.config.retry.backoff_ms))
                            .await;
                    }
                }
                Err(error) => return Err(error),
            }
        }

        Err(last_error.expect("attempts is non-zero"))
    }

    async fn request_once(
        &self,
        request: ClientRequest,
        options: RequestOptions,
    ) -> Result<ClientResponse, ClientError> {
        let request_id = self.next_request_id();
        let mut envelope =
            ClientRequestEnvelope::new(request_id, request).with_context(options.context);
        if let Some(deadline_ms) = options.deadline_ms {
            envelope = envelope.with_deadline_ms(deadline_ms);
        }
        if let Some(key) = options.idempotency_key {
            envelope = envelope.with_idempotency_key(key);
        }
        envelope.protocol_version = self.negotiated_version;

        let message = ClientWireMessage::Request(envelope);
        let frame = ClientFrame::from_message(&message)?.encode()?;
        let response = self
            .transport
            .send_frame(&self.config.identity, frame)
            .await?;
        let response = decode_message(&response, self.config.max_frame_bytes)?;
        let ClientWireMessage::Response(envelope) = response else {
            return Err(ClientError::UnexpectedMessage("response"));
        };
        unpack_response(envelope)
    }

    fn next_request_id(&self) -> String {
        let seq = self.request_seq.fetch_add(1, Ordering::SeqCst);
        format!("{}-{seq}", self.config.identity.client_id())
    }
}

impl<T> Drop for HydraClient<T> {
    fn drop(&mut self) {
        self.metrics.sessions_active.fetch_sub(1, Ordering::SeqCst);
    }
}

fn unpack_response(envelope: ClientResponseEnvelope) -> Result<ClientResponse, ClientError> {
    if envelope.protocol_version != PROTOCOL_VERSION {
        return Err(ClientError::Server(ClientErrorEnvelope::new(
            ClientErrorCode::IncompatibleVersion,
            false,
            "response protocol version mismatch",
        )));
    }
    envelope.result.map_err(ClientError::Server)
}

fn decode_message(bytes: &[u8], max_frame_bytes: usize) -> Result<ClientWireMessage, ClientError> {
    Ok(ClientFrame::decode(bytes, max_frame_bytes)?.decode_message()?)
}

fn duration_millis_saturating(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

/// Remote near-cache repair helper.
#[derive(Debug, Default)]
pub struct RemoteNearCache {
    tracker: SubscriptionWatermarkTracker,
    metrics: Arc<ClientMetrics>,
}

impl RemoteNearCache {
    /// Build a near-cache helper with shared client metrics.
    pub fn with_metrics(metrics: Arc<ClientMetrics>) -> Self {
        Self {
            tracker: SubscriptionWatermarkTracker::default(),
            metrics,
        }
    }

    /// Apply an invalidation watermark.
    pub fn on_watermark(&mut self, generation: u64, message_id: u64) -> RepairAction {
        let event = hydracache_client_protocol::InvalidationEvent::new(
            Namespace::new("near-cache").expect("static namespace"),
            StructuredKey::new(vec!["watermark".to_owned()]).expect("static key"),
            generation,
            message_id,
        );
        self.on_event(&event)
    }

    /// Apply an invalidation event.
    pub fn on_event(
        &mut self,
        event: &hydracache_client_protocol::InvalidationEvent,
    ) -> RepairAction {
        let action = self.tracker.on_event(event);
        if action != RepairAction::Apply {
            self.metrics
                .near_cache_repairs_total
                .fetch_add(1, Ordering::SeqCst);
        }
        action
    }
}

/// SDK metrics counters.
#[derive(Debug, Default)]
pub struct ClientMetrics {
    sessions_active: AtomicU64,
    near_cache_repairs_total: AtomicU64,
}

impl ClientMetrics {
    /// Snapshot metrics.
    pub fn snapshot(&self) -> ClientMetricsSnapshot {
        ClientMetricsSnapshot {
            client_sessions_active: self.sessions_active.load(Ordering::SeqCst),
            client_near_cache_repairs_total: self.near_cache_repairs_total.load(Ordering::SeqCst),
        }
    }
}

/// Bounded SDK metrics exported by W3.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientMetricsSnapshot {
    /// Active SDK sessions.
    pub client_sessions_active: u64,
    /// Conservative near-cache repairs.
    pub client_near_cache_repairs_total: u64,
}

/// Language-agnostic conformance manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConformanceManifest {
    /// Manifest version.
    pub manifest_version: u16,
    /// Protocol version under test.
    pub protocol_version: u16,
    /// Supported SDK matrix.
    pub sdks: Vec<SdkSupport>,
    /// Scenarios every supported SDK must pass.
    pub scenarios: Vec<ConformanceScenario>,
    /// Stable error mapping.
    pub errors: Vec<ErrorRetryability>,
}

/// Supported SDK entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdkSupport {
    /// SDK language.
    pub language: String,
    /// Package name.
    pub package: String,
    /// Whether this SDK is supported on green conformance.
    pub supported: bool,
}

/// One conformance scenario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConformanceScenario {
    /// Stable scenario id.
    pub id: String,
    /// Operation group.
    pub kind: String,
    /// Human-readable behavior.
    pub behavior: String,
    /// Required protocol features.
    pub requires: Vec<String>,
}

/// Stable error retryability entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorRetryability {
    /// Protocol error code.
    pub code: ClientErrorCode,
    /// SDK-facing retryability.
    pub retryable: bool,
}

/// SDK errors.
#[derive(Debug, Error)]
pub enum ClientError {
    /// Invalid client configuration.
    #[error("invalid HydraCache client config field: {0}")]
    InvalidConfig(&'static str),
    /// Transport failed.
    #[error("HydraCache client transport error: {0}")]
    Transport(String),
    /// Protocol encode/decode failed.
    #[error(transparent)]
    Protocol(#[from] ClientProtocolError),
    /// Server returned a stable error envelope.
    #[error("HydraCache server error: {0:?}")]
    Server(ClientErrorEnvelope),
    /// Unexpected wire message.
    #[error("expected HydraCache client {0} message")]
    UnexpectedMessage(&'static str),
    /// Unexpected operation response.
    #[error("expected HydraCache client {0} response")]
    UnexpectedResponse(&'static str),
}

impl From<ClientErrorEnvelope> for ClientError {
    fn from(value: ClientErrorEnvelope) -> Self {
        Self::Server(value)
    }
}
