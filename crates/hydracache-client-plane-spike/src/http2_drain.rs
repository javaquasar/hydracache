//! Bounded, transport-neutral HTTP/2 drain accounting for the HC/2 spike.
//!
//! The concrete `h2` fixture owns wire I/O. This state machine makes the
//! safety decisions explicit: stop admission with the first GOAWAY, allow
//! active streams to finish, send the final GOAWAY, attempt TLS close-notify,
//! and force termination only after a bounded deadline or an explicit reset.

use std::num::NonZeroU64;

use thiserror::Error;

/// Actor or wire event that initiated connection drain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrainInitiator {
    ServerShutdown,
    ClientHalfClose,
    PeerGoAway,
}

/// Observable phase of the bounded drain protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrainPhase {
    Open,
    FirstGoAwayPending,
    WaitingForActiveStreams,
    FinalGoAwayPending,
    WaitingForTlsCloseNotify,
    Closed,
}

/// Terminal reason. Labels are fixed and contain no peer-controlled data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrainReason {
    Graceful,
    DeadlineExpired,
    PeerReset,
}

/// Bounded deadline expressed in deterministic monotonic ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrainPolicy {
    deadline_ticks: NonZeroU64,
}

impl DrainPolicy {
    pub fn new(deadline_ticks: u64) -> Result<Self, DrainError> {
        Ok(Self {
            deadline_ticks: NonZeroU64::new(deadline_ticks).ok_or(DrainError::InvalidDeadline)?,
        })
    }

    pub const fn deadline_ticks(self) -> u64 {
        self.deadline_ticks.get()
    }
}

/// Privacy-safe, bounded-cardinality diagnostic counters for H20 evidence.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DrainMetrics {
    pub goaway_frames: u64,
    pub refused_new_streams: u64,
    pub completed_streams: u64,
    pub tls_close_notify_attempts: u64,
    pub tls_close_notify_successes: u64,
    pub deadline_forced_closes: u64,
    pub peer_resets: u64,
    pub forced_stream_terminations: usize,
}

/// Immutable diagnostic view of the drain controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrainSnapshot {
    pub phase: DrainPhase,
    pub initiator: Option<DrainInitiator>,
    pub terminal_reason: Option<DrainReason>,
    pub active_streams: usize,
    pub deadline_tick: Option<u64>,
    pub metrics: DrainMetrics,
}

/// Fail-closed lifecycle errors.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum DrainError {
    #[error("HTTP/2 drain deadline must be nonzero")]
    InvalidDeadline,
    #[error("new HTTP/2 streams are refused after drain starts")]
    NewStreamRefused,
    #[error("HTTP/2 drain transition is not legal in the current phase")]
    InvalidTransition,
    #[error("HTTP/2 active-stream accounting underflow")]
    ActiveStreamUnderflow,
    #[error("HTTP/2 drain tick moved backwards")]
    NonMonotonicTick,
    #[error("HTTP/2 drain deadline overflowed")]
    DeadlineOverflow,
}

/// Deterministic drain controller shared by candidate tests and adapters.
#[derive(Debug)]
pub struct Http2DrainController {
    policy: DrainPolicy,
    phase: DrainPhase,
    initiator: Option<DrainInitiator>,
    terminal_reason: Option<DrainReason>,
    active_streams: usize,
    now_tick: u64,
    deadline_tick: Option<u64>,
    metrics: DrainMetrics,
}

impl Http2DrainController {
    pub fn new(policy: DrainPolicy) -> Self {
        Self {
            policy,
            phase: DrainPhase::Open,
            initiator: None,
            terminal_reason: None,
            active_streams: 0,
            now_tick: 0,
            deadline_tick: None,
            metrics: DrainMetrics::default(),
        }
    }

    /// Admit one stream only before the first GOAWAY is initiated.
    pub fn open_stream(&mut self) -> Result<(), DrainError> {
        if self.phase != DrainPhase::Open {
            self.metrics.refused_new_streams += 1;
            return Err(DrainError::NewStreamRefused);
        }
        self.active_streams = self
            .active_streams
            .checked_add(1)
            .ok_or(DrainError::InvalidTransition)?;
        Ok(())
    }

    /// Start the two-GOAWAY sequence and establish its absolute deadline.
    pub fn begin(&mut self, initiator: DrainInitiator, now_tick: u64) -> Result<(), DrainError> {
        if self.phase != DrainPhase::Open {
            return Err(DrainError::InvalidTransition);
        }
        self.advance_clock(now_tick)?;
        self.deadline_tick = Some(
            now_tick
                .checked_add(self.policy.deadline_ticks())
                .ok_or(DrainError::DeadlineOverflow)?,
        );
        self.initiator = Some(initiator);
        self.phase = DrainPhase::FirstGoAwayPending;
        Ok(())
    }

    /// Record that the first GOAWAY reached the transport.
    pub fn first_goaway_flushed(&mut self) -> Result<(), DrainError> {
        if self.phase != DrainPhase::FirstGoAwayPending {
            return Err(DrainError::InvalidTransition);
        }
        self.metrics.goaway_frames += 1;
        self.phase = if self.active_streams == 0 {
            DrainPhase::FinalGoAwayPending
        } else {
            DrainPhase::WaitingForActiveStreams
        };
        Ok(())
    }

    /// Complete one previously admitted stream.
    pub fn complete_stream(&mut self) -> Result<(), DrainError> {
        if self.active_streams == 0 || self.phase == DrainPhase::Closed {
            return Err(DrainError::ActiveStreamUnderflow);
        }
        self.active_streams -= 1;
        self.metrics.completed_streams += 1;
        if self.active_streams == 0 && self.phase == DrainPhase::WaitingForActiveStreams {
            self.phase = DrainPhase::FinalGoAwayPending;
        }
        Ok(())
    }

    /// Record the final GOAWAY and begin the TLS close-notify attempt.
    pub fn final_goaway_flushed(&mut self) -> Result<(), DrainError> {
        if self.phase != DrainPhase::FinalGoAwayPending || self.active_streams != 0 {
            return Err(DrainError::InvalidTransition);
        }
        self.metrics.goaway_frames += 1;
        self.metrics.tls_close_notify_attempts += 1;
        self.phase = DrainPhase::WaitingForTlsCloseNotify;
        Ok(())
    }

    /// Finish a fully cooperative connection without forced termination.
    pub fn tls_close_notify_completed(&mut self) -> Result<(), DrainError> {
        if self.phase != DrainPhase::WaitingForTlsCloseNotify {
            return Err(DrainError::InvalidTransition);
        }
        self.metrics.tls_close_notify_successes += 1;
        self.terminal_reason = Some(DrainReason::Graceful);
        self.phase = DrainPhase::Closed;
        Ok(())
    }

    /// Advance the bounded deadline; returns the terminal outcome if it fired.
    pub fn advance_to(&mut self, now_tick: u64) -> Result<Option<DrainReason>, DrainError> {
        self.advance_clock(now_tick)?;
        if self.phase == DrainPhase::Open || self.phase == DrainPhase::Closed {
            return Ok(self.terminal_reason);
        }
        if self
            .deadline_tick
            .is_some_and(|deadline| now_tick >= deadline)
        {
            self.metrics.deadline_forced_closes += 1;
            self.metrics.forced_stream_terminations += self.active_streams;
            self.active_streams = 0;
            self.terminal_reason = Some(DrainReason::DeadlineExpired);
            self.phase = DrainPhase::Closed;
            return Ok(self.terminal_reason);
        }
        Ok(None)
    }

    /// Close immediately after an explicit peer reset, never labelling it graceful.
    pub fn peer_reset(&mut self) -> Result<DrainReason, DrainError> {
        if self.phase == DrainPhase::Closed {
            return Err(DrainError::InvalidTransition);
        }
        self.metrics.peer_resets += 1;
        self.metrics.forced_stream_terminations += self.active_streams;
        self.active_streams = 0;
        self.terminal_reason = Some(DrainReason::PeerReset);
        self.phase = DrainPhase::Closed;
        Ok(DrainReason::PeerReset)
    }

    pub const fn snapshot(&self) -> DrainSnapshot {
        DrainSnapshot {
            phase: self.phase,
            initiator: self.initiator,
            terminal_reason: self.terminal_reason,
            active_streams: self.active_streams,
            deadline_tick: self.deadline_tick,
            metrics: self.metrics,
        }
    }

    fn advance_clock(&mut self, now_tick: u64) -> Result<(), DrainError> {
        if now_tick < self.now_tick {
            return Err(DrainError::NonMonotonicTick);
        }
        self.now_tick = now_tick;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn controller() -> Http2DrainController {
        Http2DrainController::new(DrainPolicy::new(10).unwrap())
    }

    #[test]
    fn clean_drain_sends_two_goaways_and_close_notify() {
        let mut drain = controller();
        drain.begin(DrainInitiator::ServerShutdown, 3).unwrap();
        drain.first_goaway_flushed().unwrap();
        drain.final_goaway_flushed().unwrap();
        drain.tls_close_notify_completed().unwrap();
        let snapshot = drain.snapshot();
        assert_eq!(snapshot.phase, DrainPhase::Closed);
        assert_eq!(snapshot.terminal_reason, Some(DrainReason::Graceful));
        assert_eq!(snapshot.metrics.goaway_frames, 2);
        assert_eq!(snapshot.metrics.tls_close_notify_successes, 1);
        assert_eq!(snapshot.metrics.deadline_forced_closes, 0);
    }

    #[test]
    fn active_stream_finishes_before_final_goaway_and_new_stream_is_refused() {
        let mut drain = controller();
        drain.open_stream().unwrap();
        drain.begin(DrainInitiator::ServerShutdown, 0).unwrap();
        drain.first_goaway_flushed().unwrap();
        assert_eq!(drain.open_stream(), Err(DrainError::NewStreamRefused));
        assert_eq!(
            drain.final_goaway_flushed(),
            Err(DrainError::InvalidTransition)
        );
        drain.complete_stream().unwrap();
        assert_eq!(drain.snapshot().phase, DrainPhase::FinalGoAwayPending);
        drain.final_goaway_flushed().unwrap();
        drain.tls_close_notify_completed().unwrap();
        assert_eq!(drain.snapshot().metrics.completed_streams, 1);
        assert_eq!(drain.snapshot().metrics.refused_new_streams, 1);
    }

    #[test]
    fn client_half_close_and_peer_goaway_are_explicit_initiators() {
        for initiator in [DrainInitiator::ClientHalfClose, DrainInitiator::PeerGoAway] {
            let mut drain = controller();
            drain.begin(initiator, 4).unwrap();
            assert_eq!(drain.snapshot().initiator, Some(initiator));
        }
    }

    #[test]
    fn uncooperative_peer_is_forced_only_at_deadline() {
        let mut drain = controller();
        drain.open_stream().unwrap();
        drain.begin(DrainInitiator::ServerShutdown, 7).unwrap();
        drain.first_goaway_flushed().unwrap();
        assert_eq!(drain.advance_to(16).unwrap(), None);
        assert_eq!(drain.snapshot().phase, DrainPhase::WaitingForActiveStreams);
        assert_eq!(
            drain.advance_to(17).unwrap(),
            Some(DrainReason::DeadlineExpired)
        );
        assert_eq!(drain.snapshot().active_streams, 0);
        assert_eq!(drain.snapshot().metrics.deadline_forced_closes, 1);
        assert_eq!(drain.snapshot().metrics.forced_stream_terminations, 1);
    }

    #[test]
    fn reset_is_distinct_from_deadline_and_accounting_cannot_underflow() {
        let mut drain = controller();
        drain.open_stream().unwrap();
        drain.begin(DrainInitiator::PeerGoAway, 0).unwrap();
        assert_eq!(drain.peer_reset().unwrap(), DrainReason::PeerReset);
        let snapshot = drain.snapshot();
        assert_eq!(snapshot.active_streams, 0);
        assert_eq!(snapshot.metrics.peer_resets, 1);
        assert_eq!(snapshot.metrics.forced_stream_terminations, 1);
        assert_eq!(snapshot.metrics.deadline_forced_closes, 0);
        assert_eq!(
            drain.complete_stream(),
            Err(DrainError::ActiveStreamUnderflow)
        );
    }

    #[test]
    fn invalid_deadline_clock_and_duplicate_transitions_fail_closed() {
        assert_eq!(DrainPolicy::new(0), Err(DrainError::InvalidDeadline));
        let mut drain = controller();
        drain.begin(DrainInitiator::ServerShutdown, 5).unwrap();
        assert_eq!(
            drain.begin(DrainInitiator::ServerShutdown, 5),
            Err(DrainError::InvalidTransition)
        );
        assert_eq!(drain.advance_to(4), Err(DrainError::NonMonotonicTick));
    }
}
