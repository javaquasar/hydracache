//! Non-production HC/2 transport-selection spikes.
//!
//! This crate owns the transport-neutral, sans-I/O semantic harness used by
//! release 0.68 W0. It deliberately does not expose a production listener.
//! Passing these checks is necessary, but it is not evidence of real TLS,
//! HTTP/2, gRPC, cross-language generation, or daemon interoperability.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::marker::PhantomData;
use std::num::NonZeroU64;
use std::sync::{Arc, Mutex};

use thiserror::Error;

pub mod contract;
pub mod endpoint;
pub mod fault_proxy;
pub mod policy;
pub mod security;

/// First proposed HC/2 wire generation. It is isolated from HC/1 v1-v4.
pub const HC2_GENERATION: u16 = 5;
const HEADER_BYTES: usize = 4 + 4 + 2 + 8 + 1 + 8 + 4;

/// Monotonic connection generation within one authenticated client identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConnectionGeneration(NonZeroU64);

impl ConnectionGeneration {
    /// First generation for a newly created, unique client identity.
    pub const FIRST: Self = Self(NonZeroU64::MIN);

    /// Restore a persisted nonzero generation for a stable client identity.
    pub fn new(value: u64) -> Result<Self, SpikeError> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(SpikeError::InvalidConnectionGeneration)
    }

    /// Return the wire value.
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    /// Advance for reconnect. Exhaustion is fail-closed instead of wrapping.
    pub fn checked_next(self) -> Result<Self, SpikeError> {
        let next = self
            .get()
            .checked_add(1)
            .and_then(NonZeroU64::new)
            .ok_or(SpikeError::ConnectionGenerationExhausted)?;
        Ok(Self(next))
    }
}

/// Transport candidates required by the 0.68 W0 bake-off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransportCandidate {
    /// Dedicated TLS socket with HydraCache-owned framing.
    DedicatedTcpTls,
    /// Bidirectional HTTP/2 stream with generated messages.
    Http2Bidirectional,
    /// Bidirectional gRPC stream with protobuf messages.
    GrpcBidirectional,
}

impl TransportCandidate {
    /// Stable candidate list used by the common harness.
    pub const ALL: [Self; 3] = [
        Self::DedicatedTcpTls,
        Self::Http2Bidirectional,
        Self::GrpcBidirectional,
    ];

    fn magic(self) -> [u8; 4] {
        match self {
            Self::DedicatedTcpTls => *b"H2TC",
            Self::Http2Bidirectional => *b"H2HT",
            Self::GrpcBidirectional => *b"H2GR",
        }
    }
}

/// HC/2 semantic frame classes shared by every transport spike.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameKind {
    /// Connection negotiation.
    Handshake = 1,
    /// Ordinary client invocation.
    Invocation = 2,
    /// Correlated server reply.
    Reply = 3,
    /// Connection liveness control.
    Heartbeat = 4,
    /// Server-owned subscription registration.
    Subscribe = 5,
    /// Server-pushed cache signal.
    Event = 6,
    /// Explicit event loss/repair signal.
    Gap = 7,
    /// Invocation cancellation.
    Cancel = 8,
}

impl TryFrom<u8> for FrameKind {
    type Error = SpikeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Handshake),
            2 => Ok(Self::Invocation),
            3 => Ok(Self::Reply),
            4 => Ok(Self::Heartbeat),
            5 => Ok(Self::Subscribe),
            6 => Ok(Self::Event),
            7 => Ok(Self::Gap),
            8 => Ok(Self::Cancel),
            other => Err(SpikeError::UnknownMessageKind(other)),
        }
    }
}

/// Transport-neutral semantic envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpikeFrame {
    /// Protocol generation, distinct from HC/1 operation versions.
    pub generation: u16,
    /// Connection generation fencing correlation and subscription ownership.
    pub connection_generation: ConnectionGeneration,
    /// Message class.
    pub kind: FrameKind,
    /// Invocation or subscription correlation id; zero for connection control.
    pub correlation_id: u64,
    /// Bounded opaque spike payload.
    pub payload: Vec<u8>,
}

impl SpikeFrame {
    /// Construct a current-generation frame.
    pub fn current(
        connection_generation: ConnectionGeneration,
        kind: FrameKind,
        correlation_id: u64,
        payload: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            generation: HC2_GENERATION,
            connection_generation,
            kind,
            correlation_id,
            payload: payload.into(),
        }
    }

    /// Bytes retained by the bounded spike codec for this frame.
    pub fn retained_bytes(&self) -> usize {
        HEADER_BYTES + self.payload.len()
    }
}

/// Bounded connection resources used by the common harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpikeLimits {
    /// Maximum complete encoded frame.
    pub max_frame_bytes: usize,
    /// Maximum concurrent invocations.
    pub max_pending_invocations: usize,
    /// Maximum queued ordinary replies.
    pub max_reply_frames: usize,
    /// Maximum retained bytes across queued ordinary replies.
    pub max_reply_bytes: usize,
    /// Maximum queued event frames before an explicit gap replaces them.
    pub max_event_frames: usize,
    /// Maximum retained bytes across queued events or the explicit gap.
    pub max_event_bytes: usize,
    /// Maximum retained bytes in the control lane.
    pub max_control_bytes: usize,
    /// Maximum connection-owned subscriptions.
    pub max_subscriptions: usize,
    /// Maximum items accepted in one decoded batch.
    pub max_batch_items: usize,
    /// Maximum retained retry records.
    pub max_retry_slots: usize,
    /// Maximum retained reconnect records.
    pub max_reconnect_slots: usize,
    /// Maximum retained deadline registrations.
    pub max_deadlines: usize,
    /// Maximum retained topology nodes and their encoded metadata bytes.
    pub max_topology_nodes: usize,
    pub max_topology_bytes: usize,
    /// Maximum retained sessions and their opaque state bytes.
    pub max_sessions: usize,
    pub max_session_bytes: usize,
}

impl Default for SpikeLimits {
    fn default() -> Self {
        Self {
            max_frame_bytes: 64 * 1024,
            max_pending_invocations: 256,
            max_reply_frames: 256,
            max_reply_bytes: 4 * 1024 * 1024,
            max_event_frames: 32,
            max_event_bytes: 512 * 1024,
            max_control_bytes: 64 * 1024,
            max_subscriptions: 32,
            max_batch_items: 256,
            max_retry_slots: 64,
            max_reconnect_slots: 8,
            max_deadlines: 256,
            max_topology_nodes: 256,
            max_topology_bytes: 1024 * 1024,
            max_sessions: 32,
            max_session_bytes: 256 * 1024,
        }
    }
}

impl SpikeLimits {
    fn validate(self) -> Result<(), SpikeError> {
        let nonzero = [
            ("max_frame_bytes", self.max_frame_bytes),
            ("max_pending_invocations", self.max_pending_invocations),
            ("max_reply_frames", self.max_reply_frames),
            ("max_reply_bytes", self.max_reply_bytes),
            ("max_event_frames", self.max_event_frames),
            ("max_event_bytes", self.max_event_bytes),
            ("max_control_bytes", self.max_control_bytes),
            ("max_subscriptions", self.max_subscriptions),
            ("max_batch_items", self.max_batch_items),
            ("max_retry_slots", self.max_retry_slots),
            ("max_reconnect_slots", self.max_reconnect_slots),
            ("max_deadlines", self.max_deadlines),
            ("max_topology_nodes", self.max_topology_nodes),
            ("max_topology_bytes", self.max_topology_bytes),
            ("max_sessions", self.max_sessions),
            ("max_session_bytes", self.max_session_bytes),
        ];
        if let Some((name, _)) = nonzero.into_iter().find(|(_, value)| *value == 0) {
            return Err(SpikeError::InvalidLimit(name));
        }
        if self.max_frame_bytes < HEADER_BYTES
            || self.max_reply_bytes < HEADER_BYTES
            || self.max_event_bytes < HEADER_BYTES + b"event_queue_overflow".len()
            || self.max_control_bytes < HEADER_BYTES
        {
            return Err(SpikeError::InvalidLimit("byte_budget_below_frame_floor"));
        }
        Ok(())
    }
}

/// Authenticated peer context supplied by the candidate TLS boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerIdentity {
    /// Bounded client identity.
    pub client_id: String,
    /// Bounded tenant identity.
    pub tenant: String,
    /// Whether the spike adapter verified its TLS/mTLS peer before negotiation.
    pub tls_verified: bool,
}

/// Explicit connection lifecycle. Dispatch is legal only in `Ready`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Created,
    TlsVerified,
    Authenticated,
    Ready,
    Draining,
    Closed,
}

/// Initial HC/2 bootstrap state.
#[derive(Debug)]
pub struct Created;

/// Transport authentication completed successfully.
#[derive(Debug)]
pub struct TlsVerified;

/// Peer identity was derived and validated from the authenticated channel.
#[derive(Debug)]
pub struct Authenticated;

/// The authenticated peer passed the configured application authorization policy.
///
/// ```compile_fail
/// use hydracache_client_plane_spike::{
///     BootstrapConnection, ConnectionGeneration, PeerIdentity, SpikeLimits,
///     TransportCandidate, HC2_GENERATION,
/// };
/// let authenticated = BootstrapConnection::new(
///     TransportCandidate::GrpcBidirectional,
///     ConnectionGeneration::FIRST,
///     SpikeLimits::default(),
/// )
/// .verify_tls(true).unwrap()
/// .authenticate(PeerIdentity::verified("client", "tenant")).unwrap();
/// let _ = authenticated.negotiate(HC2_GENERATION);
/// ```
#[derive(Debug)]
pub struct Authorized;

/// Bounded exact-match authorization policy for authenticated client identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientAuthorizationPolicy {
    allowed: BTreeSet<(String, String)>,
}

impl ClientAuthorizationPolicy {
    /// Build a bounded allow-list. Empty policies are valid and deny every peer.
    pub fn new<I, C, T>(rules: I) -> Result<Self, SpikeError>
    where
        I: IntoIterator<Item = (C, T)>,
        C: Into<String>,
        T: Into<String>,
    {
        let mut allowed = BTreeSet::new();
        for (client_id, tenant) in rules {
            let client_id = client_id.into();
            let tenant = tenant.into();
            if client_id.trim().is_empty()
                || tenant.trim().is_empty()
                || client_id.len() > 128
                || tenant.len() > 128
            {
                return Err(SpikeError::InvalidAuthorizationPolicy);
            }
            allowed.insert((client_id, tenant));
            if allowed.len() > 256 {
                return Err(SpikeError::InvalidAuthorizationPolicy);
            }
        }
        Ok(Self { allowed })
    }

    /// Build the narrow policy used by one adapter test identity.
    pub fn exact(
        client_id: impl Into<String>,
        tenant: impl Into<String>,
    ) -> Result<Self, SpikeError> {
        Self::new([(client_id, tenant)])
    }

    fn permits(&self, identity: &PeerIdentity) -> bool {
        self.allowed.iter().any(|(client_id, tenant)| {
            client_id == &identity.client_id && tenant == &identity.tenant
        })
    }
}

/// Linear bootstrap owner. Only methods valid for `State` are available.
///
/// ```compile_fail
/// use hydracache_client_plane_spike::{
///     BootstrapConnection, Created, SpikeFrame, SpikeLimits, TransportCandidate,
/// };
/// let mut connection: BootstrapConnection<Created> =
///     BootstrapConnection::new(
///         TransportCandidate::GrpcBidirectional,
///         ConnectionGeneration::FIRST,
///         SpikeLimits::default(),
///     );
/// let _ = connection.receive(&[]);
/// ```
///
/// ```compile_fail
/// use hydracache_client_plane_spike::{
///     BootstrapConnection, Created, SpikeLimits, TransportCandidate, HC2_GENERATION,
/// };
/// let connection: BootstrapConnection<Created> =
///     BootstrapConnection::new(
///         TransportCandidate::GrpcBidirectional,
///         ConnectionGeneration::FIRST,
///         SpikeLimits::default(),
///     );
/// let _ = connection.negotiate(HC2_GENERATION);
/// ```
#[derive(Debug)]
pub struct BootstrapConnection<State> {
    inner: SpikeConnection,
    state: PhantomData<State>,
}

impl BootstrapConnection<Created> {
    /// Allocate a disconnected bootstrap owner with bounded resources.
    pub fn new(
        candidate: TransportCandidate,
        connection_generation: ConnectionGeneration,
        limits: SpikeLimits,
    ) -> Self {
        Self {
            inner: SpikeConnection::new(candidate, connection_generation, limits),
            state: PhantomData,
        }
    }

    /// Consume the created state after the adapter's TLS verifier completes.
    pub fn verify_tls(
        mut self,
        verified: bool,
    ) -> Result<BootstrapConnection<TlsVerified>, SpikeError> {
        self.inner.mark_tls_verified(verified)?;
        Ok(BootstrapConnection {
            inner: self.inner,
            state: PhantomData,
        })
    }
}

impl BootstrapConnection<TlsVerified> {
    /// Consume the TLS state after deriving a bounded peer identity.
    pub fn authenticate(
        mut self,
        identity: PeerIdentity,
    ) -> Result<BootstrapConnection<Authenticated>, SpikeError> {
        self.inner.authenticate(identity)?;
        Ok(BootstrapConnection {
            inner: self.inner,
            state: PhantomData,
        })
    }
}

impl BootstrapConnection<Authenticated> {
    /// Consume the authenticated state only when the application policy permits it.
    pub fn authorize(
        self,
        policy: &ClientAuthorizationPolicy,
    ) -> Result<BootstrapConnection<Authorized>, SpikeError> {
        let identity = self
            .inner
            .identity
            .as_ref()
            .ok_or(SpikeError::UnverifiedIdentity)?;
        if !policy.permits(identity) {
            return Err(SpikeError::UnauthorizedIdentity);
        }
        Ok(BootstrapConnection {
            inner: self.inner,
            state: PhantomData,
        })
    }
}

impl BootstrapConnection<Authorized> {
    /// Negotiate HC/2 and return the object-safe ready/draining runtime.
    pub fn negotiate(mut self, generation: u16) -> Result<SpikeConnection, SpikeError> {
        self.inner.negotiate(generation)?;
        Ok(self.inner)
    }
}

impl PeerIdentity {
    /// Build an identity for the semantic harness.
    pub fn verified(client_id: impl Into<String>, tenant: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            tenant: tenant.into(),
            tls_verified: true,
        }
    }
}

/// Validated per-identity, per-tenant, and process-global connection admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionLimits {
    pub max_per_identity: usize,
    pub max_per_tenant: usize,
    pub max_global: usize,
}

impl Default for AdmissionLimits {
    fn default() -> Self {
        Self {
            max_per_identity: 4,
            max_per_tenant: 64,
            max_global: 1024,
        }
    }
}

#[derive(Debug, Default)]
struct AdmissionState {
    global: usize,
    identities: BTreeMap<String, usize>,
    tenants: BTreeMap<String, usize>,
}

/// Shared atomic admission ledger. A returned permit owns exactly one slot.
#[derive(Debug, Clone)]
pub struct AdmissionController {
    limits: AdmissionLimits,
    state: Arc<Mutex<AdmissionState>>,
}

impl AdmissionController {
    /// Reject zero or internally contradictory admission limits.
    pub fn new(limits: AdmissionLimits) -> Result<Self, SpikeError> {
        if limits.max_per_identity == 0
            || limits.max_per_tenant == 0
            || limits.max_global == 0
            || limits.max_per_identity > limits.max_per_tenant
            || limits.max_per_tenant > limits.max_global
        {
            return Err(SpikeError::InvalidLimit("admission"));
        }
        Ok(Self {
            limits,
            state: Arc::new(Mutex::new(AdmissionState::default())),
        })
    }

    /// Atomically acquire all three scopes or mutate none of them.
    pub fn admit(&self, identity: &PeerIdentity) -> Result<AdmissionPermit, SpikeError> {
        if !identity.tls_verified || identity.client_id.is_empty() || identity.tenant.is_empty() {
            return Err(SpikeError::UnverifiedIdentity);
        }
        let mut state = self.state.lock().expect("admission ledger poisoned");
        let identity_count = state
            .identities
            .get(&identity.client_id)
            .copied()
            .unwrap_or_default();
        let tenant_count = state
            .tenants
            .get(&identity.tenant)
            .copied()
            .unwrap_or_default();
        if identity_count >= self.limits.max_per_identity {
            return Err(SpikeError::ResourceLimit("identity_connections"));
        }
        if tenant_count >= self.limits.max_per_tenant {
            return Err(SpikeError::ResourceLimit("tenant_connections"));
        }
        if state.global >= self.limits.max_global {
            return Err(SpikeError::ResourceLimit("global_connections"));
        }
        state.global += 1;
        *state
            .identities
            .entry(identity.client_id.clone())
            .or_default() += 1;
        *state.tenants.entry(identity.tenant.clone()).or_default() += 1;
        Ok(AdmissionPermit {
            state: Arc::clone(&self.state),
            client_id: identity.client_id.clone(),
            tenant: identity.tenant.clone(),
        })
    }

    /// Privacy-safe totals; identity and tenant names are not exposed.
    pub fn snapshot(&self) -> AdmissionSnapshot {
        let state = self.state.lock().expect("admission ledger poisoned");
        AdmissionSnapshot {
            global_connections: state.global,
            active_identities: state.identities.len(),
            active_tenants: state.tenants.len(),
        }
    }
}

/// Privacy-safe admission totals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AdmissionSnapshot {
    pub global_connections: usize,
    pub active_identities: usize,
    pub active_tenants: usize,
}

/// RAII ownership of one admitted connection.
#[derive(Debug)]
pub struct AdmissionPermit {
    state: Arc<Mutex<AdmissionState>>,
    client_id: String,
    tenant: String,
}

impl Drop for AdmissionPermit {
    fn drop(&mut self) {
        let mut state = self.state.lock().expect("admission ledger poisoned");
        state.global -= 1;
        decrement_or_remove(&mut state.identities, &self.client_id);
        decrement_or_remove(&mut state.tenants, &self.tenant);
    }
}

fn decrement_or_remove(counts: &mut BTreeMap<String, usize>, key: &str) {
    if let Some(count) = counts.get_mut(key) {
        *count -= 1;
        if *count == 0 {
            counts.remove(key);
        }
    }
}

/// Fail-loud W0 spike errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SpikeError {
    /// Candidate preface did not match the selected adapter.
    #[error("candidate framing preface mismatch")]
    CandidateMismatch,
    /// The frame ended before every declared field was present.
    #[error("truncated frame")]
    Truncated,
    /// The declared and actual frame lengths differ.
    #[error("frame length mismatch")]
    LengthMismatch,
    /// The complete frame exceeds the configured bound.
    #[error("frame exceeds configured limit")]
    Oversized,
    /// A future or legacy generation reached the HC/2 boundary.
    #[error("unsupported HC/2 generation {0}")]
    UnknownGeneration(u16),
    /// The message class is unknown.
    #[error("unknown HC/2 message kind {0}")]
    UnknownMessageKind(u8),
    /// Dispatch was attempted before a verified identity negotiated HC/2.
    #[error("connection is not authenticated and negotiated")]
    NotReady,
    /// The peer identity is absent, unverified, or unbounded.
    #[error("peer identity was not verified before negotiation")]
    UnverifiedIdentity,
    /// The authenticated peer is not allowed by the application policy.
    #[error("authenticated peer is not authorized")]
    UnauthorizedIdentity,
    /// The authorization policy contains an empty, oversized, or excessive rule.
    #[error("authorization policy is invalid or exceeds its bound")]
    InvalidAuthorizationPolicy,
    /// A connection-owned resource bound was reached.
    #[error("connection resource limit reached: {0}")]
    ResourceLimit(&'static str),
    /// A configured limit is zero or cannot hold its minimum control record.
    #[error("invalid nonzero resource limit: {0}")]
    InvalidLimit(&'static str),
    /// A correlation or subscription id is invalid or already active.
    #[error("duplicate or invalid correlation id")]
    InvalidCorrelation,
    /// A completion referred to no active invocation.
    #[error("unknown pending invocation")]
    UnknownInvocation,
    /// A message from another connection generation reached this runtime.
    #[error("stale connection generation {actual}; expected {expected}")]
    StaleConnectionGeneration { expected: u64, actual: u64 },
    /// A connection generation counter would wrap and become ambiguous.
    #[error("connection generation exhausted")]
    ConnectionGenerationExhausted,
    /// Zero cannot identify an owned connection generation.
    #[error("connection generation must be nonzero")]
    InvalidConnectionGeneration,
    /// The connection was already closed.
    #[error("connection is closed")]
    Closed,
    /// A lifecycle transition or operation was attempted from the wrong state.
    #[error("connection state {actual:?} does not permit {operation}")]
    InvalidState {
        operation: &'static str,
        actual: ConnectionState,
    },
}

/// Strict candidate-specific spike codec.
#[derive(Debug, Clone, Copy)]
pub struct SpikeCodec {
    candidate: TransportCandidate,
    max_frame_bytes: usize,
}

impl SpikeCodec {
    /// Create a codec for one candidate.
    pub fn new(candidate: TransportCandidate, max_frame_bytes: usize) -> Self {
        Self {
            candidate,
            max_frame_bytes,
        }
    }

    /// Encode one semantic envelope with an isolated candidate preface.
    pub fn encode(&self, frame: &SpikeFrame) -> Result<Vec<u8>, SpikeError> {
        if frame.generation != HC2_GENERATION {
            return Err(SpikeError::UnknownGeneration(frame.generation));
        }
        let total = HEADER_BYTES
            .checked_add(frame.payload.len())
            .ok_or(SpikeError::Oversized)?;
        if total > self.max_frame_bytes || total > u32::MAX as usize {
            return Err(SpikeError::Oversized);
        }
        let body_len = total - 8;
        let payload_len = u32::try_from(frame.payload.len()).map_err(|_| SpikeError::Oversized)?;
        let mut bytes = Vec::with_capacity(total);
        bytes.extend_from_slice(&self.candidate.magic());
        bytes.extend_from_slice(&(body_len as u32).to_be_bytes());
        bytes.extend_from_slice(&frame.generation.to_be_bytes());
        bytes.extend_from_slice(&frame.connection_generation.get().to_be_bytes());
        bytes.push(frame.kind as u8);
        bytes.extend_from_slice(&frame.correlation_id.to_be_bytes());
        bytes.extend_from_slice(&payload_len.to_be_bytes());
        bytes.extend_from_slice(&frame.payload);
        Ok(bytes)
    }

    /// Decode one exact frame. Trailing, truncated, oversized, unknown, and
    /// cross-candidate inputs are rejected before dispatch.
    pub fn decode(&self, bytes: &[u8]) -> Result<SpikeFrame, SpikeError> {
        if bytes.len() > self.max_frame_bytes {
            return Err(SpikeError::Oversized);
        }
        if bytes.len() < HEADER_BYTES {
            return Err(SpikeError::Truncated);
        }
        if bytes[..4] != self.candidate.magic() {
            return Err(SpikeError::CandidateMismatch);
        }
        let declared = u32::from_be_bytes(bytes[4..8].try_into().expect("fixed slice")) as usize;
        if declared.checked_add(8) != Some(bytes.len()) {
            return Err(SpikeError::LengthMismatch);
        }
        let generation = u16::from_be_bytes(bytes[8..10].try_into().expect("fixed slice"));
        if generation != HC2_GENERATION {
            return Err(SpikeError::UnknownGeneration(generation));
        }
        let connection_generation = NonZeroU64::new(u64::from_be_bytes(
            bytes[10..18].try_into().expect("fixed slice"),
        ))
        .map(ConnectionGeneration)
        .ok_or(SpikeError::StaleConnectionGeneration {
            expected: 1,
            actual: 0,
        })?;
        let kind = FrameKind::try_from(bytes[18])?;
        let correlation_id = u64::from_be_bytes(bytes[19..27].try_into().expect("fixed slice"));
        let payload_len =
            u32::from_be_bytes(bytes[27..31].try_into().expect("fixed slice")) as usize;
        if HEADER_BYTES.checked_add(payload_len) != Some(bytes.len()) {
            return Err(SpikeError::LengthMismatch);
        }
        Ok(SpikeFrame {
            generation,
            connection_generation,
            kind,
            correlation_id,
            payload: bytes[HEADER_BYTES..].to_vec(),
        })
    }
}

/// Accounting snapshot required after cancellation and disconnect tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResourceSnapshot {
    /// Active invocations awaiting exactly one completion.
    pub pending_invocations: usize,
    /// Active connection-owned subscription registrations.
    pub subscriptions: usize,
    /// Queued ordinary replies.
    pub reply_frames: usize,
    /// Retained encoded reply bytes.
    pub reply_bytes: usize,
    /// Queued event or explicit-gap frames.
    pub event_frames: usize,
    /// Retained encoded event/gap bytes.
    pub event_bytes: usize,
    /// Queued connection control frames.
    pub control_frames: usize,
    /// Retained encoded control bytes.
    pub control_bytes: usize,
    /// Retained retry and reconnect state.
    pub retry_slots: usize,
    pub reconnect_slots: usize,
    /// Retained deadlines.
    pub deadlines: usize,
    /// Retained topology state.
    pub topology_nodes: usize,
    pub topology_bytes: usize,
    /// Retained session state.
    pub sessions: usize,
    pub session_bytes: usize,
}

/// Privacy-safe monotonic counters; payloads, keys, and identities are absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConnectionMetrics {
    pub dispatched_frames: u64,
    pub rejected_frames: u64,
    pub resource_rejections: u64,
    pub cancelled_invocations: u64,
    pub event_gaps: u64,
    /// Frames refused because they belong to another connection generation.
    pub stale_generation_frames: u64,
}

/// Connection-owned semantic runtime shared by all W0 candidates.
#[derive(Debug)]
pub struct SpikeConnection {
    candidate: TransportCandidate,
    connection_generation: ConnectionGeneration,
    codec: SpikeCodec,
    limits: SpikeLimits,
    identity: Option<PeerIdentity>,
    state: ConnectionState,
    dispatch_count: u64,
    pending: BTreeSet<u64>,
    subscriptions: BTreeSet<u64>,
    replies: VecDeque<SpikeFrame>,
    events: VecDeque<SpikeFrame>,
    controls: VecDeque<SpikeFrame>,
    reply_bytes: usize,
    event_bytes: usize,
    control_bytes: usize,
    retry_slots: usize,
    reconnect_slots: usize,
    deadlines: BTreeSet<u64>,
    topology_nodes: usize,
    topology_bytes: usize,
    sessions: BTreeMap<u64, usize>,
    session_bytes: usize,
    next_lane: u8,
    metrics: ConnectionMetrics,
}

impl SpikeConnection {
    /// Create one disconnected candidate spike runtime.
    fn new(
        candidate: TransportCandidate,
        connection_generation: ConnectionGeneration,
        limits: SpikeLimits,
    ) -> Self {
        Self {
            candidate,
            connection_generation,
            codec: SpikeCodec::new(candidate, limits.max_frame_bytes),
            limits,
            identity: None,
            state: ConnectionState::Created,
            dispatch_count: 0,
            pending: BTreeSet::new(),
            subscriptions: BTreeSet::new(),
            replies: VecDeque::new(),
            events: VecDeque::new(),
            controls: VecDeque::new(),
            reply_bytes: 0,
            event_bytes: 0,
            control_bytes: 0,
            retry_slots: 0,
            reconnect_slots: 0,
            deadlines: BTreeSet::new(),
            topology_nodes: 0,
            topology_bytes: 0,
            sessions: BTreeMap::new(),
            session_bytes: 0,
            next_lane: 0,
            metrics: ConnectionMetrics::default(),
        }
    }

    /// Candidate under test.
    pub fn candidate(&self) -> TransportCandidate {
        self.candidate
    }

    /// Current explicit lifecycle state.
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Generation that owns every call, subscription, session, and frame.
    pub fn connection_generation(&self) -> ConnectionGeneration {
        self.connection_generation
    }

    /// Record successful transport-level TLS verification before identity use.
    fn mark_tls_verified(&mut self, verified: bool) -> Result<(), SpikeError> {
        self.ensure_state(ConnectionState::Created, "TLS verification")?;
        if !verified {
            return Err(SpikeError::UnverifiedIdentity);
        }
        self.state = ConnectionState::TlsVerified;
        Ok(())
    }

    /// Install a TLS-authenticated peer identity before negotiation.
    fn authenticate(&mut self, identity: PeerIdentity) -> Result<(), SpikeError> {
        self.ensure_state(ConnectionState::TlsVerified, "authentication")?;
        if !identity.tls_verified
            || identity.client_id.trim().is_empty()
            || identity.tenant.trim().is_empty()
            || identity.client_id.len() > 128
            || identity.tenant.len() > 128
        {
            return Err(SpikeError::UnverifiedIdentity);
        }
        self.identity = Some(identity);
        self.state = ConnectionState::Authenticated;
        Ok(())
    }

    /// Negotiate the isolated HC/2 generation after authentication.
    fn negotiate(&mut self, generation: u16) -> Result<(), SpikeError> {
        self.ensure_state(ConnectionState::Authenticated, "negotiation")?;
        self.limits.validate()?;
        if self.identity.is_none() {
            return Err(SpikeError::UnverifiedIdentity);
        }
        if generation != HC2_GENERATION {
            return Err(SpikeError::UnknownGeneration(generation));
        }
        self.state = ConnectionState::Ready;
        Ok(())
    }

    /// Stop accepting new work while allowing queued replies to drain.
    pub fn begin_draining(&mut self) -> Result<(), SpikeError> {
        self.ensure_state(ConnectionState::Ready, "draining")?;
        self.state = ConnectionState::Draining;
        Ok(())
    }

    /// Decode a candidate frame. Invalid generations never increment dispatch.
    pub fn receive(&mut self, bytes: &[u8]) -> Result<SpikeFrame, SpikeError> {
        self.ensure_ready()?;
        let frame = match self.codec.decode(bytes) {
            Ok(frame) => frame,
            Err(error) => {
                self.metrics.rejected_frames = self.metrics.rejected_frames.saturating_add(1);
                return Err(error);
            }
        };
        self.ensure_connection_generation(frame.connection_generation)?;
        self.dispatch_count = self.dispatch_count.saturating_add(1);
        self.metrics.dispatched_frames = self.metrics.dispatched_frames.saturating_add(1);
        Ok(frame)
    }

    /// Start one bounded invocation.
    pub fn begin_invocation(&mut self, correlation_id: u64) -> Result<(), SpikeError> {
        self.ensure_ready()?;
        if correlation_id == 0 || self.pending.contains(&correlation_id) {
            return Err(SpikeError::InvalidCorrelation);
        }
        if self.pending.len() >= self.limits.max_pending_invocations {
            self.metrics.resource_rejections = self.metrics.resource_rejections.saturating_add(1);
            return Err(SpikeError::ResourceLimit("pending_invocations"));
        }
        self.pending.insert(correlation_id);
        Ok(())
    }

    /// Complete one invocation exactly once and queue its reply.
    pub fn complete_invocation(
        &mut self,
        connection_generation: ConnectionGeneration,
        correlation_id: u64,
        payload: impl Into<Vec<u8>>,
    ) -> Result<(), SpikeError> {
        self.ensure_ready_or_draining("invocation completion")?;
        self.ensure_connection_generation(connection_generation)?;
        if !self.pending.contains(&correlation_id) {
            return Err(SpikeError::UnknownInvocation);
        }
        let payload = payload.into();
        let retained = HEADER_BYTES
            .checked_add(payload.len())
            .ok_or(SpikeError::ResourceLimit("reply_bytes"))?;
        if self.replies.len() >= self.limits.max_reply_frames
            || retained > self.limits.max_frame_bytes
            || self
                .reply_bytes
                .checked_add(retained)
                .is_none_or(|bytes| bytes > self.limits.max_reply_bytes)
        {
            self.metrics.resource_rejections = self.metrics.resource_rejections.saturating_add(1);
            return Err(SpikeError::ResourceLimit("reply_queue"));
        }
        self.pending.remove(&correlation_id);
        self.reply_bytes += retained;
        self.replies.push_back(SpikeFrame::current(
            self.connection_generation,
            FrameKind::Reply,
            correlation_id,
            payload,
        ));
        Ok(())
    }

    /// Cancel one invocation. Cancellation is idempotent for disconnect races.
    pub fn cancel_invocation(
        &mut self,
        connection_generation: ConnectionGeneration,
        correlation_id: u64,
    ) -> Result<bool, SpikeError> {
        self.ensure_open()?;
        self.ensure_connection_generation(connection_generation)?;
        let removed = self.pending.remove(&correlation_id);
        if removed {
            self.metrics.cancelled_invocations =
                self.metrics.cancelled_invocations.saturating_add(1);
        }
        Ok(removed)
    }

    /// Register a connection-owned subscription.
    pub fn register_subscription(&mut self, subscription_id: u64) -> Result<(), SpikeError> {
        self.ensure_ready()?;
        if subscription_id == 0 || self.subscriptions.contains(&subscription_id) {
            return Err(SpikeError::InvalidCorrelation);
        }
        if self.subscriptions.len() >= self.limits.max_subscriptions {
            self.metrics.resource_rejections = self.metrics.resource_rejections.saturating_add(1);
            return Err(SpikeError::ResourceLimit("subscriptions"));
        }
        self.subscriptions.insert(subscription_id);
        Ok(())
    }

    /// Push a bounded cache signal. On overflow all queued event history is
    /// replaced by one explicit gap requiring conservative repair.
    pub fn push_event(
        &mut self,
        connection_generation: ConnectionGeneration,
        subscription_id: u64,
        payload: impl Into<Vec<u8>>,
    ) -> Result<(), SpikeError> {
        self.ensure_ready()?;
        self.ensure_connection_generation(connection_generation)?;
        if !self.subscriptions.contains(&subscription_id) {
            return Err(SpikeError::InvalidCorrelation);
        }
        let payload = payload.into();
        let retained = HEADER_BYTES
            .checked_add(payload.len())
            .ok_or(SpikeError::ResourceLimit("event_bytes"))?;
        if self.events.len() >= self.limits.max_event_frames
            || retained > self.limits.max_frame_bytes
            || self
                .event_bytes
                .checked_add(retained)
                .is_none_or(|bytes| bytes > self.limits.max_event_bytes)
        {
            self.metrics.event_gaps = self.metrics.event_gaps.saturating_add(1);
            self.events.clear();
            self.event_bytes = HEADER_BYTES + b"event_queue_overflow".len();
            self.events.push_back(SpikeFrame::current(
                self.connection_generation,
                FrameKind::Gap,
                subscription_id,
                b"event_queue_overflow".to_vec(),
            ));
            return Ok(());
        }
        self.event_bytes += retained;
        self.events.push_back(SpikeFrame::current(
            self.connection_generation,
            FrameKind::Event,
            subscription_id,
            payload,
        ));
        Ok(())
    }

    /// Queue a connection heartbeat independently from replies and events.
    pub fn heartbeat(
        &mut self,
        connection_generation: ConnectionGeneration,
    ) -> Result<(), SpikeError> {
        self.ensure_ready()?;
        self.ensure_connection_generation(connection_generation)?;
        if self.controls.is_empty() {
            if HEADER_BYTES > self.limits.max_control_bytes {
                self.metrics.resource_rejections =
                    self.metrics.resource_rejections.saturating_add(1);
                return Err(SpikeError::ResourceLimit("control_bytes"));
            }
            self.control_bytes = HEADER_BYTES;
            self.controls.push_back(SpikeFrame::current(
                self.connection_generation,
                FrameKind::Heartbeat,
                0,
                Vec::new(),
            ));
        }
        Ok(())
    }

    /// Refuse an oversized decoded batch before retaining any item state.
    pub fn check_batch_items(&mut self, items: usize) -> Result<(), SpikeError> {
        self.ensure_ready()?;
        if items == 0 || items > self.limits.max_batch_items {
            self.metrics.resource_rejections = self.metrics.resource_rejections.saturating_add(1);
            return Err(SpikeError::ResourceLimit("batch_items"));
        }
        Ok(())
    }

    /// Reserve one bounded invocation retry record.
    pub fn reserve_retry(&mut self) -> Result<(), SpikeError> {
        self.ensure_open()?;
        if self.retry_slots >= self.limits.max_retry_slots {
            self.metrics.resource_rejections = self.metrics.resource_rejections.saturating_add(1);
            return Err(SpikeError::ResourceLimit("retry_slots"));
        }
        self.retry_slots += 1;
        Ok(())
    }

    /// Release one retry record; false means no record was owned.
    pub fn release_retry(&mut self) -> bool {
        if self.retry_slots == 0 {
            false
        } else {
            self.retry_slots -= 1;
            true
        }
    }

    /// Reserve one bounded reconnect record.
    pub fn reserve_reconnect(&mut self) -> Result<(), SpikeError> {
        self.ensure_open()?;
        if self.reconnect_slots >= self.limits.max_reconnect_slots {
            self.metrics.resource_rejections = self.metrics.resource_rejections.saturating_add(1);
            return Err(SpikeError::ResourceLimit("reconnect_slots"));
        }
        self.reconnect_slots += 1;
        Ok(())
    }

    /// Release one reconnect record; false means no record was owned.
    pub fn release_reconnect(&mut self) -> bool {
        if self.reconnect_slots == 0 {
            false
        } else {
            self.reconnect_slots -= 1;
            true
        }
    }

    /// Retain a bounded deadline registration keyed by correlation ID.
    pub fn register_deadline(&mut self, correlation_id: u64) -> Result<(), SpikeError> {
        self.ensure_open()?;
        if correlation_id == 0 || self.deadlines.contains(&correlation_id) {
            return Err(SpikeError::InvalidCorrelation);
        }
        if self.deadlines.len() >= self.limits.max_deadlines {
            self.metrics.resource_rejections = self.metrics.resource_rejections.saturating_add(1);
            return Err(SpikeError::ResourceLimit("deadlines"));
        }
        self.deadlines.insert(correlation_id);
        Ok(())
    }

    /// Release a deadline registration.
    pub fn remove_deadline(&mut self, correlation_id: u64) -> bool {
        self.deadlines.remove(&correlation_id)
    }

    /// Atomically replace retained topology metadata after both bounds pass.
    pub fn install_topology(
        &mut self,
        nodes: usize,
        encoded_bytes: usize,
    ) -> Result<(), SpikeError> {
        self.ensure_ready()?;
        if nodes == 0
            || nodes > self.limits.max_topology_nodes
            || encoded_bytes > self.limits.max_topology_bytes
        {
            self.metrics.resource_rejections = self.metrics.resource_rejections.saturating_add(1);
            return Err(SpikeError::ResourceLimit("topology"));
        }
        self.topology_nodes = nodes;
        self.topology_bytes = encoded_bytes;
        Ok(())
    }

    /// Retain one generation-fenced session and its bounded opaque state.
    pub fn open_session(
        &mut self,
        connection_generation: ConnectionGeneration,
        session_id: u64,
        state_bytes: usize,
    ) -> Result<(), SpikeError> {
        self.ensure_ready()?;
        self.ensure_connection_generation(connection_generation)?;
        if session_id == 0 || self.sessions.contains_key(&session_id) {
            return Err(SpikeError::InvalidCorrelation);
        }
        if self.sessions.len() >= self.limits.max_sessions
            || self
                .session_bytes
                .checked_add(state_bytes)
                .is_none_or(|bytes| bytes > self.limits.max_session_bytes)
        {
            self.metrics.resource_rejections = self.metrics.resource_rejections.saturating_add(1);
            return Err(SpikeError::ResourceLimit("sessions"));
        }
        self.sessions.insert(session_id, state_bytes);
        self.session_bytes += state_bytes;
        Ok(())
    }

    /// Release one session only from its owning connection generation.
    pub fn close_session(
        &mut self,
        connection_generation: ConnectionGeneration,
        session_id: u64,
    ) -> Result<bool, SpikeError> {
        self.ensure_open()?;
        self.ensure_connection_generation(connection_generation)?;
        if let Some(bytes) = self.sessions.remove(&session_id) {
            self.session_bytes -= bytes;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Drain one frame with deterministic round-robin fairness across reply,
    /// control, and event lanes.
    pub fn drain_next(&mut self) -> Option<SpikeFrame> {
        for _ in 0..3 {
            let lane = self.next_lane;
            self.next_lane = (self.next_lane + 1) % 3;
            let frame = match lane {
                0 => self.replies.pop_front().inspect(|frame| {
                    self.reply_bytes -= HEADER_BYTES + frame.payload.len();
                }),
                1 => self.controls.pop_front().inspect(|frame| {
                    self.control_bytes -= HEADER_BYTES + frame.payload.len();
                }),
                _ => self.events.pop_front().inspect(|frame| {
                    self.event_bytes -= HEADER_BYTES + frame.payload.len();
                }),
            };
            if frame.is_some() {
                return frame;
            }
        }
        None
    }

    /// Encode a frame through this candidate's isolated adapter.
    pub fn encode(&self, frame: &SpikeFrame) -> Result<Vec<u8>, SpikeError> {
        self.codec.encode(frame)
    }

    /// Number of valid frames that reached semantic dispatch.
    pub fn dispatch_count(&self) -> u64 {
        self.dispatch_count
    }

    /// Current bounded-resource accounting.
    pub fn resources(&self) -> ResourceSnapshot {
        ResourceSnapshot {
            pending_invocations: self.pending.len(),
            subscriptions: self.subscriptions.len(),
            reply_frames: self.replies.len(),
            reply_bytes: self.reply_bytes,
            event_frames: self.events.len(),
            event_bytes: self.event_bytes,
            control_frames: self.controls.len(),
            control_bytes: self.control_bytes,
            retry_slots: self.retry_slots,
            reconnect_slots: self.reconnect_slots,
            deadlines: self.deadlines.len(),
            topology_nodes: self.topology_nodes,
            topology_bytes: self.topology_bytes,
            sessions: self.sessions.len(),
            session_bytes: self.session_bytes,
        }
    }

    /// Monotonic privacy-safe diagnostics for this connection generation.
    pub fn metrics(&self) -> ConnectionMetrics {
        self.metrics
    }

    /// Close the connection and deterministically return every owned resource.
    pub fn disconnect(&mut self) -> ResourceSnapshot {
        self.state = ConnectionState::Closed;
        self.identity = None;
        self.pending.clear();
        self.subscriptions.clear();
        self.replies.clear();
        self.events.clear();
        self.controls.clear();
        self.reply_bytes = 0;
        self.event_bytes = 0;
        self.control_bytes = 0;
        self.retry_slots = 0;
        self.reconnect_slots = 0;
        self.deadlines.clear();
        self.topology_nodes = 0;
        self.topology_bytes = 0;
        self.sessions.clear();
        self.session_bytes = 0;
        self.resources()
    }

    fn ensure_open(&self) -> Result<(), SpikeError> {
        if self.state == ConnectionState::Closed {
            Err(SpikeError::Closed)
        } else {
            Ok(())
        }
    }

    fn ensure_connection_generation(
        &mut self,
        actual: ConnectionGeneration,
    ) -> Result<(), SpikeError> {
        if actual == self.connection_generation {
            Ok(())
        } else {
            self.metrics.stale_generation_frames =
                self.metrics.stale_generation_frames.saturating_add(1);
            Err(SpikeError::StaleConnectionGeneration {
                expected: self.connection_generation.get(),
                actual: actual.get(),
            })
        }
    }

    fn ensure_ready(&self) -> Result<(), SpikeError> {
        self.ensure_open()?;
        if self.state == ConnectionState::Ready && self.identity.is_some() {
            Ok(())
        } else {
            Err(SpikeError::NotReady)
        }
    }

    fn ensure_state(
        &self,
        expected: ConnectionState,
        operation: &'static str,
    ) -> Result<(), SpikeError> {
        if self.state == expected {
            Ok(())
        } else {
            Err(SpikeError::InvalidState {
                operation,
                actual: self.state,
            })
        }
    }

    fn ensure_ready_or_draining(&self, operation: &'static str) -> Result<(), SpikeError> {
        if matches!(
            self.state,
            ConnectionState::Ready | ConnectionState::Draining
        ) && self.identity.is_some()
        {
            Ok(())
        } else {
            Err(SpikeError::InvalidState {
                operation,
                actual: self.state,
            })
        }
    }
}
