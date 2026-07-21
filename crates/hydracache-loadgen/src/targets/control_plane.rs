//! W4A: real-daemon control-plane characterization over the public admin HTTP wire.
//!
//! W4A is deliberately unable to describe distributed cache capacity. A report
//! binds one exact daemon endpoint and role to the common W0 open-loop knee
//! predicate. Membership mutations remain separate event-latency evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::knee::{KneeResult, SustainabilityCriteria};
use crate::rate::OpenLoopConfig;
use crate::runner::{run_phases, PhaseConfig};
use crate::target::{PreloadOutcome, Target, TargetError, TargetOutcome, TargetRequest};

pub const CONTROL_PLANE_SCENARIO_VERSION: u32 = 1;
pub const CONTROL_PLANE_REPORT_VERSION: u32 = 1;
pub const CONTROL_PLANE_EVIDENCE_CLASS: &str = "w4a-real-daemon-control-plane";
pub const CONTROL_PLANE_EXECUTION_MODE: &str = "real-daemon-admin-http";
pub const CONTROL_PLANE_STATE_SCOPE: &str = "consensus-backed-metadata";
pub const CONTROL_PLANE_NETWORK_BOUNDARY: &str = "real-admin-http";
pub const CONTROL_PLANE_CAPABILITY_ENV: &str = "HYDRACACHE_RUN_PERF_CONTROL_PLANE";
pub const DAEMON_CLUSTER_PROVISIONER: &str = "daemon-cluster-process-harness";
pub const DAEMON_CAPABILITY_RECEIPT_KIND: &str = "hydracache-real-daemon-cluster-v1";
pub const NODE_CONFIG_RECEIPT_KIND: &str = "hydracache-daemon-launch-config-v1";
pub const ADMIN_STATUS_PATH: &str = "/admin/status";
pub const CLUSTER_OVERVIEW_PATH: &str = "/cluster/overview";
pub const ADMIN_DRAIN_PATH: &str = "/admin/drain";
pub const W4_CANARY_MARKER: &str = "HC-CANARY-RED:W4";

pub const CONTROL_PLANE_OFFERED_RATES_PER_SECOND: [u64; 5] = [250, 500, 1_000, 2_000, 4_000];
pub const CONTROL_PLANE_PRELOAD_OPERATIONS: u64 = 2;
pub const CONTROL_PLANE_WARMUP_OPERATIONS: u64 = 1_000;
pub const CONTROL_PLANE_STEADY_OPERATIONS: u64 = 5_000;
pub const CONTROL_PLANE_REPEATS: u32 = 5;
pub const CONTROL_PLANE_MIN_ACHIEVED_RATIO: f64 = 0.95;
pub const CONTROL_PLANE_MAX_ERROR_RATIO: f64 = 0.0;
pub const CONTROL_PLANE_MAX_TIMEOUT_RATIO: f64 = 0.0;
pub const CONTROL_PLANE_MAX_REJECTION_RATIO: f64 = 0.0;
pub const CONTROL_PLANE_BACKLOG_DRAIN_MS: u64 = 10_000;
pub const CONTROL_PLANE_MAX_ROBUST_SPREAD_RATIO: f64 = 0.15;

const MAX_ADMIN_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const CONTROL_PLANE_STATE_DIGEST_VERSION: &str = "hydracache-w4a-control-plane-state-v2";
const STEADY_COUNTER_SCOPE: &str = "loadgen-observed-warmup-and-steady-admin-http-wire-v1";
const EVENT_COUNTER_SCOPE: &str = "loadgen-observed-membership-admin-http-wire-v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlaneScenario {
    pub schema_version: u32,
    pub scenario_id: String,
    pub identity: ControlPlaneIdentity,
    pub read_only: ControlPlaneReadContract,
    pub membership_event: MembershipEventContract,
    pub reference: ControlPlaneReferenceContract,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlaneIdentity {
    pub evidence_class: String,
    pub execution_mode: String,
    pub state_scope: String,
    pub network_boundary: String,
    pub daemon_processes: bool,
    pub product_data_plane: bool,
    pub aggregate_cluster_capacity: bool,
    pub live_reshard_measured: bool,
    pub capacity_claim: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlaneReadContract {
    pub measurement_id: String,
    pub node_counts: Vec<u8>,
    pub target_roles: Vec<NodeRole>,
    pub paths: Vec<String>,
    pub timeout_millis: u64,
    pub offered_rates_per_second: Vec<u64>,
    pub preload_operations: u64,
    pub warmup_operations: u64,
    pub steady_operations: u64,
    pub repeats: u32,
    pub latency_slo_micros: u64,
    pub p999_slo_micros: Option<u64>,
    pub p999_min_samples: u64,
    pub highest_trackable_latency_micros: u64,
    pub histogram_significant_figures: u8,
    pub min_achieved_ratio: f64,
    pub max_error_ratio: f64,
    pub max_timeout_ratio: f64,
    pub max_rejection_ratio: f64,
    pub backlog_drain_millis: u64,
    pub max_robust_spread_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MembershipEventContract {
    pub measurement_id: String,
    pub actions: Vec<MembershipAction>,
    pub metric_kind: String,
    pub event_timeout_millis: u64,
    pub live_reshard_deferred: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlaneReferenceContract {
    pub capability_env: String,
    pub required_profile: String,
    pub required_receipt_kind: String,
    pub require_live_source: bool,
    pub require_complete_endpoint_set: bool,
    pub require_os_process_identity: bool,
    pub require_direct_prebuilt_exec: bool,
    pub role_change_invalidates_steady_window: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRole {
    Leader,
    Follower,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipAction {
    Add,
    Drain,
}

impl ControlPlaneScenario {
    pub fn parse_toml(text: &str) -> Result<Self, ControlPlaneError> {
        let scenario: Self = toml::from_str(text)
            .map_err(|error| ControlPlaneError::Contract(format!("invalid TOML: {error}")))?;
        scenario.validate()?;
        Ok(scenario)
    }

    pub fn load(path: &Path) -> Result<Self, ControlPlaneError> {
        let text = fs::read_to_string(path).map_err(|error| {
            ControlPlaneError::Contract(format!(
                "unable to read W4A scenario {}: {error}",
                path.display()
            ))
        })?;
        Self::parse_toml(&text)
    }

    pub fn sustainability_criteria(&self) -> SustainabilityCriteria {
        SustainabilityCriteria {
            p99_slo_us: self.read_only.latency_slo_micros,
            p999_slo_us: self.read_only.p999_slo_micros,
            min_achieved_ratio: self.read_only.min_achieved_ratio,
            max_error_ratio: self.read_only.max_error_ratio,
            max_timeout_ratio: self.read_only.max_timeout_ratio,
            max_rejection_ratio: self.read_only.max_rejection_ratio,
            max_drain_ms: self.read_only.backlog_drain_millis,
            max_robust_spread_ratio: self.read_only.max_robust_spread_ratio,
        }
    }

    pub fn validate(&self) -> Result<(), ControlPlaneError> {
        if self.schema_version != CONTROL_PLANE_SCENARIO_VERSION
            || !portable_identifier(&self.scenario_id)
        {
            return Err(ControlPlaneError::Contract(
                "W4A scenario schema/id is invalid".to_owned(),
            ));
        }
        self.identity.validate()?;
        self.read_only.validate()?;
        self.membership_event.validate()?;
        self.reference.validate()?;
        self.sustainability_criteria()
            .validate()
            .map_err(|problems| ControlPlaneError::Contract(problems.join("; ")))?;
        Ok(())
    }
}

impl ControlPlaneIdentity {
    fn validate(&self) -> Result<(), ControlPlaneError> {
        let honest = self.evidence_class == CONTROL_PLANE_EVIDENCE_CLASS
            && self.execution_mode == CONTROL_PLANE_EXECUTION_MODE
            && self.state_scope == CONTROL_PLANE_STATE_SCOPE
            && self.network_boundary == CONTROL_PLANE_NETWORK_BOUNDARY
            && self.daemon_processes
            && !self.product_data_plane
            && !self.aggregate_cluster_capacity
            && !self.live_reshard_measured
            && self.capacity_claim == "selected-admin-endpoint-read-only";
        if !honest {
            return Err(ControlPlaneError::Boundary(
                "W4A must remain real-daemon control-plane evidence with no product-data-plane, summed-capacity, or live-reshard claim".to_owned(),
            ));
        }
        Ok(())
    }
}

impl ControlPlaneReadContract {
    fn validate(&self) -> Result<(), ControlPlaneError> {
        if self.measurement_id != "admin_status_and_overview_knee_at_slo_for_3_5_7_daemons"
            || self.node_counts != [3, 5, 7]
            || self.target_roles != [NodeRole::Leader, NodeRole::Follower]
            || self.paths != [ADMIN_STATUS_PATH, CLUSTER_OVERVIEW_PATH]
            || !(1..=60_000).contains(&self.timeout_millis)
            || self.offered_rates_per_second != CONTROL_PLANE_OFFERED_RATES_PER_SECOND
            || self.preload_operations != CONTROL_PLANE_PRELOAD_OPERATIONS
            || self.warmup_operations != CONTROL_PLANE_WARMUP_OPERATIONS
            || self.steady_operations != CONTROL_PLANE_STEADY_OPERATIONS
            || self.repeats != CONTROL_PLANE_REPEATS
            || self.latency_slo_micros != 50_000
            || self.p999_slo_micros != Some(100_000)
            || self.p999_min_samples != 1_000
            || self.highest_trackable_latency_micros != 10_000_000
            || self.histogram_significant_figures != 3
            || self.min_achieved_ratio != CONTROL_PLANE_MIN_ACHIEVED_RATIO
            || self.max_error_ratio != CONTROL_PLANE_MAX_ERROR_RATIO
            || self.max_timeout_ratio != CONTROL_PLANE_MAX_TIMEOUT_RATIO
            || self.max_rejection_ratio != CONTROL_PLANE_MAX_REJECTION_RATIO
            || self.backlog_drain_millis != CONTROL_PLANE_BACKLOG_DRAIN_MS
            || self.max_robust_spread_ratio != CONTROL_PLANE_MAX_ROBUST_SPREAD_RATIO
        {
            return Err(ControlPlaneError::Contract(
                "W4A read contract must retain the exact W0 schedule, phases, five repeats, complete sustainability predicate, and 3/5/7 single-endpoint semantics".to_owned(),
            ));
        }
        Ok(())
    }
}

impl MembershipEventContract {
    fn validate(&self) -> Result<(), ControlPlaneError> {
        if self.measurement_id != "membership_add_drain_commit_and_convergence_latency_3_5_7"
            || self.actions != [MembershipAction::Add, MembershipAction::Drain]
            || self.metric_kind != "event-latency"
            || !(1..=120_000).contains(&self.event_timeout_millis)
            || !self.live_reshard_deferred
        {
            return Err(ControlPlaneError::Contract(
                "W4A membership contract must retain exact add/drain event-latency semantics"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

impl ControlPlaneReferenceContract {
    fn validate(&self) -> Result<(), ControlPlaneError> {
        if self.capability_env != CONTROL_PLANE_CAPABILITY_ENV
            || self.required_profile != "reference-v1"
            || self.required_receipt_kind != DAEMON_CAPABILITY_RECEIPT_KIND
            || !self.require_live_source
            || !self.require_complete_endpoint_set
            || !self.require_os_process_identity
            || !self.require_direct_prebuilt_exec
            || !self.role_change_invalidates_steady_window
        {
            return Err(ControlPlaneError::Contract(
                "W4A reference contract must require an OS-observed direct-prebuilt real-daemon receipt and invalidate role-changing windows".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlaneEndpoint {
    pub node_id: String,
    pub admin_addr: SocketAddr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DaemonReceiptSource {
    ObservedProcessHarness,
    SelfAsserted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrebuiltServerBinaryReceipt {
    pub canonical_path: PathBuf,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonNodeLaunchConfig {
    pub receipt_kind: String,
    pub node_id: String,
    pub client_addr: SocketAddr,
    pub cluster_addr: SocketAddr,
    pub admin_addr: SocketAddr,
    pub redis_addr: Option<SocketAddr>,
    pub storage_dir: PathBuf,
    pub cluster_start: String,
    pub seed_cluster_addrs: Vec<SocketAddr>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonNodeConfigReceipt {
    pub canonical_path: PathBuf,
    pub sha256: String,
    pub launch_config: DaemonNodeLaunchConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonNodeProcessReceipt {
    pub node_id: String,
    pub pid: u32,
    pub direct_prebuilt_exec: bool,
    pub observed_executable_path: PathBuf,
    pub observed_executable_sha256: String,
    pub config: DaemonNodeConfigReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlaneCapabilityReceiptPayload {
    pub receipt_kind: String,
    pub receipt_source: DaemonReceiptSource,
    pub execution_mode: String,
    pub profile: String,
    pub source_commit: String,
    pub runner_fingerprint_sha256: String,
    pub prebuild_manifest_canonical_path: PathBuf,
    pub prebuild_manifest_sha256: String,
    pub prebuild_contract_sha256: String,
    pub provisioner: String,
    pub direct_prebuilt_exec: bool,
    pub server_binary: PrebuiltServerBinaryReceipt,
    pub node_count: u8,
    pub nodes: Vec<DaemonNodeProcessReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct TypedPrebuildManifest {
    schema_version: u32,
    source: TypedPrebuildSource,
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
    binaries: Vec<TypedPrebuiltBinary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct TypedPrebuildSource {
    git_commit: String,
    cargo_lock_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct TypedPrebuiltBinary {
    id: String,
    canonical_path: PathBuf,
    sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlaneCapabilityAttestation {
    pub payload: Option<ControlPlaneCapabilityReceiptPayload>,
    pub receipt_sha256: String,
}

impl ControlPlaneCapabilityAttestation {
    pub fn absent() -> Self {
        Self {
            payload: None,
            receipt_sha256: String::new(),
        }
    }

    pub fn seal(
        mut payload: ControlPlaneCapabilityReceiptPayload,
    ) -> Result<Self, ControlPlaneError> {
        payload
            .nodes
            .sort_by(|left, right| left.node_id.cmp(&right.node_id));
        let receipt_sha256 = canonical_digest(&payload)?;
        Ok(Self {
            payload: Some(payload),
            receipt_sha256,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceCapabilityPolicy {
    LocalSkipLoud,
    MandatoryFailClosed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlPlaneCapabilityOutcome {
    Ready(Box<ValidatedControlPlaneCapability>),
    SkippedLoud(ControlPlaneSkip),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedControlPlaneCapability {
    pub receipt: ControlPlaneCapabilityAttestation,
    pub profile: String,
    pub source_commit: String,
    pub runner_fingerprint_sha256: String,
    pub prebuild_manifest_sha256: String,
    pub prebuild_contract_sha256: String,
    pub provisioner: String,
    pub server_binary: PrebuiltServerBinaryReceipt,
    pub node_count: u8,
    pub nodes: Vec<DaemonNodeProcessReceipt>,
    pub endpoints: Vec<ControlPlaneEndpoint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbedControlPlaneCapability {
    pub attestation: ValidatedControlPlaneCapability,
    pub baseline: Vec<PublicControlPlaneSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlPlaneSkip {
    pub code: String,
    pub message: String,
}

impl ControlPlaneCapabilityAttestation {
    pub fn require(
        self,
        scenario: &ControlPlaneScenario,
        policy: ReferenceCapabilityPolicy,
    ) -> Result<ControlPlaneCapabilityOutcome, ControlPlaneError> {
        scenario.validate()?;
        let Some(payload) = self.payload.clone() else {
            if self.receipt_sha256.is_empty() && policy == ReferenceCapabilityPolicy::LocalSkipLoud
            {
                return Ok(ControlPlaneCapabilityOutcome::SkippedLoud(ControlPlaneSkip {
                    code: "HC-W4A-CAPABILITY-MISSING".to_owned(),
                    message: "real daemon capability receipt is unavailable; no W4A evidence was produced".to_owned(),
                }));
            }
            return Err(ControlPlaneError::Capability(
                "mandatory W4A capability requires a sealed real-daemon receipt".to_owned(),
            ));
        };
        if self.receipt_sha256 != canonical_digest(&payload)? {
            return Err(ControlPlaneError::Capability(
                "W4A capability receipt digest does not seal its typed payload".to_owned(),
            ));
        }
        validate_capability_payload(&payload, scenario)?;
        Ok(ControlPlaneCapabilityOutcome::Ready(Box::new(
            validated_capability(self, &payload)?,
        )))
    }

    /// Revalidate a persisted W4A launch receipt after its mandatory lifecycle
    /// has reaped every daemon. Static source/prebuild/binary/config/topology
    /// bindings remain exact, while live OS process identity is deliberately
    /// replaced by the report's kill/wait/PID-gone lifecycle proof.
    pub fn require_archived(
        self,
        scenario: &ControlPlaneScenario,
    ) -> Result<ValidatedControlPlaneCapability, ControlPlaneError> {
        scenario.validate()?;
        let payload = self.payload.clone().ok_or_else(|| {
            ControlPlaneError::Capability(
                "archived W4A capability requires a sealed real-daemon receipt".to_owned(),
            )
        })?;
        if self.receipt_sha256 != canonical_digest(&payload)? {
            return Err(ControlPlaneError::Capability(
                "archived W4A capability receipt digest does not seal its typed payload".to_owned(),
            ));
        }
        validate_capability_payload_static(&payload, scenario)?;
        if payload.nodes.iter().any(|node| process_is_alive(node.pid)) {
            return Err(ControlPlaneError::Capability(
                "archived W4A capability still has a live daemon PID".to_owned(),
            ));
        }
        validated_capability(self, &payload)
    }
}

fn validated_capability(
    receipt: ControlPlaneCapabilityAttestation,
    payload: &ControlPlaneCapabilityReceiptPayload,
) -> Result<ValidatedControlPlaneCapability, ControlPlaneError> {
    let endpoints = payload
        .nodes
        .iter()
        .map(|node| ControlPlaneEndpoint {
            node_id: node.node_id.clone(),
            admin_addr: node.config.launch_config.admin_addr,
        })
        .collect::<Vec<_>>();
    let endpoints = normalized_endpoints(&endpoints)?;
    Ok(ValidatedControlPlaneCapability {
        receipt,
        profile: payload.profile.clone(),
        source_commit: payload.source_commit.clone(),
        runner_fingerprint_sha256: payload.runner_fingerprint_sha256.clone(),
        prebuild_manifest_sha256: payload.prebuild_manifest_sha256.clone(),
        prebuild_contract_sha256: payload.prebuild_contract_sha256.clone(),
        provisioner: payload.provisioner.clone(),
        server_binary: payload.server_binary.clone(),
        node_count: payload.node_count,
        nodes: payload.nodes.clone(),
        endpoints,
    })
}

fn validate_capability_payload(
    payload: &ControlPlaneCapabilityReceiptPayload,
    scenario: &ControlPlaneScenario,
) -> Result<(), ControlPlaneError> {
    validate_capability_payload_static(payload, scenario)?;
    for node in &payload.nodes {
        validate_live_node_process(node, &payload.server_binary)?;
    }
    Ok(())
}

fn validate_capability_payload_static(
    payload: &ControlPlaneCapabilityReceiptPayload,
    scenario: &ControlPlaneScenario,
) -> Result<(), ControlPlaneError> {
    if payload.receipt_source != DaemonReceiptSource::ObservedProcessHarness {
        return Err(ControlPlaneError::Capability(
            "self-asserted or fixture loopback sources can never satisfy mandatory W4A capability"
                .to_owned(),
        ));
    }
    if payload.receipt_kind != scenario.reference.required_receipt_kind
        || payload.execution_mode != CONTROL_PLANE_EXECUTION_MODE
        || payload.profile != scenario.reference.required_profile
        || !canonical_commit(&payload.source_commit)
        || !is_sha256(&payload.runner_fingerprint_sha256)
        || !is_sha256(&payload.prebuild_manifest_sha256)
        || !is_sha256(&payload.prebuild_contract_sha256)
        || payload.prebuild_manifest_sha256 == payload.prebuild_contract_sha256
        || payload.provisioner != DAEMON_CLUSTER_PROVISIONER
        || !payload.direct_prebuilt_exec
        || !scenario.read_only.node_counts.contains(&payload.node_count)
        || usize::from(payload.node_count) != payload.nodes.len()
    {
        return Err(ControlPlaneError::Capability(
            "mandatory W4A receipt is not an OS-observed direct-prebuilt 3/5/7-daemon candidate receipt".to_owned(),
        ));
    }
    validate_prebuild_manifest_binding(payload)?;
    validate_prebuilt_binary(&payload.server_binary)?;
    let mut node_ids = BTreeSet::new();
    let mut pids = BTreeSet::new();
    let mut all_addrs = BTreeSet::new();
    let mut config_paths = BTreeSet::new();
    for node in &payload.nodes {
        if !node_ids.insert(node.node_id.as_str())
            || node.pid == 0
            || !pids.insert(node.pid)
            || !node.direct_prebuilt_exec
            || node.observed_executable_path != payload.server_binary.canonical_path
            || node.observed_executable_sha256 != payload.server_binary.sha256
        {
            return Err(ControlPlaneError::Capability(
                "each W4A node must have a unique non-zero PID and the exact prebuilt server executable".to_owned(),
            ));
        }
        validate_node_config_artifacts(node, &mut all_addrs, &mut config_paths)?;
    }
    let initial_cluster_addrs = payload
        .nodes
        .iter()
        .filter(|node| node.config.launch_config.cluster_start == "bootstrap")
        .map(|node| node.config.launch_config.cluster_addr)
        .collect::<BTreeSet<_>>();
    let join_count = payload
        .nodes
        .iter()
        .filter(|node| node.config.launch_config.cluster_start == "join")
        .count();
    if join_count != 1 || initial_cluster_addrs.len() + 1 != payload.nodes.len() {
        return Err(ControlPlaneError::Capability(
            "W4A staged cluster receipt must contain one exact joiner and the N-1 bootstrap cohort"
                .to_owned(),
        ));
    }
    for node in &payload.nodes {
        let config = &node.config.launch_config;
        let expected_seeds = if config.cluster_start == "bootstrap" {
            initial_cluster_addrs
                .iter()
                .copied()
                .filter(|address| *address != config.cluster_addr)
                .collect::<BTreeSet<_>>()
        } else if config.cluster_start == "join" {
            initial_cluster_addrs.clone()
        } else {
            return Err(ControlPlaneError::Capability(
                "node launch config cluster_start must be bootstrap or join".to_owned(),
            ));
        };
        let actual_seeds = config
            .seed_cluster_addrs
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if actual_seeds != expected_seeds || actual_seeds.len() != config.seed_cluster_addrs.len() {
            return Err(ControlPlaneError::Capability(
                "node launch configs must bind exact staged seed sets: bootstrap peers within N-1, joiner to all N-1".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_prebuild_manifest_binding(
    payload: &ControlPlaneCapabilityReceiptPayload,
) -> Result<(), ControlPlaneError> {
    if !payload.prebuild_manifest_canonical_path.is_absolute() {
        return Err(ControlPlaneError::Capability(
            "prebuild manifest receipt path must be absolute and canonical".to_owned(),
        ));
    }
    let path = fs::canonicalize(&payload.prebuild_manifest_canonical_path).map_err(|error| {
        ControlPlaneError::Capability(format!(
            "unable to canonicalize prebuild manifest {}: {error}",
            payload.prebuild_manifest_canonical_path.display()
        ))
    })?;
    let bytes = fs::read(&path).map_err(|error| {
        ControlPlaneError::Capability(format!(
            "unable to read prebuild manifest {}: {error}",
            path.display()
        ))
    })?;
    let manifest: TypedPrebuildManifest = serde_json::from_slice(&bytes).map_err(|error| {
        ControlPlaneError::Capability(format!(
            "prebuild manifest {} is not the strict typed schema: {error}",
            path.display()
        ))
    })?;
    let target_set = manifest
        .target_set
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let server = manifest
        .binaries
        .iter()
        .find(|binary| binary.id == "hydracache-server");
    let binary_ids = manifest
        .binaries
        .iter()
        .map(|binary| binary.id.as_str())
        .collect::<BTreeSet<_>>();
    if path != payload.prebuild_manifest_canonical_path
        || sha256_bytes(&bytes) != payload.prebuild_manifest_sha256
        || manifest.schema_version != 1
        || manifest.source.git_commit != payload.source_commit
        || !is_sha256(&manifest.source.cargo_lock_sha256)
        || manifest.toolchain_identity.trim().is_empty()
        || target_set != BTreeSet::from(["hydracache-loadgen", "hydracache-server"])
        || target_set.len() != manifest.target_set.len()
        || !manifest.features.is_empty()
        || manifest.cargo_profile != "release"
        || manifest.flags != ["--locked", "--release"]
        || manifest.build_recipe
            != ["cargo build -p hydracache-loadgen -p hydracache-server --release --locked"]
        || manifest.build_contract_digest != payload.prebuild_contract_sha256
        || manifest.runner_profile != payload.profile
        || sha256_bytes(manifest.runner_fingerprint.as_bytes()) != payload.runner_fingerprint_sha256
        || manifest.platform_key.trim().is_empty()
        || binary_ids != BTreeSet::from(["hydracache-loadgen", "hydracache-server"])
        || binary_ids.len() != manifest.binaries.len()
        || server.is_none_or(|binary| {
            binary.canonical_path != payload.server_binary.canonical_path
                || binary.sha256 != payload.server_binary.sha256
        })
        || manifest
            .binaries
            .iter()
            .any(|binary| !binary.canonical_path.is_absolute() || !is_sha256(&binary.sha256))
    {
        return Err(ControlPlaneError::Capability(
            "W4A capability is not cross-bound to the exact candidate prebuild manifest/contract, runner, and server binary"
                .to_owned(),
        ));
    }
    Ok(())
}

fn validate_prebuilt_binary(binary: &PrebuiltServerBinaryReceipt) -> Result<(), ControlPlaneError> {
    if !binary.canonical_path.is_absolute() || !is_sha256(&binary.sha256) {
        return Err(ControlPlaneError::Capability(
            "prebuilt server receipt requires an absolute canonical path and lowercase SHA-256"
                .to_owned(),
        ));
    }
    let canonical = fs::canonicalize(&binary.canonical_path).map_err(|error| {
        ControlPlaneError::Capability(format!(
            "unable to canonicalize server binary {}: {error}",
            binary.canonical_path.display()
        ))
    })?;
    if canonical != binary.canonical_path
        || !fs::metadata(&canonical).is_ok_and(|metadata| metadata.is_file())
        || sha256_file(&canonical)? != binary.sha256
    {
        return Err(ControlPlaneError::Capability(
            "prebuilt server path/SHA does not match the exact regular file on disk".to_owned(),
        ));
    }
    Ok(())
}

fn validate_node_config_artifacts(
    node: &DaemonNodeProcessReceipt,
    all_addrs: &mut BTreeSet<SocketAddr>,
    config_paths: &mut BTreeSet<PathBuf>,
) -> Result<(), ControlPlaneError> {
    if !portable_identifier(&node.node_id)
        || node.config.launch_config.receipt_kind != NODE_CONFIG_RECEIPT_KIND
        || node.config.launch_config.node_id != node.node_id
        || !node.config.canonical_path.is_absolute()
        || !is_sha256(&node.config.sha256)
    {
        return Err(ControlPlaneError::Capability(
            "daemon node/config identities are incomplete or mismatched".to_owned(),
        ));
    }
    let config_path = fs::canonicalize(&node.config.canonical_path).map_err(|error| {
        ControlPlaneError::Capability(format!(
            "unable to canonicalize launch config {}: {error}",
            node.config.canonical_path.display()
        ))
    })?;
    let config_bytes = fs::read(&config_path).map_err(|error| {
        ControlPlaneError::Capability(format!(
            "unable to read launch config {}: {error}",
            config_path.display()
        ))
    })?;
    let decoded: DaemonNodeLaunchConfig =
        serde_json::from_slice(&config_bytes).map_err(|error| {
            ControlPlaneError::Capability(format!(
                "launch config {} is not strict typed JSON: {error}",
                config_path.display()
            ))
        })?;
    if config_path != node.config.canonical_path
        || !config_paths.insert(config_path)
        || sha256_bytes(&config_bytes) != node.config.sha256
        || decoded != node.config.launch_config
    {
        return Err(ControlPlaneError::Capability(
            "launch config receipt does not match the exact canonical JSON artifact".to_owned(),
        ));
    }
    let config = &node.config.launch_config;
    let addresses = [
        Some(config.client_addr),
        Some(config.cluster_addr),
        Some(config.admin_addr),
        config.redis_addr,
    ];
    if addresses.into_iter().flatten().any(|address| {
        !address.ip().is_loopback() || address.port() == 0 || !all_addrs.insert(address)
    }) {
        return Err(ControlPlaneError::Capability(
            "node configs require unique non-zero loopback client/cluster/admin/RESP endpoints"
                .to_owned(),
        ));
    }
    let storage = fs::canonicalize(&config.storage_dir).map_err(|error| {
        ControlPlaneError::Capability(format!(
            "unable to canonicalize node storage {}: {error}",
            config.storage_dir.display()
        ))
    })?;
    if storage != config.storage_dir
        || !fs::metadata(&storage).is_ok_and(|metadata| metadata.is_dir())
    {
        return Err(ControlPlaneError::Capability(
            "node storage path must be an existing canonical directory".to_owned(),
        ));
    }
    Ok(())
}

fn validate_live_node_process(
    node: &DaemonNodeProcessReceipt,
    server: &PrebuiltServerBinaryReceipt,
) -> Result<(), ControlPlaneError> {
    let observed = observed_process_executable(node.pid)?;
    if observed != server.canonical_path
        || observed != node.observed_executable_path
        || sha256_file(&observed)? != node.observed_executable_sha256
    {
        return Err(ControlPlaneError::Capability(format!(
            "PID {} is not running the exact attested prebuilt server binary",
            node.pid
        )));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn observed_process_executable(pid: u32) -> Result<PathBuf, ControlPlaneError> {
    fs::canonicalize(format!("/proc/{pid}/exe")).map_err(|error| {
        ControlPlaneError::Capability(format!(
            "unable to observe executable for PID {pid}: {error}"
        ))
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlPlaneSource {
    Live,
    Modeled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminStatusObservation {
    pub source: ControlPlaneSource,
    pub leader: Option<String>,
    pub term: u64,
    pub epoch: u64,
    pub quorum_ok: bool,
    pub members: u32,
    pub member_ids: Vec<String>,
    pub voters: u32,
    pub voter_ids: Vec<u64>,
    pub reshard_phase: String,
    pub draining: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewMemberObservation {
    pub node_id: String,
    pub role: String,
    pub reachable: bool,
    pub reachability: String,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewLeaderObservation {
    pub node_id: String,
    pub term: u64,
    pub epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewPartitionObservation {
    pub under_replicated: u64,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewConsistencyLevelObservation {
    pub level: String,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewConsistencyObservation {
    pub configured_default: Option<String>,
    pub op_counts_by_level: Vec<OverviewConsistencyLevelObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewLifecycleObservation {
    pub reshard_phase: String,
    pub upgrade_phase: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClusterOverviewObservation {
    pub source: ControlPlaneSource,
    pub members: Vec<OverviewMemberObservation>,
    pub leader: Option<OverviewLeaderObservation>,
    pub partitions: OverviewPartitionObservation,
    pub consistency: OverviewConsistencyObservation,
    pub backup_age_seconds: Option<u64>,
    pub lifecycle: OverviewLifecycleObservation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicControlPlaneSnapshot {
    pub endpoint: ControlPlaneEndpoint,
    pub admin_status: AdminStatusObservation,
    pub cluster_overview: ClusterOverviewObservation,
}

impl PublicControlPlaneSnapshot {
    pub fn target_role(&self) -> Result<NodeRole, ControlPlaneError> {
        let leader = self.admin_status.leader.as_deref().ok_or_else(|| {
            ControlPlaneError::Evidence("live W4A snapshot has no elected leader".to_owned())
        })?;
        Ok(if leader == self.endpoint.node_id {
            NodeRole::Leader
        } else {
            NodeRole::Follower
        })
    }
}

impl ProbedControlPlaneCapability {
    pub fn target_role(&self, node_id: &str) -> Result<NodeRole, ControlPlaneError> {
        self.baseline
            .iter()
            .find(|snapshot| snapshot.endpoint.node_id == node_id)
            .ok_or_else(|| {
                ControlPlaneError::Capability(format!(
                    "target node {node_id:?} is absent from the live capability receipt"
                ))
            })?
            .target_role()
    }

    pub fn receipt_sha256(&self) -> &str {
        &self.attestation.receipt.receipt_sha256
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ControlPlaneTargetCounters {
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub admin_status_requests: u64,
    pub cluster_overview_requests: u64,
    pub request_network_bytes: u64,
    pub response_network_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SteadyRepeatWireEvidence {
    pub offered_rate_per_second: u64,
    pub repeat_index: u32,
    pub counter_scope: String,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub admin_status_requests: u64,
    pub cluster_overview_requests: u64,
    pub request_network_bytes: u64,
    pub response_network_bytes: u64,
}

impl SteadyRepeatWireEvidence {
    fn from_counters(
        offered_rate_per_second: u64,
        repeat_index: u32,
        counters: ControlPlaneTargetCounters,
    ) -> Self {
        Self {
            offered_rate_per_second,
            repeat_index,
            counter_scope: STEADY_COUNTER_SCOPE.to_owned(),
            successful_requests: counters.successful_requests,
            failed_requests: counters.failed_requests,
            admin_status_requests: counters.admin_status_requests,
            cluster_overview_requests: counters.cluster_overview_requests,
            request_network_bytes: counters.request_network_bytes,
            response_network_bytes: counters.response_network_bytes,
        }
    }
}

trait ControlPlaneCounterSource: Target {
    fn take_repeat_counters(&self) -> ControlPlaneTargetCounters;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SteadyReadEvidence {
    pub target_node_id: String,
    pub target_node_role: NodeRole,
    pub complete_endpoint_set: Vec<ControlPlaneEndpoint>,
    pub start: PublicControlPlaneSnapshot,
    pub end: PublicControlPlaneSnapshot,
    pub criteria: SustainabilityCriteria,
    pub knee: KneeResult,
    pub repeat_wire_evidence: Vec<SteadyRepeatWireEvidence>,
    pub role_changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminDrainInvocationReceipt {
    pub target_node_id: String,
    pub path: String,
    pub action: String,
    pub outcome: String,
    pub started_with: u64,
    pub remaining: u64,
    pub timed_out: bool,
    pub response_sha256: String,
    pub request_network_bytes: u64,
    pub response_network_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonAddActionPayload {
    pub receipt_kind: String,
    pub provisioner: String,
    pub authority_node_id: String,
    pub target_node_id: String,
    pub outcome: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonAddInvocationReceipt {
    pub canonical_action_receipt_path: PathBuf,
    pub action_receipt_sha256: String,
    pub payload: DaemonAddActionPayload,
    pub target_process: DaemonNodeProcessReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "receipt", rename_all = "snake_case")]
pub enum MembershipActionReceipt {
    DaemonAdd(Box<DaemonAddInvocationReceipt>),
    AdminDrain(AdminDrainInvocationReceipt),
}

impl MembershipActionReceipt {
    pub fn action(&self) -> MembershipAction {
        match self {
            Self::DaemonAdd(_) => MembershipAction::Add,
            Self::AdminDrain(_) => MembershipAction::Drain,
        }
    }

    pub fn target_node_id(&self) -> &str {
        match self {
            Self::DaemonAdd(receipt) => &receipt.payload.target_node_id,
            Self::AdminDrain(receipt) => &receipt.target_node_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MembershipEventWireEvidence {
    pub counter_scope: String,
    pub action_request_network_bytes: u64,
    pub action_response_network_bytes: u64,
    pub successful_snapshot_request_network_bytes: u64,
    pub successful_snapshot_response_network_bytes: u64,
    pub observation_rounds: u64,
    pub snapshot_probe_attempts: u64,
    pub complete_snapshot_observations: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MembershipEventEvidence {
    pub action: MembershipAction,
    pub target_node_id: String,
    pub authority_node_id: String,
    pub action_receipt: MembershipActionReceipt,
    pub pre_transition_snapshots: Vec<PublicControlPlaneSnapshot>,
    pub authority_commit_snapshot: PublicControlPlaneSnapshot,
    pub post_transition_snapshots: Vec<PublicControlPlaneSnapshot>,
    pub prior_epoch: u64,
    pub new_epoch: u64,
    pub timer_kind: String,
    pub commit_latency_nanos: u64,
    pub convergence_latency_nanos: u64,
    pub wire: MembershipEventWireEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessLogReceipt {
    pub canonical_path: PathBuf,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonNodeLifecycleEvidence {
    pub node_id: String,
    pub pid: u32,
    pub kill_requested: bool,
    pub wait_completed: bool,
    pub process_no_longer_running: bool,
    pub exit_status: String,
    pub stdout_log: ProcessLogReceipt,
    pub stderr_log: ProcessLogReceipt,
    pub server_binary_path_after: PathBuf,
    pub server_binary_sha256_after: String,
    pub node_config_path_after: PathBuf,
    pub node_config_sha256_after: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlaneLifecycleReceiptPayload {
    pub receipt_kind: String,
    pub receipt_source: DaemonReceiptSource,
    pub capability_receipt_sha256: String,
    pub nodes: Vec<DaemonNodeLifecycleEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlaneLifecycleReceipt {
    pub payload: ControlPlaneLifecycleReceiptPayload,
    pub receipt_sha256: String,
}

impl ControlPlaneLifecycleReceipt {
    pub fn seal(
        mut payload: ControlPlaneLifecycleReceiptPayload,
    ) -> Result<Self, ControlPlaneError> {
        payload
            .nodes
            .sort_by(|left, right| left.node_id.cmp(&right.node_id));
        let receipt_sha256 = canonical_digest(&payload)?;
        Ok(Self {
            payload,
            receipt_sha256,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlaneReport {
    pub schema_version: u32,
    pub scenario_id: String,
    pub evidence_class: String,
    pub execution_mode: String,
    pub capability_receipt_sha256: String,
    pub capability: ControlPlaneCapabilityAttestation,
    pub node_count: u8,
    pub capacity_scope: String,
    pub aggregate_cluster_capacity: bool,
    pub product_data_plane: bool,
    pub live_reshard_measured: bool,
    pub steady_reads: Vec<SteadyReadEvidence>,
    pub membership_events: Vec<MembershipEventEvidence>,
    pub lifecycle: ControlPlaneLifecycleReceipt,
    pub deferred_claims: Vec<String>,
}

impl ControlPlaneReport {
    pub fn validate(
        &self,
        scenario: &ControlPlaneScenario,
        capability: &ProbedControlPlaneCapability,
    ) -> Result<(), ControlPlaneError> {
        scenario.validate()?;
        if self.schema_version != CONTROL_PLANE_REPORT_VERSION
            || self.scenario_id != scenario.scenario_id
            || self.evidence_class != CONTROL_PLANE_EVIDENCE_CLASS
            || self.execution_mode != CONTROL_PLANE_EXECUTION_MODE
            || self.capability_receipt_sha256 != capability.receipt_sha256()
            || self.capability.receipt_sha256 != self.capability_receipt_sha256
            || self.capability != capability.attestation.receipt
            || self.node_count != capability.attestation.node_count
            || self.capacity_scope != "per-selected-admin-endpoint-and-role-no-sum"
            || self.aggregate_cluster_capacity
            || self.product_data_plane
            || self.live_reshard_measured
            || self.deferred_claims != ["live-rebalance-reshard-performance".to_owned()]
        {
            return Err(ControlPlaneError::Boundary(
                "W4A report identity/capability is incomplete or overclaims data-plane, summed cluster, or reshard capacity".to_owned(),
            ));
        }
        let expected = normalized_endpoints(&capability.attestation.endpoints)?;
        if self.steady_reads.len() != 2 {
            return Err(ControlPlaneError::Evidence(
                "one W4A artifact must retain exactly one leader knee and one follower knee"
                    .to_owned(),
            ));
        }
        let mut roles = BTreeSet::new();
        let mut targets = BTreeSet::new();
        for evidence in &self.steady_reads {
            validate_steady_read(evidence, scenario, &expected, self.node_count)?;
            roles.insert(evidence.target_node_role);
            targets.insert(evidence.target_node_id.as_str());
        }
        if roles != BTreeSet::from([NodeRole::Leader, NodeRole::Follower]) || targets.len() != 2 {
            return Err(ControlPlaneError::Evidence(
                "W4A role observations must be separate knees on distinct exact endpoints"
                    .to_owned(),
            ));
        }
        let actions = self
            .membership_events
            .iter()
            .map(|event| event.action)
            .collect::<BTreeSet<_>>();
        if self.membership_events.len() != 2
            || actions != BTreeSet::from([MembershipAction::Add, MembershipAction::Drain])
        {
            return Err(ControlPlaneError::Evidence(
                "W4A report must retain one receipt-bound add and one receipt-bound drain event"
                    .to_owned(),
            ));
        }
        for event in &self.membership_events {
            validate_membership_event(event, scenario, capability)?;
        }
        validate_lifecycle_receipt(&self.lifecycle, capability)?;
        Ok(())
    }

    /// Validate a persisted W4A artifact after its launch processes have been
    /// reaped. The embedded capability is the complete typed predecessor;
    /// callers cannot reconstruct it from a digest or substitute a live run.
    pub fn validate_archived(
        &self,
        scenario: &ControlPlaneScenario,
    ) -> Result<ValidatedControlPlaneCapability, ControlPlaneError> {
        let archived = self.capability.clone().require_archived(scenario)?;
        let capability = ProbedControlPlaneCapability {
            attestation: archived.clone(),
            baseline: Vec::new(),
        };
        self.validate(scenario, &capability)?;
        Ok(archived)
    }
}

fn validate_steady_read(
    evidence: &SteadyReadEvidence,
    scenario: &ControlPlaneScenario,
    expected: &[ControlPlaneEndpoint],
    node_count: u8,
) -> Result<(), ControlPlaneError> {
    if evidence.role_changed
        || !portable_identifier(&evidence.target_node_id)
        || normalized_endpoints(&evidence.complete_endpoint_set)? != expected
        || evidence.start.endpoint.node_id != evidence.target_node_id
        || evidence.end.endpoint.node_id != evidence.target_node_id
        || evidence.criteria != scenario.sustainability_criteria()
        || evidence.criteria.p99_slo_us != scenario.read_only.latency_slo_micros
    {
        return Err(ControlPlaneError::Evidence(
            "W4A steady evidence crossed an endpoint/role boundary or changed the exact W0 criteria"
                .to_owned(),
        ));
    }
    validate_public_snapshot(&evidence.start, expected, node_count)?;
    validate_public_snapshot(&evidence.end, expected, node_count)?;
    if evidence.start.target_role()? != evidence.target_node_role
        || evidence.end.target_role()? != evidence.target_node_role
        || evidence.start.admin_status.leader != evidence.end.admin_status.leader
        || evidence.start.admin_status.term != evidence.end.admin_status.term
        || evidence.start.admin_status.epoch != evidence.end.admin_status.epoch
    {
        return Err(ControlPlaneError::Evidence(
            "leader/role/term/epoch changed inside an ordinary W4A window; classify it under W5"
                .to_owned(),
        ));
    }
    let knee_problems = evidence.criteria.knee_validation_problems(&evidence.knee);
    if !knee_problems.is_empty() || evidence.knee.sustainable_rate_per_second.is_none() {
        return Err(ControlPlaneError::Evidence(format!(
            "W4A knee does not recompute from raw W0 repeats and the complete predicate: {}",
            knee_problems.join("; ")
        )));
    }
    let evaluated_rates = evidence
        .knee
        .evaluated
        .iter()
        .map(|point| point.sample.offered_rate_per_second)
        .collect::<Vec<_>>();
    let expected_rates = scenario
        .read_only
        .offered_rates_per_second
        .iter()
        .map(|rate| *rate as f64)
        .collect::<Vec<_>>();
    if evaluated_rates != expected_rates {
        return Err(ControlPlaneError::Evidence(
            "W4A knee must evaluate the exact committed open-loop rate schedule".to_owned(),
        ));
    }
    let expected_wire_rows = evidence.knee.evaluated.len() * CONTROL_PLANE_REPEATS as usize;
    if evidence.repeat_wire_evidence.len() != expected_wire_rows {
        return Err(ControlPlaneError::Evidence(
            "W4A wire evidence must contain one reset-scoped counter row per raw repeat".to_owned(),
        ));
    }
    let mut wire_keys = BTreeSet::new();
    for point in &evidence.knee.evaluated {
        if point.repeats.len() != CONTROL_PLANE_REPEATS as usize {
            return Err(ControlPlaneError::Evidence(
                "W4A rate point does not contain exactly five raw repeats".to_owned(),
            ));
        }
        let offered_rate = point.sample.offered_rate_per_second as u64;
        for (index, repeat) in point.repeats.iter().enumerate() {
            if repeat.reset_state_digest.is_empty()
                || repeat.preloaded_state_digest.is_empty()
                || repeat.state_digest.is_empty()
                || repeat.phase.reset_operations != 1
                || repeat.phase.preload_operations != scenario.read_only.preload_operations
                || repeat.phase.warmup_operations != scenario.read_only.warmup_operations
                || repeat.phase.steady_operations != scenario.read_only.steady_operations
                || repeat.phase.warmup_samples_in_steady_histogram != 0
            {
                return Err(ControlPlaneError::Evidence(
                    "W4A raw repeat does not preserve reset/preload/warmup/steady phase accounting"
                        .to_owned(),
                ));
            }
            let repeat_index = u32::try_from(index + 1).expect("five repeats fit u32");
            let wire = evidence
                .repeat_wire_evidence
                .iter()
                .find(|wire| {
                    wire.offered_rate_per_second == offered_rate
                        && wire.repeat_index == repeat_index
                })
                .ok_or_else(|| {
                    ControlPlaneError::Evidence(format!(
                        "missing W4A wire counters for rate {offered_rate} repeat {repeat_index}"
                    ))
                })?;
            if !wire_keys.insert((offered_rate, repeat_index)) {
                return Err(ControlPlaneError::Evidence(
                    "duplicate W4A wire-counter rate/repeat identity".to_owned(),
                ));
            }
            validate_repeat_wire(wire, repeat)?;
        }
    }
    Ok(())
}

fn validate_repeat_wire(
    wire: &SteadyRepeatWireEvidence,
    repeat: &crate::knee::RepeatEvidence,
) -> Result<(), ControlPlaneError> {
    let observation = &repeat.steady;
    let steady_failed = observation
        .errors
        .checked_add(observation.timeouts)
        .and_then(|value| value.checked_add(observation.rejections));
    let warmup_failed = repeat
        .phase
        .warmup_errors
        .checked_add(repeat.phase.warmup_timeouts)
        .and_then(|value| value.checked_add(repeat.phase.warmup_rejections));
    let failed = steady_failed
        .and_then(|steady| warmup_failed.and_then(|warmup| steady.checked_add(warmup)));
    let successful = repeat
        .phase
        .warmup_successes
        .checked_add(observation.successes);
    let completed = repeat
        .phase
        .warmup_operations
        .checked_add(observation.completed);
    let paths = wire
        .admin_status_requests
        .checked_add(wire.cluster_overview_requests);
    if wire.counter_scope != STEADY_COUNTER_SCOPE
        || wire.repeat_index == 0
        || Some(wire.successful_requests) != successful
        || failed != Some(wire.failed_requests)
        || paths != completed
        || wire.successful_requests.checked_add(wire.failed_requests) != completed
        || wire
            .admin_status_requests
            .abs_diff(wire.cluster_overview_requests)
            > 1
        || wire.request_network_bytes == 0
        || wire.response_network_bytes == 0
    {
        return Err(ControlPlaneError::Evidence(
            "W4A steady wire counters do not conserve exact raw outcomes and alternating paths"
                .to_owned(),
        ));
    }
    Ok(())
}

fn validate_membership_event(
    event: &MembershipEventEvidence,
    scenario: &ControlPlaneScenario,
    capability: &ProbedControlPlaneCapability,
) -> Result<(), ControlPlaneError> {
    if event.action != event.action_receipt.action()
        || event.target_node_id != event.action_receipt.target_node_id()
        || !portable_identifier(&event.authority_node_id)
        || event.timer_kind != "monotonic-elapsed"
    {
        return Err(ControlPlaneError::Evidence(
            "W4A membership event lacks an exact action or monotonic timer identity".to_owned(),
        ));
    }
    validate_membership_event_timing(
        event.commit_latency_nanos,
        event.convergence_latency_nanos,
        event.prior_epoch,
        event.new_epoch,
        scenario,
    )?;
    let (before, prior_leader, prior_term, prior_epoch) =
        validate_snapshot_set(&event.pre_transition_snapshots)?;
    let (after, new_leader, new_term, new_epoch) =
        validate_snapshot_set(&event.post_transition_snapshots)?;
    if prior_leader != event.authority_node_id
        || new_leader != event.authority_node_id
        || new_term < prior_term
        || prior_epoch != event.prior_epoch
        || new_epoch != event.new_epoch
    {
        return Err(ControlPlaneError::Evidence(
            "W4A event authority/prior/new epoch is not derived from complete live snapshots"
                .to_owned(),
        ));
    }
    validate_public_snapshot(
        &event.authority_commit_snapshot,
        &after,
        u8::try_from(after.len())
            .map_err(|_| ControlPlaneError::Evidence("node count exceeds u8".to_owned()))?,
    )?;
    if event.authority_commit_snapshot.endpoint.node_id != event.authority_node_id
        || event.authority_commit_snapshot.admin_status.epoch != event.new_epoch
        || event.authority_commit_snapshot.admin_status.term != new_term
    {
        return Err(ControlPlaneError::Evidence(
            "W4A authority commit snapshot is not the exact post-action committed view".to_owned(),
        ));
    }
    validate_exact_membership_diff(event, &before, &after, capability)?;
    validate_action_receipt(event, capability)?;
    if event.wire.counter_scope != EVENT_COUNTER_SCOPE
        || event.wire.observation_rounds == 0
        || event.wire.snapshot_probe_attempts == 0
        || event.wire.complete_snapshot_observations == 0
        || event.wire.successful_snapshot_request_network_bytes == 0
        || event.wire.successful_snapshot_response_network_bytes == 0
    {
        return Err(ControlPlaneError::Evidence(
            "W4A event counters must be honest loadgen-observed wire/timer evidence".to_owned(),
        ));
    }
    match &event.action_receipt {
        MembershipActionReceipt::AdminDrain(receipt) => {
            if event.wire.action_request_network_bytes != receipt.request_network_bytes
                || event.wire.action_response_network_bytes != receipt.response_network_bytes
            {
                return Err(ControlPlaneError::Evidence(
                    "drain wire counters do not match the actual admin action receipt".to_owned(),
                ));
            }
        }
        MembershipActionReceipt::DaemonAdd(_) => {
            if event.wire.action_request_network_bytes != 0
                || event.wire.action_response_network_bytes != 0
            {
                return Err(ControlPlaneError::Evidence(
                    "process-harness add must not fabricate admin HTTP action bytes".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_snapshot_set(
    snapshots: &[PublicControlPlaneSnapshot],
) -> Result<(Vec<ControlPlaneEndpoint>, String, u64, u64), ControlPlaneError> {
    if snapshots.is_empty() {
        return Err(ControlPlaneError::Evidence(
            "W4A membership snapshot set is empty".to_owned(),
        ));
    }
    let endpoints = normalized_endpoints(
        &snapshots
            .iter()
            .map(|snapshot| snapshot.endpoint.clone())
            .collect::<Vec<_>>(),
    )?;
    let node_count = u8::try_from(endpoints.len())
        .map_err(|_| ControlPlaneError::Evidence("node count exceeds u8".to_owned()))?;
    if snapshots.len() != endpoints.len() {
        return Err(ControlPlaneError::Evidence(
            "W4A snapshot set does not contain exactly one observation per endpoint".to_owned(),
        ));
    }
    for snapshot in snapshots {
        validate_public_snapshot(snapshot, &endpoints, node_count)?;
    }
    let leaders = snapshots
        .iter()
        .filter_map(|snapshot| snapshot.admin_status.leader.clone())
        .collect::<BTreeSet<_>>();
    let terms = snapshots
        .iter()
        .map(|snapshot| snapshot.admin_status.term)
        .collect::<BTreeSet<_>>();
    let epochs = snapshots
        .iter()
        .map(|snapshot| snapshot.admin_status.epoch)
        .collect::<BTreeSet<_>>();
    if leaders.len() != 1 || terms.len() != 1 || epochs.len() != 1 {
        return Err(ControlPlaneError::Evidence(
            "complete W4A snapshot set disagrees on leader, term, or epoch".to_owned(),
        ));
    }
    Ok((
        endpoints,
        leaders.into_iter().next().expect("one leader"),
        terms.into_iter().next().expect("one term"),
        epochs.into_iter().next().expect("one epoch"),
    ))
}

fn validate_exact_membership_diff(
    event: &MembershipEventEvidence,
    before: &[ControlPlaneEndpoint],
    after: &[ControlPlaneEndpoint],
    capability: &ProbedControlPlaneCapability,
) -> Result<(), ControlPlaneError> {
    let expected = normalized_endpoints(&capability.attestation.endpoints)?;
    let (small, large) = match event.action {
        MembershipAction::Add => (before, after),
        MembershipAction::Drain => (after, before),
    };
    if large != expected
        || large.len() != small.len() + 1
        || ![3_usize, 5, 7].contains(&large.len())
    {
        return Err(ControlPlaneError::Evidence(
            "W4A event must add/drain exactly one member around the attested full 3/5/7 set"
                .to_owned(),
        ));
    }
    let small_ids = small
        .iter()
        .map(|endpoint| endpoint.node_id.as_str())
        .collect::<BTreeSet<_>>();
    let large_ids = large
        .iter()
        .map(|endpoint| endpoint.node_id.as_str())
        .collect::<BTreeSet<_>>();
    let difference = large_ids
        .difference(&small_ids)
        .copied()
        .collect::<Vec<_>>();
    if difference != [event.target_node_id.as_str()]
        || !small.iter().all(|small_endpoint| {
            large
                .iter()
                .any(|large_endpoint| large_endpoint == small_endpoint)
        })
    {
        return Err(ControlPlaneError::Evidence(
            "W4A membership snapshots changed something other than the exact target identity/endpoint"
                .to_owned(),
        ));
    }
    Ok(())
}

fn validate_action_receipt(
    event: &MembershipEventEvidence,
    capability: &ProbedControlPlaneCapability,
) -> Result<(), ControlPlaneError> {
    match &event.action_receipt {
        MembershipActionReceipt::AdminDrain(receipt) => {
            if receipt.path != ADMIN_DRAIN_PATH
                || receipt.action != "drain"
                || receipt.outcome != "accepted"
                || receipt.remaining != 0
                || receipt.timed_out
                || !is_sha256(&receipt.response_sha256)
                || receipt.request_network_bytes == 0
                || receipt.response_network_bytes == 0
                || !capability
                    .attestation
                    .endpoints
                    .iter()
                    .any(|endpoint| endpoint.node_id == receipt.target_node_id)
            {
                return Err(ControlPlaneError::Evidence(
                    "W4A drain event is not bound to an accepted real admin response".to_owned(),
                ));
            }
        }
        MembershipActionReceipt::DaemonAdd(receipt) => {
            let path =
                fs::canonicalize(&receipt.canonical_action_receipt_path).map_err(|error| {
                    ControlPlaneError::Evidence(format!(
                        "unable to canonicalize add action receipt {}: {error}",
                        receipt.canonical_action_receipt_path.display()
                    ))
                })?;
            let bytes = fs::read(&path).map_err(|error| {
                ControlPlaneError::Evidence(format!(
                    "unable to read add action receipt {}: {error}",
                    path.display()
                ))
            })?;
            let decoded: DaemonAddActionPayload =
                serde_json::from_slice(&bytes).map_err(|error| {
                    ControlPlaneError::Evidence(format!(
                        "add action receipt {} is not strict typed JSON: {error}",
                        path.display()
                    ))
                })?;
            let attested_target = capability
                .attestation
                .nodes
                .iter()
                .find(|node| node.node_id == event.target_node_id);
            if path != receipt.canonical_action_receipt_path
                || !is_sha256(&receipt.action_receipt_sha256)
                || sha256_bytes(&bytes) != receipt.action_receipt_sha256
                || decoded != receipt.payload
                || receipt.payload.receipt_kind != "hydracache-daemon-add-action-v1"
                || receipt.payload.provisioner != DAEMON_CLUSTER_PROVISIONER
                || receipt.payload.authority_node_id != event.authority_node_id
                || receipt.payload.target_node_id != event.target_node_id
                || receipt.payload.outcome != "process-started-and-admission-requested"
                || attested_target != Some(&receipt.target_process)
            {
                return Err(ControlPlaneError::Evidence(
                    "W4A add event is not bound to the exact process-harness action artifact and target process"
                        .to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_lifecycle_receipt(
    lifecycle: &ControlPlaneLifecycleReceipt,
    capability: &ProbedControlPlaneCapability,
) -> Result<(), ControlPlaneError> {
    if lifecycle.receipt_sha256 != canonical_digest(&lifecycle.payload)?
        || lifecycle.payload.receipt_kind != "hydracache-daemon-cluster-lifecycle-v1"
        || lifecycle.payload.receipt_source != DaemonReceiptSource::ObservedProcessHarness
        || lifecycle.payload.capability_receipt_sha256 != capability.receipt_sha256()
        || lifecycle.payload.nodes.len() != capability.attestation.nodes.len()
    {
        return Err(ControlPlaneError::Evidence(
            "W4A lifecycle receipt is unsealed or not bound to the exact capability".to_owned(),
        ));
    }
    let mut ids = BTreeSet::new();
    for node in &lifecycle.payload.nodes {
        let launch = capability
            .attestation
            .nodes
            .iter()
            .find(|candidate| candidate.node_id == node.node_id)
            .ok_or_else(|| {
                ControlPlaneError::Evidence(
                    "lifecycle receipt contains a node absent from the launch receipt".to_owned(),
                )
            })?;
        if !ids.insert(node.node_id.as_str())
            || node.pid != launch.pid
            || !node.kill_requested
            || !node.wait_completed
            || !node.process_no_longer_running
            || node.exit_status.trim().is_empty()
            || process_is_alive(node.pid)
            || node.server_binary_path_after != capability.attestation.server_binary.canonical_path
            || node.server_binary_sha256_after != capability.attestation.server_binary.sha256
            || sha256_file(&node.server_binary_path_after)? != node.server_binary_sha256_after
            || node.node_config_path_after != launch.config.canonical_path
            || node.node_config_sha256_after != launch.config.sha256
            || sha256_file(&node.node_config_path_after)? != node.node_config_sha256_after
        {
            return Err(ControlPlaneError::Evidence(
                "W4A lifecycle must prove kill+wait, PID exit, and post-run binary/config identity for every daemon"
                    .to_owned(),
            ));
        }
        validate_log_receipt(&node.stdout_log)?;
        validate_log_receipt(&node.stderr_log)?;
        if node.stdout_log.canonical_path == node.stderr_log.canonical_path {
            return Err(ControlPlaneError::Evidence(
                "daemon stdout/stderr lifecycle receipts must be distinct artifacts".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_log_receipt(receipt: &ProcessLogReceipt) -> Result<(), ControlPlaneError> {
    let path = fs::canonicalize(&receipt.canonical_path).map_err(|error| {
        ControlPlaneError::Evidence(format!(
            "unable to canonicalize process log {}: {error}",
            receipt.canonical_path.display()
        ))
    })?;
    let metadata = fs::metadata(&path).map_err(|error| {
        ControlPlaneError::Evidence(format!(
            "unable to stat process log {}: {error}",
            path.display()
        ))
    })?;
    if path != receipt.canonical_path
        || !metadata.is_file()
        || metadata.len() != receipt.bytes
        || !is_sha256(&receipt.sha256)
        || sha256_file(&path)? != receipt.sha256
    {
        return Err(ControlPlaneError::Evidence(
            "process log path/length/SHA receipt does not match the post-run artifact".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn process_is_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(target_os = "windows")]
fn process_is_alive(pid: u32) -> bool {
    Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!("if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ exit 0 }} else {{ exit 1 }}"),
        ])
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn process_is_alive(_pid: u32) -> bool {
    true
}

fn validate_public_snapshot(
    snapshot: &PublicControlPlaneSnapshot,
    expected: &[ControlPlaneEndpoint],
    node_count: u8,
) -> Result<(), ControlPlaneError> {
    let expected = normalized_endpoints(expected)?;
    if expected.len() != usize::from(node_count)
        || !expected
            .iter()
            .any(|endpoint| endpoint == &snapshot.endpoint)
        || snapshot.admin_status.source != ControlPlaneSource::Live
        || snapshot.cluster_overview.source != ControlPlaneSource::Live
        || !snapshot.admin_status.quorum_ok
        || snapshot.admin_status.draining
        || snapshot.admin_status.term == 0
        || snapshot.admin_status.reshard_phase != "idle"
        || snapshot.cluster_overview.lifecycle.reshard_phase != "idle"
        || snapshot.cluster_overview.lifecycle.upgrade_phase != "idle"
        || snapshot.cluster_overview.partitions.under_replicated != 0
        || snapshot.admin_status.members != u32::from(node_count)
        || snapshot.admin_status.member_ids.len() != usize::from(node_count)
        || snapshot.admin_status.voters != u32::from(node_count)
        || snapshot.admin_status.voter_ids.len() != usize::from(node_count)
        || snapshot.cluster_overview.members.len() != usize::from(node_count)
    {
        return Err(ControlPlaneError::Evidence(
            "W4A snapshot is not an exact live, quorate, healthy, idle complete view".to_owned(),
        ));
    }
    let expected_ids = expected
        .iter()
        .map(|endpoint| endpoint.node_id.as_str())
        .collect::<BTreeSet<_>>();
    let status_ids = snapshot
        .admin_status
        .member_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let overview_ids = snapshot
        .cluster_overview
        .members
        .iter()
        .map(|member| member.node_id.as_str())
        .collect::<BTreeSet<_>>();
    let all_members_exact = snapshot.cluster_overview.members.iter().all(|member| {
        portable_identifier(&member.node_id)
            && member.role == "member"
            && member.reachable
            && member.reachability == "reachable"
            && member.generation > 0
    });
    let voter_ids = snapshot
        .admin_status
        .voter_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let admin_leader = snapshot.admin_status.leader.as_deref();
    let overview_leader = snapshot.cluster_overview.leader.as_ref();
    if expected_ids != status_ids
        || expected_ids != overview_ids
        || status_ids.len() != usize::from(node_count)
        || overview_ids.len() != usize::from(node_count)
        || !all_members_exact
        || voter_ids.len() != usize::from(node_count)
        || voter_ids.contains(&0)
        || admin_leader.is_none()
        || overview_leader.map(|leader| leader.node_id.as_str()) != admin_leader
        || overview_leader.map(|leader| (leader.term, leader.epoch))
            != Some((snapshot.admin_status.term, snapshot.admin_status.epoch))
        || !admin_leader.is_some_and(|leader| expected_ids.contains(leader))
    {
        return Err(ControlPlaneError::Evidence(
            "W4A strict public documents disagree on exact members, voters, leader, term, or epoch"
                .to_owned(),
        ));
    }
    let mut consistency_levels = BTreeSet::new();
    if snapshot
        .cluster_overview
        .consistency
        .op_counts_by_level
        .iter()
        .any(|entry| !portable_identifier(&entry.level) || !consistency_levels.insert(&entry.level))
    {
        return Err(ControlPlaneError::Evidence(
            "W4A overview contains duplicate/invalid consistency counter identities".to_owned(),
        ));
    }
    Ok(())
}

fn normalized_endpoints(
    endpoints: &[ControlPlaneEndpoint],
) -> Result<Vec<ControlPlaneEndpoint>, ControlPlaneError> {
    let mut normalized = endpoints.to_vec();
    normalized.sort_by(|left, right| left.node_id.cmp(&right.node_id));
    let unique_ids = normalized
        .iter()
        .map(|endpoint| endpoint.node_id.as_str())
        .collect::<BTreeSet<_>>();
    let unique_addrs = normalized
        .iter()
        .map(|endpoint| endpoint.admin_addr)
        .collect::<BTreeSet<_>>();
    if normalized.is_empty()
        || unique_ids.len() != normalized.len()
        || unique_addrs.len() != normalized.len()
        || normalized.iter().any(|endpoint| {
            !portable_identifier(&endpoint.node_id)
                || !endpoint.admin_addr.ip().is_loopback()
                || endpoint.admin_addr.port() == 0
        })
    {
        return Err(ControlPlaneError::Evidence(
            "W4A endpoint set is empty or contains duplicate/invalid identities".to_owned(),
        ));
    }
    Ok(normalized)
}

#[derive(Debug, Error)]
pub enum ControlPlaneError {
    #[error("W4A contract rejected: {0}")]
    Contract(String),
    #[error("W4A surface boundary rejected: {0}")]
    Boundary(String),
    #[error("W4A capability rejected: {0}")]
    Capability(String),
    #[error("W4A evidence rejected: {0}")]
    Evidence(String),
    #[error("W4A admin HTTP probe failed: {0}")]
    Probe(String),
    #[error("W4A admin HTTP {path} timed out after {timeout_millis}ms on {node_id}")]
    Timeout {
        path: &'static str,
        node_id: String,
        timeout_millis: u128,
    },
}

pub fn bounded_timeout(millis: u64) -> Result<Duration, ControlPlaneError> {
    if !(1..=60_000).contains(&millis) {
        return Err(ControlPlaneError::Contract(
            "W4A admin HTTP timeout must be within 1..=60000 milliseconds".to_owned(),
        ));
    }
    Ok(Duration::from_millis(millis))
}

/// Probe both strict public documents on every OS-verified daemon receipt.
pub async fn probe_control_plane_capability(
    attestation: ValidatedControlPlaneCapability,
    timeout: Duration,
) -> Result<ProbedControlPlaneCapability, ControlPlaneError> {
    if timeout.is_zero() || timeout > Duration::from_secs(60) {
        return Err(ControlPlaneError::Capability(
            "W4A live capability probe timeout must be within (0, 60s]".to_owned(),
        ));
    }
    let expected = normalized_endpoints(&attestation.endpoints)?;
    let mut baseline = Vec::with_capacity(expected.len());
    for endpoint in &expected {
        let receipt = probe_public_snapshot_with_wire(endpoint.clone(), timeout).await?;
        validate_public_snapshot(&receipt.snapshot, &expected, attestation.node_count)?;
        baseline.push(receipt.snapshot);
    }
    let (_, leader, _, _) = validate_snapshot_set(&baseline)?;
    if !expected.iter().any(|endpoint| endpoint.node_id == leader) {
        return Err(ControlPlaneError::Capability(
            "W4A live leader is absent from the complete receipt endpoint set".to_owned(),
        ));
    }
    Ok(ProbedControlPlaneCapability {
        attestation,
        baseline,
    })
}

/// Read the strict `/admin/status` and `/cluster/overview` documents from one socket.
pub async fn probe_public_snapshot(
    endpoint: ControlPlaneEndpoint,
    timeout: Duration,
) -> Result<PublicControlPlaneSnapshot, ControlPlaneError> {
    Ok(probe_public_snapshot_with_wire(endpoint, timeout)
        .await?
        .snapshot)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SnapshotProbeReceipt {
    snapshot: PublicControlPlaneSnapshot,
    request_network_bytes: u64,
    response_network_bytes: u64,
}

async fn probe_public_snapshot_with_wire(
    endpoint: ControlPlaneEndpoint,
    timeout: Duration,
) -> Result<SnapshotProbeReceipt, ControlPlaneError> {
    let admin = admin_http_json(&endpoint, ADMIN_STATUS_PATH, true, timeout).await?;
    let overview = admin_http_json(&endpoint, CLUSTER_OVERVIEW_PATH, false, timeout).await?;
    let snapshot = PublicControlPlaneSnapshot {
        endpoint,
        admin_status: serde_json::from_slice(&admin.body).map_err(|error| {
            ControlPlaneError::Probe(format!("invalid strict {ADMIN_STATUS_PATH} JSON: {error}"))
        })?,
        cluster_overview: serde_json::from_slice(&overview.body).map_err(|error| {
            ControlPlaneError::Probe(format!(
                "invalid strict {CLUSTER_OVERVIEW_PATH} JSON: {error}"
            ))
        })?,
    };
    Ok(SnapshotProbeReceipt {
        snapshot,
        request_network_bytes: admin.request_bytes.saturating_add(overview.request_bytes),
        response_network_bytes: admin.response_bytes.saturating_add(overview.response_bytes),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AdminHttpResponse {
    body: Vec<u8>,
    request_bytes: u64,
    response_bytes: u64,
}

async fn admin_http_json(
    endpoint: &ControlPlaneEndpoint,
    path: &'static str,
    admin: bool,
    timeout: Duration,
) -> Result<AdminHttpResponse, ControlPlaneError> {
    if !matches!(
        path,
        ADMIN_STATUS_PATH | CLUSTER_OVERVIEW_PATH | ADMIN_DRAIN_PATH
    ) {
        return Err(ControlPlaneError::Probe(format!(
            "refusing non-W4A admin path {path:?}"
        )));
    }
    let future = async {
        let mut stream = TcpStream::connect(endpoint.admin_addr)
            .await
            .map_err(|error| {
                ControlPlaneError::Probe(format!(
                    "connect {} ({}): {error}",
                    endpoint.node_id, endpoint.admin_addr
                ))
            })?;
        stream.set_nodelay(true).map_err(|error| {
            ControlPlaneError::Probe(format!("set TCP_NODELAY on {}: {error}", endpoint.node_id))
        })?;
        let mut request = format!(
            "GET {path} HTTP/1.1\r\nHost: {}\r\nAccept: application/json\r\nConnection: close\r\nContent-Length: 0\r\n",
            endpoint.admin_addr
        );
        if admin {
            request.push_str(
                "x-hydracache-client-id: loadgen-w4a\r\nx-hydracache-tenant: system\r\nx-hydracache-admin: true\r\n",
            );
        }
        request.push_str("\r\n");
        stream
            .write_all(request.as_bytes())
            .await
            .map_err(|error| {
                ControlPlaneError::Probe(format!("write {path} to {}: {error}", endpoint.node_id))
            })?;
        let mut raw = Vec::new();
        stream
            .take((MAX_ADMIN_RESPONSE_BYTES + 1) as u64)
            .read_to_end(&mut raw)
            .await
            .map_err(|error| {
                ControlPlaneError::Probe(format!("read {path} from {}: {error}", endpoint.node_id))
            })?;
        if raw.len() > MAX_ADMIN_RESPONSE_BYTES {
            return Err(ControlPlaneError::Probe(format!(
                "{path} response from {} exceeds {MAX_ADMIN_RESPONSE_BYTES} bytes",
                endpoint.node_id
            )));
        }
        let body = parse_http_response(path, &raw)?;
        Ok(AdminHttpResponse {
            body,
            request_bytes: request.len() as u64,
            response_bytes: raw.len() as u64,
        })
    };
    tokio::time::timeout(timeout, future)
        .await
        .map_err(|_| ControlPlaneError::Timeout {
            path,
            node_id: endpoint.node_id.clone(),
            timeout_millis: timeout.as_millis(),
        })?
}

fn parse_http_response(path: &str, raw: &[u8]) -> Result<Vec<u8>, ControlPlaneError> {
    let boundary = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| ControlPlaneError::Probe(format!("malformed HTTP response for {path}")))?;
    let head = std::str::from_utf8(&raw[..boundary]).map_err(|error| {
        ControlPlaneError::Probe(format!("non-UTF8 HTTP headers for {path}: {error}"))
    })?;
    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap_or_default();
    let status_parts = status_line.split_whitespace().collect::<Vec<_>>();
    if status_parts.len() != 3
        || !matches!(status_parts[0], "HTTP/1.0" | "HTTP/1.1")
        || status_parts[1] != "200"
    {
        return Err(ControlPlaneError::Probe(format!(
            "{path} returned invalid status line {status_line:?}"
        )));
    }
    let mut headers = BTreeMap::new();
    for line in lines {
        let (name, value) = line.split_once(':').ok_or_else(|| {
            ControlPlaneError::Probe(format!("malformed {path} HTTP header {line:?}"))
        })?;
        let name = name.trim().to_ascii_lowercase();
        if name.is_empty()
            || headers
                .insert(name.clone(), value.trim().to_owned())
                .is_some()
        {
            return Err(ControlPlaneError::Probe(format!(
                "empty/duplicate {name:?} header in {path} response"
            )));
        }
    }
    if headers
        .get("content-type")
        .is_none_or(|value| !value.to_ascii_lowercase().starts_with("application/json"))
    {
        return Err(ControlPlaneError::Probe(format!(
            "{path} response is not explicitly application/json"
        )));
    }
    let chunked = headers
        .get("transfer-encoding")
        .is_some_and(|value| value.eq_ignore_ascii_case("chunked"));
    if headers.contains_key("transfer-encoding") && !chunked {
        return Err(ControlPlaneError::Probe(format!(
            "unsupported transfer-encoding for {path}"
        )));
    }
    if chunked && headers.contains_key("content-length") {
        return Err(ControlPlaneError::Probe(format!(
            "ambiguous content-length plus chunked framing for {path}"
        )));
    }
    let encoded_body = &raw[boundary + 4..];
    let body = if chunked {
        decode_chunked(path, encoded_body)?
    } else {
        encoded_body.to_vec()
    };
    if let Some(content_length) = headers.get("content-length") {
        let expected = content_length
            .parse::<usize>()
            .map_err(|_| ControlPlaneError::Probe(format!("invalid content-length for {path}")))?;
        if expected != body.len() {
            return Err(ControlPlaneError::Probe(format!(
                "truncated/surplus {path} body: expected {expected}, got {}",
                body.len()
            )));
        }
    } else if !chunked {
        return Err(ControlPlaneError::Probe(format!(
            "{path} response must declare exact content-length or chunked framing"
        )));
    }
    if body.is_empty() {
        return Err(ControlPlaneError::Probe(format!(
            "empty JSON body from {path}"
        )));
    }
    Ok(body)
}

fn decode_chunked(path: &str, input: &[u8]) -> Result<Vec<u8>, ControlPlaneError> {
    let mut cursor = 0_usize;
    let mut decoded = Vec::new();
    loop {
        let remaining = input.get(cursor..).ok_or_else(|| {
            ControlPlaneError::Probe(format!("truncated chunk stream for {path}"))
        })?;
        let relative_end = remaining
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| {
                ControlPlaneError::Probe(format!("truncated chunk header for {path}"))
            })?;
        let line_end = cursor + relative_end;
        let size_text = std::str::from_utf8(&input[cursor..line_end]).map_err(|error| {
            ControlPlaneError::Probe(format!("invalid chunk size for {path}: {error}"))
        })?;
        let size_hex = size_text.split(';').next().unwrap_or_default();
        let size = usize::from_str_radix(size_hex, 16).map_err(|_| {
            ControlPlaneError::Probe(format!("invalid chunk size {size_text:?} for {path}"))
        })?;
        cursor = line_end + 2;
        let data_end = cursor
            .checked_add(size)
            .ok_or_else(|| ControlPlaneError::Probe(format!("chunk size overflow for {path}")))?;
        let frame_end = data_end
            .checked_add(2)
            .ok_or_else(|| ControlPlaneError::Probe(format!("chunk frame overflow for {path}")))?;
        if frame_end > input.len() || input.get(data_end..frame_end) != Some(b"\r\n") {
            return Err(ControlPlaneError::Probe(format!(
                "truncated chunk data for {path}"
            )));
        }
        if size == 0 {
            if frame_end != input.len() {
                return Err(ControlPlaneError::Probe(format!(
                    "chunk trailers/surplus bytes are not accepted for {path}"
                )));
            }
            break;
        }
        decoded.extend_from_slice(&input[cursor..data_end]);
        if decoded.len() > MAX_ADMIN_RESPONSE_BYTES {
            return Err(ControlPlaneError::Probe(format!(
                "decoded chunked body exceeds limit for {path}"
            )));
        }
        cursor = frame_end;
    }
    Ok(decoded)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipBaseline {
    authority_node_id: String,
    prior_epoch: u64,
    snapshots: Vec<PublicControlPlaneSnapshot>,
}

impl MembershipBaseline {
    pub fn authority_node_id(&self) -> &str {
        &self.authority_node_id
    }

    pub fn prior_epoch(&self) -> u64 {
        self.prior_epoch
    }

    pub fn snapshots(&self) -> &[PublicControlPlaneSnapshot] {
        &self.snapshots
    }

    pub fn endpoints(&self) -> Vec<ControlPlaneEndpoint> {
        self.snapshots
            .iter()
            .map(|snapshot| snapshot.endpoint.clone())
            .collect()
    }
}

/// Opaque invocation token. The prior epoch is carried only by a probed live
/// baseline; callers cannot pass an epoch scalar to the observation function.
#[derive(Debug)]
pub struct MembershipTransitionInvocation {
    baseline: MembershipBaseline,
    action_receipt: MembershipActionReceipt,
    invoked_at: Instant,
}

/// Capture the complete pre-action view and derive its authority/epoch.
pub async fn capture_membership_baseline(
    authority_node_id: &str,
    endpoints: Vec<ControlPlaneEndpoint>,
    timeout: Duration,
) -> Result<MembershipBaseline, ControlPlaneError> {
    if !portable_identifier(authority_node_id)
        || timeout.is_zero()
        || timeout > Duration::from_secs(60)
    {
        return Err(ControlPlaneError::Contract(
            "membership baseline requires a valid authority and timeout within (0,60s]".to_owned(),
        ));
    }
    let endpoints = normalized_endpoints(&endpoints)?;
    let mut snapshots = Vec::with_capacity(endpoints.len());
    for endpoint in endpoints {
        snapshots.push(probe_public_snapshot(endpoint, timeout).await?);
    }
    let (_, leader, _, prior_epoch) = validate_snapshot_set(&snapshots)?;
    if leader != authority_node_id {
        return Err(ControlPlaneError::Evidence(format!(
            "declared membership authority {authority_node_id:?} is not the probed leader {leader:?}"
        )));
    }
    Ok(MembershipBaseline {
        authority_node_id: authority_node_id.to_owned(),
        prior_epoch,
        snapshots,
    })
}

/// Capture a complete baseline and derive the authority from the live documents.
pub async fn capture_membership_baseline_from_live(
    endpoints: Vec<ControlPlaneEndpoint>,
    timeout: Duration,
) -> Result<MembershipBaseline, ControlPlaneError> {
    if timeout.is_zero() || timeout > Duration::from_secs(60) {
        return Err(ControlPlaneError::Contract(
            "membership baseline timeout must be within (0,60s]".to_owned(),
        ));
    }
    let endpoints = normalized_endpoints(&endpoints)?;
    let mut snapshots = Vec::with_capacity(endpoints.len());
    for endpoint in endpoints {
        snapshots.push(probe_public_snapshot(endpoint, timeout).await?);
    }
    let (_, authority_node_id, _, prior_epoch) = validate_snapshot_set(&snapshots)?;
    Ok(MembershipBaseline {
        authority_node_id,
        prior_epoch,
        snapshots,
    })
}

/// Bind an externally orchestrated real-process add to its already captured
/// live baseline. `invoked_at` must be captured immediately before the harness
/// action and is intentionally retained only in this opaque token.
pub fn begin_daemon_add_transition(
    baseline: MembershipBaseline,
    receipt: DaemonAddInvocationReceipt,
    invoked_at: Instant,
) -> Result<MembershipTransitionInvocation, ControlPlaneError> {
    let baseline_ids = baseline
        .snapshots
        .iter()
        .map(|snapshot| snapshot.endpoint.node_id.as_str())
        .collect::<BTreeSet<_>>();
    if receipt.payload.authority_node_id != baseline.authority_node_id
        || baseline_ids.contains(receipt.payload.target_node_id.as_str())
        || receipt.payload.target_node_id != receipt.target_process.node_id
    {
        return Err(ControlPlaneError::Evidence(
            "add invocation receipt is not an absent target bound to the probed authority"
                .to_owned(),
        ));
    }
    Ok(MembershipTransitionInvocation {
        baseline,
        action_receipt: MembershipActionReceipt::DaemonAdd(Box::new(receipt)),
        invoked_at,
    })
}

/// Invoke the actual authenticated drain route and return an opaque transition
/// token whose timer began before the request was written.
pub async fn begin_admin_drain_transition(
    baseline: MembershipBaseline,
    target_node_id: &str,
    timeout: Duration,
) -> Result<MembershipTransitionInvocation, ControlPlaneError> {
    if target_node_id == baseline.authority_node_id {
        return Err(ControlPlaneError::Evidence(
            "ordinary W4A drain must not remove its observation authority; leader change belongs to W5"
                .to_owned(),
        ));
    }
    let endpoint = baseline
        .snapshots
        .iter()
        .map(|snapshot| &snapshot.endpoint)
        .find(|endpoint| endpoint.node_id == target_node_id)
        .ok_or_else(|| {
            ControlPlaneError::Evidence(
                "drain target is absent from the probed pre-transition baseline".to_owned(),
            )
        })?;
    let invoked_at = Instant::now();
    let receipt = request_admin_drain(endpoint, timeout).await?;
    Ok(MembershipTransitionInvocation {
        baseline,
        action_receipt: MembershipActionReceipt::AdminDrain(receipt),
        invoked_at,
    })
}

/// Invoke the authenticated real drain endpoint. This receipt proves request
/// acceptance only; commit and convergence are independently snapshot-derived.
pub async fn request_admin_drain(
    endpoint: &ControlPlaneEndpoint,
    timeout: Duration,
) -> Result<AdminDrainInvocationReceipt, ControlPlaneError> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct DrainWire {
        action: String,
        outcome: String,
        drain: DrainOutcomeWire,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct DrainOutcomeWire {
        started_with: u64,
        remaining: u64,
        timed_out: bool,
    }

    let response = admin_http_json(endpoint, ADMIN_DRAIN_PATH, true, timeout).await?;
    let wire: DrainWire = serde_json::from_slice(&response.body).map_err(|error| {
        ControlPlaneError::Probe(format!("invalid strict {ADMIN_DRAIN_PATH} JSON: {error}"))
    })?;
    if wire.action != "drain"
        || wire.outcome != "accepted"
        || wire.drain.remaining != 0
        || wire.drain.timed_out
    {
        return Err(ControlPlaneError::Probe(format!(
            "{ADMIN_DRAIN_PATH} returned incomplete action/outcome/drain {:?}/{:?}/{}/{}, expected accepted complete drain",
            wire.action, wire.outcome, wire.drain.remaining, wire.drain.timed_out
        )));
    }
    Ok(AdminDrainInvocationReceipt {
        target_node_id: endpoint.node_id.clone(),
        path: ADMIN_DRAIN_PATH.to_owned(),
        action: wire.action,
        outcome: wire.outcome,
        started_with: wire.drain.started_with,
        remaining: wire.drain.remaining,
        timed_out: wire.drain.timed_out,
        response_sha256: sha256_bytes(&response.body),
        request_network_bytes: response.request_bytes,
        response_network_bytes: response.response_bytes,
    })
}

/// Poll until one exact newer authority epoch is committed and every exact
/// post-action endpoint exposes the same complete view. No caller-supplied
/// prior epoch is accepted: it comes from `MembershipBaseline`.
pub async fn observe_membership_transition(
    invocation: MembershipTransitionInvocation,
    expected_endpoints_after: Vec<ControlPlaneEndpoint>,
    timeout: Duration,
    poll_interval: Duration,
) -> Result<MembershipEventEvidence, ControlPlaneError> {
    if timeout.is_zero()
        || timeout > Duration::from_secs(120)
        || poll_interval.is_zero()
        || poll_interval > timeout
    {
        return Err(ControlPlaneError::Contract(
            "membership observation requires bounded non-zero timeout/poll intervals".to_owned(),
        ));
    }
    let expected = normalized_endpoints(&expected_endpoints_after)?;
    let baseline = invocation.baseline;
    let action_receipt = invocation.action_receipt;
    let action = action_receipt.action();
    let target_node_id = action_receipt.target_node_id().to_owned();
    let authority_node_id = baseline.authority_node_id.clone();
    let prior_epoch = baseline.prior_epoch;
    if !expected
        .iter()
        .any(|endpoint| endpoint.node_id == authority_node_id)
    {
        return Err(ControlPlaneError::Capability(
            "membership observation authority is absent from the expected post-action set"
                .to_owned(),
        ));
    }
    let deadline = invocation.invoked_at + timeout;
    let mut rounds = 0_u64;
    let mut attempts = 0_u64;
    let mut complete_observations = 0_u64;
    let mut request_bytes = 0_u64;
    let mut response_bytes = 0_u64;
    let mut authority_commit = None;
    let mut committed_after_nanos = None;
    let mut latest_error = None;
    loop {
        rounds = rounds.saturating_add(1);
        let mut ordered = expected.clone();
        ordered.sort_by_key(|endpoint| endpoint.node_id != authority_node_id);
        let mut snapshots = Vec::with_capacity(ordered.len());
        for endpoint in ordered {
            attempts = attempts.saturating_add(1);
            match probe_public_snapshot_with_wire(
                endpoint,
                poll_interval.min(Duration::from_secs(10)),
            )
            .await
            {
                Ok(receipt) => {
                    request_bytes = request_bytes.saturating_add(receipt.request_network_bytes);
                    response_bytes = response_bytes.saturating_add(receipt.response_network_bytes);
                    snapshots.push(receipt.snapshot);
                }
                Err(error) => latest_error = Some(error.to_string()),
            }
        }
        if let Some(snapshot) = snapshots.iter().find(|snapshot| {
            snapshot.endpoint.node_id == authority_node_id
                && snapshot.admin_status.source == ControlPlaneSource::Live
                && snapshot.admin_status.quorum_ok
                && snapshot.admin_status.epoch > prior_epoch
                && validate_public_snapshot(snapshot, &expected, expected.len() as u8).is_ok()
        }) {
            if authority_commit.is_none() {
                authority_commit = Some(snapshot.clone());
                committed_after_nanos = Some(elapsed_since_nanos(invocation.invoked_at));
            }
        }
        let committed_epoch = authority_commit
            .as_ref()
            .map(|snapshot| snapshot.admin_status.epoch);
        let converged = snapshots.len() == expected.len()
            && validate_snapshot_set(&snapshots).is_ok_and(|(_, leader, _, epoch)| {
                leader == authority_node_id && Some(epoch) == committed_epoch
            });
        if converged {
            complete_observations = complete_observations.saturating_add(1);
            let authority_commit_snapshot = authority_commit.ok_or_else(|| {
                ControlPlaneError::Evidence(
                    "surfaces converged without a separately observed authority commit".to_owned(),
                )
            })?;
            let new_epoch = authority_commit_snapshot.admin_status.epoch;
            let commit_latency_nanos = committed_after_nanos.expect("commit snapshot sets time");
            let convergence_latency_nanos = elapsed_since_nanos(invocation.invoked_at);
            snapshots.sort_by(|left, right| left.endpoint.node_id.cmp(&right.endpoint.node_id));
            let (action_request_network_bytes, action_response_network_bytes) =
                match &action_receipt {
                    MembershipActionReceipt::AdminDrain(receipt) => (
                        receipt.request_network_bytes,
                        receipt.response_network_bytes,
                    ),
                    MembershipActionReceipt::DaemonAdd(_) => (0, 0),
                };
            return Ok(MembershipEventEvidence {
                action,
                target_node_id,
                authority_node_id,
                action_receipt,
                pre_transition_snapshots: baseline.snapshots,
                authority_commit_snapshot,
                post_transition_snapshots: snapshots,
                prior_epoch,
                new_epoch,
                timer_kind: "monotonic-elapsed".to_owned(),
                commit_latency_nanos,
                convergence_latency_nanos,
                wire: MembershipEventWireEvidence {
                    counter_scope: EVENT_COUNTER_SCOPE.to_owned(),
                    action_request_network_bytes,
                    action_response_network_bytes,
                    successful_snapshot_request_network_bytes: request_bytes,
                    successful_snapshot_response_network_bytes: response_bytes,
                    observation_rounds: rounds,
                    snapshot_probe_attempts: attempts,
                    complete_snapshot_observations: complete_observations,
                },
            });
        }
        if Instant::now() >= deadline {
            return Err(ControlPlaneError::Probe(format!(
                "membership transition did not converge within {}ms; latest_error={latest_error:?}",
                timeout.as_millis()
            )));
        }
        tokio::time::sleep(poll_interval.min(deadline.saturating_duration_since(Instant::now())))
            .await;
    }
}

fn elapsed_since_nanos(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos())
        .unwrap_or(u64::MAX)
        .max(1)
}

/// Open-loop adapter for one exact real admin endpoint. It validates every
/// response against the complete strict baseline captured by the capability.
#[derive(Debug)]
pub struct ControlPlaneTarget {
    endpoint: ControlPlaneEndpoint,
    endpoint_set: Vec<ControlPlaneEndpoint>,
    target_role: NodeRole,
    baseline: PublicControlPlaneSnapshot,
    node_count: u8,
    timeout: Duration,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    admin_status_requests: AtomicU64,
    cluster_overview_requests: AtomicU64,
    request_network_bytes: AtomicU64,
    response_network_bytes: AtomicU64,
}

impl ControlPlaneTarget {
    pub fn new(
        capability: Arc<ProbedControlPlaneCapability>,
        target_node_id: &str,
        target_role: NodeRole,
        timeout: Duration,
    ) -> Result<Self, ControlPlaneError> {
        if timeout.is_zero() || timeout > Duration::from_secs(60) {
            return Err(ControlPlaneError::Contract(
                "W4A target timeout must be within (0, 60s]".to_owned(),
            ));
        }
        let endpoint = capability
            .attestation
            .endpoints
            .iter()
            .find(|endpoint| endpoint.node_id == target_node_id)
            .cloned()
            .ok_or_else(|| {
                ControlPlaneError::Capability(format!(
                    "selected target {target_node_id:?} is absent from the probed capability"
                ))
            })?;
        let baseline = capability
            .baseline
            .iter()
            .find(|snapshot| snapshot.endpoint == endpoint)
            .cloned()
            .ok_or_else(|| {
                ControlPlaneError::Capability(
                    "selected target has no strict live baseline snapshot".to_owned(),
                )
            })?;
        let observed_role = baseline.target_role()?;
        if observed_role != target_role {
            return Err(ControlPlaneError::Capability(format!(
                "selected target {target_node_id:?} is {observed_role:?}, not requested {target_role:?}"
            )));
        }
        Ok(Self {
            endpoint,
            endpoint_set: capability.attestation.endpoints.clone(),
            target_role,
            baseline,
            node_count: capability.attestation.node_count,
            timeout,
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            admin_status_requests: AtomicU64::new(0),
            cluster_overview_requests: AtomicU64::new(0),
            request_network_bytes: AtomicU64::new(0),
            response_network_bytes: AtomicU64::new(0),
        })
    }

    pub fn endpoint(&self) -> &ControlPlaneEndpoint {
        &self.endpoint
    }

    pub fn target_role(&self) -> NodeRole {
        self.target_role
    }

    pub fn complete_endpoint_set(&self) -> &[ControlPlaneEndpoint] {
        &self.endpoint_set
    }

    pub fn counters(&self) -> ControlPlaneTargetCounters {
        ControlPlaneTargetCounters {
            successful_requests: self.successful_requests.load(Ordering::SeqCst),
            failed_requests: self.failed_requests.load(Ordering::SeqCst),
            admin_status_requests: self.admin_status_requests.load(Ordering::SeqCst),
            cluster_overview_requests: self.cluster_overview_requests.load(Ordering::SeqCst),
            request_network_bytes: self.request_network_bytes.load(Ordering::SeqCst),
            response_network_bytes: self.response_network_bytes.load(Ordering::SeqCst),
        }
    }

    /// Reset all process-local measurement counters at the repeat boundary.
    pub fn reset_counters(&self) {
        self.successful_requests.store(0, Ordering::SeqCst);
        self.failed_requests.store(0, Ordering::SeqCst);
        self.admin_status_requests.store(0, Ordering::SeqCst);
        self.cluster_overview_requests.store(0, Ordering::SeqCst);
        self.request_network_bytes.store(0, Ordering::SeqCst);
        self.response_network_bytes.store(0, Ordering::SeqCst);
    }

    /// Atomically take a repeat's counters and zero them for the next repeat.
    pub fn take_counters(&self) -> ControlPlaneTargetCounters {
        ControlPlaneTargetCounters {
            successful_requests: self.successful_requests.swap(0, Ordering::SeqCst),
            failed_requests: self.failed_requests.swap(0, Ordering::SeqCst),
            admin_status_requests: self.admin_status_requests.swap(0, Ordering::SeqCst),
            cluster_overview_requests: self.cluster_overview_requests.swap(0, Ordering::SeqCst),
            request_network_bytes: self.request_network_bytes.swap(0, Ordering::SeqCst),
            response_network_bytes: self.response_network_bytes.swap(0, Ordering::SeqCst),
        }
    }

    pub async fn public_snapshot(&self) -> Result<PublicControlPlaneSnapshot, ControlPlaneError> {
        let snapshot = probe_public_snapshot(self.endpoint.clone(), self.timeout).await?;
        validate_public_snapshot(&snapshot, &self.endpoint_set, self.node_count)?;
        if snapshot.target_role()? != self.target_role {
            return Err(ControlPlaneError::Evidence(
                "selected W4A endpoint changed role; ordinary steady window is invalid and belongs to W5"
                    .to_owned(),
            ));
        }
        Ok(snapshot)
    }

    async fn compute_state_digest(&self) -> Result<String, ControlPlaneError> {
        let snapshot = self.public_snapshot().await?;
        let encoded = serde_json::to_vec(&snapshot).map_err(|error| {
            ControlPlaneError::Evidence(format!("unable to serialize W4A state: {error}"))
        })?;
        let mut digest = Sha256::new();
        digest.update(CONTROL_PLANE_STATE_DIGEST_VERSION.as_bytes());
        digest.update(encoded);
        Ok(format!("sha256:{}", hex_digest(&digest.finalize())))
    }

    async fn execute_path(&self, path: &'static str, admin: bool) -> TargetOutcome {
        if path == ADMIN_STATUS_PATH {
            self.admin_status_requests.fetch_add(1, Ordering::SeqCst);
        } else {
            self.cluster_overview_requests
                .fetch_add(1, Ordering::SeqCst);
        }
        match admin_http_json(&self.endpoint, path, admin, self.timeout).await {
            Ok(response) => {
                self.request_network_bytes
                    .fetch_add(response.request_bytes, Ordering::SeqCst);
                self.response_network_bytes
                    .fetch_add(response.response_bytes, Ordering::SeqCst);
                let exact = if path == ADMIN_STATUS_PATH {
                    serde_json::from_slice::<AdminStatusObservation>(&response.body)
                        .ok()
                        .is_some_and(|status| status == self.baseline.admin_status)
                } else {
                    serde_json::from_slice::<ClusterOverviewObservation>(&response.body)
                        .ok()
                        .is_some_and(|overview| overview == self.baseline.cluster_overview)
                };
                if exact {
                    self.successful_requests.fetch_add(1, Ordering::SeqCst);
                    TargetOutcome::Success
                } else {
                    self.failed_requests.fetch_add(1, Ordering::SeqCst);
                    TargetOutcome::Error
                }
            }
            Err(ControlPlaneError::Timeout { .. }) => {
                self.failed_requests.fetch_add(1, Ordering::SeqCst);
                TargetOutcome::Timeout
            }
            Err(_) => {
                self.failed_requests.fetch_add(1, Ordering::SeqCst);
                TargetOutcome::Error
            }
        }
    }
}

impl ControlPlaneCounterSource for ControlPlaneTarget {
    fn take_repeat_counters(&self) -> ControlPlaneTargetCounters {
        self.take_counters()
    }
}

/// Execute the exact W4A W0 schedule against one selected endpoint/role. Wire
/// rows are captured immediately after each repeat, before the next reset can
/// clear its atomics, and cannot be supplied by the caller.
pub async fn run_control_plane_knee(
    target: Arc<ControlPlaneTarget>,
    scenario: &ControlPlaneScenario,
) -> Result<SteadyReadEvidence, ControlPlaneError> {
    scenario.validate()?;
    let start = target.public_snapshot().await?;
    let (knee, repeat_wire_evidence) =
        run_control_plane_knee_raw(Arc::clone(&target), scenario).await?;
    let end = target.public_snapshot().await?;
    let role_changed = start.admin_status.leader != end.admin_status.leader
        || start.admin_status.term != end.admin_status.term
        || start.admin_status.epoch != end.admin_status.epoch
        || start.target_role()? != end.target_role()?;
    let evidence = SteadyReadEvidence {
        target_node_id: target.endpoint.node_id.clone(),
        target_node_role: target.target_role,
        complete_endpoint_set: target.endpoint_set.clone(),
        start,
        end,
        criteria: scenario.sustainability_criteria(),
        knee,
        repeat_wire_evidence,
        role_changed,
    };
    validate_steady_read(
        &evidence,
        scenario,
        target.complete_endpoint_set(),
        target.node_count,
    )?;
    Ok(evidence)
}

async fn run_control_plane_knee_raw<T>(
    target: Arc<T>,
    scenario: &ControlPlaneScenario,
) -> Result<(KneeResult, Vec<SteadyRepeatWireEvidence>), ControlPlaneError>
where
    T: ControlPlaneCounterSource + 'static,
{
    scenario.validate()?;
    let criteria = scenario.sustainability_criteria();
    let mut points = Vec::with_capacity(scenario.read_only.offered_rates_per_second.len());
    let mut wire_rows = Vec::with_capacity(
        scenario.read_only.offered_rates_per_second.len()
            * usize::try_from(scenario.read_only.repeats).expect("u32 repeat count fits usize"),
    );
    for rate in &scenario.read_only.offered_rates_per_second {
        let config = PhaseConfig {
            preload_operations: scenario.read_only.preload_operations,
            warmup_operations: scenario.read_only.warmup_operations,
            steady: OpenLoopConfig {
                offered_rate_per_second: *rate,
                operations: scenario.read_only.steady_operations,
                highest_trackable_latency: Duration::from_micros(
                    scenario.read_only.highest_trackable_latency_micros,
                ),
                significant_figures: scenario.read_only.histogram_significant_figures,
                p999_min_samples: scenario.read_only.p999_min_samples,
                drain_timeout: Duration::from_millis(scenario.read_only.backlog_drain_millis),
            },
        };
        let mut repeats = Vec::with_capacity(
            usize::try_from(scenario.read_only.repeats).expect("u32 repeat count fits usize"),
        );
        for repeat_index in 1..=scenario.read_only.repeats {
            let phase = run_phases(Arc::clone(&target), &config)
                .await
                .map_err(|error| {
                    ControlPlaneError::Evidence(format!(
                        "W4A rate {rate} repeat {repeat_index} failed: {error}"
                    ))
                })?;
            let counters = target.take_repeat_counters();
            wire_rows.push(SteadyRepeatWireEvidence::from_counters(
                *rate,
                repeat_index,
                counters,
            ));
            repeats.push(phase.into_evidence());
        }
        points.push(criteria.evaluate_repeats(*rate as f64, repeats));
    }
    Ok((criteria.find_knee(points), wire_rows))
}

#[async_trait]
impl Target for ControlPlaneTarget {
    async fn reset(&self) -> Result<String, TargetError> {
        self.reset_counters();
        self.compute_state_digest()
            .await
            .map_err(|error| TargetError::Reset(error.to_string()))
    }

    async fn preload(&self) -> Result<PreloadOutcome, TargetError> {
        Ok(PreloadOutcome {
            operations: CONTROL_PLANE_PRELOAD_OPERATIONS,
            state_digest: self
                .compute_state_digest()
                .await
                .map_err(|error| TargetError::Preload(error.to_string()))?,
        })
    }

    async fn state_digest(&self) -> Result<String, TargetError> {
        self.compute_state_digest()
            .await
            .map_err(|error| TargetError::Warmup(error.to_string()))
    }

    async fn execute(&self, request: TargetRequest) -> TargetOutcome {
        if request.sequence.is_multiple_of(2) {
            self.execute_path(ADMIN_STATUS_PATH, true).await
        } else {
            self.execute_path(CLUSTER_OVERVIEW_PATH, false).await
        }
    }
}

/// W4 composite-canary leg: extend one observed convergence duration beyond the
/// committed bound. Acceptance means the canary is non-discriminating.
pub fn canary_control_plane_delay_breaches_the_w4a_event_budget(
    scenario: &ControlPlaneScenario,
) -> Result<(), String> {
    scenario
        .validate()
        .map_err(|error| format!("W4A canary scenario is invalid: {error}"))?;
    validate_membership_event_timing(1, 1, 1, 2, scenario)
        .map_err(|error| format!("W4A canary baseline timing is invalid: {error}"))?;
    let injected_convergence = scenario
        .membership_event
        .event_timeout_millis
        .saturating_mul(1_000_000)
        .saturating_add(1);
    if validate_membership_event_timing(1, injected_convergence, 1, 2, scenario).is_err() {
        Err(format!(
            "{W4_CANARY_MARKER} injected control-plane convergence delay breached the W4A event budget"
        ))
    } else {
        Ok(())
    }
}

fn validate_membership_event_timing(
    commit_latency_nanos: u64,
    convergence_latency_nanos: u64,
    prior_epoch: u64,
    new_epoch: u64,
    scenario: &ControlPlaneScenario,
) -> Result<(), ControlPlaneError> {
    if commit_latency_nanos == 0
        || convergence_latency_nanos < commit_latency_nanos
        || convergence_latency_nanos
            > scenario
                .membership_event
                .event_timeout_millis
                .saturating_mul(1_000_000)
        || new_epoch <= prior_epoch
    {
        return Err(ControlPlaneError::Evidence(
            "W4A membership event lacks monotonic bounded latency or a single forward epoch transition"
                .to_owned(),
        ));
    }
    Ok(())
}

pub fn membership_event_latencies_nanos(
    event: &MembershipEventEvidence,
) -> BTreeMap<&'static str, u64> {
    BTreeMap::from([
        ("commit_latency_nanos", event.commit_latency_nanos),
        ("convergence_latency_nanos", event.convergence_latency_nanos),
    ])
}

fn portable_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn canonical_commit(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && lower_hex(value)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && lower_hex(value)
}

fn lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn canonical_digest<T: Serialize>(value: &T) -> Result<String, ControlPlaneError> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        ControlPlaneError::Contract(format!("unable to serialize W4A receipt: {error}"))
    })?;
    Ok(sha256_bytes(&bytes))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    hex_digest(&Sha256::digest(bytes))
}

fn sha256_file(path: &Path) -> Result<String, ControlPlaneError> {
    let bytes = fs::read(path).map_err(|error| {
        ControlPlaneError::Capability(format!(
            "unable to read {} for SHA-256: {error}",
            path.display()
        ))
    })?;
    Ok(sha256_bytes(&bytes))
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(target_os = "windows")]
fn observed_process_executable(pid: u32) -> Result<PathBuf, ControlPlaneError> {
    let script = format!("(Get-Process -Id {pid} -ErrorAction Stop).Path");
    let output = Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &script,
        ])
        .output()
        .map_err(|error| {
            ControlPlaneError::Capability(format!(
                "unable to execute OS process identity probe for PID {pid}: {error}"
            ))
        })?;
    if !output.status.success() {
        return Err(ControlPlaneError::Capability(format!(
            "OS process identity probe rejected PID {pid}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let path = String::from_utf8(output.stdout)
        .map_err(|error| ControlPlaneError::Capability(format!("PID path is non-UTF8: {error}")))?;
    fs::canonicalize(path.trim()).map_err(|error| {
        ControlPlaneError::Capability(format!(
            "unable to canonicalize executable observed for PID {pid}: {error}"
        ))
    })
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn observed_process_executable(pid: u32) -> Result<PathBuf, ControlPlaneError> {
    let _ = pid;
    Err(ControlPlaneError::Capability(
        "mandatory W4A capability has no supported OS process-identity probe on this platform"
            .to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENARIO: &str = include_str!(
        "../../../../docs/testing/perf-scenarios/0.67/control-plane-real-daemon-v1.toml"
    );

    #[test]
    fn committed_scenario_retains_exact_w0_and_narrow_capacity_contract() {
        let scenario = ControlPlaneScenario::parse_toml(SCENARIO).unwrap();
        assert_eq!(
            scenario.read_only.offered_rates_per_second,
            CONTROL_PLANE_OFFERED_RATES_PER_SECOND
        );
        assert_eq!(scenario.read_only.repeats, 5);
        assert_eq!(
            scenario.identity.capacity_claim,
            "selected-admin-endpoint-read-only"
        );
        assert_eq!(
            scenario.sustainability_criteria().p99_slo_us,
            scenario.read_only.latency_slo_micros
        );
    }

    #[test]
    fn absent_or_self_asserted_loopback_capability_never_passes_mandatory() {
        let scenario = ControlPlaneScenario::parse_toml(SCENARIO).unwrap();
        assert!(ControlPlaneCapabilityAttestation::absent()
            .require(&scenario, ReferenceCapabilityPolicy::MandatoryFailClosed)
            .is_err());
        assert!(matches!(
            ControlPlaneCapabilityAttestation::absent()
                .require(&scenario, ReferenceCapabilityPolicy::LocalSkipLoud)
                .unwrap(),
            ControlPlaneCapabilityOutcome::SkippedLoud(_)
        ));

        let payload = ControlPlaneCapabilityReceiptPayload {
            receipt_kind: DAEMON_CAPABILITY_RECEIPT_KIND.to_owned(),
            receipt_source: DaemonReceiptSource::SelfAsserted,
            execution_mode: CONTROL_PLANE_EXECUTION_MODE.to_owned(),
            profile: "reference-v1".to_owned(),
            source_commit: "a".repeat(40),
            runner_fingerprint_sha256: "b".repeat(64),
            prebuild_manifest_canonical_path: PathBuf::from("C:/fixture/prebuild-manifest.json"),
            prebuild_manifest_sha256: "c".repeat(64),
            prebuild_contract_sha256: "d".repeat(64),
            provisioner: DAEMON_CLUSTER_PROVISIONER.to_owned(),
            direct_prebuilt_exec: true,
            server_binary: PrebuiltServerBinaryReceipt {
                canonical_path: PathBuf::from("C:/fixture/hydracache-server.exe"),
                sha256: "e".repeat(64),
            },
            node_count: 3,
            nodes: Vec::new(),
        };
        let error = ControlPlaneCapabilityAttestation::seal(payload)
            .unwrap()
            .require(&scenario, ReferenceCapabilityPolicy::MandatoryFailClosed)
            .unwrap_err()
            .to_string();
        assert!(error.contains("self-asserted or fixture loopback"));
    }

    #[test]
    fn repeat_counter_take_resets_every_atomic() {
        let endpoint = ControlPlaneEndpoint {
            node_id: "node-1".to_owned(),
            admin_addr: "127.0.0.1:19091".parse().unwrap(),
        };
        let target = ControlPlaneTarget {
            endpoint: endpoint.clone(),
            endpoint_set: vec![endpoint.clone()],
            target_role: NodeRole::Leader,
            baseline: placeholder_snapshot(endpoint),
            node_count: 1,
            timeout: Duration::from_secs(1),
            successful_requests: AtomicU64::new(7),
            failed_requests: AtomicU64::new(2),
            admin_status_requests: AtomicU64::new(5),
            cluster_overview_requests: AtomicU64::new(4),
            request_network_bytes: AtomicU64::new(100),
            response_network_bytes: AtomicU64::new(200),
        };
        let taken = target.take_counters();
        assert_eq!(taken.successful_requests, 7);
        assert_eq!(taken.failed_requests, 2);
        assert_eq!(taken.admin_status_requests, 5);
        assert_eq!(taken.cluster_overview_requests, 4);
        assert_eq!(target.counters(), ControlPlaneTargetCounters::default());
    }

    #[derive(Debug, Default)]
    struct FakeCounterTarget {
        successful: AtomicU64,
        failed: AtomicU64,
        status: AtomicU64,
        overview: AtomicU64,
        request_bytes: AtomicU64,
        response_bytes: AtomicU64,
    }

    #[async_trait]
    impl Target for FakeCounterTarget {
        async fn reset(&self) -> Result<String, TargetError> {
            let _ = self.take_repeat_counters();
            Ok("sha256:reset".to_owned())
        }

        async fn preload(&self) -> Result<PreloadOutcome, TargetError> {
            Ok(PreloadOutcome {
                operations: CONTROL_PLANE_PRELOAD_OPERATIONS,
                state_digest: "sha256:preloaded".to_owned(),
            })
        }

        async fn state_digest(&self) -> Result<String, TargetError> {
            Ok("sha256:warmed".to_owned())
        }

        async fn execute(&self, request: TargetRequest) -> TargetOutcome {
            self.successful.fetch_add(1, Ordering::SeqCst);
            if request.sequence.is_multiple_of(2) {
                self.status.fetch_add(1, Ordering::SeqCst);
            } else {
                self.overview.fetch_add(1, Ordering::SeqCst);
            }
            self.request_bytes.fetch_add(100, Ordering::SeqCst);
            self.response_bytes.fetch_add(200, Ordering::SeqCst);
            TargetOutcome::Success
        }
    }

    impl ControlPlaneCounterSource for FakeCounterTarget {
        fn take_repeat_counters(&self) -> ControlPlaneTargetCounters {
            ControlPlaneTargetCounters {
                successful_requests: self.successful.swap(0, Ordering::SeqCst),
                failed_requests: self.failed.swap(0, Ordering::SeqCst),
                admin_status_requests: self.status.swap(0, Ordering::SeqCst),
                cluster_overview_requests: self.overview.swap(0, Ordering::SeqCst),
                request_network_bytes: self.request_bytes.swap(0, Ordering::SeqCst),
                response_network_bytes: self.response_bytes.swap(0, Ordering::SeqCst),
            }
        }
    }

    #[tokio::test(start_paused = true, flavor = "current_thread")]
    async fn w4a_runner_derives_all_five_by_five_wire_rows_before_counter_reset() {
        let scenario = ControlPlaneScenario::parse_toml(SCENARIO).unwrap();
        let (knee, rows) =
            run_control_plane_knee_raw(Arc::new(FakeCounterTarget::default()), &scenario)
                .await
                .unwrap();
        assert_eq!(rows.len(), 25);
        assert_eq!(
            rows.iter()
                .map(|row| (row.offered_rate_per_second, row.repeat_index))
                .collect::<BTreeSet<_>>()
                .len(),
            25
        );
        assert!(rows.iter().all(|row| {
            row.successful_requests
                == CONTROL_PLANE_WARMUP_OPERATIONS + CONTROL_PLANE_STEADY_OPERATIONS
                && row.failed_requests == 0
                && row.admin_status_requests + row.cluster_overview_requests
                    == CONTROL_PLANE_WARMUP_OPERATIONS + CONTROL_PLANE_STEADY_OPERATIONS
        }));
        assert!(scenario
            .sustainability_criteria()
            .knee_validation_problems(&knee)
            .is_empty());
    }

    fn placeholder_snapshot(endpoint: ControlPlaneEndpoint) -> PublicControlPlaneSnapshot {
        PublicControlPlaneSnapshot {
            endpoint: endpoint.clone(),
            admin_status: AdminStatusObservation {
                source: ControlPlaneSource::Live,
                leader: Some(endpoint.node_id.clone()),
                term: 1,
                epoch: 1,
                quorum_ok: true,
                members: 1,
                member_ids: vec![endpoint.node_id.clone()],
                voters: 1,
                voter_ids: vec![1],
                reshard_phase: "idle".to_owned(),
                draining: false,
            },
            cluster_overview: ClusterOverviewObservation {
                source: ControlPlaneSource::Live,
                members: vec![OverviewMemberObservation {
                    node_id: endpoint.node_id.clone(),
                    role: "member".to_owned(),
                    reachable: true,
                    reachability: "reachable".to_owned(),
                    generation: 1,
                }],
                leader: Some(OverviewLeaderObservation {
                    node_id: endpoint.node_id,
                    term: 1,
                    epoch: 1,
                }),
                partitions: OverviewPartitionObservation {
                    under_replicated: 0,
                    count: 0,
                },
                consistency: OverviewConsistencyObservation {
                    configured_default: None,
                    op_counts_by_level: Vec::new(),
                },
                backup_age_seconds: None,
                lifecycle: OverviewLifecycleObservation {
                    reshard_phase: "idle".to_owned(),
                    upgrade_phase: "idle".to_owned(),
                },
            },
        }
    }
}
