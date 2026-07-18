use serde::{Deserialize, Serialize};

use crate::histogram::LatencySummary;

/// One repeat-aggregated candidate rate evaluated by the sustainability predicate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateSample {
    pub offered_rate_per_second: f64,
    pub achieved_rate_per_second: f64,
    pub started: u64,
    pub completed: u64,
    pub errors: u64,
    pub timeouts: u64,
    pub rejections: u64,
    pub backlog_drained: bool,
    pub drain_ms: u64,
    pub robust_spread_ratio: f64,
    pub latency: LatencySummary,
}

/// Complete throughput-at-SLO predicate. Latency alone can never pass a rate.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Highest sustainable offered rate plus every evaluated verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KneeResult {
    pub sustainable_rate_per_second: Option<f64>,
    pub evaluated: Vec<(f64, SustainabilityVerdict)>,
}

impl SustainabilityCriteria {
    /// Evaluate every part of the sustainable capacity definition.
    pub fn evaluate(&self, sample: &RateSample) -> SustainabilityVerdict {
        let mut reasons = Vec::new();
        if !sample.offered_rate_per_second.is_finite() || sample.offered_rate_per_second <= 0.0 {
            reasons.push("offered rate is not positive and finite".to_owned());
        }
        let p99 = sample.latency.p99_us;
        if p99.is_none_or(|value| value > self.p99_slo_us) {
            reasons.push("p99 exceeds the declared SLO or is unavailable".to_owned());
        }
        if let Some(p999_slo_us) = self.p999_slo_us {
            if sample.latency.p999_reportable
                && sample
                    .latency
                    .p999_us
                    .is_none_or(|value| value > p999_slo_us)
            {
                reasons.push("reportable p999 exceeds the declared SLO".to_owned());
            }
        }
        let achieved_ratio =
            sample.achieved_rate_per_second / sample.offered_rate_per_second.max(f64::EPSILON);
        if !achieved_ratio.is_finite() || achieved_ratio < self.min_achieved_ratio {
            reasons.push("achieved/offered throughput ratio is below threshold".to_owned());
        }
        let denominator = sample.started.max(1) as f64;
        for (name, count, maximum) in [
            ("error", sample.errors, self.max_error_ratio),
            ("timeout", sample.timeouts, self.max_timeout_ratio),
            ("rejection", sample.rejections, self.max_rejection_ratio),
        ] {
            if count as f64 / denominator > maximum {
                reasons.push(format!("{name} ratio exceeds budget"));
            }
        }
        if sample.completed > sample.started {
            reasons.push("completed count exceeds started count".to_owned());
        }
        if !sample.backlog_drained || sample.drain_ms > self.max_drain_ms {
            reasons.push("backlog did not drain within the declared bound".to_owned());
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

    /// Select the highest passing offered rate without extrapolation.
    pub fn find_knee(&self, samples: &[RateSample]) -> KneeResult {
        let mut evaluated = samples
            .iter()
            .map(|sample| (sample.offered_rate_per_second, self.evaluate(sample)))
            .collect::<Vec<_>>();
        evaluated.sort_by(|left, right| left.0.total_cmp(&right.0));
        let sustainable_rate_per_second = evaluated
            .iter()
            .filter(|(_, verdict)| verdict.sustainable)
            .map(|(rate, _)| *rate)
            .max_by(f64::total_cmp);
        KneeResult {
            sustainable_rate_per_second,
            evaluated,
        }
    }
}
