use std::fmt;

use serde::{Deserialize, Serialize};

use crate::grid::consistency_level::ConsistencyLevel;
use crate::grid::session_context::{PartitionKey, SessionWatermark, VersionStamp};

/// Session read escalation decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadEscalation {
    /// Candidate replica covers the session watermark and can be served.
    ServeLocal,
    /// Try a stronger consistency level.
    TryHigherLevel(ConsistencyLevel),
    /// Trigger foreground read-repair.
    ReadRepair,
    /// Wait within the caller's budget, then fail if still below watermark.
    WaitThenFail,
    /// No escalation budget remains; fail loud.
    FailUnmet,
}

/// Escalation budget for a session read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionReadBudget {
    /// Whether a stronger consistency level may be attempted.
    pub allow_higher_level: bool,
    /// Whether foreground read-repair may be attempted.
    pub allow_read_repair: bool,
    /// Bounded wait budget in logical milliseconds.
    pub wait_budget_millis: u64,
}

impl SessionReadBudget {
    /// Strict causal read budget using the full escalation ladder.
    pub const fn strict() -> Self {
        Self {
            allow_higher_level: true,
            allow_read_repair: true,
            wait_budget_millis: 1,
        }
    }

    /// Budget that immediately fails rather than serving stale.
    pub const fn fail_fast() -> Self {
        Self {
            allow_higher_level: false,
            allow_read_repair: false,
            wait_budget_millis: 0,
        }
    }
}

/// Error returned when a session guarantee cannot be met within budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionGuaranteeUnmet {
    /// Key whose watermark could not be satisfied.
    pub key: PartitionKey,
    /// Required session stamp.
    pub required: VersionStamp,
    /// Candidate replica stamp.
    pub candidate: VersionStamp,
}

impl fmt::Display for SessionGuaranteeUnmet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "session guarantee unmet for partition {}: candidate {:?} below required {:?}",
            self.key.partition.value(),
            self.candidate,
            self.required
        )
    }
}

impl std::error::Error for SessionGuaranteeUnmet {}

/// Session RYW metrics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRywMetrics {
    /// Escalations triggered by session RYW checks.
    pub session_ryw_escalations_total: u64,
    /// Reads that failed rather than serving stale.
    pub session_guarantee_unmet_total: u64,
}

/// Resolve a session read against the current watermark.
pub fn resolve_session_read(
    watermark: &SessionWatermark,
    key: &PartitionKey,
    replica_stamp: VersionStamp,
    budget: SessionReadBudget,
) -> ReadEscalation {
    let Some(required) = watermark.highest_seen(key) else {
        return ReadEscalation::ServeLocal;
    };
    if replica_stamp >= required {
        return ReadEscalation::ServeLocal;
    }
    if budget.allow_higher_level {
        return ReadEscalation::TryHigherLevel(ConsistencyLevel::Quorum);
    }
    if budget.allow_read_repair {
        return ReadEscalation::ReadRepair;
    }
    if budget.wait_budget_millis > 0 {
        return ReadEscalation::WaitThenFail;
    }
    ReadEscalation::FailUnmet
}

/// Serve a session read and update the watermark, or fail loud if below the watermark.
pub fn serve_session_read(
    watermark: &mut SessionWatermark,
    key: PartitionKey,
    replica_stamp: VersionStamp,
    budget: SessionReadBudget,
) -> Result<ReadEscalation, SessionGuaranteeUnmet> {
    match resolve_session_read(watermark, &key, replica_stamp, budget) {
        ReadEscalation::ServeLocal => {
            watermark.observe(key, replica_stamp);
            Ok(ReadEscalation::ServeLocal)
        }
        ReadEscalation::FailUnmet => {
            let required = watermark.highest_seen(&key).unwrap_or(replica_stamp);
            Err(SessionGuaranteeUnmet {
                key,
                required,
                candidate: replica_stamp,
            })
        }
        escalation => Ok(escalation),
    }
}
