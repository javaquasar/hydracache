use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::cluster::{LogicalDuration, LogicalTime};
use crate::grid::session_context::SessionId;

/// Logical heartbeat table for session-owned fenced locks.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionHeartbeats {
    last_seen: BTreeMap<SessionId, LogicalTime>,
}

impl SessionHeartbeats {
    /// Record a logical heartbeat for a session.
    pub fn record(&mut self, session: SessionId, now: LogicalTime) {
        self.last_seen.insert(session, now);
    }

    /// Return the last heartbeat for a session.
    pub fn last_seen(&self, session: &SessionId) -> Option<LogicalTime> {
        self.last_seen.get(session).copied()
    }

    /// Return whether a session is lost at `now`.
    pub fn is_lost(
        &self,
        session: &SessionId,
        now: LogicalTime,
        max_silence: LogicalDuration,
    ) -> bool {
        match self.last_seen(session) {
            Some(last_seen) => {
                now.as_millis().saturating_sub(last_seen.as_millis()) > max_silence.as_millis()
            }
            None => true,
        }
    }

    /// Return all sessions whose heartbeat is older than the allowed logical silence.
    pub fn lost_sessions(&self, now: LogicalTime, max_silence: LogicalDuration) -> Vec<SessionId> {
        self.last_seen
            .iter()
            .filter(|(_, last_seen)| {
                now.as_millis().saturating_sub(last_seen.as_millis()) > max_silence.as_millis()
            })
            .map(|(session, _)| session.clone())
            .collect()
    }
}
