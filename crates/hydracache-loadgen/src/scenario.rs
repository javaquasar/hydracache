use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::knee::SustainabilityCriteria;

/// Per-window outcome budgets in a committed scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorBudgets {
    pub max_error_ratio: f64,
    pub max_timeout_ratio: f64,
    pub max_rejection_ratio: f64,
}

/// Versioned scenario input shared by every measurement target.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub schema_version: u32,
    pub id: String,
    pub seed: u64,
    pub offered_rates_per_second: Vec<u64>,
    pub preload_operations: u64,
    pub warmup_operations: u64,
    pub steady_operations: u64,
    pub repeats: u32,
    pub p99_slo_us: u64,
    pub p999_slo_us: Option<u64>,
    pub p999_min_samples: u64,
    pub highest_trackable_latency_us: u64,
    pub histogram_significant_figures: u8,
    pub min_achieved_ratio: f64,
    pub error_budgets: ErrorBudgets,
    pub backlog_drain_ms: u64,
    pub robust_spread_tolerance: f64,
}

impl Scenario {
    /// Parse and validate a committed TOML scenario.
    pub fn from_toml(text: &str) -> Result<Self, String> {
        let scenario: Self = toml::from_str(text).map_err(|error| error.to_string())?;
        scenario.validate()?;
        Ok(scenario)
    }

    /// Fail closed on inputs that could make a capacity verdict vacuous.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("scenario schema_version must be 1".to_owned());
        }
        let rates_are_strictly_increasing = self
            .offered_rates_per_second
            .windows(2)
            .all(|pair| pair[0] < pair[1]);
        if self.id.trim().is_empty()
            || self.offered_rates_per_second.is_empty()
            || self.offered_rates_per_second.contains(&0)
            || !rates_are_strictly_increasing
            || self.steady_operations == 0
            || self.repeats < 3
            || self.p99_slo_us == 0
            || self.p999_slo_us == Some(0)
            || self.p999_min_samples == 0
            || self.highest_trackable_latency_us == 0
            || !(1..=5).contains(&self.histogram_significant_figures)
            || !(0.0..=1.0).contains(&self.min_achieved_ratio)
            || self.min_achieved_ratio == 0.0
            || self.backlog_drain_ms == 0
            || !self.robust_spread_tolerance.is_finite()
            || self.robust_spread_tolerance < 0.0
        {
            return Err("scenario has an incomplete measurement contract".to_owned());
        }
        for ratio in [
            self.error_budgets.max_error_ratio,
            self.error_budgets.max_timeout_ratio,
            self.error_budgets.max_rejection_ratio,
        ] {
            if !ratio.is_finite() || !(0.0..=1.0).contains(&ratio) {
                return Err("scenario outcome ratio is outside 0..=1".to_owned());
            }
        }
        Ok(())
    }

    /// Stable digest of the exact scenario bytes supplied to the runner.
    pub fn digest_bytes(bytes: &[u8]) -> String {
        hex_sha256(bytes)
    }

    /// Convert scenario budgets into the common knee predicate.
    pub fn sustainability_criteria(&self) -> SustainabilityCriteria {
        SustainabilityCriteria {
            p99_slo_us: self.p99_slo_us,
            p999_slo_us: self.p999_slo_us,
            min_achieved_ratio: self.min_achieved_ratio,
            max_error_ratio: self.error_budgets.max_error_ratio,
            max_timeout_ratio: self.error_budgets.max_timeout_ratio,
            max_rejection_ratio: self.error_budgets.max_rejection_ratio,
            max_drain_ms: self.backlog_drain_ms,
            max_robust_spread_ratio: self.robust_spread_tolerance,
        }
    }
}

pub(crate) fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
