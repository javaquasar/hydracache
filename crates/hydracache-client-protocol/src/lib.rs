//! Stable external client protocol primitives.
//!
//! Release 0.49 starts the external-consumer surface by reserving a small,
//! deterministic frame contract and golden fixtures. W1 expands the payload
//! schema; W0 keeps the compatibility substrate intentionally narrow.

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// First supported external client protocol version.
pub const PROTOCOL_VERSION: u16 = 1;

/// Bytes used by the unsigned length prefix.
pub const LENGTH_PREFIX_BYTES: usize = 4;

/// Bytes used by the protocol-version field inside the frame body.
pub const VERSION_BYTES: usize = 2;

/// Smallest complete frame: length prefix plus version.
pub const MIN_FRAME_BYTES: usize = LENGTH_PREFIX_BYTES + VERSION_BYTES;

/// A length-prefixed external client frame.
///
/// The wire shape is:
///
/// ```text
/// u32 body_len_be | u16 protocol_version_be | payload bytes
/// ```
///
/// `body_len` includes the version field and the payload. Unknown future
/// protocol versions are rejected loud, matching RULES R-3/R-4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientFrame {
    protocol_version: u16,
    payload: Bytes,
}

impl ClientFrame {
    /// Build a v1 frame.
    pub fn new(payload: impl Into<Bytes>) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            payload: payload.into(),
        }
    }

    /// Encode a typed wire message as this frame payload.
    pub fn from_message(message: &ClientWireMessage) -> Result<Self, ClientProtocolError> {
        let payload = postcard::to_allocvec(message)
            .map_err(|error| ClientProtocolError::Codec(error.to_string()))?;
        Ok(Self::new(payload))
    }

    /// Build a frame with an explicit protocol version for compatibility tests.
    pub fn with_version(protocol_version: u16, payload: impl Into<Bytes>) -> Self {
        Self {
            protocol_version,
            payload: payload.into(),
        }
    }

    /// Return the frame protocol version.
    pub fn protocol_version(&self) -> u16 {
        self.protocol_version
    }

    /// Return the opaque payload bytes.
    pub fn payload(&self) -> &Bytes {
        &self.payload
    }

    /// Encode the frame with a big-endian length prefix.
    pub fn encode(&self) -> Result<Bytes, ClientProtocolError> {
        let body_len = VERSION_BYTES.checked_add(self.payload.len()).ok_or(
            ClientProtocolError::FrameTooLarge {
                actual: usize::MAX,
                max: u32::MAX as usize,
            },
        )?;
        if body_len > u32::MAX as usize {
            return Err(ClientProtocolError::FrameTooLarge {
                actual: body_len,
                max: u32::MAX as usize,
            });
        }

        let mut out = Vec::with_capacity(LENGTH_PREFIX_BYTES + body_len);
        out.extend_from_slice(&(body_len as u32).to_be_bytes());
        out.extend_from_slice(&self.protocol_version.to_be_bytes());
        out.extend_from_slice(&self.payload);
        Ok(Bytes::from(out))
    }

    /// Decode the frame payload as a typed wire message.
    pub fn decode_message(&self) -> Result<ClientWireMessage, ClientProtocolError> {
        postcard::from_bytes(self.payload.as_ref())
            .map_err(|error| ClientProtocolError::Codec(error.to_string()))
    }

    /// Decode and validate a frame.
    pub fn decode(bytes: &[u8], max_frame_bytes: usize) -> Result<Self, ClientProtocolError> {
        if bytes.len() > max_frame_bytes {
            return Err(ClientProtocolError::FrameTooLarge {
                actual: bytes.len(),
                max: max_frame_bytes,
            });
        }
        if bytes.len() < MIN_FRAME_BYTES {
            return Err(ClientProtocolError::TruncatedFrame {
                actual: bytes.len(),
                needed: MIN_FRAME_BYTES,
            });
        }

        let body_len = u32::from_be_bytes(
            bytes[0..LENGTH_PREFIX_BYTES]
                .try_into()
                .expect("slice length is checked"),
        ) as usize;
        if body_len < VERSION_BYTES {
            return Err(ClientProtocolError::TruncatedFrame {
                actual: body_len,
                needed: VERSION_BYTES,
            });
        }

        let expected = LENGTH_PREFIX_BYTES + body_len;
        if expected != bytes.len() {
            return Err(ClientProtocolError::LengthMismatch {
                declared: body_len,
                actual: bytes.len().saturating_sub(LENGTH_PREFIX_BYTES),
            });
        }

        let version_start = LENGTH_PREFIX_BYTES;
        let version_end = version_start + VERSION_BYTES;
        let protocol_version = u16::from_be_bytes(
            bytes[version_start..version_end]
                .try_into()
                .expect("slice length is checked"),
        );
        if protocol_version > PROTOCOL_VERSION {
            return Err(ClientProtocolError::UnsupportedVersion {
                version: protocol_version,
                supported_max: PROTOCOL_VERSION,
            });
        }

        Ok(Self {
            protocol_version,
            payload: Bytes::copy_from_slice(&bytes[version_end..]),
        })
    }
}

/// Negotiated protocol support window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionHandshake {
    /// Lowest protocol version supported by the caller.
    pub min: u16,
    /// Highest protocol version supported by the caller.
    pub max: u16,
}

impl VersionHandshake {
    /// Create a handshake range.
    pub fn new(min: u16, max: u16) -> Self {
        Self { min, max }
    }

    /// Negotiate the highest common version.
    pub fn negotiate(self, server: VersionHandshake) -> Result<u16, ClientErrorEnvelope> {
        let min = self.min.max(server.min);
        let max = self.max.min(server.max);
        if min <= max {
            Ok(max)
        } else {
            Err(ClientErrorEnvelope::new(
                ClientErrorCode::IncompatibleVersion,
                false,
                "no common HydraCache client protocol version",
            ))
        }
    }
}

impl Default for VersionHandshake {
    fn default() -> Self {
        Self {
            min: PROTOCOL_VERSION,
            max: PROTOCOL_VERSION,
        }
    }
}

/// Namespace carried on the wire.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Namespace(String);

impl Namespace {
    /// Create a namespace.
    pub fn new(value: impl Into<String>) -> Result<Self, ClientProtocolError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ClientProtocolError::InvalidField("namespace"));
        }
        Ok(Self(value))
    }

    /// Return the namespace string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Region id carried on the wire.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RegionId(String);

impl RegionId {
    /// Create a region id.
    pub fn new(value: impl Into<String>) -> Result<Self, ClientProtocolError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ClientProtocolError::InvalidField("region"));
        }
        Ok(Self(value))
    }

    /// Return the region id string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Structured cache key made of reviewable segments.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StructuredKey {
    segments: Vec<String>,
}

impl StructuredKey {
    /// Create a structured key from segments.
    pub fn new(segments: Vec<String>) -> Result<Self, ClientProtocolError> {
        if segments.is_empty() || segments.iter().any(|segment| segment.trim().is_empty()) {
            return Err(ClientProtocolError::InvalidField("key_segments"));
        }
        Ok(Self { segments })
    }

    /// Return the key segments.
    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// Deterministic display form for local maps and diagnostics.
    pub fn stable_key(&self) -> String {
        self.segments.join(":")
    }
}

/// Remote read consistency labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadConsistency {
    /// Eventual read.
    Eventual,
    /// Strong read within the region.
    Strong,
    /// Session-aware read.
    Session,
}

/// Remote write consistency labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteConsistency {
    /// Local acknowledged write.
    Local,
    /// Quorum write.
    Quorum,
}

/// Optional context carried by every request.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientContext {
    /// Opaque session token from 0.47 causal+.
    pub session_token: Option<String>,
    /// Requested read consistency.
    pub read: Option<ReadConsistency>,
    /// Requested write consistency.
    pub write: Option<WriteConsistency>,
    /// Preferred region for routing.
    pub preferred_region: Option<RegionId>,
}

/// Watermark used by remote near-cache repair.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Watermark {
    /// B1 `last_uuid` / source generation.
    pub source_generation: u64,
    /// B1 `last_seq` / message id.
    pub message_id: u64,
}

impl Watermark {
    /// Create a watermark.
    pub const fn new(source_generation: u64, message_id: u64) -> Self {
        Self {
            source_generation,
            message_id,
        }
    }
}

/// Repair action selected for remote near-cache streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairAction {
    /// Apply normally.
    Apply,
    /// Owner/source generation changed; clear the partition.
    ClearPartition,
    /// A sequence gap was observed; repair conservatively.
    InvalidateConservatively,
}

/// Region-scoped subscription state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SubscriptionWatermarkTracker {
    last: Option<Watermark>,
}

impl SubscriptionWatermarkTracker {
    /// Apply one event watermark and return the repair action.
    pub fn on_event(&mut self, event: &InvalidationEvent) -> RepairAction {
        let next = event.watermark();
        let Some(last) = self.last else {
            self.last = Some(next);
            return RepairAction::ClearPartition;
        };

        self.last = Some(next);
        if next.source_generation != last.source_generation {
            return RepairAction::ClearPartition;
        }
        if next.message_id > last.message_id.saturating_add(1) {
            return RepairAction::InvalidateConservatively;
        }
        RepairAction::Apply
    }
}

/// Client request envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientRequestEnvelope {
    /// Stable request id.
    pub request_id: String,
    /// Negotiated protocol version.
    pub protocol_version: u16,
    /// Optional context.
    pub context: ClientContext,
    /// Deadline expressed as a logical millisecond timestamp for deterministic tests.
    pub deadline_ms: Option<u64>,
    /// Idempotency key for retry-safe writes.
    pub idempotency_key: Option<String>,
    /// Operation.
    pub request: ClientRequest,
}

impl ClientRequestEnvelope {
    /// Create an envelope for v1.
    pub fn new(request_id: impl Into<String>, request: ClientRequest) -> Self {
        Self {
            request_id: request_id.into(),
            protocol_version: PROTOCOL_VERSION,
            context: ClientContext::default(),
            deadline_ms: None,
            idempotency_key: None,
            request,
        }
    }

    /// Attach a context.
    pub fn with_context(mut self, context: ClientContext) -> Self {
        self.context = context;
        self
    }

    /// Attach a deadline.
    pub fn with_deadline_ms(mut self, deadline_ms: u64) -> Self {
        self.deadline_ms = Some(deadline_ms);
        self
    }

    /// Attach an idempotency key.
    pub fn with_idempotency_key(mut self, idempotency_key: impl Into<String>) -> Self {
        self.idempotency_key = Some(idempotency_key.into());
        self
    }

    /// Return whether the deadline is expired at a logical timestamp.
    pub fn deadline_expired(&self, now_ms: u64) -> bool {
        self.deadline_ms.is_some_and(|deadline| deadline <= now_ms)
    }
}

/// Client operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientRequest {
    /// Read one key.
    Get { ns: Namespace, key: StructuredKey },
    /// Store one value.
    Put {
        ns: Namespace,
        key: StructuredKey,
        value: Vec<u8>,
        ttl_ms: Option<u64>,
        dimensions: Vec<String>,
    },
    /// Invalidate one key.
    Invalidate { ns: Namespace, key: StructuredKey },
    /// Read many keys.
    BatchGet {
        ns: Namespace,
        keys: Vec<StructuredKey>,
    },
    /// Store many key/value pairs.
    BatchPut {
        ns: Namespace,
        entries: Vec<BatchPutEntry>,
    },
    /// Evict a whole namespace/region mapping.
    EvictRegion { ns: Namespace },
    /// Subscribe to invalidations.
    SubscribeInvalidations {
        ns: Namespace,
        region: Option<RegionId>,
        from: Option<Watermark>,
        include_value: bool,
    },
}

/// One batch put entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchPutEntry {
    /// Key to store.
    pub key: StructuredKey,
    /// Value bytes.
    pub value: Vec<u8>,
}

/// Client response envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientResponseEnvelope {
    /// Request id copied from the request.
    pub request_id: String,
    /// Protocol version used by the response.
    pub protocol_version: u16,
    /// Operation result.
    pub result: Result<ClientResponse, ClientErrorEnvelope>,
}

impl ClientResponseEnvelope {
    /// Build a successful response.
    pub fn ok(request_id: impl Into<String>, response: ClientResponse) -> Self {
        Self {
            request_id: request_id.into(),
            protocol_version: PROTOCOL_VERSION,
            result: Ok(response),
        }
    }

    /// Build an error response.
    pub fn error(request_id: impl Into<String>, error: ClientErrorEnvelope) -> Self {
        Self {
            request_id: request_id.into(),
            protocol_version: PROTOCOL_VERSION,
            result: Err(error),
        }
    }
}

/// Client responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientResponse {
    /// Optional value.
    Value { value: Option<Vec<u8>> },
    /// Put accepted.
    Stored,
    /// Invalidation accepted.
    Invalidated,
    /// Batch result in request order.
    Batch { items: Vec<BatchItemStatus> },
    /// Region/namespace eviction accepted.
    Evicted,
    /// Subscription accepted.
    Subscribed { from: Option<Watermark> },
}

/// Per-item batch status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchItemStatus {
    /// Original item index.
    pub index: usize,
    /// Per-item result.
    pub result: Result<Option<Vec<u8>>, ClientErrorEnvelope>,
}

/// Stable error envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientErrorEnvelope {
    /// Stable machine-readable error code.
    pub code: ClientErrorCode,
    /// Whether the SDK may retry.
    pub retryable: bool,
    /// Optional retry-after in milliseconds.
    pub retry_after_ms: Option<u64>,
    /// Redacted message for humans.
    pub message: String,
}

impl ClientErrorEnvelope {
    /// Create a redacted error envelope.
    pub fn new(code: ClientErrorCode, retryable: bool, message: impl Into<String>) -> Self {
        Self {
            code,
            retryable,
            retry_after_ms: None,
            message: redact_message(message.into()),
        }
    }

    /// Attach retry-after.
    pub fn with_retry_after_ms(mut self, retry_after_ms: u64) -> Self {
        self.retry_after_ms = Some(retry_after_ms);
        self
    }
}

/// Stable client error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientErrorCode {
    /// No common supported protocol version.
    IncompatibleVersion,
    /// Identity is missing.
    Unauthenticated,
    /// Identity is not allowed.
    Unauthorized,
    /// Tenant quota exceeded.
    TenantQuota,
    /// Rate limited.
    RateLimited,
    /// Residency policy denied value movement.
    ResidencyDenied,
    /// Request or value too large.
    TooLarge,
    /// Deadline expired.
    DeadlineExceeded,
    /// Optimistic conflict.
    Conflict,
    /// Backend unavailable.
    BackendUnavailable,
    /// Frame or payload is malformed.
    MalformedFrame,
}

/// Invalidation event streamed to remote near-caches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvalidationEvent {
    /// Namespace.
    pub ns: Namespace,
    /// Structured key.
    pub key: StructuredKey,
    /// B1 source generation.
    pub generation: u64,
    /// B1 message id.
    pub message_id: u64,
    /// Region where the event was applied.
    pub applied_region: Option<RegionId>,
    /// Optional value, gated by residency.
    pub value: Option<Vec<u8>>,
    /// Whether value was stripped by residency.
    pub residency_degraded: bool,
    /// Whether this event affects a subscriber's tracked cross-region view.
    pub affects_subscriber_view: bool,
}

impl InvalidationEvent {
    /// Create an invalidation event.
    pub fn new(ns: Namespace, key: StructuredKey, generation: u64, message_id: u64) -> Self {
        Self {
            ns,
            key,
            generation,
            message_id,
            applied_region: None,
            value: None,
            residency_degraded: false,
            affects_subscriber_view: false,
        }
    }

    /// Attach applied region.
    pub fn applied_in(mut self, region: RegionId) -> Self {
        self.applied_region = Some(region);
        self
    }

    /// Attach an optional value.
    pub fn with_value(mut self, value: Vec<u8>) -> Self {
        self.value = Some(value);
        self
    }

    /// Mark that a cross-region invalidation affects the subscriber's tracked view.
    pub fn affects_subscriber_view(mut self) -> Self {
        self.affects_subscriber_view = true;
        self
    }

    /// Return event watermark.
    pub fn watermark(&self) -> Watermark {
        Watermark::new(self.generation, self.message_id)
    }

    /// Return whether this event should be delivered for a region filter.
    pub fn should_deliver_to(&self, region: Option<&RegionId>) -> bool {
        match region {
            None => true,
            Some(region) => {
                self.applied_region.as_ref() == Some(region) || self.affects_subscriber_view
            }
        }
    }

    /// Enforce residency for include-value streams.
    pub fn residency_gated(mut self, value_allowed: bool) -> Self {
        if !value_allowed && self.value.is_some() {
            self.value = None;
            self.residency_degraded = true;
        }
        self
    }
}

/// Wire messages carried inside [`ClientFrame`] payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientWireMessage {
    /// Version negotiation.
    Handshake(VersionHandshake),
    /// Client request.
    Request(ClientRequestEnvelope),
    /// Server response.
    Response(ClientResponseEnvelope),
    /// Server-pushed invalidation.
    Invalidation(InvalidationEvent),
    /// Stream heartbeat.
    Heartbeat(Watermark),
}

fn redact_message(message: String) -> String {
    let mut redacted = message;
    for marker in ["value=", "secret=", "token="] {
        if let Some(index) = redacted.find(marker) {
            redacted.truncate(index + marker.len());
            redacted.push_str("<redacted>");
        }
    }
    redacted
}

/// External client protocol decode/encode errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ClientProtocolError {
    /// Frame exceeds the configured limit.
    #[error("client frame is {actual} bytes, exceeding max_frame_bytes={max}")]
    FrameTooLarge {
        /// Observed frame length.
        actual: usize,
        /// Configured limit.
        max: usize,
    },
    /// Not enough bytes were supplied to parse a complete frame.
    #[error("truncated client frame: {actual} bytes available, {needed} needed")]
    TruncatedFrame {
        /// Observed frame length.
        actual: usize,
        /// Required frame length.
        needed: usize,
    },
    /// The length prefix and supplied bytes disagree.
    #[error(
        "client frame length mismatch: declared body {declared} bytes, actual body {actual} bytes"
    )]
    LengthMismatch {
        /// Body length from the prefix.
        declared: usize,
        /// Body length present after the prefix.
        actual: usize,
    },
    /// The frame is from a future protocol version.
    #[error("unsupported client protocol version {version}; supported max is {supported_max}")]
    UnsupportedVersion {
        /// Version from the frame.
        version: u16,
        /// Highest version this reader supports.
        supported_max: u16,
    },
    /// Payload codec failed.
    #[error("client protocol codec error: {0}")]
    Codec(String),
    /// Required field is invalid.
    #[error("invalid client protocol field: {0}")]
    InvalidField(&'static str),
}
