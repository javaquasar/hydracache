use std::fmt;

use serde::{Deserialize, Serialize};

use crate::grid::session_context::{PartitionKey, SessionId, SessionWatermark, VersionStamp};

/// Monotonic per-session sequence assigned by the caller or session coordinator.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct SessionSequence(u64);

impl SessionSequence {
    /// Create a sequence value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the numeric sequence.
    pub const fn value(self) -> u64 {
        self.0
    }

    /// Return the next sequence value.
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// Last accepted write stamp for one session and partition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionWriteStamp {
    /// Session that produced the write.
    pub session_id: SessionId,
    /// Monotonic session sequence.
    pub sequence: SessionSequence,
    /// Partition/region affected by the write.
    pub key: PartitionKey,
    /// Version assigned to the write.
    pub stamp: VersionStamp,
}

impl SessionWriteStamp {
    /// Create a session write stamp.
    pub fn new(
        session_id: impl Into<SessionId>,
        sequence: SessionSequence,
        key: PartitionKey,
        stamp: VersionStamp,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            sequence,
            key,
            stamp,
        }
    }
}

/// Monotonic read guard decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonotonicReadDecision {
    /// The candidate is at or above the session's previous observation.
    ServeLocal,
    /// Serving the candidate would move the session backwards.
    PreventStale {
        /// Required floor from the session watermark.
        required: VersionStamp,
        /// Candidate replica stamp.
        candidate: VersionStamp,
    },
}

/// Monotonic write guard decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonotonicWriteDecision {
    /// The write preserves per-session ordering.
    Apply,
    /// A duplicate or older sequence would reorder this session's writes.
    PreventReorder {
        /// Last accepted sequence.
        accepted: SessionSequence,
        /// Incoming sequence.
        incoming: SessionSequence,
    },
    /// A higher sequence cannot lower the stored version for the same key.
    PreventStaleStamp {
        /// Last accepted stamp.
        accepted: VersionStamp,
        /// Incoming stamp.
        incoming: VersionStamp,
    },
}

/// Error returned by strict monotonic read application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonotonicReadViolation {
    /// Partition/region whose read would go backwards.
    pub key: PartitionKey,
    /// Required floor from the session watermark.
    pub required: VersionStamp,
    /// Candidate replica stamp.
    pub candidate: VersionStamp,
}

impl fmt::Display for MonotonicReadViolation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "monotonic read violation for partition {}: candidate {:?} below required {:?}",
            self.key.partition.value(),
            self.candidate,
            self.required
        )
    }
}

impl std::error::Error for MonotonicReadViolation {}

/// Error returned by strict monotonic write application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonotonicWriteViolation {
    /// Last accepted write.
    pub accepted: SessionWriteStamp,
    /// Incoming write that would break monotonicity.
    pub incoming: SessionWriteStamp,
    /// Guard decision explaining the violation.
    pub decision: MonotonicWriteDecision,
}

impl fmt::Display for MonotonicWriteViolation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "monotonic write violation for session {}: decision {:?}",
            self.incoming.session_id.as_str(),
            self.decision
        )
    }
}

impl std::error::Error for MonotonicWriteViolation {}

/// Session monotonicity metrics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMonotonicMetrics {
    /// Reads prevented from going backwards.
    pub monotonic_read_violations_prevented_total: u64,
    /// Writes prevented from reordering or lowering their stamp.
    pub monotonic_write_reorders_prevented_total: u64,
}

/// Resolve whether a candidate read preserves monotonic reads.
pub fn resolve_monotonic_read(
    watermark: &SessionWatermark,
    key: &PartitionKey,
    candidate: VersionStamp,
) -> MonotonicReadDecision {
    let Some(required) = watermark.highest_seen(key) else {
        return MonotonicReadDecision::ServeLocal;
    };
    if candidate >= required {
        MonotonicReadDecision::ServeLocal
    } else {
        MonotonicReadDecision::PreventStale {
            required,
            candidate,
        }
    }
}

/// Apply a monotonic read by updating the watermark only if the candidate is safe.
pub fn apply_monotonic_read(
    watermark: &mut SessionWatermark,
    key: PartitionKey,
    candidate: VersionStamp,
) -> Result<(), MonotonicReadViolation> {
    match resolve_monotonic_read(watermark, &key, candidate) {
        MonotonicReadDecision::ServeLocal => {
            watermark.observe(key, candidate);
            Ok(())
        }
        MonotonicReadDecision::PreventStale {
            required,
            candidate,
        } => Err(MonotonicReadViolation {
            key,
            required,
            candidate,
        }),
    }
}

/// Resolve whether an incoming write preserves session write order.
pub fn resolve_monotonic_write(
    accepted: Option<&SessionWriteStamp>,
    incoming: &SessionWriteStamp,
) -> MonotonicWriteDecision {
    let Some(accepted) = accepted else {
        return MonotonicWriteDecision::Apply;
    };
    if accepted.session_id != incoming.session_id || accepted.key != incoming.key {
        return MonotonicWriteDecision::Apply;
    }
    if incoming.sequence <= accepted.sequence {
        return MonotonicWriteDecision::PreventReorder {
            accepted: accepted.sequence,
            incoming: incoming.sequence,
        };
    }
    if incoming.stamp < accepted.stamp {
        return MonotonicWriteDecision::PreventStaleStamp {
            accepted: accepted.stamp,
            incoming: incoming.stamp,
        };
    }
    MonotonicWriteDecision::Apply
}

/// Apply a monotonic write, returning the new accepted stamp on success.
pub fn apply_monotonic_write(
    accepted: Option<&SessionWriteStamp>,
    incoming: SessionWriteStamp,
) -> Result<SessionWriteStamp, MonotonicWriteViolation> {
    match resolve_monotonic_write(accepted, &incoming) {
        MonotonicWriteDecision::Apply => Ok(incoming),
        decision => Err(MonotonicWriteViolation {
            accepted: accepted
                .expect("non-apply monotonic decisions require an accepted write")
                .clone(),
            incoming,
            decision,
        }),
    }
}
