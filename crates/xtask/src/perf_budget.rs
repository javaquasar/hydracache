//! Release-0.67 macro performance budget and rolling-baseline contract.
//!
//! This checker intentionally has no baseline-generation mode.  A reviewed
//! budget file and an immutable release anchor are inputs, never outputs.  The
//! first reference contract therefore remains explicitly unbootstrapped until
//! real, eligible `main` receipts are reviewed and committed.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use hydracache_loadgen::overload::{
    EligibleOverloadSurface, NodeRespStableCapability, OverloadReport, OverloadRunMode,
    ReferenceExecutionKind,
};
use hydracache_loadgen::report::WorkloadIdentity;
use hydracache_loadgen::targets::control_plane::ControlPlaneReport;
use hydracache_loadgen::targets::grid_model::GridModelReport;

pub use hydracache_loadgen::budget_receipt::{
    BinaryDigest, MacroBatchPublicationReceipt, MacroReportReceipt, ReportMetric,
    MACRO_PUBLICATION_RECEIPT_RELATIVE, MACRO_REPORT_PATHS,
};
pub use hydracache_loadgen::profile::{
    PerformanceProfile as RunnerContract, RunnerFingerprint as ObservedRunner,
};
pub use hydracache_loadgen::report::EvidenceRunMode;

pub const RELEASE: &str = "0.67.0";
pub const PROFILE_ROOT: &str = "docs/testing/perf-profiles";
pub const BUDGET_ROOT: &str = "docs/testing/perf-budgets/0.67";
pub const BASELINE_ROOT: &str = "docs/testing/perf-baselines/0.67";
pub const PREBUILD_MANIFEST_PATH: &str = "target/test-evidence/0.67/prebuild-manifest.json";
pub const VERDICT_PATH: &str = "target/test-evidence/0.67/perf-budget-verdict.json";
pub const DEFAULT_MINIMUM_MEMBERS: usize = 5;
pub const DEFAULT_MAXIMUM_MEMBERS: usize = 10;
pub const DEFAULT_MAXIMUM_AGE_DAYS: i64 = 30;
pub const CLEAN_GIT_STATUS_SHA256: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// Exact W1-W6 macro report set.  Supplemental closed-loop W3 evidence and
/// later W8/W9 reports are deliberately outside this budget set.
pub const EXPECTED_REPORTS_067: [(&str, &str); 13] = [
    ("local", "target/test-evidence/0.67/local.json"),
    (
        "client-surface",
        "target/test-evidence/0.67/client-surface.json",
    ),
    (
        "node-resp-open-loop",
        "target/test-evidence/0.67/node-resp-open-loop.json",
    ),
    (
        "control-plane-3",
        "target/test-evidence/0.67/control-plane-3.json",
    ),
    (
        "control-plane-5",
        "target/test-evidence/0.67/control-plane-5.json",
    ),
    (
        "control-plane-7",
        "target/test-evidence/0.67/control-plane-7.json",
    ),
    ("grid-model", "target/test-evidence/0.67/grid-model.json"),
    (
        "brownout-control-plane",
        "target/test-evidence/0.67/brownout-control-plane.json",
    ),
    (
        "brownout-resp-endpoint",
        "target/test-evidence/0.67/brownout-resp-endpoint.json",
    ),
    (
        "brownout-grid-model",
        "target/test-evidence/0.67/brownout-grid-model.json",
    ),
    (
        "overload-local",
        "target/test-evidence/0.67/overload-local.json",
    ),
    (
        "overload-client-surface",
        "target/test-evidence/0.67/overload-client-surface.json",
    ),
    (
        "overload-node-resp",
        "target/test-evidence/0.67/overload-node-resp.json",
    ),
];

#[derive(Debug)]
pub struct PerfBudgetError(String);

impl PerfBudgetError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for PerfBudgetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for PerfBudgetError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapStatus {
    Unbootstrapped,
    Bootstrapped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Enforcement {
    NonEnforcingTripwire,
    Ship,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportFormat {
    PerfReportV1,
    MacroReceiptV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetDirection {
    Floor,
    Ceiling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetRuleStatus {
    Unbootstrapped,
    Active,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileContract {
    pub schema_version: u32,
    pub release: String,
    pub name: String,
    pub enforcement: Enforcement,
    pub bootstrap_status: BootstrapStatus,
    pub required_platform_key: String,
    pub runner: RunnerContract,
    pub noise: NoiseContract,
    pub prebuild: PrebuildContract,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoiseContract {
    pub absolute_numbers_are_ship_evidence: bool,
    pub comparison_class: String,
    pub maximum_report_spread_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrebuildContract {
    pub schema_version: u32,
    pub toolchain_identity: String,
    pub target_set: Vec<String>,
    pub features: Vec<String>,
    pub cargo_profile: String,
    pub flags: Vec<String>,
    pub build_recipe: Vec<String>,
    pub digest: String,
}

/// Exact per-run prebuild artifact produced before any measurement process.
/// W7 re-hashes this file, Cargo.lock, and every recorded binary from disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerfPrebuildManifest {
    pub schema_version: u32,
    pub source: PrebuildSource,
    pub toolchain_identity: String,
    pub target_set: Vec<String>,
    pub features: Vec<String>,
    pub cargo_profile: String,
    pub flags: Vec<String>,
    pub build_recipe: Vec<String>,
    pub build_contract_digest: String,
    pub runner_profile: String,
    pub runner_fingerprint: String,
    pub platform_key: String,
    pub binaries: Vec<PrebuiltBinary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrebuildSource {
    pub git_commit: String,
    pub cargo_lock_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrebuiltBinary {
    pub id: String,
    pub canonical_path: PathBuf,
    pub sha256: String,
}

impl PrebuildContract {
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetContract {
    pub schema_version: u32,
    pub release: String,
    pub profile: String,
    pub enforcement: Enforcement,
    pub bootstrap_status: BootstrapStatus,
    #[serde(rename = "report")]
    pub reports: Vec<ExpectedReport>,
    #[serde(rename = "budget")]
    pub budgets: Vec<BudgetRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedReport {
    pub id: String,
    pub path: String,
    pub format: ReportFormat,
    pub report_id: String,
    pub claim_scope: String,
    pub capacity_bearing: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetRule {
    pub id: String,
    pub report: String,
    pub metric: String,
    pub unit: String,
    pub claim_scope: String,
    pub direction: BudgetDirection,
    pub status: BudgetRuleStatus,
    pub anchor_tolerance_ratio: Option<f64>,
    pub rolling_tolerance_ratio: Option<f64>,
    pub maximum_spread_ratio: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RollingBaselineManifest {
    pub schema_version: u32,
    pub release: String,
    pub profile: String,
    pub bootstrap_status: BootstrapStatus,
    pub profile_sha256: String,
    pub budget_sha256: String,
    pub selection_reason: String,
    pub policy: RollingPolicy,
    pub anchor: ReleaseAnchor,
    #[serde(rename = "candidate_member")]
    pub candidate_members: Vec<BaselineMember>,
    #[serde(rename = "member")]
    pub members: Vec<BaselineMember>,
    #[serde(rename = "rolling_metric")]
    pub rolling_metrics: Vec<RollingMetric>,
    pub change_control: BaselineChangeControl,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeControlStatus {
    PendingBootstrap,
    Approved,
}

/// Review provenance for a committed baseline update. The proposal and
/// approval both bind the complete anchor/window payload; editing a budget,
/// anchor member, selection pool, or rolling summary therefore invalidates the
/// audit trail as well as the manifest receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineChangeControl {
    pub status: ChangeControlStatus,
    pub proposal: Option<BaselineChangeProposal>,
    pub approval: Option<BaselineChangeApproval>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineChangeProposal {
    pub proposal_id: String,
    pub proposed_at: String,
    pub proposer: String,
    pub rationale: String,
    pub previous_manifest_sha256: String,
    pub proposed_payload_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineChangeApproval {
    pub proposal_sha256: String,
    pub approved_payload_sha256: String,
    pub approved_at: String,
    pub approver: String,
    pub review_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RollingPolicy {
    pub branch: String,
    pub minimum_members: usize,
    pub maximum_members: usize,
    pub maximum_age_days: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseAnchor {
    pub status: BootstrapStatus,
    pub frozen_at: String,
    pub contract_commit: String,
    pub source_run_ids: Vec<String>,
    #[serde(rename = "source_member")]
    pub source_members: Vec<BaselineMember>,
    #[serde(rename = "metric")]
    pub metrics: Vec<AnchorMetric>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnchorMetric {
    pub budget_id: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineMember {
    pub run_id: String,
    pub branch: String,
    pub source_commit: String,
    pub observed_at: String,
    pub successful: bool,
    pub quarantined: bool,
    pub calibration_passed: bool,
    pub spread_stable: bool,
    pub gate_exit_code: i32,
    pub git_status_porcelain_sha256: String,
    pub quarantine_reason: Option<String>,
    pub runner_contract: RunnerContract,
    pub runner_contract_digest: String,
    pub observed_runner: ObservedRunner,
    pub runner_fingerprint: String,
    pub toolchain_identity: String,
    pub prebuild_contract_digest: String,
    pub profile_sha256: String,
    pub budget_sha256: String,
    #[serde(rename = "report")]
    pub reports: Vec<BaselineReportReceipt>,
    #[serde(rename = "metric")]
    pub metrics: Vec<MemberMetric>,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineReportReceipt {
    pub report_id: String,
    pub report_sha256: String,
    pub scenario_digest: String,
    pub workload_digest: String,
    pub slo_digest: String,
    pub methodology_digest: String,
    pub cargo_lock_sha256: String,
    pub prebuild_manifest_sha256: String,
    pub binary_sha256: Vec<BinaryDigest>,
    pub binary_set_digest: String,
    pub stable: bool,
    pub maximum_spread_ratio: f64,
    #[serde(rename = "metric")]
    pub metrics: Vec<ReportMetric>,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemberMetric {
    pub budget_id: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RollingMetric {
    pub budget_id: String,
    pub median: f64,
    pub mad: f64,
    pub tolerance_ratio: f64,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdmissionControlReceipt {
    mode: String,
    factor_millionths: u32,
    authority: String,
    configuration_sha256: String,
    receipt_sha256: String,
}

impl AdmissionControlReceipt {
    fn recomputed_receipt(&self) -> String {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        digest_json(&payload)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum W4DaemonReceiptSource {
    ObservedProcessHarness,
    SelfAsserted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct W4PrebuiltServerBinaryReceipt {
    canonical_path: PathBuf,
    sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct W4DaemonNodeLaunchConfig {
    receipt_kind: String,
    node_id: String,
    client_addr: SocketAddr,
    cluster_addr: SocketAddr,
    admin_addr: SocketAddr,
    redis_addr: Option<SocketAddr>,
    storage_dir: PathBuf,
    cluster_start: String,
    seed_cluster_addrs: Vec<SocketAddr>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct W4DaemonNodeConfigReceipt {
    canonical_path: PathBuf,
    sha256: String,
    launch_config: W4DaemonNodeLaunchConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct W4DaemonNodeProcessReceipt {
    node_id: String,
    pid: u32,
    direct_prebuilt_exec: bool,
    observed_executable_path: PathBuf,
    observed_executable_sha256: String,
    config: W4DaemonNodeConfigReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct W4CapabilityPayload {
    receipt_kind: String,
    receipt_source: W4DaemonReceiptSource,
    execution_mode: String,
    profile: String,
    source_commit: String,
    runner_fingerprint_sha256: String,
    prebuild_manifest_canonical_path: PathBuf,
    prebuild_manifest_sha256: String,
    prebuild_contract_sha256: String,
    provisioner: String,
    direct_prebuilt_exec: bool,
    server_binary: W4PrebuiltServerBinaryReceipt,
    node_count: u8,
    nodes: Vec<W4DaemonNodeProcessReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct W4CapabilityAttestation {
    payload: Option<W4CapabilityPayload>,
    receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct W5EventTimelineReceipt {
    commit_latency_nanos: u64,
    recovery_latency_nanos: u64,
    receipt_sha256: String,
}

impl W5EventTimelineReceipt {
    fn recomputed_receipt(&self) -> String {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        digest_json(&payload)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct W5ControlPlaneProvenance {
    predecessor_artifact_sha256: String,
    predecessor_receipt_sha256: String,
    predecessor_node_count: u8,
    execution_capability_receipt_sha256: String,
    final_cleanup_receipt_sha256: String,
    scenario_sha256: String,
    receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct W5RespProvenance {
    selected_predecessor_artifact_sha256: String,
    selected_predecessor_lifecycle_sha256: String,
    archived_process_receipt_sha256: String,
    selected_capacity: W5RespSelectedCapacityContract,
    capacity_matrix_sha256: String,
    predecessor_scenario_digest: String,
    predecessor_workload_digest: String,
    fresh_selected_execution_receipt_sha256: String,
    fresh_control_execution_receipt_sha256: Vec<String>,
    scenario_sha256: String,
    receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct W5GridModelProvenance {
    w4b_artifact_sha256: String,
    w4b_reference_receipt_sha256: String,
    w4b_scenario_sha256: String,
    source_commit: String,
    runner_fingerprint: String,
    prebuild_manifest_sha256: String,
    fresh_execution: W5GridModelExecutionReceipt,
    w5_scenario_sha256: String,
    receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct W5RespSelectedCapacityContract {
    measurement_id: String,
    scenario_digest: String,
    workload: WorkloadIdentity,
    workload_contract_sha256: String,
    connections: u64,
    pipeline_depth: u64,
    preload_operations: u64,
    warmup_operations: u64,
    steady_operations: u64,
    repeats: u64,
    key_count: u64,
    multi_key_width: u64,
    reset_batch_entries: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct W5GridModelExecutionReceipt {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArchivedProcessLogReceipt {
    canonical_path: PathBuf,
    sha256: String,
    bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArchivedDaemonLifecycleEvidence {
    node_id: String,
    pid: u32,
    kill_requested: bool,
    wait_completed: bool,
    process_no_longer_running: bool,
    exit_status: String,
    stdout_log: ArchivedProcessLogReceipt,
    stderr_log: ArchivedProcessLogReceipt,
    server_binary_path_after: PathBuf,
    server_binary_sha256_after: String,
    node_config_path_after: PathBuf,
    node_config_sha256_after: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct W5ControlPlaneFinalCleanupReceipt {
    nodes: Vec<ArchivedDaemonLifecycleEvidence>,
    receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct W6ReferencePredecessorReceipt {
    profile: String,
    predecessor_report_path: PathBuf,
    predecessor_report_sha256: String,
    predecessor_measurement_id: String,
    predecessor_scenario_sha256: String,
    predecessor_payload_sha256: String,
    predecessor_lifecycle_path: Option<PathBuf>,
    predecessor_lifecycle_sha256: Option<String>,
    source_commit: String,
    cargo_lock_sha256: String,
    runner_fingerprint_sha256: String,
    prebuild_manifest_path: PathBuf,
    prebuild_receipt_sha256: String,
    stable_surface_capability_sha256: String,
    workload_identity_sha256: String,
    archived_execution_receipt_sha256: String,
    archived_execution_pid: u32,
    receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct W6FreshExecutionReceipt {
    schema_version: u32,
    surface: EligibleOverloadSurface,
    kind: ReferenceExecutionKind,
    instance_sequence: u64,
    owning_pid: u32,
    started_unix_nanos: u64,
    direct_prebuilt_exec: bool,
    stable_surface_capability_sha256: String,
    runtime_capability_sha256: Option<String>,
    selected_endpoint: Option<String>,
    receipt_sha256: String,
}

fn receipt_digest_without_field<T>(value: &T) -> String
where
    T: Serialize + Clone + ClearReceipt,
{
    let mut payload = value.clone();
    payload.clear_receipt();
    digest_json(&payload)
}

trait ClearReceipt {
    fn clear_receipt(&mut self);
}

macro_rules! impl_clear_receipt {
    ($type:ty) => {
        impl ClearReceipt for $type {
            fn clear_receipt(&mut self) {
                self.receipt_sha256.clear();
            }
        }
    };
}

impl_clear_receipt!(W5ControlPlaneProvenance);
impl_clear_receipt!(W5RespProvenance);
impl_clear_receipt!(W5GridModelProvenance);
impl_clear_receipt!(W5ControlPlaneFinalCleanupReceipt);
impl_clear_receipt!(W5GridModelExecutionReceipt);
impl_clear_receipt!(W6ReferencePredecessorReceipt);
impl_clear_receipt!(W6FreshExecutionReceipt);

#[derive(Debug, Clone, PartialEq)]
pub struct CandidateReport {
    pub id: String,
    pub path: String,
    pub report_id: String,
    pub report_sha256: String,
    pub claim_scope: String,
    pub run_mode: EvidenceRunMode,
    pub runner_profile: String,
    pub runner_contract_digest: String,
    pub runner_class: String,
    pub runner_fingerprint: String,
    pub source_commit: String,
    pub cargo_lock_sha256: String,
    pub toolchain_identity: String,
    pub prebuild_contract_digest: String,
    pub prebuild_manifest_sha256: String,
    pub binary_sha256: Vec<BinaryDigest>,
    pub binary_set_digest: String,
    pub scenario_digest: String,
    pub workload_digest: String,
    pub slo_digest: String,
    pub methodology_digest: String,
    pub stable: bool,
    pub maximum_spread_ratio: f64,
    pub metrics: BTreeMap<String, ReportMetric>,
}

#[derive(Debug, Clone)]
pub struct ContractBundle {
    pub profile: ProfileContract,
    pub profile_sha256: String,
    pub budget: BudgetContract,
    pub budget_sha256: String,
    pub baseline: RollingBaselineManifest,
    pub baseline_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerdictStatus {
    Passed,
    TripwirePassed,
    Failed,
    TripwireUnavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BudgetCheckRecord {
    pub budget_id: String,
    pub candidate: f64,
    pub anchor: Option<f64>,
    pub rolling_median: f64,
    pub unit: String,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VerdictReportInput {
    pub id: String,
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VerdictBaselineInput {
    pub run_id: String,
    pub source_commit: String,
    pub receipt_sha256: String,
    pub eligible: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BudgetVerdictPayload {
    pub schema_version: u32,
    pub release: String,
    pub profile: String,
    pub enforcement: Enforcement,
    pub candidate_commit: String,
    pub status: VerdictStatus,
    pub profile_sha256: String,
    pub budget_sha256: String,
    pub baseline_sha256: String,
    pub report_set_digest: String,
    pub reports: Vec<VerdictReportInput>,
    pub baseline_members: Vec<VerdictBaselineInput>,
    pub checks: Vec<BudgetCheckRecord>,
    pub problems: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BudgetVerdict {
    pub payload: BudgetVerdictPayload,
    pub receipt_sha256: String,
}

impl BudgetVerdict {
    pub fn new(payload: BudgetVerdictPayload) -> Self {
        let receipt_sha256 = digest_json(&payload);
        Self {
            payload,
            receipt_sha256,
        }
    }

    pub fn receipt_is_valid(&self) -> bool {
        self.receipt_sha256 == digest_json(&self.payload)
    }
}

pub fn run(args: Vec<String>) -> Result<(), PerfBudgetError> {
    let options = Options::parse(args)?;
    let root = options.root;
    let bundle = load_bundle(&root, &options.release, &options.profile)?;
    let mut problems = validate_contract_bundle(&bundle);
    let reports = match load_candidate_reports(&root, &bundle.budget) {
        Ok(reports) => reports,
        Err(error) => {
            problems.push(error.to_string());
            Vec::new()
        }
    };
    problems.extend(validate_checkout_receipt(&root, &bundle, &reports));
    let now = OffsetDateTime::now_utc();
    let mut verdict = evaluate(&bundle, &reports, now);
    verdict.payload.problems.extend(problems);
    verdict.payload.problems.sort();
    verdict.payload.problems.dedup();
    if !verdict.payload.problems.is_empty() {
        verdict.payload.status = match bundle.profile.enforcement {
            Enforcement::Ship => VerdictStatus::Failed,
            Enforcement::NonEnforcingTripwire => VerdictStatus::TripwireUnavailable,
        };
    }
    verdict = BudgetVerdict::new(verdict.payload);
    write_verdict(&root, &verdict)?;
    match verdict.payload.status {
        VerdictStatus::Passed => {
            println!("perf-budget-check: reference budgets passed");
            Ok(())
        }
        VerdictStatus::TripwirePassed => {
            println!("perf-budget-check: non-enforcing shared-runner tripwire passed");
            Ok(())
        }
        VerdictStatus::TripwireUnavailable => {
            println!(
                "perf-budget-check: non-enforcing tripwire unavailable: {:?}",
                verdict.payload.problems
            );
            Ok(())
        }
        VerdictStatus::Failed => Err(PerfBudgetError::new(format!(
            "perf-budget-check failed closed: {:?}",
            verdict.payload.problems
        ))),
    }
}

#[derive(Debug)]
struct Options {
    root: PathBuf,
    release: String,
    profile: String,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, PerfBudgetError> {
        let mut root = PathBuf::from(".");
        let mut release = None;
        let mut profile = None;
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--root" => root = next_path(&mut iter, "--root")?,
                "--release" => release = Some(next_string(&mut iter, "--release")?),
                "--profile" => profile = Some(next_string(&mut iter, "--profile")?),
                other => {
                    return Err(PerfBudgetError::new(format!(
                        "unsupported perf-budget-check argument: {other}"
                    )));
                }
            }
        }
        let release = release.ok_or_else(|| PerfBudgetError::new("--release is required"))?;
        if release != "0.67" && release != RELEASE {
            return Err(PerfBudgetError::new("only release 0.67 is supported"));
        }
        let profile = profile.ok_or_else(|| PerfBudgetError::new("--profile is required"))?;
        if profile != "reference-v1" && profile != "ci-shared" {
            return Err(PerfBudgetError::new(format!(
                "unknown performance profile {profile:?}"
            )));
        }
        Ok(Self {
            root,
            release: RELEASE.to_owned(),
            profile,
        })
    }
}

fn next_path(
    iter: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<PathBuf, PerfBudgetError> {
    next_string(iter, flag).map(PathBuf::from)
}

fn next_string(
    iter: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<String, PerfBudgetError> {
    iter.next()
        .ok_or_else(|| PerfBudgetError::new(format!("{flag} requires a value")))
}

pub fn load_bundle(
    root: &Path,
    release: &str,
    profile: &str,
) -> Result<ContractBundle, PerfBudgetError> {
    if release != RELEASE {
        return Err(PerfBudgetError::new(format!(
            "unsupported performance release {release:?}"
        )));
    }
    let profile_path = root.join(PROFILE_ROOT).join(format!("{profile}.toml"));
    let budget_path = root.join(BUDGET_ROOT).join(format!("{profile}.toml"));
    let baseline_path = root.join(BASELINE_ROOT).join(format!("{profile}.toml"));
    let profile_bytes = read_bounded(&profile_path)?;
    let budget_bytes = read_bounded(&budget_path)?;
    let baseline_bytes = read_bounded(&baseline_path)?;
    let profile: ProfileContract = parse_toml(&profile_path, &profile_bytes)?;
    let budget: BudgetContract = parse_toml(&budget_path, &budget_bytes)?;
    let baseline: RollingBaselineManifest = parse_toml(&baseline_path, &baseline_bytes)?;
    let profile_sha256 = digest_json(&profile);
    let budget_sha256 = digest_json(&budget);
    let baseline_sha256 = digest_json(&baseline);
    Ok(ContractBundle {
        profile,
        profile_sha256,
        budget,
        budget_sha256,
        baseline,
        baseline_sha256,
    })
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, PerfBudgetError> {
    let metadata = fs::metadata(path).map_err(|error| {
        PerfBudgetError::new(format!("reading metadata for {}: {error}", path.display()))
    })?;
    if metadata.len() > 4 * 1024 * 1024 {
        return Err(PerfBudgetError::new(format!(
            "{} exceeds the 4 MiB contract cap",
            path.display()
        )));
    }
    fs::read(path)
        .map_err(|error| PerfBudgetError::new(format!("reading {}: {error}", path.display())))
}

fn validate_checkout_receipt(
    root: &Path,
    bundle: &ContractBundle,
    reports: &[CandidateReport],
) -> Vec<String> {
    let mut problems = Vec::new();
    let canonical_root = match fs::canonicalize(root) {
        Ok(root) => root,
        Err(error) => {
            return vec![format!(
                "unable to canonicalize candidate checkout {}: {error}",
                root.display()
            )];
        }
    };
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(&canonical_root)
            .args(args)
            .output()
    };
    let head = match git(&["rev-parse", "HEAD"]) {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_owned()
        }
        Ok(_) | Err(_) => {
            problems.push("unable to resolve the candidate checkout commit".to_owned());
            String::new()
        }
    };
    if !is_git_commit(&head) || reports.iter().any(|report| report.source_commit != head) {
        problems.push("input report commit differs from the checked-out candidate HEAD".to_owned());
    }
    match Command::new("rustc").arg("--version").output() {
        Ok(output)
            if output.status.success()
                && canonical_toolchain_identity(&String::from_utf8_lossy(&output.stdout))
                    .is_ok_and(|toolchain| {
                        toolchain == bundle.profile.prebuild.toolchain_identity
                    }) => {}
        Ok(_) | Err(_) => problems.push(
            "live rustc toolchain differs from the exact reviewed prebuild contract".to_owned(),
        ),
    }
    problems.extend(validate_prebuild_artifacts(
        &canonical_root,
        bundle,
        reports,
        &head,
    ));
    match git(&[
        "status",
        "--porcelain=v1",
        "--untracked-files=normal",
        "--ignore-submodules=none",
    ]) {
        Ok(output) if output.status.success() && output.stdout.is_empty() => {}
        Ok(_) | Err(_) => problems.push(
            "candidate checkout is dirty or its exact status could not be verified".to_owned(),
        ),
    }
    if bundle.profile.enforcement == Enforcement::Ship
        && bundle.baseline.bootstrap_status == BootstrapStatus::Bootstrapped
        && is_git_commit(&head)
    {
        let mut ancestors = bundle
            .baseline
            .members
            .iter()
            .map(|member| member.source_commit.as_str())
            .collect::<Vec<_>>();
        ancestors.extend(
            bundle
                .baseline
                .anchor
                .source_members
                .iter()
                .map(|member| member.source_commit.as_str()),
        );
        ancestors.push(bundle.baseline.anchor.contract_commit.as_str());
        for ancestor in ancestors {
            match git(&["merge-base", "--is-ancestor", ancestor, &head]) {
                Ok(output) if output.status.success() => {}
                Ok(_) | Err(_) => problems.push(format!(
                    "baseline/anchor commit {ancestor} is not an ancestor of candidate {head}"
                )),
            }
        }
        for source in &bundle.baseline.anchor.source_members {
            match git(&[
                "merge-base",
                "--is-ancestor",
                &source.source_commit,
                &bundle.baseline.anchor.contract_commit,
            ]) {
                Ok(output) if output.status.success() => {}
                Ok(_) | Err(_) => problems.push(format!(
                    "anchor source commit {} is not an ancestor of anchor contract {}",
                    source.source_commit, bundle.baseline.anchor.contract_commit
                )),
            }
        }
    }
    problems
}

fn validate_prebuild_artifacts(
    root: &Path,
    bundle: &ContractBundle,
    reports: &[CandidateReport],
    head: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    let cargo_lock_path = root.join("Cargo.lock");
    let cargo_lock_sha256 = match sha256_file(&cargo_lock_path) {
        Ok(digest) => digest,
        Err(error) => {
            problems.push(error.to_string());
            String::new()
        }
    };
    let manifest_path = root.join(PREBUILD_MANIFEST_PATH);
    let manifest_bytes = match read_bounded(&manifest_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            problems.push(format!("prebuild manifest is unavailable: {error}"));
            return problems;
        }
    };
    let manifest_sha256 = sha256(&manifest_bytes);
    let manifest: PerfPrebuildManifest = match serde_json::from_slice(&manifest_bytes) {
        Ok(manifest) => manifest,
        Err(error) => {
            problems.push(format!("prebuild manifest is invalid: {error}"));
            return problems;
        }
    };
    let prebuild = &bundle.profile.prebuild;
    let live_platform_key = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    if manifest.schema_version != 1
        || manifest.source.git_commit != head
        || manifest.source.cargo_lock_sha256 != cargo_lock_sha256
        || canonical_toolchain_identity(&manifest.toolchain_identity)
            .ok()
            .as_deref()
            != Some(prebuild.toolchain_identity.as_str())
        || manifest.target_set != prebuild.target_set
        || manifest.features != prebuild.features
        || manifest.cargo_profile != prebuild.cargo_profile
        || manifest.flags != prebuild.flags
        || manifest.build_recipe != prebuild.build_recipe
        || manifest.build_contract_digest != prebuild.digest
        || manifest.runner_profile != bundle.profile.name
        || manifest.platform_key != bundle.profile.required_platform_key
        || manifest.platform_key != live_platform_key
        || !bundle
            .profile
            .runner
            .allowed_fingerprints
            .contains(&manifest.runner_fingerprint)
    {
        problems.push(
            "prebuild manifest does not match the exact candidate/profile/build contract"
                .to_owned(),
        );
    }

    let mut binaries = Vec::with_capacity(manifest.binaries.len());
    let mut binary_ids = BTreeSet::new();
    for binary in &manifest.binaries {
        let canonical_path = match fs::canonicalize(&binary.canonical_path) {
            Ok(path) => path,
            Err(error) => {
                problems.push(format!(
                    "receipt-bound binary {} is unavailable: {error}",
                    binary.id
                ));
                continue;
            }
        };
        let expected_path = root.join("target").join("release").join(format!(
            "{}{}",
            binary.id,
            std::env::consts::EXE_SUFFIX
        ));
        let expected_path = fs::canonicalize(&expected_path).ok();
        let regular_file = fs::metadata(&canonical_path).is_ok_and(|metadata| metadata.is_file());
        if !binary.canonical_path.is_absolute()
            || canonical_path != binary.canonical_path
            || expected_path.as_ref() != Some(&canonical_path)
            || !regular_file
            || !binary_ids.insert(binary.id.as_str())
            || binary.id.trim().is_empty()
            || !is_sha256(&binary.sha256)
        {
            problems.push(format!(
                "receipt-bound binary {} has an unsafe/duplicate identity",
                binary.id
            ));
            continue;
        }
        match sha256_file(&canonical_path) {
            Ok(actual) if actual == binary.sha256 => binaries.push(BinaryDigest {
                id: binary.id.clone(),
                sha256: actual,
            }),
            Ok(_) => problems.push(format!(
                "receipt-bound binary {} changed after the prebuild gate",
                binary.id
            )),
            Err(error) => problems.push(error.to_string()),
        }
    }
    binaries.sort_by(|left, right| left.id.cmp(&right.id));
    let expected_ids = prebuild.target_set.iter().cloned().collect::<BTreeSet<_>>();
    let observed_ids = binaries
        .iter()
        .map(|binary| binary.id.clone())
        .collect::<BTreeSet<_>>();
    let binary_set_digest = digest_json(&binaries);
    if expected_ids != observed_ids
        || observed_ids.len() != binaries.len()
        || validate_binary_set(&binaries, &binary_set_digest).is_err()
    {
        problems.push("prebuild manifest does not bind the exact required binary set".to_owned());
    }
    for report in reports {
        if report.cargo_lock_sha256 != cargo_lock_sha256
            || report.prebuild_manifest_sha256 != manifest_sha256
            || report.binary_sha256 != binaries
            || report.binary_set_digest != binary_set_digest
            || report.runner_fingerprint != manifest.runner_fingerprint
        {
            problems.push(format!(
                "candidate report {} does not match the re-hashed prebuild receipt",
                report.id
            ));
        }
    }
    problems
}

fn sha256_file(path: &Path) -> Result<String, PerfBudgetError> {
    let mut file = fs::File::open(path)
        .map_err(|error| PerfBudgetError::new(format!("opening {}: {error}", path.display())))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            PerfBudgetError::new(format!("hashing {}: {error}", path.display()))
        })?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn parse_toml<T: for<'de> Deserialize<'de>>(
    path: &Path,
    bytes: &[u8],
) -> Result<T, PerfBudgetError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        PerfBudgetError::new(format!("{} is not UTF-8: {error}", path.display()))
    })?;
    toml::from_str(text)
        .map_err(|error| PerfBudgetError::new(format!("parsing {}: {error}", path.display())))
}

pub fn validate_contract_bundle(bundle: &ContractBundle) -> Vec<String> {
    let mut problems = Vec::new();
    validate_profile(&bundle.profile, &mut problems);
    validate_budget(&bundle.budget, &mut problems);
    if bundle.profile.release != bundle.budget.release
        || bundle.profile.name != bundle.budget.profile
        || bundle.profile.enforcement != bundle.budget.enforcement
        || bundle.profile.bootstrap_status != bundle.budget.bootstrap_status
    {
        problems.push("profile and budget identities are mixed".to_owned());
    }
    for rule in &bundle.budget.budgets {
        if rule.status == BudgetRuleStatus::Active
            && rule
                .maximum_spread_ratio
                .is_some_and(|limit| limit > bundle.profile.noise.maximum_report_spread_ratio)
        {
            problems.push(format!(
                "budget {} weakens the committed profile spread ceiling",
                rule.id
            ));
        }
    }
    validate_baseline(bundle, &mut problems);
    problems.sort();
    problems.dedup();
    problems
}

fn validate_profile(profile: &ProfileContract, problems: &mut Vec<String>) {
    if profile.schema_version != 1
        || profile.release != RELEASE
        || profile.name != profile.runner.name
        || profile.runner.required_runner_class.trim().is_empty()
        || profile.required_platform_key.trim().is_empty()
        || profile.runner.minimum_logical_cores == 0
        || profile.runner.required_cpu_affinity.trim().is_empty()
        || profile.runner.required_cgroup_cpu_quota.trim().is_empty()
    {
        problems.push("performance profile identity is incomplete or unsupported".to_owned());
    }
    if !finite_ratio(profile.runner.maximum_calibration_score)
        || !finite_ratio(profile.noise.maximum_report_spread_ratio)
    {
        problems.push("profile calibration/spread tolerance is invalid".to_owned());
    }
    if profile.runner.allowed_fingerprints.iter().any(|value| {
        value.trim().is_empty()
            || profile
                .runner
                .allowed_fingerprints
                .iter()
                .filter(|x| *x == value)
                .count()
                != 1
    }) {
        problems.push("runner fingerprint allow-list contains an empty/duplicate value".to_owned());
    }
    if profile.prebuild.schema_version != 1
        || profile.prebuild.computed_digest() != profile.prebuild.digest
        || canonical_toolchain_identity(&profile.prebuild.toolchain_identity).is_err()
    {
        problems.push("prebuild_contract_digest does not recompute".to_owned());
    }
    let exact_prebuild = profile.prebuild.toolchain_identity == "rustc-1.94.0"
        && profile.prebuild.target_set == ["hydracache-loadgen", "hydracache-server"]
        && profile.prebuild.features.is_empty()
        && profile.prebuild.cargo_profile == "release"
        && profile.prebuild.flags == ["--locked", "--release"]
        && profile.prebuild.build_recipe
            == ["cargo build -p hydracache-loadgen -p hydracache-server --release --locked"];
    if profile.required_platform_key != "linux-x86_64" || !exact_prebuild {
        problems.push("W7 profile changed the frozen platform/prebuild contract".to_owned());
    }
    match profile.enforcement {
        Enforcement::Ship => {
            if profile.name != "reference-v1"
                || profile.runner.required_runner_class != "reference-v1"
                || profile.runner.minimum_logical_cores != 8
                || profile.runner.required_cpu_affinity != "dedicated-cpuset"
                || profile.runner.required_cgroup_cpu_quota != "unlimited"
                || !profile.runner.require_dedicated
                || !approx_eq(profile.runner.maximum_calibration_score, 0.05)
                || profile.noise.comparison_class != "same-dedicated-runner-fingerprint"
                || !profile.noise.absolute_numbers_are_ship_evidence
                || !approx_eq(profile.noise.maximum_report_spread_ratio, 0.05)
            {
                problems.push("reference-v1 must be the dedicated enforcing profile".to_owned());
            }
        }
        Enforcement::NonEnforcingTripwire => {
            if profile.name != "ci-shared"
                || profile.runner.required_runner_class != "github-hosted-linux-x64"
                || profile.runner.minimum_logical_cores != 2
                || profile.runner.required_cpu_affinity != "runner-managed"
                || profile.runner.required_cgroup_cpu_quota != "runner-managed"
                || profile.runner.require_dedicated
                || !approx_eq(profile.runner.maximum_calibration_score, 0.25)
                || profile.noise.absolute_numbers_are_ship_evidence
                || profile.noise.comparison_class != "rolling-same-shared-runner-class"
                || !approx_eq(profile.noise.maximum_report_spread_ratio, 0.30)
            {
                problems.push("ci-shared must remain a non-enforcing rolling tripwire".to_owned());
            }
        }
    }
    if profile.bootstrap_status == BootstrapStatus::Bootstrapped
        && (profile.runner.allowed_fingerprints.is_empty()
            || profile.prebuild.toolchain_identity.trim().is_empty()
            || !is_sha256(&profile.prebuild.digest))
    {
        problems.push("bootstrapped profile lacks immutable runner/build identity".to_owned());
    }
}

fn validate_budget(budget: &BudgetContract, problems: &mut Vec<String>) {
    if budget.schema_version != 1 || budget.release != RELEASE {
        problems.push("budget schema/release identity is unsupported".to_owned());
    }
    let expected = EXPECTED_REPORTS_067
        .iter()
        .map(|(id, path)| ((*id).to_owned(), (*path).to_owned()))
        .collect::<BTreeSet<_>>();
    let observed = budget
        .reports
        .iter()
        .map(|report| (report.id.clone(), report.path.clone()))
        .collect::<BTreeSet<_>>();
    if observed != expected || observed.len() != budget.reports.len() {
        problems.push(
            "budget must declare the exact W1-W6 report set with no missing/extra/duplicate paths"
                .to_owned(),
        );
    }
    let reports = budget
        .reports
        .iter()
        .map(|report| (report.id.as_str(), report))
        .collect::<BTreeMap<_, _>>();
    for report in &budget.reports {
        if !safe_target_report_path(&report.path)
            || report.report_id.trim().is_empty()
            || report.claim_scope.trim().is_empty()
        {
            problems.push(format!(
                "expected report {} has an unsafe/incomplete identity",
                report.id
            ));
        }
    }
    let mut budget_ids = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for rule in &budget.budgets {
        if !budget_ids.insert(rule.id.as_str())
            || !keys.insert((rule.report.as_str(), rule.metric.as_str()))
            || rule.id.trim().is_empty()
            || rule.metric.trim().is_empty()
            || rule.unit.trim().is_empty()
        {
            problems
                .push("budget ids and report/metric keys must be unique and non-empty".to_owned());
        }
        match reports.get(rule.report.as_str()) {
            Some(report) if report.claim_scope == rule.claim_scope => {}
            _ => problems.push(format!(
                "budget {} references an absent report or mismatched claim_scope",
                rule.id
            )),
        }
        match rule.status {
            BudgetRuleStatus::Unbootstrapped => {
                if rule.anchor_tolerance_ratio.is_some()
                    || rule.rolling_tolerance_ratio.is_some()
                    || rule.maximum_spread_ratio.is_some()
                {
                    problems.push(format!(
                        "unbootstrapped budget {} must not invent thresholds",
                        rule.id
                    ));
                }
            }
            BudgetRuleStatus::Active => {
                let anchor_is_valid = match budget.enforcement {
                    Enforcement::Ship => rule.anchor_tolerance_ratio.is_some_and(finite_ratio),
                    Enforcement::NonEnforcingTripwire => rule.anchor_tolerance_ratio.is_none(),
                };
                if !anchor_is_valid
                    || !rule.rolling_tolerance_ratio.is_some_and(finite_ratio)
                    || !rule.maximum_spread_ratio.is_some_and(finite_ratio)
                {
                    problems.push(format!("active budget {} has invalid thresholds", rule.id));
                }
            }
        }
    }
    for report in &budget.reports {
        if !budget.budgets.iter().any(|rule| rule.report == report.id) {
            problems.push(format!("expected report {} has no macro budget", report.id));
        }
    }
    let expected_status = match budget.bootstrap_status {
        BootstrapStatus::Unbootstrapped => BudgetRuleStatus::Unbootstrapped,
        BootstrapStatus::Bootstrapped => BudgetRuleStatus::Active,
    };
    if budget
        .budgets
        .iter()
        .any(|rule| rule.status != expected_status)
    {
        problems.push("budget bootstrap state is mixed across rows".to_owned());
    }
}

fn validate_baseline(bundle: &ContractBundle, problems: &mut Vec<String>) {
    let manifest = &bundle.baseline;
    if manifest.schema_version != 1
        || manifest.release != RELEASE
        || manifest.profile != bundle.profile.name
        || manifest.bootstrap_status != bundle.budget.bootstrap_status
    {
        problems.push("rolling baseline identity/bootstrap state is mixed".to_owned());
    }
    if manifest.profile_sha256 != bundle.profile_sha256
        || manifest.budget_sha256 != bundle.budget_sha256
    {
        problems.push(
            "rolling baseline does not bind the reviewed profile/budget file digests".to_owned(),
        );
    }
    if manifest.selection_reason != "most-recent-eligible-successful-main-medians"
        || manifest.policy.branch != "main"
        || manifest.policy.minimum_members != DEFAULT_MINIMUM_MEMBERS
        || manifest.policy.maximum_members != DEFAULT_MAXIMUM_MEMBERS
        || manifest.policy.maximum_age_days != DEFAULT_MAXIMUM_AGE_DAYS
    {
        problems.push(
            "rolling baseline policy differs from the reviewed 5/10/30d main contract".to_owned(),
        );
    }
    validate_change_control(manifest, problems);
    match manifest.bootstrap_status {
        BootstrapStatus::Unbootstrapped => {
            if manifest.anchor.status != BootstrapStatus::Unbootstrapped
                || !manifest.anchor.frozen_at.is_empty()
                || !manifest.anchor.contract_commit.is_empty()
                || !manifest.anchor.source_run_ids.is_empty()
                || !manifest.anchor.source_members.is_empty()
                || !manifest.anchor.metrics.is_empty()
                || !manifest.candidate_members.is_empty()
                || !manifest.members.is_empty()
                || !manifest.rolling_metrics.is_empty()
                || manifest.receipt_sha256 != "unbootstrapped"
            {
                problems.push(
                    "unbootstrapped baseline must contain no invented observations".to_owned(),
                );
            }
        }
        BootstrapStatus::Bootstrapped => {
            if manifest.receipt_sha256 != baseline_manifest_receipt(manifest) {
                problems.push("baseline manifest receipt digest does not recompute".to_owned());
            }
            match bundle.profile.enforcement {
                Enforcement::Ship => {
                    if manifest.anchor.status != BootstrapStatus::Bootstrapped
                        || parse_time(&manifest.anchor.frozen_at).is_err()
                        || !is_git_commit(&manifest.anchor.contract_commit)
                        || manifest.anchor.source_run_ids.len() < manifest.policy.minimum_members
                        || manifest
                            .anchor
                            .source_run_ids
                            .iter()
                            .any(|run_id| run_id.trim().is_empty())
                        || manifest
                            .anchor
                            .source_run_ids
                            .iter()
                            .collect::<BTreeSet<_>>()
                            .len()
                            != manifest.anchor.source_run_ids.len()
                        || manifest.anchor.source_members.len()
                            != manifest.anchor.source_run_ids.len()
                    {
                        problems
                            .push("immutable release anchor provenance is incomplete".to_owned());
                    }
                    validate_anchor_coverage(&bundle.budget, &manifest.anchor, problems);
                    validate_anchor_sources(bundle, problems);
                }
                Enforcement::NonEnforcingTripwire => {
                    if !anchor_is_empty(&manifest.anchor) {
                        problems.push(
                            "ci-shared rolling tripwire must not acquire an absolute release anchor"
                                .to_owned(),
                        );
                    }
                }
            }
            if manifest.members.len() < manifest.policy.minimum_members
                || manifest.members.len() > manifest.policy.maximum_members
            {
                problems.push(
                    "rolling baseline window is insufficient or exceeds its maximum".to_owned(),
                );
            }
            if manifest.candidate_members.len() < manifest.members.len() {
                problems.push(
                    "rolling selection pool is smaller than the committed selected window"
                        .to_owned(),
                );
            }
            let pool_ids = manifest
                .candidate_members
                .iter()
                .map(|member| member.run_id.as_str())
                .collect::<BTreeSet<_>>();
            if pool_ids.len() != manifest.candidate_members.len()
                || manifest.candidate_members.iter().any(|member| {
                    member.receipt_sha256 != baseline_member_receipt(member)
                        || !baseline_member_semantics_valid(bundle, member)
                })
            {
                problems.push(
                    "rolling selection pool contains duplicate or invalid receipt-bound members"
                        .to_owned(),
                );
            }
            if manifest.members.iter().any(|member| {
                !manifest
                    .candidate_members
                    .iter()
                    .any(|candidate| candidate == member)
            }) {
                problems.push(
                    "rolling selected window contains a member absent from its audited pool"
                        .to_owned(),
                );
            }
            let expected_ids = bundle
                .budget
                .budgets
                .iter()
                .map(|rule| rule.id.as_str())
                .collect::<BTreeSet<_>>();
            let run_ids = manifest
                .members
                .iter()
                .map(|member| member.run_id.as_str())
                .collect::<BTreeSet<_>>();
            if run_ids.len() != manifest.members.len()
                || manifest.members.iter().any(|member| {
                    member.run_id.trim().is_empty()
                        || parse_time(&member.observed_at).is_err()
                        || !is_git_commit(&member.source_commit)
                })
            {
                problems.push(
                    "rolling baseline member ids/source commits/timestamps are invalid or duplicated"
                        .to_owned(),
                );
            }
            for member in &manifest.members {
                if !baseline_member_semantics_valid(bundle, member) {
                    problems.push(format!(
                        "baseline member {} outcome/clean-checkout/runner/spread evidence does not recompute",
                        member.run_id
                    ));
                }
                if member.receipt_sha256 != baseline_member_receipt(member) {
                    problems.push(format!(
                        "baseline member {} receipt digest does not recompute",
                        member.run_id
                    ));
                }
                for report in &member.reports {
                    if !valid_baseline_report(report)
                        || report.receipt_sha256 != baseline_report_receipt(report)
                    {
                        problems.push(format!(
                            "baseline report {} receipt digest does not recompute",
                            report.report_id
                        ));
                    }
                    let rules = bundle
                        .budget
                        .budgets
                        .iter()
                        .filter(|rule| rule.report == report.report_id)
                        .collect::<Vec<_>>();
                    let observed = report
                        .metrics
                        .iter()
                        .map(|metric| metric.id.as_str())
                        .collect::<BTreeSet<_>>();
                    let expected = rules
                        .iter()
                        .map(|rule| rule.metric.as_str())
                        .collect::<BTreeSet<_>>();
                    if observed != expected || observed.len() != report.metrics.len() {
                        problems.push(format!(
                            "baseline report {} has a partial/duplicate budget metric set",
                            report.report_id
                        ));
                    }
                    for rule in rules {
                        let report_metric = report
                            .metrics
                            .iter()
                            .find(|metric| metric.id == rule.metric);
                        let member_metric = member
                            .metrics
                            .iter()
                            .find(|metric| metric.budget_id == rule.id);
                        if report_metric.zip(member_metric).is_none_or(
                            |(report_metric, member_metric)| {
                                report_metric.unit != rule.unit
                                    || member_metric.unit != rule.unit
                                    || !report_metric.value.is_finite()
                                    || report_metric.value <= 0.0
                                    || !approx_eq(report_metric.value, member_metric.value)
                            },
                        ) {
                            problems.push(format!(
                                "baseline member {} metric {} is not derived from its report receipt",
                                member.run_id, rule.id
                            ));
                        }
                    }
                }
                let expected_reports = bundle
                    .budget
                    .reports
                    .iter()
                    .map(|report| report.id.as_str())
                    .collect::<BTreeSet<_>>();
                let observed_reports = member
                    .reports
                    .iter()
                    .map(|report| report.report_id.as_str())
                    .collect::<BTreeSet<_>>();
                if expected_reports != observed_reports
                    || observed_reports.len() != member.reports.len()
                {
                    problems.push(format!(
                        "baseline member {} has a partial/duplicate report set",
                        member.run_id
                    ));
                }
                let ids = member
                    .metrics
                    .iter()
                    .map(|metric| metric.budget_id.as_str())
                    .collect::<BTreeSet<_>>();
                if ids != expected_ids || ids.len() != member.metrics.len() {
                    problems.push(format!(
                        "baseline member {} has a partial/duplicate metric set",
                        member.run_id
                    ));
                }
                for metric in &member.metrics {
                    let valid = bundle
                        .budget
                        .budgets
                        .iter()
                        .find(|rule| rule.id == metric.budget_id)
                        .is_some_and(|rule| {
                            metric.unit == rule.unit
                                && metric.value.is_finite()
                                && metric.value > 0.0
                        });
                    if !valid {
                        problems.push(format!(
                            "baseline member {} has an invalid value/unit for {}",
                            member.run_id, metric.budget_id
                        ));
                    }
                }
            }
            validate_rolling_metrics(&bundle.budget, manifest, problems);
        }
    }
}

fn validate_change_control(manifest: &RollingBaselineManifest, problems: &mut Vec<String>) {
    match manifest.bootstrap_status {
        BootstrapStatus::Unbootstrapped => {
            if manifest.change_control.status != ChangeControlStatus::PendingBootstrap
                || manifest.change_control.proposal.is_some()
                || manifest.change_control.approval.is_some()
            {
                problems.push(
                    "unbootstrapped baseline must retain an empty pending-bootstrap change control"
                        .to_owned(),
                );
            }
        }
        BootstrapStatus::Bootstrapped => {
            let Some(proposal) = manifest.change_control.proposal.as_ref() else {
                problems.push("bootstrapped baseline has no reviewed change proposal".to_owned());
                return;
            };
            let Some(approval) = manifest.change_control.approval.as_ref() else {
                problems.push("bootstrapped baseline has no independent approval".to_owned());
                return;
            };
            let proposed_at = parse_time(&proposal.proposed_at).ok();
            let approved_at = parse_time(&approval.approved_at).ok();
            let frozen_at = parse_time(&manifest.anchor.frozen_at).ok();
            let payload_sha256 = baseline_payload_digest(manifest);
            if manifest.change_control.status != ChangeControlStatus::Approved
                || proposal.proposal_id.trim().is_empty()
                || proposal.proposer.trim().is_empty()
                || proposal.rationale.trim().is_empty()
                || !is_sha256(&proposal.previous_manifest_sha256)
                || proposal.previous_manifest_sha256 == payload_sha256
                || proposal.previous_manifest_sha256 == manifest.receipt_sha256
                || proposal.proposed_payload_sha256 != payload_sha256
                || approval.proposal_sha256 != digest_json(proposal)
                || approval.approved_payload_sha256 != payload_sha256
                || approval.approver.trim().is_empty()
                || approval.review_reference.trim().is_empty()
                || approval.approver == proposal.proposer
                || proposed_at
                    .zip(approved_at)
                    .is_none_or(|(proposed, approved)| proposed > approved)
                || (manifest.anchor.status == BootstrapStatus::Bootstrapped
                    && frozen_at
                        .zip(proposed_at)
                        .is_none_or(|(frozen, proposed)| frozen > proposed))
            {
                problems.push(
                    "baseline proposal/independent approval audit trail does not bind the exact payload"
                        .to_owned(),
                );
            }
        }
    }
}

fn validate_anchor_coverage(
    budget: &BudgetContract,
    anchor: &ReleaseAnchor,
    problems: &mut Vec<String>,
) {
    let anchors = anchor
        .metrics
        .iter()
        .map(|metric| (metric.budget_id.as_str(), metric))
        .collect::<BTreeMap<_, _>>();
    let expected_ids = budget
        .budgets
        .iter()
        .map(|rule| rule.id.as_str())
        .collect::<BTreeSet<_>>();
    let observed_ids = anchor
        .metrics
        .iter()
        .map(|metric| metric.budget_id.as_str())
        .collect::<BTreeSet<_>>();
    if expected_ids != observed_ids || observed_ids.len() != anchor.metrics.len() {
        problems.push("release anchor has missing, extra, or duplicate metrics".to_owned());
    }
    for rule in &budget.budgets {
        match anchors.get(rule.id.as_str()) {
            Some(metric)
                if metric.unit == rule.unit && metric.value.is_finite() && metric.value > 0.0 => {}
            _ => problems.push(format!(
                "budget {} lacks a valid reference-v1 anchor",
                rule.id
            )),
        }
    }
    let capacity_reports = budget
        .reports
        .iter()
        .filter(|report| report.capacity_bearing)
        .map(|report| report.id.as_str())
        .collect::<BTreeSet<_>>();
    for report in capacity_reports {
        if !budget
            .budgets
            .iter()
            .any(|rule| rule.report == report && anchors.contains_key(rule.id.as_str()))
        {
            problems.push(format!(
                "capacity-bearing surface {report} has no reference-v1 anchor"
            ));
        }
    }
}

fn validate_anchor_sources(bundle: &ContractBundle, problems: &mut Vec<String>) {
    let anchor = &bundle.baseline.anchor;
    let source_ids = anchor
        .source_members
        .iter()
        .map(|member| member.run_id.as_str())
        .collect::<BTreeSet<_>>();
    let declared_ids = anchor
        .source_run_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if source_ids != declared_ids || source_ids.len() != anchor.source_members.len() {
        problems
            .push("release anchor source ids do not match immutable source receipts".to_owned());
    }
    let frozen_at = parse_time(&anchor.frozen_at).ok();
    let common_fingerprint = anchor
        .source_members
        .first()
        .map(|member| member.runner_fingerprint.as_str());
    let common_toolchain = anchor
        .source_members
        .first()
        .map(|member| member.toolchain_identity.as_str());
    let common_prebuild = anchor
        .source_members
        .first()
        .map(|member| member.prebuild_contract_digest.as_str());
    for member in &anchor.source_members {
        let observed_at = parse_time(&member.observed_at).ok();
        if member.branch != "main"
            || !member.successful
            || member.quarantined
            || !member.calibration_passed
            || !member.spread_stable
            || member.profile_sha256 != bundle.baseline.profile_sha256
            || member.budget_sha256 != bundle.baseline.budget_sha256
            || common_fingerprint != Some(member.runner_fingerprint.as_str())
            || common_toolchain != Some(member.toolchain_identity.as_str())
            || common_prebuild != Some(member.prebuild_contract_digest.as_str())
            || !bundle
                .profile
                .runner
                .allowed_fingerprints
                .contains(&member.runner_fingerprint)
            || member.toolchain_identity != bundle.profile.prebuild.toolchain_identity
            || member.prebuild_contract_digest != bundle.profile.prebuild.digest
            || observed_at
                .zip(frozen_at)
                .is_none_or(|(observed, frozen)| observed > frozen)
            || !is_git_commit(&member.source_commit)
            || member.receipt_sha256 != baseline_member_receipt(member)
            || !baseline_member_semantics_valid(bundle, member)
        {
            problems.push(format!(
                "release anchor source {} is not an immutable eligible main receipt",
                member.run_id
            ));
        }
    }
    if let Some(first) = anchor.source_members.first() {
        for source in anchor.source_members.iter().skip(1) {
            for report in &first.reports {
                let comparable = source
                    .reports
                    .iter()
                    .find(|candidate| candidate.report_id == report.report_id);
                if comparable.is_none_or(|candidate| {
                    candidate.scenario_digest != report.scenario_digest
                        || candidate.workload_digest != report.workload_digest
                        || candidate.slo_digest != report.slo_digest
                        || candidate.methodology_digest != report.methodology_digest
                }) {
                    problems.push(format!(
                        "release anchor source {} mixes scenario/workload/SLO/methodology contracts",
                        source.run_id
                    ));
                }
            }
        }
    }
    for rule in &bundle.budget.budgets {
        let mut values = anchor
            .source_members
            .iter()
            .filter_map(|member| {
                member
                    .metrics
                    .iter()
                    .find(|metric| metric.budget_id == rule.id)
                    .map(|metric| metric.value)
            })
            .collect::<Vec<_>>();
        let expected = median(&mut values);
        let recorded = anchor
            .metrics
            .iter()
            .find(|metric| metric.budget_id == rule.id);
        if expected.zip(recorded).is_none_or(|(expected, recorded)| {
            recorded.unit != rule.unit || !approx_eq(recorded.value, expected)
        }) {
            problems.push(format!(
                "release anchor {} does not recompute from source receipts",
                rule.id
            ));
        }
    }
}

fn baseline_member_receipts_valid(bundle: &ContractBundle, member: &BaselineMember) -> bool {
    let expected_reports = bundle
        .budget
        .reports
        .iter()
        .map(|report| report.id.as_str())
        .collect::<BTreeSet<_>>();
    let observed_reports = member
        .reports
        .iter()
        .map(|report| report.report_id.as_str())
        .collect::<BTreeSet<_>>();
    let expected_metrics = bundle
        .budget
        .budgets
        .iter()
        .map(|rule| rule.id.as_str())
        .collect::<BTreeSet<_>>();
    let observed_metrics = member
        .metrics
        .iter()
        .map(|metric| metric.budget_id.as_str())
        .collect::<BTreeSet<_>>();
    let expected_binary_ids = bundle
        .profile
        .prebuild
        .target_set
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let common_build = member.reports.first().is_some_and(|first| {
        member.reports.iter().all(|report| {
            report.cargo_lock_sha256 == first.cargo_lock_sha256
                && report.prebuild_manifest_sha256 == first.prebuild_manifest_sha256
                && report.binary_sha256 == first.binary_sha256
                && report.binary_set_digest == first.binary_set_digest
                && report
                    .binary_sha256
                    .iter()
                    .map(|binary| binary.id.as_str())
                    .collect::<BTreeSet<_>>()
                    == expected_binary_ids
        })
    });
    expected_reports == observed_reports
        && observed_reports.len() == member.reports.len()
        && expected_metrics == observed_metrics
        && observed_metrics.len() == member.metrics.len()
        && common_build
        && member.reports.iter().all(|report| {
            valid_baseline_report(report)
                && report.receipt_sha256 == baseline_report_receipt(report)
                && bundle
                    .budget
                    .budgets
                    .iter()
                    .filter(|rule| rule.report == report.report_id)
                    .all(|rule| {
                        let report_metric = report
                            .metrics
                            .iter()
                            .find(|metric| metric.id == rule.metric);
                        let member_metric = member
                            .metrics
                            .iter()
                            .find(|metric| metric.budget_id == rule.id);
                        report_metric.zip(member_metric).is_some_and(
                            |(report_metric, member_metric)| {
                                report_metric.unit == rule.unit
                                    && member_metric.unit == rule.unit
                                    && report_metric.value.is_finite()
                                    && report_metric.value > 0.0
                                    && approx_eq(report_metric.value, member_metric.value)
                            },
                        )
                    })
        })
}

fn baseline_member_semantics_valid(bundle: &ContractBundle, member: &BaselineMember) -> bool {
    let derived_success = member.gate_exit_code == 0;
    let derived_quarantine = member.quarantine_reason.is_some();
    let runner_valid = member.runner_contract == bundle.profile.runner
        && member.runner_contract_digest == digest_json(&member.runner_contract)
        && member.runner_fingerprint == member.observed_runner.fingerprint
        && member.observed_runner.runner_class == member.runner_contract.required_runner_class
        && validate_runner_observation(&member.runner_contract, &member.observed_runner).is_ok();
    let reports_stable = !member.reports.is_empty()
        && member.reports.iter().all(|report| {
            finite_nonnegative_ratio(report.maximum_spread_ratio)
                && report.stable
                    == (report.maximum_spread_ratio
                        <= bundle.profile.noise.maximum_report_spread_ratio)
                && report.stable
        });
    member.successful == derived_success
        && member.quarantined == derived_quarantine
        && member.calibration_passed == runner_valid
        && member.spread_stable == reports_stable
        && member.git_status_porcelain_sha256 == CLEAN_GIT_STATUS_SHA256
        && baseline_member_receipts_valid(bundle, member)
}

fn anchor_is_empty(anchor: &ReleaseAnchor) -> bool {
    anchor.status == BootstrapStatus::Unbootstrapped
        && anchor.frozen_at.is_empty()
        && anchor.contract_commit.is_empty()
        && anchor.source_run_ids.is_empty()
        && anchor.source_members.is_empty()
        && anchor.metrics.is_empty()
}

fn validate_rolling_metrics(
    budget: &BudgetContract,
    manifest: &RollingBaselineManifest,
    problems: &mut Vec<String>,
) {
    let recorded = manifest
        .rolling_metrics
        .iter()
        .map(|metric| (metric.budget_id.as_str(), metric))
        .collect::<BTreeMap<_, _>>();
    for rule in &budget.budgets {
        let mut samples = manifest
            .members
            .iter()
            .filter_map(|member| {
                member
                    .metrics
                    .iter()
                    .find(|metric| metric.budget_id == rule.id)
                    .map(|metric| metric.value)
            })
            .collect::<Vec<_>>();
        let expected_median = median(&mut samples);
        let expected_mad = expected_median.map(|center| {
            let mut deviations = samples
                .iter()
                .map(|value| (value - center).abs())
                .collect::<Vec<_>>();
            median(&mut deviations).unwrap_or(f64::NAN)
        });
        match (
            recorded.get(rule.id.as_str()),
            expected_median,
            expected_mad,
        ) {
            (Some(metric), Some(median), Some(mad))
                if metric.unit == rule.unit
                    && approx_eq(metric.median, median)
                    && approx_eq(metric.mad, mad)
                    && rule
                        .rolling_tolerance_ratio
                        .is_some_and(|tolerance| approx_eq(metric.tolerance_ratio, tolerance)) => {}
            _ => problems.push(format!(
                "rolling median/MAD for {} does not recompute",
                rule.id
            )),
        }
    }
    if recorded.len() != budget.budgets.len() {
        problems.push("rolling baseline has missing/extra metric summaries".to_owned());
    }
}

pub fn load_candidate_reports(
    root: &Path,
    budget: &BudgetContract,
) -> Result<Vec<CandidateReport>, PerfBudgetError> {
    validate_macro_publication_receipt(root, budget)?;
    reject_extra_macro_reports(root, budget)?;
    let reports = budget
        .reports
        .iter()
        .map(|expected| {
            let path = root.join(&expected.path);
            let bytes = read_bounded(&path)?;
            let mut report = normalize_report(expected, budget.enforcement, &bytes)?;
            let expected_metrics = budget
                .budgets
                .iter()
                .filter(|rule| rule.report == expected.id)
                .map(|rule| rule.metric.as_str())
                .collect::<BTreeSet<_>>();
            report
                .metrics
                .retain(|metric, _| expected_metrics.contains(metric.as_str()));
            if report.metrics.len() != expected_metrics.len() {
                return Err(PerfBudgetError::new(format!(
                    "{} does not contain every reviewed budget metric",
                    expected.id
                )));
            }
            Ok(report)
        })
        .collect::<Result<Vec<_>, _>>()?;
    validate_macro_dependency_graph(root, budget)?;
    Ok(reports)
}

fn validate_macro_publication_receipt(
    root: &Path,
    budget: &BudgetContract,
) -> Result<(), PerfBudgetError> {
    let marker_path = root.join(MACRO_PUBLICATION_RECEIPT_RELATIVE);
    let marker_bytes = read_bounded(&marker_path)?;
    let receipt: MacroBatchPublicationReceipt =
        serde_json::from_slice(&marker_bytes).map_err(|error| {
            PerfBudgetError::new(format!(
                "invalid W7 macro publication receipt {}: {error}",
                marker_path.display()
            ))
        })?;
    if receipt.schema_version != 1
        || receipt.release != RELEASE
        || !receipt.receipt_is_valid()
        || !is_git_commit(&receipt.source_commit)
        || !is_sha256(&receipt.runner_fingerprint)
        || !is_sha256(&receipt.prebuild_manifest_sha256)
    {
        return Err(PerfBudgetError::new(
            "W7 macro publication receipt identity/seal is invalid",
        ));
    }

    let declared = budget
        .reports
        .iter()
        .filter(|report| report.format == ReportFormat::MacroReceiptV1)
        .map(|report| (report.report_id.clone(), report.path.clone()))
        .collect::<BTreeMap<_, _>>();
    let fixed = MACRO_REPORT_PATHS
        .iter()
        .map(|(report_id, path)| ((*report_id).to_owned(), (*path).to_owned()))
        .collect::<BTreeMap<_, _>>();
    if declared != fixed {
        return Err(PerfBudgetError::new(
            "budget macro set differs from the fixed W4-W6 publication contract",
        ));
    }
    let published = receipt
        .artifacts
        .iter()
        .map(|artifact| (artifact.report_id.clone(), artifact.canonical_path.clone()))
        .collect::<BTreeMap<_, _>>();
    if receipt.artifacts.len() != fixed.len() || published != fixed {
        return Err(PerfBudgetError::new(
            "W7 publication marker does not bind the exact complete W4-W6 set",
        ));
    }

    let mut allowed_recovery_names = BTreeSet::from(["macro-publication-receipt.json".to_owned()]);
    for artifact in &receipt.artifacts {
        let expected_sidecar = format!(
            "target/test-evidence/0.67/w7-raw/{}.raw.json",
            artifact.report_id
        );
        if artifact.raw_sidecar_path != expected_sidecar
            || !safe_target_report_path(&artifact.canonical_path)
            || !safe_target_report_path(&artifact.raw_sidecar_path)
            || !is_sha256(&artifact.raw_sha256)
            || !is_sha256(&artifact.source_report_sha256)
            || !is_sha256(&artifact.envelope_sha256)
        {
            return Err(PerfBudgetError::new(format!(
                "W7 publication entry {} has an unsafe path or invalid digest",
                artifact.report_id
            )));
        }
        allowed_recovery_names.insert(format!("{}.raw.json", artifact.report_id));
        let envelope_bytes = read_bounded(&root.join(&artifact.canonical_path))?;
        let raw_bytes = read_bounded(&root.join(&artifact.raw_sidecar_path))?;
        if sha256(&envelope_bytes) != artifact.envelope_sha256
            || sha256(&raw_bytes) != artifact.raw_sha256
        {
            return Err(PerfBudgetError::new(format!(
                "W7 publication bytes changed for {}",
                artifact.report_id
            )));
        }
        let envelope: Value = serde_json::from_slice(&envelope_bytes).map_err(|error| {
            PerfBudgetError::new(format!(
                "published {} envelope is invalid JSON: {error}",
                artifact.report_id
            ))
        })?;
        let budget_receipt = envelope.get("budget_receipt").ok_or_else(|| {
            PerfBudgetError::new(format!(
                "published {} has no budget receipt",
                artifact.report_id
            ))
        })?;
        if budget_receipt.get("report_id").and_then(Value::as_str)
            != Some(artifact.report_id.as_str())
            || budget_receipt
                .get("source_report_sha256")
                .and_then(Value::as_str)
                != Some(artifact.source_report_sha256.as_str())
            || budget_receipt.get("source_commit").and_then(Value::as_str)
                != Some(receipt.source_commit.as_str())
            || budget_receipt.get("runner_profile").and_then(Value::as_str)
                != Some(receipt.runner_profile.as_str())
            || budget_receipt
                .get("runner_fingerprint")
                .and_then(Value::as_str)
                != Some(receipt.runner_fingerprint.as_str())
            || budget_receipt
                .get("prebuild_manifest_sha256")
                .and_then(Value::as_str)
                != Some(receipt.prebuild_manifest_sha256.as_str())
        {
            return Err(PerfBudgetError::new(format!(
                "published {} does not match its batch marker identity",
                artifact.report_id
            )));
        }
    }

    let recovery_dir = marker_path
        .parent()
        .ok_or_else(|| PerfBudgetError::new("W7 marker has no recovery directory"))?;
    let mut observed_recovery_names = BTreeSet::new();
    for entry in fs::read_dir(recovery_dir).map_err(|error| {
        PerfBudgetError::new(format!("reading {}: {error}", recovery_dir.display()))
    })? {
        let entry = entry.map_err(|error| PerfBudgetError::new(error.to_string()))?;
        if !entry
            .file_type()
            .map_err(|error| PerfBudgetError::new(error.to_string()))?
            .is_file()
        {
            return Err(PerfBudgetError::new(
                "W7 recovery directory contains a non-file entry",
            ));
        }
        observed_recovery_names.insert(entry.file_name().to_string_lossy().into_owned());
    }
    if observed_recovery_names != allowed_recovery_names {
        return Err(PerfBudgetError::new(format!(
            "W7 recovery directory contains stale/missing raw or temp files: expected {allowed_recovery_names:?}, observed {observed_recovery_names:?}"
        )));
    }
    Ok(())
}

fn validate_macro_dependency_graph(
    root: &Path,
    budget: &BudgetContract,
) -> Result<(), PerfBudgetError> {
    let mut sources = BTreeMap::new();
    let mut values = BTreeMap::new();
    for expected in &budget.reports {
        let bytes = read_bounded(&root.join(&expected.path))?;
        let value: Value = serde_json::from_slice(&bytes).map_err(|error| {
            PerfBudgetError::new(format!("invalid {} dependency JSON: {error}", expected.id))
        })?;
        let source_sha256 = match expected.format {
            ReportFormat::PerfReportV1 => sha256(&bytes),
            ReportFormat::MacroReceiptV1 => macro_raw_source_sha256(&expected.id, &value)?,
        };
        sources.insert(expected.id.as_str(), source_sha256);
        values.insert(expected.id.as_str(), value);
    }
    for (consumer, predecessor, pointer) in [
        (
            "brownout-control-plane",
            "control-plane-3",
            "/report/reference_provenance/predecessor_artifact_sha256",
        ),
        (
            "brownout-resp-endpoint",
            "node-resp-open-loop",
            "/report/reference_provenance/selected_predecessor_artifact_sha256",
        ),
        (
            "brownout-grid-model",
            "grid-model",
            "/report/reference_provenance/w4b_artifact_sha256",
        ),
        (
            "overload-local",
            "local",
            "/report/predecessor/reference_receipt/predecessor_report_sha256",
        ),
        (
            "overload-client-surface",
            "client-surface",
            "/report/predecessor/reference_receipt/predecessor_report_sha256",
        ),
        (
            "overload-node-resp",
            "node-resp-open-loop",
            "/report/predecessor/reference_receipt/predecessor_report_sha256",
        ),
    ] {
        let observed = values
            .get(consumer)
            .and_then(|value| value.pointer(pointer))
            .and_then(Value::as_str);
        let expected = sources.get(predecessor).map(String::as_str);
        if observed != expected {
            return Err(PerfBudgetError::new(format!(
                "{consumer} does not bind the exact raw {predecessor} source report"
            )));
        }
    }
    let w4a = values
        .get("control-plane-3")
        .ok_or_else(|| PerfBudgetError::new("control-plane-3 source is absent"))?;
    let w5a = values
        .get("brownout-control-plane")
        .ok_or_else(|| PerfBudgetError::new("W5A source is absent"))?;
    if w5a
        .pointer("/report/reference_provenance/predecessor_receipt_sha256")
        .and_then(Value::as_str)
        != w4a
            .pointer("/report/capability_receipt_sha256")
            .and_then(Value::as_str)
    {
        return Err(PerfBudgetError::new(
            "W5A predecessor receipt differs from the exact W4A capability receipt",
        ));
    }
    let w4b = values
        .get("grid-model")
        .ok_or_else(|| PerfBudgetError::new("grid-model source is absent"))?;
    let w5c = values
        .get("brownout-grid-model")
        .ok_or_else(|| PerfBudgetError::new("W5C source is absent"))?;
    if w5c
        .pointer("/report/reference_provenance/w4b_reference_receipt_sha256")
        .and_then(Value::as_str)
        != w4b
            .pointer("/report/reference_capability/receipt_sha256")
            .and_then(Value::as_str)
    {
        return Err(PerfBudgetError::new(
            "W5C predecessor receipt differs from the exact W4B reference receipt",
        ));
    }
    validate_w5b_dependency_graph(root, &values)?;
    Ok(())
}

fn macro_raw_source_sha256(report_id: &str, envelope: &Value) -> Result<String, PerfBudgetError> {
    let source = envelope
        .get("report")
        .ok_or_else(|| PerfBudgetError::new(format!("{report_id} has no typed source report")))?;
    let bytes = match report_id {
        "control-plane-3" | "control-plane-5" | "control-plane-7" => {
            let typed: ControlPlaneReport = deserialize_typed_report(report_id, source)?;
            serde_json::to_vec_pretty(&typed)
        }
        "grid-model" => {
            let typed: GridModelReport = deserialize_typed_report(report_id, source)?;
            serde_json::to_vec_pretty(&typed)
        }
        // No W5/W6 report is a predecessor of another macro-budget report.
        // Their outer receipt still seals the normalized source-report digest;
        // a raw pretty-byte identity is needed only for W4 -> W5 edges.
        _ => serde_json::to_vec(source),
    }
    .map_err(|error| {
        PerfBudgetError::new(format!(
            "serializing {report_id} dependency source: {error}"
        ))
    })?;
    Ok(sha256(&bytes))
}

fn validate_w5b_dependency_graph(
    root: &Path,
    values: &BTreeMap<&str, Value>,
) -> Result<(), PerfBudgetError> {
    let w3 = values
        .get("node-resp-open-loop")
        .ok_or_else(|| PerfBudgetError::new("W3 source is absent"))?;
    let w5b = values
        .get("brownout-resp-endpoint")
        .ok_or_else(|| PerfBudgetError::new("W5B source is absent"))?;
    let provenance = w5b
        .pointer("/report/reference_provenance")
        .ok_or_else(|| PerfBudgetError::new("W5B provenance is absent"))?;
    if provenance
        .get("archived_process_receipt_sha256")
        .and_then(Value::as_str)
        != w3
            .get("resp_endpoint_capability")
            .map(digest_json)
            .as_deref()
        || provenance
            .get("predecessor_scenario_digest")
            .and_then(Value::as_str)
            != w3.get("scenario_digest").and_then(Value::as_str)
        || provenance
            .get("predecessor_workload_digest")
            .and_then(Value::as_str)
            != w3.get("workload_digest").and_then(Value::as_str)
    {
        return Err(PerfBudgetError::new(
            "W5B predecessor source/scenario/workload receipt differs from exact W3",
        ));
    }
    let lifecycle_path = root.join("target/test-evidence/0.67/node-resp-daemon-lifecycle.json");
    let lifecycle_sha256 = sha256(&read_bounded(&lifecycle_path)?);
    if provenance
        .get("selected_predecessor_lifecycle_sha256")
        .and_then(Value::as_str)
        != Some(lifecycle_sha256.as_str())
    {
        return Err(PerfBudgetError::new(
            "W5B predecessor lifecycle differs from the exact archived W3 lifecycle",
        ));
    }
    Ok(())
}

fn reject_extra_macro_reports(root: &Path, budget: &BudgetContract) -> Result<(), PerfBudgetError> {
    let expected = budget
        .reports
        .iter()
        .map(|report| {
            Path::new(&report.path)
                .file_name()
                .map(|name| name.to_owned())
        })
        .collect::<Option<BTreeSet<_>>>()
        .ok_or_else(|| PerfBudgetError::new("expected report path has no file name"))?;
    let directory = root.join("target/test-evidence/0.67");
    let entries = fs::read_dir(&directory).map_err(|error| {
        PerfBudgetError::new(format!("reading {}: {error}", directory.display()))
    })?;
    let mut extras = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| PerfBudgetError::new(error.to_string()))?;
        let name = entry.file_name();
        let text = name.to_string_lossy();
        if is_macro_report_name(&text) && !expected.contains(&name) {
            extras.push(text.into_owned());
        }
    }
    if extras.is_empty() {
        Ok(())
    } else {
        extras.sort();
        Err(PerfBudgetError::new(format!(
            "extra W1-W6 macro reports are forbidden: {extras:?}"
        )))
    }
}

fn is_macro_report_name(name: &str) -> bool {
    name == "local.json"
        || name == "client-surface.json"
        || name.starts_with("node-resp-open-loop")
        || name.starts_with("control-plane-")
        || name == "grid-model.json"
        || name.starts_with("brownout-")
        || name.starts_with("overload-")
}

pub fn normalize_report(
    expected: &ExpectedReport,
    enforcement: Enforcement,
    bytes: &[u8],
) -> Result<CandidateReport, PerfBudgetError> {
    match expected.format {
        ReportFormat::PerfReportV1 => normalize_perf_report(expected, enforcement, bytes),
        ReportFormat::MacroReceiptV1 => normalize_macro_receipt(expected, enforcement, bytes),
    }
}

fn normalize_macro_receipt(
    expected: &ExpectedReport,
    enforcement: Enforcement,
    bytes: &[u8],
) -> Result<CandidateReport, PerfBudgetError> {
    let root: Value = serde_json::from_slice(bytes)
        .map_err(|error| PerfBudgetError::new(format!("invalid {} JSON: {error}", expected.id)))?;
    require_exact_object_keys(
        &root,
        &["report", "budget_receipt"],
        &format!("{} macro envelope", expected.id),
    )?;
    let source_report = root.get("report").ok_or_else(|| {
        PerfBudgetError::new(format!(
            "{} macro artifact must wrap its typed report and budget receipt",
            expected.id
        ))
    })?;
    let receipt_value = root
        .get("budget_receipt")
        .ok_or_else(|| PerfBudgetError::new(format!("{} has no budget receipt", expected.id)))?;
    let receipt: MacroReportReceipt =
        serde_json::from_value(receipt_value.clone()).map_err(|error| {
            PerfBudgetError::new(format!("invalid {} macro receipt: {error}", expected.id))
        })?;
    if !receipt.receipt_is_valid() {
        return Err(PerfBudgetError::new(format!(
            "{} macro receipt canonical seal does not recompute",
            expected.id
        )));
    }
    if receipt.schema_version != 1
        || receipt.release != RELEASE
        || receipt.report_id != expected.report_id
        || receipt.source_report_sha256 != digest_json(source_report)
        || receipt.claim_scope != expected.claim_scope
        || receipt.run_mode != enforcement_run_mode(enforcement)
    {
        return Err(PerfBudgetError::new(format!(
            "{} macro receipt identity is mismatched",
            expected.id
        )));
    }
    validate_runner_observation(&receipt.runner_contract, &receipt.observed_runner)?;
    if receipt.runner_profile != receipt.runner_contract.name
        || receipt.runner_contract_digest != digest_json(&receipt.runner_contract)
        || receipt.runner_fingerprint != receipt.observed_runner.fingerprint
    {
        return Err(PerfBudgetError::new(format!(
            "{} macro receipt runner contract does not recompute",
            expected.id
        )));
    }
    validate_w7_binary_set(&receipt.binary_sha256, &receipt.binary_set_digest)?;
    validate_receipt_fields(
        &receipt.runner_profile,
        &receipt.runner_contract_digest,
        &receipt.runner_fingerprint,
        &receipt.source_commit,
        &receipt.cargo_lock_sha256,
        &receipt.prebuild_contract_digest,
        &receipt.prebuild_manifest_sha256,
        &receipt.binary_set_digest,
        &receipt.scenario_digest,
        &receipt.workload_digest,
        &receipt.slo_digest,
        &receipt.methodology_digest,
        receipt.maximum_spread_ratio,
    )?;
    let toolchain_identity = canonical_toolchain_identity(&receipt.toolchain_identity)?;
    let (derived_metrics, derived_spread) =
        macro_report_metrics(&expected.id, source_report, &receipt)?;
    let metrics = metric_map(receipt.metrics)?;
    let spread_limit = match enforcement {
        Enforcement::Ship => 0.05,
        Enforcement::NonEnforcingTripwire => 0.30,
    };
    let derived_stable = derived_spread <= spread_limit;
    if metrics != derived_metrics
        || !approx_eq(receipt.maximum_spread_ratio, derived_spread)
        || receipt.stable != derived_stable
    {
        return Err(PerfBudgetError::new(format!(
            "{} macro budget metrics/spread/stability do not recompute from its typed report",
            expected.id
        )));
    }
    Ok(CandidateReport {
        id: expected.id.clone(),
        path: expected.path.clone(),
        report_id: receipt.report_id,
        report_sha256: sha256(bytes),
        claim_scope: receipt.claim_scope,
        run_mode: receipt.run_mode,
        runner_profile: receipt.runner_profile,
        runner_contract_digest: receipt.runner_contract_digest,
        runner_class: receipt.observed_runner.runner_class,
        runner_fingerprint: receipt.runner_fingerprint,
        source_commit: receipt.source_commit,
        cargo_lock_sha256: receipt.cargo_lock_sha256,
        toolchain_identity,
        prebuild_contract_digest: receipt.prebuild_contract_digest,
        prebuild_manifest_sha256: receipt.prebuild_manifest_sha256,
        binary_sha256: receipt.binary_sha256,
        binary_set_digest: receipt.binary_set_digest,
        scenario_digest: receipt.scenario_digest,
        workload_digest: receipt.workload_digest,
        slo_digest: receipt.slo_digest,
        methodology_digest: receipt.methodology_digest,
        stable: receipt.stable,
        maximum_spread_ratio: receipt.maximum_spread_ratio,
        metrics,
    })
}

fn macro_report_metrics(
    report_id: &str,
    report: &Value,
    receipt: &MacroReportReceipt,
) -> Result<(BTreeMap<String, ReportMetric>, f64), PerfBudgetError> {
    let mut metrics = BTreeMap::new();
    let mut add = |id: &str, value: f64, unit: &str| {
        insert_metric(
            &mut metrics,
            ReportMetric {
                id: id.to_owned(),
                value,
                unit: unit.to_owned(),
            },
        )
    };
    match report_id {
        "control-plane-3" | "control-plane-5" | "control-plane-7" => {
            let typed: ControlPlaneReport = deserialize_typed_report(report_id, report)?;
            require_exact_object_keys(
                report,
                &[
                    "schema_version",
                    "scenario_id",
                    "evidence_class",
                    "execution_mode",
                    "capability_receipt_sha256",
                    "capability",
                    "node_count",
                    "capacity_scope",
                    "aggregate_cluster_capacity",
                    "product_data_plane",
                    "live_reshard_measured",
                    "steady_reads",
                    "membership_events",
                    "lifecycle",
                    "deferred_claims",
                ],
                "control-plane report",
            )?;
            let expected_nodes = report_id
                .rsplit('-')
                .next()
                .and_then(|value| value.parse::<u64>().ok());
            validate_w4_capability(report, expected_nodes, receipt)?;
            validate_w4_archived_lifecycle(&typed, receipt)?;
            if report.get("schema_version").and_then(Value::as_u64) != Some(1)
                || report.get("node_count").and_then(Value::as_u64) != expected_nodes
                || report.get("evidence_class").and_then(Value::as_str)
                    != Some("w4a-real-daemon-control-plane")
                || report
                    .get("aggregate_cluster_capacity")
                    .and_then(Value::as_bool)
                    != Some(false)
                || report.get("product_data_plane").and_then(Value::as_bool) != Some(false)
                || report.get("live_reshard_measured").and_then(Value::as_bool) != Some(false)
                || !report
                    .get("capability_receipt_sha256")
                    .and_then(Value::as_str)
                    .is_some_and(is_sha256)
            {
                return Err(PerfBudgetError::new(
                    "control-plane report is not exact non-aggregate real-daemon evidence",
                ));
            }
            let events = report
                .get("membership_events")
                .and_then(Value::as_array)
                .filter(|events| events.len() == 2)
                .ok_or_else(|| PerfBudgetError::new("control-plane report must have add+drain"))?;
            let actions = events
                .iter()
                .filter_map(|event| event.get("action").and_then(Value::as_str))
                .collect::<BTreeSet<_>>();
            if actions != BTreeSet::from(["add", "drain"]) {
                return Err(PerfBudgetError::new(
                    "control-plane report does not contain exact add+drain actions",
                ));
            }
            let maximum = events
                .iter()
                .map(|event| {
                    let commit = event.get("commit_latency_nanos").and_then(Value::as_u64)?;
                    let convergence = event
                        .get("convergence_latency_nanos")
                        .and_then(Value::as_u64)?;
                    (commit > 0 && convergence >= commit)
                        .then_some(convergence as f64 / 1_000_000.0)
                })
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| PerfBudgetError::new("control-plane event latency is malformed"))?
                .into_iter()
                .max_by(f64::total_cmp)
                .ok_or_else(|| PerfBudgetError::new("control-plane event latency is absent"))?;
            add(
                "membership_add_drain_commit_and_convergence_latency.max_milliseconds",
                maximum,
                "milliseconds",
            )?;
            let reads = report
                .get("steady_reads")
                .and_then(Value::as_array)
                .filter(|reads| reads.len() == 2)
                .ok_or_else(|| PerfBudgetError::new("control-plane report has no role knees"))?;
            let mut roles = BTreeSet::new();
            let mut spread = 0.0_f64;
            for read in reads {
                let role = read
                    .get("target_node_role")
                    .and_then(Value::as_str)
                    .ok_or_else(|| PerfBudgetError::new("control-plane role knee is untyped"))?;
                roles.insert(role);
                if read.get("role_changed").and_then(Value::as_bool) != Some(false) {
                    return Err(PerfBudgetError::new(
                        "control-plane role changed inside a capacity window",
                    ));
                }
                let evidence = serde_json::json!({
                    "claim": "capacity_knee",
                    "criteria": read.get("criteria"),
                    "knee": read.get("knee"),
                });
                spread = spread.max(
                    validate_knee_evidence("w4a-role-knee", &evidence, Some(5))?
                        .maximum_spread_ratio,
                );
            }
            if roles != BTreeSet::from(["leader", "follower"]) {
                return Err(PerfBudgetError::new(
                    "control-plane report must keep leader/follower knees separate",
                ));
            }
            Ok((metrics, spread))
        }
        "grid-model" => {
            let _: GridModelReport = deserialize_typed_report(report_id, report)?;
            require_exact_object_keys(
                report,
                &[
                    "schema_version",
                    "scenario_id",
                    "scenario_sha256",
                    "evidence_class",
                    "execution_mode",
                    "daemon_processes",
                    "product_data_plane",
                    "end_to_end_cluster_capacity",
                    "byte_metric_name",
                    "run_mode",
                    "reference_capability",
                    "ack_requirement_cost",
                    "session_decision_cost",
                    "replication_primitive_curve",
                    "invalidation_fanout_cost",
                ],
                "grid-model report",
            )?;
            if report.get("schema_version").and_then(Value::as_u64) != Some(1)
                || report.get("run_mode").and_then(Value::as_str) != Some("reference")
                || report
                    .get("reference_capability")
                    .is_none_or(Value::is_null)
                || report.get("daemon_processes").and_then(Value::as_bool) != Some(false)
                || report.get("product_data_plane").and_then(Value::as_bool) != Some(false)
                || report
                    .get("end_to_end_cluster_capacity")
                    .and_then(Value::as_bool)
                    != Some(false)
            {
                return Err(PerfBudgetError::new(
                    "grid-model budget requires reference in-process non-capacity evidence",
                ));
            }
            let points = report
                .get("ack_requirement_cost")
                .and_then(Value::as_array)
                .filter(|points| !points.is_empty())
                .ok_or_else(|| PerfBudgetError::new("grid-model report has no ack-cost points"))?;
            let summaries = points
                .iter()
                .map(|point| {
                    let iterations = point.get("iterations").and_then(Value::as_u64)?;
                    let timing = point.get("timing")?;
                    let summary = validate_primitive_timing(timing, iterations).ok()?;
                    (iterations > 0).then_some((summary.0 as f64 / iterations as f64, summary.1))
                })
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| PerfBudgetError::new("grid-model ack cost is malformed"))?;
            let maximum = summaries
                .iter()
                .map(|(value, _)| *value)
                .max_by(f64::total_cmp)
                .ok_or_else(|| PerfBudgetError::new("grid-model ack cost is absent"))?;
            let mut spread = summaries
                .iter()
                .map(|(_, spread)| *spread)
                .fold(0.0_f64, f64::max);
            for collection in [
                "session_decision_cost",
                "replication_primitive_curve",
                "invalidation_fanout_cost",
            ] {
                let rows = report
                    .get(collection)
                    .and_then(Value::as_array)
                    .filter(|rows| !rows.is_empty())
                    .ok_or_else(|| {
                        PerfBudgetError::new(format!("grid-model {collection} is empty"))
                    })?;
                for row in rows {
                    let iterations =
                        row.get("iterations")
                            .and_then(Value::as_u64)
                            .ok_or_else(|| {
                                PerfBudgetError::new(format!(
                                    "grid-model {collection} iterations are absent"
                                ))
                            })?;
                    spread = spread.max(
                        validate_primitive_timing(
                            row.get("timing").ok_or_else(|| {
                                PerfBudgetError::new("grid-model timing is absent")
                            })?,
                            iterations,
                        )?
                        .1,
                    );
                }
            }
            add(
                "consistency_ack_requirement_cost_by_level_and_replica_shape.max_nanoseconds_per_operation",
                maximum,
                "nanoseconds_per_operation",
            )?;
            Ok((metrics, spread))
        }
        "brownout-control-plane" => {
            require_exact_object_keys(
                report,
                &[
                    "schema_version",
                    "scenario_id",
                    "scenario_sha256",
                    "evidence_class",
                    "run_mode",
                    "predecessor",
                    "predecessor_node_count",
                    "reference_provenance",
                    "final_cleanup",
                    "events",
                    "generic_client_write_invariant",
                    "distributed_value_invariant",
                    "live_reshard_measured",
                    "aggregate_goodput",
                ],
                "control-plane brownout report",
            )?;
            if report.get("schema_version").and_then(Value::as_u64) != Some(1)
                || report.get("run_mode").and_then(Value::as_str) != Some("reference")
                || report
                    .get("reference_provenance")
                    .is_none_or(Value::is_null)
                || [
                    "generic_client_write_invariant",
                    "distributed_value_invariant",
                    "live_reshard_measured",
                    "aggregate_goodput",
                ]
                .iter()
                .any(|field| report.get(*field).and_then(Value::as_bool) != Some(false))
            {
                return Err(PerfBudgetError::new(
                    "control-plane brownout is not exact reference operational evidence",
                ));
            }
            validate_w5_control_provenance(report, receipt)?;
            let events = report
                .get("events")
                .and_then(Value::as_array)
                .filter(|events| events.len() == 4)
                .ok_or_else(|| PerfBudgetError::new("control-plane brownout needs four actions"))?;
            let actions = events
                .iter()
                .filter_map(|event| event.get("action").and_then(Value::as_str))
                .collect::<BTreeSet<_>>();
            if actions
                != BTreeSet::from([
                    "leader_failover",
                    "member_add",
                    "member_drain",
                    "node_kill_rejoin",
                ])
            {
                return Err(PerfBudgetError::new(
                    "control-plane brownout action set is incomplete",
                ));
            }
            let recovery = max_u64(events, "transition_recovery_millis")? as f64;
            let depth = events
                .iter()
                .map(|event| {
                    let raw = event.get("raw")?;
                    let timeline = serde_json::from_value::<W5EventTimelineReceipt>(
                        raw.get("timeline")?.clone(),
                    )
                    .ok()?;
                    let declared = event.get("transition_recovery_millis")?.as_u64()?;
                    let disruption = raw.get("disruption_window")?;
                    (timeline.commit_latency_nanos > 0
                        && timeline.recovery_latency_nanos >= timeline.commit_latency_nanos
                        && timeline.receipt_sha256 == timeline.recomputed_receipt()
                        && declared
                            == timeline.recovery_latency_nanos.saturating_add(999_999) / 1_000_000)
                        .then(|| availability_dip_ppm(disruption).ok())
                        .flatten()
                })
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| {
                    PerfBudgetError::new("control-plane brownout raw windows are malformed")
                })?
                .into_iter()
                .max()
                .ok_or_else(|| PerfBudgetError::new("control-plane brownout depth is absent"))?
                as f64;
            add(
                "control_plane_brownout.recovery_milliseconds",
                recovery,
                "milliseconds",
            )?;
            add(
                "control_plane_brownout.maximum_availability_dip_ppm",
                depth,
                "parts_per_million",
            )?;
            Ok((metrics, 0.0))
        }
        "brownout-resp-endpoint" => {
            require_exact_object_keys(
                report,
                &[
                    "schema_version",
                    "scenario_id",
                    "scenario_sha256",
                    "evidence_class",
                    "run_mode",
                    "predecessor",
                    "reference_provenance",
                    "selected_endpoint_recovery_millis",
                    "event",
                    "node_local_state",
                    "automatic_failover",
                    "neighbor_visibility_claim",
                    "value_survival_claim",
                    "cross_node_failover_claim",
                    "aggregate_goodput",
                ],
                "RESP brownout report",
            )?;
            let event = report
                .get("event")
                .ok_or_else(|| PerfBudgetError::new("RESP brownout has no event"))?;
            let timeline: W5EventTimelineReceipt = serde_json::from_value(
                event
                    .get("timeline")
                    .cloned()
                    .ok_or_else(|| PerfBudgetError::new("RESP brownout has no raw timeline"))?,
            )
            .map_err(|error| {
                PerfBudgetError::new(format!("RESP timeline is not typed: {error}"))
            })?;
            let recovery = timeline.recovery_latency_nanos.saturating_add(999_999) / 1_000_000;
            if report.get("schema_version").and_then(Value::as_u64) != Some(1)
                || report.get("run_mode").and_then(Value::as_str) != Some("reference")
                || report
                    .get("reference_provenance")
                    .is_none_or(Value::is_null)
                || report
                    .get("selected_endpoint_recovery_millis")
                    .and_then(Value::as_u64)
                    != Some(recovery)
                || timeline.commit_latency_nanos == 0
                || timeline.recovery_latency_nanos < timeline.commit_latency_nanos
                || timeline.receipt_sha256 != timeline.recomputed_receipt()
                || report.get("node_local_state").and_then(Value::as_bool) != Some(true)
                || [
                    "automatic_failover",
                    "neighbor_visibility_claim",
                    "value_survival_claim",
                    "cross_node_failover_claim",
                    "aggregate_goodput",
                ]
                .iter()
                .any(|field| report.get(*field).and_then(Value::as_bool) != Some(false))
            {
                return Err(PerfBudgetError::new(
                    "RESP brownout is not exact node-local reference lifecycle evidence",
                ));
            }
            validate_w5_resp_provenance(report)?;
            add(
                "resp_endpoint_brownout.recovery_milliseconds",
                recovery as f64,
                "milliseconds",
            )?;
            add(
                "resp_endpoint_brownout.availability_dip_ppm",
                availability_dip_ppm(
                    event
                        .get("disruption_window")
                        .ok_or_else(|| PerfBudgetError::new("RESP disruption window is absent"))?,
                )? as f64,
                "parts_per_million",
            )?;
            Ok((metrics, 0.0))
        }
        "brownout-grid-model" => {
            require_exact_object_keys(
                report,
                &[
                    "schema_version",
                    "scenario_id",
                    "scenario_sha256",
                    "evidence_class",
                    "run_mode",
                    "predecessor",
                    "reference_provenance",
                    "faults",
                    "daemon_brownout_evidence",
                    "product_data_plane",
                    "live_rebalance_measured",
                    "live_reshard_measured",
                    "aggregate_goodput",
                ],
                "grid-model brownout report",
            )?;
            if report.get("schema_version").and_then(Value::as_u64) != Some(1)
                || report.get("run_mode").and_then(Value::as_str) != Some("reference")
                || report
                    .get("reference_provenance")
                    .is_none_or(Value::is_null)
                || [
                    "daemon_brownout_evidence",
                    "product_data_plane",
                    "live_rebalance_measured",
                    "live_reshard_measured",
                    "aggregate_goodput",
                ]
                .iter()
                .any(|field| report.get(*field).and_then(Value::as_bool) != Some(false))
            {
                return Err(PerfBudgetError::new(
                    "grid-model brownout crossed its reference model boundary",
                ));
            }
            validate_w5_grid_provenance(report, receipt)?;
            let faults = report
                .get("faults")
                .and_then(Value::as_array)
                .filter(|faults| faults.len() == 2)
                .ok_or_else(|| PerfBudgetError::new("grid-model brownout needs two faults"))?;
            let mut recovery = Vec::new();
            let mut increase = Vec::new();
            let mut spread = 0.0_f64;
            let mut kinds = BTreeSet::new();
            for fault in faults {
                kinds.insert(
                    fault
                        .get("fault")
                        .and_then(Value::as_str)
                        .ok_or_else(|| PerfBudgetError::new("grid-model fault is untyped"))?,
                );
                let (baseline, fault_cost, recovery_cost, fault_spread) =
                    validate_model_fault_timing(fault)?;
                recovery.push(recovery_cost);
                increase.push(fault_cost.saturating_sub(baseline));
                spread = spread.max(fault_spread);
            }
            if kinds != BTreeSet::from(["slow_replica", "unavailable_replica"]) {
                return Err(PerfBudgetError::new("grid-model fault set is incomplete"));
            }
            add(
                "grid_model_fault.maximum_recovery_cost_nanos",
                recovery.into_iter().max().unwrap_or(0) as f64,
                "nanoseconds",
            )?;
            add(
                "grid_model_fault.maximum_decision_cost_increase_nanos",
                increase.into_iter().max().unwrap_or(0) as f64,
                "nanoseconds",
            )?;
            Ok((metrics, spread))
        }
        "overload-local" | "overload-client-surface" | "overload-node-resp" => {
            let (goodput, spread) = validate_overload_budget_report(report_id, report, receipt)?;
            add(
                "overload_goodput_curve_1_2x_1_5x_2x_knee_per_eligible_surface.minimum_goodput_per_second",
                goodput,
                "operations_per_second",
            )?;
            Ok((metrics, spread))
        }
        _ => Err(PerfBudgetError::new(format!(
            "unsupported macro report identity {report_id}"
        ))),
    }
}

fn deserialize_typed_report<T>(report_id: &str, report: &Value) -> Result<T, PerfBudgetError>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(report.clone()).map_err(|error| {
        PerfBudgetError::new(format!(
            "{report_id} source report does not match its deny-unknown typed contract: {error}"
        ))
    })
}

fn max_u64(values: &[Value], field: &str) -> Result<u64, PerfBudgetError> {
    values
        .iter()
        .map(|value| value.get(field).and_then(Value::as_u64))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| PerfBudgetError::new(format!("macro metric {field} is malformed")))?
        .into_iter()
        .max()
        .ok_or_else(|| PerfBudgetError::new(format!("macro metric {field} is absent")))
}

fn validate_w4_capability(
    report: &Value,
    expected_nodes: Option<u64>,
    receipt: &MacroReportReceipt,
) -> Result<(), PerfBudgetError> {
    let attestation: W4CapabilityAttestation = serde_json::from_value(
        report
            .get("capability")
            .cloned()
            .ok_or_else(|| PerfBudgetError::new("W4A embedded capability is absent"))?,
    )
    .map_err(|error| PerfBudgetError::new(format!("W4A capability is not typed: {error}")))?;
    let payload = attestation
        .payload
        .as_ref()
        .ok_or_else(|| PerfBudgetError::new("W4A embedded capability has no observed payload"))?;
    let report_receipt = report
        .get("capability_receipt_sha256")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if attestation.receipt_sha256 != report_receipt
        || attestation.receipt_sha256 != digest_json(payload)
        || payload.receipt_kind != "hydracache-real-daemon-cluster-v1"
        || payload.receipt_source != W4DaemonReceiptSource::ObservedProcessHarness
        || payload.execution_mode != "real-daemon-admin-http"
        || payload.profile != receipt.runner_profile
        || payload.source_commit != receipt.source_commit
        || payload.runner_fingerprint_sha256 != sha256(receipt.runner_fingerprint.as_bytes())
        || payload.prebuild_manifest_sha256 != receipt.prebuild_manifest_sha256
        || payload.prebuild_contract_sha256 != receipt.prebuild_contract_digest
        || payload.provisioner != "daemon-cluster-process-harness"
        || !payload.direct_prebuilt_exec
        || Some(u64::from(payload.node_count)) != expected_nodes
        || payload.nodes.len() != usize::from(payload.node_count)
    {
        return Err(PerfBudgetError::new(
            "W4A embedded capability receipt does not bind the outer candidate receipt",
        ));
    }
    rehash_exact_absolute_file(
        &payload.prebuild_manifest_canonical_path,
        &payload.prebuild_manifest_sha256,
        "W4A prebuild manifest",
    )?;
    rehash_exact_absolute_file(
        &payload.server_binary.canonical_path,
        &payload.server_binary.sha256,
        "W4A prebuilt server",
    )?;
    let mut node_ids = BTreeSet::new();
    let mut pids = BTreeSet::new();
    let mut config_paths = BTreeSet::new();
    for node in &payload.nodes {
        if node.node_id.trim().is_empty()
            || node.pid == 0
            || !node_ids.insert(node.node_id.as_str())
            || !pids.insert(node.pid)
            || !node.direct_prebuilt_exec
            || node.observed_executable_path != payload.server_binary.canonical_path
            || node.observed_executable_sha256 != payload.server_binary.sha256
            || node.config.launch_config.receipt_kind != "hydracache-daemon-launch-config-v1"
            || node.config.launch_config.node_id != node.node_id
            || !config_paths.insert(node.config.canonical_path.clone())
        {
            return Err(PerfBudgetError::new(
                "W4A embedded node receipt is duplicate, self-asserted, or binary/config mismatched",
            ));
        }
        rehash_exact_absolute_file(
            &node.config.canonical_path,
            &node.config.sha256,
            "W4A daemon config",
        )?;
    }
    Ok(())
}

fn validate_w4_archived_lifecycle(
    report: &ControlPlaneReport,
    receipt: &MacroReportReceipt,
) -> Result<(), PerfBudgetError> {
    let capability = report
        .capability
        .payload
        .as_ref()
        .ok_or_else(|| PerfBudgetError::new("W4A lifecycle has no capability payload"))?;
    let lifecycle = &report.lifecycle;
    if lifecycle.receipt_sha256 != digest_json(&lifecycle.payload)
        || lifecycle.payload.receipt_kind != "hydracache-daemon-cluster-lifecycle-v1"
        || lifecycle.payload.receipt_source != capability.receipt_source
        || lifecycle.payload.capability_receipt_sha256 != report.capability_receipt_sha256
        || lifecycle.payload.nodes.len() != capability.nodes.len()
        || lifecycle
            .payload
            .nodes
            .windows(2)
            .any(|nodes| nodes[0].node_id >= nodes[1].node_id)
    {
        return Err(PerfBudgetError::new(
            "W4A lifecycle receipt does not close the exact observed capability",
        ));
    }
    let server_sha256 = receipt_binary_sha256(receipt, "hydracache-server")?;
    let capability_nodes = capability
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let mut pids = BTreeSet::new();
    for node in &lifecycle.payload.nodes {
        let capability_node = capability_nodes.get(node.node_id.as_str()).ok_or_else(|| {
            PerfBudgetError::new("W4A lifecycle contains a node absent from its capability")
        })?;
        if node.pid == 0
            || !pids.insert(node.pid)
            || node.pid != capability_node.pid
            || !node.kill_requested
            || !node.wait_completed
            || !node.process_no_longer_running
            || node.exit_status.trim().is_empty()
            || node.server_binary_path_after != capability.server_binary.canonical_path
            || node.server_binary_sha256_after != capability.server_binary.sha256
            || node.server_binary_sha256_after != server_sha256
            || node.node_config_path_after != capability_node.config.canonical_path
            || node.node_config_sha256_after != capability_node.config.sha256
        {
            return Err(PerfBudgetError::new(
                "W4A lifecycle node is not the killed-and-waited capability process",
            ));
        }
        validate_archived_log(
            &node.stdout_log.canonical_path,
            node.stdout_log.bytes,
            &node.stdout_log.sha256,
            "W4A stdout",
        )?;
        validate_archived_log(
            &node.stderr_log.canonical_path,
            node.stderr_log.bytes,
            &node.stderr_log.sha256,
            "W4A stderr",
        )?;
        rehash_exact_absolute_file(
            &node.server_binary_path_after,
            &node.server_binary_sha256_after,
            "W4A lifecycle server binary",
        )?;
        rehash_exact_absolute_file(
            &node.node_config_path_after,
            &node.node_config_sha256_after,
            "W4A lifecycle node config",
        )?;
    }
    Ok(())
}

fn receipt_binary_sha256<'a>(
    receipt: &'a MacroReportReceipt,
    id: &str,
) -> Result<&'a str, PerfBudgetError> {
    receipt
        .binary_sha256
        .iter()
        .find(|binary| binary.id == id)
        .map(|binary| binary.sha256.as_str())
        .ok_or_else(|| PerfBudgetError::new(format!("macro receipt has no {id} binary")))
}

fn validate_archived_log(
    path: &Path,
    expected_bytes: u64,
    expected_sha256: &str,
    label: &str,
) -> Result<(), PerfBudgetError> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| PerfBudgetError::new(format!("canonicalizing {label}: {error}")))?;
    let metadata = fs::metadata(&canonical)
        .map_err(|error| PerfBudgetError::new(format!("reading {label} metadata: {error}")))?;
    if canonical != path
        || !metadata.is_file()
        || metadata.len() != expected_bytes
        || metadata.len() > 64 * 1024 * 1024
        || !is_sha256(expected_sha256)
        || sha256_file(&canonical)? != expected_sha256
    {
        return Err(PerfBudgetError::new(format!(
            "{label} path/length/SHA receipt differs from its archived file"
        )));
    }
    Ok(())
}

fn validate_w5_control_provenance(
    report: &Value,
    receipt: &MacroReportReceipt,
) -> Result<(), PerfBudgetError> {
    let provenance: W5ControlPlaneProvenance = serde_json::from_value(
        report
            .get("reference_provenance")
            .cloned()
            .ok_or_else(|| PerfBudgetError::new("W5A provenance is absent"))?,
    )
    .map_err(|error| PerfBudgetError::new(format!("W5A provenance is not typed: {error}")))?;
    let cleanup: W5ControlPlaneFinalCleanupReceipt = serde_json::from_value(
        report
            .get("final_cleanup")
            .cloned()
            .filter(|value| !value.is_null())
            .ok_or_else(|| PerfBudgetError::new("W5A final cleanup receipt is absent"))?,
    )
    .map_err(|error| PerfBudgetError::new(format!("W5A cleanup is not typed: {error}")))?;
    let predecessor = report
        .get("predecessor")
        .ok_or_else(|| PerfBudgetError::new("W5A predecessor summary is absent"))?;
    let predecessor_nodes = report.get("predecessor_node_count").and_then(Value::as_u64);
    if provenance.scenario_sha256
        != report
            .get("scenario_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default()
        || predecessor_nodes != Some(3)
        || u64::from(provenance.predecessor_node_count) != predecessor_nodes.unwrap_or_default()
        || provenance.predecessor_artifact_sha256
            != predecessor
                .get("artifact_sha256")
                .and_then(Value::as_str)
                .unwrap_or_default()
        || provenance.predecessor_receipt_sha256
            != predecessor
                .get("reference_receipt_sha256")
                .and_then(Value::as_str)
                .unwrap_or_default()
        || provenance.final_cleanup_receipt_sha256 != cleanup.receipt_sha256
        || [
            &provenance.predecessor_artifact_sha256,
            &provenance.predecessor_receipt_sha256,
            &provenance.execution_capability_receipt_sha256,
            &provenance.final_cleanup_receipt_sha256,
            &provenance.scenario_sha256,
        ]
        .iter()
        .any(|digest| !is_sha256(digest))
        || provenance.receipt_sha256 != receipt_digest_without_field(&provenance)
    {
        return Err(PerfBudgetError::new(
            "W5A provenance receipt does not recompute",
        ));
    }
    validate_w5_final_cleanup(&cleanup, receipt)?;
    Ok(())
}

fn validate_w5_final_cleanup(
    cleanup: &W5ControlPlaneFinalCleanupReceipt,
    receipt: &MacroReportReceipt,
) -> Result<(), PerfBudgetError> {
    if cleanup.receipt_sha256 != receipt_digest_without_field(cleanup)
        || cleanup.nodes.len() < 3
        || cleanup
            .nodes
            .windows(2)
            .any(|nodes| nodes[0].node_id >= nodes[1].node_id)
    {
        return Err(PerfBudgetError::new(
            "W5A final cleanup receipt is empty, unsorted, or unsealed",
        ));
    }
    let expected_server = receipt_binary_sha256(receipt, "hydracache-server")?;
    let mut pids = BTreeSet::new();
    for node in &cleanup.nodes {
        if node.node_id.trim().is_empty()
            || node.pid == 0
            || !pids.insert(node.pid)
            || !node.kill_requested
            || !node.wait_completed
            || !node.process_no_longer_running
            || node.exit_status.trim().is_empty()
            || node.server_binary_sha256_after != expected_server
        {
            return Err(PerfBudgetError::new(
                "W5A final cleanup lacks exact unique killed-and-waited process identity",
            ));
        }
        validate_archived_log(
            &node.stdout_log.canonical_path,
            node.stdout_log.bytes,
            &node.stdout_log.sha256,
            "W5A stdout",
        )?;
        validate_archived_log(
            &node.stderr_log.canonical_path,
            node.stderr_log.bytes,
            &node.stderr_log.sha256,
            "W5A stderr",
        )?;
        rehash_exact_absolute_file(
            &node.server_binary_path_after,
            &node.server_binary_sha256_after,
            "W5A cleanup server binary",
        )?;
        rehash_exact_absolute_file(
            &node.node_config_path_after,
            &node.node_config_sha256_after,
            "W5A cleanup node config",
        )?;
    }
    Ok(())
}

fn validate_w5_resp_provenance(report: &Value) -> Result<(), PerfBudgetError> {
    let provenance: W5RespProvenance = serde_json::from_value(
        report
            .get("reference_provenance")
            .cloned()
            .ok_or_else(|| PerfBudgetError::new("W5B provenance is absent"))?,
    )
    .map_err(|error| PerfBudgetError::new(format!("W5B provenance is not typed: {error}")))?;
    let controls = provenance
        .fresh_control_execution_receipt_sha256
        .iter()
        .collect::<BTreeSet<_>>();
    let predecessor = report
        .get("predecessor")
        .ok_or_else(|| PerfBudgetError::new("W5B predecessor summary is absent"))?;
    let capacity = &provenance.selected_capacity;
    let selected_workload = capacity
        .measurement_id
        .strip_prefix("resp_open_loop_get_set_knee_at_slo_workload_")
        .map(str::to_ascii_uppercase);
    if provenance.scenario_sha256
        != report
            .get("scenario_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default()
        || provenance.selected_predecessor_artifact_sha256
            != predecessor
                .get("artifact_sha256")
                .and_then(Value::as_str)
                .unwrap_or_default()
        || provenance.archived_process_receipt_sha256
            != predecessor
                .get("reference_receipt_sha256")
                .and_then(Value::as_str)
                .unwrap_or_default()
        || !is_sha256(&provenance.selected_predecessor_artifact_sha256)
        || !is_sha256(&provenance.selected_predecessor_lifecycle_sha256)
        || !is_sha256(&provenance.archived_process_receipt_sha256)
        || !is_sha256(&provenance.capacity_matrix_sha256)
        || !is_sha256(&provenance.predecessor_scenario_digest)
        || !is_sha256(&provenance.predecessor_workload_digest)
        || !is_sha256(&provenance.fresh_selected_execution_receipt_sha256)
        || provenance.fresh_selected_execution_receipt_sha256
            == provenance.archived_process_receipt_sha256
        || !is_sha256(&provenance.scenario_sha256)
        || controls.len() != 1
        || controls.len() != provenance.fresh_control_execution_receipt_sha256.len()
        || controls.iter().any(|digest| !is_sha256(digest))
        || provenance
            .fresh_control_execution_receipt_sha256
            .windows(2)
            .any(|digests| digests[0] >= digests[1])
        || controls.iter().any(|digest| {
            digest.as_str() == provenance.fresh_selected_execution_receipt_sha256.as_str()
        })
        || selected_workload
            .as_deref()
            .is_none_or(|workload| !["A", "B", "C"].contains(&workload))
        || !is_sha256(&capacity.scenario_digest)
        || !is_sha256(&capacity.workload.digest)
        || capacity.workload_contract_sha256 != digest_json(&capacity.workload)
        || capacity.connections == 0
        || capacity.pipeline_depth == 0
        || capacity.steady_operations == 0
        || capacity.repeats < 3
        || capacity.key_count == 0
        || capacity.multi_key_width != 10
        || capacity.reset_batch_entries != 128
        || provenance.receipt_sha256 != receipt_digest_without_field(&provenance)
    {
        return Err(PerfBudgetError::new(
            "W5B provenance receipt does not recompute",
        ));
    }
    Ok(())
}

fn validate_w5_grid_provenance(
    report: &Value,
    receipt: &MacroReportReceipt,
) -> Result<(), PerfBudgetError> {
    let provenance: W5GridModelProvenance = serde_json::from_value(
        report
            .get("reference_provenance")
            .cloned()
            .ok_or_else(|| PerfBudgetError::new("W5C provenance is absent"))?,
    )
    .map_err(|error| PerfBudgetError::new(format!("W5C provenance is not typed: {error}")))?;
    let execution = &provenance.fresh_execution;
    let loadgen_sha256 = receipt_binary_sha256(receipt, "hydracache-loadgen")?;
    if provenance.w5_scenario_sha256
        != report
            .get("scenario_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default()
        || provenance.source_commit != receipt.source_commit
        || provenance.runner_fingerprint != receipt.runner_fingerprint
        || provenance.prebuild_manifest_sha256 != receipt.prebuild_manifest_sha256
        || execution.source_commit != receipt.source_commit
        || execution.cargo_lock_sha256 != receipt.cargo_lock_sha256
        || execution.runner_fingerprint != receipt.runner_fingerprint
        || execution.prebuild_manifest_sha256 != receipt.prebuild_manifest_sha256
        || execution.prebuild_contract_sha256 != receipt.prebuild_contract_digest
        || execution.loadgen_sha256 != loadgen_sha256
        || execution.process_id == 0
        || execution.started_unix_nanos == 0
        || execution.sequence == 0
        || execution.receipt_sha256 != receipt_digest_without_field(execution)
        || [
            &provenance.w4b_artifact_sha256,
            &provenance.w4b_reference_receipt_sha256,
            &provenance.w4b_scenario_sha256,
            &provenance.prebuild_manifest_sha256,
            &provenance.w5_scenario_sha256,
        ]
        .iter()
        .any(|digest| !is_sha256(digest))
        || !is_git_commit(&provenance.source_commit)
        || provenance.receipt_sha256 != receipt_digest_without_field(&provenance)
    {
        return Err(PerfBudgetError::new(
            "W5C provenance receipt does not recompute",
        ));
    }
    rehash_exact_absolute_file(
        &execution.loadgen_canonical_path,
        &execution.loadgen_sha256,
        "W5C fresh loadgen binary",
    )?;
    Ok(())
}

fn rehash_exact_absolute_file(
    path: &Path,
    expected_sha256: &str,
    label: &str,
) -> Result<(), PerfBudgetError> {
    if !path.is_absolute() || !is_sha256(expected_sha256) {
        return Err(PerfBudgetError::new(format!(
            "{label} path/digest is incomplete"
        )));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|error| PerfBudgetError::new(format!("canonicalizing {label}: {error}")))?;
    if canonical != path || sha256_file(&canonical)? != expected_sha256 {
        return Err(PerfBudgetError::new(format!(
            "{label} changed after its capability receipt"
        )));
    }
    Ok(())
}

fn require_exact_object_keys(
    value: &Value,
    expected: &[&str],
    label: &str,
) -> Result<(), PerfBudgetError> {
    let object = value
        .as_object()
        .ok_or_else(|| PerfBudgetError::new(format!("{label} is not an object")))?;
    let observed = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    if observed != expected {
        return Err(PerfBudgetError::new(format!(
            "{label} has missing or unknown fields"
        )));
    }
    Ok(())
}

/// Recompute the W4B reference summary from its five raw, fresh-model
/// observations. Returns `(median_elapsed_nanos, spread_ratio)`.
fn validate_primitive_timing(
    timing: &Value,
    iterations: u64,
) -> Result<(u64, f64), PerfBudgetError> {
    require_exact_object_keys(
        timing,
        &[
            "raw_repeats",
            "median_elapsed_nanos",
            "robust_spread_ratio_millionths",
            "stable",
        ],
        "grid-model timing",
    )?;
    if iterations != 10_000 {
        return Err(PerfBudgetError::new(
            "grid-model timing changed the committed iteration count",
        ));
    }
    let repeats = timing
        .get("raw_repeats")
        .and_then(Value::as_array)
        .filter(|repeats| repeats.len() == 5)
        .ok_or_else(|| PerfBudgetError::new("grid-model timing needs five raw repeats"))?;
    let mut elapsed = Vec::with_capacity(repeats.len());
    let mut identities = BTreeSet::new();
    for (index, repeat) in repeats.iter().enumerate() {
        require_exact_object_keys(
            repeat,
            &[
                "repeat_index",
                "warmup_iterations",
                "steady_iterations",
                "fresh_model_identity_sha256",
                "elapsed_nanos",
                "result_checksum",
            ],
            "grid-model raw repeat",
        )?;
        let identity = repeat
            .get("fresh_model_identity_sha256")
            .and_then(Value::as_str);
        let value = repeat.get("elapsed_nanos").and_then(Value::as_u64);
        if repeat.get("repeat_index").and_then(Value::as_u64) != Some(index as u64)
            || repeat.get("warmup_iterations").and_then(Value::as_u64) != Some(1_000)
            || repeat.get("steady_iterations").and_then(Value::as_u64) != Some(iterations)
            || identity.is_none_or(|identity| !is_sha256(identity) || !identities.insert(identity))
            || value == Some(0)
            || repeat
                .get("result_checksum")
                .and_then(Value::as_u64)
                .is_none_or(|value| value == 0)
        {
            return Err(PerfBudgetError::new(
                "grid-model raw repeat is not a fresh warm reference observation",
            ));
        }
        elapsed
            .push(value.ok_or_else(|| {
                PerfBudgetError::new("grid-model raw repeat has no elapsed time")
            })?);
    }
    let median = median_u64(&mut elapsed)
        .ok_or_else(|| PerfBudgetError::new("grid-model timing is empty"))?;
    let spread_millionths = robust_spread_millionths(&elapsed, median)?;
    let stable = spread_millionths <= 150_000;
    if timing.get("median_elapsed_nanos").and_then(Value::as_u64) != Some(median)
        || timing
            .get("robust_spread_ratio_millionths")
            .and_then(Value::as_u64)
            != Some(spread_millionths)
        || timing.get("stable").and_then(Value::as_bool) != Some(stable)
    {
        return Err(PerfBudgetError::new(
            "grid-model median/spread/stability does not recompute",
        ));
    }
    Ok((median, spread_millionths as f64 / 1_000_000.0))
}

fn median_u64(values: &mut [u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    Some(values[values.len() / 2])
}

fn robust_spread_millionths(values: &[u64], center: u64) -> Result<u64, PerfBudgetError> {
    if values.is_empty() || center == 0 {
        return Err(PerfBudgetError::new(
            "raw timing spread has no positive center",
        ));
    }
    let mut deviations = values
        .iter()
        .map(|value| value.abs_diff(center))
        .collect::<Vec<_>>();
    let mad = median_u64(&mut deviations)
        .ok_or_else(|| PerfBudgetError::new("raw timing spread is empty"))?;
    Ok(u64::try_from(
        u128::from(mad)
            .saturating_mul(1_000_000)
            .checked_div(u128::from(center))
            .unwrap_or(u128::MAX)
            .min(u128::from(u64::MAX)),
    )
    .unwrap_or(u64::MAX))
}

fn availability_dip_ppm(window: &Value) -> Result<u64, PerfBudgetError> {
    require_exact_object_keys(
        window,
        &[
            "offered",
            "started",
            "completed",
            "successes",
            "errors",
            "timeouts",
            "rejections",
            "backlog_high_water",
            "backlog_drained",
            "drain_ms",
            "elapsed_ms",
            "offered_rate_per_second",
            "achieved_rate_per_second",
            "latency",
        ],
        "raw open-loop window",
    )?;
    let read = |field: &str| window.get(field).and_then(Value::as_u64);
    let offered = read("offered")
        .filter(|value| *value > 0)
        .ok_or_else(|| PerfBudgetError::new("raw open-loop window has no offered operations"))?;
    let started = read("started").unwrap_or(u64::MAX);
    let completed = read("completed").unwrap_or(u64::MAX);
    let successes = read("successes").unwrap_or(u64::MAX);
    let outcomes = successes
        .checked_add(read("errors").unwrap_or(u64::MAX))
        .and_then(|value| value.checked_add(read("timeouts").unwrap_or(u64::MAX)))
        .and_then(|value| value.checked_add(read("rejections").unwrap_or(u64::MAX)));
    let latency = window.get("latency").and_then(Value::as_object);
    if started != offered
        || completed != started
        || outcomes != Some(completed)
        || window.get("backlog_drained").and_then(Value::as_bool) != Some(true)
        || latency
            .and_then(|latency| latency.get("samples"))
            .and_then(Value::as_u64)
            != Some(completed)
        || latency
            .and_then(|latency| latency.get("overflow_count"))
            .and_then(Value::as_u64)
            != Some(0)
    {
        return Err(PerfBudgetError::new(
            "raw open-loop window counters/backlog/latency do not balance",
        ));
    }
    let availability = u128::from(successes)
        .saturating_mul(1_000_000)
        .checked_div(u128::from(offered))
        .unwrap_or(0)
        .min(1_000_000);
    Ok(1_000_000_u64.saturating_sub(availability as u64))
}

fn validate_model_fault_timing(fault: &Value) -> Result<(u64, u64, u64, f64), PerfBudgetError> {
    require_exact_object_keys(
        fault,
        &[
            "fault",
            "primitive",
            "fault_adapter",
            "raw_repeats",
            "baseline_timing",
            "fault_timing",
            "recovery_timing",
            "affected_decisions",
            "injected_fault_events",
            "independent_result_checksum",
        ],
        "grid-model fault evidence",
    )?;
    if fault.get("primitive").and_then(Value::as_str) != Some("LiveReplicationPeer::send_record")
        || fault.get("affected_decisions").and_then(Value::as_u64) != Some(5_000)
        || fault.get("injected_fault_events").and_then(Value::as_u64) != Some(5_000)
        || fault
            .get("independent_result_checksum")
            .and_then(Value::as_u64)
            .is_none_or(|v| v == 0)
    {
        return Err(PerfBudgetError::new(
            "grid-model fault is not bound to the real primitive and exact work",
        ));
    }
    let repeats = fault
        .get("raw_repeats")
        .and_then(Value::as_array)
        .filter(|repeats| repeats.len() == 5)
        .ok_or_else(|| PerfBudgetError::new("grid-model fault needs five raw repeats"))?;
    let mut identities = BTreeSet::new();
    for (index, repeat) in repeats.iter().enumerate() {
        require_exact_object_keys(
            repeat,
            &[
                "repeat_index",
                "warmup_iterations",
                "steady_iterations",
                "fresh_model_identity_sha256",
                "baseline_elapsed_nanos",
                "baseline_result_checksum",
                "baseline_admitted_sends",
                "fault_elapsed_nanos",
                "fault_result_checksum",
                "injected_fault_events",
                "unavailable_decisions",
                "slow_primitive_calls",
                "recovery_elapsed_nanos",
                "recovery_result_checksum",
                "recovery_admitted_sends",
                "final_record_checksum",
            ],
            "grid-model fault raw repeat",
        )?;
        let identity = repeat
            .get("fresh_model_identity_sha256")
            .and_then(Value::as_str);
        if repeat.get("repeat_index").and_then(Value::as_u64) != Some(index as u64)
            || repeat.get("warmup_iterations").and_then(Value::as_u64) != Some(100)
            || repeat.get("steady_iterations").and_then(Value::as_u64) != Some(1_000)
            || identity.is_none_or(|identity| !is_sha256(identity) || !identities.insert(identity))
            || [
                "baseline_elapsed_nanos",
                "fault_elapsed_nanos",
                "recovery_elapsed_nanos",
                "baseline_result_checksum",
                "fault_result_checksum",
                "recovery_result_checksum",
                "final_record_checksum",
            ]
            .iter()
            .any(|field| {
                repeat
                    .get(*field)
                    .and_then(Value::as_u64)
                    .is_none_or(|v| v == 0)
            })
            || repeat.get("injected_fault_events").and_then(Value::as_u64) != Some(1_000)
        {
            return Err(PerfBudgetError::new(
                "grid-model fault raw repeat is not fresh, warm, complete evidence",
            ));
        }
    }
    let baseline = validate_model_summary(
        fault
            .get("baseline_timing")
            .ok_or_else(|| PerfBudgetError::new("baseline timing absent"))?,
        repeats,
        "baseline_elapsed_nanos",
    )?;
    let fault_cost = validate_model_summary(
        fault
            .get("fault_timing")
            .ok_or_else(|| PerfBudgetError::new("fault timing absent"))?,
        repeats,
        "fault_elapsed_nanos",
    )?;
    let recovery = validate_model_summary(
        fault
            .get("recovery_timing")
            .ok_or_else(|| PerfBudgetError::new("recovery timing absent"))?,
        repeats,
        "recovery_elapsed_nanos",
    )?;
    Ok((
        baseline.0,
        fault_cost.0,
        recovery.0,
        baseline.1.max(fault_cost.1).max(recovery.1),
    ))
}

fn validate_model_summary(
    summary: &Value,
    repeats: &[Value],
    elapsed_field: &str,
) -> Result<(u64, f64), PerfBudgetError> {
    require_exact_object_keys(
        summary,
        &[
            "median_nanos_per_iteration",
            "robust_spread_ratio_millionths",
            "stable",
        ],
        "grid-model fault timing summary",
    )?;
    let mut samples = repeats
        .iter()
        .map(|repeat| {
            repeat
                .get(elapsed_field)
                .and_then(Value::as_u64)
                .map(|value| value.saturating_add(999) / 1_000)
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| PerfBudgetError::new("grid-model fault elapsed samples are absent"))?;
    let median = median_u64(&mut samples)
        .ok_or_else(|| PerfBudgetError::new("grid-model fault timing is empty"))?
        .max(1);
    let spread = robust_spread_millionths(&samples, median)?;
    let stable = spread <= 1_000_000;
    if summary
        .get("median_nanos_per_iteration")
        .and_then(Value::as_u64)
        != Some(median)
        || summary
            .get("robust_spread_ratio_millionths")
            .and_then(Value::as_u64)
            != Some(spread)
        || summary.get("stable").and_then(Value::as_bool) != Some(stable)
    {
        return Err(PerfBudgetError::new(
            "grid-model fault median/spread/stability does not recompute",
        ));
    }
    Ok((median, spread as f64 / 1_000_000.0))
}

fn validate_overload_budget_report(
    report_id: &str,
    report: &Value,
    receipt: &MacroReportReceipt,
) -> Result<(f64, f64), PerfBudgetError> {
    let typed: OverloadReport = deserialize_typed_report(report_id, report)?;
    require_exact_object_keys(
        report,
        &[
            "schema_version",
            "release",
            "report_id",
            "scenario_id",
            "scenario_digest_sha256",
            "evidence_class",
            "claim_scope",
            "run_mode",
            "surface",
            "surface_identity",
            "predecessor",
            "target_binding",
            "daemon_lifecycle",
            "admission_control_mode",
            "baseline_goodput_per_second",
            "baseline_scheduled_p99_us",
            "points",
            "generic_cluster_capacity_claim",
            "node_native_wire_claim",
            "library_model_capacity_claim",
            "deferred_claims",
        ],
        "overload report",
    )?;
    let expected_surface = match report_id {
        "overload-local" => "local",
        "overload-client-surface" => "client_surface",
        "overload-node-resp" => "node_resp",
        _ => return Err(PerfBudgetError::new("unknown overload surface")),
    };
    if report.get("schema_version").and_then(Value::as_u64) != Some(1)
        || report.get("release").and_then(Value::as_str) != Some(RELEASE)
        || report.get("run_mode").and_then(Value::as_str) != Some("reference")
        || report.get("surface").and_then(Value::as_str) != Some(expected_surface)
        || report.get("target_binding").is_none_or(Value::is_null)
        || report.get("admission_control_mode").and_then(Value::as_str) != Some("enabled")
        || [
            "generic_cluster_capacity_claim",
            "node_native_wire_claim",
            "library_model_capacity_claim",
        ]
        .iter()
        .any(|field| report.get(*field).and_then(Value::as_bool) != Some(false))
    {
        return Err(PerfBudgetError::new(
            "overload budget requires an exact reference surface adapter without tier overclaims",
        ));
    }
    validate_overload_reference_binding(report_id, report, &typed, receipt)?;
    let baseline_goodput = report
        .get("baseline_goodput_per_second")
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or_else(|| PerfBudgetError::new("overload baseline goodput is invalid"))?;
    let baseline_p99 = report
        .get("baseline_scheduled_p99_us")
        .and_then(Value::as_u64)
        .filter(|value| *value > 0)
        .ok_or_else(|| PerfBudgetError::new("overload baseline p99 is invalid"))?;
    let points = report
        .get("points")
        .and_then(Value::as_array)
        .filter(|points| points.len() == 3)
        .ok_or_else(|| PerfBudgetError::new("overload report needs three factor points"))?;
    let factors = [1_200_000_u64, 1_500_000, 2_000_000];
    let mut minimum_goodput = f64::INFINITY;
    let mut maximum_spread = 0.0_f64;
    let mut common_reset = None;
    let mut common_preloaded = None;
    let mut knee_rate = None;
    for (point_index, point) in points.iter().enumerate() {
        require_exact_object_keys(
            point,
            &[
                "factor_millionths",
                "offered_rate_per_second",
                "repeats",
                "aggregate",
            ],
            "overload factor point",
        )?;
        let factor = factors[point_index];
        let offered_rate = point
            .get("offered_rate_per_second")
            .and_then(Value::as_u64)
            .filter(|value| *value > 0)
            .ok_or_else(|| PerfBudgetError::new("overload offered rate is absent"))?;
        if point.get("factor_millionths").and_then(Value::as_u64) != Some(factor) {
            return Err(PerfBudgetError::new("overload factor schedule changed"));
        }
        let derived_knee = u128::from(offered_rate)
            .saturating_mul(1_000_000)
            .checked_div(u128::from(factor))
            .and_then(|value| u64::try_from(value).ok())
            .filter(|value| *value > 0)
            .ok_or_else(|| PerfBudgetError::new("overload knee rate cannot be derived"))?;
        if knee_rate
            .replace(derived_knee)
            .is_some_and(|prior| prior != derived_knee)
        {
            return Err(PerfBudgetError::new(
                "overload factors are not bound to one predecessor knee",
            ));
        }
        let repeats = point
            .get("repeats")
            .and_then(Value::as_array)
            .filter(|repeats| repeats.len() == 3)
            .ok_or_else(|| PerfBudgetError::new("overload point needs three raw repeats"))?;
        let mut repeat_goodput = Vec::with_capacity(3);
        for repeat in repeats {
            let (goodput, reset, preloaded) = validate_overload_repeat(
                repeat,
                factor as u32,
                offered_rate,
                derived_knee,
                baseline_goodput,
                baseline_p99,
            )?;
            if common_reset
                .replace(reset.clone())
                .is_some_and(|prior| prior != reset)
                || common_preloaded
                    .replace(preloaded.clone())
                    .is_some_and(|prior| prior != preloaded)
                || reset != preloaded
            {
                return Err(PerfBudgetError::new(
                    "overload reset/preload digests differ across factors/repeats",
                ));
            }
            repeat_goodput.push(goodput);
        }
        let mut order = (0..repeat_goodput.len()).collect::<Vec<_>>();
        order.sort_by(|left, right| {
            repeat_goodput[*left]
                .total_cmp(&repeat_goodput[*right])
                .then_with(|| left.cmp(right))
        });
        let representative = order[1];
        let minimum = repeat_goodput.iter().copied().fold(f64::INFINITY, f64::min);
        let maximum = repeat_goodput
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let median = repeat_goodput[representative];
        let spread = if median > 0.0 {
            (maximum - minimum) / median
        } else if approx_eq(maximum, minimum) {
            0.0
        } else {
            f64::INFINITY
        };
        let aggregate = point
            .get("aggregate")
            .ok_or_else(|| PerfBudgetError::new("overload aggregate is absent"))?;
        if aggregate
            .get("representative_repeat_index")
            .and_then(Value::as_u64)
            != Some(representative as u64)
            || !json_f64_eq(aggregate, "successful_goodput_per_second", median)
            || !json_f64_eq(aggregate, "goodput_min_per_second", minimum)
            || !json_f64_eq(aggregate, "goodput_max_per_second", maximum)
            || !json_f64_eq(aggregate, "robust_goodput_spread_ratio", spread)
            || spread > 0.25
        {
            return Err(PerfBudgetError::new(
                "overload aggregate/spread does not recompute from raw repeats",
            ));
        }
        minimum_goodput = minimum_goodput.min(median);
        maximum_spread = maximum_spread.max(spread);
    }
    Ok((minimum_goodput, maximum_spread))
}

fn validate_overload_reference_binding(
    report_id: &str,
    report: &Value,
    typed: &OverloadReport,
    receipt: &MacroReportReceipt,
) -> Result<(), PerfBudgetError> {
    let expected_surface = match report_id {
        "overload-local" => EligibleOverloadSurface::Local,
        "overload-client-surface" => EligibleOverloadSurface::ClientSurface,
        "overload-node-resp" => EligibleOverloadSurface::NodeResp,
        _ => return Err(PerfBudgetError::new("unknown overload surface")),
    };
    let expected_source_id = match expected_surface {
        EligibleOverloadSurface::Local => "overload-local-v1",
        EligibleOverloadSurface::ClientSurface => "overload-client-surface-v1",
        EligibleOverloadSurface::NodeResp => "overload-node-resp-v1",
    };
    if typed.run_mode != OverloadRunMode::Reference
        || typed.surface != expected_surface
        || typed.report_id != expected_source_id
        || typed.predecessor.surface != expected_surface
        || !typed.predecessor.stable_capacity_evidence
    {
        return Err(PerfBudgetError::new(
            "W6 typed report is not the exact stable reference surface",
        ));
    }
    let binding = typed.target_binding.as_ref().ok_or_else(|| {
        PerfBudgetError::new("W6 reference report has no typed fresh target binding")
    })?;
    let expected_contract = typed
        .predecessor
        .reference_target_contract()
        .map_err(|error| PerfBudgetError::new(format!("invalid W6 predecessor: {error}")))?;
    if binding.contract != expected_contract
        || binding.contract.surface != expected_surface
        || binding.contract.stable_surface_capability_sha256
            != typed.predecessor.stable_surface_capability_sha256
        || binding.contract.workload_identity_sha256 != typed.predecessor.workload_identity_sha256
    {
        return Err(PerfBudgetError::new(
            "W6 fresh target contract differs from its exact capacity predecessor",
        ));
    }
    let predecessor_value = report
        .get("predecessor")
        .ok_or_else(|| PerfBudgetError::new("W6 predecessor JSON is absent"))?;
    let predecessor_receipt: W6ReferencePredecessorReceipt = serde_json::from_value(
        predecessor_value
            .get("reference_receipt")
            .cloned()
            .filter(|value| !value.is_null())
            .ok_or_else(|| PerfBudgetError::new("W6 reference predecessor receipt is absent"))?,
    )
    .map_err(|error| {
        PerfBudgetError::new(format!(
            "W6 reference predecessor receipt is not typed: {error}"
        ))
    })?;
    let expected_payload = typed
        .predecessor
        .payload_sha256()
        .map_err(|error| PerfBudgetError::new(format!("invalid W6 payload: {error}")))?;
    let allowed_measurements: &[&str] = match expected_surface {
        EligibleOverloadSurface::Local => &["hot_key_contention_throughput_floor"],
        EligibleOverloadSurface::ClientSurface => &[
            "client_surface_in_process_knee_at_slo_workload_a",
            "client_surface_in_process_knee_at_slo_workload_b",
            "client_surface_in_process_knee_at_slo_workload_c",
        ],
        EligibleOverloadSurface::NodeResp => &[
            "resp_open_loop_get_set_knee_at_slo_workload_a",
            "resp_open_loop_get_set_knee_at_slo_workload_b",
            "resp_open_loop_get_set_knee_at_slo_workload_c",
        ],
    };
    if predecessor_receipt.profile != "reference-v1"
        || !allowed_measurements.contains(&predecessor_receipt.predecessor_measurement_id.as_str())
        || predecessor_receipt.predecessor_payload_sha256 != expected_payload
        || predecessor_receipt.stable_surface_capability_sha256
            != typed.predecessor.stable_surface_capability_sha256
        || predecessor_receipt.workload_identity_sha256
            != typed.predecessor.workload_identity_sha256
        || predecessor_receipt.source_commit != binding.contract.source_commit
        || predecessor_receipt.cargo_lock_sha256 != binding.contract.cargo_lock_sha256
        || predecessor_receipt.prebuild_receipt_sha256 != binding.contract.prebuild_manifest_sha256
        || predecessor_receipt.runner_fingerprint_sha256 != digest_json(&receipt.observed_runner)
        || predecessor_receipt.source_commit != receipt.source_commit
        || predecessor_receipt.cargo_lock_sha256 != receipt.cargo_lock_sha256
        || predecessor_receipt.prebuild_receipt_sha256 != receipt.prebuild_manifest_sha256
        || predecessor_receipt.archived_execution_pid == 0
        || predecessor_receipt.receipt_sha256 != receipt_digest_without_field(&predecessor_receipt)
    {
        return Err(PerfBudgetError::new(
            "W6 predecessor receipt is unsealed or loses capacity/source/build identity",
        ));
    }
    validate_w6_predecessor_files(expected_surface, &predecessor_receipt)?;

    let execution_value = report
        .pointer("/target_binding/execution")
        .cloned()
        .ok_or_else(|| PerfBudgetError::new("W6 fresh execution receipt is absent"))?;
    let execution: W6FreshExecutionReceipt = serde_json::from_value(execution_value)
        .map_err(|error| PerfBudgetError::new(format!("W6 execution is not typed: {error}")))?;
    if execution.schema_version != 1
        || execution.surface != expected_surface
        || execution.instance_sequence == 0
        || execution.owning_pid == 0
        || execution.started_unix_nanos == 0
        || !execution.direct_prebuilt_exec
        || execution.stable_surface_capability_sha256
            != binding.contract.stable_surface_capability_sha256
        || execution.receipt_sha256 != receipt_digest_without_field(&execution)
        || execution.receipt_sha256 == predecessor_receipt.archived_execution_receipt_sha256
        || (expected_surface == EligibleOverloadSurface::NodeResp
            && execution.owning_pid == predecessor_receipt.archived_execution_pid)
    {
        return Err(PerfBudgetError::new(
            "W6 fresh execution receipt is unsealed or reuses archived runtime identity",
        ));
    }
    match expected_surface {
        EligibleOverloadSurface::Local | EligibleOverloadSurface::ClientSurface => {
            if execution.kind != ReferenceExecutionKind::InProcess
                || execution.runtime_capability_sha256.is_some()
                || execution.selected_endpoint.is_some()
                || binding.resp_runtime_capability.is_some()
                || typed.daemon_lifecycle.is_some()
            {
                return Err(PerfBudgetError::new(
                    "in-process W6 evidence carries a daemon/runtime identity",
                ));
            }
        }
        EligibleOverloadSurface::NodeResp => {
            validate_w6_resp_execution(binding, &execution, typed, receipt)?;
        }
    }
    Ok(())
}

fn validate_w6_predecessor_files(
    surface: EligibleOverloadSurface,
    receipt: &W6ReferencePredecessorReceipt,
) -> Result<(), PerfBudgetError> {
    rehash_exact_absolute_file(
        &receipt.predecessor_report_path,
        &receipt.predecessor_report_sha256,
        "W6 raw predecessor report",
    )?;
    rehash_exact_absolute_file(
        &receipt.prebuild_manifest_path,
        &receipt.prebuild_receipt_sha256,
        "W6 predecessor prebuild manifest",
    )?;
    match (
        surface,
        receipt.predecessor_lifecycle_path.as_ref(),
        receipt.predecessor_lifecycle_sha256.as_ref(),
    ) {
        (EligibleOverloadSurface::NodeResp, Some(path), Some(sha256)) => {
            rehash_exact_absolute_file(path, sha256, "W6 archived W3 lifecycle")?;
        }
        (EligibleOverloadSurface::Local | EligibleOverloadSurface::ClientSurface, None, None) => {}
        _ => {
            return Err(PerfBudgetError::new(
                "W6 predecessor lifecycle shape differs from its eligible surface",
            ));
        }
    }
    Ok(())
}

fn validate_w6_resp_execution(
    binding: &hydracache_loadgen::overload::ReferenceTargetBinding,
    execution: &W6FreshExecutionReceipt,
    report: &OverloadReport,
    receipt: &MacroReportReceipt,
) -> Result<(), PerfBudgetError> {
    if execution.kind != ReferenceExecutionKind::DirectDaemon {
        return Err(PerfBudgetError::new(
            "node-RESP W6 execution is not a direct daemon",
        ));
    }
    let capability = binding
        .resp_runtime_capability
        .as_ref()
        .ok_or_else(|| PerfBudgetError::new("node-RESP W6 binding has no runtime capability"))?;
    let capability_sha256 = capability
        .digest()
        .map_err(|error| PerfBudgetError::new(format!("invalid W6 RESP capability: {error}")))?;
    let stable = NodeRespStableCapability {
        schema_version: 1,
        surface: binding.contract.surface_identity.clone(),
        direct_prebuilt_exec: capability.direct_prebuilt_exec,
        fresh_data_dir_per_run: capability.fresh_data_dir,
        role: capability.config.role.clone(),
        listen_addr: capability.config.listen_addr,
        cluster_addr: capability.config.cluster_addr,
        admin_enabled: capability.config.admin_enabled,
        redis_enabled: capability.config.redis_enabled,
        redis_auth_required: capability.config.redis_auth_required,
        rediss_enabled: capability.config.rediss_enabled,
        server_binary_sha256: capability.server_binary_sha256.clone(),
        loadgen_binary_sha256: capability.loadgen_binary_sha256.clone(),
        prebuild_manifest_sha256: capability.prebuild_manifest_sha256.clone(),
        prebuild_contract_sha256: capability.prebuild_contract_digest.clone(),
        source_commit: capability.source_commit.clone(),
        cargo_lock_sha256: binding.contract.cargo_lock_sha256.clone(),
    };
    let lifecycle = report.daemon_lifecycle.as_ref().ok_or_else(|| {
        PerfBudgetError::new("node-RESP W6 report has no fresh killed-and-waited lifecycle")
    })?;
    if digest_json(&stable) != binding.contract.stable_surface_capability_sha256
        || execution.runtime_capability_sha256.as_deref() != Some(capability_sha256.as_str())
        || execution.selected_endpoint.as_deref() != Some(capability.selected_endpoint.as_str())
        || execution.owning_pid != capability.pid
        || lifecycle.pid != capability.pid
        || lifecycle.repeat_index != capability.repeat_index
        || lifecycle.resp_endpoint != capability.config.redis_addr
        || lifecycle.admin_endpoint != capability.config.admin_addr
        || lifecycle.data_dir != capability.config.storage_dir
        || lifecycle.selected_endpoint != capability.selected_endpoint
        || lifecycle.endpoint_capability_digest != capability_sha256
        || lifecycle.server_binary_sha256 != capability.server_binary_sha256
        || lifecycle.loadgen_binary_sha256 != capability.loadgen_binary_sha256
        || lifecycle.server_binary_sha256 != receipt_binary_sha256(receipt, "hydracache-server")?
        || lifecycle.loadgen_binary_sha256 != receipt_binary_sha256(receipt, "hydracache-loadgen")?
        || !lifecycle.direct_prebuilt_exec
        || !lifecycle.binaries_verified_after_measurement
        || !lifecycle.killed_and_waited
        || lifecycle.readiness.selected_endpoint != lifecycle.resp_endpoint
        || lifecycle.readiness.attempts == 0
        || lifecycle.readiness.exact_response != "+PONG\\r\\n"
        || lifecycle.readiness.request_sha256 != sha256(b"*1\r\n$4\r\nPING\r\n")
        || lifecycle.readiness.response_sha256 != sha256(b"+PONG\r\n")
    {
        return Err(PerfBudgetError::new(
            "node-RESP W6 runtime, stable capability, execution, and lifecycle do not cross-bind",
        ));
    }
    validate_archived_log(
        &lifecycle.stdout_log.canonical_path,
        lifecycle.stdout_log.bytes,
        &lifecycle.stdout_log.sha256,
        "W6 node-RESP stdout",
    )?;
    validate_archived_log(
        &lifecycle.stderr_log.canonical_path,
        lifecycle.stderr_log.bytes,
        &lifecycle.stderr_log.sha256,
        "W6 node-RESP stderr",
    )?;
    rehash_exact_absolute_file(
        &lifecycle.server_binary_path,
        &lifecycle.server_binary_sha256,
        "W6 node-RESP server binary",
    )?;
    rehash_exact_absolute_file(
        &lifecycle.loadgen_binary_path,
        &lifecycle.loadgen_binary_sha256,
        "W6 node-RESP loadgen binary",
    )?;
    Ok(())
}

fn validate_overload_repeat(
    repeat: &Value,
    factor: u32,
    offered_rate: u64,
    knee_rate: u64,
    baseline_goodput: f64,
    baseline_p99: u64,
) -> Result<(f64, String, String), PerfBudgetError> {
    require_exact_object_keys(
        repeat,
        &[
            "reset_state_digest",
            "preloaded_state_digest",
            "steady_state_digest",
            "preload_operations",
            "warmup_operations",
            "admission_control",
            "overload",
            "metrics",
            "recovery",
        ],
        "overload raw repeat",
    )?;
    let reset = repeat
        .get("reset_state_digest")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| PerfBudgetError::new("overload reset digest is absent"))?
        .to_owned();
    let preloaded = repeat
        .get("preloaded_state_digest")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| PerfBudgetError::new("overload preload digest is absent"))?
        .to_owned();
    if repeat
        .get("steady_state_digest")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
        || repeat.get("preload_operations").and_then(Value::as_u64) != Some(0)
        || repeat.get("warmup_operations").and_then(Value::as_u64) != Some(4)
    {
        return Err(PerfBudgetError::new(
            "overload repeat lifecycle differs from the committed contract",
        ));
    }
    let admission: AdmissionControlReceipt = serde_json::from_value(
        repeat
            .get("admission_control")
            .cloned()
            .ok_or_else(|| PerfBudgetError::new("overload admission receipt is absent"))?,
    )
    .map_err(|error| PerfBudgetError::new(format!("invalid admission receipt: {error}")))?;
    if admission.mode != "enabled"
        || admission.factor_millionths != factor
        || admission.authority.trim().is_empty()
        || !is_sha256(&admission.configuration_sha256)
        || admission.receipt_sha256 != admission.recomputed_receipt()
    {
        return Err(PerfBudgetError::new(
            "overload admission receipt does not recompute",
        ));
    }
    let overload = repeat
        .get("overload")
        .ok_or_else(|| PerfBudgetError::new("overload raw window is absent"))?;
    let derived = overload_metrics(overload, offered_rate, 48)?;
    let stored = repeat
        .get("metrics")
        .ok_or_else(|| PerfBudgetError::new("overload metrics are absent"))?;
    if !json_f64_eq(stored, "successful_goodput_per_second", derived.goodput)
        || stored.get("scheduled_p99_us").and_then(Value::as_u64) != Some(derived.p99)
        || !json_f64_eq(stored, "rejection_ratio", derived.rejection_ratio)
        || !json_f64_eq(stored, "error_timeout_ratio", derived.error_timeout_ratio)
        || stored.get("backlog_high_water").and_then(Value::as_u64) != Some(derived.backlog)
        || stored.get("backlog_drained").and_then(Value::as_bool) != Some(derived.drained)
    {
        return Err(PerfBudgetError::new(
            "overload metrics do not recompute from the raw scheduled window",
        ));
    }
    validate_overload_recovery(
        repeat
            .get("recovery")
            .ok_or_else(|| PerfBudgetError::new("overload recovery evidence is absent"))?,
        knee_rate,
        baseline_goodput,
        baseline_p99,
    )?;
    Ok((derived.goodput, reset, preloaded))
}

#[derive(Debug, Clone, Copy)]
struct DerivedOverloadMetrics {
    goodput: f64,
    p99: u64,
    rejection_ratio: f64,
    error_timeout_ratio: f64,
    backlog: u64,
    drained: bool,
    elapsed_ms: u64,
}

fn overload_metrics(
    window: &Value,
    expected_rate: u64,
    expected_operations: u64,
) -> Result<DerivedOverloadMetrics, PerfBudgetError> {
    let read = |field: &str| window.get(field).and_then(Value::as_u64);
    let offered = read("offered").unwrap_or(u64::MAX);
    let started = read("started").unwrap_or(u64::MAX);
    let completed = read("completed").unwrap_or(u64::MAX);
    let successes = read("successes").unwrap_or(u64::MAX);
    let errors = read("errors").unwrap_or(u64::MAX);
    let timeouts = read("timeouts").unwrap_or(u64::MAX);
    let rejections = read("rejections").unwrap_or(u64::MAX);
    let outcomes = successes
        .checked_add(errors)
        .and_then(|value| value.checked_add(timeouts))
        .and_then(|value| value.checked_add(rejections));
    let elapsed_ms = read("elapsed_ms")
        .filter(|value| *value > 0)
        .ok_or_else(|| PerfBudgetError::new("overload raw window has no elapsed time"))?;
    let p99 = window
        .pointer("/latency/p99_us")
        .and_then(Value::as_u64)
        .filter(|value| *value > 0)
        .ok_or_else(|| PerfBudgetError::new("overload raw window has no p99"))?;
    if offered != expected_operations
        || started != offered
        || completed > started
        || outcomes != Some(completed)
        || window.pointer("/latency/samples").and_then(Value::as_u64) != Some(completed)
        || window
            .pointer("/latency/overflow_count")
            .and_then(Value::as_u64)
            != Some(0)
        || !window
            .get("offered_rate_per_second")
            .and_then(Value::as_f64)
            .is_some_and(|value| approx_eq(value, expected_rate as f64))
    {
        return Err(PerfBudgetError::new(
            "overload raw window is unbalanced or bound to the wrong open-loop schedule",
        ));
    }
    let denominator = started.max(1) as f64;
    Ok(DerivedOverloadMetrics {
        goodput: successes as f64 / (elapsed_ms as f64 / 1_000.0),
        p99,
        rejection_ratio: rejections as f64 / denominator,
        error_timeout_ratio: errors.saturating_add(timeouts) as f64 / denominator,
        backlog: read("backlog_high_water").unwrap_or(u64::MAX),
        drained: window
            .get("backlog_drained")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        elapsed_ms,
    })
}

fn validate_overload_recovery(
    recovery: &Value,
    knee_rate: u64,
    baseline_goodput: f64,
    baseline_p99: u64,
) -> Result<(), PerfBudgetError> {
    require_exact_object_keys(
        recovery,
        &[
            "transition_duration_ms",
            "windows",
            "recovered_at_window",
            "consecutive_passing_windows",
            "observed_recovery_ms",
            "time_to_baseline_ms",
            "final_state_digest",
        ],
        "overload recovery evidence",
    )?;
    if recovery
        .get("final_state_digest")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err(PerfBudgetError::new(
            "overload recovery state digest is absent",
        ));
    }
    let windows = recovery
        .get("windows")
        .and_then(Value::as_array)
        .filter(|windows| !windows.is_empty() && windows.len() <= 3)
        .ok_or_else(|| PerfBudgetError::new("overload recovery window set is invalid"))?;
    let transition = recovery
        .get("transition_duration_ms")
        .and_then(Value::as_u64)
        .ok_or_else(|| PerfBudgetError::new("overload transition duration is absent"))?;
    let mut elapsed = transition;
    let mut consecutive = 0_u64;
    let mut recovered = None;
    for (index, window) in windows.iter().enumerate() {
        let metrics = overload_metrics(window, knee_rate, 48)?;
        elapsed = elapsed
            .checked_add(metrics.elapsed_ms)
            .ok_or_else(|| PerfBudgetError::new("overload recovery duration overflow"))?;
        let passing = metrics.goodput >= baseline_goodput * 0.85
            && metrics.p99 as f64 <= baseline_p99 as f64 * 1.50
            && approx_eq(metrics.error_timeout_ratio, 0.0)
            && approx_eq(metrics.rejection_ratio, 0.0)
            && metrics.drained;
        consecutive = if passing { consecutive + 1 } else { 0 };
        if consecutive == 2 {
            recovered = Some((index + 1) as u64);
            if index + 1 != windows.len() {
                return Err(PerfBudgetError::new(
                    "overload recovery retained trailing windows after confirmation",
                ));
            }
            break;
        }
    }
    let declared_recovered = recovery.get("recovered_at_window").and_then(Value::as_u64);
    let declared_time = recovery.get("time_to_baseline_ms").and_then(Value::as_u64);
    if declared_recovered != recovered
        || recovery
            .get("consecutive_passing_windows")
            .and_then(Value::as_u64)
            != Some(consecutive)
        || recovery.get("observed_recovery_ms").and_then(Value::as_u64) != Some(elapsed)
        || declared_time != recovered.map(|_| elapsed)
        || (recovered.is_none() && windows.len() != 3)
    {
        return Err(PerfBudgetError::new(
            "transition-inclusive consecutive recovery verdict does not recompute",
        ));
    }
    Ok(())
}

fn json_f64_eq(value: &Value, field: &str, expected: f64) -> bool {
    value
        .get(field)
        .and_then(Value::as_f64)
        .is_some_and(|observed| approx_eq(observed, expected))
}

fn normalize_perf_report(
    expected: &ExpectedReport,
    enforcement: Enforcement,
    bytes: &[u8],
) -> Result<CandidateReport, PerfBudgetError> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| PerfBudgetError::new(format!("invalid {} JSON: {error}", expected.id)))?;
    let get = |pointer: &str| {
        value.pointer(pointer).ok_or_else(|| {
            PerfBudgetError::new(format!("{} misses required field {pointer}", expected.id))
        })
    };
    if get("/schema_version")?.as_u64() != Some(1)
        || get("/release")?.as_str() != Some(RELEASE)
        || get("/report_id")?.as_str() != Some(expected.report_id.as_str())
        || get("/surface/claim_scope")?.as_str() != Some(expected.claim_scope.as_str())
        || get("/run_mode")?.as_str() != Some(expected_run_mode(enforcement_run_mode(enforcement)))
        || get("/profile_validation/eligible")?.as_bool() != Some(true)
        || get("/profile_validation/reasons")?
            .as_array()
            .is_none_or(|reasons| !reasons.is_empty())
    {
        return Err(PerfBudgetError::new(format!(
            "{} is not exact eligible reference evidence",
            expected.id
        )));
    }
    let runner_contract: RunnerContract = serde_json::from_value(get("/runner_contract")?.clone())
        .map_err(|error| {
            PerfBudgetError::new(format!(
                "{} runner contract is invalid: {error}",
                expected.id
            ))
        })?;
    let observed_runner: ObservedRunner = serde_json::from_value(get("/observed_runner")?.clone())
        .map_err(|error| {
            PerfBudgetError::new(format!(
                "{} runner observation is invalid: {error}",
                expected.id
            ))
        })?;
    validate_runner_observation(&runner_contract, &observed_runner)?;
    if runner_contract.name != get("/runner_profile")?.as_str().unwrap_or_default()
        || digest_json(&runner_contract)
            != get("/runner_contract_digest")?.as_str().unwrap_or_default()
    {
        return Err(PerfBudgetError::new(format!(
            "{} runner contract digest does not recompute",
            expected.id
        )));
    }
    let measurements = get("/measurements")?.as_array().ok_or_else(|| {
        PerfBudgetError::new(format!("{} measurements are not an array", expected.id))
    })?;
    validate_perf_suite_identity(&value, measurements, &expected.id)?;
    let (metrics, maximum_spread_ratio) = perf_metrics(measurements, &expected.id)?;
    let slo_digest = digest_json(&measurement_projection(measurements, Projection::Slo));
    let methodology_digest = digest_json(&serde_json::json!({
        "run_mode": get("/run_mode")?,
        "surface": get("/surface")?,
        "measurements": measurement_projection(measurements, Projection::Methodology),
    }));
    let binary_sha256 = parse_binary_set(get("/build/binary_sha256")?)?;
    let binary_set_digest = digest_json(&binary_sha256);
    let runner_profile = string_at(&value, "/runner_profile", &expected.id)?;
    let runner_contract_digest = string_at(&value, "/runner_contract_digest", &expected.id)?;
    let runner_fingerprint = string_at(&value, "/observed_runner/fingerprint", &expected.id)?;
    let source_commit = string_at(&value, "/source/git_commit", &expected.id)?;
    let cargo_lock_sha256 = string_at(&value, "/source/cargo_lock_sha256", &expected.id)?;
    let toolchain_identity =
        canonical_toolchain_identity(&string_at(&value, "/source/toolchain", &expected.id)?)?;
    let prebuild_contract_digest =
        string_at(&value, "/build/prebuild_contract_digest", &expected.id)?;
    let prebuild_manifest_sha256 =
        string_at(&value, "/build/prebuild_manifest_sha256", &expected.id)?;
    let scenario_digest = string_at(&value, "/scenario_digest", &expected.id)?;
    let workload_digest = string_at(&value, "/workload_digest", &expected.id)?;
    validate_receipt_fields(
        &runner_profile,
        &runner_contract_digest,
        &runner_fingerprint,
        &source_commit,
        &cargo_lock_sha256,
        &prebuild_contract_digest,
        &prebuild_manifest_sha256,
        &binary_set_digest,
        &scenario_digest,
        &workload_digest,
        &slo_digest,
        &methodology_digest,
        maximum_spread_ratio,
    )?;
    Ok(CandidateReport {
        id: expected.id.clone(),
        path: expected.path.clone(),
        report_id: expected.report_id.clone(),
        report_sha256: sha256(bytes),
        claim_scope: expected.claim_scope.clone(),
        run_mode: enforcement_run_mode(enforcement),
        runner_profile,
        runner_contract_digest,
        runner_class: observed_runner.runner_class,
        runner_fingerprint,
        source_commit,
        cargo_lock_sha256,
        toolchain_identity,
        prebuild_contract_digest,
        prebuild_manifest_sha256,
        binary_sha256,
        binary_set_digest,
        scenario_digest,
        workload_digest,
        slo_digest,
        methodology_digest,
        stable: get("/stable")?.as_bool() == Some(true),
        maximum_spread_ratio,
        metrics,
    })
}

fn validate_perf_suite_identity(
    root: &Value,
    measurements: &[Value],
    report_id: &str,
) -> Result<(), PerfBudgetError> {
    if measurements.is_empty() {
        return Err(PerfBudgetError::new(format!(
            "{report_id} has no typed measurements"
        )));
    }
    let mut ids = BTreeSet::new();
    let mut scenario_inputs = Vec::with_capacity(measurements.len());
    let mut workload_inputs = Vec::with_capacity(measurements.len());
    for measurement in measurements {
        let kind = measurement
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                PerfBudgetError::new(format!("{report_id} has an untyped measurement"))
            })?;
        let evidence = measurement.get("evidence").ok_or_else(|| {
            PerfBudgetError::new(format!("{report_id} has a measurement without evidence"))
        })?;
        let id = evidence
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| {
                PerfBudgetError::new(format!("{report_id} has an unnamed measurement"))
            })?;
        if !ids.insert(id) {
            return Err(PerfBudgetError::new(format!(
                "{report_id} has duplicate measurement {id}"
            )));
        }
        let scenario = evidence
            .get("scenario_digest")
            .and_then(Value::as_str)
            .filter(|digest| is_sha256(digest))
            .ok_or_else(|| {
                PerfBudgetError::new(format!(
                    "{report_id} measurement {id} has no scenario digest"
                ))
            })?;
        let workload = match kind {
            "load_curve" | "scalar" => evidence.pointer("/workload/digest"),
            "trace_replay" => evidence.get("input_digest"),
            "comparison" => evidence.get("scenario_digest"),
            _ => None,
        }
        .and_then(Value::as_str)
        .filter(|digest| is_sha256(digest))
        .ok_or_else(|| {
            PerfBudgetError::new(format!(
                "{report_id} measurement {id} has no workload digest"
            ))
        })?;
        scenario_inputs.push((id.to_owned(), scenario.to_owned()));
        workload_inputs.push((id.to_owned(), workload.to_owned()));
    }
    if root.get("scenario_digest").and_then(Value::as_str)
        != Some(digest_json(&scenario_inputs).as_str())
        || root.get("workload_digest").and_then(Value::as_str)
            != Some(digest_json(&workload_inputs).as_str())
    {
        return Err(PerfBudgetError::new(format!(
            "{report_id} suite scenario/workload digests do not recompute"
        )));
    }
    let stable = root.get("stable").and_then(Value::as_bool).ok_or_else(|| {
        PerfBudgetError::new(format!("{report_id} has no semantic stability verdict"))
    })?;
    let reasons = root
        .get("stability_reasons")
        .and_then(Value::as_array)
        .ok_or_else(|| PerfBudgetError::new(format!("{report_id} has no stability reasons")))?;
    if stable != reasons.is_empty()
        || reasons
            .iter()
            .any(|reason| reason.as_str().is_none_or(str::is_empty))
    {
        return Err(PerfBudgetError::new(format!(
            "{report_id} stored stable flag does not match its reasons"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum Projection {
    Slo,
    Methodology,
}

fn measurement_projection(measurements: &[Value], projection: Projection) -> Vec<Value> {
    measurements
        .iter()
        .map(|measurement| {
            let kind = measurement.get("kind").cloned().unwrap_or(Value::Null);
            let evidence = measurement.get("evidence").unwrap_or(&Value::Null);
            match projection {
                Projection::Slo => serde_json::json!({
                    "id": evidence.get("id"),
                    "kind": kind,
                    "criteria": evidence.get("criteria"),
                }),
                Projection::Methodology => serde_json::json!({
                    "id": evidence.get("id"),
                    "kind": kind,
                    "claim": evidence.get("claim"),
                    "dimensions": evidence.get("dimensions"),
                    "derived_from": evidence.get("derived_from"),
                }),
            }
        })
        .collect()
}

fn perf_metrics(
    measurements: &[Value],
    report_id: &str,
) -> Result<(BTreeMap<String, ReportMetric>, f64), PerfBudgetError> {
    let mut metrics = BTreeMap::new();
    let mut selected_p99 = BTreeMap::new();
    let mut max_spread = 0.0_f64;
    for measurement in measurements {
        let kind = measurement
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| PerfBudgetError::new("measurement kind is absent"))?;
        let evidence = measurement
            .get("evidence")
            .ok_or_else(|| PerfBudgetError::new("measurement evidence is absent"))?;
        let id = evidence
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| PerfBudgetError::new("measurement id is absent"))?;
        match kind {
            "load_curve" => {
                let required_repeats = if report_id == "node-resp-open-loop" {
                    5
                } else {
                    3
                };
                let selected = validate_knee_evidence(id, evidence, Some(required_repeats))?;
                max_spread = max_spread.max(selected.maximum_spread_ratio);
                if let Some(value) = selected.throughput_at_slo {
                    insert_metric(
                        &mut metrics,
                        ReportMetric {
                            id: format!("{id}.throughput_at_slo"),
                            value,
                            unit: "operations_per_second".to_owned(),
                        },
                    )?;
                }
                if let Some(value) = selected.p99_microseconds_at_slo {
                    selected_p99.insert(id.to_owned(), value);
                    insert_metric(
                        &mut metrics,
                        ReportMetric {
                            id: format!("{id}.p99_microseconds_at_slo"),
                            value,
                            unit: "microseconds".to_owned(),
                        },
                    )?;
                }
            }
            "scalar" => {
                let (values, observed_unit, scalar_spread) = validate_scalar_evidence(
                    id,
                    evidence,
                    if report_id == "node-resp-open-loop" {
                        Some(5)
                    } else {
                        Some(3)
                    },
                )?;
                max_spread = max_spread.max(scalar_spread);
                let mut median_values = values.clone();
                let median_value = median(&mut median_values)
                    .ok_or_else(|| PerfBudgetError::new(format!("scalar {id} is empty")))?;
                let min = values.iter().copied().fold(f64::INFINITY, f64::min);
                let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                for (suffix, value) in [("median", median_value), ("min", min), ("max", max)] {
                    insert_metric(
                        &mut metrics,
                        ReportMetric {
                            id: format!("{id}.{suffix}"),
                            value,
                            unit: observed_unit.clone(),
                        },
                    )?;
                }
                if matches!(
                    id,
                    "client_surface_in_process_knee_at_slo_for_a_b_c"
                        | "resp_open_loop_get_set_knee_at_slo"
                ) {
                    insert_metric(
                        &mut metrics,
                        ReportMetric {
                            id: format!("{id}.throughput_at_slo"),
                            value: min,
                            unit: observed_unit.clone(),
                        },
                    )?;
                    let dependencies = evidence
                        .get("derived_from")
                        .and_then(Value::as_array)
                        .ok_or_else(|| {
                            PerfBudgetError::new(format!(
                                "capacity aggregate {id} has no dependencies"
                            ))
                        })?;
                    let dependency_p99 = dependencies
                        .iter()
                        .map(|dependency| {
                            dependency
                                .as_str()
                                .and_then(|dependency| selected_p99.get(dependency))
                                .copied()
                                .ok_or_else(|| {
                                    PerfBudgetError::new(format!(
                                        "capacity aggregate {id} has an unbound knee dependency"
                                    ))
                                })
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    let p99 = dependency_p99
                        .into_iter()
                        .max_by(f64::total_cmp)
                        .ok_or_else(|| {
                            PerfBudgetError::new(format!(
                                "capacity aggregate {id} has no selected p99"
                            ))
                        })?;
                    insert_metric(
                        &mut metrics,
                        ReportMetric {
                            id: format!("{id}.p99_microseconds_at_slo"),
                            value: p99,
                            unit: "microseconds".to_owned(),
                        },
                    )?;
                }
            }
            "comparison" => {
                let value = evidence
                    .get("ratio")
                    .and_then(Value::as_f64)
                    .ok_or_else(|| PerfBudgetError::new(format!("comparison {id} has no ratio")))?;
                let unit = evidence
                    .get("unit")
                    .and_then(Value::as_str)
                    .ok_or_else(|| PerfBudgetError::new(format!("comparison {id} has no unit")))?;
                insert_metric(
                    &mut metrics,
                    ReportMetric {
                        id: format!("{id}.ratio"),
                        value,
                        unit: unit.to_owned(),
                    },
                )?;
            }
            "trace_replay" => {}
            other => {
                return Err(PerfBudgetError::new(format!(
                    "unknown performance measurement kind {other:?}"
                )));
            }
        }
    }
    Ok((metrics, max_spread))
}

#[derive(Debug, Clone, Copy)]
struct SelectedKneeMetric {
    throughput_at_slo: Option<f64>,
    p99_microseconds_at_slo: Option<f64>,
    maximum_spread_ratio: f64,
}

fn validate_knee_evidence(
    id: &str,
    evidence: &Value,
    exact_repeats: Option<usize>,
) -> Result<SelectedKneeMetric, PerfBudgetError> {
    let criteria = evidence
        .get("criteria")
        .and_then(Value::as_object)
        .ok_or_else(|| PerfBudgetError::new(format!("load curve {id} has no criteria")))?;
    let p99_slo = criteria_u64(criteria, "p99_slo_us", id)?;
    let p999_slo = criteria
        .get("p999_slo_us")
        .map(|value| {
            if value.is_null() {
                Ok(None)
            } else {
                value.as_u64().map(Some).ok_or_else(|| {
                    PerfBudgetError::new(format!("load curve {id} has invalid p999 SLO"))
                })
            }
        })
        .transpose()?
        .flatten();
    let min_achieved = criteria_f64(criteria, "min_achieved_ratio", id)?;
    let max_error = criteria_f64(criteria, "max_error_ratio", id)?;
    let max_timeout = criteria_f64(criteria, "max_timeout_ratio", id)?;
    let max_rejection = criteria_f64(criteria, "max_rejection_ratio", id)?;
    let max_drain = criteria_u64(criteria, "max_drain_ms", id)?;
    let max_spread = criteria_f64(criteria, "max_robust_spread_ratio", id)?;
    if p99_slo == 0
        || p999_slo == Some(0)
        || !min_achieved.is_finite()
        || !(0.0..=1.0).contains(&min_achieved)
        || min_achieved == 0.0
        || [max_error, max_timeout, max_rejection]
            .iter()
            .any(|ratio| !ratio.is_finite() || !(0.0..=1.0).contains(ratio))
        || max_drain == 0
        || !max_spread.is_finite()
        || max_spread < 0.0
    {
        return Err(PerfBudgetError::new(format!(
            "load curve {id} has vacuous sustainability criteria"
        )));
    }
    let knee = evidence
        .get("knee")
        .and_then(Value::as_object)
        .ok_or_else(|| PerfBudgetError::new(format!("load curve {id} has no knee")))?;
    let points = knee
        .get("evaluated")
        .and_then(Value::as_array)
        .filter(|points| !points.is_empty())
        .ok_or_else(|| PerfBudgetError::new(format!("load curve {id} has no evaluated rates")))?;
    let mut derived_knee: Option<f64> = None;
    let mut selected_p99 = None;
    let mut prior_rate = None;
    let mut maximum_spread = 0.0_f64;
    for point in points {
        let sample = point
            .get("sample")
            .and_then(Value::as_object)
            .ok_or_else(|| PerfBudgetError::new(format!("load curve {id} has no sample")))?;
        let offered_rate = sample_f64(sample, "offered_rate_per_second", id)?;
        if !offered_rate.is_finite()
            || offered_rate <= 0.0
            || prior_rate.is_some_and(|prior| prior >= offered_rate)
        {
            return Err(PerfBudgetError::new(format!(
                "load curve {id} rates are not positive and strictly increasing"
            )));
        }
        prior_rate = Some(offered_rate);
        let repeats = point
            .get("repeats")
            .and_then(Value::as_array)
            .filter(|repeats| {
                exact_repeats
                    .map(|expected| repeats.len() == expected)
                    .unwrap_or_else(|| repeats.len() >= 3)
            })
            .ok_or_else(|| {
                PerfBudgetError::new(format!(
                    "load curve {id} has the wrong committed raw repeat count"
                ))
            })?;
        let mut achieved = repeats
            .iter()
            .map(|repeat| {
                repeat
                    .pointer("/steady/achieved_rate_per_second")
                    .and_then(Value::as_f64)
                    .filter(|value| value.is_finite() && *value >= 0.0)
                    .ok_or_else(|| {
                        PerfBudgetError::new(format!(
                            "load curve {id} repeat has invalid achieved rate"
                        ))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut repeat_order = (0..repeats.len()).collect::<Vec<_>>();
        repeat_order.sort_by(|left, right| achieved[*left].total_cmp(&achieved[*right]));
        achieved.sort_by(f64::total_cmp);
        let achieved_median = achieved[achieved.len() / 2];
        let achieved_min = achieved[0];
        let achieved_max = achieved[achieved.len() - 1];
        let spread = if achieved_median > 0.0 {
            (achieved_max - achieved_min) / achieved_median
        } else if approx_eq(achieved_max, achieved_min) {
            0.0
        } else {
            f64::INFINITY
        };
        maximum_spread = maximum_spread.max(spread);
        if !approx_eq(
            sample_f64(sample, "achieved_rate_per_second", id)?,
            achieved_median,
        ) || !approx_eq(
            sample_f64(sample, "achieved_rate_min_per_second", id)?,
            achieved_min,
        ) || !approx_eq(
            sample_f64(sample, "achieved_rate_max_per_second", id)?,
            achieved_max,
        ) || !approx_eq(sample_f64(sample, "robust_spread_ratio", id)?, spread)
        {
            return Err(PerfBudgetError::new(format!(
                "load curve {id} aggregate does not recompute from raw repeats"
            )));
        }
        let representative = repeats[repeat_order[repeat_order.len() / 2]]
            .get("steady")
            .and_then(Value::as_object)
            .ok_or_else(|| PerfBudgetError::new(format!("load curve {id} repeat is malformed")))?;
        for field in [
            "offered",
            "started",
            "completed",
            "successes",
            "errors",
            "timeouts",
            "rejections",
            "drain_ms",
        ] {
            if sample.get(field) != representative.get(field) {
                return Err(PerfBudgetError::new(format!(
                    "load curve {id} sample counters are not from the median repeat"
                )));
            }
        }
        if sample.get("backlog_drained") != representative.get("backlog_drained")
            || sample.get("latency") != representative.get("latency")
        {
            return Err(PerfBudgetError::new(format!(
                "load curve {id} latency/backlog is not from the median repeat"
            )));
        }
        let repeats_sustainable = repeats.iter().all(|repeat| {
            repeat_satisfies_criteria(
                repeat,
                offered_rate,
                p99_slo,
                p999_slo,
                min_achieved,
                max_error,
                max_timeout,
                max_rejection,
                max_drain,
            )
        });
        let sustainable = repeats_sustainable && spread.is_finite() && spread <= max_spread;
        let stored_sustainable = point
            .pointer("/verdict/sustainable")
            .and_then(Value::as_bool)
            .ok_or_else(|| PerfBudgetError::new(format!("load curve {id} has no verdict")))?;
        let reasons = point
            .pointer("/verdict/reasons")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                PerfBudgetError::new(format!("load curve {id} has no verdict reasons"))
            })?;
        if stored_sustainable != sustainable || sustainable != reasons.is_empty() {
            return Err(PerfBudgetError::new(format!(
                "load curve {id} verdict does not recompute from raw repeats"
            )));
        }
        if sustainable {
            derived_knee = Some(offered_rate);
            selected_p99 = sample
                .get("latency")
                .and_then(|latency| latency.get("p99_us"))
                .and_then(Value::as_u64)
                .map(|value| value as f64);
        }
    }
    let declared_knee = knee
        .get("sustainable_rate_per_second")
        .and_then(Value::as_f64);
    if declared_knee != derived_knee {
        return Err(PerfBudgetError::new(format!(
            "load curve {id} selected knee does not match evaluated rates"
        )));
    }
    if evidence.get("claim").and_then(Value::as_str) == Some("capacity_knee")
        && derived_knee.is_none()
    {
        return Err(PerfBudgetError::new(format!(
            "capacity load curve {id} has no sustainable rate"
        )));
    }
    Ok(SelectedKneeMetric {
        throughput_at_slo: derived_knee,
        p99_microseconds_at_slo: selected_p99,
        maximum_spread_ratio: maximum_spread,
    })
}

#[allow(clippy::too_many_arguments)]
fn repeat_satisfies_criteria(
    repeat: &Value,
    offered_rate: f64,
    p99_slo: u64,
    p999_slo: Option<u64>,
    min_achieved: f64,
    max_error: f64,
    max_timeout: f64,
    max_rejection: f64,
    max_drain: u64,
) -> bool {
    let Some(steady) = repeat.get("steady").and_then(Value::as_object) else {
        return false;
    };
    let Some(phase) = repeat.get("phase").and_then(Value::as_object) else {
        return false;
    };
    let value_u64 = |object: &serde_json::Map<String, Value>, field: &str| {
        object.get(field).and_then(Value::as_u64)
    };
    let offered = value_u64(steady, "offered").unwrap_or(0);
    let started = value_u64(steady, "started").unwrap_or(0);
    let completed = value_u64(steady, "completed").unwrap_or(0);
    let successes = value_u64(steady, "successes").unwrap_or(u64::MAX);
    let errors = value_u64(steady, "errors").unwrap_or(u64::MAX);
    let timeouts = value_u64(steady, "timeouts").unwrap_or(u64::MAX);
    let rejections = value_u64(steady, "rejections").unwrap_or(u64::MAX);
    let achieved = steady
        .get("achieved_rate_per_second")
        .and_then(Value::as_f64)
        .unwrap_or(f64::NAN);
    let latency = steady.get("latency").and_then(Value::as_object);
    let latency_valid = latency.is_some_and(|latency| {
        let samples = value_u64(latency, "samples").unwrap_or(0);
        let p50 = value_u64(latency, "p50_us");
        let p90 = value_u64(latency, "p90_us");
        let p99 = value_u64(latency, "p99_us");
        let p999 = value_u64(latency, "p999_us");
        let maximum = value_u64(latency, "max_us");
        let p999_floor = value_u64(latency, "p999_min_samples").unwrap_or(0);
        let p999_reportable = latency
            .get("p999_reportable")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let ordered = p50
            .zip(p90)
            .zip(p99)
            .is_some_and(|((p50, p90), p99)| p50 <= p90 && p90 <= p99);
        let tails = p99.zip(maximum).is_some_and(|(p99, maximum)| {
            p99 <= maximum && p999.is_none_or(|p999| p99 <= p999 && p999 <= maximum)
        });
        samples == completed
            && p999_floor > 0
            && p999_reportable == (samples >= p999_floor)
            && p999.is_some() == p999_reportable
            && value_u64(latency, "overflow_count") == Some(0)
            && ordered
            && tails
            && p99.is_some_and(|p99| p99 <= p99_slo)
            && p999_slo.is_none_or(|slo| p999_reportable && p999.is_some_and(|p999| p999 <= slo))
    });
    let outcomes = successes
        .checked_add(errors)
        .and_then(|value| value.checked_add(timeouts))
        .and_then(|value| value.checked_add(rejections));
    let denominator = started.max(1) as f64;
    let phase_valid = value_u64(phase, "reset_operations") == Some(1)
        && value_u64(phase, "steady_operations") == Some(offered)
        && value_u64(phase, "warmup_successes") == value_u64(phase, "warmup_operations")
        && value_u64(phase, "warmup_errors") == Some(0)
        && value_u64(phase, "warmup_timeouts") == Some(0)
        && value_u64(phase, "warmup_rejections") == Some(0)
        && value_u64(phase, "warmup_samples_in_steady_histogram") == Some(0);
    offered > 0
        && started == offered
        && completed == started
        && outcomes == Some(completed)
        && steady
            .get("offered_rate_per_second")
            .and_then(Value::as_f64)
            .is_some_and(|value| approx_eq(value, offered_rate))
        && achieved.is_finite()
        && achieved / offered_rate >= min_achieved
        && errors as f64 / denominator <= max_error
        && timeouts as f64 / denominator <= max_timeout
        && rejections as f64 / denominator <= max_rejection
        && steady.get("backlog_drained").and_then(Value::as_bool) == Some(true)
        && value_u64(steady, "drain_ms").is_some_and(|value| value <= max_drain)
        && latency_valid
        && phase_valid
        && repeat
            .get("reset_state_digest")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.is_empty())
        && repeat
            .get("state_digest")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.is_empty())
}

fn validate_scalar_evidence(
    id: &str,
    evidence: &Value,
    exact_samples: Option<usize>,
) -> Result<(Vec<f64>, String, f64), PerfBudgetError> {
    let points = evidence
        .get("points")
        .and_then(Value::as_array)
        .filter(|points| !points.is_empty())
        .ok_or_else(|| PerfBudgetError::new(format!("scalar {id} has no points")))?;
    let declared_spread = evidence
        .get("max_robust_spread_ratio")
        .and_then(Value::as_f64)
        .filter(|spread| spread.is_finite() && *spread >= 0.0)
        .ok_or_else(|| PerfBudgetError::new(format!("scalar {id} has invalid spread contract")))?;
    let mut values = Vec::with_capacity(points.len());
    let mut unit = None;
    let mut maximum_spread = 0.0_f64;
    for point in points {
        let samples = point
            .get("samples")
            .and_then(Value::as_array)
            .filter(|samples| {
                exact_samples
                    .map(|expected| samples.len() == expected)
                    .unwrap_or_else(|| samples.len() >= 3)
            })
            .ok_or_else(|| PerfBudgetError::new(format!("scalar {id} has too few samples")))?
            .iter()
            .map(|sample| {
                sample
                    .as_f64()
                    .filter(|value| value.is_finite())
                    .ok_or_else(|| {
                        PerfBudgetError::new(format!("scalar {id} has a non-finite sample"))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if point.get("sample_count").and_then(Value::as_u64) != Some(samples.len() as u64) {
            return Err(PerfBudgetError::new(format!(
                "scalar {id} sample count does not recompute"
            )));
        }
        let mut ordered = samples.clone();
        ordered.sort_by(f64::total_cmp);
        let expected_value = ordered[ordered.len() / 2];
        let expected_min = ordered[0];
        let expected_max = ordered[ordered.len() - 1];
        let expected_spread = if expected_value > 0.0 {
            (expected_max - expected_min) / expected_value
        } else if approx_eq(expected_max, expected_min) {
            0.0
        } else {
            f64::INFINITY
        };
        let value = point
            .pointer("/quantity/value")
            .and_then(Value::as_f64)
            .ok_or_else(|| PerfBudgetError::new(format!("scalar {id} has invalid value")))?;
        let observed_unit = point
            .pointer("/quantity/unit")
            .and_then(Value::as_str)
            .filter(|unit| !unit.is_empty())
            .ok_or_else(|| PerfBudgetError::new(format!("scalar {id} has invalid unit")))?;
        if unit
            .replace(observed_unit)
            .is_some_and(|prior| prior != observed_unit)
            || !approx_eq(value, expected_value)
            || !approx_eq(
                point.get("min").and_then(Value::as_f64).unwrap_or(f64::NAN),
                expected_min,
            )
            || !approx_eq(
                point.get("max").and_then(Value::as_f64).unwrap_or(f64::NAN),
                expected_max,
            )
            || !approx_eq(
                point
                    .get("robust_spread_ratio")
                    .and_then(Value::as_f64)
                    .unwrap_or(f64::NAN),
                expected_spread,
            )
            || expected_spread > declared_spread
        {
            return Err(PerfBudgetError::new(format!(
                "scalar {id} aggregate/spread does not recompute from samples"
            )));
        }
        maximum_spread = maximum_spread.max(expected_spread);
        values.push(value);
    }
    Ok((values, unit.unwrap_or("unknown").to_owned(), maximum_spread))
}

fn criteria_u64(
    criteria: &serde_json::Map<String, Value>,
    field: &str,
    id: &str,
) -> Result<u64, PerfBudgetError> {
    criteria.get(field).and_then(Value::as_u64).ok_or_else(|| {
        PerfBudgetError::new(format!(
            "load curve {id} has invalid criteria field {field}"
        ))
    })
}

fn criteria_f64(
    criteria: &serde_json::Map<String, Value>,
    field: &str,
    id: &str,
) -> Result<f64, PerfBudgetError> {
    criteria.get(field).and_then(Value::as_f64).ok_or_else(|| {
        PerfBudgetError::new(format!(
            "load curve {id} has invalid criteria field {field}"
        ))
    })
}

fn sample_f64(
    sample: &serde_json::Map<String, Value>,
    field: &str,
    id: &str,
) -> Result<f64, PerfBudgetError> {
    sample.get(field).and_then(Value::as_f64).ok_or_else(|| {
        PerfBudgetError::new(format!("load curve {id} has invalid sample field {field}"))
    })
}

fn metric_map(
    metrics: Vec<ReportMetric>,
) -> Result<BTreeMap<String, ReportMetric>, PerfBudgetError> {
    let mut result = BTreeMap::new();
    for metric in metrics {
        insert_metric(&mut result, metric)?;
    }
    Ok(result)
}

fn parse_binary_set(value: &Value) -> Result<Vec<BinaryDigest>, PerfBudgetError> {
    let rows = value
        .as_array()
        .ok_or_else(|| PerfBudgetError::new("report binary_sha256 is not an array"))?;
    let mut binaries = Vec::with_capacity(rows.len());
    for row in rows {
        let tuple = row
            .as_array()
            .filter(|tuple| tuple.len() == 2)
            .ok_or_else(|| {
                PerfBudgetError::new("report binary_sha256 entries must be [id, sha256] tuples")
            })?;
        let id = tuple[0]
            .as_str()
            .ok_or_else(|| PerfBudgetError::new("report binary id is not a string"))?;
        let sha256 = tuple[1]
            .as_str()
            .ok_or_else(|| PerfBudgetError::new("report binary digest is not a string"))?;
        binaries.push(BinaryDigest {
            id: id.to_owned(),
            sha256: sha256.to_owned(),
        });
    }
    binaries.sort_by(|left, right| left.id.cmp(&right.id));
    validate_w7_binary_set(&binaries, &digest_json(&binaries))?;
    Ok(binaries)
}

fn validate_binary_set(
    binaries: &[BinaryDigest],
    expected_digest: &str,
) -> Result<(), PerfBudgetError> {
    let mut ids = BTreeSet::new();
    let canonical = binaries.windows(2).all(|pair| pair[0].id < pair[1].id);
    if binaries.is_empty()
        || !canonical
        || binaries.iter().any(|binary| {
            binary.id.trim().is_empty()
                || !is_sha256(&binary.sha256)
                || !ids.insert(binary.id.as_str())
        })
        || !is_sha256(expected_digest)
        || digest_json(binaries) != expected_digest
    {
        return Err(PerfBudgetError::new(
            "binary receipt must be a non-empty, sorted, unique, digest-bound set",
        ));
    }
    Ok(())
}

fn validate_w7_binary_set(
    binaries: &[BinaryDigest],
    expected_digest: &str,
) -> Result<(), PerfBudgetError> {
    validate_binary_set(binaries, expected_digest)?;
    let ids = binaries
        .iter()
        .map(|binary| binary.id.as_str())
        .collect::<BTreeSet<_>>();
    if ids != BTreeSet::from(["hydracache-loadgen", "hydracache-server"]) {
        return Err(PerfBudgetError::new(
            "W7 report does not bind the exact release binary set",
        ));
    }
    Ok(())
}

fn insert_metric(
    metrics: &mut BTreeMap<String, ReportMetric>,
    metric: ReportMetric,
) -> Result<(), PerfBudgetError> {
    if metric.id.trim().is_empty()
        || metric.unit.trim().is_empty()
        || !metric.value.is_finite()
        || metrics.insert(metric.id.clone(), metric).is_some()
    {
        return Err(PerfBudgetError::new(
            "report metrics must be unique, finite, and fully identified",
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_receipt_fields(
    runner_profile: &str,
    runner_contract_digest: &str,
    runner_fingerprint: &str,
    source_commit: &str,
    cargo_lock_sha256: &str,
    prebuild_contract_digest: &str,
    prebuild_manifest_sha256: &str,
    binary_set_digest: &str,
    scenario_digest: &str,
    workload_digest: &str,
    slo_digest: &str,
    methodology_digest: &str,
    maximum_spread_ratio: f64,
) -> Result<(), PerfBudgetError> {
    if runner_profile.trim().is_empty()
        || runner_fingerprint.trim().is_empty()
        || !is_git_commit(source_commit)
        || ![
            runner_contract_digest,
            cargo_lock_sha256,
            prebuild_contract_digest,
            prebuild_manifest_sha256,
            binary_set_digest,
            scenario_digest,
            workload_digest,
            slo_digest,
            methodology_digest,
        ]
        .iter()
        .all(|digest| is_sha256(digest))
        || !finite_nonnegative_ratio(maximum_spread_ratio)
    {
        return Err(PerfBudgetError::new(
            "report source/profile/fingerprint/SLO/methodology/spread/prebuild identity is incomplete",
        ));
    }
    Ok(())
}

fn string_at(value: &Value, pointer: &str, report: &str) -> Result<String, PerfBudgetError> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| PerfBudgetError::new(format!("{report} misses string field {pointer}")))
}

fn validate_runner_observation(
    contract: &RunnerContract,
    observed: &ObservedRunner,
) -> Result<(), PerfBudgetError> {
    let contract_problems = contract.contract_problems();
    let validation = contract.validate(observed);
    if !contract_problems.is_empty() || !validation.eligible {
        return Err(PerfBudgetError::new(
            "runner profile/fingerprint/affinity/quota/dedication/calibration does not revalidate",
        ));
    }
    Ok(())
}

fn canonical_toolchain_identity(raw: &str) -> Result<String, PerfBudgetError> {
    let candidate = if let Some(version) = raw.strip_prefix("rustc-") {
        version
    } else {
        let mut fields = raw.lines().next().unwrap_or_default().split_whitespace();
        if fields.next() != Some("rustc") {
            return Err(PerfBudgetError::new(
                "toolchain identity is not an exact rustc version",
            ));
        }
        fields.next().unwrap_or_default()
    };
    if candidate.is_empty()
        || candidate
            .bytes()
            .any(|byte| !byte.is_ascii_digit() && byte != b'.')
        || candidate.split('.').count() != 3
    {
        return Err(PerfBudgetError::new(
            "toolchain identity is not an exact rustc semver",
        ));
    }
    Ok(format!("rustc-{candidate}"))
}

fn expected_run_mode(mode: EvidenceRunMode) -> &'static str {
    match mode {
        EvidenceRunMode::ReferenceEvidence => "reference_evidence",
        EvidenceRunMode::CiTripwire => "ci_tripwire",
        EvidenceRunMode::Smoke => "smoke",
    }
}

fn enforcement_run_mode(enforcement: Enforcement) -> EvidenceRunMode {
    match enforcement {
        Enforcement::Ship => EvidenceRunMode::ReferenceEvidence,
        Enforcement::NonEnforcingTripwire => EvidenceRunMode::CiTripwire,
    }
}

pub fn evaluate(
    bundle: &ContractBundle,
    reports: &[CandidateReport],
    now: OffsetDateTime,
) -> BudgetVerdict {
    let mut problems = validate_contract_bundle(bundle);
    problems.extend(validate_candidate_set(bundle, reports));
    if bundle.profile.bootstrap_status == BootstrapStatus::Unbootstrapped {
        problems.push(
            "reference performance anchor/window is explicitly unbootstrapped; real main receipts are required"
                .to_owned(),
        );
    }
    let candidate_commit = common_candidate(reports, |report| &report.source_commit)
        .unwrap_or_default()
        .to_owned();
    if bundle.profile.enforcement == Enforcement::Ship
        && bundle.baseline.anchor.contract_commit == candidate_commit
        && !candidate_commit.is_empty()
    {
        problems
            .push("candidate cannot be the immutable release-anchor contract commit".to_owned());
    }
    let eligible = select_eligible_members(
        &bundle.baseline.candidate_members,
        &bundle.baseline,
        reports,
        now,
        &candidate_commit,
    );
    if bundle.baseline.bootstrap_status == BootstrapStatus::Bootstrapped {
        let selected_receipts = bundle
            .baseline
            .members
            .iter()
            .map(|member| member.receipt_sha256.as_str())
            .collect::<Vec<_>>();
        let eligible_receipts = eligible
            .iter()
            .map(|member| member.receipt_sha256.as_str())
            .collect::<Vec<_>>();
        if eligible_receipts != selected_receipts {
            problems.push(
                "rolling manifest contains mixed/stale/unstable/ineligible members or is not the newest eligible window from its audited pool".to_owned(),
            );
        }
        if eligible.len() < bundle.baseline.policy.minimum_members {
            problems.push("rolling baseline has fewer than five eligible main members".to_owned());
        }
    }
    let mut checks = Vec::new();
    if problems.is_empty() {
        evaluate_budgets(bundle, reports, &mut checks, &mut problems);
    }
    problems.sort();
    problems.dedup();
    let status = if problems.is_empty() {
        match bundle.profile.enforcement {
            Enforcement::Ship => VerdictStatus::Passed,
            Enforcement::NonEnforcingTripwire => VerdictStatus::TripwirePassed,
        }
    } else {
        match bundle.profile.enforcement {
            Enforcement::Ship => VerdictStatus::Failed,
            Enforcement::NonEnforcingTripwire => VerdictStatus::TripwireUnavailable,
        }
    };
    let report_inputs = reports
        .iter()
        .map(|report| VerdictReportInput {
            id: report.id.clone(),
            path: report.path.clone(),
            sha256: report.report_sha256.clone(),
        })
        .collect::<Vec<_>>();
    let eligible_ids = eligible
        .iter()
        .map(|member| member.run_id.as_str())
        .collect::<BTreeSet<_>>();
    let baseline_inputs = bundle
        .baseline
        .candidate_members
        .iter()
        .map(|member| VerdictBaselineInput {
            run_id: member.run_id.clone(),
            source_commit: member.source_commit.clone(),
            receipt_sha256: member.receipt_sha256.clone(),
            eligible: eligible_ids.contains(member.run_id.as_str()),
        })
        .collect::<Vec<_>>();
    let report_set_digest = digest_json(&report_inputs);
    BudgetVerdict::new(BudgetVerdictPayload {
        schema_version: 1,
        release: RELEASE.to_owned(),
        profile: bundle.profile.name.clone(),
        enforcement: bundle.profile.enforcement,
        candidate_commit,
        status,
        profile_sha256: bundle.profile_sha256.clone(),
        budget_sha256: bundle.budget_sha256.clone(),
        baseline_sha256: bundle.baseline_sha256.clone(),
        report_set_digest,
        reports: report_inputs,
        baseline_members: baseline_inputs,
        checks,
        problems,
    })
}

fn validate_candidate_set(bundle: &ContractBundle, reports: &[CandidateReport]) -> Vec<String> {
    let mut problems = Vec::new();
    let expected = bundle
        .budget
        .reports
        .iter()
        .map(|report| (report.id.as_str(), report.path.as_str()))
        .collect::<BTreeSet<_>>();
    let observed = reports
        .iter()
        .map(|report| (report.id.as_str(), report.path.as_str()))
        .collect::<BTreeSet<_>>();
    if expected != observed || observed.len() != reports.len() {
        problems.push("candidate report set is missing, extra, or duplicated".to_owned());
    }
    let runner_digest = digest_json(&bundle.profile.runner);
    let expected_binary_ids = bundle
        .profile
        .prebuild
        .target_set
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for report in reports {
        let expected_report = bundle
            .budget
            .reports
            .iter()
            .find(|expected| expected.id == report.id);
        if expected_report.is_none_or(|expected| {
            expected.report_id != report.report_id
                || expected.claim_scope != report.claim_scope
                || enforcement_run_mode(bundle.profile.enforcement) != report.run_mode
        }) {
            problems.push(format!(
                "candidate report {} has a forged surface identity",
                report.id
            ));
        }
        let expected_metrics = bundle
            .budget
            .budgets
            .iter()
            .filter(|rule| rule.report == report.id)
            .map(|rule| rule.metric.as_str())
            .collect::<BTreeSet<_>>();
        let observed_metrics = report
            .metrics
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if expected_metrics != observed_metrics || observed_metrics.len() != report.metrics.len() {
            problems.push(format!(
                "candidate report {} does not expose the exact reviewed budget metric set",
                report.id
            ));
        }
        if report.runner_profile != bundle.profile.name
            || report.runner_contract_digest != runner_digest
            || report.runner_class != bundle.profile.runner.required_runner_class
            || !bundle
                .profile
                .runner
                .allowed_fingerprints
                .iter()
                .any(|fingerprint| fingerprint == &report.runner_fingerprint)
            || report.toolchain_identity != bundle.profile.prebuild.toolchain_identity
            || report.prebuild_contract_digest != bundle.profile.prebuild.digest
        {
            problems.push(format!(
                "candidate report {} mixes profile/fingerprint/prebuild identity",
                report.id
            ));
        }
        if !is_sha256(&report.report_sha256)
            || validate_binary_set(&report.binary_sha256, &report.binary_set_digest).is_err()
            || report
                .binary_sha256
                .iter()
                .map(|binary| binary.id.as_str())
                .collect::<BTreeSet<_>>()
                != expected_binary_ids
            || validate_receipt_fields(
                &report.runner_profile,
                &report.runner_contract_digest,
                &report.runner_fingerprint,
                &report.source_commit,
                &report.cargo_lock_sha256,
                &report.prebuild_contract_digest,
                &report.prebuild_manifest_sha256,
                &report.binary_set_digest,
                &report.scenario_digest,
                &report.workload_digest,
                &report.slo_digest,
                &report.methodology_digest,
                report.maximum_spread_ratio,
            )
            .is_err()
        {
            problems.push(format!(
                "candidate report {} has incomplete receipt digests",
                report.id
            ));
        }
        if !report.stable {
            problems.push(format!("candidate report {} is unstable", report.id));
        }
    }
    if reports.first().is_some_and(|first| {
        reports.iter().skip(1).any(|report| {
            report.binary_sha256 != first.binary_sha256
                || report.binary_set_digest != first.binary_set_digest
        })
    }) {
        problems.push("candidate report set mixes actual prebuilt binary receipts".to_owned());
    }
    for label in [
        common_candidate(reports, |report| &report.source_commit),
        common_candidate(reports, |report| &report.cargo_lock_sha256),
        common_candidate(reports, |report| &report.runner_contract_digest),
        common_candidate(reports, |report| &report.runner_class),
        common_candidate(reports, |report| &report.runner_fingerprint),
        common_candidate(reports, |report| &report.toolchain_identity),
        common_candidate(reports, |report| &report.prebuild_contract_digest),
        common_candidate(reports, |report| &report.prebuild_manifest_sha256),
    ] {
        if label.is_none() {
            problems.push(
                "candidate report set mixes commit/profile/fingerprint/prebuild receipts"
                    .to_owned(),
            );
            break;
        }
    }
    problems
}

fn common_candidate<'a>(
    reports: &'a [CandidateReport],
    field: impl Fn(&'a CandidateReport) -> &'a String,
) -> Option<&'a str> {
    let first = reports.first().map(&field)?;
    reports
        .iter()
        .all(|report| field(report) == first)
        .then_some(first.as_str())
}

pub fn eligible_members<'a>(
    manifest: &'a RollingBaselineManifest,
    reports: &[CandidateReport],
    now: OffsetDateTime,
    candidate_commit: &str,
) -> Vec<&'a BaselineMember> {
    let candidate_by_id = reports
        .iter()
        .map(|report| (report.id.as_str(), report))
        .collect::<BTreeMap<_, _>>();
    let common_fingerprint = common_candidate(reports, |report| &report.runner_fingerprint);
    let common_runner_class = common_candidate(reports, |report| &report.runner_class);
    let common_toolchain = common_candidate(reports, |report| &report.toolchain_identity);
    let common_prebuild = common_candidate(reports, |report| &report.prebuild_contract_digest);
    let cutoff = now - time::Duration::days(manifest.policy.maximum_age_days);
    let mut eligible = manifest
        .members
        .iter()
        .filter_map(|member| {
            let observed = parse_time(&member.observed_at).ok()?;
            let exact_reports = member.reports.len() == candidate_by_id.len()
                && member.reports.iter().all(|baseline| {
                    candidate_by_id
                        .get(baseline.report_id.as_str())
                        .is_some_and(|candidate| {
                            baseline.scenario_digest == candidate.scenario_digest
                                && baseline.workload_digest == candidate.workload_digest
                                && baseline.slo_digest == candidate.slo_digest
                                && baseline.methodology_digest == candidate.methodology_digest
                                && valid_baseline_report(baseline)
                                && baseline.receipt_sha256 == baseline_report_receipt(baseline)
                        })
                });
            let same_runner = if manifest.profile == "reference-v1" {
                common_fingerprint == Some(member.runner_fingerprint.as_str())
            } else {
                common_runner_class == Some(member.observed_runner.runner_class.as_str())
            };
            (member.branch == manifest.policy.branch
                && member.successful
                && member.gate_exit_code == 0
                && !member.quarantined
                && member.quarantine_reason.is_none()
                && member.calibration_passed
                && member.spread_stable
                && member.git_status_porcelain_sha256 == CLEAN_GIT_STATUS_SHA256
                && member.source_commit != candidate_commit
                && is_git_commit(&member.source_commit)
                && same_runner
                && common_toolchain == Some(member.toolchain_identity.as_str())
                && common_prebuild == Some(member.prebuild_contract_digest.as_str())
                && member.profile_sha256 == manifest.profile_sha256
                && member.budget_sha256 == manifest.budget_sha256
                && observed >= cutoff
                && observed <= now
                && exact_reports
                && member.receipt_sha256 == baseline_member_receipt(member))
            .then_some((observed, member))
        })
        .collect::<Vec<_>>();
    eligible.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.run_id.cmp(&right.1.run_id))
    });
    eligible
        .into_iter()
        .take(manifest.policy.maximum_members)
        .map(|(_, member)| member)
        .collect()
}

fn evaluate_budgets(
    bundle: &ContractBundle,
    reports: &[CandidateReport],
    checks: &mut Vec<BudgetCheckRecord>,
    problems: &mut Vec<String>,
) {
    let reports = reports
        .iter()
        .map(|report| (report.id.as_str(), report))
        .collect::<BTreeMap<_, _>>();
    let anchors = bundle
        .baseline
        .anchor
        .metrics
        .iter()
        .map(|metric| (metric.budget_id.as_str(), metric))
        .collect::<BTreeMap<_, _>>();
    let rolling = bundle
        .baseline
        .rolling_metrics
        .iter()
        .map(|metric| (metric.budget_id.as_str(), metric))
        .collect::<BTreeMap<_, _>>();
    for rule in &bundle.budget.budgets {
        let Some(report) = reports.get(rule.report.as_str()) else {
            continue;
        };
        let Some(candidate) = report.metrics.get(&rule.metric) else {
            problems.push(format!("budget {} metric is absent", rule.id));
            continue;
        };
        let Some(rolling) = rolling.get(rule.id.as_str()) else {
            problems.push(format!("budget {} rolling baseline is absent", rule.id));
            continue;
        };
        let anchor = match bundle.profile.enforcement {
            Enforcement::Ship => match anchors.get(rule.id.as_str()) {
                Some(anchor) => Some(*anchor),
                None => {
                    problems.push(format!("budget {} release anchor is absent", rule.id));
                    continue;
                }
            },
            Enforcement::NonEnforcingTripwire => None,
        };
        if candidate.unit != rule.unit
            || rolling.unit != rule.unit
            || anchor.is_some_and(|anchor| anchor.unit != rule.unit)
        {
            problems.push(format!("budget {} mixes metric units", rule.id));
            continue;
        }
        if !candidate.value.is_finite() || candidate.value <= 0.0 {
            problems.push(format!(
                "budget {} candidate metric is not positive and finite",
                rule.id
            ));
            continue;
        }
        let rolling_tolerance = rule.rolling_tolerance_ratio.unwrap_or(0.0);
        let spread_limit = rule.maximum_spread_ratio.unwrap_or(0.0);
        let anchor_pass = anchor.is_none_or(|anchor| {
            threshold_pass(
                rule.direction,
                candidate.value,
                anchor.value,
                rule.anchor_tolerance_ratio.unwrap_or(0.0),
            )
        });
        let rolling_pass = threshold_pass(
            rule.direction,
            candidate.value,
            rolling.median,
            rolling_tolerance,
        );
        let spread_pass = report.maximum_spread_ratio <= spread_limit;
        let passed = anchor_pass && rolling_pass && spread_pass;
        if !passed {
            let boundary = match bundle.profile.enforcement {
                Enforcement::Ship => "release anchor, rolling baseline, or spread ceiling",
                Enforcement::NonEnforcingTripwire => "rolling baseline or spread ceiling",
            };
            problems.push(format!("budget {} breached {boundary}", rule.id));
        }
        checks.push(BudgetCheckRecord {
            budget_id: rule.id.clone(),
            candidate: candidate.value,
            anchor: anchor.map(|anchor| anchor.value),
            rolling_median: rolling.median,
            unit: rule.unit.clone(),
            passed,
        });
    }
}

fn threshold_pass(
    direction: BudgetDirection,
    candidate: f64,
    baseline: f64,
    tolerance: f64,
) -> bool {
    match direction {
        BudgetDirection::Floor => candidate >= baseline * (1.0 - tolerance),
        BudgetDirection::Ceiling => candidate <= baseline * (1.0 + tolerance),
    }
}

pub fn select_eligible_members<'a>(
    pool: &'a [BaselineMember],
    template: &RollingBaselineManifest,
    reports: &[CandidateReport],
    now: OffsetDateTime,
    candidate_commit: &str,
) -> Vec<&'a BaselineMember> {
    let mut manifest = template.clone();
    manifest.members = pool.to_vec();
    // The returned references must point at `pool`, not the cloned manifest.
    let selected_ids = eligible_members(&manifest, reports, now, candidate_commit)
        .into_iter()
        .map(|member| member.run_id.clone())
        .collect::<Vec<_>>();
    selected_ids
        .iter()
        .filter_map(|id| pool.iter().find(|member| member.run_id == *id))
        .collect()
}

pub fn baseline_report_receipt(report: &BaselineReportReceipt) -> String {
    let mut payload = report.clone();
    payload.receipt_sha256.clear();
    digest_json(&payload)
}

pub fn baseline_member_receipt(member: &BaselineMember) -> String {
    let mut payload = member.clone();
    payload.receipt_sha256.clear();
    digest_json(&payload)
}

pub fn baseline_manifest_receipt(manifest: &RollingBaselineManifest) -> String {
    let mut payload = manifest.clone();
    payload.receipt_sha256.clear();
    digest_json(&payload)
}

pub fn baseline_payload_digest(manifest: &RollingBaselineManifest) -> String {
    let mut payload = manifest.clone();
    payload.change_control = BaselineChangeControl {
        status: ChangeControlStatus::PendingBootstrap,
        proposal: None,
        approval: None,
    };
    payload.receipt_sha256.clear();
    digest_json(&payload)
}

pub fn seal_baseline_report(report: &mut BaselineReportReceipt) {
    report.receipt_sha256 = baseline_report_receipt(report);
}

pub fn seal_baseline_member(member: &mut BaselineMember) {
    for report in &mut member.reports {
        seal_baseline_report(report);
    }
    member.receipt_sha256 = baseline_member_receipt(member);
}

pub fn seal_baseline_manifest(manifest: &mut RollingBaselineManifest) {
    for member in &mut manifest.anchor.source_members {
        seal_baseline_member(member);
    }
    for member in &mut manifest.candidate_members {
        seal_baseline_member(member);
    }
    for member in &mut manifest.members {
        seal_baseline_member(member);
    }
    manifest.receipt_sha256 = baseline_manifest_receipt(manifest);
}

pub fn rolling_summaries(
    budgets: &[BudgetRule],
    members: &[BaselineMember],
) -> Result<Vec<RollingMetric>, PerfBudgetError> {
    budgets
        .iter()
        .map(|rule| {
            let mut values = members
                .iter()
                .map(|member| {
                    member
                        .metrics
                        .iter()
                        .find(|metric| metric.budget_id == rule.id)
                        .filter(|metric| metric.unit == rule.unit && metric.value.is_finite())
                        .map(|metric| metric.value)
                        .ok_or_else(|| {
                            PerfBudgetError::new(format!(
                                "baseline member misses metric {}",
                                rule.id
                            ))
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let median_value = median(&mut values)
                .ok_or_else(|| PerfBudgetError::new("rolling window is empty"))?;
            let mut deviations = values
                .iter()
                .map(|value| (value - median_value).abs())
                .collect::<Vec<_>>();
            let mad = median(&mut deviations)
                .ok_or_else(|| PerfBudgetError::new("rolling deviation window is empty"))?;
            Ok(RollingMetric {
                budget_id: rule.id.clone(),
                median: median_value,
                mad,
                tolerance_ratio: rule.rolling_tolerance_ratio.ok_or_else(|| {
                    PerfBudgetError::new(format!("active budget {} has no tolerance", rule.id))
                })?,
                unit: rule.unit.clone(),
            })
        })
        .collect()
}

pub fn write_verdict(root: &Path, verdict: &BudgetVerdict) -> Result<(), PerfBudgetError> {
    let path = root.join(VERDICT_PATH);
    let parent = path
        .parent()
        .ok_or_else(|| PerfBudgetError::new("verdict path has no parent"))?;
    fs::create_dir_all(parent)
        .map_err(|error| PerfBudgetError::new(format!("creating {}: {error}", parent.display())))?;
    if path.exists() {
        return Err(PerfBudgetError::new(format!(
            "refusing to overwrite stale verdict {}",
            path.display()
        )));
    }
    let temp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(verdict)
        .map_err(|error| PerfBudgetError::new(format!("serializing verdict: {error}")))?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp)
        .map_err(|error| {
            PerfBudgetError::new(format!(
                "refusing to overwrite stale verdict temp {}: {error}",
                temp.display()
            ))
        })?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| PerfBudgetError::new(format!("writing {}: {error}", temp.display())))?;
    fs::hard_link(&temp, &path).map_err(|error| {
        PerfBudgetError::new(format!(
            "atomically creating verdict {} without overwrite: {error}",
            path.display()
        ))
    })?;
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .and_then(|file| file.sync_all())
        .map_err(|error| PerfBudgetError::new(format!("syncing {}: {error}", path.display())))?;
    fs::remove_file(&temp)
        .map_err(|error| PerfBudgetError::new(format!("removing {}: {error}", temp.display())))?;
    Ok(())
}

fn safe_target_report_path(path: &str) -> bool {
    let path = Path::new(path);
    path.extension().and_then(|value| value.to_str()) == Some("json")
        && path.starts_with("target/test-evidence/0.67")
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn parse_time(value: &str) -> Result<OffsetDateTime, time::error::Parse> {
    OffsetDateTime::parse(value, &Rfc3339)
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    Some(if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    })
}

fn finite_ratio(value: f64) -> bool {
    value.is_finite() && (0.0..1.0).contains(&value)
}

fn finite_nonnegative_ratio(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn valid_baseline_report(report: &BaselineReportReceipt) -> bool {
    !report.report_id.trim().is_empty()
        && [
            &report.report_sha256,
            &report.scenario_digest,
            &report.workload_digest,
            &report.slo_digest,
            &report.methodology_digest,
            &report.cargo_lock_sha256,
            &report.prebuild_manifest_sha256,
            &report.binary_set_digest,
            &report.receipt_sha256,
        ]
        .iter()
        .all(|value| is_sha256(value))
        && finite_nonnegative_ratio(report.maximum_spread_ratio)
        && report.stable
        && validate_binary_set(&report.binary_sha256, &report.binary_set_digest).is_ok()
        && metric_map(report.metrics.clone()).is_ok()
}

fn approx_eq(left: f64, right: f64) -> bool {
    (left - right).abs() <= f64::EPSILON * left.abs().max(right.abs()).max(1.0) * 8.0
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

pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn digest_json<T: Serialize + ?Sized>(value: &T) -> String {
    sha256(&serde_json::to_vec(value).expect("typed W7 contract serialization cannot fail"))
}

#[cfg(test)]
mod semantic_tests {
    use super::*;

    fn repeat() -> Value {
        serde_json::json!({
            "reset_state_digest": "reset",
            "preloaded_state_digest": "preloaded",
            "state_digest": "steady",
            "phase": {
                "reset_operations": 1,
                "preload_operations": 0,
                "warmup_operations": 0,
                "steady_operations": 100,
                "reset_ms": 0,
                "preload_ms": 0,
                "warmup_ms": 0,
                "warmup_successes": 0,
                "warmup_errors": 0,
                "warmup_timeouts": 0,
                "warmup_rejections": 0,
                "steady_ms": 1000,
                "warmup_samples_in_steady_histogram": 0
            },
            "steady": {
                "offered": 100,
                "started": 100,
                "completed": 100,
                "successes": 100,
                "errors": 0,
                "timeouts": 0,
                "rejections": 0,
                "backlog_high_water": 0,
                "backlog_drained": true,
                "drain_ms": 1,
                "elapsed_ms": 1000,
                "offered_rate_per_second": 100.0,
                "achieved_rate_per_second": 100.0,
                "latency": {
                    "samples": 100,
                    "p50_us": 1,
                    "p90_us": 5,
                    "p99_us": 10,
                    "p999_us": null,
                    "p999_min_samples": 1000,
                    "p999_reportable": false,
                    "max_us": 20,
                    "overflow_count": 0
                }
            }
        })
    }

    fn evidence() -> Value {
        let repeat = repeat();
        let latency = repeat.pointer("/steady/latency").unwrap().clone();
        serde_json::json!({
            "id": "capacity",
            "claim": "capacity_knee",
            "criteria": {
                "p99_slo_us": 50,
                "p999_slo_us": null,
                "min_achieved_ratio": 0.95,
                "max_error_ratio": 0.0,
                "max_timeout_ratio": 0.0,
                "max_rejection_ratio": 0.0,
                "max_drain_ms": 100,
                "max_robust_spread_ratio": 0.05
            },
            "knee": {
                "sustainable_rate_per_second": 100.0,
                "evaluated": [{
                    "sample": {
                        "offered_rate_per_second": 100.0,
                        "achieved_rate_per_second": 100.0,
                        "achieved_rate_min_per_second": 100.0,
                        "achieved_rate_max_per_second": 100.0,
                        "offered": 100,
                        "started": 100,
                        "completed": 100,
                        "successes": 100,
                        "errors": 0,
                        "timeouts": 0,
                        "rejections": 0,
                        "backlog_drained": true,
                        "drain_ms": 1,
                        "robust_spread_ratio": 0.0,
                        "latency": latency
                    },
                    "repeats": [repeat.clone(), repeat.clone(), repeat],
                    "verdict": {"sustainable": true, "reasons": []}
                }]
            }
        })
    }

    #[test]
    fn perf_knee_metric_is_recomputed_from_raw_repeats() {
        let evidence = evidence();
        let selected = validate_knee_evidence("capacity", &evidence, Some(3)).unwrap();
        assert_eq!(selected.throughput_at_slo, Some(100.0));
        assert_eq!(selected.p99_microseconds_at_slo, Some(10.0));

        let mut forged = evidence;
        forged["knee"]["sustainable_rate_per_second"] = serde_json::json!(200.0);
        assert!(validate_knee_evidence("capacity", &forged, Some(3)).is_err());
    }
}
