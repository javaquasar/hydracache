use hydracache::{ClusterClock, LogicalDuration, LogicalTime};

/// Scheduler-owned deterministic clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SimClock {
    now: LogicalTime,
}

impl SimClock {
    /// Create a clock at `now`.
    pub const fn new(now: LogicalTime) -> Self {
        Self { now }
    }

    /// Return the current logical time.
    pub const fn now(self) -> LogicalTime {
        self.now
    }

    /// Move to an absolute logical time.
    pub fn set(&mut self, now: LogicalTime) {
        self.now = now;
    }

    /// Advance by a deterministic duration.
    pub fn advance(&mut self, duration: LogicalDuration) {
        self.now = self.now.saturating_add(duration);
    }
}

impl ClusterClock for SimClock {
    fn now(&self) -> LogicalTime {
        self.now
    }
}
