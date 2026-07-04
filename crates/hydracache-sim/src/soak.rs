use std::time::{Duration, Instant};

use serde::Serialize;

use crate::{FaultSchedule, ReplayRunner, SimConfig, SimRng, SimWorld};

/// Configuration for a continuous deterministic simulator soak.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoakConfig {
    /// Seed for the reproducible fleet of per-run simulator seeds.
    pub master_seed: u64,
    /// Wall-clock budget for the soak.
    pub budget: Duration,
    /// Scheduler steps to run for each generated seed.
    pub steps_per_seed: u64,
    /// Simulator configuration used for every generated seed.
    pub sim: SimConfig,
    /// Optional deterministic cap for tests and short gates.
    pub max_seeds: Option<u64>,
}

impl SoakConfig {
    /// Create a wall-clock-budgeted soak configuration.
    pub fn new(master_seed: u64, budget: Duration, steps_per_seed: u64, sim: SimConfig) -> Self {
        Self {
            master_seed,
            budget,
            steps_per_seed,
            sim,
            max_seeds: None,
        }
    }

    /// Add a deterministic seed cap while retaining the wall-clock budget.
    pub fn with_max_seeds(mut self, max_seeds: u64) -> Self {
        self.max_seeds = Some(max_seeds);
        self
    }
}

/// First failing seed found by a soak run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoakFailure {
    /// Simulator seed needed to reproduce the failure.
    pub seed: u64,
    /// Minimal observed step for the failing run when known.
    pub step: u64,
    /// Plain-seed step bisection result when the simulator failure reproduces.
    pub minimal_steps: Option<u64>,
    /// Human-readable invariant violations.
    pub violations: Vec<String>,
}

impl SoakFailure {
    /// Return the exact single-shot VOPR command for this failure.
    pub fn reproduce_command(&self) -> String {
        format!("vopr --seed {} --steps {}", self.seed, self.step)
    }
}

/// Summary of a continuous soak run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoakOutcome {
    /// Master seed for the generated seed fleet.
    pub master_seed: u64,
    /// Number of generated seeds executed.
    pub seeds_run: u64,
    /// Total scheduler steps executed across the fleet.
    pub total_steps: u64,
    /// First failure, if any.
    pub first_failure: Option<SoakFailure>,
}

/// Minimized failure information carried in reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Minimization {
    /// Plain-seed failures are minimized by bisecting step count.
    Steps { minimal_steps: u64 },
    /// Schedule-driven failures are minimized by shrinking the schedule.
    Schedule { faults: usize },
    /// Minimization was not run or the injected test failure is not a simulator invariant.
    NotRun { reason: String },
}

/// Score-free soak report suitable for logs and nightly artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SoakReport {
    /// Master seed for the generated seed fleet.
    pub master_seed: u64,
    /// Number of generated seeds executed.
    pub seeds_run: u64,
    /// Total scheduler steps executed across the fleet.
    pub total_steps: u64,
    /// Final report outcome.
    pub outcome: SoakReportOutcome,
}

/// Score-free soak status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum SoakReportOutcome {
    /// No invariant violation was observed before the budget ended.
    Clean,
    /// The first failing seed stopped the run.
    Failed {
        seed: u64,
        step: u64,
        reproduce: String,
        minimization: Minimization,
        violations: Vec<String>,
    },
}

impl From<&SoakOutcome> for SoakReport {
    fn from(outcome: &SoakOutcome) -> Self {
        let report_outcome = match &outcome.first_failure {
            Some(failure) => SoakReportOutcome::Failed {
                seed: failure.seed,
                step: failure.step,
                reproduce: failure.reproduce_command(),
                minimization: failure
                    .minimal_steps
                    .map(|minimal_steps| Minimization::Steps { minimal_steps })
                    .unwrap_or_else(|| Minimization::NotRun {
                        reason: "failure did not reproduce through the plain-seed simulator"
                            .to_owned(),
                    }),
                violations: failure.violations.clone(),
            },
            None => SoakReportOutcome::Clean,
        };
        Self {
            master_seed: outcome.master_seed,
            seeds_run: outcome.seeds_run,
            total_steps: outcome.total_steps,
            outcome: report_outcome,
        }
    }
}

/// Run a continuous soak against the real simulator.
pub fn run_soak(cfg: &SoakConfig) -> SoakOutcome {
    let mut outcome = run_soak_with_seed_runner(cfg, |seed, steps, sim| {
        let mut world = SimWorld::new(seed, sim.clone());
        let sim_outcome = world.run(steps);
        if sim_outcome.invariant_violations == 0 {
            None
        } else {
            Some((
                sim_outcome.steps,
                world
                    .invariant_report()
                    .violations
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            ))
        }
    });

    if let Some(failure) = outcome.first_failure.as_mut() {
        failure.minimal_steps = minimal_failing_steps(failure.seed, &cfg.sim, failure.step);
    }

    outcome
}

/// Run a soak with an injected seed runner.
///
/// This is used by deterministic tests to exercise the failure path without
/// depending on a real simulator bug. The closure returns the failing step and
/// violation strings when a seed fails.
#[doc(hidden)]
pub fn run_soak_with_seed_runner(
    cfg: &SoakConfig,
    mut run_seed: impl FnMut(u64, u64, &SimConfig) -> Option<(u64, Vec<String>)>,
) -> SoakOutcome {
    let mut fleet = SimRng::from_seed(cfg.master_seed);
    let start = Instant::now();
    let max_seeds = cfg.max_seeds.unwrap_or(u64::MAX).max(1);
    let steps_per_seed = cfg.steps_per_seed.max(1);
    let mut seeds_run = 0_u64;
    let mut total_steps = 0_u64;

    loop {
        let seed = fleet.next_u64();
        seeds_run = seeds_run.saturating_add(1);
        total_steps = total_steps.saturating_add(steps_per_seed);

        if let Some((step, violations)) = run_seed(seed, steps_per_seed, &cfg.sim) {
            return SoakOutcome {
                master_seed: cfg.master_seed,
                seeds_run,
                total_steps,
                first_failure: Some(SoakFailure {
                    seed,
                    step,
                    minimal_steps: None,
                    violations,
                }),
            };
        }

        if seeds_run >= max_seeds || start.elapsed() >= cfg.budget {
            return SoakOutcome {
                master_seed: cfg.master_seed,
                seeds_run,
                total_steps,
                first_failure: None,
            };
        }
    }
}

/// Minimize a plain-seed simulator failure by bisecting the step count.
pub fn minimal_failing_steps(seed: u64, cfg: &SimConfig, failing_steps: u64) -> Option<u64> {
    minimal_failing_steps_by(failing_steps, |steps| {
        SimWorld::new(seed, cfg.clone())
            .run(steps)
            .invariant_violations
            > 0
    })
}

/// Minimize a monotonic failure predicate by bisecting step count.
pub fn minimal_failing_steps_by(
    failing_steps: u64,
    mut fails: impl FnMut(u64) -> bool,
) -> Option<u64> {
    if failing_steps == 0 || !fails(failing_steps) {
        return None;
    }

    let mut lo = 1_u64;
    let mut hi = failing_steps;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if fails(mid) {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    Some(lo)
}

/// Shrink a fault schedule using a caller-supplied failure predicate.
pub fn shrink_failing_schedule_with(
    schedule: FaultSchedule,
    fails: impl FnMut(&FaultSchedule) -> bool,
) -> FaultSchedule {
    ReplayRunner.shrink_with(schedule, fails)
}

/// Shrink a schedule while preserving a real simulator invariant failure.
pub fn shrink_failing_schedule(seed: u64, steps: u64, schedule: FaultSchedule) -> FaultSchedule {
    let runner = ReplayRunner;
    runner.shrink_with(schedule, |candidate| {
        runner
            .run(seed, steps, candidate.clone())
            .sim
            .invariant_violations
            > 0
    })
}
