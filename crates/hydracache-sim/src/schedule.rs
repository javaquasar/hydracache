use hydracache::LogicalDuration;

use crate::{SimOutcome, SimWorld};

/// Fault schedule for deterministic replay.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FaultSchedule {
    faults: Vec<ScheduledFault>,
}

impl FaultSchedule {
    /// Create an empty schedule.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a schedule from faults, sorted by step and stable insertion data.
    pub fn from_faults(mut faults: Vec<ScheduledFault>) -> Self {
        faults.sort();
        Self { faults }
    }

    /// Add one fault.
    pub fn push(&mut self, fault: ScheduledFault) {
        self.faults.push(fault);
        self.faults.sort();
    }

    /// Return all scheduled faults.
    pub fn faults(&self) -> &[ScheduledFault] {
        &self.faults
    }

    /// Return faults scheduled for a step.
    pub fn faults_at(&self, step: u64) -> impl Iterator<Item = &ScheduledFault> {
        self.faults.iter().filter(move |fault| fault.step == step)
    }

    fn without_index(&self, index: usize) -> Self {
        let mut faults = self.faults.clone();
        faults.remove(index);
        Self::from_faults(faults)
    }
}

/// One fault injected at a deterministic scheduler step.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScheduledFault {
    /// One-based scheduler step.
    pub step: u64,
    /// Fault payload.
    pub kind: ScheduledFaultKind,
}

impl ScheduledFault {
    /// Create a scheduled fault.
    pub fn new(step: u64, kind: ScheduledFaultKind) -> Self {
        Self { step, kind }
    }
}

/// Fault kinds understood by the replay harness.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScheduledFaultKind {
    /// Drop the next directed packet.
    NetworkDrop { from: String, to: String },
    /// Delay the next directed packet.
    NetworkDelay {
        from: String,
        to: String,
        duration: LogicalDuration,
    },
    /// Partition one directed link.
    NetworkPartition { from: String, to: String },
    /// Heal one directed link.
    NetworkHeal { from: String, to: String },
    /// Corrupt a storage key.
    StorageCorruption { node: String, key: String },
    /// Crash a node.
    Crash { node: String },
    /// Restart a node.
    Restart { node: String },
    /// Synthetic failure used to test replay/shrinking mechanics.
    SyntheticViolation { invariant: String },
}

/// Reproducible failure report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureReport {
    /// Seed needed to reproduce the failure.
    pub seed: u64,
    /// Step at which the failure was observed.
    pub step: u64,
    /// Schedule used for the run.
    pub schedule: FaultSchedule,
    /// Human-readable trace.
    pub trace: Vec<String>,
}

/// Replay result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayOutcome {
    /// Seed used by the run.
    pub seed: u64,
    /// Requested step budget.
    pub steps: u64,
    /// Last simulator outcome.
    pub sim: SimOutcome,
    /// Failure, if one occurred.
    pub failure: Option<FailureReport>,
}

/// Replay and shrinking harness.
#[derive(Debug, Clone, Default)]
pub struct ReplayRunner;

impl ReplayRunner {
    /// Run a deterministic replay.
    pub fn run(&self, seed: u64, steps: u64, schedule: FaultSchedule) -> ReplayOutcome {
        let mut world = SimWorld::new(seed, Default::default());
        let mut trace = Vec::new();
        for step in 1..=steps {
            let step_faults = schedule.faults_at(step).cloned().collect::<Vec<_>>();
            for fault in step_faults {
                trace.push(format!("step {step}: {:?}", fault.kind));
                match fault.kind {
                    ScheduledFaultKind::NetworkDrop { from, to } => {
                        world.drop_next_on_link(from, to);
                    }
                    ScheduledFaultKind::NetworkDelay { from, to, duration } => {
                        world.delay_next_on_link(from, to, duration);
                    }
                    ScheduledFaultKind::NetworkPartition { from, to } => {
                        world.partition_link(from, to);
                    }
                    ScheduledFaultKind::NetworkHeal { from, to } => {
                        world.heal_link(from, to);
                    }
                    ScheduledFaultKind::Crash { node } => {
                        world.crash_node(node);
                    }
                    ScheduledFaultKind::Restart { node } => {
                        world.restart_node(node);
                    }
                    ScheduledFaultKind::StorageCorruption { .. } => {}
                    ScheduledFaultKind::SyntheticViolation { invariant } => {
                        return ReplayOutcome {
                            seed,
                            steps,
                            sim: world.outcome(),
                            failure: Some(FailureReport {
                                seed,
                                step,
                                schedule,
                                trace: vec![format!("synthetic violation: {invariant}")],
                            }),
                        };
                    }
                }
            }
            world.step();
        }
        ReplayOutcome {
            seed,
            steps,
            sim: world.outcome(),
            failure: None,
        }
    }

    /// Shrink a schedule while preserving a caller-defined failure predicate.
    pub fn shrink_with(
        &self,
        schedule: FaultSchedule,
        mut fails: impl FnMut(&FaultSchedule) -> bool,
    ) -> FaultSchedule {
        let mut current = schedule;
        let mut changed = true;
        while changed {
            changed = false;
            let mut index = 0;
            while index < current.faults.len() {
                let candidate = current.without_index(index);
                if fails(&candidate) {
                    current = candidate;
                    changed = true;
                } else {
                    index += 1;
                }
            }
        }
        current
    }

    /// Shrink a schedule while preserving replay failure for the same seed/steps.
    pub fn shrink_failure(&self, seed: u64, steps: u64, schedule: FaultSchedule) -> FaultSchedule {
        self.shrink_with(schedule, |candidate| {
            self.run(seed, steps, candidate.clone()).failure.is_some()
        })
    }
}
