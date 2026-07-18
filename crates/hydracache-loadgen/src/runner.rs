use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use crate::knee::{KneeResult, PhaseAccounting, RepeatEvidence};
use crate::rate::{run_open_loop, OpenLoopConfig, OpenLoopObservation};
use crate::scenario::Scenario;
use crate::target::{Target, TargetError, TargetRequest};

/// Reset/preload/warm-up/steady configuration for one repeat.
#[derive(Debug, Clone)]
pub struct PhaseConfig {
    pub preload_operations: u64,
    pub warmup_operations: u64,
    pub steady: OpenLoopConfig,
}

/// One repeat with explicit initial-state and phase accounting.
#[derive(Debug, Clone)]
pub struct PhaseRun {
    pub initial_state_digest: String,
    pub preloaded_state_digest: String,
    pub preload_operations: u64,
    pub warmup_operations: u64,
    pub reset_ms: u64,
    pub preload_ms: u64,
    pub warmup_ms: u64,
    pub steady: OpenLoopObservation,
    pub warmup_samples_in_steady_histogram: u64,
}

impl PhaseRun {
    /// Convert an executed phase into the canonical serialized repeat evidence.
    pub fn into_evidence(self) -> RepeatEvidence {
        RepeatEvidence {
            reset_state_digest: self.initial_state_digest,
            state_digest: self.preloaded_state_digest,
            phase: PhaseAccounting {
                reset_operations: 1,
                preload_operations: self.preload_operations,
                warmup_operations: self.warmup_operations,
                steady_operations: self.steady.offered,
                reset_ms: self.reset_ms,
                preload_ms: self.preload_ms,
                warmup_ms: self.warmup_ms,
                steady_ms: self.steady.elapsed_ms,
                warmup_samples_in_steady_histogram: self.warmup_samples_in_steady_histogram,
            },
            steady: self.steady,
        }
    }
}

/// Reset and preload a target, warm it outside the histogram, then run one steady window.
pub async fn run_phases<T: Target>(
    target: Arc<T>,
    config: &PhaseConfig,
) -> Result<PhaseRun, TargetError> {
    let reset_started = Instant::now();
    let initial_state_digest = target.reset().await?;
    let reset_ms = millis(reset_started.elapsed());
    let preload_started = Instant::now();
    let preload = target.preload().await?;
    let preload_ms = millis(preload_started.elapsed());
    if preload.operations != config.preload_operations {
        return Err(TargetError::Preload(format!(
            "declared {} preload operations but target reported {}",
            config.preload_operations, preload.operations
        )));
    }
    if preload.state_digest.is_empty() {
        return Err(TargetError::Preload(
            "preloaded state digest must be non-empty".to_owned(),
        ));
    }
    let warmup_started = Instant::now();
    for sequence in 0..config.warmup_operations {
        let _ = target.execute(TargetRequest { sequence }).await;
    }
    let warmup_ms = millis(warmup_started.elapsed());
    let steady = run_open_loop(Arc::clone(&target), &config.steady)
        .await
        .map_err(TargetError::Measurement)?;
    Ok(PhaseRun {
        initial_state_digest,
        preloaded_state_digest: preload.state_digest,
        preload_operations: config.preload_operations,
        warmup_operations: config.warmup_operations,
        reset_ms,
        preload_ms,
        warmup_ms,
        steady,
        warmup_samples_in_steady_histogram: 0,
    })
}

/// Execute every declared rate and repeat, then derive the auditable capacity knee.
pub async fn run_scenario<T: Target>(
    target: Arc<T>,
    scenario: &Scenario,
) -> Result<KneeResult, TargetError> {
    scenario.validate().map_err(TargetError::Measurement)?;
    let criteria = scenario.sustainability_criteria();
    let mut points = Vec::with_capacity(scenario.offered_rates_per_second.len());
    for rate in &scenario.offered_rates_per_second {
        let config = PhaseConfig {
            preload_operations: scenario.preload_operations,
            warmup_operations: scenario.warmup_operations,
            steady: OpenLoopConfig {
                offered_rate_per_second: *rate,
                operations: scenario.steady_operations,
                highest_trackable_latency: Duration::from_micros(
                    scenario.highest_trackable_latency_us,
                ),
                significant_figures: scenario.histogram_significant_figures,
                p999_min_samples: scenario.p999_min_samples,
                drain_timeout: Duration::from_millis(scenario.backlog_drain_ms),
            },
        };
        let mut repeats = Vec::with_capacity(scenario.repeats as usize);
        for _ in 0..scenario.repeats {
            repeats.push(
                run_phases(Arc::clone(&target), &config)
                    .await?
                    .into_evidence(),
            );
        }
        points.push(criteria.evaluate_repeats(*rate as f64, repeats));
    }
    Ok(criteria.find_knee(points))
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// Small deterministic config used by contract tests and target smoke tests.
pub fn smoke_open_loop_config(operations: u64) -> OpenLoopConfig {
    OpenLoopConfig {
        offered_rate_per_second: 10_000,
        operations,
        highest_trackable_latency: Duration::from_secs(5),
        significant_figures: 3,
        p999_min_samples: 1_000,
        drain_timeout: Duration::from_secs(2),
    }
}
