//! Strict execution and parsing contract for the supplemental `redis-benchmark` leg.
//!
//! This module deliberately does not turn a closed-loop ecosystem tool into SLO or
//! capacity evidence. The coordinated-omission-safe RESP target owns those claims;
//! `redis-benchmark` is retained only for interoperability and supplemental
//! throughput characterization on one selected, node-local daemon endpoint.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Read};
use std::io::{ErrorKind, Write as _};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::profile::{PerformanceProfile, ProfileValidation, RunnerFingerprint};
use crate::report::{BuildIdentity, SourceIdentity};
use crate::targets::resp::{
    encode_resp2_command, parse_resp2, Resp2Limits, Resp2ParseStatus, Resp2Value,
};

pub const REDIS_BENCHMARK_CONTRACT_VERSION: u32 = 1;
pub const PINNED_REDIS_BENCHMARK_VERSION: &str = "redis-benchmark 7.2.5";
pub const REDIS_BENCHMARK_MEASUREMENT_ID: &str =
    "redis_benchmark_get_set_mset_throughput_and_interop";
pub const CLOSED_LOOP_METHODOLOGY: &str = "closed-loop";
pub const SUPPLEMENTAL_CLAIM_SCOPE: &str = "supplemental-interop-throughput-no-slo-knee";
pub const NODE_LOCAL_STATE_SCOPE: &str = "node-local";
pub const SELECTED_RESP_BOUNDARY: &str = "selected-daemon-resp-tcp";
pub const PINNED_TOOL_IDENTITY_POLICY: &str = "canonical-path-sha256-pinned-per-run";
pub const EXTERNAL_PROVENANCE_REGISTRY_VERSION: u32 = 1;
pub const EXTERNAL_PREBUILD_RECEIPT_VERSION: u32 = 1;
pub const SELECTED_DAEMON_RECEIPT_VERSION: u32 = 1;
pub const REDIS_BENCHMARK_PROVENANCE_REGISTRY_PATH: &str =
    "docs/testing/perf-scenarios/0.67/redis-benchmark-provenance-v1.toml";
pub const REDIS_BENCHMARK_CSV_HEADER: [&str; 8] = [
    "test",
    "rps",
    "avg_latency_ms",
    "min_latency_ms",
    "p50_latency_ms",
    "p95_latency_ms",
    "p99_latency_ms",
    "max_latency_ms",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedisBenchmarkContract {
    pub schema_version: u32,
    pub scenario_id: String,
    pub tool: RedisBenchmarkToolContract,
    pub endpoint: RedisBenchmarkEndpoint,
    pub identity: ClosedLoopSupplementalIdentity,
    pub cases: Vec<RedisBenchmarkCase>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedisBenchmarkToolContract {
    pub program: String,
    pub version_args: Vec<String>,
    pub expected_version: String,
    pub identity_policy: String,
    pub version_timeout_seconds: u64,
    pub case_timeout_seconds: u64,
    pub max_stdout_bytes: u64,
    pub max_stderr_bytes: u64,
    pub stderr_policy: String,
    pub repeats_per_case: u8,
    pub max_robust_spread_ratio: f64,
    pub required_runner_profile: String,
    pub provenance_registry_path: String,
    pub required_provenance_id: String,
    pub execution_environment: Vec<String>,
    pub repeat_isolation: String,
    pub deterministic_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedisBenchmarkEndpoint {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClosedLoopSupplementalIdentity {
    pub measurement_id: String,
    pub methodology: String,
    pub claim_scope: String,
    pub state_scope: String,
    pub network_boundary: String,
    pub scheduled_send_latency: bool,
    pub capacity_knee_eligible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedisBenchmarkCase {
    pub id: String,
    pub clients: u32,
    pub pipeline: u32,
    pub requests: u64,
    pub data_size_bytes: u32,
    pub operations: Vec<RedisBenchmarkOperation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RedisBenchmarkOperation {
    Get,
    Set,
    Mset,
}

impl RedisBenchmarkOperation {
    fn tool_name(self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Set => "set",
            Self::Mset => "mset",
        }
    }

    fn csv_row(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Set => "SET",
            Self::Mset => "MSET (10 keys)",
        }
    }
}

impl RedisBenchmarkContract {
    pub fn parse_toml(text: &str) -> Result<Self, ExternalToolError> {
        let contract: Self = toml::from_str(text)
            .map_err(|error| ExternalToolError::Contract(format!("invalid TOML: {error}")))?;
        contract.validate()?;
        Ok(contract)
    }

    pub fn load(path: &Path) -> Result<Self, ExternalToolError> {
        let text = fs::read_to_string(path).map_err(|error| {
            ExternalToolError::Contract(format!(
                "unable to read redis-benchmark contract {}: {error}",
                path.display()
            ))
        })?;
        Self::parse_toml(&text)
    }

    pub fn validate(&self) -> Result<(), ExternalToolError> {
        if self.schema_version != REDIS_BENCHMARK_CONTRACT_VERSION {
            return Err(ExternalToolError::Contract(format!(
                "unsupported redis-benchmark contract version {}",
                self.schema_version
            )));
        }
        if !valid_identifier(&self.scenario_id) {
            return Err(ExternalToolError::Contract(
                "scenario_id must be a non-empty portable identifier".to_owned(),
            ));
        }
        if self.tool.program != "redis-benchmark"
            || self.tool.version_args != ["--version"]
            || self.tool.expected_version != PINNED_REDIS_BENCHMARK_VERSION
            || self.tool.identity_policy != PINNED_TOOL_IDENTITY_POLICY
        {
            return Err(ExternalToolError::Contract(format!(
                "tool contract must pin `redis-benchmark --version` to {PINNED_REDIS_BENCHMARK_VERSION:?} and retain canonical-path/SHA-256 identity"
            )));
        }
        if !(1..=60).contains(&self.tool.version_timeout_seconds)
            || !(1..=1_800).contains(&self.tool.case_timeout_seconds)
        {
            return Err(ExternalToolError::Contract(
                "redis-benchmark version/case timeouts must be bounded to 1..=60s and 1..=1800s"
                    .to_owned(),
            ));
        }
        if self.tool.stderr_policy != "must-be-empty" {
            return Err(ExternalToolError::Contract(
                "redis-benchmark stderr policy must remain fail-closed (`must-be-empty`)"
                    .to_owned(),
            ));
        }
        if !(1..=4 * 1024 * 1024).contains(&self.tool.max_stdout_bytes)
            || !(1..=1024 * 1024).contains(&self.tool.max_stderr_bytes)
        {
            return Err(ExternalToolError::Contract(
                "redis-benchmark stdout/stderr capture caps must be bounded to 1..=4MiB and 1..=1MiB"
                    .to_owned(),
            ));
        }
        if !(3..=9).contains(&self.tool.repeats_per_case)
            || !self.tool.max_robust_spread_ratio.is_finite()
            || !(0.0..=1.0).contains(&self.tool.max_robust_spread_ratio)
            || !valid_identifier(&self.tool.required_runner_profile)
            || self.tool.provenance_registry_path != REDIS_BENCHMARK_PROVENANCE_REGISTRY_PATH
            || !valid_identifier(&self.tool.required_provenance_id)
            || self.tool.execution_environment != ["LANG=C", "LC_ALL=C", "TZ=UTC"]
            || self.tool.repeat_isolation != "del-set-get-exact-redis-benchmark-key"
            || self.tool.deterministic_key != "key:__rand_int__"
        {
            return Err(ExternalToolError::Contract(
                "redis-benchmark evidence requires 3..=9 repeats, a finite 0..=1 spread bound, a named runner profile, and the committed immutable provenance registry row"
                    .to_owned(),
            ));
        }
        if self.endpoint.host != "127.0.0.1" || self.endpoint.port == 0 {
            return Err(ExternalToolError::Contract(
                "external RESP characterization must select an explicit IPv4 loopback endpoint"
                    .to_owned(),
            ));
        }
        self.identity.validate()?;
        if self.cases.is_empty() {
            return Err(ExternalToolError::Contract(
                "at least one redis-benchmark case is required".to_owned(),
            ));
        }
        let mut ids = BTreeSet::new();
        for case in &self.cases {
            if !valid_identifier(&case.id) || !ids.insert(case.id.as_str()) {
                return Err(ExternalToolError::Contract(format!(
                    "redis-benchmark case ids must be unique portable identifiers: {:?}",
                    case.id
                )));
            }
            if case.clients == 0
                || case.pipeline == 0
                || case.requests == 0
                || case.data_size_bytes == 0
            {
                return Err(ExternalToolError::Contract(format!(
                    "redis-benchmark case {} contains a zero execution dimension",
                    case.id
                )));
            }
            if case.operations
                != [
                    RedisBenchmarkOperation::Get,
                    RedisBenchmarkOperation::Set,
                    RedisBenchmarkOperation::Mset,
                ]
            {
                return Err(ExternalToolError::Contract(format!(
                    "redis-benchmark case {} must retain the exact GET/SET/MSET operation order",
                    case.id
                )));
            }
        }
        Ok(())
    }

    pub fn version_argv(&self) -> Vec<String> {
        self.tool.version_args.clone()
    }

    pub fn benchmark_argv(&self, case: &RedisBenchmarkCase) -> Vec<String> {
        let operations = case
            .operations
            .iter()
            .map(|operation| operation.tool_name())
            .collect::<Vec<_>>()
            .join(",");
        vec![
            "--csv".to_owned(),
            "-h".to_owned(),
            self.endpoint.host.clone(),
            "-p".to_owned(),
            self.endpoint.port.to_string(),
            "-c".to_owned(),
            case.clients.to_string(),
            "-n".to_owned(),
            case.requests.to_string(),
            "-P".to_owned(),
            case.pipeline.to_string(),
            "-d".to_owned(),
            case.data_size_bytes.to_string(),
            "-t".to_owned(),
            operations,
        ]
    }

    pub fn expected_rows(&self, case: &RedisBenchmarkCase) -> Vec<String> {
        case.operations
            .iter()
            .map(|operation| operation.csv_row().to_owned())
            .collect()
    }

    pub fn digest(&self) -> String {
        // Validation makes field order and semantic aliases unambiguous. JSON is
        // used only as a stable serialization for the typed committed contract.
        sha256(
            &serde_json::to_vec(self)
                .expect("serializing the typed redis-benchmark contract cannot fail"),
        )
    }

    /// Stable source-contract identity. The selected daemon port belongs to a
    /// per-run capability and therefore cannot participate in a baseline key.
    pub fn committed_digest(&self) -> String {
        let mut committed = self.clone();
        committed.endpoint = RedisBenchmarkEndpoint {
            host: "127.0.0.1".to_owned(),
            port: 6379,
        };
        committed.digest()
    }
}

impl ClosedLoopSupplementalIdentity {
    fn validate(&self) -> Result<(), ExternalToolError> {
        let exact = self.measurement_id == REDIS_BENCHMARK_MEASUREMENT_ID
            && self.methodology == CLOSED_LOOP_METHODOLOGY
            && self.claim_scope == SUPPLEMENTAL_CLAIM_SCOPE
            && self.state_scope == NODE_LOCAL_STATE_SCOPE
            && self.network_boundary == SELECTED_RESP_BOUNDARY
            && !self.scheduled_send_latency
            && !self.capacity_knee_eligible;
        if !exact {
            return Err(ExternalToolError::Contract(
                "redis-benchmark identity must remain closed-loop supplemental, node-local, and ineligible for an SLO knee"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalToolProvenanceRegistry {
    pub schema_version: u32,
    pub registry_id: String,
    pub entries: Vec<ExternalToolProvenanceEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalToolProvenanceEntry {
    pub provenance_id: String,
    pub platform_key: String,
    pub tool: String,
    pub version: String,
    pub approved_for_mandatory: bool,
    pub provenance: ImmutableToolProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ImmutableToolProvenance {
    SourceArchiveRecipe {
        archive_url: String,
        archive_sha256: String,
        archive_size_bytes: u64,
        build_toolchain: String,
        recipe_version: u32,
        recipe_steps: Vec<String>,
        output_path: String,
        expected_version: String,
    },
    OciImage {
        image: String,
        digest: String,
        binary_path: String,
        expected_version: String,
    },
}

impl ExternalToolProvenanceRegistry {
    pub fn parse_toml(text: &str) -> Result<Self, ExternalToolError> {
        let registry: Self = toml::from_str(text).map_err(|error| {
            ExternalToolError::Provenance(format!("invalid provenance TOML: {error}"))
        })?;
        registry.validate()?;
        Ok(registry)
    }

    pub fn load(path: &Path) -> Result<Self, ExternalToolError> {
        let text = fs::read_to_string(path).map_err(|error| {
            ExternalToolError::Provenance(format!(
                "unable to read provenance registry {}: {error}",
                path.display()
            ))
        })?;
        Self::parse_toml(&text)
    }

    pub fn validate(&self) -> Result<(), ExternalToolError> {
        if self.schema_version != EXTERNAL_PROVENANCE_REGISTRY_VERSION
            || !valid_identifier(&self.registry_id)
            || self.entries.is_empty()
        {
            return Err(ExternalToolError::Provenance(
                "external-tool provenance registry identity is incomplete".to_owned(),
            ));
        }
        let mut ids = BTreeSet::new();
        let mut platform_tools = BTreeSet::new();
        for entry in &self.entries {
            if !valid_identifier(&entry.provenance_id)
                || !valid_platform_key(&entry.platform_key)
                || entry.tool != "redis-benchmark"
                || entry.version != PINNED_REDIS_BENCHMARK_VERSION
                || !entry.approved_for_mandatory
                || !ids.insert(entry.provenance_id.as_str())
                || !platform_tools.insert((entry.platform_key.as_str(), entry.tool.as_str()))
            {
                return Err(ExternalToolError::Provenance(format!(
                    "invalid, duplicate, or unapproved provenance row {:?}",
                    entry.provenance_id
                )));
            }
            entry.provenance.validate()?;
        }
        Ok(())
    }

    pub fn digest(&self) -> String {
        sha256(
            &serde_json::to_vec(self)
                .expect("typed external provenance registry serialization cannot fail"),
        )
    }

    pub fn approved_entry(
        &self,
        platform_key: &str,
        required_provenance_id: &str,
    ) -> Option<&ExternalToolProvenanceEntry> {
        self.entries.iter().find(|entry| {
            entry.platform_key == platform_key
                && entry.provenance_id == required_provenance_id
                && entry.tool == "redis-benchmark"
                && entry.version == PINNED_REDIS_BENCHMARK_VERSION
                && entry.approved_for_mandatory
        })
    }
}

impl ImmutableToolProvenance {
    fn validate(&self) -> Result<(), ExternalToolError> {
        match self {
            Self::SourceArchiveRecipe {
                archive_url,
                archive_sha256,
                archive_size_bytes,
                build_toolchain,
                recipe_version,
                recipe_steps,
                output_path,
                expected_version,
            } => {
                if !archive_url.starts_with("https://")
                    || !is_sha256(archive_sha256)
                    || *archive_size_bytes == 0
                    || build_toolchain.trim().is_empty()
                    || *recipe_version == 0
                    || recipe_steps.len() < 3
                    || recipe_steps.iter().any(|step| step.trim().is_empty())
                    || output_path != "src/redis-benchmark"
                    || expected_version != PINNED_REDIS_BENCHMARK_VERSION
                {
                    return Err(ExternalToolError::Provenance(
                        "source/archive provenance must pin HTTPS archive SHA/size, toolchain, non-trivial recipe, output, and exact version"
                            .to_owned(),
                    ));
                }
            }
            Self::OciImage {
                image,
                digest,
                binary_path,
                expected_version,
            } => {
                if image.trim().is_empty()
                    || !is_oci_digest(digest)
                    || !image.contains("@sha256:")
                    || !image.ends_with(digest)
                    || !binary_path.starts_with('/')
                    || expected_version != PINNED_REDIS_BENCHMARK_VERSION
                {
                    return Err(ExternalToolError::Provenance(
                        "OCI provenance must use an immutable sha256 image reference and absolute binary path"
                            .to_owned(),
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn source_archive_sha256(&self) -> Option<&str> {
        match self {
            Self::SourceArchiveRecipe { archive_sha256, .. } => Some(archive_sha256),
            Self::OciImage { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalToolPrebuildReceiptPayload {
    pub schema_version: u32,
    pub platform_key: String,
    pub provenance_id: String,
    pub provenance_registry_sha256: String,
    pub source_archive_sha256: Option<String>,
    pub tool_binary_id: String,
    pub tool_canonical_path: PathBuf,
    pub tool_binary_sha256: String,
    pub prebuild_manifest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalToolPrebuildReceipt {
    pub payload: ExternalToolPrebuildReceiptPayload,
    pub receipt_sha256: String,
}

impl ExternalToolPrebuildReceipt {
    pub fn seal(payload: ExternalToolPrebuildReceiptPayload) -> Self {
        let receipt_sha256 = typed_digest(&payload);
        Self {
            payload,
            receipt_sha256,
        }
    }

    fn validate(
        &self,
        registry: &ExternalToolProvenanceRegistry,
        provenance: &ExternalToolProvenanceEntry,
    ) -> Result<(), ExternalToolError> {
        let payload = &self.payload;
        if payload.schema_version != EXTERNAL_PREBUILD_RECEIPT_VERSION
            || payload.platform_key != provenance.platform_key
            || payload.provenance_id != provenance.provenance_id
            || payload.provenance_registry_sha256 != registry.digest()
            || payload.source_archive_sha256.as_deref()
                != provenance.provenance.source_archive_sha256()
            || payload.tool_binary_id != "redis-benchmark"
            || !payload.tool_canonical_path.is_absolute()
            || !is_sha256(&payload.tool_binary_sha256)
            || !is_sha256(&payload.prebuild_manifest_sha256)
            || self.receipt_sha256 != typed_digest(payload)
        {
            return Err(ExternalToolError::PrebuildReceipt(
                "external redis-benchmark prebuild receipt is incomplete, self-inconsistent, or not bound to the approved provenance row"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedDaemonReceiptPayload {
    pub schema_version: u32,
    pub node_id: String,
    pub endpoint: RedisBenchmarkEndpoint,
    pub daemon_binary_id: String,
    pub daemon_binary_sha256: String,
    pub prebuild_manifest_sha256: String,
    pub open_loop_endpoint_capability_sha256: String,
    pub capability_source: String,
    pub daemon_processes: bool,
    pub resp_listener_capability: bool,
    pub state_scope: String,
    pub selected_endpoint_only: bool,
    pub automatic_failover: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedDaemonReceipt {
    pub payload: SelectedDaemonReceiptPayload,
    pub capability_receipt_sha256: String,
}

impl SelectedDaemonReceipt {
    pub fn seal(payload: SelectedDaemonReceiptPayload) -> Self {
        let capability_receipt_sha256 = typed_digest(&payload);
        Self {
            payload,
            capability_receipt_sha256,
        }
    }

    fn validate(
        &self,
        contract: &RedisBenchmarkContract,
        build: &BuildIdentity,
    ) -> Result<(), ExternalToolError> {
        let payload = &self.payload;
        if payload.schema_version != SELECTED_DAEMON_RECEIPT_VERSION
            || !valid_identifier(&payload.node_id)
            || payload.endpoint != contract.endpoint
            || payload.daemon_binary_id.trim().is_empty()
            || !is_sha256(&payload.daemon_binary_sha256)
            || payload.prebuild_manifest_sha256 != build.prebuild_manifest_sha256
            || !is_sha256(&payload.open_loop_endpoint_capability_sha256)
            || payload.capability_source != "real-daemon-resp-readiness"
            || !payload.daemon_processes
            || !payload.resp_listener_capability
            || payload.state_scope != NODE_LOCAL_STATE_SCOPE
            || !payload.selected_endpoint_only
            || payload.automatic_failover
            || self.capability_receipt_sha256 != typed_digest(payload)
            || !build_has_binary(
                build,
                &payload.daemon_binary_id,
                &payload.daemon_binary_sha256,
            )
        {
            return Err(ExternalToolError::DaemonCapability(
                "selected daemon receipt is incomplete or overclaims the node-local RESP boundary"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedisBenchmarkRunContext {
    pub runner_profile: PerformanceProfile,
    pub observed_runner: RunnerFingerprint,
    pub source: SourceIdentity,
    pub build: BuildIdentity,
    pub open_loop_endpoint: RespOpenLoopEndpointCapability,
    pub selected_daemon: SelectedDaemonReceipt,
    pub external_tool_prebuild: ExternalToolPrebuildReceipt,
    pub committed_contract_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespOpenLoopEndpointCapability {
    pub endpoint: RedisBenchmarkEndpoint,
    pub endpoint_capability_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalEvidenceRunMode {
    LocalInformational,
    MandatoryReference,
}

fn valid_platform_key(value: &str) -> bool {
    value.starts_with("linux-") && valid_identifier(value)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn is_oci_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(is_sha256)
}

fn typed_digest<T: Serialize>(value: &T) -> String {
    sha256(&serde_json::to_vec(value).expect("typed receipt serialization cannot fail"))
}

fn build_has_binary(build: &BuildIdentity, id: &str, digest: &str) -> bool {
    build
        .binary_sha256
        .iter()
        .any(|(observed_id, observed_digest)| observed_id == id && observed_digest == digest)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingToolPolicy {
    /// Developer convenience only: absence is visible and yields no evidence.
    LocalSkipLoud,
    /// Release evidence: absence is an ordinary hard failure.
    MandatoryFailClosed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExternalToolRunOutcome {
    Completed(Box<RedisBenchmarkEvidence>),
    SkippedLoud(SkipLoudEvidence),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkipLoudEvidence {
    pub code: String,
    pub message: String,
    pub program: String,
    pub argv: Vec<String>,
    pub platform_key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedisBenchmarkEvidence {
    pub schema_version: u32,
    pub scenario_id: String,
    pub contract_sha256: String,
    pub effective_contract_sha256: String,
    pub tool_version: String,
    pub tool_identity: ResolvedExternalTool,
    pub identity: ClosedLoopSupplementalIdentity,
    pub endpoint: RedisBenchmarkEndpoint,
    pub run_mode: ExternalEvidenceRunMode,
    pub platform_key: String,
    pub provenance_registry_sha256: String,
    pub provenance: ExternalToolProvenanceEntry,
    pub runner_profile_sha256: String,
    pub profile_validation: ProfileValidation,
    pub run_context: RedisBenchmarkRunContext,
    pub version_probe: RawProcessEvidence,
    pub cases: Vec<RedisBenchmarkCaseEvidence>,
    pub measurements_stable: bool,
    pub ship_evidence_eligible: bool,
    pub stability_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedisBenchmarkCaseEvidence {
    pub case_id: String,
    pub clients: u32,
    pub pipeline: u32,
    pub requests: u64,
    pub data_size_bytes: u32,
    pub repeats: Vec<RedisBenchmarkRepeatEvidence>,
    pub operations: Vec<RedisBenchmarkOperationAggregate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedisBenchmarkRepeatEvidence {
    pub repeat: u8,
    pub initial_state: ExternalRepeatStateEvidence,
    pub rows: Vec<RedisBenchmarkCsvRow>,
    pub process: RawProcessEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalRepeatStateEvidence {
    pub method: String,
    pub key: String,
    pub payload_bytes: u32,
    pub payload_sha256: String,
    pub reset_reply_sha256: String,
    pub preload_reply_sha256: String,
    pub verification_reply_sha256: String,
    pub state_digest: String,
}

impl ExternalRepeatStateEvidence {
    pub fn deterministic_fixture(case: &RedisBenchmarkCase) -> Self {
        let key = "key:__rand_int__".to_owned();
        let payload = vec![b'R'; case.data_size_bytes as usize];
        Self {
            method: "del-set-get-exact-redis-benchmark-key".to_owned(),
            key: key.clone(),
            payload_bytes: case.data_size_bytes,
            payload_sha256: sha256(&payload),
            reset_reply_sha256: sha256(b":0\r\n"),
            preload_reply_sha256: sha256(b"+OK\r\n"),
            verification_reply_sha256: sha256(&encode_bulk_reply(&payload)),
            state_digest: external_state_digest(key.as_bytes(), &payload),
        }
    }

    fn validate(&self, case: &RedisBenchmarkCase) -> Result<(), ExternalToolError> {
        let payload = vec![b'R'; case.data_size_bytes as usize];
        if self.method != "del-set-get-exact-redis-benchmark-key"
            || self.key != "key:__rand_int__"
            || self.payload_bytes != case.data_size_bytes
            || self.payload_sha256 != sha256(&payload)
            || !is_sha256(&self.reset_reply_sha256)
            || self.preload_reply_sha256 != sha256(b"+OK\r\n")
            || self.verification_reply_sha256 != sha256(&encode_bulk_reply(&payload))
            || self.state_digest != external_state_digest(self.key.as_bytes(), &payload)
        {
            return Err(ExternalToolError::EvidenceValidation(
                "external repeat does not prove the exact DEL/SET/GET reset+preload state"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedisBenchmarkOperationAggregate {
    pub operation: String,
    pub repeat_count: u8,
    pub requests_per_second_samples: Vec<f64>,
    pub median_requests_per_second: f64,
    pub minimum_requests_per_second: f64,
    pub maximum_requests_per_second: f64,
    pub robust_spread_ratio: f64,
    pub stable: bool,
    pub stability_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawProcessEvidence {
    pub program: String,
    pub argv: Vec<String>,
    pub execution_environment: Vec<String>,
    pub exit_code: i32,
    pub timed_out: bool,
    pub stdout: String,
    pub stderr: String,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedisBenchmarkCsvRow {
    pub name: String,
    pub requests_per_second: String,
    pub average_latency_ms: String,
    pub minimum_latency_ms: String,
    pub p50_latency_ms: String,
    pub p95_latency_ms: String,
    pub p99_latency_ms: String,
    pub maximum_latency_ms: String,
}

impl RedisBenchmarkCsvRow {
    pub fn requests_per_second_f64(&self) -> f64 {
        self.requests_per_second
            .parse()
            .expect("CSV rows are validated before construction")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedExternalTool {
    pub requested_program: String,
    pub canonical_path: PathBuf,
    pub binary_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessLimits {
    pub timeout: Duration,
    pub max_stdout_bytes: usize,
    pub max_stderr_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessCapture {
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchErrorKind {
    MissingProgram,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchError {
    pub kind: LaunchErrorKind,
    pub message: String,
}

impl LaunchError {
    pub fn missing(message: impl Into<String>) -> Self {
        Self {
            kind: LaunchErrorKind::MissingProgram,
            message: message.into(),
        }
    }

    pub fn other(message: impl Into<String>) -> Self {
        Self {
            kind: LaunchErrorKind::Other,
            message: message.into(),
        }
    }
}

pub trait ExternalToolExecutor {
    fn platform_key(&self) -> String {
        current_platform_key().to_owned()
    }

    fn resolve(&self, program: &str) -> Result<ResolvedExternalTool, LaunchError>;

    fn resolve_exact(
        &self,
        logical_program: &str,
        canonical_path: &Path,
    ) -> Result<ResolvedExternalTool, LaunchError> {
        let path = canonical_path
            .to_str()
            .ok_or_else(|| LaunchError::other("external tool canonical path is not valid UTF-8"))?;
        let mut resolved = self.resolve(path)?;
        if resolved.canonical_path != canonical_path {
            return Err(LaunchError::other(format!(
                "exact external tool resolution escaped receipt path {}",
                canonical_path.display()
            )));
        }
        resolved.requested_program = logical_program.to_owned();
        Ok(resolved)
    }

    fn execute(
        &self,
        tool: &ResolvedExternalTool,
        argv: &[String],
        limits: ProcessLimits,
    ) -> Result<ProcessCapture, LaunchError>;

    fn prepare_repeat_state(
        &self,
        _endpoint: &RedisBenchmarkEndpoint,
        _case: &RedisBenchmarkCase,
        _repeat: u8,
    ) -> Result<ExternalRepeatStateEvidence, LaunchError> {
        Err(LaunchError::other(
            "external executor has no exact RESP reset/preload controller",
        ))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemToolExecutor;

impl ExternalToolExecutor for SystemToolExecutor {
    fn resolve(&self, program: &str) -> Result<ResolvedExternalTool, LaunchError> {
        let canonical_path = resolve_executable(program).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                LaunchError::missing(error.to_string())
            } else {
                LaunchError::other(error.to_string())
            }
        })?;
        let binary_sha256 = sha256_file(&canonical_path).map_err(|error| {
            LaunchError::other(format!(
                "unable to hash {}: {error}",
                canonical_path.display()
            ))
        })?;
        Ok(ResolvedExternalTool {
            requested_program: program.to_owned(),
            canonical_path,
            binary_sha256,
        })
    }

    fn execute(
        &self,
        tool: &ResolvedExternalTool,
        argv: &[String],
        limits: ProcessLimits,
    ) -> Result<ProcessCapture, LaunchError> {
        let mut child = Command::new(&tool.canonical_path)
            .args(argv)
            .env_clear()
            .env("LANG", "C")
            .env("LC_ALL", "C")
            .env("TZ", "UTC")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                if error.kind() == io::ErrorKind::NotFound {
                    LaunchError::missing(error.to_string())
                } else {
                    LaunchError::other(error.to_string())
                }
            })?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| LaunchError::other("external tool stdout pipe is unavailable"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| LaunchError::other("external tool stderr pipe is unavailable"))?;
        let stdout = read_process_stream(stdout, limits.max_stdout_bytes);
        let stderr = read_process_stream(stderr, limits.max_stderr_bytes);
        let deadline = Instant::now() + limits.timeout;
        let (exit_code, timed_out) = loop {
            match child.try_wait() {
                Ok(Some(status)) => break (status.code(), false),
                Ok(None) if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let status = child.wait().ok();
                    break (status.and_then(|status| status.code()), true);
                }
                Ok(None) => thread::sleep(Duration::from_millis(10)),
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = join_process_stream(stdout, "stdout");
                    let _ = join_process_stream(stderr, "stderr");
                    return Err(LaunchError::other(format!(
                        "failed while waiting for external tool: {error}"
                    )));
                }
            }
        };
        let stdout_result = join_process_stream(stdout, "stdout");
        let stderr_result = join_process_stream(stderr, "stderr");
        Ok(ProcessCapture {
            exit_code,
            timed_out,
            stdout: stdout_result?,
            stderr: stderr_result?,
        })
    }

    fn prepare_repeat_state(
        &self,
        endpoint: &RedisBenchmarkEndpoint,
        case: &RedisBenchmarkCase,
        _repeat: u8,
    ) -> Result<ExternalRepeatStateEvidence, LaunchError> {
        prepare_system_repeat_state(endpoint, case)
            .map_err(|error| LaunchError::other(error.to_string()))
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StreamCaptureError {
    #[error("stream read failed: {0}")]
    Read(String),
    #[error("stream exceeded the committed {limit}-byte capture cap")]
    LimitExceeded { limit: usize },
}

pub fn read_stream_bounded<R: Read>(
    mut stream: R,
    limit: usize,
) -> Result<Vec<u8>, StreamCaptureError> {
    let mut bytes = Vec::with_capacity(limit.min(8 * 1024));
    let mut chunk = [0_u8; 8 * 1024];
    loop {
        let read = stream
            .read(&mut chunk)
            .map_err(|error| StreamCaptureError::Read(error.to_string()))?;
        if read == 0 {
            return Ok(bytes);
        }
        if bytes.len().saturating_add(read) > limit {
            return Err(StreamCaptureError::LimitExceeded { limit });
        }
        bytes.extend_from_slice(&chunk[..read]);
    }
}

fn prepare_system_repeat_state(
    endpoint: &RedisBenchmarkEndpoint,
    case: &RedisBenchmarkCase,
) -> io::Result<ExternalRepeatStateEvidence> {
    let address: SocketAddr = format!("{}:{}", endpoint.host, endpoint.port)
        .parse()
        .map_err(|error| io::Error::new(ErrorKind::InvalidInput, error))?;
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let key = b"key:__rand_int__";
    let payload = vec![b'R'; case.data_size_bytes as usize];
    let mut request = Vec::new();
    request.extend_from_slice(&encode_resp2_command([b"DEL".as_slice(), key.as_slice()]));
    request.extend_from_slice(&encode_resp2_command([
        b"SET".as_slice(),
        key.as_slice(),
        payload.as_slice(),
    ]));
    request.extend_from_slice(&encode_resp2_command([b"GET".as_slice(), key.as_slice()]));
    stream.write_all(&request)?;
    stream.flush()?;

    let mut buffer = Vec::new();
    let mut replies = Vec::new();
    let mut offset = 0_usize;
    while replies.len() < 3 {
        while offset < buffer.len() && replies.len() < 3 {
            match parse_resp2(&buffer[offset..], Resp2Limits::default())
                .map_err(|error| io::Error::new(ErrorKind::InvalidData, error.to_string()))?
            {
                Resp2ParseStatus::Incomplete => break,
                Resp2ParseStatus::Complete { value, consumed } => {
                    let raw = buffer[offset..offset + consumed].to_vec();
                    offset += consumed;
                    replies.push((value, raw));
                }
            }
        }
        if replies.len() == 3 {
            break;
        }
        if buffer.len() >= 2 * 1024 * 1024 {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "RESP reset/preload response exceeded 2 MiB",
            ));
        }
        let mut chunk = [0_u8; 8192];
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                "RESP reset/preload response was truncated",
            ));
        }
        buffer.extend_from_slice(&chunk[..read]);
    }
    if offset != buffer.len()
        || !matches!(replies[0].0, Resp2Value::Integer(value) if value >= 0)
        || !matches!(&replies[1].0, Resp2Value::Simple(value) if value == b"OK")
        || !matches!(&replies[2].0, Resp2Value::Bulk(Some(value)) if value == &payload)
    {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "RESP reset/preload did not return exact DEL/OK/verified-bulk replies",
        ));
    }
    Ok(ExternalRepeatStateEvidence {
        method: "del-set-get-exact-redis-benchmark-key".to_owned(),
        key: String::from_utf8_lossy(key).into_owned(),
        payload_bytes: case.data_size_bytes,
        payload_sha256: sha256(&payload),
        reset_reply_sha256: sha256(&replies[0].1),
        preload_reply_sha256: sha256(&replies[1].1),
        verification_reply_sha256: sha256(&replies[2].1),
        state_digest: external_state_digest(key, &payload),
    })
}

fn encode_bulk_reply(payload: &[u8]) -> Vec<u8> {
    let mut reply = format!("${}\r\n", payload.len()).into_bytes();
    reply.extend_from_slice(payload);
    reply.extend_from_slice(b"\r\n");
    reply
}

fn external_state_digest(key: &[u8], payload: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"hydracache-redis-benchmark-repeat-state-v1");
    hasher.update((key.len() as u64).to_le_bytes());
    hasher.update(key);
    hasher.update((payload.len() as u64).to_le_bytes());
    hasher.update(payload);
    hex_sha256(hasher.finalize().as_ref())
}

fn read_process_stream<R: Read + Send + 'static>(
    stream: R,
    limit: usize,
) -> thread::JoinHandle<Result<Vec<u8>, StreamCaptureError>> {
    thread::spawn(move || read_stream_bounded(stream, limit))
}

fn join_process_stream(
    handle: thread::JoinHandle<Result<Vec<u8>, StreamCaptureError>>,
    stream: &str,
) -> Result<Vec<u8>, LaunchError> {
    match handle.join() {
        Ok(Ok(bytes)) => Ok(bytes),
        Ok(Err(error)) => Err(LaunchError::other(format!(
            "external tool {stream} capture failed: {error}"
        ))),
        Err(_) => Err(LaunchError::other(format!(
            "external tool {stream} capture thread panicked"
        ))),
    }
}

pub fn run_redis_benchmark<E: ExternalToolExecutor>(
    contract: &RedisBenchmarkContract,
    policy: MissingToolPolicy,
    executor: &E,
    provenance_registry: &ExternalToolProvenanceRegistry,
    run_context: &RedisBenchmarkRunContext,
) -> Result<ExternalToolRunOutcome, ExternalToolError> {
    contract.validate()?;
    provenance_registry.validate()?;
    let version_argv = contract.version_argv();
    let platform_key = executor.platform_key();
    let Some(provenance) =
        provenance_registry.approved_entry(&platform_key, &contract.tool.required_provenance_id)
    else {
        if policy == MissingToolPolicy::LocalSkipLoud {
            return Ok(ExternalToolRunOutcome::SkippedLoud(SkipLoudEvidence {
                code: "external-tool-platform-unsupported-local-skip-loud".to_owned(),
                message: format!(
                    "platform {platform_key:?} has no approved immutable redis-benchmark provenance row; local supplemental evidence was not produced"
                ),
                program: contract.tool.program.clone(),
                argv: version_argv,
                platform_key,
            }));
        }
        return Err(ExternalToolError::UnsupportedMandatoryPlatform { platform_key });
    };
    let profile_validation = validate_run_context(
        contract,
        provenance_registry,
        provenance,
        &platform_key,
        run_context,
        policy,
    )?;
    let resolved_tool = match policy {
        MissingToolPolicy::LocalSkipLoud => executor.resolve(&contract.tool.program),
        MissingToolPolicy::MandatoryFailClosed => executor.resolve_exact(
            &contract.tool.program,
            &run_context
                .external_tool_prebuild
                .payload
                .tool_canonical_path,
        ),
    };
    let tool_identity = match resolved_tool {
        Ok(identity) => identity,
        Err(error)
            if error.kind == LaunchErrorKind::MissingProgram
                && policy == MissingToolPolicy::LocalSkipLoud =>
        {
            return Ok(ExternalToolRunOutcome::SkippedLoud(SkipLoudEvidence {
                code: "external-tool-missing-local-skip-loud".to_owned(),
                message: format!(
                    "redis-benchmark is unavailable; local supplemental evidence was not produced: {}",
                    error.message
                ),
                program: contract.tool.program.clone(),
                argv: version_argv,
                platform_key,
            }));
        }
        Err(error) if error.kind == LaunchErrorKind::MissingProgram => {
            return Err(ExternalToolError::RequiredToolMissing {
                program: contract.tool.program.clone(),
                message: error.message,
            });
        }
        Err(error) => {
            return Err(ExternalToolError::Launch {
                phase: "tool-resolution".to_owned(),
                message: error.message,
            });
        }
    };
    validate_resolved_tool(&tool_identity)?;
    validate_tool_against_prebuild(&tool_identity, run_context)?;
    let version_capture = executor
        .execute(
            &tool_identity,
            &version_argv,
            process_limits(
                contract,
                Duration::from_secs(contract.tool.version_timeout_seconds),
            ),
        )
        .map_err(|error| ExternalToolError::Launch {
            phase: "version-probe".to_owned(),
            message: error.message,
        })?;
    verify_tool_identity(executor, &tool_identity, "version-probe-post-run")?;

    let version_probe = validate_successful_capture(
        &tool_identity.canonical_path.to_string_lossy(),
        version_argv,
        &contract.tool.execution_environment,
        "version-probe",
        version_capture,
    )?;
    let tool_version = exact_single_line(&version_probe.stdout).ok_or_else(|| {
        rejected_output(
            "version-probe",
            "version output must be exactly one non-empty line",
            &version_probe,
        )
    })?;
    if tool_version != contract.tool.expected_version {
        return Err(rejected_output(
            "version-probe",
            format!(
                "tool version mismatch: expected {:?}, observed {:?}",
                contract.tool.expected_version, tool_version
            ),
            &version_probe,
        ));
    }

    let mut cases = Vec::with_capacity(contract.cases.len());
    for case in &contract.cases {
        let mut repeats = Vec::with_capacity(usize::from(contract.tool.repeats_per_case));
        for repeat in 1..=contract.tool.repeats_per_case {
            let phase = format!("benchmark-case:{}:repeat:{repeat}", case.id);
            verify_tool_identity(executor, &tool_identity, &format!("{phase}-pre-run"))?;
            let initial_state = executor
                .prepare_repeat_state(&contract.endpoint, case, repeat)
                .map_err(|error| ExternalToolError::Launch {
                    phase: format!("{phase}-reset-preload"),
                    message: error.message,
                })?;
            initial_state.validate(case)?;
            let argv = contract.benchmark_argv(case);
            let capture = executor
                .execute(
                    &tool_identity,
                    &argv,
                    process_limits(
                        contract,
                        Duration::from_secs(contract.tool.case_timeout_seconds),
                    ),
                )
                .map_err(|error| ExternalToolError::Launch {
                    phase: phase.clone(),
                    message: if error.kind == LaunchErrorKind::MissingProgram {
                        format!(
                            "redis-benchmark disappeared after its successful version probe: {}",
                            error.message
                        )
                    } else {
                        error.message
                    },
                })?;
            verify_tool_identity(executor, &tool_identity, &format!("{phase}-post-run"))?;
            let process = validate_successful_capture(
                &tool_identity.canonical_path.to_string_lossy(),
                argv,
                &contract.tool.execution_environment,
                &phase,
                capture,
            )?;
            let rows =
                parse_redis_benchmark_csv(process.stdout.as_bytes(), &contract.expected_rows(case))
                    .map_err(|error| rejected_output(&phase, error.to_string(), &process))?;
            repeats.push(RedisBenchmarkRepeatEvidence {
                repeat,
                initial_state,
                rows,
                process,
            });
        }
        let operations = aggregate_case_operations(
            &repeats,
            &contract.expected_rows(case),
            contract.tool.max_robust_spread_ratio,
        )?;
        cases.push(RedisBenchmarkCaseEvidence {
            case_id: case.id.clone(),
            clients: case.clients,
            pipeline: case.pipeline,
            requests: case.requests,
            data_size_bytes: case.data_size_bytes,
            repeats,
            operations,
        });
    }

    let run_mode = match policy {
        MissingToolPolicy::LocalSkipLoud => ExternalEvidenceRunMode::LocalInformational,
        MissingToolPolicy::MandatoryFailClosed => ExternalEvidenceRunMode::MandatoryReference,
    };
    let (measurements_stable, stability_reasons) =
        external_stability(&cases, &profile_validation, run_mode);
    let ship_evidence_eligible = run_mode == ExternalEvidenceRunMode::MandatoryReference
        && measurements_stable
        && profile_validation.eligible;
    let evidence = RedisBenchmarkEvidence {
        schema_version: REDIS_BENCHMARK_CONTRACT_VERSION,
        scenario_id: contract.scenario_id.clone(),
        contract_sha256: run_context.committed_contract_sha256.clone(),
        effective_contract_sha256: contract.digest(),
        tool_version,
        tool_identity,
        identity: contract.identity.clone(),
        endpoint: contract.endpoint.clone(),
        run_mode,
        platform_key,
        provenance_registry_sha256: provenance_registry.digest(),
        provenance: provenance.clone(),
        runner_profile_sha256: typed_digest(&run_context.runner_profile),
        profile_validation,
        run_context: run_context.clone(),
        version_probe,
        cases,
        measurements_stable,
        ship_evidence_eligible,
        stability_reasons,
    };
    evidence.validate(contract, provenance_registry)?;
    Ok(ExternalToolRunOutcome::Completed(Box::new(evidence)))
}

fn validate_run_context(
    contract: &RedisBenchmarkContract,
    registry: &ExternalToolProvenanceRegistry,
    provenance: &ExternalToolProvenanceEntry,
    platform_key: &str,
    context: &RedisBenchmarkRunContext,
    policy: MissingToolPolicy,
) -> Result<ProfileValidation, ExternalToolError> {
    validate_source_identity(&context.source)?;
    validate_build_identity(&context.build)?;
    context
        .external_tool_prebuild
        .validate(registry, provenance)?;
    context.selected_daemon.validate(contract, &context.build)?;
    if context.runner_profile.name != contract.tool.required_runner_profile {
        return Err(ExternalToolError::RunnerIdentity(format!(
            "runner profile {:?} does not match required {:?}",
            context.runner_profile.name, contract.tool.required_runner_profile
        )));
    }
    let profile_validation = context.runner_profile.validate(&context.observed_runner);
    if policy == MissingToolPolicy::MandatoryFailClosed && !profile_validation.eligible {
        return Err(ExternalToolError::RunnerIdentity(format!(
            "mandatory redis-benchmark runner is ineligible: {:?}",
            profile_validation.reasons
        )));
    }
    let prebuild = &context.external_tool_prebuild.payload;
    let daemon = &context.selected_daemon.payload;
    if context.committed_contract_sha256 != contract.committed_digest()
        || prebuild.platform_key != platform_key
        || prebuild.prebuild_manifest_sha256 != context.build.prebuild_manifest_sha256
        || context.open_loop_endpoint.endpoint != contract.endpoint
        || context.open_loop_endpoint.endpoint != daemon.endpoint
        || !is_sha256(&context.open_loop_endpoint.endpoint_capability_sha256)
        || context.open_loop_endpoint.endpoint_capability_sha256
            != daemon.open_loop_endpoint_capability_sha256
    {
        return Err(ExternalToolError::PrebuildReceipt(
            "run context does not cross-bind the approved platform, prebuild manifest, actual binaries, and exact open-loop RESP endpoint capability"
                .to_owned(),
        ));
    }
    Ok(profile_validation)
}

fn validate_source_identity(source: &SourceIdentity) -> Result<(), ExternalToolError> {
    let commit_is_hex = matches!(source.git_commit.len(), 40 | 64)
        && source
            .git_commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
    if !commit_is_hex
        || !is_sha256(&source.cargo_lock_sha256)
        || source.toolchain.trim().is_empty()
        || source.build_flags.is_empty()
        || source.build_flags.iter().any(|flag| flag.trim().is_empty())
    {
        return Err(ExternalToolError::SourceIdentity(
            "source identity must bind an exact commit, Cargo.lock SHA-256, toolchain, and explicit build flags"
                .to_owned(),
        ));
    }
    Ok(())
}

fn validate_build_identity(build: &BuildIdentity) -> Result<(), ExternalToolError> {
    let mut ids = BTreeSet::new();
    if !is_sha256(&build.prebuild_contract_digest)
        || !is_sha256(&build.prebuild_manifest_sha256)
        || build.binary_sha256.is_empty()
        || build.binary_sha256.iter().any(|(id, digest)| {
            id.trim().is_empty() || !is_sha256(digest) || !ids.insert(id.as_str())
        })
    {
        return Err(ExternalToolError::PrebuildReceipt(
            "build identity must contain unique binary ids and lowercase SHA-256 values bound to the prebuild contract/manifest"
                .to_owned(),
        ));
    }
    Ok(())
}

fn validate_tool_against_prebuild(
    tool: &ResolvedExternalTool,
    context: &RedisBenchmarkRunContext,
) -> Result<(), ExternalToolError> {
    let receipt = &context.external_tool_prebuild.payload;
    if tool.canonical_path != receipt.tool_canonical_path
        || tool.binary_sha256 != receipt.tool_binary_sha256
    {
        return Err(ExternalToolError::PrebuildReceipt(format!(
            "actual redis-benchmark path/SHA does not match the prebuild receipt: actual={tool:?}, receipt_path={:?}, receipt_sha256={}",
            receipt.tool_canonical_path, receipt.tool_binary_sha256
        )));
    }
    Ok(())
}

fn aggregate_case_operations(
    repeats: &[RedisBenchmarkRepeatEvidence],
    expected_operations: &[String],
    max_robust_spread_ratio: f64,
) -> Result<Vec<RedisBenchmarkOperationAggregate>, ExternalToolError> {
    if repeats.len() < 3 || repeats.len() > usize::from(u8::MAX) {
        return Err(ExternalToolError::Aggregation(
            "redis-benchmark aggregation requires at least three repeats".to_owned(),
        ));
    }
    let mut aggregates = Vec::with_capacity(expected_operations.len());
    for operation in expected_operations {
        let mut samples = Vec::with_capacity(repeats.len());
        for repeat in repeats {
            let row = repeat
                .rows
                .iter()
                .find(|row| row.name == *operation)
                .ok_or_else(|| {
                    ExternalToolError::Aggregation(format!(
                        "repeat {} has no {operation:?} row",
                        repeat.repeat
                    ))
                })?;
            samples.push(row.requests_per_second_f64());
        }
        let mut sorted = samples.clone();
        sorted.sort_by(f64::total_cmp);
        let minimum = sorted[0];
        let maximum = sorted[sorted.len() - 1];
        let median = if sorted.len().is_multiple_of(2) {
            let upper = sorted.len() / 2;
            (sorted[upper - 1] + sorted[upper]) / 2.0
        } else {
            sorted[sorted.len() / 2]
        };
        let robust_spread_ratio = if median > 0.0 {
            (maximum - minimum) / median
        } else {
            f64::INFINITY
        };
        let mut stability_reasons = Vec::new();
        if !robust_spread_ratio.is_finite() || robust_spread_ratio > max_robust_spread_ratio {
            stability_reasons.push(format!(
                "throughput robust spread {robust_spread_ratio:.6} exceeds committed limit {max_robust_spread_ratio:.6}"
            ));
        }
        aggregates.push(RedisBenchmarkOperationAggregate {
            operation: operation.clone(),
            repeat_count: repeats.len() as u8,
            requests_per_second_samples: samples,
            median_requests_per_second: median,
            minimum_requests_per_second: minimum,
            maximum_requests_per_second: maximum,
            robust_spread_ratio,
            stable: stability_reasons.is_empty(),
            stability_reasons,
        });
    }
    Ok(aggregates)
}

fn external_stability(
    cases: &[RedisBenchmarkCaseEvidence],
    profile_validation: &ProfileValidation,
    run_mode: ExternalEvidenceRunMode,
) -> (bool, Vec<String>) {
    let mut reasons = cases
        .iter()
        .flat_map(|case| {
            case.operations.iter().flat_map(move |operation| {
                operation.stability_reasons.iter().map(move |reason| {
                    format!(
                        "case {} operation {}: {reason}",
                        case.case_id, operation.operation
                    )
                })
            })
        })
        .collect::<Vec<_>>();
    reasons.extend(
        profile_validation
            .reasons
            .iter()
            .map(|reason| format!("runner profile: {reason}")),
    );
    if run_mode == ExternalEvidenceRunMode::LocalInformational {
        reasons.push("local informational external-tool run is not ship evidence".to_owned());
    }
    reasons.sort();
    reasons.dedup();
    let measurements_stable = cases
        .iter()
        .flat_map(|case| &case.operations)
        .all(|operation| operation.stable);
    (measurements_stable, reasons)
}

impl RedisBenchmarkEvidence {
    pub fn validate(
        &self,
        contract: &RedisBenchmarkContract,
        provenance_registry: &ExternalToolProvenanceRegistry,
    ) -> Result<(), ExternalToolError> {
        contract.validate()?;
        provenance_registry.validate()?;
        let provenance = provenance_registry
            .approved_entry(&self.platform_key, &contract.tool.required_provenance_id)
            .ok_or_else(|| ExternalToolError::UnsupportedMandatoryPlatform {
                platform_key: self.platform_key.clone(),
            })?;
        let policy = match self.run_mode {
            ExternalEvidenceRunMode::LocalInformational => MissingToolPolicy::LocalSkipLoud,
            ExternalEvidenceRunMode::MandatoryReference => MissingToolPolicy::MandatoryFailClosed,
        };
        let expected_profile = validate_run_context(
            contract,
            provenance_registry,
            provenance,
            &self.platform_key,
            &self.run_context,
            policy,
        )?;
        validate_resolved_tool(&self.tool_identity)?;
        validate_tool_against_prebuild(&self.tool_identity, &self.run_context)?;
        if self.schema_version != REDIS_BENCHMARK_CONTRACT_VERSION
            || self.scenario_id != contract.scenario_id
            || self.contract_sha256 != self.run_context.committed_contract_sha256
            || self.effective_contract_sha256 != contract.digest()
            || self.tool_version != contract.tool.expected_version
            || self.identity != contract.identity
            || self.endpoint != contract.endpoint
            || self.provenance_registry_sha256 != provenance_registry.digest()
            || &self.provenance != provenance
            || self.runner_profile_sha256 != typed_digest(&self.run_context.runner_profile)
            || self.profile_validation != expected_profile
        {
            return Err(ExternalToolError::EvidenceValidation(
                "external report identity/provenance/profile fields do not recompute".to_owned(),
            ));
        }
        validate_raw_process(
            &self.version_probe,
            &self.tool_identity,
            &contract.version_argv(),
            &contract.tool.execution_environment,
        )?;
        if exact_single_line(&self.version_probe.stdout).as_deref()
            != Some(contract.tool.expected_version.as_str())
        {
            return Err(ExternalToolError::EvidenceValidation(
                "serialized version probe no longer proves the exact tool version".to_owned(),
            ));
        }
        if self.cases.len() != contract.cases.len() {
            return Err(ExternalToolError::EvidenceValidation(
                "external report has a partial/extra case matrix".to_owned(),
            ));
        }
        for (case, expected_case) in self.cases.iter().zip(&contract.cases) {
            if case.case_id != expected_case.id
                || case.clients != expected_case.clients
                || case.pipeline != expected_case.pipeline
                || case.requests != expected_case.requests
                || case.data_size_bytes != expected_case.data_size_bytes
                || case.repeats.len() != usize::from(contract.tool.repeats_per_case)
            {
                return Err(ExternalToolError::EvidenceValidation(format!(
                    "external report case {:?} does not match its committed dimensions/repeat count",
                    case.case_id
                )));
            }
            let mut initial_state_digests = BTreeSet::new();
            for (index, repeat) in case.repeats.iter().enumerate() {
                if repeat.repeat != (index + 1) as u8 {
                    return Err(ExternalToolError::EvidenceValidation(format!(
                        "case {} repeat numbering is not contiguous",
                        case.case_id
                    )));
                }
                repeat.initial_state.validate(expected_case)?;
                initial_state_digests.insert(repeat.initial_state.state_digest.clone());
                validate_raw_process(
                    &repeat.process,
                    &self.tool_identity,
                    &contract.benchmark_argv(expected_case),
                    &contract.tool.execution_environment,
                )?;
                let reparsed = parse_redis_benchmark_csv(
                    repeat.process.stdout.as_bytes(),
                    &contract.expected_rows(expected_case),
                )
                .map_err(|error| ExternalToolError::EvidenceValidation(error.to_string()))?;
                if reparsed != repeat.rows {
                    return Err(ExternalToolError::EvidenceValidation(format!(
                        "case {} repeat {} rows do not match raw CSV",
                        case.case_id, repeat.repeat
                    )));
                }
            }
            if initial_state_digests.len() != 1 {
                return Err(ExternalToolError::EvidenceValidation(format!(
                    "case {} repeats do not begin from one reproducible reset+preload state",
                    case.case_id
                )));
            }
            let expected_aggregates = aggregate_case_operations(
                &case.repeats,
                &contract.expected_rows(expected_case),
                contract.tool.max_robust_spread_ratio,
            )?;
            if case.operations != expected_aggregates {
                return Err(ExternalToolError::EvidenceValidation(format!(
                    "case {} aggregates/stability do not recompute from raw repeats",
                    case.case_id
                )));
            }
        }
        let (measurements_stable, stability_reasons) =
            external_stability(&self.cases, &self.profile_validation, self.run_mode);
        let ship_evidence_eligible = self.run_mode == ExternalEvidenceRunMode::MandatoryReference
            && measurements_stable
            && self.profile_validation.eligible;
        if self.measurements_stable != measurements_stable
            || self.ship_evidence_eligible != ship_evidence_eligible
            || self.stability_reasons != stability_reasons
        {
            return Err(ExternalToolError::EvidenceValidation(
                "external report stability/ship verdict does not recompute".to_owned(),
            ));
        }
        Ok(())
    }
}

fn validate_raw_process(
    process: &RawProcessEvidence,
    tool: &ResolvedExternalTool,
    expected_argv: &[String],
    expected_environment: &[String],
) -> Result<(), ExternalToolError> {
    if process.program != tool.canonical_path.to_string_lossy()
        || process.argv != expected_argv
        || process.execution_environment != expected_environment
        || process.exit_code != 0
        || process.timed_out
        || !process.stderr.is_empty()
        || process.stdout_bytes != process.stdout.len() as u64
        || process.stderr_bytes != process.stderr.len() as u64
        || process.stdout_sha256 != sha256(process.stdout.as_bytes())
        || process.stderr_sha256 != sha256(process.stderr.as_bytes())
    {
        return Err(ExternalToolError::EvidenceValidation(
            "serialized raw process evidence is inconsistent with its argv/status/streams"
                .to_owned(),
        ));
    }
    Ok(())
}

fn process_limits(contract: &RedisBenchmarkContract, timeout: Duration) -> ProcessLimits {
    ProcessLimits {
        timeout,
        max_stdout_bytes: usize::try_from(contract.tool.max_stdout_bytes)
            .expect("validated stdout capture cap must fit usize"),
        max_stderr_bytes: usize::try_from(contract.tool.max_stderr_bytes)
            .expect("validated stderr capture cap must fit usize"),
    }
}

fn validate_resolved_tool(tool: &ResolvedExternalTool) -> Result<(), ExternalToolError> {
    let path_is_absolute = tool.canonical_path.is_absolute();
    let digest_is_sha256 = tool.binary_sha256.len() == 64
        && tool
            .binary_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
    if tool.requested_program != "redis-benchmark" || !path_is_absolute || !digest_is_sha256 {
        return Err(ExternalToolError::ToolIdentity(format!(
            "resolved redis-benchmark identity is incomplete: {tool:?}"
        )));
    }
    Ok(())
}

fn verify_tool_identity<E: ExternalToolExecutor>(
    executor: &E,
    expected: &ResolvedExternalTool,
    phase: &str,
) -> Result<(), ExternalToolError> {
    let observed = executor
        .resolve_exact(&expected.requested_program, &expected.canonical_path)
        .map_err(|error| ExternalToolError::Launch {
            phase: phase.to_owned(),
            message: error.message,
        })?;
    validate_resolved_tool(&observed)?;
    if &observed != expected {
        return Err(ExternalToolError::ToolIdentity(format!(
            "redis-benchmark canonical path or binary SHA-256 changed during {phase}: expected {expected:?}, observed {observed:?}"
        )));
    }
    Ok(())
}

fn validate_successful_capture(
    program: &str,
    argv: Vec<String>,
    execution_environment: &[String],
    phase: &str,
    capture: ProcessCapture,
) -> Result<RawProcessEvidence, ExternalToolError> {
    let stdout_sha256 = sha256(&capture.stdout);
    let stderr_sha256 = sha256(&capture.stderr);
    let stdout = String::from_utf8(capture.stdout.clone()).map_err(|_| {
        ExternalToolError::OutputRejected {
            phase: phase.to_owned(),
            reason: "stdout is not valid UTF-8".to_owned(),
            exit_code: capture.exit_code,
            stdout_sha256: stdout_sha256.clone(),
            stderr_sha256: stderr_sha256.clone(),
        }
    })?;
    let stderr = String::from_utf8(capture.stderr.clone()).map_err(|_| {
        ExternalToolError::OutputRejected {
            phase: phase.to_owned(),
            reason: "stderr is not valid UTF-8".to_owned(),
            exit_code: capture.exit_code,
            stdout_sha256: stdout_sha256.clone(),
            stderr_sha256: stderr_sha256.clone(),
        }
    })?;
    let evidence = RawProcessEvidence {
        program: program.to_owned(),
        argv,
        execution_environment: execution_environment.to_vec(),
        exit_code: capture.exit_code.unwrap_or(-1),
        timed_out: capture.timed_out,
        stdout,
        stderr,
        stdout_bytes: capture.stdout.len().try_into().unwrap_or(u64::MAX),
        stderr_bytes: capture.stderr.len().try_into().unwrap_or(u64::MAX),
        stdout_sha256,
        stderr_sha256,
    };
    if capture.timed_out {
        return Err(rejected_output(
            phase,
            "external tool exceeded its committed per-process timeout",
            &evidence,
        ));
    }
    if capture.exit_code != Some(0) {
        return Err(rejected_output(
            phase,
            "external tool exited unsuccessfully",
            &evidence,
        ));
    }
    if !evidence.stderr.is_empty() {
        return Err(rejected_output(
            phase,
            "external tool emitted unexpected stderr",
            &evidence,
        ));
    }
    Ok(evidence)
}

fn rejected_output(
    phase: impl Into<String>,
    reason: impl Into<String>,
    evidence: &RawProcessEvidence,
) -> ExternalToolError {
    ExternalToolError::OutputRejected {
        phase: phase.into(),
        reason: reason.into(),
        exit_code: Some(evidence.exit_code),
        stdout_sha256: evidence.stdout_sha256.clone(),
        stderr_sha256: evidence.stderr_sha256.clone(),
    }
}

fn exact_single_line(value: &str) -> Option<String> {
    let value = value.strip_suffix('\n').unwrap_or(value);
    let value = value.strip_suffix('\r').unwrap_or(value);
    (!value.is_empty() && !value.contains(['\r', '\n'])).then(|| value.to_owned())
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ExternalToolError {
    #[error("invalid redis-benchmark contract: {0}")]
    Contract(String),
    #[error("invalid external-tool provenance: {0}")]
    Provenance(String),
    #[error("mandatory redis-benchmark evidence has no approved provenance for platform {platform_key:?}")]
    UnsupportedMandatoryPlatform { platform_key: String },
    #[error("invalid external-tool prebuild receipt: {0}")]
    PrebuildReceipt(String),
    #[error("invalid selected-daemon capability receipt: {0}")]
    DaemonCapability(String),
    #[error("invalid performance runner identity: {0}")]
    RunnerIdentity(String),
    #[error("invalid source identity: {0}")]
    SourceIdentity(String),
    #[error("invalid redis-benchmark repeat aggregation: {0}")]
    Aggregation(String),
    #[error("invalid redis-benchmark evidence report: {0}")]
    EvidenceValidation(String),
    #[error("required external tool {program:?} is missing: {message}")]
    RequiredToolMissing { program: String, message: String },
    #[error("unable to launch external tool during {phase}: {message}")]
    Launch { phase: String, message: String },
    #[error("external tool identity rejected: {0}")]
    ToolIdentity(String),
    #[error(
        "external tool output rejected during {phase}: {reason}; exit={exit_code:?}; stdout_sha256={stdout_sha256}; stderr_sha256={stderr_sha256}"
    )]
    OutputRejected {
        phase: String,
        reason: String,
        exit_code: Option<i32>,
        stdout_sha256: String,
        stderr_sha256: String,
    },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RedisBenchmarkCsvError {
    #[error("redis-benchmark CSV output is empty")]
    Empty,
    #[error("redis-benchmark CSV output is truncated because it lacks a final newline")]
    Truncated,
    #[error("redis-benchmark CSV output is not valid UTF-8")]
    InvalidUtf8,
    #[error("invalid redis-benchmark CSV row {line}: {reason}")]
    InvalidRow { line: usize, reason: String },
    #[error("duplicate redis-benchmark CSV row {0:?}")]
    Duplicate(String),
    #[error("unknown redis-benchmark CSV row {0:?}")]
    Unknown(String),
    #[error("missing redis-benchmark CSV row(s) {0:?}")]
    Missing(Vec<String>),
    #[error("redis-benchmark CSV row {row:?} contains non-finite throughput {value:?}")]
    NonFinite { row: String, value: String },
    #[error("redis-benchmark CSV row {row:?} contains invalid throughput {value:?}")]
    InvalidThroughput { row: String, value: String },
    #[error("redis-benchmark CSV row {row:?} contains invalid {column} latency {value:?}")]
    InvalidLatency {
        row: String,
        column: String,
        value: String,
    },
}

pub fn parse_redis_benchmark_csv(
    bytes: &[u8],
    expected_rows: &[String],
) -> Result<Vec<RedisBenchmarkCsvRow>, RedisBenchmarkCsvError> {
    if bytes.is_empty() {
        return Err(RedisBenchmarkCsvError::Empty);
    }
    if !bytes.ends_with(b"\n") {
        return Err(RedisBenchmarkCsvError::Truncated);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| RedisBenchmarkCsvError::InvalidUtf8)?;
    let expected = expected_rows.iter().cloned().collect::<BTreeSet<_>>();
    if expected.len() != expected_rows.len() || expected.iter().any(String::is_empty) {
        return Err(RedisBenchmarkCsvError::InvalidRow {
            line: 0,
            reason: "expected row names must be non-empty and unique".to_owned(),
        });
    }

    let mut lines = text.split_terminator('\n');
    let header_line = lines.next().ok_or(RedisBenchmarkCsvError::Empty)?;
    let header_line = header_line.strip_suffix('\r').unwrap_or(header_line);
    let header = parse_csv_fields(header_line)
        .map_err(|reason| RedisBenchmarkCsvError::InvalidRow { line: 1, reason })?;
    if header != REDIS_BENCHMARK_CSV_HEADER {
        return Err(RedisBenchmarkCsvError::InvalidRow {
            line: 1,
            reason: format!(
                "expected exact redis-benchmark 7.2.5 header {:?}, observed {header:?}",
                REDIS_BENCHMARK_CSV_HEADER
            ),
        });
    }

    let mut observed = BTreeMap::new();
    for (index, raw_line) in lines.enumerate() {
        let line_number = index + 2;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let fields =
            parse_csv_fields(line).map_err(|reason| RedisBenchmarkCsvError::InvalidRow {
                line: line_number,
                reason,
            })?;
        if fields.len() != REDIS_BENCHMARK_CSV_HEADER.len() {
            return Err(RedisBenchmarkCsvError::InvalidRow {
                line: line_number,
                reason: format!(
                    "expected exactly {} fields, observed {}",
                    REDIS_BENCHMARK_CSV_HEADER.len(),
                    fields.len()
                ),
            });
        }
        let name = fields[0].clone();
        if !expected.contains(&name) {
            return Err(RedisBenchmarkCsvError::Unknown(name));
        }
        if observed.contains_key(&name) {
            return Err(RedisBenchmarkCsvError::Duplicate(name));
        }
        let value = fields[1].clone();
        let throughput =
            value
                .parse::<f64>()
                .map_err(|_| RedisBenchmarkCsvError::InvalidThroughput {
                    row: name.clone(),
                    value: value.clone(),
                })?;
        if !throughput.is_finite() {
            return Err(RedisBenchmarkCsvError::NonFinite { row: name, value });
        }
        if throughput <= 0.0 {
            return Err(RedisBenchmarkCsvError::InvalidThroughput { row: name, value });
        }
        for (column, latency) in REDIS_BENCHMARK_CSV_HEADER[2..].iter().zip(&fields[2..]) {
            let parsed =
                latency
                    .parse::<f64>()
                    .map_err(|_| RedisBenchmarkCsvError::InvalidLatency {
                        row: name.clone(),
                        column: (*column).to_owned(),
                        value: latency.clone(),
                    })?;
            if !parsed.is_finite() || parsed < 0.0 {
                return Err(RedisBenchmarkCsvError::InvalidLatency {
                    row: name.clone(),
                    column: (*column).to_owned(),
                    value: latency.clone(),
                });
            }
        }
        observed.insert(
            name.clone(),
            RedisBenchmarkCsvRow {
                name,
                requests_per_second: value,
                average_latency_ms: fields[2].clone(),
                minimum_latency_ms: fields[3].clone(),
                p50_latency_ms: fields[4].clone(),
                p95_latency_ms: fields[5].clone(),
                p99_latency_ms: fields[6].clone(),
                maximum_latency_ms: fields[7].clone(),
            },
        );
    }

    let missing = expected_rows
        .iter()
        .filter(|name| !observed.contains_key(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(RedisBenchmarkCsvError::Missing(missing));
    }
    Ok(expected_rows
        .iter()
        .map(|name| {
            observed
                .remove(name)
                .expect("missing rows were rejected before ordering")
        })
        .collect())
}

fn parse_csv_fields(line: &str) -> Result<Vec<String>, String> {
    let bytes = line.as_bytes();
    let mut fields = Vec::new();
    let mut cursor = 0;
    loop {
        let (field, next) = parse_quoted_field(line, cursor)?;
        fields.push(field);
        cursor = next;
        if cursor == bytes.len() {
            return Ok(fields);
        }
        if bytes.get(cursor) != Some(&b',') {
            return Err("quoted CSV fields must be separated by one comma".to_owned());
        }
        cursor += 1;
        if cursor == bytes.len() {
            return Err("CSV row ends after a field separator".to_owned());
        }
    }
}

fn parse_quoted_field(line: &str, start: usize) -> Result<(String, usize), String> {
    let bytes = line.as_bytes();
    if bytes.get(start) != Some(&b'\"') {
        return Err("CSV fields must be quoted".to_owned());
    }
    let mut cursor = start + 1;
    let mut output = Vec::new();
    while let Some(byte) = bytes.get(cursor).copied() {
        if byte == b'\"' {
            if bytes.get(cursor + 1) == Some(&b'\"') {
                output.push(b'\"');
                cursor += 2;
                continue;
            }
            let value = String::from_utf8(output)
                .map_err(|_| "quoted field is not valid UTF-8".to_owned())?;
            return Ok((value, cursor + 1));
        }
        output.push(byte);
        cursor += 1;
    }
    Err("unterminated quoted field".to_owned())
}

fn resolve_executable(program: &str) -> io::Result<PathBuf> {
    let requested = Path::new(program);
    if requested.is_absolute() || requested.components().count() > 1 {
        return canonical_executable(requested);
    }
    let path = std::env::var_os("PATH")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "PATH is not set"))?;
    for directory in std::env::split_paths(&path) {
        for name in executable_names(program) {
            let candidate = directory.join(name);
            if let Ok(candidate) = canonical_executable(&candidate) {
                return Ok(candidate);
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("unable to resolve executable {program:?} on PATH"),
    ))
}

pub fn current_platform_key() -> &'static str {
    if cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        "linux-x86_64-gnu"
    } else if cfg!(all(
        target_os = "linux",
        target_arch = "aarch64",
        target_env = "gnu"
    )) {
        "linux-aarch64-gnu"
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "windows-x86_64-msvc"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "macos-aarch64"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "macos-x86_64"
    } else {
        "unsupported-platform"
    }
}

fn canonical_executable(path: &Path) -> io::Result<PathBuf> {
    let canonical = fs::canonicalize(path)?;
    if !canonical.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a regular file", canonical.display()),
        ));
    }
    Ok(canonical)
}

#[cfg(not(windows))]
fn executable_names(program: &str) -> Vec<String> {
    vec![program.to_owned()]
}

#[cfg(windows)]
fn executable_names(program: &str) -> Vec<String> {
    if Path::new(program).extension().is_some() {
        return vec![program.to_owned()];
    }
    let extensions = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_owned());
    extensions
        .split(';')
        .filter(|extension| !extension.is_empty())
        .map(|extension| format!("{program}{extension}"))
        .collect()
}

fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_sha256(hasher.finalize().as_ref()))
}

pub fn sha256(bytes: &[u8]) -> String {
    hex_sha256(&Sha256::digest(bytes))
}

fn hex_sha256(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            use std::fmt::Write as _;
            let _ = write!(output, "{byte:02x}");
            output
        })
}
