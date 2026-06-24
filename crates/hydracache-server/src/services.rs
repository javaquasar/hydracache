use std::time::Duration;

use serde::Serialize;

/// Background service set tracked by the daemon.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServiceSet {
    running: bool,
    in_flight: usize,
}

impl ServiceSet {
    /// Start background services.
    pub fn start(&mut self) {
        self.running = true;
    }

    /// Stop background services.
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Track a newly accepted request.
    pub fn begin_request(&mut self) {
        self.in_flight = self.in_flight.saturating_add(1);
    }

    /// Track a completed request.
    pub fn finish_request(&mut self) {
        self.in_flight = self.in_flight.saturating_sub(1);
    }

    /// Return active request count.
    pub fn in_flight(&self) -> usize {
        self.in_flight
    }

    /// Return whether services are running.
    pub fn is_running(&self) -> bool {
        self.running
    }
}

/// Graceful drain controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GracefulShutdown {
    drain_timeout: Duration,
}

impl GracefulShutdown {
    /// Create a drain controller.
    pub fn new(drain_timeout: Duration) -> Self {
        Self { drain_timeout }
    }

    /// Drain in-flight work in a deterministic fast-test model.
    pub fn drain(&self, services: &mut ServiceSet) -> DrainOutcome {
        let started_with = services.in_flight();
        let timed_out = self.drain_timeout.is_zero() && started_with > 0;
        if !timed_out {
            while services.in_flight() > 0 {
                services.finish_request();
            }
        }
        DrainOutcome {
            started_with,
            remaining: services.in_flight(),
            timed_out,
        }
    }
}

/// Result of a graceful drain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DrainOutcome {
    /// Requests observed when drain started.
    pub started_with: usize,
    /// Requests still active after the drain window.
    pub remaining: usize,
    /// Whether the drain window timed out.
    pub timed_out: bool,
}
