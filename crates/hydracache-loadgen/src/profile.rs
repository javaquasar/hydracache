use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub const REFERENCE_RUNNER_CLASS: &str = "self-hosted-bare-metal-v1";
pub const REFERENCE_FINGERPRINT_SCHEMA_VERSION: u32 = 3;
pub const REFERENCE_HOST_CONTRACT_VERSION: &str = "hydracache-reference-host-v3";
pub const REFERENCE_STORAGE_CLASS: &str = "local-nvme";
pub const REFERENCE_OS_IMAGE: &str = "ubuntu-24.04";
pub const REFERENCE_MEASUREMENT_CPUS: [u32; 4] = [1, 2, 3, 4];
pub const REFERENCE_SMT_CONTROL: &str = "off";
pub const REFERENCE_ONLINE_CPUS: &str = "0-7";
pub const REFERENCE_ISOLATED_CPUS: &str = "1-4";
pub const REFERENCE_HOUSEKEEPING_CPUS: &str = "0,5-7";
pub const REFERENCE_IRQ_AFFINITY_POLICY: &str = "housekeeping-only-v1";

/// One logical measurement CPU bound to its physical package/core identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeasurementCore {
    pub logical_cpu: u32,
    pub package_id: u32,
    pub core_id: u32,
}

/// CPU-isolation facts independently probed at measurement time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CpuIsolationAttestation {
    pub smt_control: String,
    pub online_cpus: String,
    pub isolated_cpus: String,
    pub nohz_full_cpus: String,
    pub rcu_nocbs_cpus: String,
    pub housekeeping_cpus: String,
    pub irq_affinity_policy: String,
}

/// Independently probed facts required by reference fingerprint schema v3.
///
/// Raw DMI UUIDs, disk serials, provider ids, and account metadata are never
/// serialized. Only domain-separated SHA-256 digests may enter this structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RunnerAttestationV3 {
    pub schema_version: u32,
    pub contract_version: String,
    pub virtualization: String,
    pub physical_cores: u32,
    pub measurement_cores: Vec<MeasurementCore>,
    pub cpu_isolation: CpuIsolationAttestation,
    pub host_digest: String,
    pub storage_class: String,
    pub storage_identity_digest: String,
    pub os_image: String,
    pub toolchain_identity: String,
    pub prebuild_contract_digest: String,
}

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
    #[serde(default)]
    pub attestation: RunnerAttestationV3,
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
    /// Validate the profile contract before it is allowed to validate a runner.
    pub fn contract_problems(&self) -> Vec<String> {
        let mut reasons = Vec::new();
        if self.name.is_empty()
            || self.required_runner_class.is_empty()
            || self.minimum_logical_cores == 0
            || self.required_cpu_affinity.is_empty()
            || self.required_cgroup_cpu_quota.is_empty()
        {
            reasons.push("performance profile identity is incomplete".to_owned());
        }
        if !self.maximum_calibration_score.is_finite() || self.maximum_calibration_score < 0.0 {
            reasons.push("profile calibration threshold is invalid".to_owned());
        }
        if self.allowed_fingerprints.iter().any(String::is_empty) {
            reasons.push("profile contains an empty runner fingerprint".to_owned());
        }
        reasons
    }

    /// Validate observed facts; a caller-supplied profile name is never sufficient.
    pub fn validate(&self, observed: &RunnerFingerprint) -> ProfileValidation {
        let mut reasons = self.contract_problems();
        if observed.fingerprint.is_empty()
            || observed.cpu_model.is_empty()
            || observed.ram_bytes == 0
            || observed.os.is_empty()
            || observed.kernel.is_empty()
            || observed.governor.is_empty()
            || observed.turbo.is_empty()
        {
            reasons.push("observed runner identity is incomplete".to_owned());
        }
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
            || observed.calibration_score < 0.0
            || observed.calibration_score > self.maximum_calibration_score
        {
            reasons.push("runner calibration is outside the committed tolerance".to_owned());
        }
        if self.required_runner_class == REFERENCE_RUNNER_CLASS {
            reasons.extend(reference_attestation_problems(&observed.attestation));
        }
        ProfileValidation {
            eligible: reasons.is_empty(),
            reasons,
        }
    }
}

pub fn reference_attestation_problems(attestation: &RunnerAttestationV3) -> Vec<String> {
    let mut reasons = Vec::new();
    if attestation.schema_version != REFERENCE_FINGERPRINT_SCHEMA_VERSION {
        reasons.push("reference runner fingerprint schema is not v3".to_owned());
    }
    if attestation.contract_version != REFERENCE_HOST_CONTRACT_VERSION {
        reasons.push("reference host contract version is not approved".to_owned());
    }
    if attestation.virtualization != "none" {
        reasons
            .push("reference runner virtualization was detected or not proven absent".to_owned());
    }
    if attestation.physical_cores < 6 {
        reasons.push("reference host has fewer than six physical cores".to_owned());
    }
    let logical = attestation
        .measurement_cores
        .iter()
        .map(|core| core.logical_cpu)
        .collect::<Vec<_>>();
    if logical != REFERENCE_MEASUREMENT_CPUS {
        reasons.push("measurement topology does not cover exact logical CPUs 1-4".to_owned());
    }
    let physical = attestation
        .measurement_cores
        .iter()
        .map(|core| (core.package_id, core.core_id))
        .collect::<BTreeSet<_>>();
    if physical.len() != REFERENCE_MEASUREMENT_CPUS.len() {
        reasons.push(
            "measurement cpuset contains SMT siblings or duplicate physical cores".to_owned(),
        );
    }
    let isolation = &attestation.cpu_isolation;
    if isolation.smt_control != REFERENCE_SMT_CONTROL {
        reasons.push("reference host SMT is not disabled".to_owned());
    }
    if isolation.online_cpus != REFERENCE_ONLINE_CPUS {
        reasons.push("reference host online CPU set is not the committed 0-7 set".to_owned());
    }
    if isolation.isolated_cpus != REFERENCE_ISOLATED_CPUS
        || isolation.nohz_full_cpus != REFERENCE_ISOLATED_CPUS
        || isolation.rcu_nocbs_cpus != REFERENCE_ISOLATED_CPUS
    {
        reasons.push(
            "measurement CPUs are not isolated from scheduling ticks and RCU callbacks".to_owned(),
        );
    }
    if isolation.housekeeping_cpus != REFERENCE_HOUSEKEEPING_CPUS {
        reasons.push("housekeeping CPU set does not match the committed profile".to_owned());
    }
    if isolation.irq_affinity_policy != REFERENCE_IRQ_AFFINITY_POLICY {
        reasons.push("IRQ affinity is not proven housekeeping-only".to_owned());
    }
    if !is_sha256(&attestation.host_digest) {
        reasons.push("privacy-safe physical host digest is missing or malformed".to_owned());
    }
    if attestation.storage_class != REFERENCE_STORAGE_CLASS {
        reasons.push("reference root storage is not proven local NVMe".to_owned());
    }
    if !is_sha256(&attestation.storage_identity_digest) {
        reasons.push("privacy-safe storage identity digest is missing or malformed".to_owned());
    }
    if attestation.os_image != REFERENCE_OS_IMAGE {
        reasons.push("reference OS image contract does not match Ubuntu 24.04".to_owned());
    }
    if attestation.toolchain_identity.trim().is_empty() {
        reasons.push("reference toolchain identity is missing".to_owned());
    }
    if !is_sha256(&attestation.prebuild_contract_digest) {
        reasons.push("reference prebuild contract digest is missing or malformed".to_owned());
    }
    reasons
}

pub fn reference_cpu_isolation() -> CpuIsolationAttestation {
    CpuIsolationAttestation {
        smt_control: REFERENCE_SMT_CONTROL.to_owned(),
        online_cpus: REFERENCE_ONLINE_CPUS.to_owned(),
        isolated_cpus: REFERENCE_ISOLATED_CPUS.to_owned(),
        nohz_full_cpus: REFERENCE_ISOLATED_CPUS.to_owned(),
        rcu_nocbs_cpus: REFERENCE_ISOLATED_CPUS.to_owned(),
        housekeeping_cpus: REFERENCE_HOUSEKEEPING_CPUS.to_owned(),
        irq_affinity_policy: REFERENCE_IRQ_AFFINITY_POLICY.to_owned(),
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
