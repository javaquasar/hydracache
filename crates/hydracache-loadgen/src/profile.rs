use serde::{Deserialize, Serialize};

/// Observed host/runner facts captured at measurement time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunnerFingerprint {
    pub runner_class: String,
    pub fingerprint: String,
    pub cpu_model: String,
    pub logical_cores: u32,
    pub ram_bytes: u64,
    pub os: String,
    pub kernel: String,
    pub cpu_affinity: String,
    pub cgroup_cpu_quota: String,
    pub governor: String,
    pub turbo: String,
    pub shared_hardware: bool,
    pub calibration_score: f64,
}

/// Committed requirements for a named performance runner profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerformanceProfile {
    pub name: String,
    pub required_runner_class: String,
    pub allowed_fingerprints: Vec<String>,
    pub minimum_logical_cores: u32,
    pub required_cpu_affinity: String,
    pub required_cgroup_cpu_quota: String,
    pub require_dedicated: bool,
    pub maximum_calibration_score: f64,
}

/// Explainable profile-match verdict stored with a report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileValidation {
    pub eligible: bool,
    pub reasons: Vec<String>,
}

impl PerformanceProfile {
    /// Validate observed facts; a caller-supplied profile name is never sufficient.
    pub fn validate(&self, observed: &RunnerFingerprint) -> ProfileValidation {
        let mut reasons = Vec::new();
        if observed.runner_class != self.required_runner_class {
            reasons.push("runner class does not match the committed profile".to_owned());
        }
        if !self.allowed_fingerprints.is_empty()
            && !self
                .allowed_fingerprints
                .iter()
                .any(|fingerprint| fingerprint == &observed.fingerprint)
        {
            reasons.push("observed runner fingerprint is not approved".to_owned());
        }
        if observed.logical_cores < self.minimum_logical_cores {
            reasons.push("observed core count is below the committed profile".to_owned());
        }
        if observed.cpu_affinity != self.required_cpu_affinity {
            reasons.push("CPU affinity does not match the committed profile".to_owned());
        }
        if observed.cgroup_cpu_quota != self.required_cgroup_cpu_quota {
            reasons.push("cgroup CPU quota does not match the committed profile".to_owned());
        }
        if self.require_dedicated && observed.shared_hardware {
            reasons.push("reference runner reports shared hardware".to_owned());
        }
        if !observed.calibration_score.is_finite()
            || observed.calibration_score > self.maximum_calibration_score
        {
            reasons.push("runner calibration is outside the committed tolerance".to_owned());
        }
        ProfileValidation {
            eligible: reasons.is_empty(),
            reasons,
        }
    }
}
