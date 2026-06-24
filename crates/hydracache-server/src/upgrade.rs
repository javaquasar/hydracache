use serde::Serialize;
use thiserror::Error;

/// Listener handoff strategy used during a zero-downtime upgrade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpgradeStrategy {
    /// A newly spawned process receives the already-bound listener.
    InheritedSocket,
    /// Old and new processes overlap on the same address through reuse-port style binding.
    ReusePort,
}

impl UpgradeStrategy {
    /// Return the default strategy for the current platform.
    pub fn platform_default() -> Self {
        if cfg!(windows) {
            Self::ReusePort
        } else {
            Self::InheritedSocket
        }
    }
}

/// Upgrade phases visible to readiness checks and diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpgradePhase {
    /// Handoff has been prepared, but the replacement is not ready yet.
    Prepared,
    /// Replacement process is ready to accept traffic.
    NewReady,
    /// Old process stopped accepting and is draining active work.
    OldDraining,
    /// Upgrade finished without dropped in-flight work.
    Complete,
}

/// Operator-provided upgrade plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradePlan {
    generation: u64,
    member_id: String,
    strategy: UpgradeStrategy,
}

impl UpgradePlan {
    /// Create a plan for the next process generation.
    pub fn new(generation: u64, member_id: impl Into<String>) -> Self {
        Self {
            generation,
            member_id: member_id.into(),
            strategy: UpgradeStrategy::platform_default(),
        }
    }

    /// Override the listener handoff strategy.
    pub fn with_strategy(mut self, strategy: UpgradeStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Validate and prepare the handoff.
    pub fn prepare(self) -> Result<GracefulUpgrade, UpgradeError> {
        if self.generation == 0 {
            return Err(UpgradeError::InvalidGeneration);
        }
        if self.member_id.trim().is_empty() {
            return Err(UpgradeError::MissingMemberId);
        }
        Ok(GracefulUpgrade {
            plan: self,
            phase: UpgradePhase::Prepared,
            old_accepting: true,
            new_ready: false,
            in_flight: 0,
            completed: 0,
        })
    }
}

/// Deterministic zero-downtime handoff model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GracefulUpgrade {
    plan: UpgradePlan,
    phase: UpgradePhase,
    old_accepting: bool,
    new_ready: bool,
    in_flight: usize,
    completed: usize,
}

impl GracefulUpgrade {
    /// Mark the replacement process ready before the old process drains.
    pub fn mark_new_ready(&mut self) {
        self.new_ready = true;
        self.phase = UpgradePhase::NewReady;
    }

    /// Stop the old process from accepting new work and start drain.
    pub fn start_draining_old(&mut self) -> Result<(), UpgradeError> {
        if !self.new_ready {
            return Err(UpgradeError::ReplacementNotReady);
        }
        self.old_accepting = false;
        self.phase = UpgradePhase::OldDraining;
        Ok(())
    }

    /// Record work accepted by the old process while it is still serving.
    pub fn record_request(&mut self) -> bool {
        if !self.old_accepting {
            return false;
        }
        self.in_flight = self.in_flight.saturating_add(1);
        true
    }

    /// Mark one in-flight request as completed.
    pub fn finish_request(&mut self) {
        if self.in_flight > 0 {
            self.in_flight -= 1;
            self.completed = self.completed.saturating_add(1);
        }
    }

    /// Finish upgrade after all in-flight work is drained.
    pub fn complete(mut self) -> Result<UpgradeReport, UpgradeError> {
        if !self.new_ready {
            return Err(UpgradeError::ReplacementNotReady);
        }
        if self.in_flight > 0 {
            return Err(UpgradeError::InFlightRequestsRemaining(self.in_flight));
        }
        self.phase = UpgradePhase::Complete;
        Ok(UpgradeReport {
            generation: self.plan.generation,
            member_id: self.plan.member_id,
            strategy: self.plan.strategy,
            phase: self.phase,
            completed_requests: self.completed,
            dropped_requests: 0,
        })
    }

    /// Return current phase.
    pub fn phase(&self) -> UpgradePhase {
        self.phase
    }

    /// Return whether old and replacement process keep the same member identity.
    pub fn membership_stable(&self) -> bool {
        !self.plan.member_id.trim().is_empty()
    }

    /// Return active work still attached to the old process.
    pub fn in_flight(&self) -> usize {
        self.in_flight
    }

    /// Return whether the old process still accepts new traffic.
    pub fn old_accepting(&self) -> bool {
        self.old_accepting
    }
}

/// Successful upgrade result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpgradeReport {
    /// Process generation that completed.
    pub generation: u64,
    /// Stable cluster member identity.
    pub member_id: String,
    /// Listener handoff strategy used.
    pub strategy: UpgradeStrategy,
    /// Final phase.
    pub phase: UpgradePhase,
    /// Requests completed while old process drained.
    pub completed_requests: usize,
    /// Requests dropped by the handoff.
    pub dropped_requests: usize,
}

/// Fail-loud upgrade errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum UpgradeError {
    /// Generation zero is reserved for uninitialized runtimes.
    #[error("upgrade generation must be greater than zero")]
    InvalidGeneration,
    /// Cluster member identity must remain explicit across handoff.
    #[error("upgrade requires a non-empty member id")]
    MissingMemberId,
    /// The old process cannot drain before the replacement is ready.
    #[error("replacement process is not ready")]
    ReplacementNotReady,
    /// Upgrade cannot complete while old process still owns work.
    #[error("{0} in-flight request(s) remain during upgrade")]
    InFlightRequestsRemaining(usize),
}
