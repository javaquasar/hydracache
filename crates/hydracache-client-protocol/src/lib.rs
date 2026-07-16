//! Stable external client protocol primitives.
//!
//! Release 0.49 starts the external-consumer surface by reserving a small,
//! deterministic frame contract and golden fixtures. W1 expands the payload
//! schema; W0 keeps the compatibility substrate intentionally narrow.

use bytes::Bytes;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod hibernate;
pub mod java_migration;

/// First supported external client protocol version.
pub const MIN_PROTOCOL_VERSION: u16 = 1;

/// Highest supported external client protocol version.
pub const PROTOCOL_VERSION: u16 = 4;

/// First protocol version that carries the IMap/Fenced Lock operation family.
pub const LOCK_PROTOCOL_VERSION: u16 = 2;

/// First protocol version that carries TTL metadata and explicit expiry operations.
pub const TTL_PROTOCOL_VERSION: u16 = 3;

/// First protocol version that carries Redis-lock conditional value operations.
pub const REDIS_LOCK_PROTOCOL_VERSION: u16 = 4;

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
    /// Build a frame at the highest supported protocol version.
    pub fn new(payload: impl Into<Bytes>) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            payload: payload.into(),
        }
    }

    /// Encode a typed wire message as this frame payload.
    pub fn from_message(message: &ClientWireMessage) -> Result<Self, ClientProtocolError> {
        Self::from_message_with_version(message.protocol_version(), message)
    }

    /// Encode a typed wire message as this frame payload at an explicit version.
    pub fn from_message_with_version(
        protocol_version: u16,
        message: &ClientWireMessage,
    ) -> Result<Self, ClientProtocolError> {
        ensure_frame_version_supported(protocol_version)?;
        ensure_message_version_matches_frame(protocol_version, message)?;
        ensure_message_supported_by_frame_version(protocol_version, message)?;
        let payload = encode_wire_message_for_version(protocol_version, message)?;
        Ok(Self::with_version(protocol_version, payload))
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
        let message =
            decode_wire_message_for_version(self.protocol_version, self.payload.as_ref())?;
        ensure_message_version_matches_frame(self.protocol_version, &message)?;
        ensure_message_supported_by_frame_version(self.protocol_version, &message)?;
        Ok(message)
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
        if !(MIN_PROTOCOL_VERSION..=PROTOCOL_VERSION).contains(&protocol_version) {
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
            min: MIN_PROTOCOL_VERSION,
            max: PROTOCOL_VERSION,
        }
    }
}

/// Return whether a request/response envelope version is in the supported window.
pub fn protocol_version_supported(protocol_version: u16) -> bool {
    (MIN_PROTOCOL_VERSION..=PROTOCOL_VERSION).contains(&protocol_version)
}

/// Reject an unsupported protocol version with the stable wire error.
pub fn ensure_supported_protocol_version(protocol_version: u16) -> Result<(), ClientErrorEnvelope> {
    if protocol_version_supported(protocol_version) {
        Ok(())
    } else {
        Err(ClientErrorEnvelope::new(
            ClientErrorCode::IncompatibleVersion,
            false,
            format!(
                "unsupported HydraCache client protocol version {protocol_version}; supported range is {MIN_PROTOCOL_VERSION}..={PROTOCOL_VERSION}"
            ),
        ))
    }
}

/// Reject an operation whose minimum version is newer than the negotiated version.
pub fn require_protocol_version(
    protocol_version: u16,
    required_min: u16,
    operation: &'static str,
) -> Result<(), ClientErrorEnvelope> {
    ensure_supported_protocol_version(protocol_version)?;
    if protocol_version >= required_min {
        Ok(())
    } else {
        Err(ClientErrorEnvelope::new(
            ClientErrorCode::IncompatibleVersion,
            false,
            format!(
                "{operation} requires HydraCache client protocol version {required_min} or newer"
            ),
        ))
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

/// Linearizable-capable consistency labels for lock/CAS operations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockConsistency {
    /// A single replica; rejected for lock/CAS operations.
    One,
    /// Quorum-applied command.
    #[default]
    Quorum,
    /// Each quorum-applied command.
    EachQuorum,
    /// All replicas applied the command.
    All,
}

/// Expected value shape for single-key compare-and-set operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CasExpectation {
    /// Match one exact current value.
    Exact(Vec<u8>),
    /// Match any live value, but fail when the key is absent/tombstoned.
    Present,
}

/// Entry-event projection requested by Java/IMap-style listeners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryEventProjection {
    /// Plain near-cache invalidation signal.
    Invalidation,
    /// IMap entry-event shaped cache signal.
    IMapEntryEvent,
}

/// Source signal that can be projected into an IMap entry event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryEventSource {
    /// A value was written, but the signal does not prove whether it was add or update.
    Stored,
    /// A value was explicitly removed or tombstoned.
    Removed,
    /// A key was invalidated without a stronger transition reason.
    KeyInvalidated,
    /// A tag invalidated one or more keys.
    TagInvalidated,
    /// A whole cache/namespace was flushed.
    Flushed,
    /// A value expired.
    Expired,
    /// A value was evicted.
    Evicted,
    /// A stale loader result was discarded.
    StaleLoadDiscarded,
    /// Unknown or transport-specific signal.
    Unknown,
}

/// Entry-event kind exposed to Java/IMap-style listeners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryEventKind {
    /// A key changed, but the signal cannot distinguish add from update.
    Upserted,
    /// A key was removed/tombstoned.
    Removed,
    /// A key was evicted or expired.
    Evicted,
    /// A freshness invalidation without business-event semantics.
    Invalidated,
}

impl EntryEventKind {
    /// Conservatively project a cache/invalidation source into an entry-event kind.
    pub const fn from_source(source: EntryEventSource) -> Self {
        match source {
            EntryEventSource::Stored => Self::Upserted,
            EntryEventSource::Removed => Self::Removed,
            EntryEventSource::Expired | EntryEventSource::Evicted => Self::Evicted,
            EntryEventSource::KeyInvalidated
            | EntryEventSource::TagInvalidated
            | EntryEventSource::Flushed
            | EntryEventSource::StaleLoadDiscarded
            | EntryEventSource::Unknown => Self::Invalidated,
        }
    }
}

/// IMap entry-event shaped cache signal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryEvent {
    /// Namespace.
    pub ns: Namespace,
    /// Key when the underlying signal is key-scoped.
    pub key: Option<StructuredKey>,
    /// Conservative event kind.
    pub kind: EntryEventKind,
    /// Optional value, gated by residency and transport support.
    pub value: Option<Vec<u8>>,
    /// Whether value inclusion was degraded by residency.
    pub residency_degraded: bool,
    /// Event watermark, if the source carries one.
    pub watermark: Option<Watermark>,
}

/// Executable contract for W6 listener semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryListenerContract {
    /// Signals may be coalesced and are not a complete event history.
    pub coalesced: bool,
    /// Delivery uses bounded buffers.
    pub bounded_buffer: bool,
    /// Slow listeners are dropped/reported through lag counters.
    pub lag_drop_counter: bool,
    /// This surface must not be used as a business event log.
    pub business_event_log: bool,
}

impl EntryListenerContract {
    /// Return the shipped W6 listener contract.
    pub const fn cache_signal() -> Self {
        Self {
            coalesced: true,
            bounded_buffer: true,
            lag_drop_counter: true,
            business_event_log: false,
        }
    }
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

        if next.source_generation != last.source_generation {
            self.last = Some(next);
            return RepairAction::ClearPartition;
        }
        if next.message_id > last.message_id.saturating_add(1) {
            self.last = Some(next);
            return RepairAction::InvalidateConservatively;
        }
        self.last = Some(Watermark::new(
            last.source_generation,
            last.message_id.max(next.message_id),
        ));
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
    /// Create an envelope for the highest supported protocol version.
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

    /// Validate the envelope version and operation minimum version.
    pub fn validate_protocol(&self) -> Result<(), ClientErrorEnvelope> {
        self.request.ensure_supported_by(self.protocol_version)
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
    /// Set or replace the expiry for one key.
    Expire {
        ns: Namespace,
        key: StructuredKey,
        ttl_ms: u64,
    },
    /// Remove the expiry for one key without changing its value.
    Persist { ns: Namespace, key: StructuredKey },
    /// Read remaining TTL metadata for one key.
    GetTtl { ns: Namespace, key: StructuredKey },
    /// Store one value only when the declared condition holds.
    ConditionalPut {
        ns: Namespace,
        key: StructuredKey,
        value: Vec<u8>,
        ttl_ms: Option<u64>,
        condition: ConditionalPutCondition,
    },
    /// Invalidate one key only when the current live value matches.
    CompareValueAndInvalidate {
        ns: Namespace,
        key: StructuredKey,
        expected_value: Vec<u8>,
    },
    /// Replace expiry only when the current live value matches.
    CompareValueAndExpire {
        ns: Namespace,
        key: StructuredKey,
        expected_value: Vec<u8>,
        ttl_ms: u64,
        mode: CompareValueExpireMode,
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
    /// Subscribe to IMap entry-event shaped cache signals.
    SubscribeEntryEvents {
        ns: Namespace,
        region: Option<RegionId>,
        from: Option<Watermark>,
        include_value: bool,
        projection: EntryEventProjection,
    },
    /// Try to acquire a session-bound fenced lock.
    TryLock {
        ns: Namespace,
        key: StructuredKey,
        lease_ms: u64,
        wait_ms: u64,
        level: LockConsistency,
    },
    /// Release a fenced lock with the current fence token.
    Unlock {
        ns: Namespace,
        key: StructuredKey,
        fence: u64,
    },
    /// Renew the lease for the current lock owner.
    RenewLockLease {
        ns: Namespace,
        key: StructuredKey,
        fence: u64,
        lease_ms: u64,
    },
    /// Privileged fence-advancing release.
    ForceUnlock { ns: Namespace, key: StructuredKey },
    /// Read current lock ownership metadata.
    GetLockOwnership { ns: Namespace, key: StructuredKey },
    /// Single-key compare-and-set for IMap replace ergonomics.
    CompareAndSet {
        ns: Namespace,
        key: StructuredKey,
        expected: CasExpectation,
        new_value: Vec<u8>,
        level: LockConsistency,
    },
    /// Single-key conditional tombstone for IMap remove(key, value).
    RemoveIfValue {
        ns: Namespace,
        key: StructuredKey,
        expected: Vec<u8>,
        level: LockConsistency,
    },
}

impl ClientRequest {
    /// Minimum protocol version required by this operation.
    pub fn minimum_protocol_version(&self) -> u16 {
        match self {
            Self::Get { .. }
            | Self::Invalidate { .. }
            | Self::BatchGet { .. }
            | Self::BatchPut { .. }
            | Self::EvictRegion { .. }
            | Self::SubscribeInvalidations { .. } => MIN_PROTOCOL_VERSION,
            Self::Put {
                ttl_ms: Some(_), ..
            }
            | Self::Expire { .. }
            | Self::Persist { .. }
            | Self::GetTtl { .. } => TTL_PROTOCOL_VERSION,
            Self::ConditionalPut { .. }
            | Self::CompareValueAndInvalidate { .. }
            | Self::CompareValueAndExpire { .. } => REDIS_LOCK_PROTOCOL_VERSION,
            Self::Put { ttl_ms: None, .. } => MIN_PROTOCOL_VERSION,
            Self::SubscribeEntryEvents { .. }
            | Self::TryLock { .. }
            | Self::Unlock { .. }
            | Self::RenewLockLease { .. }
            | Self::ForceUnlock { .. }
            | Self::GetLockOwnership { .. }
            | Self::CompareAndSet { .. }
            | Self::RemoveIfValue { .. } => LOCK_PROTOCOL_VERSION,
        }
    }

    /// Validate this operation against a negotiated protocol version.
    pub fn ensure_supported_by(&self, protocol_version: u16) -> Result<(), ClientErrorEnvelope> {
        require_protocol_version(
            protocol_version,
            self.minimum_protocol_version(),
            self.operation_name(),
        )
    }

    fn operation_name(&self) -> &'static str {
        match self {
            Self::Get { .. } => "get",
            Self::Put { .. } => "put",
            Self::Invalidate { .. } => "invalidate",
            Self::BatchGet { .. } => "batch_get",
            Self::BatchPut { .. } => "batch_put",
            Self::Expire { .. } => "expire",
            Self::Persist { .. } => "persist",
            Self::GetTtl { .. } => "get_ttl",
            Self::ConditionalPut { .. } => "conditional_put",
            Self::CompareValueAndInvalidate { .. } => "compare_value_and_invalidate",
            Self::CompareValueAndExpire { .. } => "compare_value_and_expire",
            Self::EvictRegion { .. } => "evict_region",
            Self::SubscribeInvalidations { .. } => "subscribe_invalidations",
            Self::SubscribeEntryEvents { .. } => "subscribe_entry_events",
            Self::TryLock { .. } => "try_lock",
            Self::Unlock { .. } => "unlock",
            Self::RenewLockLease { .. } => "renew_lock_lease",
            Self::ForceUnlock { .. } => "force_unlock",
            Self::GetLockOwnership { .. } => "get_lock_ownership",
            Self::CompareAndSet { .. } => "compare_and_set",
            Self::RemoveIfValue { .. } => "remove_if_value",
        }
    }
}

/// Condition used by v4 conditional value writes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionalPutCondition {
    /// Store only if the key is missing or expired.
    IfAbsent,
    /// Store only if the current live value exactly matches the supplied bytes.
    IfPresentValue(Vec<u8>),
}

/// Expiry update mode for token-safe Redis lock extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompareValueExpireMode {
    /// Replace the remaining TTL with the supplied TTL.
    Replace,
    /// Replace the remaining TTL only when the current live entry already has an expiry.
    ReplaceIfExpiring,
    /// Add the supplied TTL to the current remaining TTL.
    AddToRemaining,
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

    /// Return this response encoded for a negotiated protocol version.
    pub fn with_protocol_version(mut self, protocol_version: u16) -> Self {
        self.protocol_version = protocol_version;
        self
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
    /// Expiry mutation result.
    Expiry {
        /// Whether the key existed and the expiry state changed as requested.
        applied: bool,
    },
    /// Remaining TTL metadata.
    Ttl {
        /// Redis-compatible remaining TTL state.
        state: TtlState,
    },
    /// Conditional put result.
    ConditionalStored {
        /// Whether the value was stored.
        stored: bool,
    },
    /// Compare-value mutation result.
    CompareValueApplied {
        /// Whether the compare-value mutation was applied.
        applied: bool,
    },
    /// Region/namespace eviction accepted.
    Evicted,
    /// Subscription accepted.
    Subscribed { from: Option<Watermark> },
    /// Fenced lock acquired.
    LockAcquired { fence: u64 },
    /// Fenced lock is currently held by another owner.
    LockBusy,
    /// Fenced lock released.
    LockReleased,
    /// Fenced lock lease renewed.
    LockLeaseRenewed,
    /// Current lock ownership.
    LockOwnership { fence: Option<u64>, locked: bool },
    /// CAS applied and produced a new monotonic version.
    CasApplied { new_version: u64 },
    /// CAS did not apply; carries the current live value if present.
    CasMismatch { current: Option<Vec<u8>> },
}

impl ClientResponse {
    fn minimum_protocol_version(&self) -> u16 {
        match self {
            Self::Value { .. }
            | Self::Stored
            | Self::Invalidated
            | Self::Batch { .. }
            | Self::Evicted
            | Self::Subscribed { .. } => MIN_PROTOCOL_VERSION,
            Self::LockAcquired { .. }
            | Self::LockBusy
            | Self::LockReleased
            | Self::LockLeaseRenewed
            | Self::LockOwnership { .. }
            | Self::CasApplied { .. }
            | Self::CasMismatch { .. } => LOCK_PROTOCOL_VERSION,
            Self::Expiry { .. } | Self::Ttl { .. } => TTL_PROTOCOL_VERSION,
            Self::ConditionalStored { .. } | Self::CompareValueApplied { .. } => {
                REDIS_LOCK_PROTOCOL_VERSION
            }
        }
    }

    fn response_name(&self) -> &'static str {
        match self {
            Self::Value { .. } => "value",
            Self::Stored => "stored",
            Self::Invalidated => "invalidated",
            Self::Batch { .. } => "batch",
            Self::Expiry { .. } => "expiry",
            Self::Ttl { .. } => "ttl",
            Self::ConditionalStored { .. } => "conditional_stored",
            Self::CompareValueApplied { .. } => "compare_value_applied",
            Self::Evicted => "evicted",
            Self::Subscribed { .. } => "subscribed",
            Self::LockAcquired { .. } => "lock_acquired",
            Self::LockBusy => "lock_busy",
            Self::LockReleased => "lock_released",
            Self::LockLeaseRenewed => "lock_lease_renewed",
            Self::LockOwnership { .. } => "lock_ownership",
            Self::CasApplied { .. } => "cas_applied",
            Self::CasMismatch { .. } => "cas_mismatch",
        }
    }
}

/// Redis-compatible remaining TTL state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TtlState {
    /// Key does not exist or is already expired.
    Missing,
    /// Key exists and has no expiry.
    Persistent,
    /// Key exists and has a positive remaining TTL in milliseconds.
    ExpiresIn { ttl_ms: u64 },
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

impl EntryEvent {
    /// Project a near-cache invalidation into an IMap-shaped entry-event signal.
    pub fn from_invalidation(event: InvalidationEvent) -> Self {
        let watermark = event.watermark();
        Self {
            ns: event.ns,
            key: Some(event.key),
            kind: EntryEventKind::Invalidated,
            value: event.value,
            residency_degraded: event.residency_degraded,
            watermark: Some(watermark),
        }
    }

    /// Build an entry event from a known cache signal source.
    pub fn from_source(
        ns: Namespace,
        key: Option<StructuredKey>,
        source: EntryEventSource,
        value: Option<Vec<u8>>,
        watermark: Option<Watermark>,
    ) -> Self {
        Self {
            ns,
            key,
            kind: EntryEventKind::from_source(source),
            value,
            residency_degraded: false,
            watermark,
        }
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

impl ClientWireMessage {
    /// Return the protocol version that should be used for the outer frame.
    pub fn protocol_version(&self) -> u16 {
        match self {
            Self::Handshake(handshake) => handshake.max,
            Self::Request(envelope) => envelope.protocol_version,
            Self::Response(envelope) => envelope.protocol_version,
            Self::Invalidation(_) | Self::Heartbeat(_) => PROTOCOL_VERSION,
        }
    }
}

fn ensure_frame_version_supported(protocol_version: u16) -> Result<(), ClientProtocolError> {
    if (MIN_PROTOCOL_VERSION..=PROTOCOL_VERSION).contains(&protocol_version) {
        Ok(())
    } else {
        Err(ClientProtocolError::UnsupportedVersion {
            version: protocol_version,
            supported_max: PROTOCOL_VERSION,
        })
    }
}

fn ensure_message_version_matches_frame(
    frame_version: u16,
    message: &ClientWireMessage,
) -> Result<(), ClientProtocolError> {
    let declared_version = match message {
        ClientWireMessage::Handshake(handshake) => Some(handshake.max),
        ClientWireMessage::Request(envelope) => Some(envelope.protocol_version),
        ClientWireMessage::Response(envelope) => Some(envelope.protocol_version),
        ClientWireMessage::Invalidation(_) | ClientWireMessage::Heartbeat(_) => None,
    };
    if declared_version.is_none_or(|version| version == frame_version) {
        Ok(())
    } else {
        Err(ClientProtocolError::VersionMismatch {
            frame_version,
            message_version: declared_version.expect("checked as some"),
        })
    }
}

fn ensure_message_supported_by_frame_version(
    frame_version: u16,
    message: &ClientWireMessage,
) -> Result<(), ClientProtocolError> {
    match message {
        ClientWireMessage::Request(envelope)
            if envelope.request.minimum_protocol_version() > frame_version =>
        {
            Err(unsupported_for_version(
                envelope.request.operation_name(),
                frame_version,
            ))
        }
        ClientWireMessage::Response(envelope) => match &envelope.result {
            Ok(response) if response.minimum_protocol_version() > frame_version => Err(
                unsupported_for_version(response.response_name(), frame_version),
            ),
            _ => Ok(()),
        },
        _ => Ok(()),
    }
}

fn encode_wire_message_for_version(
    protocol_version: u16,
    message: &ClientWireMessage,
) -> Result<Vec<u8>, ClientProtocolError> {
    let encoded = match protocol_version {
        MIN_PROTOCOL_VERSION => postcard::to_allocvec(&WireMessageV1::try_from(message)?),
        LOCK_PROTOCOL_VERSION => postcard::to_allocvec(&WireMessageV2::try_from(message)?),
        TTL_PROTOCOL_VERSION => postcard::to_allocvec(&WireMessageV3::try_from(message)?),
        PROTOCOL_VERSION => postcard::to_allocvec(message),
        _ => unreachable!("supported protocol versions are checked before encoding"),
    };
    encoded.map_err(|error| ClientProtocolError::Codec(error.to_string()))
}

fn decode_wire_message_for_version(
    protocol_version: u16,
    payload: &[u8],
) -> Result<ClientWireMessage, ClientProtocolError> {
    ensure_frame_version_supported(protocol_version)?;
    match protocol_version {
        MIN_PROTOCOL_VERSION => decode_postcard_exact::<WireMessageV1>(payload).map(Into::into),
        LOCK_PROTOCOL_VERSION => decode_postcard_exact::<WireMessageV2>(payload).map(Into::into),
        TTL_PROTOCOL_VERSION => decode_postcard_exact::<WireMessageV3>(payload).map(Into::into),
        PROTOCOL_VERSION => decode_postcard_exact(payload),
        _ => unreachable!("supported protocol versions are checked before decoding"),
    }
}

fn decode_postcard_exact<T: DeserializeOwned>(payload: &[u8]) -> Result<T, ClientProtocolError> {
    let (message, remainder) = postcard::take_from_bytes(payload)
        .map_err(|error| ClientProtocolError::Codec(error.to_string()))?;
    if !remainder.is_empty() {
        return Err(ClientProtocolError::Codec(
            "trailing bytes after client wire message".to_owned(),
        ));
    }
    Ok(message)
}

// This is the exact v1 schema published by c396437. Keep it separate from
// later schemas: sharing the wider v2 enum would let lock/CAS discriminants be
// smuggled through a frame labeled as v1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireMessageV1 {
    Handshake(VersionHandshake),
    Request(RequestEnvelopeV1),
    Response(ResponseEnvelopeV1),
    Invalidation(InvalidationEvent),
    Heartbeat(Watermark),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RequestEnvelopeV1 {
    request_id: String,
    protocol_version: u16,
    context: ClientContext,
    deadline_ms: Option<u64>,
    idempotency_key: Option<String>,
    request: RequestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RequestV1 {
    Get {
        ns: Namespace,
        key: StructuredKey,
    },
    Put {
        ns: Namespace,
        key: StructuredKey,
        value: Vec<u8>,
        ttl_ms: Option<u64>,
        dimensions: Vec<String>,
    },
    Invalidate {
        ns: Namespace,
        key: StructuredKey,
    },
    BatchGet {
        ns: Namespace,
        keys: Vec<StructuredKey>,
    },
    BatchPut {
        ns: Namespace,
        entries: Vec<BatchPutEntry>,
    },
    EvictRegion {
        ns: Namespace,
    },
    SubscribeInvalidations {
        ns: Namespace,
        region: Option<RegionId>,
        from: Option<Watermark>,
        include_value: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ResponseEnvelopeV1 {
    request_id: String,
    protocol_version: u16,
    result: Result<ResponseV1, ClientErrorEnvelope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ResponseV1 {
    Value { value: Option<Vec<u8>> },
    Stored,
    Invalidated,
    Batch { items: Vec<BatchItemStatus> },
    Evicted,
    Subscribed { from: Option<Watermark> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireMessageV2 {
    Handshake(VersionHandshake),
    Request(RequestEnvelopeV2),
    Response(ResponseEnvelopeV2),
    Invalidation(InvalidationEvent),
    Heartbeat(Watermark),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RequestEnvelopeV2 {
    request_id: String,
    protocol_version: u16,
    context: ClientContext,
    deadline_ms: Option<u64>,
    idempotency_key: Option<String>,
    request: RequestV2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RequestV2 {
    Get {
        ns: Namespace,
        key: StructuredKey,
    },
    Put {
        ns: Namespace,
        key: StructuredKey,
        value: Vec<u8>,
        ttl_ms: Option<u64>,
        dimensions: Vec<String>,
    },
    Invalidate {
        ns: Namespace,
        key: StructuredKey,
    },
    BatchGet {
        ns: Namespace,
        keys: Vec<StructuredKey>,
    },
    BatchPut {
        ns: Namespace,
        entries: Vec<BatchPutEntry>,
    },
    EvictRegion {
        ns: Namespace,
    },
    SubscribeInvalidations {
        ns: Namespace,
        region: Option<RegionId>,
        from: Option<Watermark>,
        include_value: bool,
    },
    SubscribeEntryEvents {
        ns: Namespace,
        region: Option<RegionId>,
        from: Option<Watermark>,
        include_value: bool,
        projection: EntryEventProjection,
    },
    TryLock {
        ns: Namespace,
        key: StructuredKey,
        lease_ms: u64,
        wait_ms: u64,
        level: LockConsistency,
    },
    Unlock {
        ns: Namespace,
        key: StructuredKey,
        fence: u64,
    },
    RenewLockLease {
        ns: Namespace,
        key: StructuredKey,
        fence: u64,
        lease_ms: u64,
    },
    ForceUnlock {
        ns: Namespace,
        key: StructuredKey,
    },
    GetLockOwnership {
        ns: Namespace,
        key: StructuredKey,
    },
    CompareAndSet {
        ns: Namespace,
        key: StructuredKey,
        expected: CasExpectation,
        new_value: Vec<u8>,
        level: LockConsistency,
    },
    RemoveIfValue {
        ns: Namespace,
        key: StructuredKey,
        expected: Vec<u8>,
        level: LockConsistency,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ResponseEnvelopeV2 {
    request_id: String,
    protocol_version: u16,
    result: Result<ResponseV2, ClientErrorEnvelope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ResponseV2 {
    Value { value: Option<Vec<u8>> },
    Stored,
    Invalidated,
    Batch { items: Vec<BatchItemStatus> },
    Evicted,
    Subscribed { from: Option<Watermark> },
    LockAcquired { fence: u64 },
    LockBusy,
    LockReleased,
    LockLeaseRenewed,
    LockOwnership { fence: Option<u64>, locked: bool },
    CasApplied { new_version: u64 },
    CasMismatch { current: Option<Vec<u8>> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireMessageV3 {
    Handshake(VersionHandshake),
    Request(RequestEnvelopeV3),
    Response(ResponseEnvelopeV3),
    Invalidation(InvalidationEvent),
    Heartbeat(Watermark),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RequestEnvelopeV3 {
    request_id: String,
    protocol_version: u16,
    context: ClientContext,
    deadline_ms: Option<u64>,
    idempotency_key: Option<String>,
    request: RequestV3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RequestV3 {
    Get {
        ns: Namespace,
        key: StructuredKey,
    },
    Put {
        ns: Namespace,
        key: StructuredKey,
        value: Vec<u8>,
        ttl_ms: Option<u64>,
        dimensions: Vec<String>,
    },
    Invalidate {
        ns: Namespace,
        key: StructuredKey,
    },
    BatchGet {
        ns: Namespace,
        keys: Vec<StructuredKey>,
    },
    BatchPut {
        ns: Namespace,
        entries: Vec<BatchPutEntry>,
    },
    Expire {
        ns: Namespace,
        key: StructuredKey,
        ttl_ms: u64,
    },
    Persist {
        ns: Namespace,
        key: StructuredKey,
    },
    GetTtl {
        ns: Namespace,
        key: StructuredKey,
    },
    EvictRegion {
        ns: Namespace,
    },
    SubscribeInvalidations {
        ns: Namespace,
        region: Option<RegionId>,
        from: Option<Watermark>,
        include_value: bool,
    },
    SubscribeEntryEvents {
        ns: Namespace,
        region: Option<RegionId>,
        from: Option<Watermark>,
        include_value: bool,
        projection: EntryEventProjection,
    },
    TryLock {
        ns: Namespace,
        key: StructuredKey,
        lease_ms: u64,
        wait_ms: u64,
        level: LockConsistency,
    },
    Unlock {
        ns: Namespace,
        key: StructuredKey,
        fence: u64,
    },
    RenewLockLease {
        ns: Namespace,
        key: StructuredKey,
        fence: u64,
        lease_ms: u64,
    },
    ForceUnlock {
        ns: Namespace,
        key: StructuredKey,
    },
    GetLockOwnership {
        ns: Namespace,
        key: StructuredKey,
    },
    CompareAndSet {
        ns: Namespace,
        key: StructuredKey,
        expected: CasExpectation,
        new_value: Vec<u8>,
        level: LockConsistency,
    },
    RemoveIfValue {
        ns: Namespace,
        key: StructuredKey,
        expected: Vec<u8>,
        level: LockConsistency,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ResponseEnvelopeV3 {
    request_id: String,
    protocol_version: u16,
    result: Result<ResponseV3, ClientErrorEnvelope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ResponseV3 {
    Value { value: Option<Vec<u8>> },
    Stored,
    Invalidated,
    Batch { items: Vec<BatchItemStatus> },
    Expiry { applied: bool },
    Ttl { state: TtlState },
    Evicted,
    Subscribed { from: Option<Watermark> },
    LockAcquired { fence: u64 },
    LockBusy,
    LockReleased,
    LockLeaseRenewed,
    LockOwnership { fence: Option<u64>, locked: bool },
    CasApplied { new_version: u64 },
    CasMismatch { current: Option<Vec<u8>> },
}

impl TryFrom<&ClientWireMessage> for WireMessageV1 {
    type Error = ClientProtocolError;

    fn try_from(message: &ClientWireMessage) -> Result<Self, Self::Error> {
        Ok(match message {
            ClientWireMessage::Handshake(handshake) => Self::Handshake(*handshake),
            ClientWireMessage::Request(envelope) => {
                Self::Request(RequestEnvelopeV1::try_from(envelope)?)
            }
            ClientWireMessage::Response(envelope) => {
                Self::Response(ResponseEnvelopeV1::try_from(envelope)?)
            }
            ClientWireMessage::Invalidation(event) => Self::Invalidation(event.clone()),
            ClientWireMessage::Heartbeat(watermark) => Self::Heartbeat(*watermark),
        })
    }
}

impl From<WireMessageV1> for ClientWireMessage {
    fn from(message: WireMessageV1) -> Self {
        match message {
            WireMessageV1::Handshake(handshake) => Self::Handshake(handshake),
            WireMessageV1::Request(envelope) => Self::Request(envelope.into()),
            WireMessageV1::Response(envelope) => Self::Response(envelope.into()),
            WireMessageV1::Invalidation(event) => Self::Invalidation(event),
            WireMessageV1::Heartbeat(watermark) => Self::Heartbeat(watermark),
        }
    }
}

impl TryFrom<&ClientRequestEnvelope> for RequestEnvelopeV1 {
    type Error = ClientProtocolError;

    fn try_from(envelope: &ClientRequestEnvelope) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: envelope.request_id.clone(),
            protocol_version: envelope.protocol_version,
            context: envelope.context.clone(),
            deadline_ms: envelope.deadline_ms,
            idempotency_key: envelope.idempotency_key.clone(),
            request: RequestV1::try_from(&envelope.request)?,
        })
    }
}

impl From<RequestEnvelopeV1> for ClientRequestEnvelope {
    fn from(envelope: RequestEnvelopeV1) -> Self {
        Self {
            request_id: envelope.request_id,
            protocol_version: envelope.protocol_version,
            context: envelope.context,
            deadline_ms: envelope.deadline_ms,
            idempotency_key: envelope.idempotency_key,
            request: envelope.request.into(),
        }
    }
}

impl TryFrom<&ClientResponseEnvelope> for ResponseEnvelopeV1 {
    type Error = ClientProtocolError;

    fn try_from(envelope: &ClientResponseEnvelope) -> Result<Self, Self::Error> {
        let result = match &envelope.result {
            Ok(response) => Ok(ResponseV1::try_from(response)?),
            Err(error) => Err(error.clone()),
        };
        Ok(Self {
            request_id: envelope.request_id.clone(),
            protocol_version: envelope.protocol_version,
            result,
        })
    }
}

impl From<ResponseEnvelopeV1> for ClientResponseEnvelope {
    fn from(envelope: ResponseEnvelopeV1) -> Self {
        Self {
            request_id: envelope.request_id,
            protocol_version: envelope.protocol_version,
            result: envelope.result.map(Into::into),
        }
    }
}

impl TryFrom<&ClientRequest> for RequestV1 {
    type Error = ClientProtocolError;

    fn try_from(request: &ClientRequest) -> Result<Self, Self::Error> {
        Ok(match request {
            ClientRequest::Get { ns, key } => Self::Get {
                ns: ns.clone(),
                key: key.clone(),
            },
            ClientRequest::Put {
                ns,
                key,
                value,
                ttl_ms,
                dimensions,
            } => Self::Put {
                ns: ns.clone(),
                key: key.clone(),
                value: value.clone(),
                ttl_ms: *ttl_ms,
                dimensions: dimensions.clone(),
            },
            ClientRequest::Invalidate { ns, key } => Self::Invalidate {
                ns: ns.clone(),
                key: key.clone(),
            },
            ClientRequest::BatchGet { ns, keys } => Self::BatchGet {
                ns: ns.clone(),
                keys: keys.clone(),
            },
            ClientRequest::BatchPut { ns, entries } => Self::BatchPut {
                ns: ns.clone(),
                entries: entries.clone(),
            },
            ClientRequest::EvictRegion { ns } => Self::EvictRegion { ns: ns.clone() },
            ClientRequest::SubscribeInvalidations {
                ns,
                region,
                from,
                include_value,
            } => Self::SubscribeInvalidations {
                ns: ns.clone(),
                region: region.clone(),
                from: *from,
                include_value: *include_value,
            },
            unsupported => {
                return Err(unsupported_for_version(unsupported.operation_name(), 1));
            }
        })
    }
}

impl From<RequestV1> for ClientRequest {
    fn from(request: RequestV1) -> Self {
        match request {
            RequestV1::Get { ns, key } => Self::Get { ns, key },
            RequestV1::Put {
                ns,
                key,
                value,
                ttl_ms,
                dimensions,
            } => Self::Put {
                ns,
                key,
                value,
                ttl_ms,
                dimensions,
            },
            RequestV1::Invalidate { ns, key } => Self::Invalidate { ns, key },
            RequestV1::BatchGet { ns, keys } => Self::BatchGet { ns, keys },
            RequestV1::BatchPut { ns, entries } => Self::BatchPut { ns, entries },
            RequestV1::EvictRegion { ns } => Self::EvictRegion { ns },
            RequestV1::SubscribeInvalidations {
                ns,
                region,
                from,
                include_value,
            } => Self::SubscribeInvalidations {
                ns,
                region,
                from,
                include_value,
            },
        }
    }
}

impl TryFrom<&ClientResponse> for ResponseV1 {
    type Error = ClientProtocolError;

    fn try_from(response: &ClientResponse) -> Result<Self, Self::Error> {
        Ok(match response {
            ClientResponse::Value { value } => Self::Value {
                value: value.clone(),
            },
            ClientResponse::Stored => Self::Stored,
            ClientResponse::Invalidated => Self::Invalidated,
            ClientResponse::Batch { items } => Self::Batch {
                items: items.clone(),
            },
            ClientResponse::Evicted => Self::Evicted,
            ClientResponse::Subscribed { from } => Self::Subscribed { from: *from },
            unsupported => {
                return Err(unsupported_for_version(unsupported.response_name(), 1));
            }
        })
    }
}

impl From<ResponseV1> for ClientResponse {
    fn from(response: ResponseV1) -> Self {
        match response {
            ResponseV1::Value { value } => Self::Value { value },
            ResponseV1::Stored => Self::Stored,
            ResponseV1::Invalidated => Self::Invalidated,
            ResponseV1::Batch { items } => Self::Batch { items },
            ResponseV1::Evicted => Self::Evicted,
            ResponseV1::Subscribed { from } => Self::Subscribed { from },
        }
    }
}

impl TryFrom<&ClientWireMessage> for WireMessageV2 {
    type Error = ClientProtocolError;

    fn try_from(message: &ClientWireMessage) -> Result<Self, Self::Error> {
        Ok(match message {
            ClientWireMessage::Handshake(handshake) => Self::Handshake(*handshake),
            ClientWireMessage::Request(envelope) => {
                Self::Request(RequestEnvelopeV2::try_from(envelope)?)
            }
            ClientWireMessage::Response(envelope) => {
                Self::Response(ResponseEnvelopeV2::try_from(envelope)?)
            }
            ClientWireMessage::Invalidation(event) => Self::Invalidation(event.clone()),
            ClientWireMessage::Heartbeat(watermark) => Self::Heartbeat(*watermark),
        })
    }
}

impl TryFrom<&ClientWireMessage> for WireMessageV3 {
    type Error = ClientProtocolError;

    fn try_from(message: &ClientWireMessage) -> Result<Self, Self::Error> {
        Ok(match message {
            ClientWireMessage::Handshake(handshake) => Self::Handshake(*handshake),
            ClientWireMessage::Request(envelope) => {
                Self::Request(RequestEnvelopeV3::try_from(envelope)?)
            }
            ClientWireMessage::Response(envelope) => {
                Self::Response(ResponseEnvelopeV3::try_from(envelope)?)
            }
            ClientWireMessage::Invalidation(event) => Self::Invalidation(event.clone()),
            ClientWireMessage::Heartbeat(watermark) => Self::Heartbeat(*watermark),
        })
    }
}

impl From<WireMessageV2> for ClientWireMessage {
    fn from(message: WireMessageV2) -> Self {
        match message {
            WireMessageV2::Handshake(handshake) => Self::Handshake(handshake),
            WireMessageV2::Request(envelope) => Self::Request(envelope.into()),
            WireMessageV2::Response(envelope) => Self::Response(envelope.into()),
            WireMessageV2::Invalidation(event) => Self::Invalidation(event),
            WireMessageV2::Heartbeat(watermark) => Self::Heartbeat(watermark),
        }
    }
}

impl From<WireMessageV3> for ClientWireMessage {
    fn from(message: WireMessageV3) -> Self {
        match message {
            WireMessageV3::Handshake(handshake) => Self::Handshake(handshake),
            WireMessageV3::Request(envelope) => Self::Request(envelope.into()),
            WireMessageV3::Response(envelope) => Self::Response(envelope.into()),
            WireMessageV3::Invalidation(event) => Self::Invalidation(event),
            WireMessageV3::Heartbeat(watermark) => Self::Heartbeat(watermark),
        }
    }
}

impl TryFrom<&ClientRequestEnvelope> for RequestEnvelopeV2 {
    type Error = ClientProtocolError;

    fn try_from(envelope: &ClientRequestEnvelope) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: envelope.request_id.clone(),
            protocol_version: envelope.protocol_version,
            context: envelope.context.clone(),
            deadline_ms: envelope.deadline_ms,
            idempotency_key: envelope.idempotency_key.clone(),
            request: RequestV2::try_from(&envelope.request)?,
        })
    }
}

impl TryFrom<&ClientRequestEnvelope> for RequestEnvelopeV3 {
    type Error = ClientProtocolError;

    fn try_from(envelope: &ClientRequestEnvelope) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: envelope.request_id.clone(),
            protocol_version: envelope.protocol_version,
            context: envelope.context.clone(),
            deadline_ms: envelope.deadline_ms,
            idempotency_key: envelope.idempotency_key.clone(),
            request: RequestV3::try_from(&envelope.request)?,
        })
    }
}

impl From<RequestEnvelopeV2> for ClientRequestEnvelope {
    fn from(envelope: RequestEnvelopeV2) -> Self {
        Self {
            request_id: envelope.request_id,
            protocol_version: envelope.protocol_version,
            context: envelope.context,
            deadline_ms: envelope.deadline_ms,
            idempotency_key: envelope.idempotency_key,
            request: envelope.request.into(),
        }
    }
}

impl From<RequestEnvelopeV3> for ClientRequestEnvelope {
    fn from(envelope: RequestEnvelopeV3) -> Self {
        Self {
            request_id: envelope.request_id,
            protocol_version: envelope.protocol_version,
            context: envelope.context,
            deadline_ms: envelope.deadline_ms,
            idempotency_key: envelope.idempotency_key,
            request: envelope.request.into(),
        }
    }
}

impl TryFrom<&ClientResponseEnvelope> for ResponseEnvelopeV2 {
    type Error = ClientProtocolError;

    fn try_from(envelope: &ClientResponseEnvelope) -> Result<Self, Self::Error> {
        let result = match &envelope.result {
            Ok(response) => Ok(ResponseV2::try_from(response)?),
            Err(error) => Err(error.clone()),
        };
        Ok(Self {
            request_id: envelope.request_id.clone(),
            protocol_version: envelope.protocol_version,
            result,
        })
    }
}

impl TryFrom<&ClientResponseEnvelope> for ResponseEnvelopeV3 {
    type Error = ClientProtocolError;

    fn try_from(envelope: &ClientResponseEnvelope) -> Result<Self, Self::Error> {
        let result = match &envelope.result {
            Ok(response) => Ok(ResponseV3::try_from(response)?),
            Err(error) => Err(error.clone()),
        };
        Ok(Self {
            request_id: envelope.request_id.clone(),
            protocol_version: envelope.protocol_version,
            result,
        })
    }
}

impl From<ResponseEnvelopeV2> for ClientResponseEnvelope {
    fn from(envelope: ResponseEnvelopeV2) -> Self {
        Self {
            request_id: envelope.request_id,
            protocol_version: envelope.protocol_version,
            result: envelope.result.map(Into::into),
        }
    }
}

impl From<ResponseEnvelopeV3> for ClientResponseEnvelope {
    fn from(envelope: ResponseEnvelopeV3) -> Self {
        Self {
            request_id: envelope.request_id,
            protocol_version: envelope.protocol_version,
            result: envelope.result.map(Into::into),
        }
    }
}

fn unsupported_for_version(operation: &'static str, version: u16) -> ClientProtocolError {
    ClientProtocolError::UnsupportedMessageForVersion { operation, version }
}

impl TryFrom<&ClientRequest> for RequestV2 {
    type Error = ClientProtocolError;

    fn try_from(request: &ClientRequest) -> Result<Self, Self::Error> {
        Ok(match request {
            ClientRequest::Get { ns, key } => Self::Get {
                ns: ns.clone(),
                key: key.clone(),
            },
            ClientRequest::Put {
                ns,
                key,
                value,
                ttl_ms,
                dimensions,
            } => Self::Put {
                ns: ns.clone(),
                key: key.clone(),
                value: value.clone(),
                ttl_ms: *ttl_ms,
                dimensions: dimensions.clone(),
            },
            ClientRequest::Invalidate { ns, key } => Self::Invalidate {
                ns: ns.clone(),
                key: key.clone(),
            },
            ClientRequest::BatchGet { ns, keys } => Self::BatchGet {
                ns: ns.clone(),
                keys: keys.clone(),
            },
            ClientRequest::BatchPut { ns, entries } => Self::BatchPut {
                ns: ns.clone(),
                entries: entries.clone(),
            },
            ClientRequest::EvictRegion { ns } => Self::EvictRegion { ns: ns.clone() },
            ClientRequest::SubscribeInvalidations {
                ns,
                region,
                from,
                include_value,
            } => Self::SubscribeInvalidations {
                ns: ns.clone(),
                region: region.clone(),
                from: *from,
                include_value: *include_value,
            },
            ClientRequest::SubscribeEntryEvents {
                ns,
                region,
                from,
                include_value,
                projection,
            } => Self::SubscribeEntryEvents {
                ns: ns.clone(),
                region: region.clone(),
                from: *from,
                include_value: *include_value,
                projection: *projection,
            },
            ClientRequest::TryLock {
                ns,
                key,
                lease_ms,
                wait_ms,
                level,
            } => Self::TryLock {
                ns: ns.clone(),
                key: key.clone(),
                lease_ms: *lease_ms,
                wait_ms: *wait_ms,
                level: *level,
            },
            ClientRequest::Unlock { ns, key, fence } => Self::Unlock {
                ns: ns.clone(),
                key: key.clone(),
                fence: *fence,
            },
            ClientRequest::RenewLockLease {
                ns,
                key,
                fence,
                lease_ms,
            } => Self::RenewLockLease {
                ns: ns.clone(),
                key: key.clone(),
                fence: *fence,
                lease_ms: *lease_ms,
            },
            ClientRequest::ForceUnlock { ns, key } => Self::ForceUnlock {
                ns: ns.clone(),
                key: key.clone(),
            },
            ClientRequest::GetLockOwnership { ns, key } => Self::GetLockOwnership {
                ns: ns.clone(),
                key: key.clone(),
            },
            ClientRequest::CompareAndSet {
                ns,
                key,
                expected,
                new_value,
                level,
            } => Self::CompareAndSet {
                ns: ns.clone(),
                key: key.clone(),
                expected: expected.clone(),
                new_value: new_value.clone(),
                level: *level,
            },
            ClientRequest::RemoveIfValue {
                ns,
                key,
                expected,
                level,
            } => Self::RemoveIfValue {
                ns: ns.clone(),
                key: key.clone(),
                expected: expected.clone(),
                level: *level,
            },
            ClientRequest::Expire { .. } => return Err(unsupported_for_version("expire", 2)),
            ClientRequest::Persist { .. } => return Err(unsupported_for_version("persist", 2)),
            ClientRequest::GetTtl { .. } => return Err(unsupported_for_version("get_ttl", 2)),
            ClientRequest::ConditionalPut { .. } => {
                return Err(unsupported_for_version("conditional_put", 2));
            }
            ClientRequest::CompareValueAndInvalidate { .. } => {
                return Err(unsupported_for_version("compare_value_and_invalidate", 2));
            }
            ClientRequest::CompareValueAndExpire { .. } => {
                return Err(unsupported_for_version("compare_value_and_expire", 2));
            }
        })
    }
}

impl TryFrom<&ClientRequest> for RequestV3 {
    type Error = ClientProtocolError;

    fn try_from(request: &ClientRequest) -> Result<Self, Self::Error> {
        Ok(match request {
            ClientRequest::Get { ns, key } => Self::Get {
                ns: ns.clone(),
                key: key.clone(),
            },
            ClientRequest::Put {
                ns,
                key,
                value,
                ttl_ms,
                dimensions,
            } => Self::Put {
                ns: ns.clone(),
                key: key.clone(),
                value: value.clone(),
                ttl_ms: *ttl_ms,
                dimensions: dimensions.clone(),
            },
            ClientRequest::Invalidate { ns, key } => Self::Invalidate {
                ns: ns.clone(),
                key: key.clone(),
            },
            ClientRequest::BatchGet { ns, keys } => Self::BatchGet {
                ns: ns.clone(),
                keys: keys.clone(),
            },
            ClientRequest::BatchPut { ns, entries } => Self::BatchPut {
                ns: ns.clone(),
                entries: entries.clone(),
            },
            ClientRequest::Expire { ns, key, ttl_ms } => Self::Expire {
                ns: ns.clone(),
                key: key.clone(),
                ttl_ms: *ttl_ms,
            },
            ClientRequest::Persist { ns, key } => Self::Persist {
                ns: ns.clone(),
                key: key.clone(),
            },
            ClientRequest::GetTtl { ns, key } => Self::GetTtl {
                ns: ns.clone(),
                key: key.clone(),
            },
            ClientRequest::EvictRegion { ns } => Self::EvictRegion { ns: ns.clone() },
            ClientRequest::SubscribeInvalidations {
                ns,
                region,
                from,
                include_value,
            } => Self::SubscribeInvalidations {
                ns: ns.clone(),
                region: region.clone(),
                from: *from,
                include_value: *include_value,
            },
            ClientRequest::SubscribeEntryEvents {
                ns,
                region,
                from,
                include_value,
                projection,
            } => Self::SubscribeEntryEvents {
                ns: ns.clone(),
                region: region.clone(),
                from: *from,
                include_value: *include_value,
                projection: *projection,
            },
            ClientRequest::TryLock {
                ns,
                key,
                lease_ms,
                wait_ms,
                level,
            } => Self::TryLock {
                ns: ns.clone(),
                key: key.clone(),
                lease_ms: *lease_ms,
                wait_ms: *wait_ms,
                level: *level,
            },
            ClientRequest::Unlock { ns, key, fence } => Self::Unlock {
                ns: ns.clone(),
                key: key.clone(),
                fence: *fence,
            },
            ClientRequest::RenewLockLease {
                ns,
                key,
                fence,
                lease_ms,
            } => Self::RenewLockLease {
                ns: ns.clone(),
                key: key.clone(),
                fence: *fence,
                lease_ms: *lease_ms,
            },
            ClientRequest::ForceUnlock { ns, key } => Self::ForceUnlock {
                ns: ns.clone(),
                key: key.clone(),
            },
            ClientRequest::GetLockOwnership { ns, key } => Self::GetLockOwnership {
                ns: ns.clone(),
                key: key.clone(),
            },
            ClientRequest::CompareAndSet {
                ns,
                key,
                expected,
                new_value,
                level,
            } => Self::CompareAndSet {
                ns: ns.clone(),
                key: key.clone(),
                expected: expected.clone(),
                new_value: new_value.clone(),
                level: *level,
            },
            ClientRequest::RemoveIfValue {
                ns,
                key,
                expected,
                level,
            } => Self::RemoveIfValue {
                ns: ns.clone(),
                key: key.clone(),
                expected: expected.clone(),
                level: *level,
            },
            ClientRequest::ConditionalPut { .. } => {
                return Err(unsupported_for_version("conditional_put", 3));
            }
            ClientRequest::CompareValueAndInvalidate { .. } => {
                return Err(unsupported_for_version("compare_value_and_invalidate", 3));
            }
            ClientRequest::CompareValueAndExpire { .. } => {
                return Err(unsupported_for_version("compare_value_and_expire", 3));
            }
        })
    }
}

impl From<RequestV2> for ClientRequest {
    fn from(request: RequestV2) -> Self {
        match request {
            RequestV2::Get { ns, key } => Self::Get { ns, key },
            RequestV2::Put {
                ns,
                key,
                value,
                ttl_ms,
                dimensions,
            } => Self::Put {
                ns,
                key,
                value,
                ttl_ms,
                dimensions,
            },
            RequestV2::Invalidate { ns, key } => Self::Invalidate { ns, key },
            RequestV2::BatchGet { ns, keys } => Self::BatchGet { ns, keys },
            RequestV2::BatchPut { ns, entries } => Self::BatchPut { ns, entries },
            RequestV2::EvictRegion { ns } => Self::EvictRegion { ns },
            RequestV2::SubscribeInvalidations {
                ns,
                region,
                from,
                include_value,
            } => Self::SubscribeInvalidations {
                ns,
                region,
                from,
                include_value,
            },
            RequestV2::SubscribeEntryEvents {
                ns,
                region,
                from,
                include_value,
                projection,
            } => Self::SubscribeEntryEvents {
                ns,
                region,
                from,
                include_value,
                projection,
            },
            RequestV2::TryLock {
                ns,
                key,
                lease_ms,
                wait_ms,
                level,
            } => Self::TryLock {
                ns,
                key,
                lease_ms,
                wait_ms,
                level,
            },
            RequestV2::Unlock { ns, key, fence } => Self::Unlock { ns, key, fence },
            RequestV2::RenewLockLease {
                ns,
                key,
                fence,
                lease_ms,
            } => Self::RenewLockLease {
                ns,
                key,
                fence,
                lease_ms,
            },
            RequestV2::ForceUnlock { ns, key } => Self::ForceUnlock { ns, key },
            RequestV2::GetLockOwnership { ns, key } => Self::GetLockOwnership { ns, key },
            RequestV2::CompareAndSet {
                ns,
                key,
                expected,
                new_value,
                level,
            } => Self::CompareAndSet {
                ns,
                key,
                expected,
                new_value,
                level,
            },
            RequestV2::RemoveIfValue {
                ns,
                key,
                expected,
                level,
            } => Self::RemoveIfValue {
                ns,
                key,
                expected,
                level,
            },
        }
    }
}

impl From<RequestV3> for ClientRequest {
    fn from(request: RequestV3) -> Self {
        match request {
            RequestV3::Get { ns, key } => Self::Get { ns, key },
            RequestV3::Put {
                ns,
                key,
                value,
                ttl_ms,
                dimensions,
            } => Self::Put {
                ns,
                key,
                value,
                ttl_ms,
                dimensions,
            },
            RequestV3::Invalidate { ns, key } => Self::Invalidate { ns, key },
            RequestV3::BatchGet { ns, keys } => Self::BatchGet { ns, keys },
            RequestV3::BatchPut { ns, entries } => Self::BatchPut { ns, entries },
            RequestV3::Expire { ns, key, ttl_ms } => Self::Expire { ns, key, ttl_ms },
            RequestV3::Persist { ns, key } => Self::Persist { ns, key },
            RequestV3::GetTtl { ns, key } => Self::GetTtl { ns, key },
            RequestV3::EvictRegion { ns } => Self::EvictRegion { ns },
            RequestV3::SubscribeInvalidations {
                ns,
                region,
                from,
                include_value,
            } => Self::SubscribeInvalidations {
                ns,
                region,
                from,
                include_value,
            },
            RequestV3::SubscribeEntryEvents {
                ns,
                region,
                from,
                include_value,
                projection,
            } => Self::SubscribeEntryEvents {
                ns,
                region,
                from,
                include_value,
                projection,
            },
            RequestV3::TryLock {
                ns,
                key,
                lease_ms,
                wait_ms,
                level,
            } => Self::TryLock {
                ns,
                key,
                lease_ms,
                wait_ms,
                level,
            },
            RequestV3::Unlock { ns, key, fence } => Self::Unlock { ns, key, fence },
            RequestV3::RenewLockLease {
                ns,
                key,
                fence,
                lease_ms,
            } => Self::RenewLockLease {
                ns,
                key,
                fence,
                lease_ms,
            },
            RequestV3::ForceUnlock { ns, key } => Self::ForceUnlock { ns, key },
            RequestV3::GetLockOwnership { ns, key } => Self::GetLockOwnership { ns, key },
            RequestV3::CompareAndSet {
                ns,
                key,
                expected,
                new_value,
                level,
            } => Self::CompareAndSet {
                ns,
                key,
                expected,
                new_value,
                level,
            },
            RequestV3::RemoveIfValue {
                ns,
                key,
                expected,
                level,
            } => Self::RemoveIfValue {
                ns,
                key,
                expected,
                level,
            },
        }
    }
}

impl TryFrom<&ClientResponse> for ResponseV2 {
    type Error = ClientProtocolError;

    fn try_from(response: &ClientResponse) -> Result<Self, Self::Error> {
        Ok(match response {
            ClientResponse::Value { value } => Self::Value {
                value: value.clone(),
            },
            ClientResponse::Stored => Self::Stored,
            ClientResponse::Invalidated => Self::Invalidated,
            ClientResponse::Batch { items } => Self::Batch {
                items: items.clone(),
            },
            ClientResponse::Evicted => Self::Evicted,
            ClientResponse::Subscribed { from } => Self::Subscribed { from: *from },
            ClientResponse::LockAcquired { fence } => Self::LockAcquired { fence: *fence },
            ClientResponse::LockBusy => Self::LockBusy,
            ClientResponse::LockReleased => Self::LockReleased,
            ClientResponse::LockLeaseRenewed => Self::LockLeaseRenewed,
            ClientResponse::LockOwnership { fence, locked } => Self::LockOwnership {
                fence: *fence,
                locked: *locked,
            },
            ClientResponse::CasApplied { new_version } => Self::CasApplied {
                new_version: *new_version,
            },
            ClientResponse::CasMismatch { current } => Self::CasMismatch {
                current: current.clone(),
            },
            ClientResponse::Expiry { .. } => return Err(unsupported_for_version("expiry", 2)),
            ClientResponse::Ttl { .. } => return Err(unsupported_for_version("ttl", 2)),
            ClientResponse::ConditionalStored { .. } => {
                return Err(unsupported_for_version("conditional_stored", 2));
            }
            ClientResponse::CompareValueApplied { .. } => {
                return Err(unsupported_for_version("compare_value_applied", 2));
            }
        })
    }
}

impl TryFrom<&ClientResponse> for ResponseV3 {
    type Error = ClientProtocolError;

    fn try_from(response: &ClientResponse) -> Result<Self, Self::Error> {
        Ok(match response {
            ClientResponse::Value { value } => Self::Value {
                value: value.clone(),
            },
            ClientResponse::Stored => Self::Stored,
            ClientResponse::Invalidated => Self::Invalidated,
            ClientResponse::Batch { items } => Self::Batch {
                items: items.clone(),
            },
            ClientResponse::Expiry { applied } => Self::Expiry { applied: *applied },
            ClientResponse::Ttl { state } => Self::Ttl { state: *state },
            ClientResponse::Evicted => Self::Evicted,
            ClientResponse::Subscribed { from } => Self::Subscribed { from: *from },
            ClientResponse::LockAcquired { fence } => Self::LockAcquired { fence: *fence },
            ClientResponse::LockBusy => Self::LockBusy,
            ClientResponse::LockReleased => Self::LockReleased,
            ClientResponse::LockLeaseRenewed => Self::LockLeaseRenewed,
            ClientResponse::LockOwnership { fence, locked } => Self::LockOwnership {
                fence: *fence,
                locked: *locked,
            },
            ClientResponse::CasApplied { new_version } => Self::CasApplied {
                new_version: *new_version,
            },
            ClientResponse::CasMismatch { current } => Self::CasMismatch {
                current: current.clone(),
            },
            ClientResponse::ConditionalStored { .. } => {
                return Err(unsupported_for_version("conditional_stored", 3));
            }
            ClientResponse::CompareValueApplied { .. } => {
                return Err(unsupported_for_version("compare_value_applied", 3));
            }
        })
    }
}

impl From<ResponseV2> for ClientResponse {
    fn from(response: ResponseV2) -> Self {
        match response {
            ResponseV2::Value { value } => Self::Value { value },
            ResponseV2::Stored => Self::Stored,
            ResponseV2::Invalidated => Self::Invalidated,
            ResponseV2::Batch { items } => Self::Batch { items },
            ResponseV2::Evicted => Self::Evicted,
            ResponseV2::Subscribed { from } => Self::Subscribed { from },
            ResponseV2::LockAcquired { fence } => Self::LockAcquired { fence },
            ResponseV2::LockBusy => Self::LockBusy,
            ResponseV2::LockReleased => Self::LockReleased,
            ResponseV2::LockLeaseRenewed => Self::LockLeaseRenewed,
            ResponseV2::LockOwnership { fence, locked } => Self::LockOwnership { fence, locked },
            ResponseV2::CasApplied { new_version } => Self::CasApplied { new_version },
            ResponseV2::CasMismatch { current } => Self::CasMismatch { current },
        }
    }
}

impl From<ResponseV3> for ClientResponse {
    fn from(response: ResponseV3) -> Self {
        match response {
            ResponseV3::Value { value } => Self::Value { value },
            ResponseV3::Stored => Self::Stored,
            ResponseV3::Invalidated => Self::Invalidated,
            ResponseV3::Batch { items } => Self::Batch { items },
            ResponseV3::Expiry { applied } => Self::Expiry { applied },
            ResponseV3::Ttl { state } => Self::Ttl { state },
            ResponseV3::Evicted => Self::Evicted,
            ResponseV3::Subscribed { from } => Self::Subscribed { from },
            ResponseV3::LockAcquired { fence } => Self::LockAcquired { fence },
            ResponseV3::LockBusy => Self::LockBusy,
            ResponseV3::LockReleased => Self::LockReleased,
            ResponseV3::LockLeaseRenewed => Self::LockLeaseRenewed,
            ResponseV3::LockOwnership { fence, locked } => Self::LockOwnership { fence, locked },
            ResponseV3::CasApplied { new_version } => Self::CasApplied { new_version },
            ResponseV3::CasMismatch { current } => Self::CasMismatch { current },
        }
    }
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
    /// The frame header and the version carried by its message envelope disagree.
    #[error(
        "client protocol version mismatch: frame uses v{frame_version}, message uses v{message_version}"
    )]
    VersionMismatch {
        /// Version encoded in the outer frame header.
        frame_version: u16,
        /// Version encoded in the inner handshake or request/response envelope.
        message_version: u16,
    },
    /// A message variant did not exist in the requested historical wire version.
    #[error("client protocol operation {operation} is unavailable in wire version {version}")]
    UnsupportedMessageForVersion {
        /// Stable operation name used for diagnostics without serializing payload data.
        operation: &'static str,
        /// Historical wire version requested by the caller.
        version: u16,
    },
    /// Payload codec failed.
    #[error("client protocol codec error: {0}")]
    Codec(String),
    /// Required field is invalid.
    #[error("invalid client protocol field: {0}")]
    InvalidField(&'static str),
}
