use serde::{Deserialize, Serialize};

use crate::histogram::LatencySummary;
use crate::rate::OpenLoopObservation;

/// Per-repeat lifecycle accounting kept beside the steady-state observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhaseAccounting {
    pub reset_operations: u64,
    pub preload_operations: u64,
    pub warmup_operations: u64,
    pub steady_operations: u64,
    pub reset_ms: u64,
    pub preload_ms: u64,
    pub warmup_ms: u64,
    pub steady_ms: u64,
    pub warmup_samples_in_steady_histogram: u64,
}

/// Raw, auditable evidence for one reset/preload/warm-up/steady repeat.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepeatEvidence {
    pub reset_state_digest: String,
    pub state_digest: String,
    pub phase: PhaseAccounting,
    pub steady: OpenLoopObservation,
}

/// Median/min/max aggregation of at least three raw repeats at one offered rate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateSample {
    pub offered_rate_per_second: f64,
    pub achieved_rate_per_second: f64,
    pub achieved_rate_min_per_second: f64,
    pub achieved_rate_max_per_second: f64,
    pub offered: u64,
    pub started: u64,
    pub completed: u64,
    pub successes: u64,
    pub errors: u64,
    pub timeouts: u64,
    pub rejections: u64,
    pub backlog_drained: bool,
    pub drain_ms: u64,
    pub robust_spread_ratio: f64,
    pub latency: LatencySummary,
}

/// Complete throughput-at-SLO predicate. Latency alone can never pass a rate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SustainabilityCriteria {
    pub p99_slo_us: u64,
    pub p999_slo_us: Option<u64>,
    pub min_achieved_ratio: f64,
    pub max_error_ratio: f64,
    pub max_timeout_ratio: f64,
    pub max_rejection_ratio: f64,
    pub max_drain_ms: u64,
    pub max_robust_spread_ratio: f64,
}

/// Explainable verdict for one offered rate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SustainabilityVerdict {
    pub sustainable: bool,
    pub reasons: Vec<String>,
}

/// Raw repeats, their reproducible aggregate, and the resulting predicate verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RatePointEvidence {
    pub sample: RateSample,
    pub repeats: Vec<RepeatEvidence>,
    pub verdict: SustainabilityVerdict,
}

/// Highest sustainable offered rate plus every auditable evaluated rate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KneeResult {
    pub sustainable_rate_per_second: Option<f64>,
    pub evaluated: Vec<RatePointEvidence>,
}

impl RateSample {
    /// Aggregate raw repeats without discarding their min/max or spread.
    pub fn from_repeats(
        offered_rate_per_second: f64,
        repeats: &[RepeatEvidence],
    ) -> Result<Self, String> {
        if !offered_rate_per_second.is_finite() || offered_rate_per_second <= 0.0 {
            return Err("offered rate must be positive and finite".to_owned());
        }
        if repeats.len() < 3 {
            return Err("capacity evidence requires at least three repeats".to_owned());
        }
        let achieved = repeats
            .iter()
            .map(|repeat| repeat.steady.achieved_rate_per_second)
            .collect::<Vec<_>>();
        if achieved.iter().any(|rate| !rate.is_finite() || *rate < 0.0) {
            return Err("repeat achieved rates must be non-negative and finite".to_owned());
        }
        let achieved_median = median_f64(&achieved);
        let achieved_min = achieved.iter().copied().min_by(f64::total_cmp).unwrap();
        let achieved_max = achieved.iter().copied().max_by(f64::total_cmp).unwrap();
        let robust_spread_ratio = if achieved_median > 0.0 {
            (achieved_max - achieved_min) / achieved_median
        } else if achieved_max == achieved_min {
            0.0
        } else {
            f64::INFINITY
        };
        let observations = repeats
            .iter()
            .map(|repeat| &repeat.steady)
            .collect::<Vec<_>>();
        let summaries = observations
            .iter()
            .map(|observation| &observation.latency)
            .collect::<Vec<_>>();
        Ok(Self {
            offered_rate_per_second,
            achieved_rate_per_second: achieved_median,
            achieved_rate_min_per_second: achieved_min,
            achieved_rate_max_per_second: achieved_max,
            offered: median_u64(observations.iter().map(|value| value.offered)),
            started: median_u64(observations.iter().map(|value| value.started)),
            completed: median_u64(observations.iter().map(|value| value.completed)),
            successes: median_u64(observations.iter().map(|value| value.successes)),
            errors: median_u64(observations.iter().map(|value| value.errors)),
            timeouts: median_u64(observations.iter().map(|value| value.timeouts)),
            rejections: median_u64(observations.iter().map(|value| value.rejections)),
            backlog_drained: observations.iter().all(|value| value.backlog_drained),
            drain_ms: observations
                .iter()
                .map(|value| value.drain_ms)
                .max()
                .unwrap(),
            robust_spread_ratio,
            latency: aggregate_latency(&summaries),
        })
    }
}

impl SustainabilityCriteria {
    /// Reject criteria that could make a performance verdict vacuous or non-finite.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut reasons = Vec::new();
        if self.p99_slo_us == 0 || self.p999_slo_us == Some(0) {
            reasons.push("latency SLOs must be positive".to_owned());
        }
        if !valid_ratio(self.min_achieved_ratio) || self.min_achieved_ratio == 0.0 {
            reasons.push("minimum achieved ratio must be finite and in (0, 1]".to_owned());
        }
        for (name, ratio) in [
            ("error", self.max_error_ratio),
            ("timeout", self.max_timeout_ratio),
            ("rejection", self.max_rejection_ratio),
        ] {
            if !valid_ratio(ratio) {
                reasons.push(format!("maximum {name} ratio must be finite and in [0, 1]"));
            }
        }
        if self.max_drain_ms == 0 {
            reasons.push("maximum drain time must be positive".to_owned());
        }
        if !self.max_robust_spread_ratio.is_finite() || self.max_robust_spread_ratio < 0.0 {
            reasons.push("maximum robust spread must be non-negative and finite".to_owned());
        }
        if reasons.is_empty() {
            Ok(())
        } else {
            Err(reasons)
        }
    }

    /// Build a rate point from raw repeats and evaluate the complete predicate.
    pub fn evaluate_repeats(
        &self,
        offered_rate_per_second: f64,
        repeats: Vec<RepeatEvidence>,
    ) -> RatePointEvidence {
        let normalized_rate =
            if offered_rate_per_second.is_finite() && offered_rate_per_second > 0.0 {
                offered_rate_per_second
            } else {
                0.0
            };
        let sample =
            RateSample::from_repeats(offered_rate_per_second, &repeats).unwrap_or_else(|_| {
                RateSample {
                    offered_rate_per_second: normalized_rate,
                    achieved_rate_per_second: 0.0,
                    achieved_rate_min_per_second: 0.0,
                    achieved_rate_max_per_second: 0.0,
                    offered: 0,
                    started: 0,
                    completed: 0,
                    successes: 0,
                    errors: 0,
                    timeouts: 0,
                    rejections: 0,
                    backlog_drained: false,
                    drain_ms: u64::MAX,
                    robust_spread_ratio: f64::MAX,
                    latency: empty_latency(),
                }
            });
        let verdict = self.evaluate_point(&sample, &repeats);
        RatePointEvidence {
            sample,
            repeats,
            verdict,
        }
    }

    /// Select the highest passing offered rate without extrapolation.
    pub fn find_knee(&self, mut evaluated: Vec<RatePointEvidence>) -> KneeResult {
        for point in &mut evaluated {
            point.verdict = self.evaluate_point(&point.sample, &point.repeats);
        }
        evaluated.sort_by(|left, right| {
            left.sample
                .offered_rate_per_second
                .total_cmp(&right.sample.offered_rate_per_second)
        });
        let sustainable_rate_per_second = evaluated
            .iter()
            .filter(|point| point.verdict.sustainable)
            .map(|point| point.sample.offered_rate_per_second)
            .max_by(f64::total_cmp);
        KneeResult {
            sustainable_rate_per_second,
            evaluated,
        }
    }

    /// Recompute all aggregates and verdicts stored in a serialized knee.
    pub fn knee_validation_problems(&self, knee: &KneeResult) -> Vec<String> {
        let mut problems = self.validate().err().unwrap_or_default();
        if knee.evaluated.is_empty() {
            problems.push("knee has no evaluated rates".to_owned());
            return problems;
        }
        let mut derived = None;
        let mut prior_rate = None;
        for point in &knee.evaluated {
            if prior_rate.is_some_and(|rate| rate >= point.sample.offered_rate_per_second) {
                problems
                    .push("evaluated knee rates must be unique and strictly increasing".to_owned());
            }
            prior_rate = Some(point.sample.offered_rate_per_second);
            let expected_sample =
                RateSample::from_repeats(point.sample.offered_rate_per_second, &point.repeats);
            match expected_sample {
                Ok(expected) if expected == point.sample => {}
                Ok(_) => problems.push(format!(
                    "rate {} aggregate does not match raw repeats",
                    point.sample.offered_rate_per_second
                )),
                Err(error) => problems.push(format!(
                    "rate {} cannot be aggregated: {error}",
                    point.sample.offered_rate_per_second
                )),
            }
            let expected_verdict = self.evaluate_point(&point.sample, &point.repeats);
            if expected_verdict != point.verdict {
                problems.push(format!(
                    "rate {} verdict does not match the declared criteria",
                    point.sample.offered_rate_per_second
                ));
            }
            if expected_verdict.sustainable {
                derived = Some(
                    derived.map_or(point.sample.offered_rate_per_second, |current: f64| {
                        current.max(point.sample.offered_rate_per_second)
                    }),
                );
            }
        }
        if knee.sustainable_rate_per_second != derived {
            problems.push("declared knee does not match evaluated rates".to_owned());
        }
        problems
    }

    fn evaluate_point(
        &self,
        sample: &RateSample,
        repeats: &[RepeatEvidence],
    ) -> SustainabilityVerdict {
        let mut reasons = self.validate().err().unwrap_or_default();
        match RateSample::from_repeats(sample.offered_rate_per_second, repeats) {
            Ok(expected) if expected == *sample => {}
            Ok(_) => reasons.push("aggregate does not match raw repeats".to_owned()),
            Err(error) => reasons.push(error),
        }
        for (index, repeat) in repeats.iter().enumerate() {
            for problem in
                observation_problems(&repeat.steady, sample.offered_rate_per_second, self)
            {
                reasons.push(format!("repeat {}: {problem}", index + 1));
            }
            if repeat.reset_state_digest.is_empty() || repeat.state_digest.is_empty() {
                reasons.push(format!(
                    "repeat {}: state digests are incomplete",
                    index + 1
                ));
            }
            if repeat.phase.steady_operations != repeat.steady.offered
                || repeat.phase.reset_operations != 1
                || repeat.phase.warmup_samples_in_steady_histogram != 0
            {
                reasons.push(format!(
                    "repeat {}: phase accounting is inconsistent",
                    index + 1
                ));
            }
        }
        if !sample.robust_spread_ratio.is_finite()
            || sample.robust_spread_ratio > self.max_robust_spread_ratio
        {
            reasons.push("repeat spread exceeds tolerance".to_owned());
        }
        SustainabilityVerdict {
            sustainable: reasons.is_empty(),
            reasons,
        }
    }
}

fn observation_problems(
    observation: &OpenLoopObservation,
    offered_rate_per_second: f64,
    criteria: &SustainabilityCriteria,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if observation.offered == 0 || observation.started != observation.offered {
        reasons.push("started count does not match offered count".to_owned());
    }
    if observation.completed != observation.started {
        reasons.push("completed count does not match started count".to_owned());
    }
    let outcomes = observation
        .successes
        .checked_add(observation.errors)
        .and_then(|total| total.checked_add(observation.timeouts))
        .and_then(|total| total.checked_add(observation.rejections));
    if outcomes != Some(observation.completed) {
        reasons.push("outcome counts do not match completed count".to_owned());
    }
    if observation.latency.samples != observation.completed {
        reasons.push("latency sample count does not match completed count".to_owned());
    }
    reasons.extend(latency_problems(&observation.latency));
    if !observation.offered_rate_per_second.is_finite()
        || observation.offered_rate_per_second != offered_rate_per_second
    {
        reasons.push("repeat offered rate does not match rate point".to_owned());
    }
    let achieved_ratio =
        observation.achieved_rate_per_second / offered_rate_per_second.max(f64::EPSILON);
    if !achieved_ratio.is_finite() || achieved_ratio < criteria.min_achieved_ratio {
        reasons.push("achieved/offered throughput ratio is below threshold".to_owned());
    }
    if observation
        .latency
        .p99_us
        .is_none_or(|value| value > criteria.p99_slo_us)
    {
        reasons.push("p99 exceeds the declared SLO or is unavailable".to_owned());
    }
    if let Some(p999_slo_us) = criteria.p999_slo_us {
        if !observation.latency.p999_reportable {
            reasons.push("p999 is required but the declared sample floor was not met".to_owned());
        } else if observation
            .latency
            .p999_us
            .is_none_or(|value| value > p999_slo_us)
        {
            reasons.push("reportable p999 exceeds the declared SLO".to_owned());
        }
    }
    let denominator = observation.started.max(1) as f64;
    for (name, count, maximum) in [
        ("error", observation.errors, criteria.max_error_ratio),
        ("timeout", observation.timeouts, criteria.max_timeout_ratio),
        (
            "rejection",
            observation.rejections,
            criteria.max_rejection_ratio,
        ),
    ] {
        if count as f64 / denominator > maximum {
            reasons.push(format!("{name} ratio exceeds budget"));
        }
    }
    if !observation.backlog_drained || observation.drain_ms > criteria.max_drain_ms {
        reasons.push("backlog did not drain within the declared bound".to_owned());
    }
    reasons
}

fn latency_problems(summary: &LatencySummary) -> Vec<String> {
    let mut reasons = Vec::new();
    if summary.samples == 0 || summary.p999_min_samples == 0 {
        reasons.push("latency summary has no samples or p999 floor".to_owned());
    }
    if summary.overflow_count > 0 {
        reasons.push("latency histogram overflowed its declared range".to_owned());
    }
    let should_report_p999 = summary.samples >= summary.p999_min_samples;
    if summary.p999_reportable != should_report_p999
        || summary.p999_us.is_some() != should_report_p999
    {
        reasons.push("p999 reportability marker is inconsistent".to_owned());
    }
    let ordered = [summary.p50_us, summary.p90_us, summary.p99_us]
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .is_some_and(|values| values.windows(2).all(|pair| pair[0] <= pair[1]));
    if !ordered {
        reasons.push("latency percentiles are missing or unordered".to_owned());
    }
    if let (Some(p99), Some(max)) = (summary.p99_us, summary.max_us) {
        if p99 > max || summary.p999_us.is_some_and(|p999| p999 > max || p999 < p99) {
            reasons.push("tail latency is inconsistent with maximum latency".to_owned());
        }
    } else {
        reasons.push("maximum latency is unavailable".to_owned());
    }
    reasons
}

fn aggregate_latency(summaries: &[&LatencySummary]) -> LatencySummary {
    let report_p999 = summaries.iter().all(|summary| summary.p999_reportable);
    LatencySummary {
        samples: median_u64(summaries.iter().map(|summary| summary.samples)),
        p50_us: median_optional(summaries.iter().map(|summary| summary.p50_us)),
        p90_us: median_optional(summaries.iter().map(|summary| summary.p90_us)),
        p99_us: median_optional(summaries.iter().map(|summary| summary.p99_us)),
        p999_us: report_p999
            .then(|| median_optional(summaries.iter().map(|summary| summary.p999_us)))
            .flatten(),
        p999_min_samples: summaries
            .iter()
            .map(|summary| summary.p999_min_samples)
            .max()
            .unwrap_or(0),
        p999_reportable: report_p999,
        max_us: summaries.iter().filter_map(|summary| summary.max_us).max(),
        overflow_count: summaries
            .iter()
            .map(|summary| summary.overflow_count)
            .max()
            .unwrap_or(0),
    }
}

fn median_optional(values: impl Iterator<Item = Option<u64>>) -> Option<u64> {
    values.collect::<Option<Vec<_>>>().map(median_u64)
}

fn median_u64(values: impl IntoIterator<Item = u64>) -> u64 {
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort_unstable();
    values[values.len() / 2]
}

fn median_f64(values: &[f64]) -> f64 {
    let mut values = values.to_vec();
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn valid_ratio(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn empty_latency() -> LatencySummary {
    LatencySummary {
        samples: 0,
        p50_us: None,
        p90_us: None,
        p99_us: None,
        p999_us: None,
        p999_min_samples: 0,
        p999_reportable: false,
        max_us: None,
        overflow_count: 0,
    }
}
