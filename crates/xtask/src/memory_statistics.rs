use std::cmp::Ordering;

pub const ESTIMATOR_VERSION: &str = "memory-statistics-071-v1";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeSample {
    pub sequence: u64,
    pub monotonic_seconds: f64,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConfidenceInterval {
    pub estimate: f64,
    pub lower: f64,
    pub upper: f64,
    pub samples: usize,
    pub seed: u64,
    pub iterations: usize,
    pub block_samples: usize,
}

pub fn validated_series(samples: &[TimeSample]) -> Result<Vec<TimeSample>, String> {
    if samples.len() < 3 {
        return Err("at least three samples are required".to_owned());
    }
    let mut ordered = samples.to_vec();
    ordered.sort_by_key(|sample| sample.sequence);
    for (index, sample) in ordered.iter().enumerate() {
        if !sample.monotonic_seconds.is_finite() || !sample.value.is_finite() {
            return Err(format!("sample {index} contains a non-finite value"));
        }
        if index > 0 {
            let previous = ordered[index - 1];
            if sample.sequence == previous.sequence {
                return Err(format!("duplicate sample sequence {}", sample.sequence));
            }
            if sample.monotonic_seconds <= previous.monotonic_seconds {
                return Err(format!(
                    "monotonic clock regressed at sequence {}",
                    sample.sequence
                ));
            }
        }
    }
    Ok(ordered)
}

pub fn require_final_checkpoint(
    samples: &[TimeSample],
    expected_final_sequence: u64,
) -> Result<Vec<TimeSample>, String> {
    let ordered = validated_series(samples)?;
    if ordered.last().map(|sample| sample.sequence) != Some(expected_final_sequence) {
        return Err(format!(
            "missing final checkpoint sequence {expected_final_sequence}"
        ));
    }
    Ok(ordered)
}

pub fn validated_cumulative_counter(samples: &[TimeSample]) -> Result<Vec<TimeSample>, String> {
    let ordered = validated_series(samples)?;
    if ordered.windows(2).any(|pair| pair[1].value < pair[0].value) {
        return Err("cumulative counter reset or regressed".to_owned());
    }
    Ok(ordered)
}

pub fn theil_sen_slope(samples: &[TimeSample]) -> Result<f64, String> {
    let samples = validated_series(samples)?;
    let mut slopes = Vec::with_capacity(samples.len() * (samples.len() - 1) / 2);
    for (index, left) in samples.iter().enumerate() {
        for right in samples.iter().skip(index + 1) {
            slopes.push(
                (right.value - left.value) / (right.monotonic_seconds - left.monotonic_seconds),
            );
        }
    }
    median(&mut slopes)
}

pub fn hodges_lehmann_paired(baseline: &[f64], candidate: &[f64]) -> Result<f64, String> {
    if baseline.len() != candidate.len() || baseline.is_empty() {
        return Err("paired estimator requires equal non-empty cohorts".to_owned());
    }
    let differences: Vec<_> = baseline
        .iter()
        .zip(candidate)
        .map(|(baseline, candidate)| candidate - baseline)
        .collect();
    if differences.iter().any(|value| !value.is_finite()) {
        return Err("paired estimator received a non-finite value".to_owned());
    }
    let mut walsh = Vec::with_capacity(differences.len() * (differences.len() + 1) / 2);
    for (index, left) in differences.iter().enumerate() {
        for right in differences.iter().skip(index) {
            walsh.push((left + right) / 2.0);
        }
    }
    median(&mut walsh)
}

pub fn moving_block_slope_interval(
    samples: &[TimeSample],
    block_samples: usize,
    iterations: usize,
    seed: u64,
    confidence: f64,
) -> Result<ConfidenceInterval, String> {
    let ordered = validated_series(samples)?;
    if block_samples < 2 || block_samples > ordered.len() {
        return Err("block_samples must be between 2 and the sample count".to_owned());
    }
    if iterations < 100 {
        return Err("at least 100 bootstrap iterations are required".to_owned());
    }
    if !(0.5..1.0).contains(&confidence) {
        return Err("confidence must be in [0.5, 1.0)".to_owned());
    }
    let estimate = theil_sen_slope(&ordered)?;
    let deltas: Vec<_> = ordered
        .windows(2)
        .map(|pair| {
            (pair[1].value - pair[0].value)
                / (pair[1].monotonic_seconds - pair[0].monotonic_seconds)
        })
        .collect();
    let effective_block = block_samples.min(deltas.len());
    let starts = deltas.len() - effective_block + 1;
    let mut rng = SplitMix64(seed);
    let mut estimates = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let mut resampled = Vec::with_capacity(deltas.len());
        while resampled.len() < deltas.len() {
            let start = (rng.next() as usize) % starts;
            resampled.extend_from_slice(&deltas[start..start + effective_block]);
        }
        resampled.truncate(deltas.len());
        estimates.push(resampled.iter().sum::<f64>() / resampled.len() as f64);
    }
    estimates.sort_by(total_cmp);
    let alpha = (1.0 - confidence) / 2.0;
    let lower_index = ((iterations - 1) as f64 * alpha).floor() as usize;
    let upper_index = ((iterations - 1) as f64 * (1.0 - alpha)).ceil() as usize;
    Ok(ConfidenceInterval {
        estimate,
        lower: estimates[lower_index],
        upper: estimates[upper_index.min(iterations - 1)],
        samples: ordered.len(),
        seed,
        iterations,
        block_samples,
    })
}

pub fn practical_verdict(
    interval: ConfidenceInterval,
    baseline: f64,
    absolute_minimum: f64,
    relative_minimum: f64,
    lower_is_better: bool,
) -> Result<bool, String> {
    if !baseline.is_finite()
        || !absolute_minimum.is_finite()
        || !relative_minimum.is_finite()
        || absolute_minimum < 0.0
        || relative_minimum < 0.0
    {
        return Err("invalid practical-significance inputs".to_owned());
    }
    let practical = absolute_minimum.max(baseline.abs() * relative_minimum);
    if lower_is_better {
        Ok(interval.upper <= -practical)
    } else {
        Ok(interval.lower >= practical)
    }
}

fn median(values: &mut [f64]) -> Result<f64, String> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return Err("median requires finite values".to_owned());
    }
    values.sort_by(total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        Ok((values[middle - 1] + values[middle]) / 2.0)
    } else {
        Ok(values[middle])
    }
}

fn total_cmp(left: &f64, right: &f64) -> Ordering {
    left.total_cmp(right)
}

struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }
}
