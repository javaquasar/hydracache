//! W5 brownout evidence split by the three surfaces that actually exist.
//!
//! Reference producers consume typed W4 predecessor artifacts and raw W0
//! windows.  A driver can no longer hand this module a ready-made W5 verdict.
//! Smoke fixtures carry an explicit origin and cannot be promoted to reference
//! evidence by changing a report-mode field.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use hydracache::{
    AdaptiveWindow, ClusterEpoch, InMemoryReplicatedValueStore, LiveReplicationPeer, PartitionId,
    ReplicatedValueRecord, ReplicatedValueStore,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::rate::OpenLoopObservation;
use crate::report::{
    DimensionValue, EvidenceRunMode, LoadClaim, LoadCurveEvidence, MeasurementEvidence, PerfReport,
    RespEndpointCapability, WorkloadIdentity,
};
use crate::targets::control_plane as w4a;
use crate::targets::grid_model as w4b;
use crate::tiers::resp_reference::{
    RespDaemonEvidence, ValidatedRespReferenceContext, LOADGEN_BINARY_ID, SERVER_BINARY_ID,
};

pub const BROWNOUT_SCENARIO_VERSION: u32 = 1;
pub const BROWNOUT_REPORT_VERSION: u32 = 1;
pub const BROWNOUT_RELEASE: &str = "0.67.0";
pub const W5_CANARY_MARKER: &str = "HC-CANARY-RED:W5";
pub const CONTROL_PLANE_CAPABILITY_ENV: &str = "HYDRACACHE_RUN_PERF_CONTROL_PLANE";
pub const RESP_REFERENCE_ENV: &str = "HYDRACACHE_RUN_PERF_RESP";
pub const GRID_MODEL_REFERENCE_ENV: &str = "HYDRACACHE_RUN_PERF_CORE";

pub const CONTROL_PLANE_EVIDENCE_CLASS: &str = "w5a-control-plane-metadata-brownout";
pub const RESP_EVIDENCE_CLASS: &str = "w5b-selected-node-local-resp-brownout";
pub const GRID_MODEL_EVIDENCE_CLASS: &str = "w5c-in-process-model-replica-fault";

const CONTROL_PLANE_PREDECESSOR: &str = "w4a-real-daemon-control-plane";
const RESP_PREDECESSOR: &str = "w3-node-local-resp-open-loop";
const GRID_MODEL_PREDECESSOR: &str = "w4b-in-process-library-model";
const CONTROL_PLANE_HEADLINE: &str = "committed_metadata_transition_recovery_millis";
const RESP_HEADLINE: &str = "selected_endpoint_socket_and_throughput_recovery_millis";
const GRID_MODEL_HEADLINE: &str = "modeled_replica_decision_backpressure_recovery_nanos";
const REFERENCE_PROFILE: &str = "reference-v1";
const FRACTION_MILLIONTHS: u32 = 600_000;
const OBSERVATION_WINDOW_MILLIS: u64 = 10_000;
const RAW_MODEL_REPEATS: u8 = 5;
const MODEL_WARMUP_ITERATIONS: u64 = 100;
const MODEL_ITERATIONS: u64 = 1_000;
const MODEL_MAX_SPREAD_MILLIONTHS: u64 = 1_000_000;
const MODEL_SLOW_DELAY_MICROS: u64 = 25;
const MODEL_MAX_RECOVERY_NANOS_PER_ITERATION: u64 = 1_000_000;
const RESP_CAPACITY_MEASUREMENTS: [(&str, &str); 3] = [
    ("resp_open_loop_get_set_knee_at_slo_workload_a", "A"),
    ("resp_open_loop_get_set_knee_at_slo_workload_b", "B"),
    ("resp_open_loop_get_set_knee_at_slo_workload_c", "C"),
];

#[derive(Debug, Error)]
pub enum BrownoutError {
    #[error("W5 contract rejected: {0}")]
    Contract(String),
    #[error("W5 claim boundary rejected: {0}")]
    Boundary(String),
    #[error("W5 reference capability rejected: {0}")]
    Capability(String),
    #[error("W5 evidence rejected: {0}")]
    Evidence(String),
    #[error("W5 driver failed: {0}")]
    Driver(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrownoutRunMode {
    DeterministicSmoke,
    Reference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ObservationOrigin {
    Fixture,
    Observed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubKneeLoadContract {
    pub predecessor_evidence_class: String,
    pub requires_valid_predecessor: bool,
    pub fixed_rate_fraction_millionths: u32,
    pub observation_window_millis: u64,
}

impl SubKneeLoadContract {
    fn validate(&self, predecessor: &str) -> Result<(), BrownoutError> {
        if self.predecessor_evidence_class != predecessor
            || !self.requires_valid_predecessor
            || self.fixed_rate_fraction_millionths != FRACTION_MILLIONTHS
            || self.observation_window_millis != OBSERVATION_WINDOW_MILLIS
        {
            return Err(BrownoutError::Contract(format!(
                "W5 load must be the committed 60% window of {predecessor}"
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PredecessorSummary {
    pub evidence_class: String,
    pub artifact_sha256: String,
    pub reference_receipt_sha256: String,
    pub knee_rate_per_second: u64,
    pub offered_rate_per_second: u64,
    pub rate_fraction_millionths: u32,
}

impl PredecessorSummary {
    fn validate(&self, contract: &SubKneeLoadContract) -> Result<(), BrownoutError> {
        let expected = sub_knee_rate(
            self.knee_rate_per_second,
            contract.fixed_rate_fraction_millionths,
        )?;
        if self.evidence_class != contract.predecessor_evidence_class
            || !is_sha256(&self.artifact_sha256)
            || !is_sha256(&self.reference_receipt_sha256)
            || self.knee_rate_per_second == 0
            || self.offered_rate_per_second != expected
            || self.offered_rate_per_second >= self.knee_rate_per_second
            || self.rate_fraction_millionths != contract.fixed_rate_fraction_millionths
        {
            return Err(BrownoutError::Evidence(
                "W5 predecessor is not an exact typed sub-knee reference binding".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlaneBrownoutScenario {
    pub schema_version: u32,
    pub scenario_id: String,
    pub identity: ControlPlaneBrownoutIdentity,
    pub load: SubKneeLoadContract,
    pub events: ControlPlaneEventContract,
    pub reference: ControlPlaneReferenceContract,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlaneBrownoutIdentity {
    pub evidence_class: String,
    pub authority: String,
    pub execution_mode: String,
    pub network_boundary: String,
    pub headline_metric: String,
    pub generic_client_write_invariant: bool,
    pub distributed_value_invariant: bool,
    pub live_reshard_measured: bool,
    pub aggregate_goodput: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlPlaneBrownoutAction {
    LeaderFailover,
    MemberAdd,
    MemberDrain,
    NodeKillRejoin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlaneEventContract {
    pub actions: Vec<ControlPlaneBrownoutAction>,
    pub max_leader_unavailable_millis: u64,
    pub max_transition_recovery_millis: u64,
    pub require_no_lost_committed_metadata_transition: bool,
    pub require_converged_public_membership_views: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlaneReferenceContract {
    pub capability_env: String,
    pub required_profile: String,
    pub predecessor_node_count: u8,
    pub require_real_daemons: bool,
    pub require_process_fault_control: bool,
    pub require_complete_public_endpoint_set: bool,
    pub committed_scenario_sha256: String,
}

impl ControlPlaneBrownoutScenario {
    pub fn parse_toml(text: &str) -> Result<Self, BrownoutError> {
        let scenario: Self = toml::from_str(text)
            .map_err(|error| BrownoutError::Contract(format!("invalid W5A TOML: {error}")))?;
        scenario.validate()?;
        Ok(scenario)
    }

    pub fn load(path: &Path) -> Result<Self, BrownoutError> {
        let text = fs::read_to_string(path).map_err(|error| {
            BrownoutError::Contract(format!("unable to read {}: {error}", path.display()))
        })?;
        Self::parse_toml(&text)
    }

    pub fn contract_sha256(&self) -> String {
        let mut payload = self.clone();
        payload.reference.committed_scenario_sha256.clear();
        digest_json(&payload)
    }

    pub fn validate(&self) -> Result<(), BrownoutError> {
        self.load.validate(CONTROL_PLANE_PREDECESSOR)?;
        if self.schema_version != BROWNOUT_SCENARIO_VERSION
            || !portable_identifier(&self.scenario_id)
            || !is_sha256(&self.reference.committed_scenario_sha256)
        {
            return Err(BrownoutError::Contract(
                "W5A schema, id, or committed digest is invalid".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn validate_exact_reference_shape(&self) -> Result<(), BrownoutError> {
        self.validate()?;
        let actions = BTreeSet::from([
            ControlPlaneBrownoutAction::LeaderFailover,
            ControlPlaneBrownoutAction::MemberAdd,
            ControlPlaneBrownoutAction::MemberDrain,
            ControlPlaneBrownoutAction::NodeKillRejoin,
        ]);
        let exact = self.scenario_id == "brownout-control-plane-v1"
            && self.identity.evidence_class == CONTROL_PLANE_EVIDENCE_CLASS
            && self.identity.authority == "consensus-backed-committed-metadata"
            && self.identity.execution_mode == "real-daemon-admin-observation"
            && self.identity.network_boundary == "public-admin-http"
            && self.identity.headline_metric == CONTROL_PLANE_HEADLINE
            && !self.identity.generic_client_write_invariant
            && !self.identity.distributed_value_invariant
            && !self.identity.live_reshard_measured
            && !self.identity.aggregate_goodput
            && self.events.actions.iter().copied().collect::<BTreeSet<_>>() == actions
            && self.events.actions.len() == actions.len()
            && self.events.max_leader_unavailable_millis == 5_000
            && self.events.max_transition_recovery_millis == 15_000
            && self.events.require_no_lost_committed_metadata_transition
            && self.events.require_converged_public_membership_views
            && self.reference.capability_env == CONTROL_PLANE_CAPABILITY_ENV
            && self.reference.required_profile == REFERENCE_PROFILE
            && self.reference.predecessor_node_count == 3
            && self.reference.require_real_daemons
            && self.reference.require_process_fault_control
            && self.reference.require_complete_public_endpoint_set
            && self.reference.committed_scenario_sha256 == self.contract_sha256();
        if !exact {
            return Err(BrownoutError::Contract(
                "W5A reference accepts only the exact committed shape and digest".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespBrownoutScenario {
    pub schema_version: u32,
    pub scenario_id: String,
    pub identity: RespBrownoutIdentity,
    pub load: SubKneeLoadContract,
    pub event: RespEndpointEventContract,
    pub reference: RespReferenceContract,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespBrownoutIdentity {
    pub evidence_class: String,
    pub authority: String,
    pub execution_mode: String,
    pub network_boundary: String,
    pub headline_metric: String,
    pub node_local_state: bool,
    pub automatic_failover: bool,
    pub neighbor_visibility_claim: bool,
    pub value_survival_claim: bool,
    pub cross_node_failover_claim: bool,
    pub data_recovery_claim: bool,
    pub aggregate_goodput: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespEndpointEventContract {
    pub action: String,
    pub recovery_definition: String,
    pub require_socket_unavailability_observed: bool,
    pub require_post_restart_steady_throughput: bool,
    pub max_recovery_millis: u64,
    pub independent_control_endpoints: u8,
    pub min_independent_control_availability_ppm: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespReferenceContract {
    pub required_profile: String,
    pub require_real_daemon_resp: bool,
    pub require_selected_process_control: bool,
    pub require_prebuild_receipt: bool,
    pub committed_scenario_sha256: String,
}

impl RespBrownoutScenario {
    pub fn parse_toml(text: &str) -> Result<Self, BrownoutError> {
        let scenario: Self = toml::from_str(text)
            .map_err(|error| BrownoutError::Contract(format!("invalid W5B TOML: {error}")))?;
        scenario.validate()?;
        Ok(scenario)
    }

    pub fn load(path: &Path) -> Result<Self, BrownoutError> {
        let text = fs::read_to_string(path).map_err(|error| {
            BrownoutError::Contract(format!("unable to read {}: {error}", path.display()))
        })?;
        Self::parse_toml(&text)
    }

    pub fn contract_sha256(&self) -> String {
        let mut payload = self.clone();
        payload.reference.committed_scenario_sha256.clear();
        digest_json(&payload)
    }

    pub fn validate(&self) -> Result<(), BrownoutError> {
        self.load.validate(RESP_PREDECESSOR)?;
        if self.schema_version != BROWNOUT_SCENARIO_VERSION
            || !portable_identifier(&self.scenario_id)
            || !is_sha256(&self.reference.committed_scenario_sha256)
        {
            return Err(BrownoutError::Contract(
                "W5B schema, id, or committed digest is invalid".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn validate_exact_reference_shape(&self) -> Result<(), BrownoutError> {
        self.validate()?;
        let exact = self.scenario_id == "brownout-resp-endpoint-v1"
            && self.identity.evidence_class == RESP_EVIDENCE_CLASS
            && self.identity.authority == "one-selected-node-local-resp-endpoint"
            && self.identity.execution_mode == "real-daemon-selected-resp-process-kill-restart"
            && self.identity.network_boundary == "real-resp-tcp"
            && self.identity.headline_metric == RESP_HEADLINE
            && self.identity.node_local_state
            && !self.identity.automatic_failover
            && !self.identity.neighbor_visibility_claim
            && !self.identity.value_survival_claim
            && !self.identity.cross_node_failover_claim
            && !self.identity.data_recovery_claim
            && !self.identity.aggregate_goodput
            && self.event.action == "selected_endpoint_kill_restart"
            && self.event.recovery_definition
                == "socket_available_and_post_restart_steady_throughput"
            && self.event.require_socket_unavailability_observed
            && self.event.require_post_restart_steady_throughput
            && self.event.max_recovery_millis == 15_000
            && self.event.independent_control_endpoints == 1
            && self.event.min_independent_control_availability_ppm == 990_000
            && self.reference.required_profile == REFERENCE_PROFILE
            && self.reference.require_real_daemon_resp
            && self.reference.require_selected_process_control
            && self.reference.require_prebuild_receipt
            && self.reference.committed_scenario_sha256 == self.contract_sha256();
        if !exact {
            return Err(BrownoutError::Contract(
                "W5B reference accepts only the exact committed shape and digest".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelBrownoutScenario {
    pub schema_version: u32,
    pub scenario_id: String,
    pub identity: GridModelBrownoutIdentity,
    pub work: ModelFaultWorkContract,
    pub faults: GridModelFaultContract,
    pub reference: GridModelReferenceContract,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelBrownoutIdentity {
    pub evidence_class: String,
    pub authority: String,
    pub execution_mode: String,
    pub network_boundary: String,
    pub headline_metric: String,
    pub daemon_brownout_evidence: bool,
    pub product_data_plane: bool,
    pub live_rebalance_measured: bool,
    pub live_reshard_measured: bool,
    pub aggregate_goodput: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelReplicaFault {
    SlowReplica,
    UnavailableReplica,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelFaultWorkContract {
    pub predecessor_evidence_class: String,
    pub requires_valid_predecessor_report: bool,
    pub no_synthetic_capacity_knee: bool,
    pub fixed_iterations: u64,
    pub warmup_iterations: u64,
    pub raw_repeats: u8,
    pub maximum_robust_spread_ratio_millionths: u64,
    pub fresh_model_per_repeat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelFaultContract {
    pub faults: Vec<ModelReplicaFault>,
    pub iterations: u64,
    pub slow_replica_delay_micros: u64,
    pub unavailable_backpressure_per_iteration: u64,
    pub max_recovery_cost_nanos: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelReferenceContract {
    pub required_profile: String,
    pub require_prebuild_receipt: bool,
    pub require_constructed_model: bool,
    pub require_observed_fault_injection: bool,
    pub committed_scenario_sha256: String,
}

impl GridModelBrownoutScenario {
    pub fn parse_toml(text: &str) -> Result<Self, BrownoutError> {
        let scenario: Self = toml::from_str(text)
            .map_err(|error| BrownoutError::Contract(format!("invalid W5C TOML: {error}")))?;
        scenario.validate()?;
        Ok(scenario)
    }

    pub fn load(path: &Path) -> Result<Self, BrownoutError> {
        let text = fs::read_to_string(path).map_err(|error| {
            BrownoutError::Contract(format!("unable to read {}: {error}", path.display()))
        })?;
        Self::parse_toml(&text)
    }

    pub fn contract_sha256(&self) -> String {
        let mut payload = self.clone();
        payload.reference.committed_scenario_sha256.clear();
        digest_json(&payload)
    }

    pub fn validate(&self) -> Result<(), BrownoutError> {
        if self.schema_version != BROWNOUT_SCENARIO_VERSION
            || !portable_identifier(&self.scenario_id)
            || self.work.predecessor_evidence_class != GRID_MODEL_PREDECESSOR
            || !self.work.requires_valid_predecessor_report
            || !self.work.no_synthetic_capacity_knee
            || self.work.fixed_iterations == 0
            || self.work.warmup_iterations == 0
            || self.work.raw_repeats == 0
            || self.work.raw_repeats > 15
            || self.work.maximum_robust_spread_ratio_millionths > 1_000_000
            || !self.work.fresh_model_per_repeat
            || self.faults.iterations != self.work.fixed_iterations
            || self.faults.slow_replica_delay_micros == 0
            || self.faults.unavailable_backpressure_per_iteration == 0
            || self.faults.max_recovery_cost_nanos == 0
            || !is_sha256(&self.reference.committed_scenario_sha256)
        {
            return Err(BrownoutError::Contract(
                "W5C model, measurement, or digest contract is invalid".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn validate_exact_reference_shape(&self) -> Result<(), BrownoutError> {
        self.validate()?;
        let faults = BTreeSet::from([
            ModelReplicaFault::SlowReplica,
            ModelReplicaFault::UnavailableReplica,
        ]);
        let exact = self.scenario_id == "brownout-grid-model-v1"
            && self.identity.evidence_class == GRID_MODEL_EVIDENCE_CLASS
            && self.identity.authority == "constructed-in-process-replica-model"
            && self.identity.execution_mode == "in-process-model-fault-injection"
            && self.identity.network_boundary == "none-in-process"
            && self.identity.headline_metric == GRID_MODEL_HEADLINE
            && !self.identity.daemon_brownout_evidence
            && !self.identity.product_data_plane
            && !self.identity.live_rebalance_measured
            && !self.identity.live_reshard_measured
            && !self.identity.aggregate_goodput
            && self.work.fixed_iterations == MODEL_ITERATIONS
            && self.work.warmup_iterations == MODEL_WARMUP_ITERATIONS
            && self.work.raw_repeats == RAW_MODEL_REPEATS
            && self.work.maximum_robust_spread_ratio_millionths == MODEL_MAX_SPREAD_MILLIONTHS
            && self.faults.faults.iter().copied().collect::<BTreeSet<_>>() == faults
            && self.faults.faults.len() == faults.len()
            && self.faults.iterations == MODEL_ITERATIONS
            && self.faults.slow_replica_delay_micros == MODEL_SLOW_DELAY_MICROS
            && self.faults.unavailable_backpressure_per_iteration == 1
            && self.faults.max_recovery_cost_nanos == MODEL_MAX_RECOVERY_NANOS_PER_ITERATION
            && self.reference.required_profile == REFERENCE_PROFILE
            && self.reference.require_prebuild_receipt
            && self.reference.require_constructed_model
            && self.reference.require_observed_fault_injection
            && self.reference.committed_scenario_sha256 == self.contract_sha256();
        if !exact {
            return Err(BrownoutError::Contract(
                "W5C reference accepts only the exact committed shape and digest".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Persisted W4A capacity evidence.  The W4 processes must already have been
/// killed and waited: W5 never depends on keeping a predecessor PID alive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlanePredecessor {
    summary: PredecessorSummary,
    predecessor_node_count: u8,
    w4_scenario_sha256: String,
    source_commit: String,
    runner_fingerprint_sha256: String,
    prebuild_manifest_sha256: String,
    prebuild_contract_sha256: String,
    server_binary_sha256: String,
    archived_capability_receipt_sha256: String,
    archived_lifecycle_receipt_sha256: String,
    archived_pids: BTreeSet<u32>,
    leader_node_id: String,
    leader_term: u64,
    leader_epoch: u64,
}

impl ControlPlanePredecessor {
    /// Revalidate an archived W4A report from its exact bytes. The report embeds
    /// the sealed launch attestation and its mandatory lifecycle receipt, so W5
    /// cannot substitute either a caller-supplied attestation or a live old PID.
    pub fn from_w4a_reference(
        scenario: &w4a::ControlPlaneScenario,
        report: &w4a::ControlPlaneReport,
        artifact_json: &[u8],
        rate_fraction_millionths: u32,
        predecessor_node_count: u8,
    ) -> Result<Self, BrownoutError> {
        let archived = report
            .validate_archived(scenario)
            .map_err(|error| BrownoutError::Capability(format!("invalid W4A report: {error}")))?;
        let parsed: w4a::ControlPlaneReport =
            serde_json::from_slice(artifact_json).map_err(|error| {
                BrownoutError::Capability(format!("W4A artifact is not typed JSON: {error}"))
            })?;
        let parsed_archived = parsed.validate_archived(scenario).map_err(|error| {
            BrownoutError::Capability(format!("serialized W4A artifact is invalid: {error}"))
        })?;
        if digest_json(&parsed) != digest_json(report) || parsed_archived != archived {
            return Err(BrownoutError::Capability(
                "W5A predecessor object differs from the exact artifact bytes".to_owned(),
            ));
        }
        let leader = report
            .steady_reads
            .iter()
            .find(|read| read.target_node_role == w4a::NodeRole::Leader)
            .ok_or_else(|| BrownoutError::Capability("W4A has no leader knee".to_owned()))?;
        let knee_rate_per_second = exact_u64_rate(
            leader
                .knee
                .sustainable_rate_per_second
                .ok_or_else(|| BrownoutError::Capability("W4A leader knee is empty".to_owned()))?,
            "W4A leader knee",
        )?;
        let offered_rate_per_second =
            sub_knee_rate(knee_rate_per_second, rate_fraction_millionths)?;
        if predecessor_node_count != 3
            || report.node_count != predecessor_node_count
            || archived.node_count != predecessor_node_count
        {
            return Err(BrownoutError::Capability(
                "W5A accepts only the canonical archived 3-node W4A predecessor".to_owned(),
            ));
        }
        let result = Self {
            summary: PredecessorSummary {
                evidence_class: CONTROL_PLANE_PREDECESSOR.to_owned(),
                artifact_sha256: sha256_hex(artifact_json),
                reference_receipt_sha256: report.capability_receipt_sha256.clone(),
                knee_rate_per_second,
                offered_rate_per_second,
                rate_fraction_millionths,
            },
            predecessor_node_count,
            w4_scenario_sha256: digest_json(scenario),
            source_commit: archived.source_commit.clone(),
            runner_fingerprint_sha256: archived.runner_fingerprint_sha256.clone(),
            prebuild_manifest_sha256: archived.prebuild_manifest_sha256.clone(),
            prebuild_contract_sha256: archived.prebuild_contract_sha256.clone(),
            server_binary_sha256: archived.server_binary.sha256.clone(),
            archived_capability_receipt_sha256: archived.receipt.receipt_sha256.clone(),
            archived_lifecycle_receipt_sha256: report.lifecycle.receipt_sha256.clone(),
            archived_pids: archived.nodes.iter().map(|node| node.pid).collect(),
            leader_node_id: leader.target_node_id.clone(),
            leader_term: leader.start.admin_status.term,
            leader_epoch: leader.start.admin_status.epoch,
        };
        result.validate(rate_fraction_millionths, predecessor_node_count)?;
        Ok(result)
    }

    pub fn summary(&self) -> &PredecessorSummary {
        &self.summary
    }

    pub fn predecessor_node_count(&self) -> u8 {
        self.predecessor_node_count
    }

    fn validate(&self, fraction: u32, predecessor_node_count: u8) -> Result<(), BrownoutError> {
        self.summary.validate(&SubKneeLoadContract {
            predecessor_evidence_class: CONTROL_PLANE_PREDECESSOR.to_owned(),
            requires_valid_predecessor: true,
            fixed_rate_fraction_millionths: fraction,
            observation_window_millis: OBSERVATION_WINDOW_MILLIS,
        })?;
        if self.predecessor_node_count != 3
            || self.predecessor_node_count != predecessor_node_count
            || self.archived_pids.len() != usize::from(self.predecessor_node_count)
            || !is_sha256(&self.w4_scenario_sha256)
            || !is_git_commit(&self.source_commit)
            || !is_sha256(&self.runner_fingerprint_sha256)
            || !is_sha256(&self.prebuild_manifest_sha256)
            || !is_sha256(&self.prebuild_contract_sha256)
            || !is_sha256(&self.server_binary_sha256)
            || !is_sha256(&self.archived_capability_receipt_sha256)
            || !is_sha256(&self.archived_lifecycle_receipt_sha256)
            || self.archived_capability_receipt_sha256 != self.summary.reference_receipt_sha256
            || self.archived_pids.is_empty()
            || self.archived_pids.contains(&0)
            || !portable_identifier(&self.leader_node_id)
            || self.leader_term == 0
            || self.leader_epoch == 0
        {
            return Err(BrownoutError::Capability(
                "W5A predecessor lost archived W4A scenario/source/build/lifecycle identity"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlaneExecutionReceipt {
    w4_scenario_sha256: String,
    capability_receipt_sha256: String,
    source_commit: String,
    runner_fingerprint_sha256: String,
    prebuild_manifest_sha256: String,
    prebuild_contract_sha256: String,
    server_binary_sha256: String,
    node_pids: Vec<(String, u32)>,
    receipt_sha256: String,
}

impl ControlPlaneExecutionReceipt {
    fn computed_receipt(&self) -> String {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        digest_json(&payload)
    }
}

/// Fresh W5A processes.  This wrapper is intentionally distinct from the
/// archived predecessor and proves a new run receipt/PID set over the same
/// stable W4 scenario/source/runner/prebuild/server identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlPlaneExecutionCapability {
    live: w4a::ProbedControlPlaneCapability,
    receipt: ControlPlaneExecutionReceipt,
}

impl ControlPlaneExecutionCapability {
    pub fn from_fresh_w4a(
        scenario: &w4a::ControlPlaneScenario,
        live: w4a::ProbedControlPlaneCapability,
    ) -> Result<Self, BrownoutError> {
        let revalidated = match live
            .attestation
            .receipt
            .clone()
            .require(
                scenario,
                w4a::ReferenceCapabilityPolicy::MandatoryFailClosed,
            )
            .map_err(|error| {
                BrownoutError::Capability(format!("invalid fresh W5A capability: {error}"))
            })? {
            w4a::ControlPlaneCapabilityOutcome::Ready(capability) => *capability,
            w4a::ControlPlaneCapabilityOutcome::SkippedLoud(skip) => {
                return Err(BrownoutError::Capability(format!(
                    "fresh W5A capability unexpectedly skipped: {}",
                    skip.message
                )));
            }
        };
        if revalidated != live.attestation {
            return Err(BrownoutError::Capability(
                "fresh W5A capability differs from its sealed W4A launch attestation".to_owned(),
            ));
        }
        snapshot_consensus(&live.baseline)?;
        let mut node_pids = live
            .attestation
            .nodes
            .iter()
            .map(|node| (node.node_id.clone(), node.pid))
            .collect::<Vec<_>>();
        node_pids.sort();
        let mut receipt = ControlPlaneExecutionReceipt {
            w4_scenario_sha256: digest_json(scenario),
            capability_receipt_sha256: live.receipt_sha256().to_owned(),
            source_commit: live.attestation.source_commit.clone(),
            runner_fingerprint_sha256: live.attestation.runner_fingerprint_sha256.clone(),
            prebuild_manifest_sha256: live.attestation.prebuild_manifest_sha256.clone(),
            prebuild_contract_sha256: live.attestation.prebuild_contract_sha256.clone(),
            server_binary_sha256: live.attestation.server_binary.sha256.clone(),
            node_pids,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt.computed_receipt();
        let capability = Self { live, receipt };
        capability.validate()?;
        Ok(capability)
    }

    pub fn live(&self) -> &w4a::ProbedControlPlaneCapability {
        &self.live
    }

    fn validate(&self) -> Result<(), BrownoutError> {
        if !is_sha256(&self.receipt.w4_scenario_sha256)
            || !is_sha256(&self.receipt.capability_receipt_sha256)
            || !is_git_commit(&self.receipt.source_commit)
            || !is_sha256(&self.receipt.runner_fingerprint_sha256)
            || !is_sha256(&self.receipt.prebuild_manifest_sha256)
            || !is_sha256(&self.receipt.prebuild_contract_sha256)
            || !is_sha256(&self.receipt.server_binary_sha256)
            || self.receipt.node_pids.is_empty()
            || self.receipt.node_pids.iter().any(|(_, pid)| *pid == 0)
            || self.receipt.capability_receipt_sha256 != self.live.receipt_sha256()
            || self.receipt.receipt_sha256 != self.receipt.computed_receipt()
        {
            return Err(BrownoutError::Capability(
                "fresh W5A execution receipt is incomplete or unsealed".to_owned(),
            ));
        }
        Ok(())
    }
}

/// A W3 RESP capacity artifact revalidated from exact JSON bytes.  The selected
/// process capability and knee are extracted from the report, not restated by
/// the W5 caller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespSelectedCapacityContract {
    pub measurement_id: String,
    pub scenario_digest: String,
    pub workload: WorkloadIdentity,
    pub workload_contract_sha256: String,
    pub connections: u64,
    pub pipeline_depth: u64,
    pub preload_operations: u64,
    pub warmup_operations: u64,
    pub steady_operations: u64,
    pub repeats: u64,
    pub key_count: u64,
    pub multi_key_width: u64,
    pub reset_batch_entries: u64,
}

impl RespSelectedCapacityContract {
    fn from_curve(
        curve: &LoadCurveEvidence,
        expected_workload: &str,
    ) -> Result<Self, BrownoutError> {
        if curve.dimensions.get("workload")
            != Some(&DimensionValue::Text(expected_workload.to_owned()))
        {
            return Err(BrownoutError::Capability(format!(
                "W3 capacity curve {} has the wrong workload label",
                curve.id
            )));
        }
        let contract = Self {
            measurement_id: curve.id.clone(),
            scenario_digest: curve.scenario_digest.clone(),
            workload: curve.workload.clone(),
            workload_contract_sha256: digest_json(&curve.workload),
            connections: curve_dimension_u64(curve, "connections")?,
            pipeline_depth: curve_dimension_u64(curve, "pipeline_depth")?,
            preload_operations: curve_dimension_u64(curve, "preload_operations")?,
            warmup_operations: curve_dimension_u64(curve, "warmup_operations")?,
            steady_operations: curve_dimension_u64(curve, "steady_operations")?,
            repeats: curve_dimension_u64(curve, "repeats")?,
            key_count: curve_dimension_u64(curve, "key_count")?,
            // These are fixed by the validated W3 0.67 A/B/C scenario and
            // target adapter; the effective scenario digest binds both.
            multi_key_width: 10,
            reset_batch_entries: 128,
        };
        contract.validate(expected_workload)?;
        Ok(contract)
    }

    fn validate(&self, expected_workload: &str) -> Result<(), BrownoutError> {
        let workload_dimension = self
            .measurement_id
            .strip_prefix("resp_open_loop_get_set_knee_at_slo_workload_")
            .map(str::to_ascii_uppercase);
        let weight_sum = self
            .workload
            .operation_mix
            .iter()
            .map(|operation| operation.weight)
            .sum::<f64>();
        if workload_dimension.as_deref() != Some(expected_workload)
            || !is_sha256(&self.scenario_digest)
            || self.workload.generator != "hydracache-cache-sim-key-schedule"
            || self.workload.seed.is_none_or(|seed| seed == 0)
            || self.workload.key_count != Some(self.key_count)
            || self.workload.operation_mix.is_empty()
            || self.workload.payload_mix.len() != 1
            || self.workload.payload_mix[0].bytes == 0
            || self.workload.payload_mix[0].weight != 1.0
            || !is_sha256(&self.workload.digest)
            || self.workload_contract_sha256 != digest_json(&self.workload)
            || !weight_sum.is_finite()
            || (weight_sum - 1.0).abs() > f64::EPSILON
            || self.connections == 0
            || self.pipeline_depth == 0
            || self.preload_operations == 0
            || self.warmup_operations == 0
            || self.steady_operations == 0
            || self.repeats == 0
            || self.key_count == 0
            || self.multi_key_width != 10
            || self.reset_batch_entries != 128
        {
            return Err(BrownoutError::Capability(
                "selected W3 RESP capacity curve lost its exact workload/target contract"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

fn resp_capacity_workload(measurement_id: &str) -> Result<&'static str, BrownoutError> {
    RESP_CAPACITY_MEASUREMENTS
        .iter()
        .find_map(|(id, workload)| (*id == measurement_id).then_some(*workload))
        .ok_or_else(|| {
            BrownoutError::Capability("W5B selected an unknown W3 capacity measurement".to_owned())
        })
}

fn select_w3_resp_capacity(
    report: &PerfReport,
) -> Result<(RespSelectedCapacityContract, u64, String), BrownoutError> {
    let mut candidates = Vec::with_capacity(RESP_CAPACITY_MEASUREMENTS.len());
    let mut matrix = Vec::with_capacity(RESP_CAPACITY_MEASUREMENTS.len());
    for (measurement_id, workload) in RESP_CAPACITY_MEASUREMENTS {
        let matching = report
            .measurements
            .iter()
            .filter_map(|measurement| match measurement {
                MeasurementEvidence::LoadCurve(curve) if curve.id == measurement_id => Some(curve),
                _ => None,
            })
            .collect::<Vec<_>>();
        if matching.len() != 1 || matching[0].claim != LoadClaim::CapacityKnee {
            return Err(BrownoutError::Capability(format!(
                "W3 predecessor requires exactly one capacity curve {measurement_id}"
            )));
        }
        let curve = matching[0];
        let knee_rate_per_second = exact_u64_rate(
            curve
                .knee
                .as_ref()
                .and_then(|knee| knee.sustainable_rate_per_second)
                .ok_or_else(|| {
                    BrownoutError::Capability(format!(
                        "W3 capacity curve {measurement_id} has no sustainable knee"
                    ))
                })?,
            measurement_id,
        )?;
        let contract = RespSelectedCapacityContract::from_curve(curve, workload)?;
        matrix.push(RespCapacityMatrixBinding {
            measurement_id: contract.measurement_id.clone(),
            scenario_digest: contract.scenario_digest.clone(),
            workload_contract_sha256: contract.workload_contract_sha256.clone(),
            sustainable_rate_per_second: knee_rate_per_second,
        });
        candidates.push((knee_rate_per_second, contract));
    }
    matrix.sort_by(|left, right| left.measurement_id.cmp(&right.measurement_id));
    candidates.sort_by(|(left_rate, left), (right_rate, right)| {
        left_rate
            .cmp(right_rate)
            .then_with(|| left.measurement_id.cmp(&right.measurement_id))
    });
    let (knee_rate_per_second, selected) = candidates.into_iter().next().ok_or_else(|| {
        BrownoutError::Capability("W3 capacity matrix is unexpectedly empty".to_owned())
    })?;
    Ok((selected, knee_rate_per_second, digest_json(&matrix)))
}

fn curve_dimension_u64(curve: &LoadCurveEvidence, dimension: &str) -> Result<u64, BrownoutError> {
    match curve.dimensions.get(dimension) {
        Some(DimensionValue::U64(value)) => Ok(*value),
        _ => Err(BrownoutError::Capability(format!(
            "W3 capacity curve {} lacks typed u64 dimension {dimension}",
            curve.id
        ))),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RespCapacityMatrixBinding {
    measurement_id: String,
    scenario_digest: String,
    workload_contract_sha256: String,
    sustainable_rate_per_second: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespCapacityPredecessor {
    summary: PredecessorSummary,
    capability: RespEndpointCapability,
    selected_capacity: RespSelectedCapacityContract,
    capacity_matrix_sha256: String,
    lifecycle_artifact_sha256: String,
    old_pid: u32,
    scenario_digest: String,
    workload_digest: String,
    surface_digest: String,
    source_digest: String,
    build_digest: String,
    runner_fingerprint_sha256: String,
    config_semantics_sha256: String,
    source_commit: String,
}

impl RespCapacityPredecessor {
    /// Consume the persisted W3 capacity report and its separately persisted
    /// lifecycle artifact.  The predecessor PID must already be killed/waited;
    /// W5B always launches a different fresh PID.
    pub fn from_w3_reference(
        report: &PerfReport,
        artifact_json: &[u8],
        lifecycle: &RespDaemonEvidence,
        lifecycle_json: &[u8],
        rate_fraction_millionths: u32,
    ) -> Result<Self, BrownoutError> {
        validate_perf_reference(report, artifact_json)?;
        let capability = report.resp_endpoint_capability.clone().ok_or_else(|| {
            BrownoutError::Capability("W3 artifact has no RESP process capability".to_owned())
        })?;
        let (selected_capacity, knee_rate_per_second, capacity_matrix_sha256) =
            select_w3_resp_capacity(report)?;
        let reference_receipt_sha256 = digest_json(&capability);
        validate_archived_resp_lifecycle(&capability, lifecycle, lifecycle_json)?;
        let result = Self {
            summary: PredecessorSummary {
                evidence_class: RESP_PREDECESSOR.to_owned(),
                artifact_sha256: sha256_hex(artifact_json),
                reference_receipt_sha256,
                knee_rate_per_second,
                offered_rate_per_second: sub_knee_rate(
                    knee_rate_per_second,
                    rate_fraction_millionths,
                )?,
                rate_fraction_millionths,
            },
            selected_capacity,
            capacity_matrix_sha256,
            lifecycle_artifact_sha256: sha256_hex(lifecycle_json),
            old_pid: capability.pid,
            scenario_digest: report.scenario_digest.clone(),
            workload_digest: report.workload_digest.clone(),
            surface_digest: digest_json(&report.surface),
            source_digest: digest_json(&report.source),
            build_digest: digest_json(&report.build),
            runner_fingerprint_sha256: digest_json(&report.observed_runner),
            config_semantics_sha256: resp_config_semantics_sha256(&capability),
            capability,
            source_commit: report.source.git_commit.clone(),
        };
        result.validate(rate_fraction_millionths)?;
        Ok(result)
    }

    pub fn summary(&self) -> &PredecessorSummary {
        &self.summary
    }

    pub fn selected_endpoint(&self) -> &str {
        &self.capability.selected_endpoint
    }

    pub fn pid(&self) -> u32 {
        self.capability.pid
    }

    pub fn selected_capacity(&self) -> &RespSelectedCapacityContract {
        &self.selected_capacity
    }

    fn validate(&self, fraction: u32) -> Result<(), BrownoutError> {
        self.summary.validate(&SubKneeLoadContract {
            predecessor_evidence_class: RESP_PREDECESSOR.to_owned(),
            requires_valid_predecessor: true,
            fixed_rate_fraction_millionths: fraction,
            observation_window_millis: OBSERVATION_WINDOW_MILLIS,
        })?;
        if self.capability.schema_version != 1
            || self.capability.pid == 0
            || !self.capability.direct_prebuilt_exec
            || !self.capability.fresh_data_dir
            || self.capability.selected_endpoint
                != format!("hydracache-server@{}", self.capability.config.redis_addr)
            || !is_sha256(&self.capability.server_binary_sha256)
            || !is_sha256(&self.capability.loadgen_binary_sha256)
            || !is_sha256(&self.capability.prebuild_manifest_sha256)
            || !is_sha256(&self.capability.prebuild_contract_digest)
            || !is_sha256(&self.capacity_matrix_sha256)
            || !is_sha256(&self.lifecycle_artifact_sha256)
            || self.old_pid != self.capability.pid
            || !is_sha256(&self.scenario_digest)
            || !is_sha256(&self.workload_digest)
            || !is_sha256(&self.surface_digest)
            || !is_sha256(&self.source_digest)
            || !is_sha256(&self.build_digest)
            || !is_git_commit(&self.source_commit)
            || !is_sha256(&self.runner_fingerprint_sha256)
            || !is_sha256(&self.config_semantics_sha256)
            || self.capability.source_commit != self.source_commit
        {
            return Err(BrownoutError::Capability(
                "W5B predecessor lost archived W3 scenario/workload/source/build/lifecycle identity"
                    .to_owned(),
            ));
        }
        let expected_workload = resp_capacity_workload(&self.selected_capacity.measurement_id)?;
        self.selected_capacity.validate(expected_workload)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespExecutionReceipt {
    scenario_digest: String,
    workload_digest: String,
    selected_capacity_sha256: String,
    capacity_matrix_sha256: String,
    surface_digest: String,
    source_digest: String,
    build_digest: String,
    runner_fingerprint_sha256: String,
    endpoint_capability_sha256: String,
    process_image_receipt_sha256: String,
    pid: u32,
    selected_endpoint: String,
    receipt_sha256: String,
}

impl RespExecutionReceipt {
    fn computed_receipt(&self) -> String {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        digest_json(&payload)
    }
}

/// Fresh W5B daemon capability constructed from a freshly validated W3 build
/// context.  Stable identities match the archived capacity artifact; PID,
/// endpoint receipt, data directory, and run receipt are necessarily new.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespExecutionCapability {
    capability: RespEndpointCapability,
    process: ObservedProcessImage,
    receipt: RespExecutionReceipt,
}

impl RespExecutionCapability {
    pub fn from_fresh_w3_launch(
        predecessor: &RespCapacityPredecessor,
        context: &ValidatedRespReferenceContext,
        capability: RespEndpointCapability,
        process: ObservedProcessImage,
    ) -> Result<Self, BrownoutError> {
        predecessor.validate(predecessor.summary.rate_fraction_millionths)?;
        context.verify_binaries_unchanged().map_err(|error| {
            BrownoutError::Capability(format!("fresh W5B prebuilt binaries changed: {error}"))
        })?;
        process.validate()?;
        if digest_json(&context.surface) != predecessor.surface_digest
            || digest_json(&context.source) != predecessor.source_digest
            || digest_json(&context.build) != predecessor.build_digest
            || digest_json(&context.runner) != predecessor.runner_fingerprint_sha256
            || context.source.git_commit != predecessor.source_commit
            || context.manifest_sha256 != predecessor.capability.prebuild_manifest_sha256
            || context.build.prebuild_contract_digest
                != predecessor.capability.prebuild_contract_digest
            || context.server.sha256 != predecessor.capability.server_binary_sha256
            || context.loadgen.sha256 != predecessor.capability.loadgen_binary_sha256
            || capability.pid == predecessor.old_pid
            || capability.pid != process.pid
            || capability.source_commit != predecessor.source_commit
            || capability.prebuild_manifest_sha256
                != predecessor.capability.prebuild_manifest_sha256
            || capability.prebuild_contract_digest
                != predecessor.capability.prebuild_contract_digest
            || capability.server_binary_sha256 != predecessor.capability.server_binary_sha256
            || capability.loadgen_binary_sha256 != predecessor.capability.loadgen_binary_sha256
            || resp_config_semantics_sha256(&capability) != predecessor.config_semantics_sha256
            || capability.server_binary_sha256 != process.binary_sha256
            || capability.selected_endpoint
                != format!("hydracache-server@{}", capability.config.redis_addr)
        {
            return Err(BrownoutError::Capability(
                "fresh W5B launch does not match archived W3 stable identities or reused its PID"
                    .to_owned(),
            ));
        }
        let mut receipt = RespExecutionReceipt {
            scenario_digest: predecessor.scenario_digest.clone(),
            workload_digest: predecessor.workload_digest.clone(),
            selected_capacity_sha256: digest_json(&predecessor.selected_capacity),
            capacity_matrix_sha256: predecessor.capacity_matrix_sha256.clone(),
            surface_digest: predecessor.surface_digest.clone(),
            source_digest: predecessor.source_digest.clone(),
            build_digest: predecessor.build_digest.clone(),
            runner_fingerprint_sha256: predecessor.runner_fingerprint_sha256.clone(),
            endpoint_capability_sha256: digest_json(&capability),
            process_image_receipt_sha256: process.receipt_sha256.clone(),
            pid: capability.pid,
            selected_endpoint: capability.selected_endpoint.clone(),
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt.computed_receipt();
        let execution = Self {
            capability,
            process,
            receipt,
        };
        execution.validate()?;
        Ok(execution)
    }

    pub fn endpoint(&self) -> &str {
        &self.capability.selected_endpoint
    }

    pub fn pid(&self) -> u32 {
        self.capability.pid
    }

    pub fn endpoint_capability(&self) -> &RespEndpointCapability {
        &self.capability
    }

    pub fn process_image(&self) -> &ObservedProcessImage {
        &self.process
    }

    pub fn socket_endpoint(&self) -> std::net::SocketAddr {
        self.capability.config.redis_addr
    }

    fn validate(&self) -> Result<(), BrownoutError> {
        self.process.validate()?;
        if self.capability.pid != self.process.pid
            || self.receipt.pid != self.capability.pid
            || self.receipt.selected_endpoint != self.capability.selected_endpoint
            || !is_sha256(&self.receipt.selected_capacity_sha256)
            || !is_sha256(&self.receipt.capacity_matrix_sha256)
            || self.receipt.endpoint_capability_sha256 != digest_json(&self.capability)
            || self.receipt.process_image_receipt_sha256 != self.process.receipt_sha256
            || self.receipt.receipt_sha256 != self.receipt.computed_receipt()
        {
            return Err(BrownoutError::Capability(
                "fresh W5B execution capability is internally inconsistent or unsealed".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Exact W4B report bytes and their validated reference attestation.  W5C does
/// not invent a capacity knee for a library/model surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelPredecessor {
    artifact_sha256: String,
    scenario_sha256: String,
    reference_receipt_sha256: String,
    profile: String,
    source_commit: String,
    runner_fingerprint: String,
    prebuild_manifest_sha256: String,
    source: w4b::GridModelSourceAttestation,
    runner: w4b::GridModelRunnerAttestation,
    prebuild: w4b::GridModelPrebuildAttestation,
}

impl GridModelPredecessor {
    pub fn from_w4b_reference(
        scenario: &w4b::GridModelScenario,
        report: &w4b::GridModelReport,
        artifact_json: &[u8],
    ) -> Result<Self, BrownoutError> {
        scenario
            .validate_exact_reference_shape()
            .map_err(|error| BrownoutError::Capability(format!("invalid W4B scenario: {error}")))?;
        report
            .validate(scenario)
            .map_err(|error| BrownoutError::Capability(format!("invalid W4B report: {error}")))?;
        if report.run_mode != w4b::GridModelRunMode::Reference {
            return Err(BrownoutError::Boundary(
                "W5C cannot consume W4B smoke provenance".to_owned(),
            ));
        }
        let parsed: w4b::GridModelReport =
            serde_json::from_slice(artifact_json).map_err(|error| {
                BrownoutError::Capability(format!("W4B artifact is not typed JSON: {error}"))
            })?;
        parsed.validate(scenario).map_err(|error| {
            BrownoutError::Capability(format!("serialized W4B artifact is invalid: {error}"))
        })?;
        if digest_json(&parsed) != digest_json(report) {
            return Err(BrownoutError::Capability(
                "W5C predecessor object differs from the exact artifact bytes".to_owned(),
            ));
        }
        let attestation = report.reference_capability.as_ref().ok_or_else(|| {
            BrownoutError::Capability("W4B reference has no attestation".to_owned())
        })?;
        let validated = attestation.validate(scenario).map_err(|error| {
            BrownoutError::Capability(format!("W4B attestation is invalid: {error}"))
        })?;
        let predecessor = Self {
            artifact_sha256: sha256_hex(artifact_json),
            scenario_sha256: report.scenario_sha256.clone(),
            reference_receipt_sha256: validated.receipt_sha256,
            profile: validated.profile,
            source_commit: validated.source_commit,
            runner_fingerprint: validated.runner_fingerprint,
            prebuild_manifest_sha256: validated.prebuild_manifest_sha256,
            source: attestation.source.clone(),
            runner: attestation.runner.clone(),
            prebuild: attestation.prebuild.clone(),
        };
        predecessor.validate()?;
        Ok(predecessor)
    }

    fn validate(&self) -> Result<(), BrownoutError> {
        if !is_sha256(&self.artifact_sha256)
            || !is_sha256(&self.scenario_sha256)
            || !is_sha256(&self.reference_receipt_sha256)
            || self.profile != REFERENCE_PROFILE
            || !is_git_commit(&self.source_commit)
            || !is_sha256(&self.runner_fingerprint)
            || !is_sha256(&self.prebuild_manifest_sha256)
            || self.source.git_commit != self.source_commit
            || !is_sha256(&self.source.cargo_lock_sha256)
            || !self.source.git_clean
            || !self.source.cargo_lock_verified_from_disk
            || !self.source.verified_before_measurement
            || self.runner.observed_w7_fingerprint != self.runner_fingerprint
            || self.runner.receipt_sha256 != self.runner.computed_receipt_sha256()
            || self.prebuild.source != self.source
            || self.prebuild.runner_fingerprint != self.runner_fingerprint
            || self.prebuild.prebuild_manifest_sha256 != self.prebuild_manifest_sha256
            || self.prebuild.build_contract_digest != self.prebuild.build.computed_digest()
            || self.prebuild.build_contract_digest != self.prebuild.build.digest
            || self.prebuild.receipt_sha256 != self.prebuild.computed_receipt_sha256()
            || !self.prebuild.files_verified_from_disk
            || !self.prebuild.verified_before_measurement
        {
            return Err(BrownoutError::Capability(
                "W5C predecessor lost exact W4B artifact/source/runner/prebuild identity"
                    .to_owned(),
            ));
        }
        let binary_ids = self
            .prebuild
            .binaries
            .iter()
            .map(|binary| binary.id.as_str())
            .collect::<BTreeSet<_>>();
        if self.prebuild.binaries.len() != 2
            || binary_ids != BTreeSet::from([LOADGEN_BINARY_ID, SERVER_BINARY_ID])
        {
            return Err(BrownoutError::Capability(
                "W5C predecessor lost the exact W4B loadgen/server prebuild set".to_owned(),
            ));
        }
        for binary in &self.prebuild.binaries {
            let canonical = fs::canonicalize(&binary.canonical_path).map_err(|error| {
                BrownoutError::Capability(format!(
                    "cannot canonicalize W5C predecessor binary {}: {error}",
                    binary.canonical_path.display()
                ))
            })?;
            if canonical != binary.canonical_path
                || !is_sha256(&binary.sha256)
                || hash_file(&canonical)? != binary.sha256
            {
                return Err(BrownoutError::Capability(
                    "W5C predecessor binary no longer matches its disk-verified W4B identity"
                        .to_owned(),
                ));
            }
        }
        Ok(())
    }

    fn binary(&self, id: &str) -> Result<&w4b::GridModelVerifiedBinary, BrownoutError> {
        self.prebuild
            .binaries
            .iter()
            .find(|binary| binary.id == id)
            .ok_or_else(|| {
                BrownoutError::Capability(format!(
                    "W5C W4B predecessor has no prebuilt {id} identity"
                ))
            })
    }

    pub fn fresh_execution_receipt(
        &self,
        context: &ValidatedRespReferenceContext,
        sequence: u64,
        started_unix_nanos: u64,
    ) -> Result<GridModelExecutionReceipt, BrownoutError> {
        self.validate()?;
        context.verify_binaries_unchanged().map_err(|error| {
            BrownoutError::Capability(format!(
                "fresh W5C prebuilt binaries changed before measurement: {error}"
            ))
        })?;
        let fresh_source = w4b::GridModelSourceAttestation::from_verified_w7(
            context.source.git_commit.clone(),
            context.source.cargo_lock_sha256.clone(),
        )
        .map_err(|error| {
            BrownoutError::Capability(format!("invalid fresh W5C source identity: {error}"))
        })?;
        let fresh_runner = w4b::GridModelRunnerAttestation::from_observed_w7(&context.runner)
            .map_err(|error| {
                BrownoutError::Capability(format!("invalid fresh W5C runner identity: {error}"))
            })?;
        let predecessor_loadgen = self.binary(LOADGEN_BINARY_ID)?;
        let predecessor_server = self.binary(SERVER_BINARY_ID)?;
        let context_binary_hashes = context
            .build
            .binary_sha256
            .iter()
            .cloned()
            .collect::<BTreeMap<_, _>>();
        if context.profile.name != self.profile
            || fresh_source != self.source
            || fresh_runner != self.runner
            || context.manifest_path != self.prebuild.prebuild_manifest_path
            || context.manifest_sha256 != self.prebuild_manifest_sha256
            || context.build.prebuild_manifest_sha256 != self.prebuild_manifest_sha256
            || context.build.prebuild_contract_digest != self.prebuild.build_contract_digest
            || context.source.toolchain != self.prebuild.build.toolchain_identity
            || context.source.build_flags != self.prebuild.build.flags
            || context.loadgen.canonical_path != predecessor_loadgen.canonical_path
            || context.loadgen.sha256 != predecessor_loadgen.sha256
            || context.server.canonical_path != predecessor_server.canonical_path
            || context.server.sha256 != predecessor_server.sha256
            || context_binary_hashes.get(LOADGEN_BINARY_ID) != Some(&context.loadgen.sha256)
            || context_binary_hashes.get(SERVER_BINARY_ID) != Some(&context.server.sha256)
            || context_binary_hashes.len() != 2
        {
            return Err(BrownoutError::Capability(
                "fresh W5C context differs from the exact W4B source/runner/prebuild identity"
                    .to_owned(),
            ));
        }
        GridModelExecutionReceipt::from_fresh_context(self, context, sequence, started_unix_nanos)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedProcessImage {
    node_id: String,
    pid: u32,
    binary_path: PathBuf,
    binary_sha256: String,
    config_path: PathBuf,
    config_sha256: String,
    receipt_sha256: String,
}

impl ObservedProcessImage {
    /// Observe the executable/config bytes rather than accepting caller-provided
    /// hashes.  This is the handoff used by a concrete process launcher.
    pub fn from_observed_paths(
        node_id: impl Into<String>,
        pid: u32,
        binary_path: &Path,
        config_path: &Path,
    ) -> Result<Self, BrownoutError> {
        let node_id = node_id.into();
        let binary_path = fs::canonicalize(binary_path).map_err(|error| {
            BrownoutError::Capability(format!(
                "cannot canonicalize observed binary {}: {error}",
                binary_path.display()
            ))
        })?;
        let config_path = fs::canonicalize(config_path).map_err(|error| {
            BrownoutError::Capability(format!(
                "cannot canonicalize observed config {}: {error}",
                config_path.display()
            ))
        })?;
        let mut receipt = Self {
            node_id,
            pid,
            binary_sha256: hash_file(&binary_path)?,
            config_sha256: hash_file(&config_path)?,
            binary_path,
            config_path,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt.computed_receipt();
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn from_w4a_receipt(node: &w4a::DaemonNodeProcessReceipt) -> Result<Self, BrownoutError> {
        let mut receipt = Self {
            node_id: node.node_id.clone(),
            pid: node.pid,
            binary_path: node.observed_executable_path.clone(),
            binary_sha256: node.observed_executable_sha256.clone(),
            config_path: node.config.canonical_path.clone(),
            config_sha256: node.config.sha256.clone(),
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt.computed_receipt();
        receipt.validate()?;
        Ok(receipt)
    }

    fn fixture(node_id: &str, pid: u32, discriminator: u8) -> Self {
        let mut receipt = Self {
            node_id: node_id.to_owned(),
            pid,
            binary_path: PathBuf::from(format!("fixture/server-{discriminator}")),
            binary_sha256: format!("{discriminator:x}").repeat(64),
            config_path: PathBuf::from(format!("fixture/node-{discriminator}.toml")),
            config_sha256: format!("{:x}", discriminator.saturating_add(1)).repeat(64),
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt.computed_receipt();
        receipt
    }

    fn computed_receipt(&self) -> String {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        digest_json(&payload)
    }

    fn validate(&self) -> Result<(), BrownoutError> {
        if !portable_identifier(&self.node_id)
            || self.pid == 0
            || self.binary_path.as_os_str().is_empty()
            || self.config_path.as_os_str().is_empty()
            || !is_sha256(&self.binary_sha256)
            || !is_sha256(&self.config_sha256)
            || self.receipt_sha256 != self.computed_receipt()
        {
            return Err(BrownoutError::Capability(
                "observed process image is incomplete or its receipt seal is broken".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaitedProcessTermination {
    pid: u32,
    exit_status: String,
    receipt_sha256: String,
}

impl WaitedProcessTermination {
    /// Construct only from an OS `wait` result.  There are no caller booleans
    /// such as `kill_requested` or `wait_completed` to promote.
    pub fn from_wait_status(pid: u32, status: ExitStatus) -> Result<Self, BrownoutError> {
        let mut receipt = Self {
            pid,
            exit_status: format!("{status:?}"),
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt.computed_receipt();
        receipt.validate()?;
        Ok(receipt)
    }

    fn fixture(pid: u32) -> Self {
        let mut receipt = Self {
            pid,
            exit_status: "fixture-waited-exit".to_owned(),
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt.computed_receipt();
        receipt
    }

    fn computed_receipt(&self) -> String {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        digest_json(&payload)
    }

    fn validate(&self) -> Result<(), BrownoutError> {
        if self.pid == 0
            || self.exit_status.is_empty()
            || self.receipt_sha256 != self.computed_receipt()
        {
            return Err(BrownoutError::Evidence(
                "process termination is not bound to one waited PID".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedEventTimeline {
    commit_latency_nanos: u64,
    recovery_latency_nanos: u64,
    receipt_sha256: String,
}

impl ObservedEventTimeline {
    pub fn from_instants(
        action_started: Instant,
        committed: Instant,
        recovered: Instant,
    ) -> Result<Self, BrownoutError> {
        let commit_latency_nanos = duration_nanos(
            committed
                .checked_duration_since(action_started)
                .ok_or_else(|| {
                    BrownoutError::Evidence("commit instant precedes event start".to_owned())
                })?,
        );
        let recovery_latency_nanos = duration_nanos(
            recovered
                .checked_duration_since(action_started)
                .ok_or_else(|| {
                    BrownoutError::Evidence("recovery instant precedes event start".to_owned())
                })?,
        );
        Self::from_nanos(commit_latency_nanos, recovery_latency_nanos)
    }

    fn from_nanos(
        commit_latency_nanos: u64,
        recovery_latency_nanos: u64,
    ) -> Result<Self, BrownoutError> {
        let mut receipt = Self {
            commit_latency_nanos,
            recovery_latency_nanos,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt.computed_receipt();
        receipt.validate()?;
        Ok(receipt)
    }

    fn computed_receipt(&self) -> String {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        digest_json(&payload)
    }

    fn validate(&self) -> Result<(), BrownoutError> {
        if self.commit_latency_nanos == 0
            || self.recovery_latency_nanos < self.commit_latency_nanos
            || self.receipt_sha256 != self.computed_receipt()
        {
            return Err(BrownoutError::Evidence(
                "event timeline is unordered, empty, or unsealed".to_owned(),
            ));
        }
        Ok(())
    }

    fn recovery_millis(&self) -> u64 {
        self.recovery_latency_nanos.saturating_add(999_999) / 1_000_000
    }

    fn commit_millis(&self) -> u64 {
        self.commit_latency_nanos.saturating_add(999_999) / 1_000_000
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", content = "receipt", rename_all = "snake_case")]
pub enum ControlPlaneActionReceipt {
    LeaderFailover {
        target: ObservedProcessImage,
        termination: WaitedProcessTermination,
    },
    MemberAdd {
        action: Box<w4a::DaemonAddInvocationReceipt>,
        added_process: ObservedProcessImage,
    },
    MemberDrain {
        action: w4a::AdminDrainInvocationReceipt,
        target: ObservedProcessImage,
        cleanup: WaitedProcessTermination,
    },
    NodeKillRejoin {
        original: ObservedProcessImage,
        termination: WaitedProcessTermination,
        restarted: ObservedProcessImage,
    },
}

impl ControlPlaneActionReceipt {
    pub fn leader_failover(
        target: &w4a::DaemonNodeProcessReceipt,
        termination: WaitedProcessTermination,
    ) -> Result<Self, BrownoutError> {
        let target = ObservedProcessImage::from_w4a_receipt(target)?;
        termination.validate()?;
        if target.pid != termination.pid {
            return Err(BrownoutError::Evidence(
                "leader-failover wait receipt targets a different PID".to_owned(),
            ));
        }
        Ok(Self::LeaderFailover {
            target,
            termination,
        })
    }

    pub fn member_add(action: w4a::DaemonAddInvocationReceipt) -> Result<Self, BrownoutError> {
        validate_add_action_artifact(&action, true)?;
        let added_process = ObservedProcessImage::from_observed_paths(
            action.target_process.node_id.clone(),
            action.target_process.pid,
            &action.target_process.observed_executable_path,
            &action.target_process.config.canonical_path,
        )?;
        if action.payload.target_node_id != added_process.node_id
            || action.target_process.observed_executable_sha256 != added_process.binary_sha256
            || action.target_process.config.sha256 != added_process.config_sha256
        {
            return Err(BrownoutError::Evidence(
                "member-add action/process target mismatch".to_owned(),
            ));
        }
        Ok(Self::MemberAdd {
            action: Box::new(action),
            added_process,
        })
    }

    pub fn member_drain(
        action: w4a::AdminDrainInvocationReceipt,
        target: &w4a::DaemonNodeProcessReceipt,
        cleanup: WaitedProcessTermination,
    ) -> Result<Self, BrownoutError> {
        let target = ObservedProcessImage::from_w4a_receipt(target)?;
        cleanup.validate()?;
        if action.target_node_id != target.node_id
            || cleanup.pid != target.pid
            || action.path != w4a::ADMIN_DRAIN_PATH
            || action.action != "drain"
            || action.outcome != "accepted"
            || action.remaining != 0
            || action.timed_out
            || !is_sha256(&action.response_sha256)
            || action.request_network_bytes == 0
            || action.response_network_bytes == 0
        {
            return Err(BrownoutError::Evidence(
                "member-drain action/process target mismatch".to_owned(),
            ));
        }
        Ok(Self::MemberDrain {
            action,
            target,
            cleanup,
        })
    }

    pub fn node_kill_rejoin(
        original: &w4a::DaemonNodeProcessReceipt,
        termination: WaitedProcessTermination,
        restarted: ObservedProcessImage,
    ) -> Result<Self, BrownoutError> {
        let original = ObservedProcessImage::from_w4a_receipt(original)?;
        termination.validate()?;
        restarted.validate()?;
        if original.pid != termination.pid
            || original.node_id != restarted.node_id
            || original.pid == restarted.pid
            || original.binary_sha256 != restarted.binary_sha256
            || original.config_sha256 != restarted.config_sha256
        {
            return Err(BrownoutError::Evidence(
                "kill/rejoin must wait the exact old PID and restart the same binary/config under a new PID"
                    .to_owned(),
            ));
        }
        Ok(Self::NodeKillRejoin {
            original,
            termination,
            restarted,
        })
    }

    fn action(&self) -> ControlPlaneBrownoutAction {
        match self {
            Self::LeaderFailover { .. } => ControlPlaneBrownoutAction::LeaderFailover,
            Self::MemberAdd { .. } => ControlPlaneBrownoutAction::MemberAdd,
            Self::MemberDrain { .. } => ControlPlaneBrownoutAction::MemberDrain,
            Self::NodeKillRejoin { .. } => ControlPlaneBrownoutAction::NodeKillRejoin,
        }
    }
}

fn validate_add_action_artifact(
    action: &w4a::DaemonAddInvocationReceipt,
    require_physical_artifact: bool,
) -> Result<(), BrownoutError> {
    if action.payload.receipt_kind != "hydracache-daemon-add-action-v1"
        || action.payload.provisioner != w4a::DAEMON_CLUSTER_PROVISIONER
        || action.payload.outcome != "process-started-and-admission-requested"
        || action.payload.target_node_id != action.target_process.node_id
        || !is_sha256(&action.action_receipt_sha256)
    {
        return Err(BrownoutError::Evidence(
            "W5A add action is not the exact process-harness receipt".to_owned(),
        ));
    }
    if require_physical_artifact {
        let path = fs::canonicalize(&action.canonical_action_receipt_path).map_err(|error| {
            BrownoutError::Evidence(format!(
                "cannot canonicalize W5A add action receipt {}: {error}",
                action.canonical_action_receipt_path.display()
            ))
        })?;
        let bytes = fs::read(&path).map_err(|error| {
            BrownoutError::Evidence(format!(
                "cannot read W5A add action receipt {}: {error}",
                path.display()
            ))
        })?;
        let decoded: w4a::DaemonAddActionPayload =
            serde_json::from_slice(&bytes).map_err(|error| {
                BrownoutError::Evidence(format!(
                    "W5A add action receipt is not typed JSON: {error}"
                ))
            })?;
        if path != action.canonical_action_receipt_path
            || sha256_hex(&bytes) != action.action_receipt_sha256
            || decoded != action.payload
        {
            return Err(BrownoutError::Evidence(
                "W5A add action bytes/path/hash differ from its typed receipt".to_owned(),
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawControlPlaneEvent {
    origin: ObservationOrigin,
    pub receipt: ControlPlaneActionReceipt,
    pub before: Vec<w4a::PublicControlPlaneSnapshot>,
    pub after: Vec<w4a::PublicControlPlaneSnapshot>,
    pub before_window: OpenLoopObservation,
    pub disruption_window: OpenLoopObservation,
    pub recovered_window: OpenLoopObservation,
    pub timeline: ObservedEventTimeline,
}

impl RawControlPlaneEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn from_observed(
        receipt: ControlPlaneActionReceipt,
        before: Vec<w4a::PublicControlPlaneSnapshot>,
        after: Vec<w4a::PublicControlPlaneSnapshot>,
        before_window: OpenLoopObservation,
        disruption_window: OpenLoopObservation,
        recovered_window: OpenLoopObservation,
        action_started: Instant,
        committed: Instant,
        recovered: Instant,
    ) -> Result<Self, BrownoutError> {
        Ok(Self {
            origin: ObservationOrigin::Observed,
            receipt,
            before,
            after,
            before_window,
            disruption_window,
            recovered_window,
            timeline: ObservedEventTimeline::from_instants(action_started, committed, recovered)?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlaneEventEvidence {
    pub action: ControlPlaneBrownoutAction,
    pub transition_recovery_millis: u64,
    pub raw: RawControlPlaneEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlaneReferenceProvenance {
    predecessor_artifact_sha256: String,
    predecessor_receipt_sha256: String,
    predecessor_node_count: u8,
    execution_capability_receipt_sha256: String,
    final_cleanup_receipt_sha256: String,
    scenario_sha256: String,
    receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlaneFinalCleanupReceipt {
    pub nodes: Vec<w4a::DaemonNodeLifecycleEvidence>,
    pub receipt_sha256: String,
}

impl ControlPlaneFinalCleanupReceipt {
    pub fn from_observed(
        mut nodes: Vec<w4a::DaemonNodeLifecycleEvidence>,
    ) -> Result<Self, BrownoutError> {
        nodes.sort_by(|left, right| left.node_id.cmp(&right.node_id));
        let mut receipt = Self {
            nodes,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt.computed_receipt();
        receipt.validate_physical()?;
        Ok(receipt)
    }

    fn computed_receipt(&self) -> String {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        digest_json(&payload)
    }

    fn validate_physical(&self) -> Result<(), BrownoutError> {
        if self.nodes.is_empty()
            || self.receipt_sha256 != self.computed_receipt()
            || self
                .nodes
                .windows(2)
                .any(|nodes| nodes[0].node_id >= nodes[1].node_id)
        {
            return Err(BrownoutError::Evidence(
                "W5A final cleanup receipt is empty, unsorted, duplicate, or unsealed".to_owned(),
            ));
        }
        let mut pids = BTreeSet::new();
        for node in &self.nodes {
            if !portable_identifier(&node.node_id)
                || node.pid == 0
                || !pids.insert(node.pid)
                || !node.kill_requested
                || !node.wait_completed
                || !node.process_no_longer_running
                || node.exit_status.is_empty()
                || !valid_process_log(&node.stdout_log)?
                || !valid_process_log(&node.stderr_log)?
                || fs::canonicalize(&node.server_binary_path_after).map_err(|error| {
                    BrownoutError::Evidence(format!(
                        "cannot canonicalize W5A cleanup binary {}: {error}",
                        node.server_binary_path_after.display()
                    ))
                })? != node.server_binary_path_after
                || hash_file(&node.server_binary_path_after)? != node.server_binary_sha256_after
                || fs::canonicalize(&node.node_config_path_after).map_err(|error| {
                    BrownoutError::Evidence(format!(
                        "cannot canonicalize W5A cleanup config {}: {error}",
                        node.node_config_path_after.display()
                    ))
                })? != node.node_config_path_after
                || hash_file(&node.node_config_path_after)? != node.node_config_sha256_after
            {
                return Err(BrownoutError::Evidence(
                    "W5A final cleanup lacks exact kill/wait/log/binary/config proof".to_owned(),
                ));
            }
        }
        Ok(())
    }
}

impl ControlPlaneReferenceProvenance {
    fn from_validated(
        scenario: &ControlPlaneBrownoutScenario,
        predecessor: &ControlPlanePredecessor,
        execution: &ControlPlaneExecutionCapability,
        cleanup: &ControlPlaneFinalCleanupReceipt,
    ) -> Self {
        let mut receipt = Self {
            predecessor_artifact_sha256: predecessor.summary.artifact_sha256.clone(),
            predecessor_receipt_sha256: predecessor.summary.reference_receipt_sha256.clone(),
            predecessor_node_count: predecessor.predecessor_node_count,
            execution_capability_receipt_sha256: execution.receipt.receipt_sha256.clone(),
            final_cleanup_receipt_sha256: cleanup.receipt_sha256.clone(),
            scenario_sha256: scenario.contract_sha256(),
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt.computed_receipt();
        receipt
    }

    fn computed_receipt(&self) -> String {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        digest_json(&payload)
    }

    fn validate_shape(
        &self,
        scenario: &ControlPlaneBrownoutScenario,
        predecessor: &PredecessorSummary,
        cleanup: &ControlPlaneFinalCleanupReceipt,
    ) -> Result<(), BrownoutError> {
        if self.predecessor_artifact_sha256 != predecessor.artifact_sha256
            || self.predecessor_receipt_sha256 != predecessor.reference_receipt_sha256
            || self.predecessor_node_count != scenario.reference.predecessor_node_count
            || self.predecessor_node_count != 3
            || !is_sha256(&self.execution_capability_receipt_sha256)
            || self.final_cleanup_receipt_sha256 != cleanup.receipt_sha256
            || self.scenario_sha256 != scenario.contract_sha256()
            || self.receipt_sha256 != self.computed_receipt()
        {
            return Err(BrownoutError::Capability(
                "W5A reference provenance lost predecessor topology, fresh execution, or cleanup identity"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlaneBrownoutReport {
    pub schema_version: u32,
    pub scenario_id: String,
    pub scenario_sha256: String,
    pub evidence_class: String,
    pub run_mode: BrownoutRunMode,
    pub predecessor: PredecessorSummary,
    pub predecessor_node_count: u8,
    pub reference_provenance: Option<ControlPlaneReferenceProvenance>,
    pub final_cleanup: Option<ControlPlaneFinalCleanupReceipt>,
    pub events: Vec<ControlPlaneEventEvidence>,
    pub generic_client_write_invariant: bool,
    pub distributed_value_invariant: bool,
    pub live_reshard_measured: bool,
    pub aggregate_goodput: bool,
}

impl ControlPlaneBrownoutReport {
    pub fn validate(&self, scenario: &ControlPlaneBrownoutScenario) -> Result<(), BrownoutError> {
        scenario.validate()?;
        self.predecessor.validate(&scenario.load)?;
        if self.schema_version != BROWNOUT_REPORT_VERSION
            || self.scenario_id != scenario.scenario_id
            || self.scenario_sha256 != scenario.contract_sha256()
            || self.evidence_class != CONTROL_PLANE_EVIDENCE_CLASS
            || self.predecessor_node_count != scenario.reference.predecessor_node_count
            || self.predecessor_node_count != 3
            || self.reference_provenance.as_ref().is_some_and(|receipt| {
                receipt.predecessor_node_count != self.predecessor_node_count
            })
            || self.generic_client_write_invariant
            || self.distributed_value_invariant
            || self.live_reshard_measured
            || self.aggregate_goodput
        {
            return Err(BrownoutError::Boundary(
                "W5A report crossed into client writes, values, reshard, or aggregate capacity"
                    .to_owned(),
            ));
        }
        match (
            self.run_mode,
            self.reference_provenance.as_ref(),
            self.final_cleanup.as_ref(),
        ) {
            (BrownoutRunMode::DeterministicSmoke, None, None) => {}
            (BrownoutRunMode::Reference, Some(receipt), Some(cleanup)) => {
                cleanup.validate_physical()?;
                receipt.validate_shape(scenario, &self.predecessor, cleanup)?;
            }
            (BrownoutRunMode::DeterministicSmoke, _, _) => {
                return Err(BrownoutError::Boundary(
                    "W5A smoke cannot carry reference provenance or real cleanup".to_owned(),
                ));
            }
            _ => {
                return Err(BrownoutError::Capability(
                    "W5A reference is missing sealed provenance/final-cleanup receipts".to_owned(),
                ));
            }
        }
        let expected = scenario
            .events
            .actions
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let actual = self
            .events
            .iter()
            .map(|event| event.action)
            .collect::<BTreeSet<_>>();
        if self.events.len() != expected.len() || actual != expected {
            return Err(BrownoutError::Evidence(
                "W5A must retain exactly one raw event for every committed action".to_owned(),
            ));
        }
        for event in &self.events {
            validate_control_plane_event(
                event,
                scenario,
                self.run_mode,
                self.predecessor.offered_rate_per_second,
            )?;
        }
        Ok(())
    }

    pub fn validate_reference(
        &self,
        scenario: &ControlPlaneBrownoutScenario,
        predecessor: &ControlPlanePredecessor,
        execution: &ControlPlaneExecutionCapability,
    ) -> Result<(), BrownoutError> {
        scenario.validate_exact_reference_shape()?;
        self.validate(scenario)?;
        predecessor.validate(
            scenario.load.fixed_rate_fraction_millionths,
            scenario.reference.predecessor_node_count,
        )?;
        validate_w5a_capability(predecessor, execution)?;
        validate_control_receipts_against_capability(self, execution.live())?;
        validate_control_final_cleanup(self, execution.live())?;
        if self.run_mode != BrownoutRunMode::Reference
            || self.predecessor != predecessor.summary
            || self.predecessor_node_count != predecessor.predecessor_node_count
            || self.reference_provenance
                != Some(ControlPlaneReferenceProvenance::from_validated(
                    scenario,
                    predecessor,
                    execution,
                    self.final_cleanup.as_ref().ok_or_else(|| {
                        BrownoutError::Capability(
                            "W5A reference cleanup disappeared during validation".to_owned(),
                        )
                    })?,
                ))
        {
            return Err(BrownoutError::Capability(
                "W5A artifact does not bind the exact W4A predecessor and fresh execution capability"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlPlaneBrownoutLoadPlan {
    pub offered_rate_per_second: u64,
    pub observation_window_millis: u64,
    pub predecessor_w4_scenario_sha256: String,
    pub predecessor_artifact_sha256: String,
}

#[async_trait]
pub trait ControlPlaneBrownoutDriver: Send + Sync {
    async fn observe_leader_failover(
        &self,
        plan: &ControlPlaneBrownoutLoadPlan,
    ) -> Result<RawControlPlaneEvent, BrownoutError>;

    async fn observe_member_add(
        &self,
        plan: &ControlPlaneBrownoutLoadPlan,
    ) -> Result<RawControlPlaneEvent, BrownoutError>;

    async fn observe_member_drain(
        &self,
        plan: &ControlPlaneBrownoutLoadPlan,
    ) -> Result<RawControlPlaneEvent, BrownoutError>;

    async fn observe_node_kill_rejoin(
        &self,
        plan: &ControlPlaneBrownoutLoadPlan,
    ) -> Result<RawControlPlaneEvent, BrownoutError>;

    /// Reap every process still live after the four observed actions and seal
    /// physical log/binary/config lifecycle evidence before report creation.
    async fn finalize_cleanup(&self) -> Result<ControlPlaneFinalCleanupReceipt, BrownoutError>;
}

pub async fn run_control_plane_reference<D: ControlPlaneBrownoutDriver>(
    scenario: &ControlPlaneBrownoutScenario,
    predecessor: ControlPlanePredecessor,
    execution: &ControlPlaneExecutionCapability,
    driver: &D,
) -> Result<ControlPlaneBrownoutReport, BrownoutError> {
    scenario.validate_exact_reference_shape()?;
    predecessor.validate(
        scenario.load.fixed_rate_fraction_millionths,
        scenario.reference.predecessor_node_count,
    )?;
    validate_w5a_capability(&predecessor, execution)?;
    if std::env::var(&scenario.reference.capability_env).as_deref() != Ok("1") {
        return Err(BrownoutError::Capability(format!(
            "{}=1 is required before W5A may control real processes",
            scenario.reference.capability_env
        )));
    }
    let plan = ControlPlaneBrownoutLoadPlan {
        offered_rate_per_second: predecessor.summary.offered_rate_per_second,
        observation_window_millis: scenario.load.observation_window_millis,
        predecessor_w4_scenario_sha256: predecessor.w4_scenario_sha256.clone(),
        predecessor_artifact_sha256: predecessor.summary.artifact_sha256.clone(),
    };
    let raw_events = vec![
        driver.observe_leader_failover(&plan).await?,
        driver.observe_member_add(&plan).await?,
        driver.observe_member_drain(&plan).await?,
        driver.observe_node_kill_rejoin(&plan).await?,
    ];
    let final_cleanup = driver.finalize_cleanup().await?;
    let events = raw_events
        .into_iter()
        .map(|raw| ControlPlaneEventEvidence {
            action: raw.receipt.action(),
            transition_recovery_millis: raw.timeline.recovery_millis(),
            raw,
        })
        .collect();
    let report = ControlPlaneBrownoutReport {
        schema_version: BROWNOUT_REPORT_VERSION,
        scenario_id: scenario.scenario_id.clone(),
        scenario_sha256: scenario.contract_sha256(),
        evidence_class: CONTROL_PLANE_EVIDENCE_CLASS.to_owned(),
        run_mode: BrownoutRunMode::Reference,
        predecessor: predecessor.summary.clone(),
        predecessor_node_count: predecessor.predecessor_node_count,
        reference_provenance: Some(ControlPlaneReferenceProvenance::from_validated(
            scenario,
            &predecessor,
            execution,
            &final_cleanup,
        )),
        final_cleanup: Some(final_cleanup),
        events,
        generic_client_write_invariant: false,
        distributed_value_invariant: false,
        live_reshard_measured: false,
        aggregate_goodput: false,
    };
    report.validate_reference(scenario, &predecessor, execution)?;
    Ok(report)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocketUnavailableReceipt {
    endpoint: String,
    error_kind: String,
    receipt_sha256: String,
}

impl SocketUnavailableReceipt {
    pub fn from_connect_error(
        endpoint: impl Into<String>,
        error: &io::Error,
    ) -> Result<Self, BrownoutError> {
        let mut receipt = Self {
            endpoint: endpoint.into(),
            error_kind: format!("{:?}", error.kind()),
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt.computed_receipt();
        receipt.validate()?;
        Ok(receipt)
    }

    fn fixture(endpoint: &str) -> Self {
        let mut receipt = Self {
            endpoint: endpoint.to_owned(),
            error_kind: "ConnectionRefused".to_owned(),
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt.computed_receipt();
        receipt
    }

    fn computed_receipt(&self) -> String {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        digest_json(&payload)
    }

    fn validate(&self) -> Result<(), BrownoutError> {
        if !valid_socket_endpoint(&self.endpoint)
            || self.error_kind.is_empty()
            || self.receipt_sha256 != self.computed_receipt()
        {
            return Err(BrownoutError::Evidence(
                "RESP socket-down receipt is missing the exact endpoint/error".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndependentRespRawWindow {
    pub endpoint: String,
    pub process: ObservedProcessImage,
    pub cleanup: WaitedProcessTermination,
    pub window: OpenLoopObservation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawRespEndpointEvent {
    origin: ObservationOrigin,
    pub selected_endpoint: String,
    pub original: ObservedProcessImage,
    pub termination: WaitedProcessTermination,
    pub restarted: ObservedProcessImage,
    pub restarted_cleanup: WaitedProcessTermination,
    pub socket_unavailable: SocketUnavailableReceipt,
    pub before_window: OpenLoopObservation,
    pub disruption_window: OpenLoopObservation,
    pub recovered_window: OpenLoopObservation,
    pub independent_controls: Vec<IndependentRespRawWindow>,
    pub timeline: ObservedEventTimeline,
}

impl RawRespEndpointEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn from_observed(
        selected_endpoint: impl Into<String>,
        original: ObservedProcessImage,
        termination: WaitedProcessTermination,
        restarted: ObservedProcessImage,
        restarted_cleanup: WaitedProcessTermination,
        socket_unavailable: SocketUnavailableReceipt,
        before_window: OpenLoopObservation,
        disruption_window: OpenLoopObservation,
        recovered_window: OpenLoopObservation,
        independent_controls: Vec<IndependentRespRawWindow>,
        kill_started: Instant,
        socket_down: Instant,
        recovered: Instant,
    ) -> Result<Self, BrownoutError> {
        original.validate()?;
        termination.validate()?;
        restarted.validate()?;
        restarted_cleanup.validate()?;
        socket_unavailable.validate()?;
        let selected_endpoint = selected_endpoint.into();
        if original.pid != termination.pid
            || original.node_id != restarted.node_id
            || original.pid == restarted.pid
            || restarted.pid != restarted_cleanup.pid
            || original.binary_sha256 != restarted.binary_sha256
            || original.config_sha256 != restarted.config_sha256
            || socket_unavailable.endpoint != selected_endpoint
        {
            return Err(BrownoutError::Evidence(
                "RESP lifecycle is not exact PID wait/restart with identical binary/config/socket"
                    .to_owned(),
            ));
        }
        Ok(Self {
            origin: ObservationOrigin::Observed,
            selected_endpoint,
            original,
            termination,
            restarted,
            restarted_cleanup,
            socket_unavailable,
            before_window,
            disruption_window,
            recovered_window,
            independent_controls,
            timeline: ObservedEventTimeline::from_instants(kill_started, socket_down, recovered)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespReferenceProvenance {
    selected_predecessor_artifact_sha256: String,
    selected_predecessor_lifecycle_sha256: String,
    archived_process_receipt_sha256: String,
    selected_capacity: RespSelectedCapacityContract,
    capacity_matrix_sha256: String,
    predecessor_scenario_digest: String,
    predecessor_workload_digest: String,
    fresh_selected_execution_receipt_sha256: String,
    fresh_control_execution_receipt_sha256: Vec<String>,
    scenario_sha256: String,
    receipt_sha256: String,
}

impl RespReferenceProvenance {
    fn from_validated(
        scenario: &RespBrownoutScenario,
        predecessor: &RespCapacityPredecessor,
        selected: &RespExecutionCapability,
        controls: &[RespExecutionCapability],
    ) -> Self {
        let mut control_digests = controls
            .iter()
            .map(|control| control.receipt.receipt_sha256.clone())
            .collect::<Vec<_>>();
        control_digests.sort();
        let mut receipt = Self {
            selected_predecessor_artifact_sha256: predecessor.summary.artifact_sha256.clone(),
            selected_predecessor_lifecycle_sha256: predecessor.lifecycle_artifact_sha256.clone(),
            archived_process_receipt_sha256: predecessor.summary.reference_receipt_sha256.clone(),
            selected_capacity: predecessor.selected_capacity.clone(),
            capacity_matrix_sha256: predecessor.capacity_matrix_sha256.clone(),
            predecessor_scenario_digest: predecessor.scenario_digest.clone(),
            predecessor_workload_digest: predecessor.workload_digest.clone(),
            fresh_selected_execution_receipt_sha256: selected.receipt.receipt_sha256.clone(),
            fresh_control_execution_receipt_sha256: control_digests,
            scenario_sha256: scenario.contract_sha256(),
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt.computed_receipt();
        receipt
    }

    fn computed_receipt(&self) -> String {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        digest_json(&payload)
    }

    fn validate_shape(
        &self,
        scenario: &RespBrownoutScenario,
        predecessor: &PredecessorSummary,
    ) -> Result<(), BrownoutError> {
        let expected_workload = resp_capacity_workload(&self.selected_capacity.measurement_id)?;
        self.selected_capacity.validate(expected_workload)?;
        if self.selected_predecessor_artifact_sha256 != predecessor.artifact_sha256
            || self.archived_process_receipt_sha256 != predecessor.reference_receipt_sha256
            || !is_sha256(&self.selected_predecessor_lifecycle_sha256)
            || !is_sha256(&self.capacity_matrix_sha256)
            || !is_sha256(&self.predecessor_scenario_digest)
            || !is_sha256(&self.predecessor_workload_digest)
            || !is_sha256(&self.fresh_selected_execution_receipt_sha256)
            || self.fresh_control_execution_receipt_sha256.len()
                != usize::from(scenario.event.independent_control_endpoints)
            || self
                .fresh_control_execution_receipt_sha256
                .iter()
                .any(|digest| !is_sha256(digest))
            || self
                .fresh_control_execution_receipt_sha256
                .windows(2)
                .any(|digests| digests[0] >= digests[1])
            || self.scenario_sha256 != scenario.contract_sha256()
            || self.receipt_sha256 != self.computed_receipt()
        {
            return Err(BrownoutError::Capability(
                "W5B reference provenance lost the exact selected W3 curve/matrix or execution identities"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespBrownoutReport {
    pub schema_version: u32,
    pub scenario_id: String,
    pub scenario_sha256: String,
    pub evidence_class: String,
    pub run_mode: BrownoutRunMode,
    pub predecessor: PredecessorSummary,
    pub reference_provenance: Option<RespReferenceProvenance>,
    pub selected_endpoint_recovery_millis: u64,
    pub event: RawRespEndpointEvent,
    pub node_local_state: bool,
    pub automatic_failover: bool,
    pub neighbor_visibility_claim: bool,
    pub value_survival_claim: bool,
    pub cross_node_failover_claim: bool,
    pub aggregate_goodput: bool,
}

impl RespBrownoutReport {
    pub fn validate(&self, scenario: &RespBrownoutScenario) -> Result<(), BrownoutError> {
        scenario.validate()?;
        self.predecessor.validate(&scenario.load)?;
        if self.schema_version != BROWNOUT_REPORT_VERSION
            || self.scenario_id != scenario.scenario_id
            || self.scenario_sha256 != scenario.contract_sha256()
            || self.evidence_class != RESP_EVIDENCE_CLASS
            || !self.node_local_state
            || self.automatic_failover
            || self.neighbor_visibility_claim
            || self.value_survival_claim
            || self.cross_node_failover_claim
            || self.aggregate_goodput
            || self.selected_endpoint_recovery_millis != self.event.timeline.recovery_millis()
            || self.selected_endpoint_recovery_millis > scenario.event.max_recovery_millis
        {
            return Err(BrownoutError::Boundary(
                "W5B must remain one node-local endpoint lifecycle with no failover/value-recovery claim"
                    .to_owned(),
            ));
        }
        match (self.run_mode, self.reference_provenance.as_ref()) {
            (BrownoutRunMode::DeterministicSmoke, None) => {}
            (BrownoutRunMode::Reference, Some(receipt)) => {
                receipt.validate_shape(scenario, &self.predecessor)?;
            }
            (BrownoutRunMode::DeterministicSmoke, Some(_)) => {
                return Err(BrownoutError::Boundary(
                    "W5B smoke cannot carry reference provenance".to_owned(),
                ));
            }
            _ => {
                return Err(BrownoutError::Capability(
                    "W5B reference is missing a sealed provenance receipt".to_owned(),
                ));
            }
        }
        validate_resp_raw_event(
            &self.event,
            scenario,
            self.run_mode,
            self.predecessor.offered_rate_per_second,
        )
    }

    pub fn validate_reference(
        &self,
        scenario: &RespBrownoutScenario,
        predecessor: &RespCapacityPredecessor,
        selected: &RespExecutionCapability,
        controls: &[RespExecutionCapability],
    ) -> Result<(), BrownoutError> {
        scenario.validate_exact_reference_shape()?;
        self.validate(scenario)?;
        predecessor.validate(scenario.load.fixed_rate_fraction_millionths)?;
        selected.validate()?;
        validate_resp_controls(scenario, predecessor, selected, controls, &self.event)?;
        if self.run_mode != BrownoutRunMode::Reference
            || self.predecessor != predecessor.summary
            || self.reference_provenance
                != Some(RespReferenceProvenance::from_validated(
                    scenario,
                    predecessor,
                    selected,
                    controls,
                ))
            || self.event.selected_endpoint != selected.capability.config.redis_addr.to_string()
            || self.event.original != selected.process
            || self.event.original.pid == predecessor.old_pid
        {
            return Err(BrownoutError::Capability(
                "W5B artifact does not bind archived W3 capacity to new selected/control processes"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RespBrownoutLoadPlan {
    pub offered_rate_per_second: u64,
    pub observation_window_millis: u64,
    pub independent_control_endpoints: u8,
    pub selected_capacity: RespSelectedCapacityContract,
    pub capacity_matrix_sha256: String,
    pub predecessor_scenario_digest: String,
    pub predecessor_workload_digest: String,
    pub predecessor_artifact_sha256: String,
    pub predecessor_lifecycle_sha256: String,
}

#[async_trait]
pub trait RespBrownoutDriver: Send + Sync {
    /// Execute the process/socket lifecycle and return only raw process receipts
    /// and W0 windows.  The W5 verdict is always derived by this module.
    async fn observe_selected_endpoint_kill_restart(
        &self,
        plan: &RespBrownoutLoadPlan,
    ) -> Result<RawRespEndpointEvent, BrownoutError>;
}

pub async fn run_resp_reference<D: RespBrownoutDriver>(
    scenario: &RespBrownoutScenario,
    predecessor: RespCapacityPredecessor,
    selected: RespExecutionCapability,
    independent_controls: Vec<RespExecutionCapability>,
    driver: &D,
) -> Result<RespBrownoutReport, BrownoutError> {
    scenario.validate_exact_reference_shape()?;
    require_reference_gate(RESP_REFERENCE_ENV, "W5B process/socket lifecycle")?;
    predecessor.validate(scenario.load.fixed_rate_fraction_millionths)?;
    selected.validate()?;
    for control in &independent_controls {
        control.validate()?;
    }
    let plan = RespBrownoutLoadPlan {
        offered_rate_per_second: predecessor.summary.offered_rate_per_second,
        observation_window_millis: scenario.load.observation_window_millis,
        independent_control_endpoints: scenario.event.independent_control_endpoints,
        selected_capacity: predecessor.selected_capacity.clone(),
        capacity_matrix_sha256: predecessor.capacity_matrix_sha256.clone(),
        predecessor_scenario_digest: predecessor.scenario_digest.clone(),
        predecessor_workload_digest: predecessor.workload_digest.clone(),
        predecessor_artifact_sha256: predecessor.summary.artifact_sha256.clone(),
        predecessor_lifecycle_sha256: predecessor.lifecycle_artifact_sha256.clone(),
    };
    let raw = driver.observe_selected_endpoint_kill_restart(&plan).await?;
    validate_resp_controls(
        scenario,
        &predecessor,
        &selected,
        &independent_controls,
        &raw,
    )?;
    let report = RespBrownoutReport {
        schema_version: BROWNOUT_REPORT_VERSION,
        scenario_id: scenario.scenario_id.clone(),
        scenario_sha256: scenario.contract_sha256(),
        evidence_class: RESP_EVIDENCE_CLASS.to_owned(),
        run_mode: BrownoutRunMode::Reference,
        predecessor: predecessor.summary.clone(),
        reference_provenance: Some(RespReferenceProvenance::from_validated(
            scenario,
            &predecessor,
            &selected,
            &independent_controls,
        )),
        selected_endpoint_recovery_millis: raw.timeline.recovery_millis(),
        event: raw,
        node_local_state: true,
        automatic_failover: false,
        neighbor_visibility_claim: false,
        value_survival_claim: false,
        cross_node_failover_claim: false,
        aggregate_goodput: false,
    };
    report.validate_reference(scenario, &predecessor, &selected, &independent_controls)?;
    Ok(report)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelPredecessorBinding {
    pub evidence_class: String,
    pub artifact_sha256: Option<String>,
    pub reference_receipt_sha256: Option<String>,
    pub synthetic_knee_rate_per_second: Option<u64>,
}

impl ModelPredecessorBinding {
    fn smoke() -> Self {
        Self {
            evidence_class: GRID_MODEL_PREDECESSOR.to_owned(),
            artifact_sha256: None,
            reference_receipt_sha256: None,
            synthetic_knee_rate_per_second: None,
        }
    }

    fn from_reference(predecessor: &GridModelPredecessor) -> Self {
        Self {
            evidence_class: GRID_MODEL_PREDECESSOR.to_owned(),
            artifact_sha256: Some(predecessor.artifact_sha256.clone()),
            reference_receipt_sha256: Some(predecessor.reference_receipt_sha256.clone()),
            synthetic_knee_rate_per_second: None,
        }
    }

    fn validate(&self, run_mode: BrownoutRunMode) -> Result<(), BrownoutError> {
        if self.evidence_class != GRID_MODEL_PREDECESSOR
            || self.synthetic_knee_rate_per_second.is_some()
        {
            return Err(BrownoutError::Boundary(
                "W5C must bind W4B without inventing a synthetic capacity knee".to_owned(),
            ));
        }
        match (
            run_mode,
            self.artifact_sha256.as_deref(),
            self.reference_receipt_sha256.as_deref(),
        ) {
            (BrownoutRunMode::DeterministicSmoke, None, None) => Ok(()),
            (BrownoutRunMode::Reference, Some(artifact), Some(receipt))
                if is_sha256(artifact) && is_sha256(receipt) =>
            {
                Ok(())
            }
            (BrownoutRunMode::DeterministicSmoke, _, _) => Err(BrownoutError::Boundary(
                "W5C smoke cannot carry W4B reference receipts".to_owned(),
            )),
            _ => Err(BrownoutError::Capability(
                "W5C reference is missing its exact W4B artifact/attestation".to_owned(),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplicaFaultInvocation {
    Sent {
        admitted: bool,
        max_in_flight: usize,
    },
    Unavailable {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelFaultRawRepeat {
    pub repeat_index: u8,
    pub warmup_iterations: u64,
    pub steady_iterations: u64,
    pub fresh_model_identity_sha256: String,
    pub baseline_elapsed_nanos: u64,
    pub baseline_result_checksum: u64,
    pub baseline_admitted_sends: u64,
    pub fault_elapsed_nanos: u64,
    pub fault_result_checksum: u64,
    pub injected_fault_events: u64,
    pub unavailable_decisions: u64,
    pub slow_primitive_calls: u64,
    pub recovery_elapsed_nanos: u64,
    pub recovery_result_checksum: u64,
    pub recovery_admitted_sends: u64,
    pub final_record_checksum: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelTimingSummary {
    pub median_nanos_per_iteration: u64,
    pub robust_spread_ratio_millionths: u64,
    pub stable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelReplicaFaultEvidence {
    pub fault: ModelReplicaFault,
    pub primitive: String,
    pub fault_adapter: String,
    pub raw_repeats: Vec<ModelFaultRawRepeat>,
    pub baseline_timing: ModelTimingSummary,
    pub fault_timing: ModelTimingSummary,
    pub recovery_timing: ModelTimingSummary,
    pub affected_decisions: u64,
    pub injected_fault_events: u64,
    pub independent_result_checksum: u64,
}

/// Sealed proof that W5C executed in a new loadgen process invocation after
/// revalidating the exact W4B source/runner/prebuild context. The canonical
/// loadgen path is re-hashed whenever reference evidence is validated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelExecutionReceipt {
    source_commit: String,
    cargo_lock_sha256: String,
    runner_fingerprint: String,
    prebuild_manifest_sha256: String,
    prebuild_contract_sha256: String,
    loadgen_canonical_path: PathBuf,
    loadgen_sha256: String,
    process_id: u32,
    started_unix_nanos: u64,
    sequence: u64,
    receipt_sha256: String,
}

impl GridModelExecutionReceipt {
    fn from_fresh_context(
        predecessor: &GridModelPredecessor,
        context: &ValidatedRespReferenceContext,
        sequence: u64,
        started_unix_nanos: u64,
    ) -> Result<Self, BrownoutError> {
        let mut receipt = Self {
            source_commit: context.source.git_commit.clone(),
            cargo_lock_sha256: context.source.cargo_lock_sha256.clone(),
            runner_fingerprint: context.runner.fingerprint.clone(),
            prebuild_manifest_sha256: context.manifest_sha256.clone(),
            prebuild_contract_sha256: context.build.prebuild_contract_digest.clone(),
            loadgen_canonical_path: context.loadgen.canonical_path.clone(),
            loadgen_sha256: context.loadgen.sha256.clone(),
            process_id: std::process::id(),
            started_unix_nanos,
            sequence,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt.computed_receipt();
        receipt.validate_against(predecessor)?;
        Ok(receipt)
    }

    fn computed_receipt(&self) -> String {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        digest_json(&payload)
    }

    fn validate_shape(&self) -> Result<(), BrownoutError> {
        let canonical = fs::canonicalize(&self.loadgen_canonical_path).map_err(|error| {
            BrownoutError::Capability(format!(
                "cannot canonicalize fresh W5C loadgen {}: {error}",
                self.loadgen_canonical_path.display()
            ))
        })?;
        if !is_git_commit(&self.source_commit)
            || !is_sha256(&self.cargo_lock_sha256)
            || !is_sha256(&self.runner_fingerprint)
            || !is_sha256(&self.prebuild_manifest_sha256)
            || !is_sha256(&self.prebuild_contract_sha256)
            || canonical != self.loadgen_canonical_path
            || canonical.file_name().and_then(|name| name.to_str())
                != Some(format!("{LOADGEN_BINARY_ID}{}", std::env::consts::EXE_SUFFIX).as_str())
            || !is_sha256(&self.loadgen_sha256)
            || hash_file(&canonical)? != self.loadgen_sha256
            || self.process_id == 0
            || self.started_unix_nanos == 0
            || self.sequence == 0
            || self.receipt_sha256 != self.computed_receipt()
        {
            return Err(BrownoutError::Capability(
                "fresh W5C execution receipt is incomplete, non-canonical, changed on disk, or unsealed"
                    .to_owned(),
            ));
        }
        Ok(())
    }

    fn validate_against(&self, predecessor: &GridModelPredecessor) -> Result<(), BrownoutError> {
        self.validate_shape()?;
        predecessor.validate()?;
        let loadgen = predecessor.binary(LOADGEN_BINARY_ID)?;
        if self.source_commit != predecessor.source_commit
            || self.cargo_lock_sha256 != predecessor.source.cargo_lock_sha256
            || self.runner_fingerprint != predecessor.runner_fingerprint
            || self.prebuild_manifest_sha256 != predecessor.prebuild_manifest_sha256
            || self.prebuild_contract_sha256 != predecessor.prebuild.build_contract_digest
            || self.loadgen_canonical_path != loadgen.canonical_path
            || self.loadgen_sha256 != loadgen.sha256
        {
            return Err(BrownoutError::Capability(
                "fresh W5C execution does not match the exact W4B source/runner/prebuild identity"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelReferenceProvenance {
    w4b_artifact_sha256: String,
    w4b_reference_receipt_sha256: String,
    w4b_scenario_sha256: String,
    source_commit: String,
    runner_fingerprint: String,
    prebuild_manifest_sha256: String,
    fresh_execution: GridModelExecutionReceipt,
    w5_scenario_sha256: String,
    receipt_sha256: String,
}

impl GridModelReferenceProvenance {
    fn from_validated(
        scenario: &GridModelBrownoutScenario,
        predecessor: &GridModelPredecessor,
        execution: &GridModelExecutionReceipt,
    ) -> Self {
        let mut receipt = Self {
            w4b_artifact_sha256: predecessor.artifact_sha256.clone(),
            w4b_reference_receipt_sha256: predecessor.reference_receipt_sha256.clone(),
            w4b_scenario_sha256: predecessor.scenario_sha256.clone(),
            source_commit: predecessor.source_commit.clone(),
            runner_fingerprint: predecessor.runner_fingerprint.clone(),
            prebuild_manifest_sha256: predecessor.prebuild_manifest_sha256.clone(),
            fresh_execution: execution.clone(),
            w5_scenario_sha256: scenario.contract_sha256(),
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt.computed_receipt();
        receipt
    }

    fn computed_receipt(&self) -> String {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        digest_json(&payload)
    }

    fn validate_shape(&self, scenario: &GridModelBrownoutScenario) -> Result<(), BrownoutError> {
        self.fresh_execution.validate_shape()?;
        if !is_sha256(&self.w4b_artifact_sha256)
            || !is_sha256(&self.w4b_reference_receipt_sha256)
            || !is_sha256(&self.w4b_scenario_sha256)
            || !is_git_commit(&self.source_commit)
            || !is_sha256(&self.runner_fingerprint)
            || !is_sha256(&self.prebuild_manifest_sha256)
            || self.fresh_execution.source_commit != self.source_commit
            || self.fresh_execution.runner_fingerprint != self.runner_fingerprint
            || self.fresh_execution.prebuild_manifest_sha256 != self.prebuild_manifest_sha256
            || self.w5_scenario_sha256 != scenario.contract_sha256()
            || self.receipt_sha256 != self.computed_receipt()
        {
            return Err(BrownoutError::Capability(
                "W5C reference provenance is incomplete or unsealed".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridModelBrownoutReport {
    pub schema_version: u32,
    pub scenario_id: String,
    pub scenario_sha256: String,
    pub evidence_class: String,
    pub run_mode: BrownoutRunMode,
    pub predecessor: ModelPredecessorBinding,
    pub reference_provenance: Option<GridModelReferenceProvenance>,
    pub faults: Vec<ModelReplicaFaultEvidence>,
    pub daemon_brownout_evidence: bool,
    pub product_data_plane: bool,
    pub live_rebalance_measured: bool,
    pub live_reshard_measured: bool,
    pub aggregate_goodput: bool,
}

impl GridModelBrownoutReport {
    pub fn validate(&self, scenario: &GridModelBrownoutScenario) -> Result<(), BrownoutError> {
        scenario.validate()?;
        self.predecessor.validate(self.run_mode)?;
        if self.schema_version != BROWNOUT_REPORT_VERSION
            || self.scenario_id != scenario.scenario_id
            || self.scenario_sha256 != scenario.contract_sha256()
            || self.evidence_class != GRID_MODEL_EVIDENCE_CLASS
            || self.daemon_brownout_evidence
            || self.product_data_plane
            || self.live_rebalance_measured
            || self.live_reshard_measured
            || self.aggregate_goodput
        {
            return Err(BrownoutError::Boundary(
                "W5C report crossed its in-process library/model boundary".to_owned(),
            ));
        }
        match (self.run_mode, self.reference_provenance.as_ref()) {
            (BrownoutRunMode::DeterministicSmoke, None) => {}
            (BrownoutRunMode::Reference, Some(receipt)) => {
                receipt.validate_shape(scenario)?;
            }
            (BrownoutRunMode::DeterministicSmoke, Some(_)) => {
                return Err(BrownoutError::Boundary(
                    "W5C smoke cannot carry reference provenance".to_owned(),
                ));
            }
            _ => {
                return Err(BrownoutError::Capability(
                    "W5C reference is missing sealed W4B provenance".to_owned(),
                ));
            }
        }
        let expected = scenario
            .faults
            .faults
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let actual = self
            .faults
            .iter()
            .map(|fault| fault.fault)
            .collect::<BTreeSet<_>>();
        if self.faults.len() != expected.len() || actual != expected {
            return Err(BrownoutError::Evidence(
                "W5C must retain exactly one slow and one unavailable primitive fault".to_owned(),
            ));
        }
        for fault in &self.faults {
            validate_model_fault(fault, scenario, self.run_mode)?;
        }
        Ok(())
    }

    pub fn validate_reference(
        &self,
        scenario: &GridModelBrownoutScenario,
        predecessor: &GridModelPredecessor,
        execution: &GridModelExecutionReceipt,
    ) -> Result<(), BrownoutError> {
        scenario.validate_exact_reference_shape()?;
        predecessor.validate()?;
        execution.validate_against(predecessor)?;
        self.validate(scenario)?;
        if self.run_mode != BrownoutRunMode::Reference
            || self.predecessor != ModelPredecessorBinding::from_reference(predecessor)
            || self.reference_provenance
                != Some(GridModelReferenceProvenance::from_validated(
                    scenario,
                    predecessor,
                    execution,
                ))
            || self.faults.iter().any(|fault| {
                !fault.baseline_timing.stable
                    || !fault.fault_timing.stable
                    || !fault.recovery_timing.stable
            })
        {
            return Err(BrownoutError::Capability(
                "W5C reference does not bind stable raw measurements to the exact W4B artifact"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

/// Execute the actual exported `LiveReplicationPeer::send_record` primitive
/// behind a typed fault adapter.  Slow invokes the primitive after a bounded
/// delay; unavailable returns a typed unavailability before invocation.
fn invoke_replication_primitive(
    fault: Option<ModelReplicaFault>,
    delay_micros: u64,
    peer: &mut LiveReplicationPeer,
    store: &mut InMemoryReplicatedValueStore,
    key: String,
    record: ReplicatedValueRecord,
) -> Result<ReplicaFaultInvocation, BrownoutError> {
    match fault {
        Some(ModelReplicaFault::SlowReplica) => {
            std::thread::sleep(Duration::from_micros(delay_micros));
        }
        Some(ModelReplicaFault::UnavailableReplica) => {
            return Ok(ReplicaFaultInvocation::Unavailable {
                reason: "replica-fault-adapter-unavailable".to_owned(),
            });
        }
        None => {}
    }
    let sent = peer
        .send_record(store, key, record, true)
        .map_err(|error| BrownoutError::Driver(format!("replication primitive failed: {error}")))?;
    Ok(ReplicaFaultInvocation::Sent {
        admitted: sent.admitted,
        max_in_flight: sent.max_in_flight,
    })
}

pub async fn run_grid_model_reference(
    scenario: &GridModelBrownoutScenario,
    predecessor: GridModelPredecessor,
    execution: GridModelExecutionReceipt,
) -> Result<GridModelBrownoutReport, BrownoutError> {
    scenario.validate_exact_reference_shape()?;
    require_reference_gate(GRID_MODEL_REFERENCE_ENV, "W5C model fault reference")?;
    predecessor.validate()?;
    execution.validate_against(&predecessor)?;
    let faults = measure_model_faults(
        scenario,
        scenario.work.warmup_iterations,
        scenario.work.fixed_iterations,
        scenario.work.raw_repeats,
    )?;
    let report = GridModelBrownoutReport {
        schema_version: BROWNOUT_REPORT_VERSION,
        scenario_id: scenario.scenario_id.clone(),
        scenario_sha256: scenario.contract_sha256(),
        evidence_class: GRID_MODEL_EVIDENCE_CLASS.to_owned(),
        run_mode: BrownoutRunMode::Reference,
        predecessor: ModelPredecessorBinding::from_reference(&predecessor),
        reference_provenance: Some(GridModelReferenceProvenance::from_validated(
            scenario,
            &predecessor,
            &execution,
        )),
        faults,
        daemon_brownout_evidence: false,
        product_data_plane: false,
        live_rebalance_measured: false,
        live_reshard_measured: false,
        aggregate_goodput: false,
    };
    report.validate_reference(scenario, &predecessor, &execution)?;
    Ok(report)
}

/// Fast, explicitly non-promotable plumbing run.  It still executes the real
/// primitive/fault seam but reduces steady work to keep the ordinary test lane
/// cheap; reference validation requires the exact committed shape and W4B
/// predecessor.
pub fn run_grid_model_smoke(
    scenario: &GridModelBrownoutScenario,
) -> Result<GridModelBrownoutReport, BrownoutError> {
    scenario.validate()?;
    let faults = measure_model_faults(
        scenario,
        scenario.work.warmup_iterations.min(4),
        scenario.work.fixed_iterations.min(16),
        scenario.work.raw_repeats,
    )?;
    let report = GridModelBrownoutReport {
        schema_version: BROWNOUT_REPORT_VERSION,
        scenario_id: scenario.scenario_id.clone(),
        scenario_sha256: scenario.contract_sha256(),
        evidence_class: GRID_MODEL_EVIDENCE_CLASS.to_owned(),
        run_mode: BrownoutRunMode::DeterministicSmoke,
        predecessor: ModelPredecessorBinding::smoke(),
        reference_provenance: None,
        faults,
        daemon_brownout_evidence: false,
        product_data_plane: false,
        live_rebalance_measured: false,
        live_reshard_measured: false,
        aggregate_goodput: false,
    };
    report.validate(scenario)?;
    Ok(report)
}

fn measure_model_faults(
    scenario: &GridModelBrownoutScenario,
    warmup_iterations: u64,
    steady_iterations: u64,
    repeats: u8,
) -> Result<Vec<ModelReplicaFaultEvidence>, BrownoutError> {
    scenario
        .faults
        .faults
        .iter()
        .copied()
        .map(|fault| {
            measure_model_fault(
                scenario,
                fault,
                warmup_iterations,
                steady_iterations,
                repeats,
            )
        })
        .collect()
}

fn measure_model_fault(
    scenario: &GridModelBrownoutScenario,
    fault: ModelReplicaFault,
    warmup_iterations: u64,
    steady_iterations: u64,
    repeats: u8,
) -> Result<ModelReplicaFaultEvidence, BrownoutError> {
    let mut raw_repeats = Vec::with_capacity(usize::from(repeats));
    for repeat_index in 0..repeats {
        raw_repeats.push(measure_model_repeat(
            scenario,
            fault,
            repeat_index,
            warmup_iterations,
            steady_iterations,
        )?);
    }
    let baseline_timing = summarize_model_timing(
        &raw_repeats,
        steady_iterations,
        scenario.work.maximum_robust_spread_ratio_millionths,
        |repeat| repeat.baseline_elapsed_nanos,
    )?;
    let fault_timing = summarize_model_timing(
        &raw_repeats,
        steady_iterations,
        scenario.work.maximum_robust_spread_ratio_millionths,
        |repeat| repeat.fault_elapsed_nanos,
    )?;
    let recovery_timing = summarize_model_timing(
        &raw_repeats,
        steady_iterations,
        scenario.work.maximum_robust_spread_ratio_millionths,
        |repeat| repeat.recovery_elapsed_nanos,
    )?;
    let affected_decisions = steady_iterations.saturating_mul(u64::from(repeats));
    let injected_fault_events = raw_repeats
        .iter()
        .map(|repeat| repeat.injected_fault_events)
        .sum();
    let independent_result_checksum =
        raw_repeats
            .iter()
            .fold(checksum_seed(), |checksum, repeat| {
                checksum_mix(
                    checksum_mix(checksum, repeat.baseline_result_checksum),
                    checksum_mix(
                        repeat.fault_result_checksum,
                        repeat.recovery_result_checksum,
                    ),
                )
            });
    Ok(ModelReplicaFaultEvidence {
        fault,
        primitive: "LiveReplicationPeer::send_record".to_owned(),
        fault_adapter: match fault {
            ModelReplicaFault::SlowReplica => "bounded-delay-before-real-primitive",
            ModelReplicaFault::UnavailableReplica => "typed-unavailable-before-real-primitive",
        }
        .to_owned(),
        raw_repeats,
        baseline_timing,
        fault_timing,
        recovery_timing,
        affected_decisions,
        injected_fault_events,
        independent_result_checksum,
    })
}

fn measure_model_repeat(
    scenario: &GridModelBrownoutScenario,
    fault: ModelReplicaFault,
    repeat_index: u8,
    warmup_iterations: u64,
    steady_iterations: u64,
) -> Result<ModelFaultRawRepeat, BrownoutError> {
    let fresh_model_identity_sha256 = digest_json(&(
        "w5c-fresh-replication-model-v1",
        fault,
        repeat_index,
        warmup_iterations,
        steady_iterations,
    ));
    let mut peer = LiveReplicationPeer::new(
        format!("w5c-peer-{repeat_index}"),
        AdaptiveWindow::new(1, 4, 64),
    );
    let mut store = InMemoryReplicatedValueStore::with_budget(u64::MAX);
    let mut next_version = 1_u64;
    for _ in 0..warmup_iterations {
        let invocation = invoke_replication_primitive(
            None,
            0,
            &mut peer,
            &mut store,
            "w5c-key".to_owned(),
            model_record(next_version, repeat_index),
        )?;
        if !matches!(
            invocation,
            ReplicaFaultInvocation::Sent { admitted: true, .. }
        ) {
            return Err(BrownoutError::Driver(
                "W5C warmup primitive unexpectedly rejected a send".to_owned(),
            ));
        }
        next_version = next_version.saturating_add(1);
    }

    let baseline_started = Instant::now();
    let mut baseline_checksum = checksum_seed();
    let mut baseline_admitted = 0_u64;
    for iteration in 0..steady_iterations {
        let record = model_record(next_version, repeat_index);
        let record_checksum = record.artifact_checksum();
        let invocation = invoke_replication_primitive(
            None,
            0,
            &mut peer,
            &mut store,
            "w5c-key".to_owned(),
            record,
        )?;
        let code = invocation_code(&invocation);
        if matches!(
            invocation,
            ReplicaFaultInvocation::Sent { admitted: true, .. }
        ) {
            baseline_admitted = baseline_admitted.saturating_add(1);
        }
        baseline_checksum = checksum_mix(baseline_checksum, iteration);
        baseline_checksum = checksum_mix(baseline_checksum, code);
        baseline_checksum = checksum_mix(baseline_checksum, record_checksum);
        next_version = next_version.saturating_add(1);
    }
    let baseline_elapsed_nanos = elapsed_nanos(baseline_started);

    let fault_started = Instant::now();
    let mut fault_checksum = checksum_seed();
    let mut injected_fault_events = 0_u64;
    let mut unavailable_decisions = 0_u64;
    let mut slow_primitive_calls = 0_u64;
    for iteration in 0..steady_iterations {
        let record = model_record(next_version, repeat_index);
        let record_checksum = record.artifact_checksum();
        let invocation = invoke_replication_primitive(
            Some(fault),
            scenario.faults.slow_replica_delay_micros,
            &mut peer,
            &mut store,
            "w5c-key".to_owned(),
            record,
        )?;
        injected_fault_events = injected_fault_events.saturating_add(1);
        match &invocation {
            ReplicaFaultInvocation::Unavailable { .. } => {
                unavailable_decisions = unavailable_decisions.saturating_add(1);
            }
            ReplicaFaultInvocation::Sent { admitted: true, .. } => {
                slow_primitive_calls = slow_primitive_calls.saturating_add(1);
            }
            ReplicaFaultInvocation::Sent {
                admitted: false, ..
            } => {}
        }
        fault_checksum = checksum_mix(fault_checksum, iteration);
        fault_checksum = checksum_mix(fault_checksum, invocation_code(&invocation));
        fault_checksum = checksum_mix(fault_checksum, record_checksum);
        next_version = next_version.saturating_add(1);
    }
    let fault_elapsed_nanos = elapsed_nanos(fault_started);

    let recovery_started = Instant::now();
    let mut recovery_checksum = checksum_seed();
    let mut recovery_admitted = 0_u64;
    for iteration in 0..steady_iterations {
        let record = model_record(next_version, repeat_index);
        let record_checksum = record.artifact_checksum();
        let invocation = invoke_replication_primitive(
            None,
            0,
            &mut peer,
            &mut store,
            "w5c-key".to_owned(),
            record,
        )?;
        if matches!(
            invocation,
            ReplicaFaultInvocation::Sent { admitted: true, .. }
        ) {
            recovery_admitted = recovery_admitted.saturating_add(1);
        }
        recovery_checksum = checksum_mix(recovery_checksum, iteration);
        recovery_checksum = checksum_mix(recovery_checksum, invocation_code(&invocation));
        recovery_checksum = checksum_mix(recovery_checksum, record_checksum);
        next_version = next_version.saturating_add(1);
    }
    let recovery_elapsed_nanos = elapsed_nanos(recovery_started);
    let final_record_checksum = store
        .get("w5c-key")
        .map_err(|error| BrownoutError::Driver(format!("W5C store read failed: {error}")))?
        .ok_or_else(|| BrownoutError::Driver("W5C store lost its final record".to_owned()))?
        .artifact_checksum();
    Ok(ModelFaultRawRepeat {
        repeat_index,
        warmup_iterations,
        steady_iterations,
        fresh_model_identity_sha256,
        baseline_elapsed_nanos,
        baseline_result_checksum: baseline_checksum,
        baseline_admitted_sends: baseline_admitted,
        fault_elapsed_nanos,
        fault_result_checksum: fault_checksum,
        injected_fault_events,
        unavailable_decisions,
        slow_primitive_calls,
        recovery_elapsed_nanos,
        recovery_result_checksum: recovery_checksum,
        recovery_admitted_sends: recovery_admitted,
        final_record_checksum,
    })
}

fn validate_control_plane_event(
    event: &ControlPlaneEventEvidence,
    scenario: &ControlPlaneBrownoutScenario,
    run_mode: BrownoutRunMode,
    offered_rate_per_second: u64,
) -> Result<(), BrownoutError> {
    if event.action != event.raw.receipt.action()
        || event.transition_recovery_millis != event.raw.timeline.recovery_millis()
        || event.transition_recovery_millis > scenario.events.max_transition_recovery_millis
    {
        return Err(BrownoutError::Evidence(
            "W5A event action/recovery is not derived from its raw typed receipt".to_owned(),
        ));
    }
    let expected_origin = match run_mode {
        BrownoutRunMode::DeterministicSmoke => ObservationOrigin::Fixture,
        BrownoutRunMode::Reference => ObservationOrigin::Observed,
    };
    if event.raw.origin != expected_origin {
        return Err(BrownoutError::Boundary(
            "W5A smoke/observed event origin cannot be promoted across run modes".to_owned(),
        ));
    }
    event.raw.timeline.validate()?;
    validate_open_loop_window(&event.raw.before_window, offered_rate_per_second)?;
    validate_open_loop_window(&event.raw.disruption_window, offered_rate_per_second)?;
    validate_open_loop_window(&event.raw.recovered_window, offered_rate_per_second)?;
    if availability_ppm(&event.raw.recovered_window)? < 990_000 {
        return Err(BrownoutError::Evidence(
            "W5A recovered W0 window did not return to at least 99% availability".to_owned(),
        ));
    }
    let before = snapshot_consensus(&event.raw.before)?;
    let after = snapshot_consensus(&event.raw.after)?;
    match &event.raw.receipt {
        ControlPlaneActionReceipt::LeaderFailover {
            target,
            termination,
        } => {
            target.validate()?;
            termination.validate()?;
            if target.node_id != before.leader
                || target.pid != termination.pid
                || before.members != after.members
                || after.leader == before.leader
                || after.term <= before.term
                || after.epoch < before.epoch
                || event.raw.timeline.commit_millis()
                    > scenario.events.max_leader_unavailable_millis
            {
                return Err(BrownoutError::Evidence(
                    "leader failover must wait the exact leader PID and observe a higher-term different leader over the same members"
                        .to_owned(),
                ));
            }
        }
        ControlPlaneActionReceipt::MemberAdd {
            action,
            added_process,
        } => {
            added_process.validate()?;
            validate_add_action_artifact(action, run_mode == BrownoutRunMode::Reference)?;
            let added = set_difference(&after.members, &before.members);
            if added != BTreeSet::from([added_process.node_id.clone()])
                || !set_difference(&before.members, &after.members).is_empty()
                || after.epoch != before.epoch.saturating_add(1)
                || action.payload.target_node_id != added_process.node_id
                || action.payload.authority_node_id != before.leader
                || action.target_process.pid != added_process.pid
                || action.target_process.observed_executable_sha256 != added_process.binary_sha256
                || action.target_process.config.sha256 != added_process.config_sha256
            {
                return Err(BrownoutError::Evidence(
                    "member add must bind its process/action receipt to an exact +1 committed membership diff"
                        .to_owned(),
                ));
            }
        }
        ControlPlaneActionReceipt::MemberDrain {
            action,
            target,
            cleanup,
        } => {
            target.validate()?;
            cleanup.validate()?;
            let removed = set_difference(&before.members, &after.members);
            if removed != BTreeSet::from([target.node_id.clone()])
                || !set_difference(&after.members, &before.members).is_empty()
                || after.epoch != before.epoch.saturating_add(1)
                || action.target_node_id != target.node_id
                || action.path != w4a::ADMIN_DRAIN_PATH
                || action.action != "drain"
                || action.outcome != "accepted"
                || action.remaining != 0
                || action.timed_out
                || !is_sha256(&action.response_sha256)
                || action.request_network_bytes == 0
                || action.response_network_bytes == 0
                || cleanup.pid != target.pid
            {
                return Err(BrownoutError::Evidence(
                    "member drain must bind its admin receipt to an exact -1 committed membership diff"
                        .to_owned(),
                ));
            }
        }
        ControlPlaneActionReceipt::NodeKillRejoin {
            original,
            termination,
            restarted,
        } => {
            original.validate()?;
            termination.validate()?;
            restarted.validate()?;
            if original.pid != termination.pid
                || original.node_id != restarted.node_id
                || original.pid == restarted.pid
                || original.binary_sha256 != restarted.binary_sha256
                || original.config_sha256 != restarted.config_sha256
                || original.node_id == before.leader
                || before.members != after.members
                || before.leader != after.leader
                || after.epoch < before.epoch
            {
                return Err(BrownoutError::Evidence(
                    "node kill/rejoin must wait the exact non-leader PID and rejoin the same member/image under a new PID"
                        .to_owned(),
                ));
            }
        }
    }
    Ok(())
}

#[derive(Debug)]
struct SnapshotConsensus {
    leader: String,
    term: u64,
    epoch: u64,
    members: BTreeSet<String>,
}

fn snapshot_consensus(
    snapshots: &[w4a::PublicControlPlaneSnapshot],
) -> Result<SnapshotConsensus, BrownoutError> {
    if snapshots.is_empty() {
        return Err(BrownoutError::Evidence(
            "control-plane snapshot set is empty".to_owned(),
        ));
    }
    let endpoint_ids = snapshots
        .iter()
        .map(|snapshot| snapshot.endpoint.node_id.clone())
        .collect::<BTreeSet<_>>();
    if endpoint_ids.len() != snapshots.len() {
        return Err(BrownoutError::Evidence(
            "control-plane snapshot set has duplicate endpoints".to_owned(),
        ));
    }
    let first = &snapshots[0];
    let leader = first.admin_status.leader.clone().ok_or_else(|| {
        BrownoutError::Evidence("control-plane snapshot has no elected leader".to_owned())
    })?;
    let term = first.admin_status.term;
    let epoch = first.admin_status.epoch;
    let members = first
        .admin_status
        .member_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if term == 0
        || epoch == 0
        || members.is_empty()
        || !members.contains(&leader)
        || endpoint_ids != members
    {
        return Err(BrownoutError::Evidence(
            "control-plane snapshot has incomplete endpoint/member/leader identity".to_owned(),
        ));
    }
    for snapshot in snapshots {
        let observed_members = snapshot
            .admin_status
            .member_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let overview_members = snapshot
            .cluster_overview
            .members
            .iter()
            .map(|member| member.node_id.clone())
            .collect::<BTreeSet<_>>();
        let overview_leader = snapshot.cluster_overview.leader.as_ref();
        if snapshot.admin_status.source != w4a::ControlPlaneSource::Live
            || snapshot.cluster_overview.source != w4a::ControlPlaneSource::Live
            || snapshot.admin_status.leader.as_deref() != Some(leader.as_str())
            || snapshot.admin_status.term != term
            || snapshot.admin_status.epoch != epoch
            || !snapshot.admin_status.quorum_ok
            || snapshot.admin_status.members as usize != members.len()
            || observed_members != members
            || overview_members != members
            || overview_leader.map(|value| value.node_id.as_str()) != Some(leader.as_str())
            || overview_leader.map(|value| (value.term, value.epoch)) != Some((term, epoch))
        {
            return Err(BrownoutError::Evidence(
                "public control-plane documents do not converge on one live leader/term/epoch/member set"
                    .to_owned(),
            ));
        }
    }
    Ok(SnapshotConsensus {
        leader,
        term,
        epoch,
        members,
    })
}

fn validate_w5a_capability(
    predecessor: &ControlPlanePredecessor,
    execution: &ControlPlaneExecutionCapability,
) -> Result<(), BrownoutError> {
    execution.validate()?;
    let capability = execution.live();
    let fresh_pids = capability
        .attestation
        .nodes
        .iter()
        .map(|node| node.pid)
        .collect::<BTreeSet<_>>();
    if capability.attestation.profile != REFERENCE_PROFILE
        || capability.attestation.node_count != predecessor.predecessor_node_count
        || execution.receipt.w4_scenario_sha256 != predecessor.w4_scenario_sha256
        || capability.attestation.source_commit != predecessor.source_commit
        || capability.attestation.runner_fingerprint_sha256 != predecessor.runner_fingerprint_sha256
        || capability.attestation.prebuild_manifest_sha256 != predecessor.prebuild_manifest_sha256
        || capability.attestation.prebuild_contract_sha256 != predecessor.prebuild_contract_sha256
        || capability.attestation.server_binary.sha256 != predecessor.server_binary_sha256
        || !is_sha256(capability.receipt_sha256())
        || capability.receipt_sha256() == predecessor.archived_capability_receipt_sha256
        || capability.attestation.nodes.len() != usize::from(capability.attestation.node_count)
        || !fresh_pids.is_disjoint(&predecessor.archived_pids)
        || capability.attestation.nodes.iter().any(|node| {
            node.pid == 0
                || !node.direct_prebuilt_exec
                || node.observed_executable_sha256 != capability.attestation.server_binary.sha256
        })
    {
        return Err(BrownoutError::Capability(
            "W5A fresh execution must match archived stable identities with a new receipt/PID set"
                .to_owned(),
        ));
    }
    let baseline = snapshot_consensus(&capability.baseline)?;
    let nodes = capability
        .attestation
        .nodes
        .iter()
        .map(|node| node.node_id.clone())
        .collect::<BTreeSet<_>>();
    if baseline.members != nodes {
        return Err(BrownoutError::Capability(
            "W5A live capability baseline is not the exact process receipt member set".to_owned(),
        ));
    }
    Ok(())
}

fn validate_control_receipts_against_capability(
    report: &ControlPlaneBrownoutReport,
    capability: &w4a::ProbedControlPlaneCapability,
) -> Result<(), BrownoutError> {
    let processes = capability
        .attestation
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let baseline_members = snapshot_consensus(&capability.baseline)?.members;
    let mut current_members = baseline_members.clone();
    let mut transient_add: Option<ObservedProcessImage> = None;
    for event in &report.events {
        let before_members = snapshot_consensus(&event.raw.before)?.members;
        let after_members = snapshot_consensus(&event.raw.after)?.members;
        if before_members != current_members {
            return Err(BrownoutError::Capability(
                "W5A actions are not one ordered fresh-cluster add/drain lifecycle".to_owned(),
            ));
        }
        let capability_process = match &event.raw.receipt {
            ControlPlaneActionReceipt::LeaderFailover { target, .. } => Some(target),
            ControlPlaneActionReceipt::MemberAdd { added_process, .. } => {
                if transient_add.is_some()
                    || added_process.binary_sha256 != capability.attestation.server_binary.sha256
                    || before_members.contains(&added_process.node_id)
                    || !after_members.contains(&added_process.node_id)
                {
                    return Err(BrownoutError::Capability(
                        "W5A member-add is not one new same-binary transient process".to_owned(),
                    ));
                }
                transient_add = Some(added_process.clone());
                None
            }
            ControlPlaneActionReceipt::MemberDrain {
                target, cleanup, ..
            } => {
                let added = transient_add.as_ref().ok_or_else(|| {
                    BrownoutError::Capability(
                        "W5A member-drain has no preceding transient add".to_owned(),
                    )
                })?;
                if target != added
                    || cleanup.pid != added.pid
                    || after_members.contains(&added.node_id)
                {
                    return Err(BrownoutError::Capability(
                        "W5A member-drain did not remove and reap its exact transient add"
                            .to_owned(),
                    ));
                }
                None
            }
            ControlPlaneActionReceipt::NodeKillRejoin { original, .. } => Some(original),
        };
        if let Some(observed) = capability_process {
            let expected = processes.get(observed.node_id.as_str()).ok_or_else(|| {
                BrownoutError::Capability(
                    "W5A process-control target is absent from capability receipt".to_owned(),
                )
            })?;
            if observed.pid != expected.pid
                || observed.binary_sha256 != expected.observed_executable_sha256
                || observed.config_sha256 != expected.config.sha256
            {
                return Err(BrownoutError::Capability(
                    "W5A action did not target the exact capability PID/binary/config".to_owned(),
                ));
            }
        }
        current_members = after_members;
    }
    if current_members != baseline_members || transient_add.is_none() {
        return Err(BrownoutError::Capability(
            "W5A sequence did not restore the exact fresh capability membership".to_owned(),
        ));
    }
    Ok(())
}

fn validate_control_final_cleanup(
    report: &ControlPlaneBrownoutReport,
    capability: &w4a::ProbedControlPlaneCapability,
) -> Result<(), BrownoutError> {
    let cleanup = report.final_cleanup.as_ref().ok_or_else(|| {
        BrownoutError::Capability("W5A reference has no final cleanup receipt".to_owned())
    })?;
    cleanup.validate_physical()?;
    let initial = capability
        .attestation
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let final_nodes = cleanup
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    if final_nodes.keys().copied().collect::<BTreeSet<_>>()
        != initial.keys().copied().collect::<BTreeSet<_>>()
    {
        return Err(BrownoutError::Capability(
            "W5A final cleanup is not the exact original capability member-id set".to_owned(),
        ));
    }
    let leader = report
        .events
        .iter()
        .find_map(|event| match &event.raw.receipt {
            ControlPlaneActionReceipt::LeaderFailover {
                target,
                termination,
            } => Some((target, termination)),
            _ => None,
        });
    let rejoin = report
        .events
        .iter()
        .find_map(|event| match &event.raw.receipt {
            ControlPlaneActionReceipt::NodeKillRejoin {
                original,
                termination,
                restarted,
            } => Some((original, termination, restarted)),
            _ => None,
        });
    let (leader_target, leader_termination) = leader.ok_or_else(|| {
        BrownoutError::Capability("W5A final cleanup has no leader termination".to_owned())
    })?;
    let (rejoin_original, rejoin_termination, rejoin_restarted) = rejoin.ok_or_else(|| {
        BrownoutError::Capability("W5A final cleanup has no node-rejoin lifecycle".to_owned())
    })?;
    if leader_target.node_id == rejoin_original.node_id
        || leader_termination.pid != leader_target.pid
        || rejoin_termination.pid != rejoin_original.pid
    {
        return Err(BrownoutError::Capability(
            "W5A leader and non-leader event terminations overlap or mismatch".to_owned(),
        ));
    }
    let initial_pids = initial
        .values()
        .map(|node| node.pid)
        .collect::<BTreeSet<_>>();
    for (node_id, initial_node) in &initial {
        let final_node = final_nodes.get(node_id).ok_or_else(|| {
            BrownoutError::Capability(format!("W5A final cleanup lost original node {node_id}"))
        })?;
        if final_node.server_binary_path_after
            != capability.attestation.server_binary.canonical_path
            || final_node.server_binary_sha256_after != capability.attestation.server_binary.sha256
            || final_node.node_config_path_after != initial_node.config.canonical_path
            || final_node.node_config_sha256_after != initial_node.config.sha256
        {
            return Err(BrownoutError::Capability(
                "W5A final cleanup changed a capability binary/config identity".to_owned(),
            ));
        }
        let expected_pid = if *node_id == leader_target.node_id {
            if final_node.pid == initial_node.pid || initial_pids.contains(&final_node.pid) {
                return Err(BrownoutError::Capability(
                    "W5A leader restart reused an initial capability PID".to_owned(),
                ));
            }
            final_node.pid
        } else if *node_id == rejoin_original.node_id {
            rejoin_restarted.pid
        } else {
            initial_node.pid
        };
        if final_node.pid != expected_pid {
            return Err(BrownoutError::Capability(
                "W5A final cleanup PID coverage differs from event restart/untouched processes"
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_resp_raw_event(
    event: &RawRespEndpointEvent,
    scenario: &RespBrownoutScenario,
    run_mode: BrownoutRunMode,
    offered_rate_per_second: u64,
) -> Result<(), BrownoutError> {
    let expected_origin = match run_mode {
        BrownoutRunMode::DeterministicSmoke => ObservationOrigin::Fixture,
        BrownoutRunMode::Reference => ObservationOrigin::Observed,
    };
    if event.origin != expected_origin {
        return Err(BrownoutError::Boundary(
            "W5B smoke/observed lifecycle origin cannot be promoted across run modes".to_owned(),
        ));
    }
    event.original.validate()?;
    event.termination.validate()?;
    event.restarted.validate()?;
    event.restarted_cleanup.validate()?;
    event.socket_unavailable.validate()?;
    event.timeline.validate()?;
    if event.original.pid != event.termination.pid
        || event.original.node_id != event.restarted.node_id
        || event.original.pid == event.restarted.pid
        || event.restarted.pid != event.restarted_cleanup.pid
        || event.original.binary_sha256 != event.restarted.binary_sha256
        || event.original.config_sha256 != event.restarted.config_sha256
        || event.socket_unavailable.endpoint != event.selected_endpoint
        || event.timeline.recovery_millis() > scenario.event.max_recovery_millis
    {
        return Err(BrownoutError::Evidence(
            "W5B must kill/wait/restart the exact selected PID with unchanged binary/config/socket"
                .to_owned(),
        ));
    }
    validate_open_loop_window(&event.before_window, offered_rate_per_second)?;
    validate_open_loop_window(&event.disruption_window, offered_rate_per_second)?;
    validate_open_loop_window(&event.recovered_window, offered_rate_per_second)?;
    if availability_ppm(&event.recovered_window)? < 990_000
        || availability_ppm(&event.disruption_window)? >= availability_ppm(&event.before_window)?
    {
        return Err(BrownoutError::Evidence(
            "W5B selected endpoint lacks an observed outage followed by steady recovery".to_owned(),
        ));
    }
    if event.independent_controls.len() != usize::from(scenario.event.independent_control_endpoints)
    {
        return Err(BrownoutError::Evidence(
            "W5B has the wrong number of independent RESP control windows".to_owned(),
        ));
    }
    let mut endpoints = BTreeSet::new();
    for control in &event.independent_controls {
        control.process.validate()?;
        control.cleanup.validate()?;
        validate_open_loop_window(&control.window, offered_rate_per_second)?;
        if control.process.pid != control.cleanup.pid
            || control.endpoint == event.selected_endpoint
            || !endpoints.insert(control.endpoint.as_str())
            || availability_ppm(&control.window)?
                < scenario.event.min_independent_control_availability_ppm
        {
            return Err(BrownoutError::Evidence(
                "W5B independent control is duplicate, selected, or below its availability floor"
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_resp_controls(
    scenario: &RespBrownoutScenario,
    predecessor: &RespCapacityPredecessor,
    selected: &RespExecutionCapability,
    controls: &[RespExecutionCapability],
    raw: &RawRespEndpointEvent,
) -> Result<(), BrownoutError> {
    if controls.len() != usize::from(scenario.event.independent_control_endpoints)
        || raw.independent_controls.len() != controls.len()
    {
        return Err(BrownoutError::Capability(
            "W5B requires one fresh typed execution capability per independent control".to_owned(),
        ));
    }
    let raw_by_endpoint = raw
        .independent_controls
        .iter()
        .map(|control| (control.endpoint.as_str(), control))
        .collect::<BTreeMap<_, _>>();
    let mut endpoints = BTreeSet::new();
    let mut pids = BTreeSet::from([selected.capability.pid]);
    for control in controls {
        control.validate()?;
        let socket_endpoint = control.capability.config.redis_addr.to_string();
        if control.receipt.scenario_digest != predecessor.scenario_digest
            || control.receipt.workload_digest != predecessor.workload_digest
            || control.receipt.selected_capacity_sha256
                != digest_json(&predecessor.selected_capacity)
            || control.receipt.capacity_matrix_sha256 != predecessor.capacity_matrix_sha256
            || control.receipt.surface_digest != predecessor.surface_digest
            || control.receipt.source_digest != predecessor.source_digest
            || control.receipt.build_digest != predecessor.build_digest
            || control.receipt.runner_fingerprint_sha256 != predecessor.runner_fingerprint_sha256
            || control.capability.selected_endpoint == selected.capability.selected_endpoint
            || control.capability.pid == selected.capability.pid
            || control.capability.pid == predecessor.old_pid
            || !pids.insert(control.capability.pid)
            || control.capability.source_commit != predecessor.source_commit
            || control.capability.prebuild_manifest_sha256
                != predecessor.capability.prebuild_manifest_sha256
            || control.capability.prebuild_contract_digest
                != predecessor.capability.prebuild_contract_digest
            || !endpoints.insert(socket_endpoint.clone())
        {
            return Err(BrownoutError::Capability(
                "W5B control capability is not an independent same-build RESP endpoint".to_owned(),
            ));
        }
        let observed = raw_by_endpoint
            .get(socket_endpoint.as_str())
            .ok_or_else(|| {
                BrownoutError::Capability(
                    "W5B control capability has no matching raw W0 window".to_owned(),
                )
            })?;
        if observed.process != control.process
            || observed.process.pid != control.capability.pid
            || observed.cleanup.pid != control.capability.pid
        {
            return Err(BrownoutError::Capability(
                "W5B control process/cleanup differs from its fresh W3 capability".to_owned(),
            ));
        }
    }
    if selected.receipt.scenario_digest != predecessor.scenario_digest
        || selected.receipt.workload_digest != predecessor.workload_digest
        || selected.receipt.selected_capacity_sha256 != digest_json(&predecessor.selected_capacity)
        || selected.receipt.capacity_matrix_sha256 != predecessor.capacity_matrix_sha256
        || selected.receipt.surface_digest != predecessor.surface_digest
        || selected.receipt.source_digest != predecessor.source_digest
        || selected.receipt.build_digest != predecessor.build_digest
        || selected.receipt.runner_fingerprint_sha256 != predecessor.runner_fingerprint_sha256
        || selected.capability.pid == predecessor.old_pid
    {
        return Err(BrownoutError::Capability(
            "W5B selected fresh capability lost archived W3 stable scenario/workload/build identity"
                .to_owned(),
        ));
    }
    Ok(())
}

fn validate_model_fault(
    evidence: &ModelReplicaFaultEvidence,
    scenario: &GridModelBrownoutScenario,
    run_mode: BrownoutRunMode,
) -> Result<(), BrownoutError> {
    let expected_repeats = scenario.work.raw_repeats;
    let expected_warmup = match run_mode {
        BrownoutRunMode::Reference => scenario.work.warmup_iterations,
        BrownoutRunMode::DeterministicSmoke => scenario.work.warmup_iterations.min(4),
    };
    let expected_iterations = match run_mode {
        BrownoutRunMode::Reference => scenario.work.fixed_iterations,
        BrownoutRunMode::DeterministicSmoke => scenario.work.fixed_iterations.min(16),
    };
    if evidence.primitive != "LiveReplicationPeer::send_record"
        || evidence.fault_adapter
            != match evidence.fault {
                ModelReplicaFault::SlowReplica => "bounded-delay-before-real-primitive",
                ModelReplicaFault::UnavailableReplica => "typed-unavailable-before-real-primitive",
            }
        || evidence.raw_repeats.len() != usize::from(expected_repeats)
    {
        return Err(BrownoutError::Evidence(
            "W5C fault is not the declared exported primitive/adapter/repeat shape".to_owned(),
        ));
    }
    let mut identities = BTreeSet::new();
    for (index, repeat) in evidence.raw_repeats.iter().enumerate() {
        let mut expected = expected_model_repeat(
            evidence.fault,
            index as u8,
            expected_warmup,
            expected_iterations,
        );
        expected.baseline_elapsed_nanos = repeat.baseline_elapsed_nanos;
        expected.fault_elapsed_nanos = repeat.fault_elapsed_nanos;
        expected.recovery_elapsed_nanos = repeat.recovery_elapsed_nanos;
        if repeat != &expected || !identities.insert(repeat.fresh_model_identity_sha256.as_str()) {
            return Err(BrownoutError::Evidence(
                "W5C raw repeat differs from the independent checksum/outcome oracle or reused a model"
                    .to_owned(),
            ));
        }
    }
    let baseline = summarize_model_timing(
        &evidence.raw_repeats,
        expected_iterations,
        scenario.work.maximum_robust_spread_ratio_millionths,
        |repeat| repeat.baseline_elapsed_nanos,
    )?;
    let fault = summarize_model_timing(
        &evidence.raw_repeats,
        expected_iterations,
        scenario.work.maximum_robust_spread_ratio_millionths,
        |repeat| repeat.fault_elapsed_nanos,
    )?;
    let recovery = summarize_model_timing(
        &evidence.raw_repeats,
        expected_iterations,
        scenario.work.maximum_robust_spread_ratio_millionths,
        |repeat| repeat.recovery_elapsed_nanos,
    )?;
    let checksum = evidence
        .raw_repeats
        .iter()
        .fold(checksum_seed(), |checksum, repeat| {
            checksum_mix(
                checksum_mix(checksum, repeat.baseline_result_checksum),
                checksum_mix(
                    repeat.fault_result_checksum,
                    repeat.recovery_result_checksum,
                ),
            )
        });
    let affected = expected_iterations.saturating_mul(u64::from(expected_repeats));
    if evidence.baseline_timing != baseline
        || evidence.fault_timing != fault
        || evidence.recovery_timing != recovery
        || evidence.affected_decisions != affected
        || evidence.injected_fault_events != affected
        || evidence.independent_result_checksum != checksum
        || evidence.recovery_timing.median_nanos_per_iteration
            > scenario.faults.max_recovery_cost_nanos
        || (evidence.fault == ModelReplicaFault::SlowReplica
            && evidence.fault_timing.median_nanos_per_iteration
                <= evidence.baseline_timing.median_nanos_per_iteration)
    {
        return Err(BrownoutError::Evidence(
            "W5C derived timing, fault count, spread, recovery, or independent checksum is forged"
                .to_owned(),
        ));
    }
    Ok(())
}

fn expected_model_repeat(
    fault: ModelReplicaFault,
    repeat_index: u8,
    warmup_iterations: u64,
    steady_iterations: u64,
) -> ModelFaultRawRepeat {
    let identity = digest_json(&(
        "w5c-fresh-replication-model-v1",
        fault,
        repeat_index,
        warmup_iterations,
        steady_iterations,
    ));
    let baseline_start = warmup_iterations.saturating_add(1);
    let fault_start = baseline_start.saturating_add(steady_iterations);
    let recovery_start = fault_start.saturating_add(steady_iterations);
    let baseline_checksum =
        expected_phase_checksum(baseline_start, repeat_index, steady_iterations, 1);
    let fault_code = if fault == ModelReplicaFault::SlowReplica {
        1
    } else {
        3
    };
    let fault_checksum =
        expected_phase_checksum(fault_start, repeat_index, steady_iterations, fault_code);
    let recovery_checksum =
        expected_phase_checksum(recovery_start, repeat_index, steady_iterations, 1);
    let final_version = recovery_start
        .saturating_add(steady_iterations)
        .saturating_sub(1);
    ModelFaultRawRepeat {
        repeat_index,
        warmup_iterations,
        steady_iterations,
        fresh_model_identity_sha256: identity,
        // Timing values are replaced by the observed values before the full
        // structural comparison below.
        baseline_elapsed_nanos: 0,
        baseline_result_checksum: baseline_checksum,
        baseline_admitted_sends: steady_iterations,
        fault_elapsed_nanos: 0,
        fault_result_checksum: fault_checksum,
        injected_fault_events: steady_iterations,
        unavailable_decisions: if fault == ModelReplicaFault::UnavailableReplica {
            steady_iterations
        } else {
            0
        },
        slow_primitive_calls: if fault == ModelReplicaFault::SlowReplica {
            steady_iterations
        } else {
            0
        },
        recovery_elapsed_nanos: 0,
        recovery_result_checksum: recovery_checksum,
        recovery_admitted_sends: steady_iterations,
        final_record_checksum: model_record(final_version, repeat_index).artifact_checksum(),
    }
}

fn expected_phase_checksum(
    start_version: u64,
    repeat_index: u8,
    iterations: u64,
    invocation_code: u64,
) -> u64 {
    let mut checksum = checksum_seed();
    for iteration in 0..iterations {
        checksum = checksum_mix(checksum, iteration);
        checksum = checksum_mix(checksum, invocation_code);
        checksum = checksum_mix(
            checksum,
            model_record(start_version.saturating_add(iteration), repeat_index).artifact_checksum(),
        );
    }
    checksum
}

fn summarize_model_timing<F>(
    repeats: &[ModelFaultRawRepeat],
    iterations: u64,
    maximum_spread_millionths: u64,
    value: F,
) -> Result<ModelTimingSummary, BrownoutError>
where
    F: Fn(&ModelFaultRawRepeat) -> u64,
{
    if repeats.is_empty() || iterations == 0 {
        return Err(BrownoutError::Evidence(
            "W5C timing summary has no repeats/iterations".to_owned(),
        ));
    }
    let mut samples = repeats
        .iter()
        .map(|repeat| value(repeat).saturating_add(iterations - 1) / iterations)
        .collect::<Vec<_>>();
    samples.sort_unstable();
    let median = samples[samples.len() / 2].max(1);
    let mut deviations = samples
        .iter()
        .map(|sample| sample.abs_diff(median))
        .collect::<Vec<_>>();
    deviations.sort_unstable();
    let median_absolute_deviation = deviations[deviations.len() / 2];
    let spread = u64::try_from(
        u128::from(median_absolute_deviation)
            .saturating_mul(1_000_000)
            .checked_div(u128::from(median))
            .unwrap_or(u128::MAX)
            .min(u128::from(u64::MAX)),
    )
    .unwrap_or(u64::MAX);
    Ok(ModelTimingSummary {
        median_nanos_per_iteration: median,
        robust_spread_ratio_millionths: spread,
        stable: spread <= maximum_spread_millionths,
    })
}

fn model_record(version: u64, repeat_index: u8) -> ReplicatedValueRecord {
    let mut payload = vec![repeat_index; 24];
    payload.extend_from_slice(&version.to_le_bytes());
    ReplicatedValueRecord::value(PartitionId::new(7), version, ClusterEpoch::new(67), payload)
}

fn invocation_code(invocation: &ReplicaFaultInvocation) -> u64 {
    match invocation {
        ReplicaFaultInvocation::Sent { admitted: true, .. } => 1,
        ReplicaFaultInvocation::Sent {
            admitted: false, ..
        } => 2,
        ReplicaFaultInvocation::Unavailable { .. } => 3,
    }
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
    duration_nanos(started.elapsed()).max(1)
}

fn duration_nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos())
        .unwrap_or(u64::MAX)
        .max(1)
}

pub fn run_control_plane_smoke(
    scenario: &ControlPlaneBrownoutScenario,
) -> Result<ControlPlaneBrownoutReport, BrownoutError> {
    scenario.validate()?;
    let rate = 600_u64;
    let predecessor = fixture_predecessor(CONTROL_PLANE_PREDECESSOR, rate);
    let base = fixture_control_snapshots(&["node-a", "node-b", "node-c"], "node-a", 1, 1);
    let leader_after = fixture_control_snapshots(&["node-a", "node-b", "node-c"], "node-b", 2, 1);
    let add_after =
        fixture_control_snapshots(&["node-a", "node-b", "node-c", "node-d"], "node-a", 1, 2);
    let drain_after = fixture_control_snapshots(&["node-a", "node-b"], "node-a", 1, 2);
    let node_a = ObservedProcessImage::fixture("node-a", 101, 1);
    let node_c = ObservedProcessImage::fixture("node-c", 103, 3);
    let node_d_receipt = fixture_w4a_process("node-d", 104, 4);
    let node_d = ObservedProcessImage::from_w4a_receipt(&node_d_receipt)?;
    let leader_receipt = ControlPlaneActionReceipt::LeaderFailover {
        target: node_a,
        termination: WaitedProcessTermination::fixture(101),
    };
    let add_receipt = ControlPlaneActionReceipt::MemberAdd {
        action: Box::new(w4a::DaemonAddInvocationReceipt {
            canonical_action_receipt_path: PathBuf::from("fixture/member-add.json"),
            action_receipt_sha256: "4".repeat(64),
            payload: w4a::DaemonAddActionPayload {
                receipt_kind: "hydracache-daemon-add-action-v1".to_owned(),
                provisioner: w4a::DAEMON_CLUSTER_PROVISIONER.to_owned(),
                authority_node_id: "node-a".to_owned(),
                target_node_id: "node-d".to_owned(),
                outcome: "process-started-and-admission-requested".to_owned(),
            },
            target_process: node_d_receipt,
        }),
        added_process: node_d,
    };
    let drain_receipt = ControlPlaneActionReceipt::MemberDrain {
        action: w4a::AdminDrainInvocationReceipt {
            target_node_id: "node-c".to_owned(),
            path: w4a::ADMIN_DRAIN_PATH.to_owned(),
            action: "drain".to_owned(),
            outcome: "accepted".to_owned(),
            started_with: 3,
            remaining: 0,
            timed_out: false,
            response_sha256: "5".repeat(64),
            request_network_bytes: 64,
            response_network_bytes: 64,
        },
        target: node_c.clone(),
        cleanup: WaitedProcessTermination::fixture(node_c.pid),
    };
    let kill_receipt = ControlPlaneActionReceipt::NodeKillRejoin {
        original: node_c.clone(),
        termination: WaitedProcessTermination::fixture(node_c.pid),
        restarted: fixture_restarted_process(&node_c, 203),
    };
    let events = vec![
        fixture_control_event(leader_receipt, base.clone(), leader_after, rate, 250)?,
        fixture_control_event(add_receipt, base.clone(), add_after, rate, 300)?,
        fixture_control_event(drain_receipt, base.clone(), drain_after, rate, 300)?,
        fixture_control_event(kill_receipt, base.clone(), base, rate, 400)?,
    ]
    .into_iter()
    .map(|raw| ControlPlaneEventEvidence {
        action: raw.receipt.action(),
        transition_recovery_millis: raw.timeline.recovery_millis(),
        raw,
    })
    .collect();
    let report = ControlPlaneBrownoutReport {
        schema_version: BROWNOUT_REPORT_VERSION,
        scenario_id: scenario.scenario_id.clone(),
        scenario_sha256: scenario.contract_sha256(),
        evidence_class: CONTROL_PLANE_EVIDENCE_CLASS.to_owned(),
        run_mode: BrownoutRunMode::DeterministicSmoke,
        predecessor,
        predecessor_node_count: scenario.reference.predecessor_node_count,
        reference_provenance: None,
        final_cleanup: None,
        events,
        generic_client_write_invariant: false,
        distributed_value_invariant: false,
        live_reshard_measured: false,
        aggregate_goodput: false,
    };
    report.validate(scenario)?;
    Ok(report)
}

pub fn run_resp_smoke(
    scenario: &RespBrownoutScenario,
) -> Result<RespBrownoutReport, BrownoutError> {
    scenario.validate()?;
    let rate = 600_u64;
    let original = ObservedProcessImage::fixture("resp-selected", 201, 6);
    let restarted = fixture_restarted_process(&original, 202);
    let raw = RawRespEndpointEvent {
        origin: ObservationOrigin::Fixture,
        selected_endpoint: "127.0.0.1:6379".to_owned(),
        original,
        termination: WaitedProcessTermination::fixture(201),
        restarted,
        restarted_cleanup: WaitedProcessTermination::fixture(202),
        socket_unavailable: SocketUnavailableReceipt::fixture("127.0.0.1:6379"),
        before_window: fixture_open_loop(rate, 1_000_000),
        disruption_window: fixture_open_loop(rate, 0),
        recovered_window: fixture_open_loop(rate, 1_000_000),
        independent_controls: vec![IndependentRespRawWindow {
            endpoint: "127.0.0.1:6380".to_owned(),
            process: ObservedProcessImage::fixture("resp-control-1", 301, 8),
            cleanup: WaitedProcessTermination::fixture(301),
            window: fixture_open_loop(rate, 1_000_000),
        }],
        timeline: ObservedEventTimeline::from_nanos(50_000_000, 300_000_000)?,
    };
    let report = RespBrownoutReport {
        schema_version: BROWNOUT_REPORT_VERSION,
        scenario_id: scenario.scenario_id.clone(),
        scenario_sha256: scenario.contract_sha256(),
        evidence_class: RESP_EVIDENCE_CLASS.to_owned(),
        run_mode: BrownoutRunMode::DeterministicSmoke,
        predecessor: fixture_predecessor(RESP_PREDECESSOR, rate),
        reference_provenance: None,
        selected_endpoint_recovery_millis: raw.timeline.recovery_millis(),
        event: raw,
        node_local_state: true,
        automatic_failover: false,
        neighbor_visibility_claim: false,
        value_survival_claim: false,
        cross_node_failover_claim: false,
        aggregate_goodput: false,
    };
    report.validate(scenario)?;
    Ok(report)
}

fn fixture_control_event(
    receipt: ControlPlaneActionReceipt,
    before: Vec<w4a::PublicControlPlaneSnapshot>,
    after: Vec<w4a::PublicControlPlaneSnapshot>,
    rate: u64,
    recovery_millis: u64,
) -> Result<RawControlPlaneEvent, BrownoutError> {
    Ok(RawControlPlaneEvent {
        origin: ObservationOrigin::Fixture,
        receipt,
        before,
        after,
        before_window: fixture_open_loop(rate, 1_000_000),
        disruption_window: fixture_open_loop(rate, 500_000),
        recovered_window: fixture_open_loop(rate, 1_000_000),
        timeline: ObservedEventTimeline::from_nanos(
            recovery_millis.saturating_mul(500_000),
            recovery_millis.saturating_mul(1_000_000),
        )?,
    })
}

fn fixture_predecessor(evidence_class: &str, offered_rate_per_second: u64) -> PredecessorSummary {
    PredecessorSummary {
        evidence_class: evidence_class.to_owned(),
        artifact_sha256: "a".repeat(64),
        reference_receipt_sha256: "b".repeat(64),
        knee_rate_per_second: 1_000,
        offered_rate_per_second,
        rate_fraction_millionths: FRACTION_MILLIONTHS,
    }
}

fn fixture_restarted_process(original: &ObservedProcessImage, pid: u32) -> ObservedProcessImage {
    let mut restarted = original.clone();
    restarted.pid = pid;
    restarted.receipt_sha256 = restarted.computed_receipt();
    restarted
}

fn fixture_w4a_process(
    node_id: &str,
    pid: u32,
    discriminator: u8,
) -> w4a::DaemonNodeProcessReceipt {
    let client_port = 7_000_u16.saturating_add(u16::from(discriminator));
    let cluster_port = 8_000_u16.saturating_add(u16::from(discriminator));
    let admin_port = 9_000_u16.saturating_add(u16::from(discriminator));
    w4a::DaemonNodeProcessReceipt {
        node_id: node_id.to_owned(),
        pid,
        direct_prebuilt_exec: true,
        observed_executable_path: PathBuf::from("fixture/hydracache-server"),
        observed_executable_sha256: format!("{discriminator:x}").repeat(64),
        config: w4a::DaemonNodeConfigReceipt {
            canonical_path: PathBuf::from(format!("fixture/{node_id}.toml")),
            sha256: format!("{:x}", discriminator.saturating_add(1)).repeat(64),
            launch_config: w4a::DaemonNodeLaunchConfig {
                receipt_kind: w4a::NODE_CONFIG_RECEIPT_KIND.to_owned(),
                node_id: node_id.to_owned(),
                client_addr: format!("127.0.0.1:{client_port}")
                    .parse()
                    .expect("fixture address"),
                cluster_addr: format!("127.0.0.1:{cluster_port}")
                    .parse()
                    .expect("fixture address"),
                admin_addr: format!("127.0.0.1:{admin_port}")
                    .parse()
                    .expect("fixture address"),
                redis_addr: None,
                storage_dir: PathBuf::from(format!("fixture/{node_id}-data")),
                cluster_start: "join".to_owned(),
                seed_cluster_addrs: Vec::new(),
            },
        },
    }
}

fn fixture_control_snapshots(
    members: &[&str],
    leader: &str,
    term: u64,
    epoch: u64,
) -> Vec<w4a::PublicControlPlaneSnapshot> {
    let member_ids = members
        .iter()
        .map(|member| (*member).to_owned())
        .collect::<Vec<_>>();
    let voter_ids = (1..=members.len() as u64).collect::<Vec<_>>();
    members
        .iter()
        .enumerate()
        .map(|(index, endpoint)| {
            let admin_port = 9_100_u16.saturating_add(index as u16);
            w4a::PublicControlPlaneSnapshot {
                endpoint: w4a::ControlPlaneEndpoint {
                    node_id: (*endpoint).to_owned(),
                    admin_addr: format!("127.0.0.1:{admin_port}")
                        .parse()
                        .expect("fixture address"),
                },
                admin_status: w4a::AdminStatusObservation {
                    source: w4a::ControlPlaneSource::Live,
                    leader: Some(leader.to_owned()),
                    term,
                    epoch,
                    quorum_ok: true,
                    members: members.len() as u32,
                    member_ids: member_ids.clone(),
                    voters: members.len() as u32,
                    voter_ids: voter_ids.clone(),
                    reshard_phase: "idle".to_owned(),
                    draining: false,
                },
                cluster_overview: w4a::ClusterOverviewObservation {
                    source: w4a::ControlPlaneSource::Live,
                    members: members
                        .iter()
                        .map(|member| w4a::OverviewMemberObservation {
                            node_id: (*member).to_owned(),
                            role: if *member == leader {
                                "leader"
                            } else {
                                "follower"
                            }
                            .to_owned(),
                            reachable: true,
                            reachability: "reachable".to_owned(),
                            generation: epoch,
                        })
                        .collect(),
                    leader: Some(w4a::OverviewLeaderObservation {
                        node_id: leader.to_owned(),
                        term,
                        epoch,
                    }),
                    partitions: w4a::OverviewPartitionObservation {
                        under_replicated: 0,
                        count: 0,
                    },
                    consistency: w4a::OverviewConsistencyObservation {
                        configured_default: None,
                        op_counts_by_level: Vec::new(),
                    },
                    backup_age_seconds: None,
                    lifecycle: w4a::OverviewLifecycleObservation {
                        reshard_phase: "idle".to_owned(),
                        upgrade_phase: "idle".to_owned(),
                    },
                },
            }
        })
        .collect()
}

fn fixture_open_loop(rate: u64, availability_ppm: u32) -> OpenLoopObservation {
    let offered = rate.saturating_mul(OBSERVATION_WINDOW_MILLIS) / 1_000;
    let successes = offered.saturating_mul(u64::from(availability_ppm)) / 1_000_000;
    let errors = offered.saturating_sub(successes);
    OpenLoopObservation {
        offered,
        started: offered,
        completed: offered,
        successes,
        errors,
        timeouts: 0,
        rejections: 0,
        backlog_high_water: 1,
        backlog_drained: true,
        drain_ms: 1,
        elapsed_ms: OBSERVATION_WINDOW_MILLIS,
        offered_rate_per_second: rate as f64,
        achieved_rate_per_second: successes as f64 / (OBSERVATION_WINDOW_MILLIS as f64 / 1_000.0),
        latency: crate::histogram::LatencySummary {
            samples: offered,
            p50_us: Some(100),
            p90_us: Some(150),
            p99_us: Some(200),
            p999_us: Some(250),
            p999_min_samples: 1_000,
            p999_reportable: offered >= 1_000,
            max_us: Some(300),
            overflow_count: 0,
        },
    }
}

pub fn canary_extended_leader_downtime_breaches_the_control_plane_brownout_budget(
    control: &ControlPlaneBrownoutScenario,
    resp: &RespBrownoutScenario,
    model: &GridModelBrownoutScenario,
) -> Result<(), String> {
    let mut control_report = run_control_plane_smoke(control)
        .map_err(|error| format!("W5A canary baseline failed: {error}"))?;
    let event = control_report
        .events
        .iter_mut()
        .find(|event| event.action == ControlPlaneBrownoutAction::LeaderFailover)
        .ok_or_else(|| "W5A canary baseline has no leader-failover event".to_owned())?;
    event.raw.timeline = ObservedEventTimeline::from_nanos(
        control
            .events
            .max_leader_unavailable_millis
            .saturating_add(1)
            .saturating_mul(1_000_000),
        control
            .events
            .max_transition_recovery_millis
            .saturating_mul(1_000_000),
    )
    .map_err(|error| error.to_string())?;
    event.transition_recovery_millis = event.raw.timeline.recovery_millis();

    let mut resp_report =
        run_resp_smoke(resp).map_err(|error| format!("W5B canary baseline failed: {error}"))?;
    resp_report.automatic_failover = true;
    resp_report.neighbor_visibility_claim = true;
    resp_report.cross_node_failover_claim = true;

    let mut model_report = run_grid_model_smoke(model)
        .map_err(|error| format!("W5C canary baseline failed: {error}"))?;
    for fault in &mut model_report.faults {
        fault.injected_fault_events = 0;
        for repeat in &mut fault.raw_repeats {
            repeat.injected_fault_events = 0;
            repeat.unavailable_decisions = 0;
            repeat.slow_primitive_calls = 0;
        }
    }

    if control_report.validate(control).is_err()
        && resp_report.validate(resp).is_err()
        && model_report.validate(model).is_err()
    {
        Err(format!(
            "{W5_CANARY_MARKER} extended leader downtime, false RESP neighbor failover, and a no-op model fault were all rejected"
        ))
    } else {
        Ok(())
    }
}

fn validate_open_loop_window(
    window: &OpenLoopObservation,
    expected_rate_per_second: u64,
) -> Result<(), BrownoutError> {
    let classified = window
        .successes
        .checked_add(window.errors)
        .and_then(|value| value.checked_add(window.timeouts))
        .and_then(|value| value.checked_add(window.rejections))
        .ok_or_else(|| BrownoutError::Evidence("W0 window accounting overflow".to_owned()))?;
    let rate_matches =
        (window.offered_rate_per_second - expected_rate_per_second as f64).abs() <= f64::EPSILON;
    if window.offered == 0
        || window.started != window.offered
        || window.completed != window.started
        || classified != window.completed
        || !window.backlog_drained
        || window.backlog_high_water > window.started
        || window.elapsed_ms == 0
        || !rate_matches
        || !window.achieved_rate_per_second.is_finite()
        || window.achieved_rate_per_second < 0.0
        || window.latency.samples != window.completed
        || window.latency.p99_us.is_none()
        || window.latency.overflow_count != 0
    {
        return Err(BrownoutError::Evidence(
            "raw W0 window is unbalanced, closed-loop, undrained, or rate/latency forged"
                .to_owned(),
        ));
    }
    Ok(())
}

fn availability_ppm(window: &OpenLoopObservation) -> Result<u32, BrownoutError> {
    if window.offered == 0 {
        return Err(BrownoutError::Evidence(
            "cannot derive availability from an empty W0 window".to_owned(),
        ));
    }
    let value = u128::from(window.successes)
        .saturating_mul(1_000_000)
        .checked_div(u128::from(window.offered))
        .unwrap_or(0)
        .min(1_000_000);
    Ok(u32::try_from(value).unwrap_or(1_000_000))
}

fn validate_perf_reference(report: &PerfReport, artifact_json: &[u8]) -> Result<(), BrownoutError> {
    if report.run_mode != EvidenceRunMode::ReferenceEvidence
        || !report.stable
        || !report.validation_problems().is_empty()
    {
        return Err(BrownoutError::Capability(
            "W5B accepts only a semantically valid stable W3 reference report".to_owned(),
        ));
    }
    let parsed: PerfReport = serde_json::from_slice(artifact_json).map_err(|error| {
        BrownoutError::Capability(format!("W3 artifact is not typed JSON: {error}"))
    })?;
    if parsed.run_mode != EvidenceRunMode::ReferenceEvidence
        || !parsed.stable
        || !parsed.validation_problems().is_empty()
        || digest_json(&parsed) != digest_json(report)
    {
        return Err(BrownoutError::Capability(
            "W3 object differs from or fails validation as the exact artifact bytes".to_owned(),
        ));
    }
    Ok(())
}

fn require_reference_gate(variable: &str, operation: &str) -> Result<(), BrownoutError> {
    if std::env::var(variable).as_deref() != Ok("1") {
        return Err(BrownoutError::Capability(format!(
            "{variable}=1 is required before {operation} may run"
        )));
    }
    Ok(())
}

fn validate_archived_resp_lifecycle(
    capability: &RespEndpointCapability,
    lifecycle: &RespDaemonEvidence,
    lifecycle_json: &[u8],
) -> Result<(), BrownoutError> {
    let parsed: RespDaemonEvidence = serde_json::from_slice(lifecycle_json).map_err(|error| {
        BrownoutError::Capability(format!("W3 lifecycle artifact is not typed JSON: {error}"))
    })?;
    if &parsed != lifecycle
        || lifecycle.pid != capability.pid
        || !lifecycle.direct_prebuilt_exec
        || !lifecycle.binaries_verified_after_measurement
        || !lifecycle.killed_and_waited
        || lifecycle.server_binary_sha256 != capability.server_binary_sha256
        || lifecycle.loadgen_binary_sha256 != capability.loadgen_binary_sha256
        || lifecycle.resp_endpoint != capability.config.redis_addr
        || lifecycle.admin_endpoint != capability.config.admin_addr
        || lifecycle.selected_endpoint != capability.selected_endpoint
        || lifecycle.endpoint_capability_digest != digest_json(capability)
        || lifecycle.data_dir != capability.config.storage_dir
        || hash_file(&lifecycle.server_binary_path)? != lifecycle.server_binary_sha256
        || hash_file(&lifecycle.loadgen_binary_path)? != lifecycle.loadgen_binary_sha256
        || !valid_log_evidence(&lifecycle.stdout_log)?
        || !valid_log_evidence(&lifecycle.stderr_log)?
        || !is_sha256(&lifecycle.readiness.request_sha256)
        || !is_sha256(&lifecycle.readiness.response_sha256)
        || lifecycle.readiness.selected_endpoint != capability.config.redis_addr
        || lifecycle.readiness.attempts == 0
    {
        return Err(BrownoutError::Capability(
            "W5B predecessor lifecycle does not prove exact W3 PID cleanup and unchanged artifacts"
                .to_owned(),
        ));
    }
    Ok(())
}

fn valid_log_evidence(
    evidence: &crate::tiers::resp_reference::LogEvidence,
) -> Result<bool, BrownoutError> {
    let metadata = fs::metadata(&evidence.canonical_path).map_err(|error| {
        BrownoutError::Capability(format!(
            "unable to stat archived W3 log {}: {error}",
            evidence.canonical_path.display()
        ))
    })?;
    Ok(metadata.len() == evidence.bytes
        && is_sha256(&evidence.sha256)
        && hash_file(&evidence.canonical_path)? == evidence.sha256)
}

fn valid_process_log(evidence: &w4a::ProcessLogReceipt) -> Result<bool, BrownoutError> {
    let canonical = fs::canonicalize(&evidence.canonical_path).map_err(|error| {
        BrownoutError::Evidence(format!(
            "unable to canonicalize W5A cleanup log {}: {error}",
            evidence.canonical_path.display()
        ))
    })?;
    let metadata = fs::metadata(&canonical).map_err(|error| {
        BrownoutError::Evidence(format!(
            "unable to stat W5A cleanup log {}: {error}",
            canonical.display()
        ))
    })?;
    Ok(canonical == evidence.canonical_path
        && metadata.is_file()
        && metadata.len() == evidence.bytes
        && is_sha256(&evidence.sha256)
        && hash_file(&canonical)? == evidence.sha256)
}

fn resp_config_semantics_sha256(capability: &RespEndpointCapability) -> String {
    #[derive(Serialize)]
    struct StableRespConfig<'a> {
        role: &'a str,
        listen_addr: std::net::SocketAddr,
        cluster_addr: std::net::SocketAddr,
        admin_enabled: bool,
        redis_enabled: bool,
        redis_auth_required: bool,
        rediss_enabled: bool,
    }
    digest_json(&StableRespConfig {
        role: &capability.config.role,
        listen_addr: capability.config.listen_addr,
        cluster_addr: capability.config.cluster_addr,
        admin_enabled: capability.config.admin_enabled,
        redis_enabled: capability.config.redis_enabled,
        redis_auth_required: capability.config.redis_auth_required,
        rediss_enabled: capability.config.rediss_enabled,
    })
}

fn sub_knee_rate(knee: u64, fraction_millionths: u32) -> Result<u64, BrownoutError> {
    let rate = u128::from(knee)
        .checked_mul(u128::from(fraction_millionths))
        .and_then(|value| value.checked_div(1_000_000))
        .and_then(|value| u64::try_from(value).ok())
        .ok_or_else(|| BrownoutError::Evidence("sub-knee rate overflow".to_owned()))?;
    if knee == 0 || rate == 0 || rate >= knee {
        return Err(BrownoutError::Evidence(
            "sub-knee rate must be non-zero and strictly below its predecessor".to_owned(),
        ));
    }
    Ok(rate)
}

fn exact_u64_rate(rate: f64, label: &str) -> Result<u64, BrownoutError> {
    if !rate.is_finite() || rate <= 0.0 || rate.fract() != 0.0 || rate > u64::MAX as f64 {
        return Err(BrownoutError::Capability(format!(
            "{label} is not an exact positive integer rate"
        )));
    }
    Ok(rate as u64)
}

fn set_difference(left: &BTreeSet<String>, right: &BTreeSet<String>) -> BTreeSet<String> {
    left.difference(right).cloned().collect()
}

fn digest_json<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("serializable W5 contract/evidence");
    sha256_hex(&bytes)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn hash_file(path: &Path) -> Result<String, BrownoutError> {
    let bytes = fs::read(path).map_err(|error| {
        BrownoutError::Capability(format!(
            "unable to read observed file {}: {error}",
            path.display()
        ))
    })?;
    Ok(sha256_hex(&bytes))
}

fn portable_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn is_git_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_socket_endpoint(value: &str) -> bool {
    value.parse::<std::net::SocketAddr>().is_ok()
}
