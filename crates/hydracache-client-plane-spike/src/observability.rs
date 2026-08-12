//! Stable, bounded, privacy-safe diagnostics for the non-production HC/2 plane.
//!
//! Values flow out through typed samples. Peer-controlled strings never become
//! metric names or labels, and the only tenant correlation is a salted bucket.

use std::collections::VecDeque;

use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::http2_drain::DrainReason;
use crate::security::SecurityRejection;
use crate::{
    ConnectionGeneration, ConnectionMetrics, ConnectionState, ResourceSnapshot, SpikeConnection,
    TransportCandidate,
};

/// Versioned contract consumed by metric and trace adapters.
pub const DIAGNOSTICS_SCHEMA_VERSION: &str = "hydracache.hc2.client_plane.v1";
/// Tenant labels are deliberately lossy and cannot exceed this cardinality.
pub const TENANT_BUCKET_COUNT: u8 = 64;
/// Hard upper bound for retained trace records per recorder.
pub const MAX_TRACE_RECORDS: usize = 128;
/// Hard upper bound for metric series emitted by one recorder snapshot.
pub const MAX_METRIC_SERIES_PER_CONNECTION: usize = 48;

/// Per-process salt used to make tenant buckets deployment-local.
#[derive(Clone)]
pub struct DiagnosticSalt([u8; 32]);

impl DiagnosticSalt {
    /// Construct a salt. An all-zero salt is rejected because it would make
    /// buckets identical across unconfigured deployments.
    pub fn new(bytes: [u8; 32]) -> Result<Self, ObservabilityError> {
        if bytes.iter().all(|byte| *byte == 0) {
            return Err(ObservabilityError::InvalidSalt);
        }
        Ok(Self(bytes))
    }

    fn tenant_bucket(&self, tenant: &str) -> u8 {
        let mut digest = Sha256::new();
        digest.update(self.0);
        digest.update((tenant.len() as u64).to_be_bytes());
        digest.update(tenant.as_bytes());
        digest.finalize()[0] % TENANT_BUCKET_COUNT
    }
}

impl std::fmt::Debug for DiagnosticSalt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("DiagnosticSalt([REDACTED])")
    }
}

/// Stable transport label. The wire endpoint and authority are never labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportLabel {
    DedicatedTcpTls,
    Http2Bidirectional,
    GrpcBidirectional,
}

impl From<TransportCandidate> for TransportLabel {
    fn from(candidate: TransportCandidate) -> Self {
        match candidate {
            TransportCandidate::DedicatedTcpTls => Self::DedicatedTcpTls,
            TransportCandidate::Http2Bidirectional => Self::Http2Bidirectional,
            TransportCandidate::GrpcBidirectional => Self::GrpcBidirectional,
        }
    }
}

/// Stable lifecycle label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StateLabel {
    Created,
    TlsVerified,
    Authenticated,
    Ready,
    Draining,
    Closed,
}

impl From<ConnectionState> for StateLabel {
    fn from(state: ConnectionState) -> Self {
        match state {
            ConnectionState::Created => Self::Created,
            ConnectionState::TlsVerified => Self::TlsVerified,
            ConnectionState::Authenticated => Self::Authenticated,
            ConnectionState::Ready => Self::Ready,
            ConnectionState::Draining => Self::Draining,
            ConnectionState::Closed => Self::Closed,
        }
    }
}

/// Labels common to every series. Every field has a finite domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DiagnosticLabels {
    pub transport: TransportLabel,
    pub tenant_bucket: u8,
}

/// Metric semantic required by exporters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricKind {
    Counter,
    Gauge,
}

/// One stable metric sample. `reason` is always a closed taxonomy label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MetricSample {
    pub name: &'static str,
    pub kind: MetricKind,
    pub value: u64,
    pub labels: DiagnosticLabels,
    /// Present only on the state gauge. Mutable state never splits counters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<StateLabel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<&'static str>,
}

/// One bounded trace record. It cannot carry arbitrary peer data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TraceSample {
    pub sequence: u64,
    pub event: &'static str,
    pub labels: DiagnosticLabels,
    pub state: StateLabel,
    pub connection_generation: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<&'static str>,
}

/// Serializable output accepted by logs, tests, Prometheus/OTLP adapters, or an
/// operator status endpoint. This module does not install a network exporter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiagnosticExport {
    pub schema: &'static str,
    pub metrics: Vec<MetricSample>,
    pub traces: Vec<TraceSample>,
}

/// Adapter boundary that keeps the recorder independent from a telemetry SDK.
pub trait DiagnosticSink {
    fn metric(&mut self, sample: &MetricSample);
    fn trace(&mut self, sample: &TraceSample);
}

/// Stable deadline outcomes. Correlation IDs never leave through diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineOutcome {
    Completed,
    TimedOut,
    Cancelled,
}

impl DeadlineOutcome {
    const ALL: [Self; 3] = [Self::Completed, Self::TimedOut, Self::Cancelled];

    const fn index(self) -> usize {
        match self {
            Self::Completed => 0,
            Self::TimedOut => 1,
            Self::Cancelled => 2,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::TimedOut => "timed_out",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct DiagnosticCounters {
    dispatched_frames: u64,
    rejected_frames: u64,
    resource_rejections: u64,
    cancellations: u64,
    gaps: u64,
    stale_generation_frames: u64,
    reconnects: u64,
    retries_scheduled: u64,
    retries_exhausted: u64,
    repairs: u64,
    session_heartbeats: u64,
    session_losses: u64,
    security_rejections: [u64; 11],
    deadline_outcomes: [u64; 3],
    drain_reasons: [u64; 3],
    trace_dropped: u64,
}

/// Bounded recorder for one logical authenticated client identity.
#[derive(Debug)]
pub struct ClientPlaneDiagnostics {
    candidate: TransportCandidate,
    generation: ConnectionGeneration,
    state: ConnectionState,
    tenant_bucket: u8,
    resources: ResourceSnapshot,
    runtime_baseline: ConnectionMetrics,
    counters: DiagnosticCounters,
    traces: VecDeque<TraceSample>,
    trace_capacity: usize,
    next_sequence: u64,
}

impl ClientPlaneDiagnostics {
    /// Build one recorder. The tenant is hashed immediately and never retained.
    pub fn new(
        candidate: TransportCandidate,
        generation: ConnectionGeneration,
        tenant: &str,
        salt: &DiagnosticSalt,
        trace_capacity: usize,
    ) -> Result<Self, ObservabilityError> {
        if tenant.is_empty() {
            return Err(ObservabilityError::EmptyTenant);
        }
        if trace_capacity == 0 || trace_capacity > MAX_TRACE_RECORDS {
            return Err(ObservabilityError::InvalidTraceCapacity);
        }
        let mut diagnostics = Self {
            candidate,
            generation,
            state: ConnectionState::Created,
            tenant_bucket: salt.tenant_bucket(tenant),
            resources: ResourceSnapshot::default(),
            runtime_baseline: ConnectionMetrics::default(),
            counters: DiagnosticCounters::default(),
            traces: VecDeque::with_capacity(trace_capacity),
            trace_capacity,
            next_sequence: 1,
        };
        diagnostics.push_trace("connection_created", None);
        Ok(diagnostics)
    }

    /// Observe authoritative gauges and monotonic counters from the runtime.
    pub fn observe_connection(
        &mut self,
        connection: &SpikeConnection,
    ) -> Result<(), ObservabilityError> {
        if connection.candidate() != self.candidate {
            return Err(ObservabilityError::TransportChanged);
        }
        let actual = connection.connection_generation();
        if actual != self.generation {
            self.reconnect(actual)?;
        }
        let metrics = connection.metrics();
        self.counters.dispatched_frames = self.counters.dispatched_frames.saturating_add(
            metrics
                .dispatched_frames
                .saturating_sub(self.runtime_baseline.dispatched_frames),
        );
        self.counters.rejected_frames = self.counters.rejected_frames.saturating_add(
            metrics
                .rejected_frames
                .saturating_sub(self.runtime_baseline.rejected_frames),
        );
        self.counters.resource_rejections = self.counters.resource_rejections.saturating_add(
            metrics
                .resource_rejections
                .saturating_sub(self.runtime_baseline.resource_rejections),
        );
        let cancellations = metrics
            .cancelled_invocations
            .saturating_sub(self.runtime_baseline.cancelled_invocations);
        let gaps = metrics
            .event_gaps
            .saturating_sub(self.runtime_baseline.event_gaps);
        self.counters.cancellations = self.counters.cancellations.saturating_add(cancellations);
        self.counters.gaps = self.counters.gaps.saturating_add(gaps);
        self.counters.stale_generation_frames =
            self.counters.stale_generation_frames.saturating_add(
                metrics
                    .stale_generation_frames
                    .saturating_sub(self.runtime_baseline.stale_generation_frames),
            );
        if cancellations > 0 {
            self.push_trace("invocation_cancelled", None);
        }
        if gaps > 0 {
            self.push_trace("event_gap", None);
        }
        let previous_state = self.state;
        self.state = connection.state();
        self.resources = connection.resources();
        self.runtime_baseline = metrics;
        if self.state != previous_state {
            self.push_trace("state_changed", Some(state_label(self.state)));
        }
        Ok(())
    }

    /// Advance to a strictly newer connection generation.
    pub fn reconnect(
        &mut self,
        generation: ConnectionGeneration,
    ) -> Result<(), ObservabilityError> {
        if generation <= self.generation {
            return Err(ObservabilityError::GenerationDidNotAdvance);
        }
        self.generation = generation;
        self.state = ConnectionState::Created;
        self.resources = ResourceSnapshot::default();
        self.runtime_baseline = ConnectionMetrics::default();
        self.counters.reconnects = self.counters.reconnects.saturating_add(1);
        self.push_trace("connection_reconnect", None);
        Ok(())
    }

    pub fn record_retry_scheduled(&mut self) {
        self.counters.retries_scheduled = self.counters.retries_scheduled.saturating_add(1);
        self.push_trace("retry_scheduled", None);
    }

    pub fn record_retry_exhausted(&mut self) {
        self.counters.retries_exhausted = self.counters.retries_exhausted.saturating_add(1);
        self.push_trace("retry_exhausted", None);
    }

    pub fn record_repair(&mut self) {
        self.counters.repairs = self.counters.repairs.saturating_add(1);
        self.push_trace("cache_repair", None);
    }

    pub fn record_security_rejection(&mut self, reason: SecurityRejection) {
        self.counters.security_rejections[security_index(reason)] =
            self.counters.security_rejections[security_index(reason)].saturating_add(1);
        self.push_trace("security_rejected", Some(reason.label()));
    }

    pub fn record_deadline(&mut self, outcome: DeadlineOutcome) {
        self.counters.deadline_outcomes[outcome.index()] =
            self.counters.deadline_outcomes[outcome.index()].saturating_add(1);
        self.push_trace("deadline_outcome", Some(outcome.label()));
    }

    pub fn record_session_heartbeat(&mut self) {
        self.counters.session_heartbeats = self.counters.session_heartbeats.saturating_add(1);
        self.push_trace("session_heartbeat", None);
    }

    pub fn record_session_loss(&mut self) {
        self.counters.session_losses = self.counters.session_losses.saturating_add(1);
        self.push_trace("session_loss", None);
    }

    pub fn record_drain(&mut self, reason: DrainReason) {
        self.counters.drain_reasons[drain_index(reason)] =
            self.counters.drain_reasons[drain_index(reason)].saturating_add(1);
        self.push_trace("connection_drain", Some(drain_label(reason)));
    }

    /// Produce one bounded serializable snapshot.
    pub fn snapshot(&self) -> DiagnosticExport {
        DiagnosticExport {
            schema: DIAGNOSTICS_SCHEMA_VERSION,
            metrics: self.metric_samples(),
            traces: self.traces.iter().cloned().collect(),
        }
    }

    /// Send the current snapshot to a telemetry adapter without granting it
    /// access to runtime state or peer-owned values.
    pub fn export_to(&self, sink: &mut impl DiagnosticSink) {
        let snapshot = self.snapshot();
        for metric in &snapshot.metrics {
            sink.metric(metric);
        }
        for trace in &snapshot.traces {
            sink.trace(trace);
        }
    }

    fn metric_samples(&self) -> Vec<MetricSample> {
        let mut samples = Vec::with_capacity(MAX_METRIC_SERIES_PER_CONNECTION);
        samples.push(self.metric(
            "hydracache_hc2_connection_state",
            MetricKind::Gauge,
            1,
            Some(self.state.into()),
            None,
        ));
        let mut gauge =
            |name, value| samples.push(self.metric(name, MetricKind::Gauge, value, None, None));
        gauge(
            "hydracache_hc2_connection_generation",
            self.generation.get(),
        );
        gauge(
            "hydracache_hc2_pending_invocations",
            as_u64(self.resources.pending_invocations),
        );
        gauge(
            "hydracache_hc2_subscriptions",
            as_u64(self.resources.subscriptions),
        );
        gauge(
            "hydracache_hc2_reply_queue_frames",
            as_u64(self.resources.reply_frames),
        );
        gauge(
            "hydracache_hc2_reply_queue_bytes",
            as_u64(self.resources.reply_bytes),
        );
        gauge(
            "hydracache_hc2_event_queue_frames",
            as_u64(self.resources.event_frames),
        );
        gauge(
            "hydracache_hc2_event_queue_bytes",
            as_u64(self.resources.event_bytes),
        );
        gauge(
            "hydracache_hc2_control_queue_frames",
            as_u64(self.resources.control_frames),
        );
        gauge(
            "hydracache_hc2_control_queue_bytes",
            as_u64(self.resources.control_bytes),
        );
        gauge(
            "hydracache_hc2_retry_slots",
            as_u64(self.resources.retry_slots),
        );
        gauge(
            "hydracache_hc2_reconnect_slots",
            as_u64(self.resources.reconnect_slots),
        );
        gauge("hydracache_hc2_deadlines", as_u64(self.resources.deadlines));
        gauge(
            "hydracache_hc2_topology_nodes",
            as_u64(self.resources.topology_nodes),
        );
        gauge(
            "hydracache_hc2_topology_bytes",
            as_u64(self.resources.topology_bytes),
        );
        gauge("hydracache_hc2_sessions", as_u64(self.resources.sessions));
        gauge(
            "hydracache_hc2_session_bytes",
            as_u64(self.resources.session_bytes),
        );
        let counters = [
            (
                "hydracache_hc2_dispatched_frames_total",
                self.counters.dispatched_frames,
            ),
            (
                "hydracache_hc2_rejected_frames_total",
                self.counters.rejected_frames,
            ),
            (
                "hydracache_hc2_resource_rejections_total",
                self.counters.resource_rejections,
            ),
            (
                "hydracache_hc2_cancellations_total",
                self.counters.cancellations,
            ),
            ("hydracache_hc2_event_gaps_total", self.counters.gaps),
            (
                "hydracache_hc2_stale_generation_frames_total",
                self.counters.stale_generation_frames,
            ),
            ("hydracache_hc2_reconnects_total", self.counters.reconnects),
            (
                "hydracache_hc2_retries_scheduled_total",
                self.counters.retries_scheduled,
            ),
            (
                "hydracache_hc2_retries_exhausted_total",
                self.counters.retries_exhausted,
            ),
            ("hydracache_hc2_repairs_total", self.counters.repairs),
            (
                "hydracache_hc2_session_heartbeats_total",
                self.counters.session_heartbeats,
            ),
            (
                "hydracache_hc2_session_losses_total",
                self.counters.session_losses,
            ),
            (
                "hydracache_hc2_trace_dropped_total",
                self.counters.trace_dropped,
            ),
        ];
        for (name, value) in counters {
            samples.push(self.metric(name, MetricKind::Counter, value, None, None));
        }
        for reason in SECURITY_REJECTIONS {
            let name = if matches!(
                reason,
                SecurityRejection::MissingClientCertificate
                    | SecurityRejection::AuthorizationDenied
            ) {
                "hydracache_hc2_auth_rejections_total"
            } else {
                "hydracache_hc2_tls_rejections_total"
            };
            samples.push(self.metric(
                name,
                MetricKind::Counter,
                self.counters.security_rejections[security_index(reason)],
                None,
                Some(reason.label()),
            ));
        }
        for outcome in DeadlineOutcome::ALL {
            samples.push(self.metric(
                "hydracache_hc2_deadline_outcomes_total",
                MetricKind::Counter,
                self.counters.deadline_outcomes[outcome.index()],
                None,
                Some(outcome.label()),
            ));
        }
        for reason in DRAIN_REASONS {
            samples.push(self.metric(
                "hydracache_hc2_drains_total",
                MetricKind::Counter,
                self.counters.drain_reasons[drain_index(reason)],
                None,
                Some(drain_label(reason)),
            ));
        }
        debug_assert!(samples.len() <= MAX_METRIC_SERIES_PER_CONNECTION);
        samples
    }

    fn metric(
        &self,
        name: &'static str,
        kind: MetricKind,
        value: u64,
        state: Option<StateLabel>,
        reason: Option<&'static str>,
    ) -> MetricSample {
        MetricSample {
            name,
            kind,
            value,
            labels: self.labels(),
            state,
            reason,
        }
    }

    fn labels(&self) -> DiagnosticLabels {
        DiagnosticLabels {
            transport: self.candidate.into(),
            tenant_bucket: self.tenant_bucket,
        }
    }

    fn push_trace(&mut self, event: &'static str, reason: Option<&'static str>) {
        if self.traces.len() == self.trace_capacity {
            self.traces.pop_front();
            self.counters.trace_dropped = self.counters.trace_dropped.saturating_add(1);
        }
        self.traces.push_back(TraceSample {
            sequence: self.next_sequence,
            event,
            labels: self.labels(),
            state: self.state.into(),
            connection_generation: self.generation.get(),
            reason,
        });
        self.next_sequence = self.next_sequence.saturating_add(1);
    }
}

/// Invalid configuration or unsafe recorder transition.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ObservabilityError {
    #[error("diagnostic salt must not be all zero")]
    InvalidSalt,
    #[error("tenant identity must not be empty")]
    EmptyTenant,
    #[error("trace capacity must be within 1..=128")]
    InvalidTraceCapacity,
    #[error("one diagnostic recorder cannot change transport")]
    TransportChanged,
    #[error("connection generation must advance monotonically")]
    GenerationDidNotAdvance,
}

const SECURITY_REJECTIONS: [SecurityRejection; 11] = [
    SecurityRejection::UnknownCa,
    SecurityRejection::MissingClientCertificate,
    SecurityRejection::HostnameMismatch,
    SecurityRejection::CertificateExpired,
    SecurityRejection::CertificateNotYetValid,
    SecurityRejection::WrongClientCa,
    SecurityRejection::InvalidExtendedKeyUsage,
    SecurityRejection::CertificateChainDepth,
    SecurityRejection::CertificateChainSize,
    SecurityRejection::TlsProtocolPolicy,
    SecurityRejection::AuthorizationDenied,
];

const DRAIN_REASONS: [DrainReason; 3] = [
    DrainReason::Graceful,
    DrainReason::DeadlineExpired,
    DrainReason::PeerReset,
];

const fn security_index(reason: SecurityRejection) -> usize {
    match reason {
        SecurityRejection::UnknownCa => 0,
        SecurityRejection::MissingClientCertificate => 1,
        SecurityRejection::HostnameMismatch => 2,
        SecurityRejection::CertificateExpired => 3,
        SecurityRejection::CertificateNotYetValid => 4,
        SecurityRejection::WrongClientCa => 5,
        SecurityRejection::InvalidExtendedKeyUsage => 6,
        SecurityRejection::CertificateChainDepth => 7,
        SecurityRejection::CertificateChainSize => 8,
        SecurityRejection::TlsProtocolPolicy => 9,
        SecurityRejection::AuthorizationDenied => 10,
    }
}

const fn drain_index(reason: DrainReason) -> usize {
    match reason {
        DrainReason::Graceful => 0,
        DrainReason::DeadlineExpired => 1,
        DrainReason::PeerReset => 2,
    }
}

const fn drain_label(reason: DrainReason) -> &'static str {
    match reason {
        DrainReason::Graceful => "graceful",
        DrainReason::DeadlineExpired => "deadline_expired",
        DrainReason::PeerReset => "peer_reset",
    }
}

const fn state_label(state: ConnectionState) -> &'static str {
    match state {
        ConnectionState::Created => "created",
        ConnectionState::TlsVerified => "tls_verified",
        ConnectionState::Authenticated => "authenticated",
        ConnectionState::Ready => "ready",
        ConnectionState::Draining => "draining",
        ConnectionState::Closed => "closed",
    }
}

fn as_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
