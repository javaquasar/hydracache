use std::time::{Duration, SystemTime};

use bytes::Bytes;
use thiserror::Error;

/// Independently gated HC/2 transport identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportKind {
    /// Generated gRPC bidirectional stream over mandatory mTLS.
    GrpcBidirectional,
    /// Generated messages over a dedicated bidirectional HTTP/2 stream.
    Http2Bidirectional,
    /// Generated messages over HydraCache-owned TLS framing.
    DedicatedTcpTls,
}

/// Negotiated HC/2 capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    Data,
    Batch,
    Subscriptions,
    Topology,
    FencedSessions,
    Diagnostics,
}

/// Stable HC/2 failure classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidRequest,
    Unauthenticated,
    Unauthorized,
    NotFound,
    Conflict,
    QuotaExceeded,
    DeadlineExceeded,
    Unavailable,
    Unsupported,
    GapRepairRequired,
    SessionLost,
    Internal,
}

/// Stable retry direction. The preview client never retries implicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryAdvice {
    Never,
    SameConnection,
    ReconnectIdempotent,
    RepairRequired,
}

/// Privacy-safe SDK error.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("HC/2 {code:?}: {safe_detail}")]
pub struct ClientError {
    pub(crate) code: ErrorCode,
    pub(crate) retry: RetryAdvice,
    pub(crate) safe_detail: &'static str,
}

impl ClientError {
    /// Construct a privacy-safe adapter or policy error. Details are static so
    /// payloads, credentials, and peer-controlled text cannot leak through it.
    pub const fn new(code: ErrorCode, retry: RetryAdvice, safe_detail: &'static str) -> Self {
        Self {
            code,
            retry,
            safe_detail,
        }
    }

    /// Stable error code.
    pub const fn code(&self) -> ErrorCode {
        self.code
    }

    /// Stable retry advice. No retry has already occurred.
    pub const fn retry_advice(&self) -> RetryAdvice {
        self.retry
    }

    /// Bounded detail that contains no payload or peer-supplied secret.
    pub const fn safe_detail(&self) -> &'static str {
        self.safe_detail
    }
}

/// Explicit limits for every retained client-plane owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientLimits {
    pub max_inbound_message_bytes: usize,
    pub max_outbound_frames: usize,
    pub max_pending_invocations: usize,
    pub max_subscriptions: usize,
    pub max_subscription_events: usize,
    pub max_sessions: usize,
    pub max_topology_nodes: usize,
    pub max_batch_items: usize,
}

impl Default for ClientLimits {
    fn default() -> Self {
        Self {
            max_inbound_message_bytes: 4 * 1024 * 1024,
            max_outbound_frames: 1024,
            max_pending_invocations: 4096,
            max_subscriptions: 1024,
            max_subscription_events: 1024,
            max_sessions: 1024,
            max_topology_nodes: 256,
            max_batch_items: 1024,
        }
    }
}

impl ClientLimits {
    pub(crate) fn validate(self) -> Result<Self, ClientError> {
        let bounded = [
            (self.max_inbound_message_bytes, 1024, 16 * 1024 * 1024),
            (self.max_outbound_frames, 1, 65_536),
            (self.max_pending_invocations, 1, 1_000_000),
            (self.max_subscriptions, 1, 65_536),
            (self.max_subscription_events, 1, 65_536),
            (self.max_sessions, 1, 65_536),
            (self.max_topology_nodes, 1, 4096),
            (self.max_batch_items, 1, 4096),
        ];
        if bounded
            .into_iter()
            .any(|(value, minimum, maximum)| value < minimum || value > maximum)
        {
            return Err(ClientError::new(
                ErrorCode::InvalidRequest,
                RetryAdvice::Never,
                "client limit is outside its supported range",
            ));
        }
        Ok(self)
    }
}

/// Immutable client identity and runtime policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientConfig {
    pub client_id: String,
    pub tenant: String,
    /// Exact HC/2 generation to request. Defaults to the current preferred generation.
    pub protocol_generation: u32,
    pub connection_generation: u64,
    pub connect_timeout: Duration,
    pub default_request_timeout: Duration,
    pub capabilities: Vec<Capability>,
    pub limits: ClientLimits,
}

impl ClientConfig {
    /// Create the preview configuration with every current capability required.
    pub fn new(client_id: impl Into<String>, tenant: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            tenant: tenant.into(),
            protocol_generation: crate::HC2_GENERATION,
            connection_generation: 1,
            connect_timeout: Duration::from_secs(10),
            default_request_timeout: Duration::from_secs(5),
            capabilities: vec![
                Capability::Data,
                Capability::Batch,
                Capability::Subscriptions,
                Capability::Topology,
                Capability::FencedSessions,
                Capability::Diagnostics,
            ],
            limits: ClientLimits::default(),
        }
    }

    pub(crate) fn validate(mut self) -> Result<Self, ClientError> {
        if !valid_text(&self.client_id, 128) || !valid_text(&self.tenant, 128) {
            return Err(invalid(
                "client identity is blank, oversized, or contains controls",
            ));
        }
        if self.connection_generation == 0 {
            return Err(invalid("connection generation must be nonzero"));
        }
        if !crate::is_supported_hc2_generation(self.protocol_generation) {
            return Err(invalid("unsupported HC/2 protocol generation"));
        }
        if !valid_duration(self.connect_timeout) || !valid_duration(self.default_request_timeout) {
            return Err(invalid(
                "client timeout must be positive and at most five minutes",
            ));
        }
        self.capabilities.sort_unstable();
        self.capabilities.dedup();
        if self.capabilities.is_empty() {
            return Err(invalid("at least one capability is required"));
        }
        self.limits = self.limits.validate()?;
        Ok(self)
    }
}

/// Per-invocation deadline and idempotency policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestOptions {
    pub timeout: Duration,
    pub idempotency_key: Bytes,
}

impl RequestOptions {
    /// Create options with a bounded timeout and no idempotency key.
    pub fn with_timeout(timeout: Duration) -> Result<Self, ClientError> {
        let options = Self {
            timeout,
            idempotency_key: Bytes::new(),
        };
        options.validate()?;
        Ok(options)
    }

    pub(crate) fn validate(&self) -> Result<(), ClientError> {
        if !valid_duration(self.timeout) {
            return Err(invalid(
                "request timeout must be positive and at most five minutes",
            ));
        }
        if self.idempotency_key.len() > 128 {
            return Err(invalid("idempotency key exceeds 128 bytes"));
        }
        Ok(())
    }
}

/// Immutable value result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheValue {
    pub value: Bytes,
    pub expires_at: Option<SystemTime>,
}

/// Mutation result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MutationResult {
    pub applied: bool,
}

/// Typed batch operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchOperation {
    Get {
        key: Bytes,
    },
    Put {
        key: Bytes,
        value: Bytes,
        ttl: Option<Duration>,
    },
    Delete {
        key: Bytes,
    },
    CompareAndSet {
        key: Bytes,
        expected: Bytes,
        replacement: Bytes,
        ttl: Option<Duration>,
    },
}

/// One ordered batch result. Both values are absent for a successful GET miss.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchItemResult {
    pub item_id: u32,
    pub value: Option<CacheValue>,
    pub mutation: Option<MutationResult>,
}

/// Immutable topology endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeEndpoint {
    pub node_id: String,
    pub node_epoch: u64,
    pub endpoint_uri: String,
    pub server_name: String,
}

/// Latest monotonic topology snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TopologySnapshot {
    pub epoch: u64,
    pub nodes: Vec<NodeEndpoint>,
}

/// Subscription event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheEvent {
    pub subscription_id: u64,
    pub watermark: u64,
    pub key: Bytes,
    pub value: Bytes,
    pub removed: bool,
}

/// Ordered event or explicit repair boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionEvent {
    Event(CacheEvent),
    Gap {
        subscription_id: u64,
        after_watermark: u64,
    },
    Closed(ClientError),
}

/// Privacy-safe, pull-based runtime counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClientMetricsSnapshot {
    pub submitted: u64,
    pub completed: u64,
    pub failed: u64,
    pub cancelled: u64,
    pub late_responses: u64,
    pub events: u64,
    pub dropped_events: u64,
    pub pending_invocations: usize,
    pub active_subscriptions: usize,
    pub active_sessions: usize,
}

pub(crate) fn validate_key(key: &Bytes) -> Result<(), ClientError> {
    if key.is_empty() || key.len() > 1024 * 1024 {
        return Err(invalid("key length must be in [1, 1048576]"));
    }
    Ok(())
}

pub(crate) fn validate_value(value: &Bytes) -> Result<(), ClientError> {
    if value.len() > 16 * 1024 * 1024 {
        return Err(invalid("value exceeds 16777216 bytes"));
    }
    Ok(())
}

pub(crate) fn ttl_millis(ttl: Option<Duration>) -> Result<u64, ClientError> {
    let Some(ttl) = ttl else { return Ok(0) };
    if ttl > Duration::from_secs(3650 * 24 * 60 * 60) {
        return Err(invalid("TTL exceeds ten years"));
    }
    u64::try_from(ttl.as_millis()).map_err(|_| invalid("TTL cannot be represented in milliseconds"))
}

fn valid_duration(value: Duration) -> bool {
    !value.is_zero() && value <= Duration::from_secs(300)
}

fn valid_text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}

pub(crate) const fn invalid(detail: &'static str) -> ClientError {
    ClientError::new(ErrorCode::InvalidRequest, RetryAdvice::Never, detail)
}
