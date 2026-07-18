use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::time::Instant;

use crate::histogram::{LatencyHistogram, LatencySummary};
use crate::target::{Target, TargetOutcome, TargetRequest};

/// Deterministic fixed-rate schedule expressed in monotonic nanoseconds.
#[derive(Debug, Clone)]
pub struct FixedRateSchedule {
    start_ns: u64,
    interval_ns: u64,
    next_sequence: u64,
}

/// One offered operation and its original scheduled timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduledTick {
    pub sequence: u64,
    pub scheduled_ns: u64,
}

impl FixedRateSchedule {
    /// Construct a non-skipping schedule. Fractional intervals are rounded down to one nanosecond.
    pub fn new(start_ns: u64, offered_rate_per_second: u64) -> Result<Self, String> {
        if offered_rate_per_second == 0 {
            return Err("offered rate must be positive".to_owned());
        }
        Ok(Self {
            start_ns,
            interval_ns: (1_000_000_000 / offered_rate_per_second).max(1),
            next_sequence: 0,
        })
    }

    /// Return every tick due by `now_ns`; delayed callers never skip missed offers.
    pub fn due_ticks(&mut self, now_ns: u64) -> Vec<ScheduledTick> {
        let mut due = Vec::new();
        loop {
            let scheduled_ns = self
                .start_ns
                .saturating_add(self.next_sequence.saturating_mul(self.interval_ns));
            if scheduled_ns > now_ns {
                break;
            }
            due.push(ScheduledTick {
                sequence: self.next_sequence,
                scheduled_ns,
            });
            self.next_sequence = self.next_sequence.saturating_add(1);
        }
        due
    }

    /// Scheduled timestamp for a sequence without changing schedule state.
    pub fn scheduled_ns(&self, sequence: u64) -> u64 {
        self.start_ns
            .saturating_add(sequence.saturating_mul(self.interval_ns))
    }

    /// Fixed interval in nanoseconds.
    pub fn interval_ns(&self) -> u64 {
        self.interval_ns
    }
}

/// One bounded open-loop steady-window run.
#[derive(Debug, Clone)]
pub struct OpenLoopConfig {
    pub offered_rate_per_second: u64,
    pub operations: u64,
    pub highest_trackable_latency: Duration,
    pub significant_figures: u8,
    pub p999_min_samples: u64,
    pub drain_timeout: Duration,
}

/// Normalized observation emitted by the fixed-rate driver.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenLoopObservation {
    pub offered: u64,
    pub started: u64,
    pub completed: u64,
    pub successes: u64,
    pub errors: u64,
    pub timeouts: u64,
    pub rejections: u64,
    pub backlog_high_water: u64,
    pub backlog_drained: bool,
    pub drain_ms: u64,
    pub elapsed_ms: u64,
    pub offered_rate_per_second: f64,
    pub achieved_rate_per_second: f64,
    pub latency: LatencySummary,
}

struct Completion {
    scheduled: Instant,
    finished: Instant,
    outcome: TargetOutcome,
}

/// Execute a target at fixed scheduled send times without waiting for prior responses.
pub async fn run_open_loop<T: Target>(
    target: Arc<T>,
    config: &OpenLoopConfig,
) -> Result<OpenLoopObservation, String> {
    if config.offered_rate_per_second == 0 || config.operations == 0 {
        return Err("open-loop rate and operation count must be positive".to_owned());
    }
    let interval_ns = (1_000_000_000 / config.offered_rate_per_second).max(1);
    let interval = Duration::from_nanos(interval_ns);
    let origin = Instant::now();
    let (sender, mut receiver) = mpsc::unbounded_channel::<Completion>();
    let mut histogram =
        LatencyHistogram::new(config.highest_trackable_latency, config.significant_figures)?;
    let mut started = 0_u64;
    let mut completed = 0_u64;
    let mut successes = 0_u64;
    let mut errors = 0_u64;
    let mut timeouts = 0_u64;
    let mut rejections = 0_u64;
    let mut backlog_high_water = 0_u64;

    for sequence in 0..config.operations {
        let scheduled = origin + multiply_duration(interval, sequence);
        tokio::time::sleep_until(scheduled).await;
        while let Ok(completion) = receiver.try_recv() {
            account_completion(
                completion,
                &mut histogram,
                &mut completed,
                &mut successes,
                &mut errors,
                &mut timeouts,
                &mut rejections,
            );
        }
        started = started.saturating_add(1);
        backlog_high_water = backlog_high_water.max(started.saturating_sub(completed));
        let target = Arc::clone(&target);
        let sender = sender.clone();
        tokio::spawn(async move {
            let outcome = target.execute(TargetRequest { sequence }).await;
            let _ = sender.send(Completion {
                scheduled,
                finished: Instant::now(),
                outcome,
            });
        });
    }
    drop(sender);

    let drain_started = Instant::now();
    while completed < started {
        let remaining = config.drain_timeout.saturating_sub(drain_started.elapsed());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, receiver.recv()).await {
            Ok(Some(completion)) => account_completion(
                completion,
                &mut histogram,
                &mut completed,
                &mut successes,
                &mut errors,
                &mut timeouts,
                &mut rejections,
            ),
            Ok(None) | Err(_) => break,
        }
    }
    let elapsed = origin.elapsed();
    let elapsed_seconds = elapsed.as_secs_f64().max(f64::EPSILON);
    Ok(OpenLoopObservation {
        offered: config.operations,
        started,
        completed,
        successes,
        errors,
        timeouts,
        rejections,
        backlog_high_water,
        backlog_drained: completed == started,
        drain_ms: millis(drain_started.elapsed()),
        elapsed_ms: millis(elapsed),
        offered_rate_per_second: config.offered_rate_per_second as f64,
        achieved_rate_per_second: completed as f64 / elapsed_seconds,
        latency: histogram.summary(config.p999_min_samples),
    })
}

#[allow(clippy::too_many_arguments)]
fn account_completion(
    completion: Completion,
    histogram: &mut LatencyHistogram,
    completed: &mut u64,
    successes: &mut u64,
    errors: &mut u64,
    timeouts: &mut u64,
    rejections: &mut u64,
) {
    histogram.record(
        completion
            .finished
            .saturating_duration_since(completion.scheduled),
    );
    *completed = completed.saturating_add(1);
    match completion.outcome {
        TargetOutcome::Success => *successes = successes.saturating_add(1),
        TargetOutcome::Rejected => *rejections = rejections.saturating_add(1),
        TargetOutcome::Error => *errors = errors.saturating_add(1),
        TargetOutcome::Timeout => *timeouts = timeouts.saturating_add(1),
    }
}

fn multiply_duration(duration: Duration, multiplier: u64) -> Duration {
    let nanos = duration.as_nanos().saturating_mul(u128::from(multiplier));
    Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}
