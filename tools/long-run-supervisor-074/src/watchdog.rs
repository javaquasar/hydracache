use crate::{CheckpointPayload, Phase};
use thiserror::Error;

pub const PROCESS_CPU_TIME_NS: &str = "process_cpu_time_ns";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressHealth {
    Healthy,
    Warning,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressObservation {
    pub useful_progress: bool,
    pub last_useful_progress_unix_seconds: u64,
    pub health: ProgressHealth,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WatchdogError {
    #[error("watchdog sequence or deadlines are invalid")]
    InvalidConfiguration,
    #[error("checkpoint sequence must advance exactly once")]
    Sequence,
    #[error("checkpoint observation time reversed")]
    ObservationTimeReversal,
    #[error("campaign, role, harness, or daemon identity changed")]
    IdentityDrift,
    #[error("phase moved backward or skipped a required phase")]
    PhaseTransition,
    #[error("a cumulative checkpoint counter regressed or disappeared")]
    CounterRegression,
    #[error("post-work idle changed the operation count")]
    IdleOperationDrift,
    #[error("measured progress omitted the process CPU counter")]
    MissingMeasuredCpuCounter,
}

#[derive(Debug, Clone)]
pub struct ProgressWatchdog {
    sequence: u64,
    sample: CheckpointPayload,
    observed_unix_seconds: u64,
    last_useful_progress_unix_seconds: u64,
    warning_gap_seconds: u64,
    rejection_gap_seconds: u64,
}

impl ProgressWatchdog {
    pub fn new(
        sequence: u64,
        sample: CheckpointPayload,
        observed_unix_seconds: u64,
        warning_gap_seconds: u64,
        rejection_gap_seconds: u64,
    ) -> Result<Self, WatchdogError> {
        if sequence == 0 || warning_gap_seconds == 0 || rejection_gap_seconds <= warning_gap_seconds
        {
            return Err(WatchdogError::InvalidConfiguration);
        }
        Ok(Self {
            sequence,
            sample,
            observed_unix_seconds,
            last_useful_progress_unix_seconds: observed_unix_seconds,
            warning_gap_seconds,
            rejection_gap_seconds,
        })
    }

    pub fn observe(
        &mut self,
        sequence: u64,
        sample: CheckpointPayload,
        observed_unix_seconds: u64,
    ) -> Result<ProgressObservation, WatchdogError> {
        if sequence != self.sequence.saturating_add(1) {
            return Err(WatchdogError::Sequence);
        }
        if observed_unix_seconds < self.observed_unix_seconds {
            return Err(WatchdogError::ObservationTimeReversal);
        }
        validate_identity(&self.sample, &sample)?;
        validate_phase_transition(&self.sample.phase, &sample.phase)?;
        validate_cumulative_counters(&self.sample, &sample)?;
        let useful = useful_progress(&self.sample, &sample)?;

        self.sequence = sequence;
        self.sample = sample;
        self.observed_unix_seconds = observed_unix_seconds;
        if useful {
            self.last_useful_progress_unix_seconds = observed_unix_seconds;
        }
        Ok(ProgressObservation {
            useful_progress: useful,
            last_useful_progress_unix_seconds: self.last_useful_progress_unix_seconds,
            health: self.health_at(observed_unix_seconds),
        })
    }

    pub fn health_at(&self, now_unix_seconds: u64) -> ProgressHealth {
        let gap = now_unix_seconds.saturating_sub(self.last_useful_progress_unix_seconds);
        if gap > self.rejection_gap_seconds {
            ProgressHealth::Rejected
        } else if gap > self.warning_gap_seconds {
            ProgressHealth::Warning
        } else {
            ProgressHealth::Healthy
        }
    }

    pub fn last_useful_progress_unix_seconds(&self) -> u64 {
        self.last_useful_progress_unix_seconds
    }
}

fn validate_identity(
    previous: &CheckpointPayload,
    current: &CheckpointPayload,
) -> Result<(), WatchdogError> {
    if previous.campaign_id != current.campaign_id
        || previous.role != current.role
        || previous.harness != current.harness
        || previous.daemon != current.daemon
    {
        return Err(WatchdogError::IdentityDrift);
    }
    Ok(())
}

fn validate_phase_transition(previous: &Phase, current: &Phase) -> Result<(), WatchdogError> {
    let previous = phase_rank(previous);
    let current = phase_rank(current);
    if current == previous || current == previous + 1 || current == phase_rank(&Phase::Terminal) {
        Ok(())
    } else {
        Err(WatchdogError::PhaseTransition)
    }
}

fn validate_cumulative_counters(
    previous: &CheckpointPayload,
    current: &CheckpointPayload,
) -> Result<(), WatchdogError> {
    if current.phase_epoch < previous.phase_epoch
        || current.completed < previous.completed
        || current.failed < previous.failed
        || current.rejected < previous.rejected
        || current.timed_out < previous.timed_out
        || current.telemetry_sequence < previous.telemetry_sequence
        || map_regressed(&previous.surface_counters, &current.surface_counters)
        || cumulative_resource_regressed(previous, current)
    {
        return Err(WatchdogError::CounterRegression);
    }
    Ok(())
}

fn useful_progress(
    previous: &CheckpointPayload,
    current: &CheckpointPayload,
) -> Result<bool, WatchdogError> {
    if phase_rank(&current.phase) > phase_rank(&previous.phase) {
        return Ok(true);
    }
    match current.phase {
        Phase::Startup | Phase::Warmup => Ok(current.completed > previous.completed
            || current.phase_epoch > previous.phase_epoch
            || current.milestone != previous.milestone),
        Phase::Measured => {
            let previous_cpu = previous
                .resource_counters
                .get(PROCESS_CPU_TIME_NS)
                .ok_or(WatchdogError::MissingMeasuredCpuCounter)?;
            let current_cpu = current
                .resource_counters
                .get(PROCESS_CPU_TIME_NS)
                .ok_or(WatchdogError::MissingMeasuredCpuCounter)?;
            Ok(current.completed > previous.completed
                && sum(&current.surface_counters) > sum(&previous.surface_counters)
                && current_cpu > previous_cpu)
        }
        Phase::Drain => Ok(current.outstanding < previous.outstanding),
        Phase::DurableCompanion => {
            Ok(current.phase_epoch > previous.phase_epoch
                || current.milestone != previous.milestone)
        }
        Phase::PostWorkIdle => {
            if current.completed != previous.completed {
                return Err(WatchdogError::IdleOperationDrift);
            }
            Ok(current.telemetry_sequence > previous.telemetry_sequence)
        }
        Phase::Reconciliation => Ok(current.phase_epoch > previous.phase_epoch
            || current.milestone != previous.milestone
            || current.owner_counters != previous.owner_counters),
        Phase::Terminal => Ok(false),
    }
}

fn cumulative_resource_regressed(
    previous: &CheckpointPayload,
    current: &CheckpointPayload,
) -> bool {
    match (
        previous.resource_counters.get(PROCESS_CPU_TIME_NS),
        current.resource_counters.get(PROCESS_CPU_TIME_NS),
    ) {
        (Some(previous), Some(current)) => current < previous,
        (Some(_), None) => true,
        _ => false,
    }
}

fn map_regressed(
    previous: &std::collections::BTreeMap<String, u64>,
    current: &std::collections::BTreeMap<String, u64>,
) -> bool {
    previous
        .iter()
        .any(|(key, value)| current.get(key).is_none_or(|current| current < value))
}

fn sum(counters: &std::collections::BTreeMap<String, u64>) -> u128 {
    counters.values().map(|value| u128::from(*value)).sum()
}

fn phase_rank(phase: &Phase) -> u8 {
    match phase {
        Phase::Startup => 0,
        Phase::Warmup => 1,
        Phase::Measured => 2,
        Phase::Drain => 3,
        Phase::DurableCompanion => 4,
        Phase::PostWorkIdle => 5,
        Phase::Reconciliation => 6,
        Phase::Terminal => 7,
    }
}
