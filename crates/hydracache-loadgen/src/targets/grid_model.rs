//! W4B: in-process library/model primitive characterization.
//!
//! This evidence class exercises exported consistency, session, replication-store,
//! and in-process invalidation APIs. It deliberately starts no daemon, crosses no
//! product data-plane wire, and makes no end-to-end cluster-capacity claim.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::hint::black_box;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use hydracache::{
    resolve_session_read, resolve_session_read_mode, within_staleness_bound, AdaptiveWindow,
    CacheInvalidation, CacheInvalidationBus, CacheInvalidationMessage, CacheInvalidationReceive,
    CacheInvalidationReceiver, ClusterEpoch, ClusterNodeId, ConsistencyLevel,
    EffectiveReplicationMap, HybridLogicalClock, InMemoryInvalidationBus,
    InMemoryReplicatedValueStore, LiveReplicationPeer, PartitionId, PartitionKey, ReadEscalation,
    RegionId, Replicas, ReplicatedValueRecord, SessionReadBudget, SessionReadMode,
    SessionWatermark, StalenessBound, StalenessDecision, VersionStamp,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use thiserror::Error;

pub const GRID_MODEL_SCENARIO_VERSION: u32 = 1;
pub const GRID_MODEL_REPORT_VERSION: u32 = 1;
pub const GRID_MODEL_EVIDENCE_CLASS: &str = "w4b-in-process-library-model";
pub const GRID_MODEL_EXECUTION_MODE: &str = "constructed-in-process-model";
pub const GRID_MODEL_STATE_SCOPE: &str = "library-model-primitives";
pub const GRID_MODEL_NETWORK_BOUNDARY: &str = "none-in-process";
pub const W4_CANARY_MARKER: &str = "HC-CANARY-RED:W4";

const REFERENCE_SCENARIO_ID: &str = "grid-model-primitives-v1";
const REFERENCE_ITERATIONS: u64 = 10_000;
const REFERENCE_WARMUP_ITERATIONS: u64 = 1_000;
const REFERENCE_RAW_REPEATS: u8 = 5;
const REFERENCE_MAXIMUM_SPREAD_MILLIONTHS: u64 = 150_000;
const REFERENCE_REPLICA_SHAPES: &[u8] = &[1, 3, 5, 7];
const REFERENCE_REGION_SHAPES: &[u8] = &[1, 2, 3];
const REFERENCE_PEER_SHAPES: &[u8] = &[1, 2, 4, 6];
const REFERENCE_PAYLOAD_BYTES: &[u32] = &[64, 256, 1_024, 4_096];
const REFERENCE_SUBSCRIBERS: &[u32] = &[1, 3, 5, 7];
const REFERENCE_WATERMARK_ENTRIES: u32 = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelScenario {
    pub schema_version: u32,
    pub scenario_id: String,
    pub identity: GridModelIdentity,
    pub dimensions: GridModelDimensions,
    pub measurement: GridModelMeasurementContract,
    pub reference: GridModelReferenceContract,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelIdentity {
    pub evidence_class: String,
    pub execution_mode: String,
    pub state_scope: String,
    pub network_boundary: String,
    pub daemon_processes: bool,
    pub product_data_plane: bool,
    pub end_to_end_cluster_capacity: bool,
    pub value_replication_separate_from_invalidation: bool,
    pub byte_metric_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelDimensions {
    pub iterations: u64,
    pub replica_shapes: Vec<u8>,
    pub region_shapes: Vec<u8>,
    pub replication_peer_shapes: Vec<u8>,
    pub payload_bytes: Vec<u32>,
    pub invalidation_subscribers: Vec<u32>,
    pub watermark_entries: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelMeasurementContract {
    pub warmup_iterations: u64,
    pub raw_repeats: u8,
    pub maximum_robust_spread_ratio_millionths: u64,
    pub fresh_model_per_repeat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelReferenceContract {
    pub required_profile: String,
    pub require_prebuild_receipt: bool,
    pub require_runner_fingerprint: bool,
    pub required_platform_key: String,
    pub committed_scenario_sha256: String,
    pub runner: GridModelRunnerContract,
    pub prebuild: GridModelPrebuildContract,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelRunnerContract {
    pub required_runner_class: String,
    pub minimum_logical_cores: u32,
    pub required_cpu_affinity: String,
    pub required_cgroup_cpu_quota: String,
    pub require_dedicated: bool,
    pub maximum_calibration_score_millionths: u64,
}

/// Exact W7-compatible build contract. `digest` binds all preceding fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelPrebuildContract {
    pub schema_version: u32,
    pub toolchain_identity: String,
    pub target_set: Vec<String>,
    pub features: Vec<String>,
    pub cargo_profile: String,
    pub flags: Vec<String>,
    pub build_recipe: Vec<String>,
    pub digest: String,
}

impl GridModelScenario {
    pub fn parse_toml(text: &str) -> Result<Self, GridModelError> {
        let scenario: Self = toml::from_str(text)
            .map_err(|error| GridModelError::Contract(format!("invalid TOML: {error}")))?;
        scenario.validate()?;
        Ok(scenario)
    }

    pub fn load(path: &Path) -> Result<Self, GridModelError> {
        let text = fs::read_to_string(path).map_err(|error| {
            GridModelError::Contract(format!(
                "unable to read W4B scenario {}: {error}",
                path.display()
            ))
        })?;
        Self::parse_toml(&text)
    }

    pub fn validate(&self) -> Result<(), GridModelError> {
        if self.schema_version != GRID_MODEL_SCENARIO_VERSION
            || !portable_identifier(&self.scenario_id)
        {
            return Err(GridModelError::Contract(
                "W4B scenario version/id is invalid".to_owned(),
            ));
        }
        self.identity.validate()?;
        self.dimensions.validate()?;
        self.measurement.validate()?;
        self.reference.validate()?;
        Ok(())
    }

    /// Stable digest over the entire scenario except the digest field itself.
    /// Reduced smoke scenarios receive their own digest; only reference mode
    /// requires equality with the digest committed in TOML.
    pub fn contract_sha256(&self) -> String {
        let mut payload = self.clone();
        payload.reference.committed_scenario_sha256.clear();
        digest_json(&payload)
    }

    pub fn validate_exact_reference_shape(&self) -> Result<(), GridModelError> {
        self.validate()?;
        let exact = self.scenario_id == REFERENCE_SCENARIO_ID
            && self.dimensions.iterations == REFERENCE_ITERATIONS
            && self.dimensions.replica_shapes == REFERENCE_REPLICA_SHAPES
            && self.dimensions.region_shapes == REFERENCE_REGION_SHAPES
            && self.dimensions.replication_peer_shapes == REFERENCE_PEER_SHAPES
            && self.dimensions.payload_bytes == REFERENCE_PAYLOAD_BYTES
            && self.dimensions.invalidation_subscribers == REFERENCE_SUBSCRIBERS
            && self.dimensions.watermark_entries == REFERENCE_WATERMARK_ENTRIES
            && self.measurement.warmup_iterations == REFERENCE_WARMUP_ITERATIONS
            && self.measurement.raw_repeats == REFERENCE_RAW_REPEATS
            && self.measurement.maximum_robust_spread_ratio_millionths
                == REFERENCE_MAXIMUM_SPREAD_MILLIONTHS
            && self.measurement.fresh_model_per_repeat
            && self.reference.committed_scenario_sha256 == self.contract_sha256();
        if !exact {
            return Err(GridModelError::Contract(
                "W4B reference mode accepts only the exact committed scenario shape and digest"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

impl GridModelIdentity {
    fn validate(&self) -> Result<(), GridModelError> {
        let honest = self.evidence_class == GRID_MODEL_EVIDENCE_CLASS
            && self.execution_mode == GRID_MODEL_EXECUTION_MODE
            && self.state_scope == GRID_MODEL_STATE_SCOPE
            && self.network_boundary == GRID_MODEL_NETWORK_BOUNDARY
            && !self.daemon_processes
            && !self.product_data_plane
            && !self.end_to_end_cluster_capacity
            && self.value_replication_separate_from_invalidation
            && self.byte_metric_name == "modeled_replica_copy_bytes_per_input_byte";
        if !honest {
            return Err(GridModelError::Boundary(
                "W4B must remain an in-process library/model report with no daemon, product-data-plane, cluster-capacity, or committed-byte-amplification claim".to_owned(),
            ));
        }
        Ok(())
    }
}

impl GridModelDimensions {
    fn validate(&self) -> Result<(), GridModelError> {
        if self.iterations == 0
            || self.iterations > 100_000_000
            || self.replica_shapes.is_empty()
            || self.region_shapes.is_empty()
            || self.replication_peer_shapes.is_empty()
            || self.payload_bytes.is_empty()
            || self.invalidation_subscribers.is_empty()
            || self.watermark_entries == 0
            || self
                .replica_shapes
                .iter()
                .any(|value| *value == 0 || *value > 9)
            || self
                .region_shapes
                .iter()
                .any(|value| *value == 0 || *value > 9)
            || self
                .replication_peer_shapes
                .iter()
                .any(|value| *value == 0 || *value > 8)
            || self.payload_bytes.contains(&0)
            || self.invalidation_subscribers.contains(&0)
            || !unique_and_increasing(&self.replica_shapes)
            || !unique_and_increasing(&self.region_shapes)
            || !unique_and_increasing(&self.replication_peer_shapes)
            || !unique_and_increasing(&self.payload_bytes)
            || !unique_and_increasing(&self.invalidation_subscribers)
        {
            return Err(GridModelError::Contract(
                "W4B dimensions must be non-empty, unique, increasing, non-zero, and bounded"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

impl GridModelMeasurementContract {
    fn validate(&self) -> Result<(), GridModelError> {
        if self.warmup_iterations == 0
            || self.warmup_iterations > 10_000_000
            || !(1..=15).contains(&self.raw_repeats)
            || self.maximum_robust_spread_ratio_millionths > 1_000_000
            || !self.fresh_model_per_repeat
        {
            return Err(GridModelError::Contract(
                "W4B measurement requires bounded warmup/repeats/spread and a fresh model per repeat"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

impl GridModelReferenceContract {
    fn validate(&self) -> Result<(), GridModelError> {
        if self.required_profile != "reference-v1"
            || !self.require_prebuild_receipt
            || !self.require_runner_fingerprint
            || self.required_platform_key != "linux-x86_64"
            || !is_sha256(&self.committed_scenario_sha256)
            || self.runner.required_runner_class != "reference-v1"
            || self.runner.minimum_logical_cores != 8
            || self.runner.required_cpu_affinity != "dedicated-cpuset"
            || self.runner.required_cgroup_cpu_quota != "unlimited"
            || !self.runner.require_dedicated
            || self.runner.maximum_calibration_score_millionths != 50_000
        {
            return Err(GridModelError::Contract(
                "W4B reference contract must exactly match reference-v1 runner policy".to_owned(),
            ));
        }
        self.prebuild.validate()
    }
}

impl GridModelPrebuildContract {
    pub fn computed_digest(&self) -> String {
        #[derive(Serialize)]
        struct Payload<'a> {
            schema_version: u32,
            toolchain_identity: &'a str,
            target_set: &'a [String],
            features: &'a [String],
            cargo_profile: &'a str,
            flags: &'a [String],
            build_recipe: &'a [String],
        }
        digest_json(&Payload {
            schema_version: self.schema_version,
            toolchain_identity: &self.toolchain_identity,
            target_set: &self.target_set,
            features: &self.features,
            cargo_profile: &self.cargo_profile,
            flags: &self.flags,
            build_recipe: &self.build_recipe,
        })
    }

    fn validate(&self) -> Result<(), GridModelError> {
        let exact = self.schema_version == 1
            && self.toolchain_identity == "rustc-1.94.0"
            && self.target_set == ["hydracache-loadgen", "hydracache-server"]
            && self.features.is_empty()
            && self.cargo_profile == "release"
            && self.flags == ["--locked", "--release"]
            && self.build_recipe
                == ["cargo build -p hydracache-loadgen -p hydracache-server --release --locked"]
            && self.digest == self.computed_digest();
        if !exact {
            return Err(GridModelError::Contract(
                "W4B prebuild contract differs from the exact committed W7 reference-v1 contract"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelSourceAttestation {
    pub git_commit: String,
    pub cargo_lock_sha256: String,
    pub git_clean: bool,
    pub cargo_lock_verified_from_disk: bool,
    pub verified_before_measurement: bool,
}

impl GridModelSourceAttestation {
    pub fn from_verified_w7(
        git_commit: impl Into<String>,
        cargo_lock_sha256: impl Into<String>,
    ) -> Result<Self, GridModelError> {
        let source = Self {
            git_commit: git_commit.into(),
            cargo_lock_sha256: cargo_lock_sha256.into(),
            git_clean: true,
            cargo_lock_verified_from_disk: true,
            verified_before_measurement: true,
        };
        source.validate()?;
        Ok(source)
    }

    fn validate(&self) -> Result<(), GridModelError> {
        if !is_git_commit(&self.git_commit)
            || !is_sha256(&self.cargo_lock_sha256)
            || !self.git_clean
            || !self.cargo_lock_verified_from_disk
            || !self.verified_before_measurement
        {
            return Err(GridModelError::Capability(
                "W4B source receipt is non-canonical, dirty, or not disk-verified before measurement"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelRunnerAttestation {
    pub runner_class: String,
    /// Exact fingerprint emitted by the observed W7 runner context.
    pub observed_w7_fingerprint: String,
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
    pub calibration_score_millionths: u64,
    pub observed_before_measurement: bool,
    /// Seal over this typed W4B view; distinct from the W7 fingerprint.
    pub receipt_sha256: String,
}

impl GridModelRunnerAttestation {
    pub fn from_observed_w7(
        observed: &crate::profile::RunnerFingerprint,
    ) -> Result<Self, GridModelError> {
        if !observed.calibration_score.is_finite()
            || !(0.0..=1.0).contains(&observed.calibration_score)
        {
            return Err(GridModelError::Capability(
                "W4B observed W7 runner has an invalid calibration score".to_owned(),
            ));
        }
        let scaled = observed.calibration_score * 1_000_000.0;
        let mut runner = Self {
            runner_class: observed.runner_class.clone(),
            observed_w7_fingerprint: observed.fingerprint.clone(),
            cpu_model: observed.cpu_model.clone(),
            logical_cores: observed.logical_cores,
            ram_bytes: observed.ram_bytes,
            os: observed.os.clone(),
            kernel: observed.kernel.clone(),
            cpu_affinity: observed.cpu_affinity.clone(),
            cgroup_cpu_quota: observed.cgroup_cpu_quota.clone(),
            governor: observed.governor.clone(),
            turbo: observed.turbo.clone(),
            shared_hardware: observed.shared_hardware,
            calibration_score_millionths: scaled.round() as u64,
            observed_before_measurement: true,
            receipt_sha256: String::new(),
        };
        runner.seal();
        Ok(runner)
    }

    pub fn seal(&mut self) {
        self.receipt_sha256 = self.computed_receipt_sha256();
    }

    pub fn computed_receipt_sha256(&self) -> String {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        digest_json(&payload)
    }

    fn validate(&self, contract: &GridModelRunnerContract) -> Result<(), GridModelError> {
        if self.runner_class != contract.required_runner_class
            || self.observed_w7_fingerprint.trim().is_empty()
            || self.receipt_sha256 != self.computed_receipt_sha256()
            || self.cpu_model.trim().is_empty()
            || self.logical_cores < contract.minimum_logical_cores
            || self.ram_bytes == 0
            || self.os.trim().is_empty()
            || self.kernel.trim().is_empty()
            || self.cpu_affinity != contract.required_cpu_affinity
            || self.cgroup_cpu_quota != contract.required_cgroup_cpu_quota
            || self.governor.trim().is_empty()
            || self.turbo.trim().is_empty()
            || (contract.require_dedicated && self.shared_hardware)
            || self.calibration_score_millionths > contract.maximum_calibration_score_millionths
            || !self.observed_before_measurement
        {
            return Err(GridModelError::Capability(
                "W4B runner receipt is unsealed or differs from the committed dedicated-runner contract"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelVerifiedBinary {
    pub id: String,
    pub canonical_path: PathBuf,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelPrebuildAttestation {
    pub schema_version: u32,
    pub source: GridModelSourceAttestation,
    pub build: GridModelPrebuildContract,
    pub build_contract_digest: String,
    pub prebuild_manifest_path: PathBuf,
    /// Exact SHA-256 of the W7 prebuild-manifest file verified from disk.
    pub prebuild_manifest_sha256: String,
    pub runner_profile: String,
    pub runner_fingerprint: String,
    pub platform_key: String,
    pub binaries: Vec<GridModelVerifiedBinary>,
    pub files_verified_from_disk: bool,
    pub verified_before_measurement: bool,
    /// Seal over this typed W4B view; distinct from the manifest file SHA.
    pub receipt_sha256: String,
}

impl GridModelPrebuildAttestation {
    /// Read, hash, parse, and cross-bind the exact W7 prebuild manifest. The
    /// caller cannot substitute synthetic build fields or binary hashes.
    pub fn from_verified_manifest(
        scenario: &GridModelScenario,
        source: GridModelSourceAttestation,
        runner: &GridModelRunnerAttestation,
        manifest_path: PathBuf,
        expected_manifest_sha256: String,
    ) -> Result<Self, GridModelError> {
        scenario.validate_exact_reference_shape()?;
        source.validate()?;
        runner.validate(&scenario.reference.runner)?;
        let (canonical_manifest_path, manifest) =
            read_w7_prebuild_manifest(&manifest_path, &expected_manifest_sha256)?;
        let build = manifest.build_contract();
        let binaries = manifest
            .binaries
            .into_iter()
            .map(|binary| GridModelVerifiedBinary {
                id: binary.id,
                canonical_path: binary.canonical_path,
                sha256: binary.sha256,
            })
            .collect();
        let mut receipt = Self {
            schema_version: 1,
            source: source.clone(),
            build,
            build_contract_digest: manifest.build_contract_digest,
            prebuild_manifest_path: canonical_manifest_path,
            prebuild_manifest_sha256: expected_manifest_sha256,
            runner_profile: manifest.runner_profile,
            runner_fingerprint: manifest.runner_fingerprint,
            platform_key: manifest.platform_key,
            binaries,
            files_verified_from_disk: true,
            verified_before_measurement: true,
            receipt_sha256: String::new(),
        };
        receipt.seal();
        receipt.validate(scenario, &source, runner)?;
        Ok(receipt)
    }

    pub fn seal(&mut self) {
        self.receipt_sha256 = self.computed_receipt_sha256();
    }

    pub fn computed_receipt_sha256(&self) -> String {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        digest_json(&payload)
    }

    fn validate(
        &self,
        scenario: &GridModelScenario,
        source: &GridModelSourceAttestation,
        runner: &GridModelRunnerAttestation,
    ) -> Result<(), GridModelError> {
        validate_verified_binaries(&self.binaries)?;
        validate_w7_manifest_against_attestation(self)?;
        if self.schema_version != 1
            || &self.source != source
            || self.build != scenario.reference.prebuild
            || self.build_contract_digest != self.build.computed_digest()
            || self.build_contract_digest != scenario.reference.prebuild.digest
            || !is_sha256(&self.prebuild_manifest_sha256)
            || self.runner_profile != scenario.reference.required_profile
            || self.runner_fingerprint != runner.observed_w7_fingerprint
            || self.platform_key != scenario.reference.required_platform_key
            || !self.files_verified_from_disk
            || !self.verified_before_measurement
            || self.receipt_sha256 != self.computed_receipt_sha256()
        {
            return Err(GridModelError::Capability(
                "W4B prebuild receipt is not an exact, disk-verified, source/runner-bound W7 receipt"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelReferenceAttestation {
    pub profile: String,
    pub scenario_sha256: String,
    pub source: GridModelSourceAttestation,
    pub runner: GridModelRunnerAttestation,
    pub prebuild: GridModelPrebuildAttestation,
    pub receipt_sha256: String,
}

impl GridModelReferenceAttestation {
    /// Seal an exact typed source/runner/prebuild capability. Validation runs
    /// before the receipt is returned, so no caller-supplied top-level digest
    /// can be accepted on faith.
    pub fn from_verified_parts(
        scenario: &GridModelScenario,
        source: GridModelSourceAttestation,
        runner: GridModelRunnerAttestation,
        prebuild: GridModelPrebuildAttestation,
    ) -> Result<Self, GridModelError> {
        let mut attestation = Self {
            profile: scenario.reference.required_profile.clone(),
            scenario_sha256: scenario.contract_sha256(),
            source,
            runner,
            prebuild,
            receipt_sha256: String::new(),
        };
        attestation.seal();
        attestation.validate(scenario)?;
        Ok(attestation)
    }

    pub fn seal(&mut self) {
        self.receipt_sha256 = self.computed_receipt_sha256();
    }

    pub fn computed_receipt_sha256(&self) -> String {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        digest_json(&payload)
    }

    pub fn validate(
        &self,
        scenario: &GridModelScenario,
    ) -> Result<ValidatedGridModelReference, GridModelError> {
        scenario.validate_exact_reference_shape()?;
        self.source.validate()?;
        if self.profile != scenario.reference.required_profile
            || self.scenario_sha256 != scenario.reference.committed_scenario_sha256
            || self.receipt_sha256 != self.computed_receipt_sha256()
        {
            return Err(GridModelError::Capability(
                "W4B reference receipt has a non-canonical source or broken scenario/receipt binding"
                    .to_owned(),
            ));
        }
        self.runner.validate(&scenario.reference.runner)?;
        self.prebuild
            .validate(scenario, &self.source, &self.runner)?;
        Ok(ValidatedGridModelReference {
            profile: self.profile.clone(),
            source_commit: self.source.git_commit.clone(),
            runner_fingerprint: self.runner.observed_w7_fingerprint.clone(),
            prebuild_manifest_sha256: self.prebuild.prebuild_manifest_sha256.clone(),
            receipt_sha256: self.receipt_sha256.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedGridModelReference {
    pub profile: String,
    pub source_commit: String,
    pub runner_fingerprint: String,
    pub prebuild_manifest_sha256: String,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitiveRawRepeat {
    pub repeat_index: u8,
    pub warmup_iterations: u64,
    pub steady_iterations: u64,
    pub fresh_model_identity_sha256: String,
    pub elapsed_nanos: u64,
    pub result_checksum: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitiveTimingSummary {
    pub raw_repeats: Vec<PrimitiveRawRepeat>,
    pub median_elapsed_nanos: u64,
    pub robust_spread_ratio_millionths: u64,
    pub stable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AckRequirementCostPoint {
    pub consistency_level: String,
    pub replica_count: u8,
    pub region_count: u8,
    pub iterations: u64,
    pub requirement_checksum: u64,
    pub timing: PrimitiveTimingSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionDecisionCostPoint {
    pub helper: String,
    pub watermark_entries: u32,
    pub iterations: u64,
    pub decision_checksum: u64,
    pub timing: PrimitiveTimingSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplicationPrimitiveCostPoint {
    pub replica_peers: u8,
    pub payload_bytes: u32,
    pub iterations: u64,
    pub admitted_sends: u64,
    pub input_bytes: u64,
    pub modeled_replica_copy_bytes: u64,
    pub modeled_replica_copy_bytes_per_input_byte_millionths: u64,
    pub store_retained_bytes: u64,
    pub final_record_checksum: u64,
    pub result_checksum: u64,
    pub timing: PrimitiveTimingSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvalidationFanoutCostPoint {
    pub bus_kind: String,
    pub subscriber_count: u32,
    pub iterations: u64,
    pub messages_published: u64,
    pub deliveries_observed: u64,
    pub value_bytes_replicated: u64,
    pub delivery_checksum: u64,
    pub timing: PrimitiveTimingSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelReport {
    pub schema_version: u32,
    pub scenario_id: String,
    pub scenario_sha256: String,
    pub evidence_class: String,
    pub execution_mode: String,
    pub daemon_processes: bool,
    pub product_data_plane: bool,
    pub end_to_end_cluster_capacity: bool,
    pub byte_metric_name: String,
    pub run_mode: GridModelRunMode,
    pub reference_capability: Option<GridModelReferenceAttestation>,
    pub ack_requirement_cost: Vec<AckRequirementCostPoint>,
    pub session_decision_cost: Vec<SessionDecisionCostPoint>,
    pub replication_primitive_curve: Vec<ReplicationPrimitiveCostPoint>,
    pub invalidation_fanout_cost: Vec<InvalidationFanoutCostPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GridModelRunMode {
    Smoke,
    Reference,
}

impl GridModelReport {
    pub fn validate(&self, scenario: &GridModelScenario) -> Result<(), GridModelError> {
        scenario.validate()?;
        if self.schema_version != GRID_MODEL_REPORT_VERSION
            || self.scenario_id != scenario.scenario_id
            || self.scenario_sha256 != scenario.contract_sha256()
            || self.evidence_class != GRID_MODEL_EVIDENCE_CLASS
            || self.execution_mode != GRID_MODEL_EXECUTION_MODE
            || self.daemon_processes
            || self.product_data_plane
            || self.end_to_end_cluster_capacity
            || self.byte_metric_name != "modeled_replica_copy_bytes_per_input_byte"
            || self.ack_requirement_cost.is_empty()
            || self.session_decision_cost.is_empty()
            || self.replication_primitive_curve.is_empty()
            || self.invalidation_fanout_cost.is_empty()
        {
            return Err(GridModelError::Boundary(
                "W4B report is empty, scenario-unbound, merged with W4A, or overclaims a daemon/data-plane/cluster-capacity boundary".to_owned(),
            ));
        }
        match self.run_mode {
            GridModelRunMode::Smoke if self.reference_capability.is_none() => {}
            GridModelRunMode::Reference => {
                self.reference_capability
                    .as_ref()
                    .ok_or_else(|| {
                        GridModelError::Capability(
                            "W4B reference report has no capability receipt".to_owned(),
                        )
                    })?
                    .validate(scenario)?;
            }
            GridModelRunMode::Smoke => {
                return Err(GridModelError::Boundary(
                    "W4B smoke report cannot carry a reference capability claim".to_owned(),
                ));
            }
        }

        self.validate_ack_points(scenario)?;
        self.validate_session_points(scenario)?;
        self.validate_replication_points(scenario)?;
        self.validate_invalidation_points(scenario)
    }

    fn validate_ack_points(&self, scenario: &GridModelScenario) -> Result<(), GridModelError> {
        let expected_keys = expected_ack_keys(scenario);
        let observed = self
            .ack_requirement_cost
            .iter()
            .map(|point| {
                (
                    point.consistency_level.as_str(),
                    point.replica_count,
                    point.region_count,
                )
            })
            .collect::<BTreeSet<_>>();
        if observed.len() != self.ack_requirement_cost.len() || observed != expected_keys {
            return Err(GridModelError::Evidence(
                "W4B acknowledgement matrix is duplicate, partial, or outside committed dimensions"
                    .to_owned(),
            ));
        }
        for point in &self.ack_requirement_cost {
            let expected = expected_ack_checksum(
                &point.consistency_level,
                point.replica_count,
                point.region_count,
                scenario.dimensions.iterations,
            )?;
            let key = format!(
                "{}-{}-{}",
                point.consistency_level, point.replica_count, point.region_count
            );
            if point.iterations != scenario.dimensions.iterations
                || point.requirement_checksum != expected
            {
                return Err(GridModelError::Evidence(
                    "W4B acknowledgement checksum differs from the independently derived result"
                        .to_owned(),
                ));
            }
            validate_timing(&point.timing, scenario, "ack-requirement", &key, expected)?;
        }
        Ok(())
    }

    fn validate_session_points(&self, scenario: &GridModelScenario) -> Result<(), GridModelError> {
        let expected_helpers = BTreeSet::from([
            "resolve_session_read",
            "resolve_session_read_mode",
            "within_staleness_bound",
        ]);
        let observed = self
            .session_decision_cost
            .iter()
            .map(|point| point.helper.as_str())
            .collect::<BTreeSet<_>>();
        if observed.len() != self.session_decision_cost.len() || observed != expected_helpers {
            return Err(GridModelError::Evidence(
                "W4B session helper evidence is duplicate, partial, or renamed".to_owned(),
            ));
        }
        for point in &self.session_decision_cost {
            let expected =
                expected_session_checksum(&point.helper, scenario.dimensions.iterations)?;
            if point.watermark_entries != scenario.dimensions.watermark_entries
                || point.iterations != scenario.dimensions.iterations
                || point.decision_checksum != expected
            {
                return Err(GridModelError::Evidence(
                    "W4B session checksum differs from the independently derived result".to_owned(),
                ));
            }
            validate_timing(
                &point.timing,
                scenario,
                "session-decision",
                &point.helper,
                expected,
            )?;
        }
        Ok(())
    }

    fn validate_replication_points(
        &self,
        scenario: &GridModelScenario,
    ) -> Result<(), GridModelError> {
        let expected_keys = scenario
            .dimensions
            .replication_peer_shapes
            .iter()
            .flat_map(|peers| {
                scenario
                    .dimensions
                    .payload_bytes
                    .iter()
                    .map(move |payload| (*peers, *payload))
            })
            .collect::<BTreeSet<_>>();
        let observed = self
            .replication_primitive_curve
            .iter()
            .map(|point| (point.replica_peers, point.payload_bytes))
            .collect::<BTreeSet<_>>();
        if observed.len() != self.replication_primitive_curve.len() || observed != expected_keys {
            return Err(GridModelError::Evidence(
                "W4B replication curve is duplicate, partial, or outside committed dimensions"
                    .to_owned(),
            ));
        }
        for point in &self.replication_primitive_curve {
            let expected = expected_replication_outcome(
                point.replica_peers,
                point.payload_bytes,
                scenario.measurement.warmup_iterations,
                scenario.dimensions.iterations,
            )?;
            if point.iterations != scenario.dimensions.iterations
                || point.admitted_sends != expected.admitted_sends
                || point.input_bytes != expected.input_bytes
                || point.modeled_replica_copy_bytes != expected.modeled_replica_copy_bytes
                || point.modeled_replica_copy_bytes_per_input_byte_millionths != expected.ratio
                || point.store_retained_bytes != expected.store_retained_bytes
                || point.final_record_checksum != expected.final_record_checksum
                || point.result_checksum != expected.result_checksum
            {
                return Err(GridModelError::Evidence(
                    "W4B replication accounting/checksum differs from the independent model"
                        .to_owned(),
                ));
            }
            validate_timing(
                &point.timing,
                scenario,
                "replication-primitive",
                &format!("{}-{}", point.replica_peers, point.payload_bytes),
                expected.result_checksum,
            )?;
        }
        Ok(())
    }

    fn validate_invalidation_points(
        &self,
        scenario: &GridModelScenario,
    ) -> Result<(), GridModelError> {
        let expected_keys = scenario
            .dimensions
            .invalidation_subscribers
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let observed = self
            .invalidation_fanout_cost
            .iter()
            .map(|point| point.subscriber_count)
            .collect::<BTreeSet<_>>();
        if observed.len() != self.invalidation_fanout_cost.len() || observed != expected_keys {
            return Err(GridModelError::Evidence(
                "W4B invalidation curve is duplicate, partial, or outside committed dimensions"
                    .to_owned(),
            ));
        }
        for point in &self.invalidation_fanout_cost {
            let expected_checksum = expected_invalidation_checksum(
                point.subscriber_count,
                scenario.dimensions.iterations,
            );
            if point.bus_kind != "in-memory-in-process-broadcast"
                || point.iterations != scenario.dimensions.iterations
                || point.messages_published != scenario.dimensions.iterations
                || point.deliveries_observed
                    != scenario
                        .dimensions
                        .iterations
                        .saturating_mul(u64::from(point.subscriber_count))
                || point.value_bytes_replicated != 0
                || point.delivery_checksum != expected_checksum
            {
                return Err(GridModelError::Evidence(
                    "W4B invalidation fanout must be exact, in-process, value-free, and checksum-bound"
                        .to_owned(),
                ));
            }
            validate_timing(
                &point.timing,
                scenario,
                "invalidation-fanout",
                &point.subscriber_count.to_string(),
                expected_checksum,
            )?;
        }
        Ok(())
    }
}

/// W4 composite-canary leg: replace a real output with a constant, non-zero
/// short-circuit. The independent checksum oracle must still reject it.
pub fn canary_grid_model_short_circuit_is_rejected(
    scenario: &GridModelScenario,
    report: &GridModelReport,
) -> Result<(), String> {
    report
        .validate(scenario)
        .map_err(|error| format!("W4B canary baseline is invalid: {error}"))?;
    let mut injected = report.clone();
    let point = injected
        .ack_requirement_cost
        .first_mut()
        .ok_or_else(|| "W4B canary baseline has no acknowledgement point".to_owned())?;
    point.requirement_checksum = 1;
    for repeat in &mut point.timing.raw_repeats {
        repeat.result_checksum = 1;
    }
    if injected.validate(scenario).is_err() {
        Err(format!(
            "{W4_CANARY_MARKER} injected non-zero grid-model short circuit was rejected"
        ))
    } else {
        Ok(())
    }
}

/// Run the reduced/fast W4B instrument without claiming reference eligibility.
pub async fn run_grid_model_smoke(
    scenario: &GridModelScenario,
) -> Result<GridModelReport, GridModelError> {
    run_grid_model(scenario, GridModelRunMode::Smoke, None).await
}

/// Run W4B reference evidence only for the exact committed shape and after a
/// sealed source/prebuild/runner receipt has passed every structural binding.
pub async fn run_grid_model_reference(
    scenario: &GridModelScenario,
    attestation: GridModelReferenceAttestation,
) -> Result<GridModelReport, GridModelError> {
    scenario.validate_exact_reference_shape()?;
    attestation.validate(scenario)?;
    run_grid_model(scenario, GridModelRunMode::Reference, Some(attestation)).await
}

async fn run_grid_model(
    scenario: &GridModelScenario,
    run_mode: GridModelRunMode,
    reference_capability: Option<GridModelReferenceAttestation>,
) -> Result<GridModelReport, GridModelError> {
    scenario.validate()?;
    let report = GridModelReport {
        schema_version: GRID_MODEL_REPORT_VERSION,
        scenario_id: scenario.scenario_id.clone(),
        scenario_sha256: scenario.contract_sha256(),
        evidence_class: GRID_MODEL_EVIDENCE_CLASS.to_owned(),
        execution_mode: GRID_MODEL_EXECUTION_MODE.to_owned(),
        daemon_processes: false,
        product_data_plane: false,
        end_to_end_cluster_capacity: false,
        byte_metric_name: "modeled_replica_copy_bytes_per_input_byte".to_owned(),
        run_mode,
        reference_capability,
        ack_requirement_cost: run_ack_requirement_cost(scenario)?,
        session_decision_cost: run_session_decision_cost(scenario)?,
        replication_primitive_curve: run_replication_primitive_curve(scenario)?,
        invalidation_fanout_cost: run_invalidation_fanout_cost(scenario).await?,
    };
    report.validate(scenario)?;
    Ok(report)
}

fn run_ack_requirement_cost(
    scenario: &GridModelScenario,
) -> Result<Vec<AckRequirementCostPoint>, GridModelError> {
    const LEVELS: [ConsistencyLevel; 5] = [
        ConsistencyLevel::One,
        ConsistencyLevel::LocalQuorum,
        ConsistencyLevel::Quorum,
        ConsistencyLevel::EachQuorum,
        ConsistencyLevel::All,
    ];
    let mut points = Vec::new();
    for &replica_count in &scenario.dimensions.replica_shapes {
        let region_counts = scenario
            .dimensions
            .region_shapes
            .iter()
            .map(|regions| (*regions).min(replica_count))
            .collect::<BTreeSet<_>>();
        for region_count in region_counts {
            for level in LEVELS {
                let level_label = consistency_label(level);
                let key = format!("{level_label}-{replica_count}-{region_count}");
                let timing = measure_repeats(scenario, "ack-requirement", &key, || {
                    run_ack_once(
                        level,
                        replica_count,
                        region_count,
                        scenario.measurement.warmup_iterations,
                        scenario.dimensions.iterations,
                    )
                })?;
                points.push(AckRequirementCostPoint {
                    consistency_level: level_label.to_owned(),
                    replica_count,
                    region_count,
                    iterations: scenario.dimensions.iterations,
                    requirement_checksum: timing.raw_repeats[0].result_checksum,
                    timing,
                });
            }
        }
    }
    Ok(points)
}

fn run_ack_once(
    level: ConsistencyLevel,
    replica_count: u8,
    region_count: u8,
    warmup_iterations: u64,
    steady_iterations: u64,
) -> Result<MeasuredOnce, GridModelError> {
    let nodes = (0..replica_count)
        .map(|index| ClusterNodeId::new(format!("model-node-{index}")))
        .collect::<Vec<_>>();
    let map = EffectiveReplicationMap::new(Replicas::new(
        nodes[0].clone(),
        nodes.iter().skip(1).cloned().collect(),
    ));
    let topology = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            (
                node.clone(),
                RegionId::new(format!(
                    "model-region-{}",
                    index % usize::from(region_count)
                )),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let local_region = RegionId::new("model-region-0");
    for _ in 0..warmup_iterations {
        black_box(level.required_acks(&map, &topology, &local_region));
    }
    let started = Instant::now();
    let mut checksum = checksum_seed();
    for iteration in 0..steady_iterations {
        let requirement = black_box(level.required_acks(
            black_box(&map),
            black_box(&topology),
            black_box(&local_region),
        ));
        checksum = checksum_mix(checksum, requirement.total_replicas as u64);
        checksum = checksum_mix(checksum, requirement.required_total as u64);
        checksum = checksum_mix(
            checksum,
            requirement
                .required_per_region
                .values()
                .copied()
                .sum::<usize>() as u64,
        );
        checksum = checksum_mix(checksum, iteration);
    }
    Ok(MeasuredOnce {
        elapsed_nanos: elapsed_nanos(started),
        checksum,
    })
}

fn run_session_decision_cost(
    scenario: &GridModelScenario,
) -> Result<Vec<SessionDecisionCostPoint>, GridModelError> {
    [
        "resolve_session_read",
        "resolve_session_read_mode",
        "within_staleness_bound",
    ]
    .into_iter()
    .map(|helper| {
        let timing = measure_repeats(scenario, "session-decision", helper, || {
            run_session_once(
                helper,
                scenario.dimensions.watermark_entries,
                scenario.measurement.warmup_iterations,
                scenario.dimensions.iterations,
            )
        })?;
        Ok(SessionDecisionCostPoint {
            helper: helper.to_owned(),
            watermark_entries: scenario.dimensions.watermark_entries,
            iterations: scenario.dimensions.iterations,
            decision_checksum: timing.raw_repeats[0].result_checksum,
            timing,
        })
    })
    .collect()
}

fn run_session_once(
    helper: &str,
    entry_count: u32,
    warmup_iterations: u64,
    steady_iterations: u64,
) -> Result<MeasuredOnce, GridModelError> {
    let (watermark, keys) = fresh_watermark(entry_count)?;
    for iteration in 0..warmup_iterations {
        black_box(run_session_operation(helper, &watermark, &keys, iteration)?);
    }
    let started = Instant::now();
    let mut checksum = checksum_seed();
    for iteration in 0..steady_iterations {
        checksum = checksum_mix(
            checksum,
            run_session_operation(helper, &watermark, &keys, iteration)?,
        );
        checksum = checksum_mix(checksum, iteration);
    }
    Ok(MeasuredOnce {
        elapsed_nanos: elapsed_nanos(started),
        checksum,
    })
}

fn fresh_watermark(
    entry_count: u32,
) -> Result<(SessionWatermark, Vec<PartitionKey>), GridModelError> {
    let mut watermark = SessionWatermark::new(entry_count as usize);
    let keys = (0..entry_count)
        .map(|index| {
            let key = PartitionKey::new(
                PartitionId::new(index),
                RegionId::new(format!("model-region-{}", index % 3)),
            );
            watermark.record_write(
                key.clone(),
                VersionStamp::new(
                    u64::from(index) + 10,
                    ClusterEpoch::new(7),
                    HybridLogicalClock::new(u64::from(index) + 1_000, 0),
                ),
            );
            key
        })
        .collect::<Vec<_>>();
    if watermark.len() != keys.len() {
        return Err(GridModelError::Execution(
            "W4B session watermark setup was unexpectedly coarsened".to_owned(),
        ));
    }
    Ok((watermark, keys))
}

fn run_session_operation(
    helper: &str,
    watermark: &SessionWatermark,
    keys: &[PartitionKey],
    iteration: u64,
) -> Result<u64, GridModelError> {
    let key = &keys[(iteration as usize) % keys.len()];
    let required = watermark
        .highest_seen(key)
        .ok_or_else(|| GridModelError::Execution("W4B session watermark lost a key".to_owned()))?;
    let candidate = stamp_with_version(required, required.version.saturating_sub(1));
    let bounded_mode = SessionReadMode::BoundedStaleness {
        max: StalenessBound::versions(2),
    };
    match helper {
        "resolve_session_read" => Ok(read_escalation_code(black_box(resolve_session_read(
            black_box(watermark),
            black_box(key),
            black_box(candidate),
            SessionReadBudget::strict(),
        )))),
        "resolve_session_read_mode" => Ok(staleness_decision_code(black_box(
            resolve_session_read_mode(
                black_box(watermark),
                black_box(key),
                Some(stamp_with_version(
                    required,
                    required.version.saturating_sub(2),
                )),
                black_box(candidate),
                bounded_mode,
            ),
        ))),
        "within_staleness_bound" => Ok(u64::from(black_box(within_staleness_bound(
            black_box(watermark),
            black_box(key),
            Some(stamp_with_version(
                required,
                required.version.saturating_sub(2),
            )),
            black_box(candidate),
            bounded_mode,
        )))),
        _ => Err(GridModelError::Execution(format!(
            "unknown W4B session helper {helper}"
        ))),
    }
}

fn run_replication_primitive_curve(
    scenario: &GridModelScenario,
) -> Result<Vec<ReplicationPrimitiveCostPoint>, GridModelError> {
    let mut points = Vec::new();
    for &replica_peers in &scenario.dimensions.replication_peer_shapes {
        for &payload_bytes in &scenario.dimensions.payload_bytes {
            let expected = expected_replication_outcome(
                replica_peers,
                payload_bytes,
                scenario.measurement.warmup_iterations,
                scenario.dimensions.iterations,
            )?;
            let key = format!("{replica_peers}-{payload_bytes}");
            let timing = measure_repeats(scenario, "replication-primitive", &key, || {
                run_replication_once(
                    replica_peers,
                    payload_bytes,
                    scenario.measurement.warmup_iterations,
                    scenario.dimensions.iterations,
                    &expected,
                )
            })?;
            points.push(ReplicationPrimitiveCostPoint {
                replica_peers,
                payload_bytes,
                iterations: scenario.dimensions.iterations,
                admitted_sends: expected.admitted_sends,
                input_bytes: expected.input_bytes,
                modeled_replica_copy_bytes: expected.modeled_replica_copy_bytes,
                modeled_replica_copy_bytes_per_input_byte_millionths: expected.ratio,
                store_retained_bytes: expected.store_retained_bytes,
                final_record_checksum: expected.final_record_checksum,
                result_checksum: expected.result_checksum,
                timing,
            });
        }
    }
    Ok(points)
}

fn run_replication_once(
    replica_peers: u8,
    payload_bytes: u32,
    warmup_iterations: u64,
    steady_iterations: u64,
    expected: &ReplicationOutcome,
) -> Result<MeasuredOnce, GridModelError> {
    let mut peers = (0..replica_peers)
        .map(|index| {
            LiveReplicationPeer::new(
                format!("model-backup-{index}"),
                AdaptiveWindow::new(1, 4, 64),
            )
        })
        .collect::<Vec<_>>();
    let store_budget = u64::from(payload_bytes)
        .checked_mul(u64::from(replica_peers))
        .and_then(|value| value.checked_mul(4))
        .ok_or_else(|| GridModelError::Execution("store budget overflow".to_owned()))?;
    let mut stores = (0..replica_peers)
        .map(|_| InMemoryReplicatedValueStore::with_budget(store_budget))
        .collect::<Vec<_>>();
    let payload = vec![0xA5; payload_bytes as usize];
    for iteration in 0..warmup_iterations {
        send_replication_iteration(&mut peers, &mut stores, &payload, iteration, false)?;
    }
    let started = Instant::now();
    let mut admitted_sends = 0_u64;
    let mut modeled_copy_bytes = 0_u64;
    for iteration in 0..steady_iterations {
        let accounting = send_replication_iteration(
            &mut peers,
            &mut stores,
            &payload,
            warmup_iterations + iteration,
            true,
        )?;
        admitted_sends = admitted_sends.saturating_add(accounting.0);
        modeled_copy_bytes = modeled_copy_bytes
            .checked_add(accounting.1)
            .ok_or_else(|| GridModelError::Execution("replica-copy bytes overflow".to_owned()))?;
    }
    let elapsed_nanos = elapsed_nanos(started);
    let retained = stores
        .iter()
        .map(InMemoryReplicatedValueStore::total_bytes)
        .try_fold(0_u64, |total, value| total.checked_add(value))
        .ok_or_else(|| GridModelError::Execution("retained bytes overflow".to_owned()))?;
    let final_checksum = stores
        .iter()
        .flat_map(|store| store.snapshot().into_values())
        .fold(checksum_seed(), |checksum, record| {
            checksum_mix(checksum, record.artifact_checksum())
        });
    if admitted_sends != expected.admitted_sends
        || modeled_copy_bytes != expected.modeled_replica_copy_bytes
        || retained != expected.store_retained_bytes
        || final_checksum != expected.final_record_checksum
    {
        return Err(GridModelError::Execution(
            "W4B observed replication state differs from its independent expected model".to_owned(),
        ));
    }
    Ok(MeasuredOnce {
        elapsed_nanos,
        checksum: expected.result_checksum,
    })
}

fn send_replication_iteration(
    peers: &mut [LiveReplicationPeer],
    stores: &mut [InMemoryReplicatedValueStore],
    payload: &[u8],
    iteration: u64,
    account: bool,
) -> Result<(u64, u64), GridModelError> {
    let record = ReplicatedValueRecord::value(
        PartitionId::new((iteration % 257) as u32),
        iteration + 1,
        ClusterEpoch::new(7),
        payload.to_vec(),
    );
    let mut sends = 0_u64;
    let mut bytes = 0_u64;
    for (index, (peer, store)) in peers.iter_mut().zip(stores.iter_mut()).enumerate() {
        let send = black_box(
            peer.send_record(store, format!("model-key-{index}"), record.clone(), true)
                .map_err(|error| GridModelError::Execution(error.to_string()))?,
        );
        if !send.admitted {
            return Err(GridModelError::Execution(
                "W4B peer window rejected an acknowledged model send".to_owned(),
            ));
        }
        if account {
            sends = sends.saturating_add(1);
            bytes = bytes.saturating_add(record.approx_bytes());
        }
    }
    Ok((sends, bytes))
}

async fn run_invalidation_fanout_cost(
    scenario: &GridModelScenario,
) -> Result<Vec<InvalidationFanoutCostPoint>, GridModelError> {
    let mut points = Vec::new();
    for &subscribers in &scenario.dimensions.invalidation_subscribers {
        let key = subscribers.to_string();
        let mut raw_repeats = Vec::new();
        for repeat_index in 0..scenario.measurement.raw_repeats {
            let measured = run_invalidation_once(
                subscribers,
                scenario.measurement.warmup_iterations,
                scenario.dimensions.iterations,
            )
            .await?;
            raw_repeats.push(PrimitiveRawRepeat {
                repeat_index,
                warmup_iterations: scenario.measurement.warmup_iterations,
                steady_iterations: scenario.dimensions.iterations,
                fresh_model_identity_sha256: fresh_model_identity(
                    "invalidation-fanout",
                    &key,
                    repeat_index,
                ),
                elapsed_nanos: measured.elapsed_nanos,
                result_checksum: measured.checksum,
            });
        }
        let timing = summarize_timing(raw_repeats, scenario)?;
        let checksum = expected_invalidation_checksum(subscribers, scenario.dimensions.iterations);
        points.push(InvalidationFanoutCostPoint {
            bus_kind: "in-memory-in-process-broadcast".to_owned(),
            subscriber_count: subscribers,
            iterations: scenario.dimensions.iterations,
            messages_published: scenario.dimensions.iterations,
            deliveries_observed: scenario
                .dimensions
                .iterations
                .saturating_mul(u64::from(subscribers)),
            value_bytes_replicated: 0,
            delivery_checksum: checksum,
            timing,
        });
    }
    Ok(points)
}

async fn run_invalidation_once(
    subscriber_count: u32,
    warmup_iterations: u64,
    steady_iterations: u64,
) -> Result<MeasuredOnce, GridModelError> {
    let capacity = usize::try_from(warmup_iterations.max(steady_iterations))
        .unwrap_or(usize::MAX)
        .clamp(1, 1_000_000);
    let bus = InMemoryInvalidationBus::new(capacity);
    let mut receivers = (0..subscriber_count)
        .map(|_| bus.subscribe())
        .collect::<Vec<_>>();
    if bus.receiver_count() != subscriber_count as usize {
        return Err(GridModelError::Execution(
            "W4B invalidation bus did not retain the exact subscriber count".to_owned(),
        ));
    }
    for iteration in 0..warmup_iterations {
        publish_and_receive(&bus, &mut receivers, "warmup", iteration).await?;
    }
    let started = Instant::now();
    let mut checksum = checksum_seed();
    for iteration in 0..steady_iterations {
        publish_and_receive(&bus, &mut receivers, "steady", iteration).await?;
        checksum = checksum_mix(checksum, iteration);
        for receiver_index in 0..subscriber_count {
            checksum = checksum_mix(checksum, u64::from(receiver_index));
        }
    }
    Ok(MeasuredOnce {
        elapsed_nanos: elapsed_nanos(started),
        checksum,
    })
}

async fn publish_and_receive(
    bus: &InMemoryInvalidationBus,
    receivers: &mut [Box<dyn CacheInvalidationReceiver>],
    phase: &str,
    iteration: u64,
) -> Result<(), GridModelError> {
    let expected_key = format!("model-{phase}-invalidation-{iteration}");
    bus.publish(CacheInvalidationMessage::new(
        "w4b-model-source",
        CacheInvalidation::key(expected_key.clone()),
    ))
    .await
    .map_err(|error| GridModelError::Execution(error.to_string()))?;
    for receiver in receivers {
        match receiver.recv().await {
            CacheInvalidationReceive::Message(message)
                if message.source_id() == "w4b-model-source"
                    && message.invalidation().key_value() == Some(expected_key.as_str()) => {}
            other => {
                return Err(GridModelError::Execution(format!(
                    "W4B invalidation delivery was missing, lagged, closed, or malformed: {other:?}"
                )));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct MeasuredOnce {
    elapsed_nanos: u64,
    checksum: u64,
}

fn measure_repeats(
    scenario: &GridModelScenario,
    primitive: &str,
    key: &str,
    mut run_once: impl FnMut() -> Result<MeasuredOnce, GridModelError>,
) -> Result<PrimitiveTimingSummary, GridModelError> {
    let mut repeats = Vec::new();
    for repeat_index in 0..scenario.measurement.raw_repeats {
        let measured = run_once()?;
        repeats.push(PrimitiveRawRepeat {
            repeat_index,
            warmup_iterations: scenario.measurement.warmup_iterations,
            steady_iterations: scenario.dimensions.iterations,
            fresh_model_identity_sha256: fresh_model_identity(primitive, key, repeat_index),
            elapsed_nanos: measured.elapsed_nanos,
            result_checksum: measured.checksum,
        });
    }
    summarize_timing(repeats, scenario)
}

fn summarize_timing(
    raw_repeats: Vec<PrimitiveRawRepeat>,
    scenario: &GridModelScenario,
) -> Result<PrimitiveTimingSummary, GridModelError> {
    let elapsed = raw_repeats
        .iter()
        .map(|repeat| repeat.elapsed_nanos)
        .collect::<Vec<_>>();
    let median_elapsed_nanos = median_u64(&elapsed)
        .ok_or_else(|| GridModelError::Evidence("W4B repeat set is empty".to_owned()))?;
    let robust_spread_ratio_millionths = robust_spread_millionths(&elapsed, median_elapsed_nanos)?;
    Ok(PrimitiveTimingSummary {
        raw_repeats,
        median_elapsed_nanos,
        robust_spread_ratio_millionths,
        stable: robust_spread_ratio_millionths
            <= scenario.measurement.maximum_robust_spread_ratio_millionths,
    })
}

fn validate_timing(
    timing: &PrimitiveTimingSummary,
    scenario: &GridModelScenario,
    primitive: &str,
    key: &str,
    expected_checksum: u64,
) -> Result<(), GridModelError> {
    if timing.raw_repeats.len() != usize::from(scenario.measurement.raw_repeats) {
        return Err(GridModelError::Evidence(
            "W4B timing does not contain the committed raw repeat count".to_owned(),
        ));
    }
    let mut identities = BTreeSet::new();
    for (index, repeat) in timing.raw_repeats.iter().enumerate() {
        let repeat_index = u8::try_from(index)
            .map_err(|_| GridModelError::Evidence("repeat index overflow".to_owned()))?;
        let expected_identity = fresh_model_identity(primitive, key, repeat_index);
        if repeat.repeat_index != repeat_index
            || repeat.warmup_iterations != scenario.measurement.warmup_iterations
            || repeat.steady_iterations != scenario.dimensions.iterations
            || repeat.fresh_model_identity_sha256 != expected_identity
            || !identities.insert(&repeat.fresh_model_identity_sha256)
            || repeat.elapsed_nanos == 0
            || repeat.result_checksum != expected_checksum
        {
            return Err(GridModelError::Evidence(
                "W4B raw repeat is missing warmup, fresh-model identity, elapsed time, or exact checksum"
                    .to_owned(),
            ));
        }
    }
    let elapsed = timing
        .raw_repeats
        .iter()
        .map(|repeat| repeat.elapsed_nanos)
        .collect::<Vec<_>>();
    let median = median_u64(&elapsed)
        .ok_or_else(|| GridModelError::Evidence("W4B repeat set is empty".to_owned()))?;
    let spread = robust_spread_millionths(&elapsed, median)?;
    let stable = spread <= scenario.measurement.maximum_robust_spread_ratio_millionths;
    if timing.median_elapsed_nanos != median
        || timing.robust_spread_ratio_millionths != spread
        || timing.stable != stable
    {
        return Err(GridModelError::Evidence(
            "W4B median/spread/stability summary was not recomputed from raw repeats".to_owned(),
        ));
    }
    Ok(())
}

fn expected_ack_keys(scenario: &GridModelScenario) -> BTreeSet<(&'static str, u8, u8)> {
    const LEVELS: [ConsistencyLevel; 5] = [
        ConsistencyLevel::One,
        ConsistencyLevel::LocalQuorum,
        ConsistencyLevel::Quorum,
        ConsistencyLevel::EachQuorum,
        ConsistencyLevel::All,
    ];
    scenario
        .dimensions
        .replica_shapes
        .iter()
        .flat_map(|replicas| {
            let regions = scenario
                .dimensions
                .region_shapes
                .iter()
                .map(|regions| (*regions).min(*replicas))
                .collect::<BTreeSet<_>>();
            regions.into_iter().flat_map(move |regions| {
                LEVELS
                    .into_iter()
                    .map(move |level| (consistency_label(level), *replicas, regions))
            })
        })
        .collect()
}

fn expected_ack_checksum(
    level: &str,
    replicas: u8,
    regions: u8,
    iterations: u64,
) -> Result<u64, GridModelError> {
    let region_counts = (0..regions)
        .map(|region| {
            (0..replicas)
                .filter(|node| *node % regions == region)
                .count() as u64
        })
        .collect::<Vec<_>>();
    let quorum = |count: u64| count / 2 + 1;
    let (required_total, required_per_region_sum) = match level {
        "one" => (1, 0),
        "local_quorum" => {
            let value = quorum(region_counts[0]);
            (value, value)
        }
        "quorum" => (quorum(u64::from(replicas)), 0),
        "each_quorum" => {
            let value = region_counts.iter().copied().map(quorum).sum();
            (value, value)
        }
        "all" => (u64::from(replicas).max(1), 0),
        _ => {
            return Err(GridModelError::Evidence(format!(
                "unknown acknowledgement level {level}"
            )));
        }
    };
    let mut checksum = checksum_seed();
    for iteration in 0..iterations {
        checksum = checksum_mix(checksum, u64::from(replicas));
        checksum = checksum_mix(checksum, required_total);
        checksum = checksum_mix(checksum, required_per_region_sum);
        checksum = checksum_mix(checksum, iteration);
    }
    Ok(checksum)
}

fn expected_session_checksum(helper: &str, iterations: u64) -> Result<u64, GridModelError> {
    let code = match helper {
        "resolve_session_read" => 13,
        "resolve_session_read_mode" => 201,
        "within_staleness_bound" => 1,
        _ => {
            return Err(GridModelError::Evidence(format!(
                "unknown session helper {helper}"
            )));
        }
    };
    let mut checksum = checksum_seed();
    for iteration in 0..iterations {
        checksum = checksum_mix(checksum, code);
        checksum = checksum_mix(checksum, iteration);
    }
    Ok(checksum)
}

#[derive(Debug, Clone)]
struct ReplicationOutcome {
    admitted_sends: u64,
    input_bytes: u64,
    modeled_replica_copy_bytes: u64,
    ratio: u64,
    store_retained_bytes: u64,
    final_record_checksum: u64,
    result_checksum: u64,
}

fn expected_replication_outcome(
    replica_peers: u8,
    payload_bytes: u32,
    warmup_iterations: u64,
    steady_iterations: u64,
) -> Result<ReplicationOutcome, GridModelError> {
    let admitted_sends = steady_iterations
        .checked_mul(u64::from(replica_peers))
        .ok_or_else(|| GridModelError::Evidence("admitted send overflow".to_owned()))?;
    let input_bytes = steady_iterations
        .checked_mul(u64::from(payload_bytes))
        .ok_or_else(|| GridModelError::Evidence("input byte overflow".to_owned()))?;
    let modeled_replica_copy_bytes = input_bytes
        .checked_mul(u64::from(replica_peers))
        .ok_or_else(|| GridModelError::Evidence("modeled copy byte overflow".to_owned()))?;
    let ratio = modeled_replica_copy_bytes
        .checked_mul(1_000_000)
        .and_then(|value| value.checked_div(input_bytes))
        .ok_or_else(|| GridModelError::Evidence("modeled copy ratio overflow".to_owned()))?;
    let store_retained_bytes = u64::from(payload_bytes)
        .checked_mul(u64::from(replica_peers))
        .ok_or_else(|| GridModelError::Evidence("retained byte overflow".to_owned()))?;
    let final_iteration = warmup_iterations
        .checked_add(steady_iterations)
        .and_then(|value| value.checked_sub(1))
        .ok_or_else(|| GridModelError::Evidence("final iteration overflow".to_owned()))?;
    let final_record = ReplicatedValueRecord::value(
        PartitionId::new((final_iteration % 257) as u32),
        final_iteration + 1,
        ClusterEpoch::new(7),
        vec![0xA5; payload_bytes as usize],
    );
    let final_record_checksum = (0..replica_peers).fold(checksum_seed(), |checksum, _| {
        checksum_mix(checksum, final_record.artifact_checksum())
    });
    let result_checksum = [
        admitted_sends,
        input_bytes,
        modeled_replica_copy_bytes,
        ratio,
        store_retained_bytes,
        final_record_checksum,
    ]
    .into_iter()
    .fold(checksum_seed(), checksum_mix);
    Ok(ReplicationOutcome {
        admitted_sends,
        input_bytes,
        modeled_replica_copy_bytes,
        ratio,
        store_retained_bytes,
        final_record_checksum,
        result_checksum,
    })
}

fn expected_invalidation_checksum(subscribers: u32, iterations: u64) -> u64 {
    let mut checksum = checksum_seed();
    for iteration in 0..iterations {
        checksum = checksum_mix(checksum, iteration);
        for receiver_index in 0..subscribers {
            checksum = checksum_mix(checksum, u64::from(receiver_index));
        }
    }
    checksum
}

fn fresh_model_identity(primitive: &str, key: &str, repeat_index: u8) -> String {
    digest_json(&(primitive, key, repeat_index, "fresh-model-before-warmup-v1"))
}

fn median_u64(values: &[u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let middle = sorted.len() / 2;
    Some(if sorted.len().is_multiple_of(2) {
        u64::try_from((u128::from(sorted[middle - 1]) + u128::from(sorted[middle])) / 2)
            .unwrap_or(u64::MAX)
    } else {
        sorted[middle]
    })
}

fn robust_spread_millionths(values: &[u64], median: u64) -> Result<u64, GridModelError> {
    let deviations = values
        .iter()
        .map(|value| value.abs_diff(median))
        .collect::<Vec<_>>();
    let mad = median_u64(&deviations)
        .ok_or_else(|| GridModelError::Evidence("W4B spread set is empty".to_owned()))?;
    u64::try_from(
        u128::from(mad)
            .saturating_mul(1_000_000)
            .checked_div(u128::from(median.max(1)))
            .unwrap_or(u128::MAX),
    )
    .map_err(|_| GridModelError::Evidence("W4B spread ratio overflow".to_owned()))
}

fn consistency_label(level: ConsistencyLevel) -> &'static str {
    match level {
        ConsistencyLevel::One => "one",
        ConsistencyLevel::LocalQuorum => "local_quorum",
        ConsistencyLevel::Quorum => "quorum",
        ConsistencyLevel::EachQuorum => "each_quorum",
        ConsistencyLevel::All => "all",
    }
}

fn stamp_with_version(stamp: VersionStamp, version: u64) -> VersionStamp {
    VersionStamp::new(version, stamp.epoch, stamp.hlc)
}

fn read_escalation_code(decision: ReadEscalation) -> u64 {
    match decision {
        ReadEscalation::ServeLocal => 1,
        ReadEscalation::TryHigherLevel(level) => 10 + consistency_code(level),
        ReadEscalation::ReadRepair => 20,
        ReadEscalation::WaitThenFail => 30,
        ReadEscalation::FailUnmet => 40,
    }
}

fn staleness_decision_code(decision: StalenessDecision) -> u64 {
    match decision {
        StalenessDecision::ServeCausal {
            observed_version_lag,
        } => 100 + observed_version_lag,
        StalenessDecision::ServeFast {
            observed_version_lag,
        } => 200 + observed_version_lag,
        StalenessDecision::Escalate {
            observed_version_lag,
            ..
        } => 300 + observed_version_lag,
    }
}

fn consistency_code(level: ConsistencyLevel) -> u64 {
    match level {
        ConsistencyLevel::One => 1,
        ConsistencyLevel::LocalQuorum => 2,
        ConsistencyLevel::Quorum => 3,
        ConsistencyLevel::EachQuorum => 4,
        ConsistencyLevel::All => 5,
    }
}

fn unique_and_increasing<T>(values: &[T]) -> bool
where
    T: Ord + Copy,
{
    values.windows(2).all(|window| window[0] < window[1])
        && values.iter().copied().collect::<BTreeSet<_>>().len() == values.len()
}

fn portable_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn is_git_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct W7PrebuildManifest {
    schema_version: u32,
    source: W7PrebuildSource,
    toolchain_identity: String,
    target_set: Vec<String>,
    features: Vec<String>,
    cargo_profile: String,
    flags: Vec<String>,
    build_recipe: Vec<String>,
    build_contract_digest: String,
    runner_profile: String,
    runner_fingerprint: String,
    platform_key: String,
    binaries: Vec<W7PrebuiltBinary>,
}

impl W7PrebuildManifest {
    fn build_contract(&self) -> GridModelPrebuildContract {
        GridModelPrebuildContract {
            schema_version: self.schema_version,
            toolchain_identity: self.toolchain_identity.clone(),
            target_set: self.target_set.clone(),
            features: self.features.clone(),
            cargo_profile: self.cargo_profile.clone(),
            flags: self.flags.clone(),
            build_recipe: self.build_recipe.clone(),
            digest: self.build_contract_digest.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct W7PrebuildSource {
    git_commit: String,
    cargo_lock_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct W7PrebuiltBinary {
    id: String,
    canonical_path: PathBuf,
    sha256: String,
}

fn read_w7_prebuild_manifest(
    path: &Path,
    expected_sha256: &str,
) -> Result<(PathBuf, W7PrebuildManifest), GridModelError> {
    if !is_sha256(expected_sha256) {
        return Err(GridModelError::Capability(
            "W4B prebuild manifest SHA-256 is not canonical lowercase hex".to_owned(),
        ));
    }
    let canonical = fs::canonicalize(path).map_err(|error| {
        GridModelError::Capability(format!(
            "unable to canonicalize W7 prebuild manifest {}: {error}",
            path.display()
        ))
    })?;
    if canonical.file_name().and_then(|value| value.to_str()) != Some("prebuild-manifest.json") {
        return Err(GridModelError::Capability(
            "W4B reference capability must use the exact W7 prebuild-manifest.json artifact"
                .to_owned(),
        ));
    }
    let metadata = fs::metadata(&canonical).map_err(|error| {
        GridModelError::Capability(format!(
            "unable to stat W7 prebuild manifest {}: {error}",
            canonical.display()
        ))
    })?;
    if !metadata.is_file() || metadata.len() > 1024 * 1024 {
        return Err(GridModelError::Capability(
            "W7 prebuild manifest is not a bounded regular file".to_owned(),
        ));
    }
    let bytes = fs::read(&canonical).map_err(|error| {
        GridModelError::Capability(format!(
            "unable to read W7 prebuild manifest {}: {error}",
            canonical.display()
        ))
    })?;
    if sha256_bytes(&bytes) != expected_sha256 {
        return Err(GridModelError::Capability(
            "W7 prebuild manifest bytes differ from the validated context SHA-256".to_owned(),
        ));
    }
    let manifest = serde_json::from_slice(&bytes).map_err(|error| {
        GridModelError::Capability(format!(
            "W7 prebuild manifest does not match its exact typed schema: {error}"
        ))
    })?;
    Ok((canonical, manifest))
}

fn validate_w7_manifest_against_attestation(
    attestation: &GridModelPrebuildAttestation,
) -> Result<(), GridModelError> {
    let (canonical, manifest) = read_w7_prebuild_manifest(
        &attestation.prebuild_manifest_path,
        &attestation.prebuild_manifest_sha256,
    )?;
    let manifest_binaries = manifest
        .binaries
        .iter()
        .map(|binary| GridModelVerifiedBinary {
            id: binary.id.clone(),
            canonical_path: binary.canonical_path.clone(),
            sha256: binary.sha256.clone(),
        })
        .collect::<Vec<_>>();
    if canonical != attestation.prebuild_manifest_path
        || manifest.schema_version != attestation.schema_version
        || manifest.source.git_commit != attestation.source.git_commit
        || manifest.source.cargo_lock_sha256 != attestation.source.cargo_lock_sha256
        || manifest.build_contract() != attestation.build
        || manifest.build_contract_digest != attestation.build_contract_digest
        || manifest.runner_profile != attestation.runner_profile
        || manifest.runner_fingerprint != attestation.runner_fingerprint
        || manifest.platform_key != attestation.platform_key
        || manifest_binaries != attestation.binaries
    {
        return Err(GridModelError::Capability(
            "serialized W4B prebuild receipt differs from the exact W7 manifest on disk".to_owned(),
        ));
    }
    Ok(())
}

fn validate_verified_binaries(binaries: &[GridModelVerifiedBinary]) -> Result<(), GridModelError> {
    let ids = binaries
        .iter()
        .map(|binary| binary.id.as_str())
        .collect::<BTreeSet<_>>();
    if binaries.len() != 2 || ids != BTreeSet::from(["hydracache-loadgen", "hydracache-server"]) {
        return Err(GridModelError::Capability(
            "W4B prebuild receipt must contain exactly loadgen and server binaries".to_owned(),
        ));
    }
    let mut paths = BTreeSet::new();
    for binary in binaries {
        let canonical = fs::canonicalize(&binary.canonical_path).map_err(|error| {
            GridModelError::Capability(format!(
                "unable to canonicalize verified W4B binary {}: {error}",
                binary.canonical_path.display()
            ))
        })?;
        let expected_filename = format!("{}{}", binary.id, std::env::consts::EXE_SUFFIX);
        if canonical != binary.canonical_path
            || !paths.insert(canonical.clone())
            || canonical.file_name().and_then(|value| value.to_str())
                != Some(expected_filename.as_str())
            || canonical
                .parent()
                .and_then(Path::file_name)
                .and_then(|value| value.to_str())
                != Some("release")
            || !is_sha256(&binary.sha256)
        {
            return Err(GridModelError::Capability(format!(
                "W4B binary {} lacks a unique canonical release path or canonical SHA-256",
                binary.id
            )));
        }
        let metadata = fs::metadata(&canonical).map_err(|error| {
            GridModelError::Capability(format!(
                "unable to stat verified W4B binary {}: {error}",
                canonical.display()
            ))
        })?;
        if !metadata.is_file() || sha256_file(&canonical)? != binary.sha256 {
            return Err(GridModelError::Capability(format!(
                "W4B binary {} differs from the disk-verified prebuild SHA-256",
                binary.id
            )));
        }
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, GridModelError> {
    let mut file = File::open(path).map_err(|error| {
        GridModelError::Capability(format!(
            "unable to open verified W4B binary {}: {error}",
            path.display()
        ))
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            GridModelError::Capability(format!(
                "unable to hash verified W4B binary {}: {error}",
                path.display()
            ))
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn digest_json<T: Serialize + ?Sized>(value: &T) -> String {
    sha256_bytes(&serde_json::to_vec(value).expect("typed W4B evidence serialization cannot fail"))
}

fn checksum_seed() -> u64 {
    0xcbf2_9ce4_8422_2325
}

fn checksum_mix(checksum: u64, value: u64) -> u64 {
    checksum
        .wrapping_mul(0x0000_0100_0000_01b3)
        .wrapping_add(value)
}

fn elapsed_nanos(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos())
        .unwrap_or(u64::MAX)
        .max(1)
}

#[derive(Debug, Error)]
pub enum GridModelError {
    #[error("W4B contract rejected: {0}")]
    Contract(String),
    #[error("W4B surface boundary rejected: {0}")]
    Boundary(String),
    #[error("W4B capability rejected: {0}")]
    Capability(String),
    #[error("W4B evidence rejected: {0}")]
    Evidence(String),
    #[error("W4B primitive execution failed: {0}")]
    Execution(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMITTED_SCENARIO: &str =
        include_str!("../../../../docs/testing/perf-scenarios/0.67/grid-model-primitives-v1.toml");

    #[test]
    fn committed_reference_shape_and_digest_are_exact() {
        let scenario = GridModelScenario::parse_toml(COMMITTED_SCENARIO).unwrap();
        assert_eq!(
            scenario.contract_sha256(),
            scenario.reference.committed_scenario_sha256
        );
        scenario.validate_exact_reference_shape().unwrap();

        let mut reduced = scenario;
        reduced.dimensions.iterations -= 1;
        reduced.reference.committed_scenario_sha256 = reduced.contract_sha256();
        assert!(reduced.validate().is_ok());
        assert!(reduced.validate_exact_reference_shape().is_err());
    }

    #[tokio::test]
    async fn smoke_repeats_are_warm_fresh_and_checksum_validated() {
        let mut scenario = GridModelScenario::parse_toml(COMMITTED_SCENARIO).unwrap();
        scenario.dimensions.iterations = 4;
        scenario.dimensions.replica_shapes = vec![1];
        scenario.dimensions.region_shapes = vec![1];
        scenario.dimensions.replication_peer_shapes = vec![1];
        scenario.dimensions.payload_bytes = vec![8];
        scenario.dimensions.invalidation_subscribers = vec![1];
        scenario.dimensions.watermark_entries = 2;
        scenario.measurement.warmup_iterations = 2;
        scenario.measurement.raw_repeats = 2;
        let report = run_grid_model_smoke(&scenario).await.unwrap();
        report.validate(&scenario).unwrap();
        assert!(report
            .ack_requirement_cost
            .iter()
            .all(|point| point.timing.raw_repeats.len() == 2));
        assert!(canary_grid_model_short_circuit_is_rejected(&scenario, &report).is_err());

        let mut constant_nonzero = report;
        constant_nonzero.ack_requirement_cost[0].requirement_checksum = 1;
        for repeat in &mut constant_nonzero.ack_requirement_cost[0].timing.raw_repeats {
            repeat.result_checksum = 1;
        }
        assert!(constant_nonzero.validate(&scenario).is_err());
    }
}
