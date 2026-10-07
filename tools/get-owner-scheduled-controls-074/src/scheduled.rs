//! Bounded extension of the canonical non-skipping fixed-rate schedule.
use std::sync::Arc;
use std::time::Duration;

use hdrhistogram::Histogram;
use serde::Serialize;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time::Instant;

use crate::rate::FixedRateSchedule;
use crate::target::{Target, TargetOutcome, TargetRequest};

#[derive(Clone, Debug, Serialize)]
pub struct Config {
    pub operations: u64,
    pub offered_rate_per_second: u64,
    pub concurrency: usize,
    pub maximum_queued: usize,
    pub operation_timeout_ns: u64,
    pub drain_timeout_ns: u64,
    pub slo_ns: u64,
    pub highest_trackable_ns: u64,
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=10_000).contains(&self.operations)
            || !(1..=1_000_000).contains(&self.offered_rate_per_second)
            || ![1, 8, 32, 128].contains(&self.concurrency)
            || self.maximum_queued > 1024
            || !(1..=5_000_000_000).contains(&self.operation_timeout_ns)
            || !(1..=5_000_000_000).contains(&self.drain_timeout_ns)
            || self.slo_ns == 0
            || self.slo_ns > self.highest_trackable_ns
            || !(1..=30_000_000_000).contains(&self.highest_trackable_ns)
        {
            return Err("unsupported or out-of-budget scheduled control".to_owned());
        }
        let schedule = FixedRateSchedule::new(0, self.offered_rate_per_second)?;
        if schedule.scheduled_ns(self.operations - 1) > 15_000_000_000 {
            return Err("scheduled window exceeds local 15-second budget".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Success,
    Error,
    Timeout,
    QueueTimeout,
    TargetRejected,
    AdmissionRejected,
    Incomplete,
}

#[derive(Clone, Debug, Serialize)]
pub struct Sample {
    pub sequence: u64,
    pub scheduled_ns: u64,
    pub started_ns: Option<u64>,
    pub terminal_ns: u64,
    pub outcome: Outcome,
    // None for incomplete operations, never fabricate a response latency.
    pub scheduled_latency_ns: Option<u64>,
    pub service_latency_ns: Option<u64>,
    // Incomplete latency is a censored lower bound, not a histogram sample.
    pub incomplete_lower_bound_ns: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct Latency {
    pub unit: &'static str,
    pub samples: u64,
    pub p50_ns: Option<u64>,
    pub p95_ns: Option<u64>,
    pub p99_ns: Option<u64>,
    pub overflow_count: u64,
}

#[derive(Debug, Serialize)]
pub struct Observation {
    pub profile_id: &'static str,
    pub promotable: bool,
    pub product_performance_claim: bool,
    pub config: Config,
    pub offered: u64,
    pub target_started: u64,
    pub target_completed: u64,
    pub successes: u64,
    pub errors: u64,
    pub timeouts: u64,
    pub queue_timeouts: u64,
    pub target_rejections: u64,
    pub admission_rejections: u64,
    pub incomplete: u64,
    pub late_successes: u64,
    pub good_successes: u64,
    pub elapsed_ns: u64,
    pub goodput_operations_per_second: f64,
    pub good_fraction_of_all_offers: f64,
    pub pending_high_water: usize,
    pub execution_slots: usize,
    pub owned_tasks_drained: bool,
    pub scheduled_response_latency: Latency,
    pub service_response_latency: Latency,
    pub samples: Vec<Sample>,
}

fn offset(origin: Instant, at: Instant) -> u64 {
    u64::try_from(at.saturating_duration_since(origin).as_nanos()).unwrap_or(u64::MAX)
}
fn outcome(value: TargetOutcome) -> Outcome {
    match value {
        TargetOutcome::Success => Outcome::Success,
        TargetOutcome::Error => Outcome::Error,
        TargetOutcome::Timeout => Outcome::Timeout,
        TargetOutcome::Rejected => Outcome::TargetRejected,
    }
}

struct Completion {
    sample: Sample,
}

fn collect(
    joined: Result<Completion, tokio::task::JoinError>,
    samples: &mut [Option<Sample>],
) -> Result<(), String> {
    let sample = joined
        .map_err(|error| format!("owned request task failed: {error}"))?
        .sample;
    let sequence = sample.sequence as usize;
    if samples[sequence].is_some() {
        return Err("duplicate completion".to_owned());
    }
    samples[sequence] = Some(sample);
    Ok(())
}

async fn collect_or_drain(
    joined: Result<Completion, tokio::task::JoinError>,
    samples: &mut [Option<Sample>],
    tasks: &mut JoinSet<Completion>,
) -> Result<(), String> {
    if let Err(error) = collect(joined, samples) {
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
        return Err(error);
    }
    Ok(())
}

/// Original timestamps never move to actual send/slot-acquisition time. Offers
/// do not wait for responses; pending requests have a hard tool-only cap.
pub async fn run<T: Target>(target: Arc<T>, config: &Config) -> Result<Observation, String> {
    config.validate()?;
    let schedule = FixedRateSchedule::new(0, config.offered_rate_per_second)?;
    let origin = Instant::now();
    let permits = Arc::new(Semaphore::new(config.concurrency));
    let capacity = config.concurrency + config.maximum_queued;
    let mut tasks = JoinSet::new();
    let mut samples = vec![None; config.operations as usize];
    // Start timestamps are tiny bounded per-offer slots, including queued work
    // cancelled during drain. No completion channel or unbounded request queue.
    let starts = Arc::new(std::sync::Mutex::new(vec![
        None;
        config.operations as usize
    ]));
    let mut high_water = 0;
    for sequence in 0..config.operations {
        let scheduled_ns = schedule.scheduled_ns(sequence);
        let scheduled = origin + Duration::from_nanos(scheduled_ns);
        tokio::time::sleep_until(scheduled).await;
        while let Some(joined) = tasks.try_join_next() {
            collect_or_drain(joined, &mut samples, &mut tasks).await?;
        }
        if tasks.len() == capacity {
            let terminal_ns = offset(origin, Instant::now());
            samples[sequence as usize] = Some(Sample {
                sequence,
                scheduled_ns,
                started_ns: None,
                terminal_ns,
                outcome: Outcome::AdmissionRejected,
                scheduled_latency_ns: None,
                service_latency_ns: None,
                incomplete_lower_bound_ns: None,
            });
            continue;
        }
        let target = Arc::clone(&target);
        let permits = Arc::clone(&permits);
        let starts = Arc::clone(&starts);
        let timeout_ns = config.operation_timeout_ns;
        tasks.spawn(async move {
            let deadline = scheduled + Duration::from_nanos(timeout_ns);
            let permit = tokio::time::timeout_at(deadline, permits.acquire_owned()).await;
            let started = Instant::now();
            if permit.is_err() || started >= deadline {
                return Completion {
                    sample: Sample {
                        sequence,
                        scheduled_ns,
                        started_ns: None,
                        terminal_ns: offset(origin, started),
                        outcome: Outcome::QueueTimeout,
                        scheduled_latency_ns: None,
                        service_latency_ns: None,
                        incomplete_lower_bound_ns: None,
                    },
                };
            }
            let _permit = permit
                .expect("timeout handled")
                .expect("tool semaphore stays open");
            let started_ns = offset(origin, started);
            starts.lock().expect("tool starts lock")[sequence as usize] = Some(started_ns);
            // The deadline includes generator delay and queue time. Work whose
            // scheduled deadline has passed is not executed with a fresh budget.
            let result =
                match tokio::time::timeout_at(deadline, target.execute(TargetRequest { sequence }))
                    .await
                {
                    Ok(value) => outcome(value),
                    Err(_) => Outcome::Timeout,
                };
            let terminal_ns = offset(origin, Instant::now());
            Completion {
                sample: Sample {
                    sequence,
                    scheduled_ns,
                    started_ns: Some(started_ns),
                    terminal_ns,
                    outcome: result,
                    scheduled_latency_ns: Some(terminal_ns.saturating_sub(scheduled_ns)),
                    service_latency_ns: Some(terminal_ns.saturating_sub(started_ns)),
                    incomplete_lower_bound_ns: None,
                },
            }
        });
        high_water = high_water.max(tasks.len());
    }
    let drain_deadline = Instant::now() + Duration::from_nanos(config.drain_timeout_ns);
    while !tasks.is_empty() {
        match tokio::time::timeout_at(drain_deadline, tasks.join_next()).await {
            Ok(Some(joined)) => collect_or_drain(joined, &mut samples, &mut tasks).await?,
            Ok(None) | Err(_) => break,
        }
    }
    // Account already completed owners before aborting; only genuine unfinished
    // requests become censored incomplete samples. Abort and join before return.
    while let Some(joined) = tasks.try_join_next() {
        collect_or_drain(joined, &mut samples, &mut tasks).await?;
    }
    tasks.abort_all();
    while let Some(joined) = tasks.join_next().await {
        match joined {
            Ok(completion) => collect_or_drain(Ok(completion), &mut samples, &mut tasks).await?,
            Err(error) if error.is_cancelled() => {}
            Err(error) => collect_or_drain(Err(error), &mut samples, &mut tasks).await?,
        }
    }
    let terminal_ns = offset(origin, Instant::now());
    let starts = starts.lock().map_err(|_| "starts lock poisoned")?;
    let samples: Vec<Sample> = samples
        .into_iter()
        .enumerate()
        .map(|(sequence, sample)| {
            sample.unwrap_or_else(|| {
                let scheduled_ns = schedule.scheduled_ns(sequence as u64);
                Sample {
                    sequence: sequence as u64,
                    scheduled_ns,
                    started_ns: starts[sequence],
                    terminal_ns,
                    outcome: Outcome::Incomplete,
                    scheduled_latency_ns: None,
                    service_latency_ns: None,
                    incomplete_lower_bound_ns: Some(terminal_ns.saturating_sub(scheduled_ns)),
                }
            })
        })
        .collect();
    drop(starts);
    project(
        config,
        samples,
        terminal_ns,
        high_water,
        permits.available_permits() == config.concurrency,
    )
}

fn latency(samples: &[Sample], config: &Config, service: bool) -> Result<Latency, String> {
    let mut histogram = Histogram::<u64>::new_with_bounds(1, config.highest_trackable_ns, 3)
        .map_err(|error| error.to_string())?;
    let mut overflow = 0;
    for value in samples.iter().filter_map(|sample| {
        if service {
            sample.service_latency_ns
        } else {
            sample.scheduled_latency_ns
        }
    }) {
        if value > config.highest_trackable_ns {
            overflow += 1;
        }
        histogram
            .record(value.clamp(1, config.highest_trackable_ns))
            .map_err(|error| error.to_string())?;
    }
    let quantile = |q| (!histogram.is_empty()).then(|| histogram.value_at_quantile(q));
    Ok(Latency {
        unit: "nanosecond-histogram-not-clock-resolution",
        samples: histogram.len(),
        p50_ns: quantile(0.5),
        p95_ns: quantile(0.95),
        p99_ns: quantile(0.99),
        overflow_count: overflow,
    })
}

pub fn project(
    config: &Config,
    samples: Vec<Sample>,
    elapsed_ns: u64,
    high_water: usize,
    drained: bool,
) -> Result<Observation, String> {
    config.validate()?;
    if samples.len() != config.operations as usize
        || elapsed_ns == 0
        || high_water > config.concurrency + config.maximum_queued
        || !drained
    {
        return Err("incomplete/out-of-budget accounting or owned task cleanup".to_owned());
    }
    let schedule = FixedRateSchedule::new(0, config.offered_rate_per_second)?;
    for (sequence, sample) in samples.iter().enumerate() {
        if sample.sequence != sequence as u64
            || sample.scheduled_ns != schedule.scheduled_ns(sequence as u64)
            || sample.terminal_ns < sample.scheduled_ns
            || sample.terminal_ns > elapsed_ns
        {
            return Err("schedule or terminal timestamp drift".to_owned());
        }
        match sample.outcome {
            Outcome::AdmissionRejected | Outcome::QueueTimeout => {
                if sample.outcome == Outcome::QueueTimeout
                    && sample.terminal_ns - sample.scheduled_ns < config.operation_timeout_ns
                {
                    return Err("queue timeout before original deadline".to_owned());
                }
                if sample.started_ns.is_some()
                    || sample.scheduled_latency_ns.is_some()
                    || sample.service_latency_ns.is_some()
                    || sample.incomplete_lower_bound_ns.is_some()
                {
                    return Err("fabricated admission response".to_owned());
                }
            }
            Outcome::Incomplete => {
                if sample
                    .started_ns
                    .is_some_and(|start| start < sample.scheduled_ns || start > sample.terminal_ns)
                    || sample.scheduled_latency_ns.is_some()
                    || sample.service_latency_ns.is_some()
                    || sample.incomplete_lower_bound_ns
                        != Some(sample.terminal_ns - sample.scheduled_ns)
                {
                    return Err("incomplete response fabricated or lower bound missing".to_owned());
                }
            }
            _ => {
                let start = sample.started_ns.ok_or("response without start")?;
                if start < sample.scheduled_ns
                    || start > sample.terminal_ns
                    || sample.scheduled_latency_ns != Some(sample.terminal_ns - sample.scheduled_ns)
                    || sample.service_latency_ns != Some(sample.terminal_ns - start)
                    || sample.incomplete_lower_bound_ns.is_some()
                {
                    return Err("latency boundary conflation".to_owned());
                }
            }
        }
    }
    let count = |outcome| {
        samples
            .iter()
            .filter(|sample| sample.outcome == outcome)
            .count() as u64
    };
    let successes = count(Outcome::Success);
    let late = samples
        .iter()
        .filter(|sample| {
            sample.outcome == Outcome::Success
                && sample
                    .scheduled_latency_ns
                    .is_some_and(|value| value > config.slo_ns)
        })
        .count() as u64;
    let good = successes - late;
    let started = samples
        .iter()
        .filter(|sample| sample.started_ns.is_some())
        .count() as u64;
    let completed = successes
        + count(Outcome::Error)
        + count(Outcome::Timeout)
        + count(Outcome::TargetRejected);
    Ok(Observation {
        profile_id: "get-owner-scheduled-controls-074-v1",
        promotable: false,
        product_performance_claim: false,
        config: config.clone(),
        offered: config.operations,
        target_started: started,
        target_completed: completed,
        successes,
        errors: count(Outcome::Error),
        timeouts: count(Outcome::Timeout),
        queue_timeouts: count(Outcome::QueueTimeout),
        target_rejections: count(Outcome::TargetRejected),
        admission_rejections: count(Outcome::AdmissionRejected),
        incomplete: count(Outcome::Incomplete),
        late_successes: late,
        good_successes: good,
        elapsed_ns,
        goodput_operations_per_second: good as f64 * 1e9 / elapsed_ns as f64,
        good_fraction_of_all_offers: good as f64 / config.operations as f64,
        pending_high_water: high_water,
        execution_slots: config.concurrency,
        owned_tasks_drained: drained,
        scheduled_response_latency: latency(&samples, config, false)?,
        service_response_latency: latency(&samples, config, true)?,
        samples,
    })
}
