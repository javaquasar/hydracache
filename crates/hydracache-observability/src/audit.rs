use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use hydracache::{ClusterEpoch, RegionId};
use serde::Serialize;

/// Consumer audit event schema registered in `docs/COMPAT.md`.
pub const CONSUMER_AUDIT_EVENT_SCHEMA_VERSION: u32 = 1;

/// How key material may appear in audit payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditKeyPolicy {
    /// Do not include key detail.
    Omit,
    /// Include only a stable hash of the key.
    Hash,
}

/// Redaction policy applied before audit events are created.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AuditRedactionPolicy {
    /// Key redaction mode.
    pub key_policy: AuditKeyPolicy,
}

impl AuditRedactionPolicy {
    /// Create a policy that hashes keys and never records values.
    pub fn hash_keys() -> Self {
        Self {
            key_policy: AuditKeyPolicy::Hash,
        }
    }

    /// Create a policy that omits keys.
    pub fn omit_keys() -> Self {
        Self {
            key_policy: AuditKeyPolicy::Omit,
        }
    }

    /// Redact one key according to this policy.
    pub fn redact_key(self, key: &str) -> AuditKey {
        match self.key_policy {
            AuditKeyPolicy::Omit => AuditKey::Omitted,
            AuditKeyPolicy::Hash => AuditKey::Hash(stable_key_hash(key)),
        }
    }
}

fn stable_key_hash(key: &str) -> String {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Redacted key representation for audit payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditKey {
    /// Key was omitted.
    Omitted,
    /// Stable key hash.
    Hash(String),
    /// Operator-provided dimensions that are safe for audit payloads.
    Dimensions(BTreeMap<String, String>),
}

/// Consumer/governance audit event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuditEvent {
    /// Client identity or authorization failed.
    AuthFailure {
        /// Tenant if it was present and verified enough to report.
        tenant: Option<String>,
        /// Public route.
        route: String,
        /// Request id when available.
        request_id: Option<String>,
    },
    /// Tenant quota/rate/fair-share rejected a request.
    QuotaRejected {
        /// Tenant.
        tenant: String,
        /// Namespace.
        namespace: String,
        /// Request id when available.
        request_id: Option<String>,
    },
    /// Residency governance refused value movement or serving.
    ResidencyRefused {
        /// Namespace.
        namespace: String,
        /// Redacted key detail.
        key: AuditKey,
        /// Source region, when known.
        source_region: Option<RegionId>,
        /// Target/serving region, when known.
        target_region: Option<RegionId>,
        /// Policy epoch that made the decision.
        policy_epoch: ClusterEpoch,
    },
    /// Region failover was committed or refused/degraded.
    RegionFailover {
        /// Previous region.
        from: RegionId,
        /// Target region.
        to: RegionId,
        /// Control-plane epoch.
        epoch: ClusterEpoch,
    },
    /// Governance policy changed.
    PolicyChanged {
        /// Namespace.
        namespace: String,
        /// Control-plane policy epoch.
        policy_epoch: ClusterEpoch,
        /// Redacted operator summary.
        summary: String,
    },
    /// Non-mandatory advisory event.
    Advisory {
        /// Advisory name.
        name: String,
        /// Redacted detail.
        detail: String,
    },
}

/// Versioned audit event envelope for operator-shipped logs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuditEnvelope {
    /// Audit schema version.
    pub schema_version: u32,
    /// Redacted event payload.
    pub event: AuditEvent,
}

impl AuditEnvelope {
    /// Create an envelope at the current schema version.
    pub fn new(event: AuditEvent) -> Self {
        Self {
            schema_version: CONSUMER_AUDIT_EVENT_SCHEMA_VERSION,
            event,
        }
    }
}

impl AuditEvent {
    /// Build a redacted residency refusal event.
    pub fn residency_refused(
        namespace: impl Into<String>,
        key: &str,
        source_region: Option<RegionId>,
        target_region: Option<RegionId>,
        policy_epoch: ClusterEpoch,
        redaction: AuditRedactionPolicy,
    ) -> Self {
        Self::ResidencyRefused {
            namespace: namespace.into(),
            key: redaction.redact_key(key),
            source_region,
            target_region,
            policy_epoch,
        }
    }

    /// Return whether sink failure must fail the guarded operation closed.
    pub fn is_mandatory(&self) -> bool {
        !matches!(self, Self::Advisory { .. })
    }
}

/// Audit sink error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditError {
    message: String,
}

impl AuditError {
    /// Create an error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for AuditError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AuditError {}

/// Operator-supplied audit sink.
pub trait AuditSink: Send + Sync {
    /// Record one event.
    fn record(&self, event: &AuditEvent) -> Result<(), AuditError>;
}

impl<T> AuditSink for Arc<T>
where
    T: AuditSink + ?Sized,
{
    fn record(&self, event: &AuditEvent) -> Result<(), AuditError> {
        (**self).record(event)
    }
}

/// Audit outcome when the recorder applies mandatory/advisory policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditOutcome {
    /// Event reached the sink.
    Recorded,
    /// Optional advisory event was dropped because the sink was unavailable.
    DroppedAdvisory,
}

/// Audit sink health counters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct AuditHealth {
    /// Events successfully recorded.
    pub audit_recorded_total: u64,
    /// Sink write failures.
    pub audit_sink_failures_total: u64,
    /// Mandatory events that failed closed.
    pub audit_mandatory_fail_closed_total: u64,
    /// Optional advisory events dropped due to sink failure.
    pub audit_advisory_dropped_total: u64,
    /// Last redacted error.
    pub last_error: Option<String>,
}

/// Recorder that applies mandatory fail-closed audit semantics.
#[derive(Debug)]
pub struct AuditRecorder<S> {
    sink: S,
    health: AuditHealth,
}

impl<S> AuditRecorder<S>
where
    S: AuditSink,
{
    /// Create a recorder.
    pub fn new(sink: S) -> Self {
        Self {
            sink,
            health: AuditHealth::default(),
        }
    }

    /// Record one event.
    pub fn record(&mut self, event: &AuditEvent) -> Result<AuditOutcome, AuditError> {
        match self.sink.record(event) {
            Ok(()) => {
                self.health.audit_recorded_total =
                    self.health.audit_recorded_total.saturating_add(1);
                Ok(AuditOutcome::Recorded)
            }
            Err(error) if event.is_mandatory() => {
                self.health.audit_sink_failures_total =
                    self.health.audit_sink_failures_total.saturating_add(1);
                self.health.audit_mandatory_fail_closed_total = self
                    .health
                    .audit_mandatory_fail_closed_total
                    .saturating_add(1);
                self.health.last_error = Some(error.to_string());
                Err(error)
            }
            Err(error) => {
                self.health.audit_sink_failures_total =
                    self.health.audit_sink_failures_total.saturating_add(1);
                self.health.audit_advisory_dropped_total =
                    self.health.audit_advisory_dropped_total.saturating_add(1);
                self.health.last_error = Some(error.to_string());
                Ok(AuditOutcome::DroppedAdvisory)
            }
        }
    }

    /// Return audit health counters.
    pub fn health(&self) -> &AuditHealth {
        &self.health
    }
}

/// In-memory bounded audit sink for tests and small adapters.
#[derive(Debug)]
pub struct InMemoryAuditSink {
    events: Mutex<Vec<AuditEvent>>,
    available: AtomicBool,
    capacity: usize,
}

/// Default maximum number of audit events retained in memory.
pub const IN_MEMORY_AUDIT_CAPACITY: usize = 4_096;

impl InMemoryAuditSink {
    /// Create an available sink.
    pub fn new() -> Self {
        Self::with_capacity(IN_MEMORY_AUDIT_CAPACITY)
    }

    /// Create an available sink with an explicit non-zero event budget.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: Mutex::new(Vec::with_capacity(capacity.clamp(1, 256))),
            available: AtomicBool::new(true),
            capacity: capacity.max(1),
        }
    }

    /// Create an unavailable sink.
    pub fn unavailable() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            available: AtomicBool::new(false),
            capacity: IN_MEMORY_AUDIT_CAPACITY,
        }
    }

    /// Set sink availability.
    pub fn set_available(&self, available: bool) {
        self.available.store(available, Ordering::SeqCst);
    }

    /// Return the currently retained bounded snapshot.
    pub fn events(&self) -> Vec<AuditEvent> {
        self.events.lock().expect("audit mutex").clone()
    }

    /// Return the number of events currently retained by this bounded sink.
    pub fn len(&self) -> usize {
        self.events.lock().expect("audit mutex").len()
    }

    /// Return whether the bounded sink currently retains no events.
    pub fn is_empty(&self) -> bool {
        self.events.lock().expect("audit mutex").is_empty()
    }
}

impl Default for InMemoryAuditSink {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditSink for InMemoryAuditSink {
    fn record(&self, event: &AuditEvent) -> Result<(), AuditError> {
        if !self.available.load(Ordering::SeqCst) {
            return Err(AuditError::new("audit sink unavailable"));
        }
        let mut events = self.events.lock().expect("audit mutex");
        if events.len() >= self.capacity {
            return Err(AuditError::new("in-memory audit sink capacity exhausted"));
        }
        events.push(event.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mandatory_event(index: usize) -> AuditEvent {
        AuditEvent::AuthFailure {
            tenant: Some("tenant".to_owned()),
            route: "/client/v1/data".to_owned(),
            request_id: Some(format!("request-{index}")),
        }
    }

    #[test]
    fn bounded_sink_fails_mandatory_audit_closed_at_capacity() {
        let sink = Arc::new(InMemoryAuditSink::with_capacity(2));
        let mut recorder = AuditRecorder::new(Arc::clone(&sink));
        assert_eq!(
            recorder.record(&mandatory_event(1)).unwrap(),
            AuditOutcome::Recorded
        );
        assert_eq!(
            recorder.record(&mandatory_event(2)).unwrap(),
            AuditOutcome::Recorded
        );

        let error = recorder.record(&mandatory_event(3)).unwrap_err();

        assert_eq!(error.to_string(), "in-memory audit sink capacity exhausted");
        assert_eq!(sink.events().len(), 2);
        assert_eq!(recorder.health().audit_sink_failures_total, 1);
        assert_eq!(recorder.health().audit_mandatory_fail_closed_total, 1);
    }
}
