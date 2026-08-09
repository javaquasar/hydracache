//! Non-production HC/2 transport-selection spikes.
//!
//! This crate owns the transport-neutral, sans-I/O semantic harness used by
//! release 0.68 W0. It deliberately does not expose a production listener.
//! Passing these checks is necessary, but it is not evidence of real TLS,
//! HTTP/2, gRPC, cross-language generation, or daemon interoperability.

use std::collections::{BTreeSet, VecDeque};

use thiserror::Error;

/// First proposed HC/2 wire generation. It is isolated from HC/1 v1-v4.
pub const HC2_GENERATION: u16 = 5;
const HEADER_BYTES: usize = 4 + 4 + 2 + 1 + 8 + 4;

/// Transport candidates required by the 0.68 W0 bake-off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Message class.
    pub kind: FrameKind,
    /// Invocation or subscription correlation id; zero for connection control.
    pub correlation_id: u64,
    /// Bounded opaque spike payload.
    pub payload: Vec<u8>,
}

impl SpikeFrame {
    /// Construct a current-generation frame.
    pub fn current(kind: FrameKind, correlation_id: u64, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            generation: HC2_GENERATION,
            kind,
            correlation_id,
            payload: payload.into(),
        }
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
    /// Maximum queued event frames before an explicit gap replaces them.
    pub max_event_frames: usize,
    /// Maximum connection-owned subscriptions.
    pub max_subscriptions: usize,
}

impl Default for SpikeLimits {
    fn default() -> Self {
        Self {
            max_frame_bytes: 64 * 1024,
            max_pending_invocations: 256,
            max_reply_frames: 256,
            max_event_frames: 32,
            max_subscriptions: 32,
        }
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
    /// A connection-owned resource bound was reached.
    #[error("connection resource limit reached: {0}")]
    ResourceLimit(&'static str),
    /// A correlation or subscription id is invalid or already active.
    #[error("duplicate or invalid correlation id")]
    InvalidCorrelation,
    /// A completion referred to no active invocation.
    #[error("unknown pending invocation")]
    UnknownInvocation,
    /// The connection was already closed.
    #[error("connection is closed")]
    Closed,
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
        let kind = FrameKind::try_from(bytes[10])?;
        let correlation_id = u64::from_be_bytes(bytes[11..19].try_into().expect("fixed slice"));
        let payload_len =
            u32::from_be_bytes(bytes[19..23].try_into().expect("fixed slice")) as usize;
        if HEADER_BYTES.checked_add(payload_len) != Some(bytes.len()) {
            return Err(SpikeError::LengthMismatch);
        }
        Ok(SpikeFrame {
            generation,
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
    /// Queued event or explicit-gap frames.
    pub event_frames: usize,
    /// Queued connection control frames.
    pub control_frames: usize,
}

/// Connection-owned semantic runtime shared by all W0 candidates.
#[derive(Debug)]
pub struct SpikeConnection {
    candidate: TransportCandidate,
    codec: SpikeCodec,
    limits: SpikeLimits,
    identity: Option<PeerIdentity>,
    negotiated: bool,
    closed: bool,
    dispatch_count: u64,
    pending: BTreeSet<u64>,
    subscriptions: BTreeSet<u64>,
    replies: VecDeque<SpikeFrame>,
    events: VecDeque<SpikeFrame>,
    controls: VecDeque<SpikeFrame>,
    next_lane: u8,
}

impl SpikeConnection {
    /// Create one disconnected candidate spike runtime.
    pub fn new(candidate: TransportCandidate, limits: SpikeLimits) -> Self {
        Self {
            candidate,
            codec: SpikeCodec::new(candidate, limits.max_frame_bytes),
            limits,
            identity: None,
            negotiated: false,
            closed: false,
            dispatch_count: 0,
            pending: BTreeSet::new(),
            subscriptions: BTreeSet::new(),
            replies: VecDeque::new(),
            events: VecDeque::new(),
            controls: VecDeque::new(),
            next_lane: 0,
        }
    }

    /// Candidate under test.
    pub fn candidate(&self) -> TransportCandidate {
        self.candidate
    }

    /// Install a TLS-authenticated peer identity before negotiation.
    pub fn authenticate(&mut self, identity: PeerIdentity) -> Result<(), SpikeError> {
        self.ensure_open()?;
        if !identity.tls_verified
            || identity.client_id.trim().is_empty()
            || identity.tenant.trim().is_empty()
            || identity.client_id.len() > 128
            || identity.tenant.len() > 128
        {
            return Err(SpikeError::UnverifiedIdentity);
        }
        self.identity = Some(identity);
        Ok(())
    }

    /// Negotiate the isolated HC/2 generation after authentication.
    pub fn negotiate(&mut self, generation: u16) -> Result<(), SpikeError> {
        self.ensure_open()?;
        if self.identity.is_none() {
            return Err(SpikeError::UnverifiedIdentity);
        }
        if generation != HC2_GENERATION {
            return Err(SpikeError::UnknownGeneration(generation));
        }
        self.negotiated = true;
        Ok(())
    }

    /// Decode a candidate frame. Invalid generations never increment dispatch.
    pub fn receive(&mut self, bytes: &[u8]) -> Result<SpikeFrame, SpikeError> {
        self.ensure_ready()?;
        let frame = self.codec.decode(bytes)?;
        self.dispatch_count = self.dispatch_count.saturating_add(1);
        Ok(frame)
    }

    /// Start one bounded invocation.
    pub fn begin_invocation(&mut self, correlation_id: u64) -> Result<(), SpikeError> {
        self.ensure_ready()?;
        if correlation_id == 0 || self.pending.contains(&correlation_id) {
            return Err(SpikeError::InvalidCorrelation);
        }
        if self.pending.len() >= self.limits.max_pending_invocations {
            return Err(SpikeError::ResourceLimit("pending_invocations"));
        }
        self.pending.insert(correlation_id);
        Ok(())
    }

    /// Complete one invocation exactly once and queue its reply.
    pub fn complete_invocation(
        &mut self,
        correlation_id: u64,
        payload: impl Into<Vec<u8>>,
    ) -> Result<(), SpikeError> {
        self.ensure_ready()?;
        if !self.pending.remove(&correlation_id) {
            return Err(SpikeError::UnknownInvocation);
        }
        if self.replies.len() >= self.limits.max_reply_frames {
            return Err(SpikeError::ResourceLimit("reply_frames"));
        }
        self.replies.push_back(SpikeFrame::current(
            FrameKind::Reply,
            correlation_id,
            payload,
        ));
        Ok(())
    }

    /// Cancel one invocation. Cancellation is idempotent for disconnect races.
    pub fn cancel_invocation(&mut self, correlation_id: u64) -> Result<bool, SpikeError> {
        self.ensure_open()?;
        Ok(self.pending.remove(&correlation_id))
    }

    /// Register a connection-owned subscription.
    pub fn register_subscription(&mut self, subscription_id: u64) -> Result<(), SpikeError> {
        self.ensure_ready()?;
        if subscription_id == 0 || self.subscriptions.contains(&subscription_id) {
            return Err(SpikeError::InvalidCorrelation);
        }
        if self.subscriptions.len() >= self.limits.max_subscriptions {
            return Err(SpikeError::ResourceLimit("subscriptions"));
        }
        self.subscriptions.insert(subscription_id);
        Ok(())
    }

    /// Push a bounded cache signal. On overflow all queued event history is
    /// replaced by one explicit gap requiring conservative repair.
    pub fn push_event(
        &mut self,
        subscription_id: u64,
        payload: impl Into<Vec<u8>>,
    ) -> Result<(), SpikeError> {
        self.ensure_ready()?;
        if !self.subscriptions.contains(&subscription_id) {
            return Err(SpikeError::InvalidCorrelation);
        }
        if self.events.len() >= self.limits.max_event_frames {
            self.events.clear();
            self.events.push_back(SpikeFrame::current(
                FrameKind::Gap,
                subscription_id,
                b"event_queue_overflow".to_vec(),
            ));
            return Ok(());
        }
        self.events.push_back(SpikeFrame::current(
            FrameKind::Event,
            subscription_id,
            payload,
        ));
        Ok(())
    }

    /// Queue a connection heartbeat independently from replies and events.
    pub fn heartbeat(&mut self) -> Result<(), SpikeError> {
        self.ensure_ready()?;
        if self.controls.is_empty() {
            self.controls
                .push_back(SpikeFrame::current(FrameKind::Heartbeat, 0, Vec::new()));
        }
        Ok(())
    }

    /// Drain one frame with deterministic round-robin fairness across reply,
    /// control, and event lanes.
    pub fn drain_next(&mut self) -> Option<SpikeFrame> {
        for _ in 0..3 {
            let lane = self.next_lane;
            self.next_lane = (self.next_lane + 1) % 3;
            let frame = match lane {
                0 => self.replies.pop_front(),
                1 => self.controls.pop_front(),
                _ => self.events.pop_front(),
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
            event_frames: self.events.len(),
            control_frames: self.controls.len(),
        }
    }

    /// Close the connection and deterministically return every owned resource.
    pub fn disconnect(&mut self) -> ResourceSnapshot {
        self.closed = true;
        self.negotiated = false;
        self.identity = None;
        self.pending.clear();
        self.subscriptions.clear();
        self.replies.clear();
        self.events.clear();
        self.controls.clear();
        self.resources()
    }

    fn ensure_open(&self) -> Result<(), SpikeError> {
        if self.closed {
            Err(SpikeError::Closed)
        } else {
            Ok(())
        }
    }

    fn ensure_ready(&self) -> Result<(), SpikeError> {
        self.ensure_open()?;
        if self.negotiated && self.identity.is_some() {
            Ok(())
        } else {
            Err(SpikeError::NotReady)
        }
    }
}
