use serde::{Deserialize, Serialize};

use crate::grid::hardening::{MergePolicy, ReplicatedValueRecord};
use crate::grid::session_context::{PartitionKey, SessionWatermark, VersionStamp};

/// Explicit staleness budget for a session read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StalenessBound {
    /// Maximum allowed lag in monotonic value versions.
    pub max_version_lag: u64,
}

impl StalenessBound {
    /// Create a version-lag bound.
    pub const fn versions(max_version_lag: u64) -> Self {
        Self { max_version_lag }
    }
}

/// Session read freshness mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionReadMode {
    /// Require the full session watermark before serving locally.
    #[default]
    Causal,
    /// Allow bounded lag, but never below the causal dependency floor.
    BoundedStaleness {
        /// Explicit max staleness.
        max: StalenessBound,
    },
}

/// Why a bounded-staleness read had to escalate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StalenessEscalationReason {
    /// Candidate is below a W4 causal dependency floor.
    BelowCausalFloor,
    /// Strict causal mode requires the full session watermark.
    BelowSessionWatermark,
    /// Candidate is above the causal floor but outside the chosen staleness bound.
    BeyondBound,
}

/// Bounded-staleness read decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StalenessDecision {
    /// Strict causal mode can serve locally.
    ServeCausal {
        /// Observed version lag from the session watermark.
        observed_version_lag: u64,
    },
    /// Bounded mode can serve locally within the explicit budget.
    ServeFast {
        /// Observed version lag from the session watermark.
        observed_version_lag: u64,
    },
    /// The caller must escalate through W2.
    Escalate {
        /// Escalation reason.
        reason: StalenessEscalationReason,
        /// Observed version lag from the session watermark.
        observed_version_lag: u64,
    },
}

/// Bounded staleness metrics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundedStalenessMetrics {
    /// Reads served locally by bounded staleness.
    pub bounded_staleness_fast_serves_total: u64,
    /// Bounded-staleness reads that escalated.
    pub bounded_staleness_escalations_total: u64,
}

/// Reduce concurrent replicated records to one converged value through the merge policy.
pub fn converge_replicated_values<P, I>(policy: &P, records: I) -> Option<ReplicatedValueRecord>
where
    P: MergePolicy + ?Sized,
    I: IntoIterator<Item = ReplicatedValueRecord>,
{
    let mut winner = None;
    for record in records {
        winner = policy.merge(winner.as_ref(), &record);
    }
    winner
}

/// Resolve a read under either strict causal or bounded-staleness mode.
pub fn resolve_session_read_mode(
    watermark: &SessionWatermark,
    key: &PartitionKey,
    causal_floor: Option<VersionStamp>,
    candidate: VersionStamp,
    mode: SessionReadMode,
) -> StalenessDecision {
    let observed_version_lag = watermark
        .highest_seen(key)
        .map(|seen| seen.version_distance_from(candidate))
        .unwrap_or_default();

    if causal_floor.is_some_and(|floor| candidate < floor) {
        return StalenessDecision::Escalate {
            reason: StalenessEscalationReason::BelowCausalFloor,
            observed_version_lag,
        };
    }

    match mode {
        SessionReadMode::Causal => {
            if watermark
                .highest_seen(key)
                .is_some_and(|required| candidate < required)
            {
                StalenessDecision::Escalate {
                    reason: StalenessEscalationReason::BelowSessionWatermark,
                    observed_version_lag,
                }
            } else {
                StalenessDecision::ServeCausal {
                    observed_version_lag,
                }
            }
        }
        SessionReadMode::BoundedStaleness { max }
            if observed_version_lag <= max.max_version_lag =>
        {
            StalenessDecision::ServeFast {
                observed_version_lag,
            }
        }
        SessionReadMode::BoundedStaleness { .. } => StalenessDecision::Escalate {
            reason: StalenessEscalationReason::BeyondBound,
            observed_version_lag,
        },
    }
}

/// Return whether the read can be served locally under the chosen mode.
pub fn within_staleness_bound(
    watermark: &SessionWatermark,
    key: &PartitionKey,
    causal_floor: Option<VersionStamp>,
    candidate: VersionStamp,
    mode: SessionReadMode,
) -> bool {
    matches!(
        resolve_session_read_mode(watermark, key, causal_floor, candidate, mode),
        StalenessDecision::ServeCausal { .. } | StalenessDecision::ServeFast { .. }
    )
}
