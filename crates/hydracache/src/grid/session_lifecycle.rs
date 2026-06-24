use serde::{Deserialize, Serialize};

use crate::grid::elasticity::RegionId;
use crate::grid::session_context::{
    SessionId, SessionRequest, SessionToken, SessionTokenError, SessionWatermark,
};

/// Session token TTL in logical milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTtl {
    ttl_millis: u64,
}

impl SessionTtl {
    /// Create a TTL from milliseconds, normalizing zero to one millisecond.
    pub const fn from_millis(ttl_millis: u64) -> Self {
        Self {
            ttl_millis: if ttl_millis == 0 { 1 } else { ttl_millis },
        }
    }

    /// Create a TTL from seconds.
    pub const fn from_secs(ttl_secs: u64) -> Self {
        Self::from_millis(ttl_secs.saturating_mul(1_000))
    }

    /// Return the TTL in logical milliseconds.
    pub const fn as_millis(self) -> u64 {
        self.ttl_millis
    }

    /// Return whether a token issued at `issued_at_millis` is expired at `now_millis`.
    pub fn is_expired(self, issued_at_millis: u64, now_millis: u64) -> bool {
        now_millis.saturating_sub(issued_at_millis) > self.ttl_millis
    }
}

impl Default for SessionTtl {
    fn default() -> Self {
        Self::from_secs(900)
    }
}

/// Session lifecycle validation decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionLifecycleDecision {
    /// Token is valid and can preserve session guarantees.
    Accepted(SessionRequest),
    /// Token expired; caller should rebuild and temporarily use sessionless reads.
    RebuildSessionless(SessionRequest),
}

/// Validate a token's MAC/nonce/session and TTL.
pub fn validate_session_lifecycle(
    token: &SessionToken,
    expected_session: &SessionId,
    secret: &[u8],
    min_nonce: u64,
    ttl: SessionTtl,
    now_millis: u64,
) -> Result<SessionLifecycleDecision, SessionTokenError> {
    token.verify(expected_session, secret, min_nonce)?;
    if ttl.is_expired(token.issued_at_millis, now_millis) {
        return Err(SessionTokenError::Expired);
    }
    Ok(SessionLifecycleDecision::Accepted(SessionRequest::Session(
        token.clone(),
    )))
}

/// Convert an expired-token outcome into the safe downgrade path.
pub fn rebuild_expired_sessionless(
    error: SessionTokenError,
) -> Result<SessionRequest, SessionTokenError> {
    match error {
        SessionTokenError::Expired => Ok(SessionRequest::Sessionless),
        other => Err(other),
    }
}

/// Session failover recovery action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionFailoverAction {
    /// No session dependencies exist, so no repair is needed.
    PreserveEmpty,
    /// The promoted region must repair up to the client-carried watermark before serving.
    RepairToWatermark,
    /// Repair is unavailable; downgrade to sessionless and rebuild.
    RebuildSessionless,
}

/// Bounded failover recovery report for diagnostics/audit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionFailoverRecovery {
    /// Promoted region that will own the session after failover.
    pub promoted_region: RegionId,
    /// Recovery action.
    pub action: SessionFailoverAction,
    /// Number of client-carried watermark entries that survived server failover.
    pub watermark_entries: u64,
    /// Whether session guarantees are preserved.
    pub guarantees_preserved: bool,
}

/// Recover a client-carried session after region promotion.
pub fn recover_session_after_failover(
    watermark: &SessionWatermark,
    promoted_region: impl Into<RegionId>,
    repair_available: bool,
) -> SessionFailoverRecovery {
    let promoted_region = promoted_region.into();
    if watermark.is_empty() {
        return SessionFailoverRecovery {
            promoted_region,
            action: SessionFailoverAction::PreserveEmpty,
            watermark_entries: 0,
            guarantees_preserved: true,
        };
    }

    if repair_available {
        SessionFailoverRecovery {
            promoted_region,
            action: SessionFailoverAction::RepairToWatermark,
            watermark_entries: watermark.len() as u64,
            guarantees_preserved: true,
        }
    } else {
        SessionFailoverRecovery {
            promoted_region,
            action: SessionFailoverAction::RebuildSessionless,
            watermark_entries: watermark.len() as u64,
            guarantees_preserved: false,
        }
    }
}
