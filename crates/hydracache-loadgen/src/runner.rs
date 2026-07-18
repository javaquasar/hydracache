use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use crate::rate::{run_open_loop, OpenLoopConfig, OpenLoopObservation};
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
    pub preload_operations: u64,
    pub warmup_operations: u64,
    pub reset_ms: u64,
    pub preload_ms: u64,
    pub warmup_ms: u64,
    pub steady: OpenLoopObservation,
    pub warmup_samples_in_steady_histogram: u64,
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
    target.preload().await?;
    let preload_ms = millis(preload_started.elapsed());
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
        preload_operations: config.preload_operations,
        warmup_operations: config.warmup_operations,
        reset_ms,
        preload_ms,
        warmup_ms,
        steady,
        warmup_samples_in_steady_histogram: 0,
    })
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
