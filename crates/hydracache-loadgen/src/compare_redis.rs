//! W8 same-box comparison between one real HydraCache RESP endpoint and Redis.
//!
//! This is deliberately not a general benchmark wrapper. Reference evidence is
//! admitted only when it consumes an already-valid W3 report, the exact W3/W7
//! prebuild identities, one pinned `redis-benchmark` binary, and one immutable
//! Redis OCI image. Both systems receive the same command argv and reset state;
//! repeat order alternates AB/BA to expose order and thermal bias.

use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::profile::{PerformanceProfile, RunnerFingerprint};
use crate::report::{
    BuildIdentity, EvidenceRunMode, PerfReport, RespEndpointCapability, SourceIdentity,
};
use crate::resp_external::{
    current_platform_key, parse_redis_benchmark_csv, ExternalToolExecutor,
    ExternalToolPrebuildReceipt, ExternalToolProvenanceRegistry, LaunchErrorKind, ProcessLimits,
    RedisBenchmarkContract, RedisBenchmarkCsvRow, RedisBenchmarkEvidence, ResolvedExternalTool,
    SystemToolExecutor, REDIS_BENCHMARK_PROVENANCE_REGISTRY_PATH,
};
use crate::targets::resp::{
    encode_resp2_command, parse_resp2, Resp2Limits, Resp2ParseStatus, Resp2Value,
};
use crate::tiers::resp::{RespReferenceSuiteEvidence, RespReferenceSuiteReceipt};
use crate::tiers::resp_reference::{
    start_reference_daemon, LogEvidence, RespDaemonEvidence, RespDaemonLaunch, RespReferencePorts,
    ValidatedRespReferenceContext, VerifiedBinary, RESP_PING_FRAME, RESP_PONG_DISPLAY,
    RESP_PONG_FRAME,
};

pub const W8_SCENARIO_RELATIVE_PATH: &str =
    "docs/testing/perf-scenarios/0.67/compare-redis-v1.toml";
pub const W3_OPEN_LOOP_REPORT_RELATIVE_PATH: &str =
    "target/test-evidence/0.67/node-resp-open-loop.json";
pub const W3_DAEMON_LIFECYCLE_RELATIVE_PATH: &str =
    "target/test-evidence/0.67/node-resp-daemon-lifecycle.json";
pub const W3_EXTERNAL_REPORT_RELATIVE_PATH: &str =
    "target/test-evidence/0.67/node-resp-redis-benchmark.json";
pub const W3_SUITE_RECEIPT_RELATIVE_PATH: &str =
    "target/test-evidence/0.67/node-resp-suite-receipt.json";
pub const W3_EXTERNAL_SCENARIO_RELATIVE_PATH: &str =
    "docs/testing/perf-scenarios/0.67/resp-external-redis-benchmark-v1.toml";
pub const W8_REPORT_RELATIVE_PATH: &str = "target/test-evidence/0.67/compare-redis.json";
pub const W8_CANARY_MARKER: &str = "HC-CANARY-RED:W8";
pub const W8_REPORT_SCHEMA_VERSION: u32 = 1;
pub const W8_MEASUREMENT_ID: &str = "same_box_redis_vs_hydracache_resp_get_set_ratio";
pub const W8_METHOD: &str = "same-box-same-tool-alternating-ab-ba-closed-loop";
pub const W8_CLAIM_SCOPE: &str = "selected-node-local-resp-endpoint-comparison-only";
pub const W8_INTERPRETATION: &str = "Measured on one eligible reference runner with one exact redis-benchmark binary; this is not a universal superiority or aggregate-cluster-capacity claim.";

const MAX_CONTRACT_BYTES: u64 = 1024 * 1024;
const MAX_W3_REPORT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_W3_LIFECYCLE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_W3_SUITE_RECEIPT_BYTES: u64 = 1024 * 1024;
const MAX_W3_LOG_BYTES: u64 = 64 * 1024 * 1024;
const W8_DAEMON_REPEAT_INDEX: u32 = 80_008;
const COMMAND_ENVIRONMENT: [&str; 3] = ["LANG=C", "LC_ALL=C", "TZ=UTC"];
static W8_ARTIFACT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedisComparisonScenario {
    pub schema_version: u32,
    pub scenario_id: String,
    pub measurement_id: String,
    pub methodology: String,
    pub claim_scope: String,
    pub interpretation: String,
    pub required_runner_profile: String,
    pub repeats: u8,
    pub connections: u32,
    pub requests_per_system_case: u64,
    pub payload_bytes: u32,
    pub pipelines: Vec<u32>,
    pub operations: Vec<String>,
    pub key: String,
    pub max_robust_spread_ratio: f64,
    pub version_timeout_seconds: u64,
    pub case_timeout_seconds: u64,
    pub container_start_timeout_seconds: u64,
    pub max_stdout_bytes: u64,
    pub max_stderr_bytes: u64,
    pub execution_environment: Vec<String>,
    pub tool: ComparisonToolContract,
    pub docker: DockerContract,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonToolContract {
    pub program: String,
    pub version_args: Vec<String>,
    pub expected_version: String,
    pub identity_policy: String,
    pub required_provenance_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DockerContract {
    pub program: String,
    pub version_args: Vec<String>,
    pub version_prefix: String,
    pub identity_policy: String,
    pub platform: String,
    pub image_name: String,
    pub image_version: String,
    pub image_index_digest: String,
    pub image_platform_manifest_digest: String,
    pub container_port: u16,
    pub server_argv: Vec<String>,
}

impl RedisComparisonScenario {
    pub fn parse_toml(text: &str) -> Result<Self, RedisComparisonError> {
        let value = toml::from_str::<Self>(text)
            .map_err(|error| RedisComparisonError::Contract(format!("invalid TOML: {error}")))?;
        value.validate()?;
        Ok(value)
    }

    pub fn load(path: &Path) -> Result<Self, RedisComparisonError> {
        let metadata = fs::metadata(path).map_err(|error| {
            RedisComparisonError::Contract(format!(
                "unable to stat W8 scenario {}: {error}",
                path.display()
            ))
        })?;
        if metadata.len() == 0 || metadata.len() > MAX_CONTRACT_BYTES {
            return Err(RedisComparisonError::Contract(format!(
                "W8 scenario {} must be bounded and non-empty",
                path.display()
            )));
        }
        let text = fs::read_to_string(path).map_err(|error| {
            RedisComparisonError::Contract(format!(
                "unable to read W8 scenario {}: {error}",
                path.display()
            ))
        })?;
        Self::parse_toml(&text)
    }

    pub fn validate(&self) -> Result<(), RedisComparisonError> {
        let exact = self.schema_version == W8_REPORT_SCHEMA_VERSION
            && portable_id(&self.scenario_id)
            && self.measurement_id == W8_MEASUREMENT_ID
            && self.methodology == W8_METHOD
            && self.claim_scope == W8_CLAIM_SCOPE
            && self.interpretation == W8_INTERPRETATION
            && self.required_runner_profile == "reference-v1"
            && (5..=9).contains(&self.repeats)
            && self.connections > 0
            && self.requests_per_system_case > 0
            && self.payload_bytes > 0
            && self.pipelines == [1, 10]
            && self.operations == ["get", "set"]
            && self.key == "key:__rand_int__"
            && self.max_robust_spread_ratio.is_finite()
            && (0.0..=1.0).contains(&self.max_robust_spread_ratio)
            && (1..=60).contains(&self.version_timeout_seconds)
            && (1..=1_800).contains(&self.case_timeout_seconds)
            && (1..=300).contains(&self.container_start_timeout_seconds)
            && (1..=4 * 1024 * 1024).contains(&self.max_stdout_bytes)
            && (1..=1024 * 1024).contains(&self.max_stderr_bytes)
            && self.execution_environment == COMMAND_ENVIRONMENT
            && self.tool.program == "redis-benchmark"
            && self.tool.version_args == ["--version"]
            && self.tool.expected_version == "redis-benchmark 7.2.5"
            && self.tool.identity_policy == "w3-external-prebuild-receipt-canonical-path-sha256"
            && self.tool.required_provenance_id
                == "redis-benchmark-7.2.5-linux-x86_64-gnu-source-v1"
            && self.docker.program == "docker"
            && self.docker.version_args == ["--version"]
            && self.docker.version_prefix == "Docker version "
            && self.docker.identity_policy == "canonical-path-sha256-plus-exact-version-output"
            && self.docker.platform == "linux/amd64"
            && self.docker.image_name == "redis"
            && self.docker.image_version == "7.2.5"
            && self.docker.image_index_digest
                == "sha256:3aaec283e6e593bde528077d60280ac1589887067a39273348860837c9346d7e"
            && self.docker.image_platform_manifest_digest
                == "sha256:301f993bbc91d0b50b0737a97962905657d9f595e9935282b7db16a563b53d1b"
            && self.docker.container_port == 6379
            && self.docker.server_argv == ["redis-server", "--save", "", "--appendonly", "no"];
        if !exact {
            return Err(RedisComparisonError::Contract(
                "W8 must retain the exact five-repeat, GET/SET, pipeline-1/10, pinned-tool, pinned-image same-box contract"
                    .to_owned(),
            ));
        }
        Ok(())
    }

    pub fn digest(&self) -> String {
        typed_digest(self)
    }

    pub fn image_reference(&self) -> String {
        format!(
            "{}@{}",
            self.docker.image_name, self.docker.image_index_digest
        )
    }

    pub fn benchmark_argv(&self, endpoint: SocketAddr, pipeline: u32) -> Vec<String> {
        vec![
            "--csv".to_owned(),
            "-h".to_owned(),
            endpoint.ip().to_string(),
            "-p".to_owned(),
            endpoint.port().to_string(),
            "-c".to_owned(),
            self.connections.to_string(),
            "-n".to_owned(),
            self.requests_per_system_case.to_string(),
            "-P".to_owned(),
            pipeline.to_string(),
            "-d".to_owned(),
            self.payload_bytes.to_string(),
            "-t".to_owned(),
            self.operations.join(","),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedisComparisonRunMode {
    LocalInformational,
    MandatoryReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonSystem {
    Hydracache,
    Redis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionOrder {
    HydracacheThenRedis,
    RedisThenHydracache,
}

impl ExecutionOrder {
    fn for_repeat(repeat: u8) -> Self {
        if repeat % 2 == 1 {
            Self::HydracacheThenRedis
        } else {
            Self::RedisThenHydracache
        }
    }

    fn systems(self) -> [ComparisonSystem; 2] {
        match self {
            Self::HydracacheThenRedis => [ComparisonSystem::Hydracache, ComparisonSystem::Redis],
            Self::RedisThenHydracache => [ComparisonSystem::Redis, ComparisonSystem::Hydracache],
        }
    }
}

/// The only W3 predecessor shape admitted by W8. Every path is explicit so a
/// caller cannot accidentally substitute the open-loop report while omitting
/// the lifecycle, supplemental-tool evidence, or the suite seal that binds
/// all three artifacts together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W3ReferenceArtifactSet {
    pub open_loop_report: PathBuf,
    pub daemon_lifecycle: PathBuf,
    pub external_report: PathBuf,
    pub suite_receipt: PathBuf,
}

impl W3ReferenceArtifactSet {
    pub fn canonical(repo_root: &Path) -> Result<Self, RedisComparisonError> {
        let root = fs::canonicalize(repo_root).map_err(|error| {
            RedisComparisonError::W3(format!(
                "unable to canonicalize repository root {}: {error}",
                repo_root.display()
            ))
        })?;
        Ok(Self {
            open_loop_report: root.join(W3_OPEN_LOOP_REPORT_RELATIVE_PATH),
            daemon_lifecycle: root.join(W3_DAEMON_LIFECYCLE_RELATIVE_PATH),
            external_report: root.join(W3_EXTERNAL_REPORT_RELATIVE_PATH),
            suite_receipt: root.join(W3_SUITE_RECEIPT_RELATIVE_PATH),
        })
    }

    fn read_canonical(
        &self,
        repo_root: &Path,
    ) -> Result<LoadedW3ArtifactSet, RedisComparisonError> {
        let expected = Self::canonical(repo_root)?;
        Ok(LoadedW3ArtifactSet {
            open_loop: read_exact_canonical_artifact(
                "W3 open-loop report",
                &self.open_loop_report,
                &expected.open_loop_report,
                MAX_W3_REPORT_BYTES,
            )?,
            lifecycle: read_exact_canonical_artifact(
                "W3 daemon lifecycle",
                &self.daemon_lifecycle,
                &expected.daemon_lifecycle,
                MAX_W3_LIFECYCLE_BYTES,
            )?,
            external: read_exact_canonical_artifact(
                "W3 external-tool report",
                &self.external_report,
                &expected.external_report,
                MAX_W3_REPORT_BYTES,
            )?,
            suite: read_exact_canonical_artifact(
                "W3 suite receipt",
                &self.suite_receipt,
                &expected.suite_receipt,
                MAX_W3_SUITE_RECEIPT_BYTES,
            )?,
        })
    }
}

struct LoadedArtifact {
    canonical_path: PathBuf,
    bytes: Vec<u8>,
}

struct LoadedW3ArtifactSet {
    open_loop: LoadedArtifact,
    lifecycle: LoadedArtifact,
    external: LoadedArtifact,
    suite: LoadedArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct W3ReferenceBinding {
    pub canonical_path: PathBuf,
    pub artifact_sha256: String,
    pub daemon_lifecycle_path: PathBuf,
    pub daemon_lifecycle_sha256: String,
    pub external_report_path: PathBuf,
    pub external_report_sha256: String,
    pub suite_receipt_path: PathBuf,
    pub suite_receipt_artifact_sha256: String,
    pub suite_receipt_sha256: String,
    pub report_id: String,
    pub scenario_id: String,
    pub source: SourceIdentity,
    pub build: BuildIdentity,
    pub runner_profile: String,
    pub runner_contract_digest: String,
    pub observed_runner_fingerprint: String,
    pub selected_endpoint: String,
    pub endpoint_capability_sha256: String,
    pub archived_pid: u32,
    pub archived_started_unix_nanos: u64,
    pub external_tool_prebuild_receipt_sha256: String,
}

impl W3ReferenceBinding {
    pub fn load(
        repo_root: &Path,
        artifacts: &W3ReferenceArtifactSet,
        context: &ValidatedRespReferenceContext,
        external_tool_prebuild: &ExternalToolPrebuildReceipt,
    ) -> Result<Self, RedisComparisonError> {
        context
            .verify_binaries_unchanged()
            .map_err(|error| RedisComparisonError::W3(error.to_string()))?;
        let loaded = artifacts.read_canonical(repo_root)?;
        let report: PerfReport =
            serde_json::from_slice(&loaded.open_loop.bytes).map_err(|error| {
                RedisComparisonError::W3(format!("canonical W3 artifact is invalid JSON: {error}"))
            })?;
        let canonical = report.to_pretty_json().map_err(|error| {
            RedisComparisonError::W3(format!("W3 artifact cannot be revalidated: {error}"))
        })?;
        if loaded.open_loop.bytes != canonical {
            return Err(RedisComparisonError::W3(
                "W3 artifact bytes are not the canonical validated PERF_REPORT serialization"
                    .to_owned(),
            ));
        }
        let lifecycle: RespDaemonEvidence = parse_canonical_pretty_json_with_newline(
            "W3 daemon lifecycle",
            &loaded.lifecycle.bytes,
        )?;
        let external: RedisBenchmarkEvidence = parse_canonical_pretty_json_with_newline(
            "W3 external-tool report",
            &loaded.external.bytes,
        )?;
        let suite: RespReferenceSuiteReceipt =
            parse_canonical_pretty_json_with_newline("W3 suite receipt", &loaded.suite.bytes)?;
        let problems = report.validation_problems();
        let capability = report.resp_endpoint_capability.as_ref().ok_or_else(|| {
            RedisComparisonError::W3("W3 report has no typed endpoint capability".to_owned())
        })?;
        let capability_sha256 = capability
            .digest()
            .map_err(|error| RedisComparisonError::W3(error.to_string()))?;
        let exact = problems.is_empty()
            && report.stable
            && report.run_mode == EvidenceRunMode::ReferenceEvidence
            && report.report_id == "node-resp-open-loop-reference-v1"
            && report.surface.surface_kind == "node-resp"
            && report.surface.execution_mode == "real-daemon-tcp-resp-open-loop"
            && report.surface.state_scope == "node-local"
            && report.surface.network_boundary == "loopback-tcp"
            && report.surface.claim_scope == "selected-endpoint-capacity"
            && report.runner_profile == "reference-v1"
            && report.source == context.source
            && report.build == context.build
            && report.runner_contract == context.profile
            && report.observed_runner == context.runner
            && capability.server_binary_sha256 == context.server.sha256
            && capability.loadgen_binary_sha256 == context.loadgen.sha256
            && capability.prebuild_manifest_sha256 == context.manifest_sha256
            && capability.prebuild_contract_digest == context.build.prebuild_contract_digest
            && capability.source_commit == context.source.git_commit;
        if !exact {
            return Err(RedisComparisonError::W3(format!(
                "W3 predecessor is not exact receipt-bound reference evidence: {problems:?}"
            )));
        }
        validate_archived_lifecycle(capability, &lifecycle, &context.server, &context.loadgen)?;
        validate_suite_artifact_seal(
            &suite,
            &loaded.open_loop.bytes,
            &loaded.external.bytes,
            &loaded.lifecycle.bytes,
        )?;
        let external_contract =
            RedisBenchmarkContract::load(&repo_root.join(W3_EXTERNAL_SCENARIO_RELATIVE_PATH))
                .map_err(|error| RedisComparisonError::W3(error.to_string()))?;
        let provenance_registry = ExternalToolProvenanceRegistry::load(
            &repo_root.join(REDIS_BENCHMARK_PROVENANCE_REGISTRY_PATH),
        )
        .map_err(|error| RedisComparisonError::W3(error.to_string()))?;
        external
            .validate(&external_contract, &provenance_registry)
            .map_err(|error| RedisComparisonError::W3(error.to_string()))?;
        let suite_evidence = RespReferenceSuiteEvidence {
            open_loop: report.clone(),
            external: external.clone(),
            daemon: lifecycle.clone(),
        };
        suite
            .validate(
                &suite_evidence,
                &loaded.open_loop.bytes,
                &loaded.external.bytes,
                &loaded.lifecycle.bytes,
            )
            .map_err(|error| RedisComparisonError::W3(error.to_string()))?;
        let external_context = &external.run_context;
        if external_context.source != context.source
            || external_context.build != context.build
            || external_context.runner_profile != context.profile
            || external_context.observed_runner != context.runner
            || external_context.external_tool_prebuild != *external_tool_prebuild
            || external_tool_prebuild.receipt_sha256
                != external_context.external_tool_prebuild.receipt_sha256
        {
            return Err(RedisComparisonError::W3(
                "W3 external-tool report and W8 do not share the exact source, prebuild, runner, and external-tool receipt"
                    .to_owned(),
            ));
        }
        context
            .verify_binaries_unchanged()
            .map_err(|error| RedisComparisonError::W3(error.to_string()))?;
        let selected_endpoint = capability.selected_endpoint.clone();
        let archived_started_unix_nanos = capability.started_unix_nanos;
        let external_tool_prebuild_receipt_sha256 = external_context
            .external_tool_prebuild
            .receipt_sha256
            .clone();
        Ok(Self {
            canonical_path: loaded.open_loop.canonical_path,
            artifact_sha256: sha256(&loaded.open_loop.bytes),
            daemon_lifecycle_path: loaded.lifecycle.canonical_path,
            daemon_lifecycle_sha256: sha256(&loaded.lifecycle.bytes),
            external_report_path: loaded.external.canonical_path,
            external_report_sha256: sha256(&loaded.external.bytes),
            suite_receipt_path: loaded.suite.canonical_path,
            suite_receipt_artifact_sha256: sha256(&loaded.suite.bytes),
            suite_receipt_sha256: suite.receipt_sha256.clone(),
            report_id: report.report_id.clone(),
            scenario_id: report.scenario_id.clone(),
            source: report.source.clone(),
            build: report.build.clone(),
            runner_profile: report.runner_profile.clone(),
            runner_contract_digest: report.runner_contract_digest.clone(),
            observed_runner_fingerprint: report.observed_runner.fingerprint.clone(),
            selected_endpoint,
            endpoint_capability_sha256: capability_sha256,
            archived_pid: lifecycle.pid,
            archived_started_unix_nanos,
            external_tool_prebuild_receipt_sha256,
        })
    }

    fn archived_artifacts_still_match(&self) -> bool {
        if !bound_artifact_matches(
            &self.canonical_path,
            &self.artifact_sha256,
            MAX_W3_REPORT_BYTES,
        ) || !bound_artifact_matches(
            &self.daemon_lifecycle_path,
            &self.daemon_lifecycle_sha256,
            MAX_W3_LIFECYCLE_BYTES,
        ) || !bound_artifact_matches(
            &self.external_report_path,
            &self.external_report_sha256,
            MAX_W3_REPORT_BYTES,
        ) || !bound_artifact_matches(
            &self.suite_receipt_path,
            &self.suite_receipt_artifact_sha256,
            MAX_W3_SUITE_RECEIPT_BYTES,
        ) {
            return false;
        }
        let Ok(bytes) = fs::read(&self.suite_receipt_path) else {
            return false;
        };
        let Ok(suite) = serde_json::from_slice::<RespReferenceSuiteReceipt>(&bytes) else {
            return false;
        };
        suite.receipt_sha256 == self.suite_receipt_sha256
            && suite.receipt_sha256 == typed_digest(&suite.payload)
            && suite.payload.open_loop_report_sha256 == self.artifact_sha256
            && suite.payload.external_report_sha256 == self.external_report_sha256
            && suite.payload.daemon_lifecycle_sha256 == self.daemon_lifecycle_sha256
            && self.archived_pid != 0
            && self.archived_started_unix_nanos != 0
            && !process_is_alive(self.archived_pid)
    }
}

fn bound_artifact_matches(path: &Path, expected_sha256: &str, maximum_bytes: u64) -> bool {
    if !path.is_absolute() || !is_sha256(expected_sha256) {
        return false;
    }
    let Ok(canonical) = fs::canonicalize(path) else {
        return false;
    };
    let Ok(metadata) = fs::metadata(&canonical) else {
        return false;
    };
    canonical == path
        && metadata.is_file()
        && metadata.len() > 0
        && metadata.len() <= maximum_bytes
        && fs::read(canonical).is_ok_and(|bytes| sha256(&bytes) == expected_sha256)
}

fn read_exact_canonical_artifact(
    label: &str,
    supplied_path: &Path,
    expected_path: &Path,
    maximum_bytes: u64,
) -> Result<LoadedArtifact, RedisComparisonError> {
    let supplied = fs::canonicalize(supplied_path).map_err(|error| {
        RedisComparisonError::W3(format!(
            "{label} {} is missing or cannot be canonicalized: {error}",
            supplied_path.display()
        ))
    })?;
    let expected = fs::canonicalize(expected_path).map_err(|error| {
        RedisComparisonError::W3(format!(
            "canonical {label} {} is unavailable: {error}",
            expected_path.display()
        ))
    })?;
    if supplied != expected {
        return Err(RedisComparisonError::W3(format!(
            "W8 accepts only canonical {label} {}; got {}",
            expected.display(),
            supplied.display()
        )));
    }
    let metadata = fs::metadata(&supplied)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > maximum_bytes {
        return Err(RedisComparisonError::W3(format!(
            "canonical {label} must be a bounded non-empty regular file"
        )));
    }
    let bytes = fs::read(&supplied)?;
    if bytes.len() as u64 != metadata.len() {
        return Err(RedisComparisonError::W3(format!(
            "canonical {label} changed while it was being read"
        )));
    }
    Ok(LoadedArtifact {
        canonical_path: supplied,
        bytes,
    })
}

fn parse_canonical_pretty_json_with_newline<T>(
    label: &str,
    bytes: &[u8],
) -> Result<T, RedisComparisonError>
where
    T: DeserializeOwned + Serialize,
{
    let value = serde_json::from_slice::<T>(bytes).map_err(|error| {
        RedisComparisonError::W3(format!("{label} is invalid typed JSON: {error}"))
    })?;
    let mut canonical = serde_json::to_vec_pretty(&value).map_err(|error| {
        RedisComparisonError::W3(format!("{label} cannot be canonically serialized: {error}"))
    })?;
    canonical.push(b'\n');
    if canonical != bytes {
        return Err(RedisComparisonError::W3(format!(
            "{label} is not the exact canonical pretty-JSON artifact"
        )));
    }
    Ok(value)
}

fn validate_suite_artifact_seal(
    suite: &RespReferenceSuiteReceipt,
    open_loop_bytes: &[u8],
    external_bytes: &[u8],
    lifecycle_bytes: &[u8],
) -> Result<(), RedisComparisonError> {
    let payload = &suite.payload;
    let exact = payload.schema_version == 1
        && suite.receipt_sha256 == typed_digest(payload)
        && payload.open_loop_report_sha256 == sha256(open_loop_bytes)
        && payload.external_report_sha256 == sha256(external_bytes)
        && payload.daemon_lifecycle_sha256 == sha256(lifecycle_bytes);
    if !exact {
        return Err(RedisComparisonError::W3(
            "W3 suite receipt does not seal the exact four-artifact predecessor set".to_owned(),
        ));
    }
    Ok(())
}

fn validate_archived_lifecycle(
    capability: &RespEndpointCapability,
    lifecycle: &RespDaemonEvidence,
    expected_server: &VerifiedBinary,
    expected_loadgen: &VerifiedBinary,
) -> Result<(), RedisComparisonError> {
    let capability_sha256 = capability
        .digest()
        .map_err(|error| RedisComparisonError::W3(error.to_string()))?;
    validate_archived_binary(
        "W3 server binary",
        &lifecycle.server_binary_path,
        &lifecycle.server_binary_sha256,
        expected_server,
    )?;
    validate_archived_binary(
        "W3 loadgen binary",
        &lifecycle.loadgen_binary_path,
        &lifecycle.loadgen_binary_sha256,
        expected_loadgen,
    )?;
    validate_log_receipt("W3 daemon stdout", &lifecycle.stdout_log)?;
    validate_log_receipt("W3 daemon stderr", &lifecycle.stderr_log)?;
    let exact = lifecycle.pid == capability.pid
        && lifecycle.repeat_index == capability.repeat_index
        && lifecycle.direct_prebuilt_exec
        && lifecycle.binaries_verified_after_measurement
        && lifecycle.killed_and_waited
        && lifecycle.resp_endpoint == capability.config.redis_addr
        && lifecycle.admin_endpoint == capability.config.admin_addr
        && lifecycle.selected_endpoint == capability.selected_endpoint
        && lifecycle.endpoint_capability_digest == capability_sha256
        && lifecycle.data_dir == capability.config.storage_dir
        && lifecycle.server_binary_sha256 == capability.server_binary_sha256
        && lifecycle.loadgen_binary_sha256 == capability.loadgen_binary_sha256
        && lifecycle.readiness.selected_endpoint == capability.config.redis_addr
        && lifecycle.readiness.attempts > 0
        && lifecycle.readiness.exact_response == RESP_PONG_DISPLAY
        && lifecycle.readiness.request_sha256 == sha256(RESP_PING_FRAME)
        && lifecycle.readiness.response_sha256 == sha256(RESP_PONG_FRAME)
        && lifecycle.stdout_log.canonical_path != lifecycle.stderr_log.canonical_path
        && !process_is_alive(lifecycle.pid);
    if !exact {
        return Err(RedisComparisonError::W3(
            "W3 lifecycle does not prove the exact archived PID, readiness, binary, log, and kill/wait identity"
                .to_owned(),
        ));
    }
    Ok(())
}

fn validate_archived_binary(
    label: &str,
    recorded_path: &Path,
    recorded_sha256: &str,
    expected: &VerifiedBinary,
) -> Result<(), RedisComparisonError> {
    let path = fs::canonicalize(recorded_path).map_err(|error| {
        RedisComparisonError::W3(format!(
            "unable to canonicalize {label} {}: {error}",
            recorded_path.display()
        ))
    })?;
    let expected_path = fs::canonicalize(&expected.canonical_path).map_err(|error| {
        RedisComparisonError::W3(format!(
            "unable to revalidate expected {label} {}: {error}",
            expected.canonical_path.display()
        ))
    })?;
    let metadata = fs::metadata(&path)?;
    if path != expected_path
        || path != recorded_path
        || !metadata.is_file()
        || recorded_sha256 != expected.sha256
        || !is_sha256(recorded_sha256)
        || sha256(&fs::read(path)?) != recorded_sha256
    {
        return Err(RedisComparisonError::W3(format!(
            "{label} canonical path/SHA no longer matches the W3/W7 prebuild receipt"
        )));
    }
    Ok(())
}

fn validate_log_receipt(label: &str, receipt: &LogEvidence) -> Result<(), RedisComparisonError> {
    let path = fs::canonicalize(&receipt.canonical_path).map_err(|error| {
        RedisComparisonError::W3(format!(
            "unable to canonicalize {label} {}: {error}",
            receipt.canonical_path.display()
        ))
    })?;
    let metadata = fs::metadata(&path)?;
    if path != receipt.canonical_path
        || !metadata.is_file()
        || metadata.len() != receipt.bytes
        || metadata.len() > MAX_W3_LOG_BYTES
        || !is_sha256(&receipt.sha256)
        || sha256(&fs::read(path)?) != receipt.sha256
    {
        return Err(RedisComparisonError::W3(format!(
            "{label} canonical path/length/SHA no longer matches its lifecycle receipt"
        )));
    }
    Ok(())
}

fn file_sha_matches(path: &Path, expected_sha256: &str) -> bool {
    if !path.is_absolute() || !is_sha256(expected_sha256) {
        return false;
    }
    let Ok(canonical) = fs::canonicalize(path) else {
        return false;
    };
    canonical == path
        && fs::metadata(&canonical).is_ok_and(|metadata| metadata.is_file())
        && fs::read(canonical).is_ok_and(|bytes| sha256(&bytes) == expected_sha256)
}

#[cfg(target_os = "linux")]
fn process_is_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(target_os = "windows")]
fn process_is_alive(pid: u32) -> bool {
    std::process::Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!(
                "if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ exit 0 }} else {{ exit 1 }}"
            ),
        ])
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn process_is_alive(_pid: u32) -> bool {
    // Unknown process semantics cannot prove an archived PID is gone.
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutableIdentity {
    pub logical_name: String,
    pub canonical_path: PathBuf,
    pub sha256: String,
}

impl From<&ResolvedExternalTool> for ExecutableIdentity {
    fn from(value: &ResolvedExternalTool) -> Self {
        Self {
            logical_name: value.requested_program.clone(),
            canonical_path: value.canonical_path.clone(),
            sha256: value.binary_sha256.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawCommandEvidence {
    pub executable: ExecutableIdentity,
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

impl RawCommandEvidence {
    fn validate_integrity(&self) -> bool {
        self.executable.canonical_path.is_absolute()
            && is_sha256(&self.executable.sha256)
            && self.exit_code == 0
            && !self.timed_out
            && self.stdout_bytes == self.stdout.len() as u64
            && self.stderr_bytes == self.stderr.len() as u64
            && self.stdout_sha256 == sha256(self.stdout.as_bytes())
            && self.stderr_sha256 == sha256(self.stderr.as_bytes())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespResetPreloadEvidence {
    pub endpoint: SocketAddr,
    pub method: String,
    pub key: String,
    pub payload_bytes: u32,
    pub payload_sha256: String,
    pub reset_reply_sha256: String,
    pub preload_reply_sha256: String,
    pub verification_reply_sha256: String,
    pub logical_state_sha256: String,
}

impl RespResetPreloadEvidence {
    fn validate(&self, scenario: &RedisComparisonScenario, endpoint: SocketAddr) -> bool {
        let payload = vec![b'R'; scenario.payload_bytes as usize];
        self.endpoint == endpoint
            && endpoint.ip().is_loopback()
            && endpoint.port() != 0
            && self.method == "resp-del-set-get-exact-single-benchmark-key"
            && self.key == scenario.key
            && self.payload_bytes == scenario.payload_bytes
            && self.payload_sha256 == sha256(&payload)
            && is_sha256(&self.reset_reply_sha256)
            && self.preload_reply_sha256 == sha256(b"+OK\r\n")
            && self.verification_reply_sha256 == sha256(&encode_bulk_reply(&payload))
            && self.logical_state_sha256 == logical_state_digest(self.key.as_bytes(), &payload)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemCaseEvidence {
    pub system: ComparisonSystem,
    pub pipeline: u32,
    pub endpoint: SocketAddr,
    pub initial_state: RespResetPreloadEvidence,
    pub process: RawCommandEvidence,
    pub rows: Vec<RedisBenchmarkCsvRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonRepeatEvidence {
    pub repeat: u8,
    pub order: ExecutionOrder,
    pub cases: Vec<SystemCaseEvidence>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonAggregate {
    pub pipeline: u32,
    pub operation: String,
    pub ratio_definition: String,
    pub hydracache_requests_per_second: Vec<f64>,
    pub redis_requests_per_second: Vec<f64>,
    pub hydracache_over_redis_ratio: Vec<f64>,
    pub hydracache_median_requests_per_second: f64,
    pub redis_median_requests_per_second: f64,
    pub median_hydracache_over_redis_ratio: f64,
    pub hydracache_robust_spread_ratio: f64,
    pub redis_robust_spread_ratio: f64,
    pub ratio_robust_spread_ratio: f64,
    pub median_ratio_hydracache_first: f64,
    pub median_ratio_redis_first: f64,
    pub order_bias_ratio: f64,
    pub stable: bool,
    pub stability_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DockerImageIdentity {
    pub requested_reference: String,
    pub image_name: String,
    pub image_version: String,
    pub index_digest: String,
    pub platform: String,
    pub platform_manifest_digest: String,
    pub local_image_id: String,
    pub container_image_id: String,
    pub repo_digests: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DockerRedisEvidence {
    pub docker_identity: ExecutableIdentity,
    pub docker_version: String,
    pub version_probe: RawCommandEvidence,
    pub pull: RawCommandEvidence,
    pub manifest_inspect: RawCommandEvidence,
    pub image_inspect: RawCommandEvidence,
    pub container_run: RawCommandEvidence,
    pub port_inspect: RawCommandEvidence,
    pub container_inspect: RawCommandEvidence,
    pub container_stop: RawCommandEvidence,
    pub container_name: String,
    pub container_id: String,
    pub redis_endpoint: SocketAddr,
    pub readiness_request_sha256: String,
    pub readiness_response_sha256: String,
    pub readiness_attempts: u32,
    pub image: DockerImageIdentity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SameBoxRedisComparisonReport {
    pub schema_version: u32,
    pub release: String,
    pub report_id: String,
    pub scenario_id: String,
    pub scenario_sha256: String,
    pub measurement_id: String,
    pub methodology: String,
    pub claim_scope: String,
    pub interpretation: String,
    pub run_mode: RedisComparisonRunMode,
    pub source: SourceIdentity,
    pub build: BuildIdentity,
    pub runner_profile: PerformanceProfile,
    pub observed_runner: RunnerFingerprint,
    pub host_fingerprint: String,
    pub w3_reference: W3ReferenceBinding,
    pub hydracache_endpoint_capability: RespEndpointCapability,
    pub hydracache_endpoint_capability_sha256: String,
    pub hydracache_daemon_lifecycle: RespDaemonEvidence,
    pub redis_benchmark_identity: ExecutableIdentity,
    pub redis_benchmark_prebuild_receipt_sha256: String,
    pub redis_benchmark_version: String,
    pub redis_benchmark_version_probe: RawCommandEvidence,
    pub docker: DockerRedisEvidence,
    pub repeats: Vec<ComparisonRepeatEvidence>,
    pub aggregates: Vec<ComparisonAggregate>,
    pub measurements_stable: bool,
    pub ship_evidence_eligible: bool,
    pub stability_reasons: Vec<String>,
}

impl SameBoxRedisComparisonReport {
    pub fn validation_problems(&self, scenario: &RedisComparisonScenario) -> Vec<String> {
        let mut problems = Vec::new();
        if let Err(error) = scenario.validate() {
            problems.push(error.to_string());
        }
        if self.schema_version != W8_REPORT_SCHEMA_VERSION
            || self.release != "0.67.0"
            || self.report_id != "same-box-redis-vs-hydracache-resp-reference-v1"
            || self.scenario_id != scenario.scenario_id
            || self.scenario_sha256 != scenario.digest()
            || self.measurement_id != W8_MEASUREMENT_ID
            || self.methodology != W8_METHOD
            || self.claim_scope != W8_CLAIM_SCOPE
            || self.interpretation != W8_INTERPRETATION
        {
            problems.push("W8 report lost its exact method/scope/scenario identity".to_owned());
        }
        if self.runner_profile.name != scenario.required_runner_profile
            || self.runner_profile.validate(&self.observed_runner).eligible
                != self
                    .runner_profile
                    .validate(&self.observed_runner)
                    .reasons
                    .is_empty()
            || !self.runner_profile.validate(&self.observed_runner).eligible
            || self.host_fingerprint != self.observed_runner.fingerprint
            || self.w3_reference.source != self.source
            || self.w3_reference.build != self.build
            || self.w3_reference.runner_profile != self.runner_profile.name
            || self.w3_reference.observed_runner_fingerprint != self.observed_runner.fingerprint
            || self.w3_reference.external_tool_prebuild_receipt_sha256
                != self.redis_benchmark_prebuild_receipt_sha256
            || !self.w3_reference.archived_artifacts_still_match()
        {
            problems.push(
                "W8 and the sealed W3 artifact set do not share the exact eligible host/profile/source/prebuild identity"
                    .to_owned(),
            );
        }
        let capability_digest = self.hydracache_endpoint_capability.digest().ok();
        let capability = &self.hydracache_endpoint_capability;
        let lifecycle = &self.hydracache_daemon_lifecycle;
        if capability_digest.as_deref() != Some(self.hydracache_endpoint_capability_sha256.as_str())
            || capability.source_commit != self.source.git_commit
            || capability.prebuild_manifest_sha256 != self.build.prebuild_manifest_sha256
            || capability.prebuild_contract_digest != self.build.prebuild_contract_digest
            || !self.build.binary_sha256.iter().any(|(id, digest)| {
                id == "hydracache-server" && digest == &capability.server_binary_sha256
            })
            || !self.build.binary_sha256.iter().any(|(id, digest)| {
                id == "hydracache-loadgen" && digest == &capability.loadgen_binary_sha256
            })
            || lifecycle.pid != capability.pid
            || lifecycle.resp_endpoint != capability.config.redis_addr
            || lifecycle.endpoint_capability_digest != self.hydracache_endpoint_capability_sha256
            || !lifecycle.direct_prebuilt_exec
            || !lifecycle.binaries_verified_after_measurement
            || !lifecycle.killed_and_waited
            || process_is_alive(lifecycle.pid)
            || capability.pid == self.w3_reference.archived_pid
            || capability.started_unix_nanos <= self.w3_reference.archived_started_unix_nanos
            || !file_sha_matches(
                &lifecycle.server_binary_path,
                &lifecycle.server_binary_sha256,
            )
            || !file_sha_matches(
                &lifecycle.loadgen_binary_path,
                &lifecycle.loadgen_binary_sha256,
            )
            || validate_log_receipt("W8 daemon stdout", &lifecycle.stdout_log).is_err()
            || validate_log_receipt("W8 daemon stderr", &lifecycle.stderr_log).is_err()
        {
            problems.push(
                "live HydraCache endpoint/lifecycle is not bound to the exact W3 source and binaries"
                    .to_owned(),
            );
        }
        if !self.redis_benchmark_identity.canonical_path.is_absolute()
            || !is_sha256(&self.redis_benchmark_identity.sha256)
            || !is_sha256(&self.redis_benchmark_prebuild_receipt_sha256)
            || self.redis_benchmark_identity.logical_name != scenario.tool.program
            || self.redis_benchmark_version != scenario.tool.expected_version
            || !raw_command_matches(
                &self.redis_benchmark_version_probe,
                &self.redis_benchmark_identity,
                &scenario.tool.version_args,
            )
            || exact_single_line(&self.redis_benchmark_version_probe.stdout).as_deref()
                != Some(scenario.tool.expected_version.as_str())
            || !self.redis_benchmark_version_probe.stderr.is_empty()
        {
            problems
                .push("redis-benchmark exact W3 prebuild identity/version is invalid".to_owned());
        }
        problems.extend(validate_docker_evidence(&self.docker, scenario));
        problems.extend(validate_repeats(
            &self.repeats,
            scenario,
            capability.config.redis_addr,
            self.docker.redis_endpoint,
            &self.redis_benchmark_identity,
        ));
        let derived = derive_aggregates(&self.repeats, scenario);
        match derived {
            Ok(aggregates) if aggregate_lists_equal(&aggregates, &self.aggregates) => {
                let mut derived_reasons = aggregates
                    .iter()
                    .flat_map(|aggregate| aggregate.stability_reasons.iter().cloned())
                    .collect::<Vec<_>>();
                derived_reasons.sort();
                derived_reasons.dedup();
                let expected_stable = derived_reasons.is_empty();
                if self.measurements_stable != expected_stable
                    || self.stability_reasons != derived_reasons
                {
                    problems.push(
                        "stored W8 stability verdict/reasons do not match raw spread and order-bias evidence"
                            .to_owned(),
                    );
                }
            }
            Ok(_) => {
                problems.push("stored W8 aggregates do not match raw repeat evidence".to_owned())
            }
            Err(error) => problems.push(error.to_string()),
        }
        problems.sort();
        problems.dedup();
        let expected_ship = self.run_mode == RedisComparisonRunMode::MandatoryReference
            && self.measurements_stable
            && problems.is_empty()
            && self.stability_reasons.is_empty();
        if self.ship_evidence_eligible != expected_ship {
            problems.push("stored W8 ship verdict is self-asserted or inconsistent".to_owned());
        }
        problems.sort();
        problems.dedup();
        problems
    }

    pub fn to_pretty_json(
        &self,
        scenario: &RedisComparisonScenario,
    ) -> Result<Vec<u8>, RedisComparisonError> {
        let problems = self.validation_problems(scenario);
        if !problems.is_empty() {
            return Err(RedisComparisonError::Evidence(format!(
                "W8 report validation failed: {problems:?}"
            )));
        }
        serde_json::to_vec_pretty(self).map_err(RedisComparisonError::Json)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparisonSkipLoud {
    pub code: String,
    pub message: String,
    pub missing_program: String,
}

#[derive(Debug)]
pub enum RedisComparisonOutcome {
    Completed(Box<SameBoxRedisComparisonReport>),
    SkippedLoud(ComparisonSkipLoud),
}

#[derive(Debug, thiserror::Error)]
pub enum RedisComparisonError {
    #[error("invalid W8 comparison contract: {0}")]
    Contract(String),
    #[error("W3 predecessor validation failed: {0}")]
    W3(String),
    #[error("W8 reference prerequisite failed: {0}")]
    Prerequisite(String),
    #[error("W8 external process failed during {phase}: {detail}")]
    Process { phase: String, detail: String },
    #[error("W8 evidence validation failed: {0}")]
    Evidence(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

fn validate_docker_evidence(
    docker: &DockerRedisEvidence,
    scenario: &RedisComparisonScenario,
) -> Vec<String> {
    let mut problems = Vec::new();
    let identity = &docker.docker_identity;
    let expected_ref = scenario.image_reference();
    if identity.logical_name != scenario.docker.program
        || !identity.canonical_path.is_absolute()
        || !is_sha256(&identity.sha256)
        || !raw_command_matches(
            &docker.version_probe,
            identity,
            &scenario.docker.version_args,
        )
        || !docker
            .docker_version
            .starts_with(&scenario.docker.version_prefix)
        || exact_single_line(&docker.version_probe.stdout).as_deref()
            != Some(docker.docker_version.as_str())
    {
        problems.push("Docker canonical binary/version provenance is incomplete".to_owned());
    }
    let expected_pull = vec![
        "pull".to_owned(),
        "--platform".to_owned(),
        scenario.docker.platform.clone(),
        expected_ref.clone(),
    ];
    let expected_inspect = vec![
        "image".to_owned(),
        "inspect".to_owned(),
        expected_ref.clone(),
    ];
    let expected_manifest_inspect = vec![
        "manifest".to_owned(),
        "inspect".to_owned(),
        "--verbose".to_owned(),
        expected_ref.clone(),
    ];
    let expected_port = vec![
        "port".to_owned(),
        docker.container_name.clone(),
        format!("{}/tcp", scenario.docker.container_port),
    ];
    let expected_container_inspect = vec![
        "container".to_owned(),
        "inspect".to_owned(),
        docker.container_name.clone(),
    ];
    let expected_stop = vec![
        "stop".to_owned(),
        "--time".to_owned(),
        "10".to_owned(),
        docker.container_name.clone(),
    ];
    let mut expected_run = vec![
        "run".to_owned(),
        "--detach".to_owned(),
        "--rm".to_owned(),
        "--name".to_owned(),
        docker.container_name.clone(),
        "--platform".to_owned(),
        scenario.docker.platform.clone(),
        "--publish".to_owned(),
        format!("127.0.0.1::{}/tcp", scenario.docker.container_port),
        expected_ref.clone(),
    ];
    expected_run.extend(scenario.docker.server_argv.clone());
    if !raw_command_matches(&docker.pull, identity, &expected_pull)
        || !raw_command_matches(
            &docker.manifest_inspect,
            identity,
            &expected_manifest_inspect,
        )
        || !raw_command_matches(&docker.image_inspect, identity, &expected_inspect)
        || !raw_command_matches(&docker.container_run, identity, &expected_run)
        || !raw_command_matches(&docker.port_inspect, identity, &expected_port)
        || !raw_command_matches(
            &docker.container_inspect,
            identity,
            &expected_container_inspect,
        )
        || !raw_command_matches(&docker.container_stop, identity, &expected_stop)
    {
        problems.push(
            "Docker argv/raw command receipts do not match the committed W8 method".to_owned(),
        );
    }
    if !manifest_output_contains_platform_digest(
        &docker.manifest_inspect.stdout,
        &scenario.docker.image_platform_manifest_digest,
        "linux",
        "amd64",
    ) {
        problems.push(
            "Docker manifest receipt does not prove the committed linux/amd64 manifest digest"
                .to_owned(),
        );
    }
    let image = &docker.image;
    if image.requested_reference != expected_ref
        || image.image_name != scenario.docker.image_name
        || image.image_version != scenario.docker.image_version
        || image.index_digest != scenario.docker.image_index_digest
        || image.platform != scenario.docker.platform
        || image.platform_manifest_digest != scenario.docker.image_platform_manifest_digest
        || !is_oci_digest(&image.local_image_id)
        || image.container_image_id != image.local_image_id
        || !image
            .repo_digests
            .iter()
            .any(|digest| digest == &expected_ref)
        || docker.redis_endpoint.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST)
        || docker.redis_endpoint.port() == 0
        || docker.container_id.len() != 64
        || !docker
            .container_id
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || docker.readiness_request_sha256 != sha256(b"*1\r\n$4\r\nPING\r\n")
        || docker.readiness_response_sha256 != sha256(b"+PONG\r\n")
        || docker.readiness_attempts == 0
    {
        problems.push(
            "Redis container does not prove the pinned image, platform, live identity, and readiness"
                .to_owned(),
        );
    }
    problems
}

fn validate_repeats(
    repeats: &[ComparisonRepeatEvidence],
    scenario: &RedisComparisonScenario,
    hydracache_endpoint: SocketAddr,
    redis_endpoint: SocketAddr,
    tool: &ExecutableIdentity,
) -> Vec<String> {
    let mut problems = Vec::new();
    if repeats.len() != usize::from(scenario.repeats) {
        problems.push(format!(
            "W8 requires exactly {} raw repeats, observed {}",
            scenario.repeats,
            repeats.len()
        ));
        return problems;
    }
    for (index, repeat) in repeats.iter().enumerate() {
        let repeat_number = u8::try_from(index + 1).unwrap_or(u8::MAX);
        let expected_order = ExecutionOrder::for_repeat(repeat_number);
        if repeat.repeat != repeat_number
            || repeat.order != expected_order
            || repeat.cases.len() != scenario.pipelines.len() * 2
        {
            problems.push(format!(
                "repeat {repeat_number} lost its exact alternating order or case count"
            ));
            continue;
        }
        let mut cursor = 0_usize;
        for pipeline in &scenario.pipelines {
            for system in expected_order.systems() {
                let case = &repeat.cases[cursor];
                cursor += 1;
                let endpoint = match system {
                    ComparisonSystem::Hydracache => hydracache_endpoint,
                    ComparisonSystem::Redis => redis_endpoint,
                };
                let argv = scenario.benchmark_argv(endpoint, *pipeline);
                let expected_rows = ["GET".to_owned(), "SET".to_owned()];
                let parsed =
                    parse_redis_benchmark_csv(case.process.stdout.as_bytes(), &expected_rows);
                if case.system != system
                    || case.pipeline != *pipeline
                    || case.endpoint != endpoint
                    || !case.initial_state.validate(scenario, endpoint)
                    || !raw_command_matches(&case.process, tool, &argv)
                    || !case.process.stderr.is_empty()
                    || parsed.as_ref().ok() != Some(&case.rows)
                {
                    problems.push(format!(
                        "repeat {repeat_number} pipeline {pipeline} {system:?} is not an exact same-tool run"
                    ));
                }
            }
        }
    }
    problems
}

fn derive_aggregates(
    repeats: &[ComparisonRepeatEvidence],
    scenario: &RedisComparisonScenario,
) -> Result<Vec<ComparisonAggregate>, RedisComparisonError> {
    if repeats.len() != usize::from(scenario.repeats) {
        return Err(RedisComparisonError::Evidence(
            "cannot aggregate an incomplete W8 repeat set".to_owned(),
        ));
    }
    let mut output = Vec::new();
    for pipeline in &scenario.pipelines {
        for operation in ["GET", "SET"] {
            let mut hydracache = Vec::with_capacity(repeats.len());
            let mut redis = Vec::with_capacity(repeats.len());
            let mut ratio_hydra_first = Vec::new();
            let mut ratio_redis_first = Vec::new();
            for repeat in repeats {
                let hydra = case_rps(repeat, *pipeline, operation, ComparisonSystem::Hydracache)?;
                let redis_rps = case_rps(repeat, *pipeline, operation, ComparisonSystem::Redis)?;
                if hydra <= 0.0 || redis_rps <= 0.0 || !hydra.is_finite() || !redis_rps.is_finite()
                {
                    return Err(RedisComparisonError::Evidence(
                        "W8 throughput samples must be finite and positive".to_owned(),
                    ));
                }
                hydracache.push(hydra);
                redis.push(redis_rps);
                let ratio = hydra / redis_rps;
                match repeat.order {
                    ExecutionOrder::HydracacheThenRedis => ratio_hydra_first.push(ratio),
                    ExecutionOrder::RedisThenHydracache => ratio_redis_first.push(ratio),
                }
            }
            if ratio_hydra_first.len() < 2 || ratio_redis_first.len() < 2 {
                return Err(RedisComparisonError::Evidence(
                    "alternating W8 repeats require at least two observations in both AB and BA order"
                        .to_owned(),
                ));
            }
            let ratios = hydracache
                .iter()
                .zip(&redis)
                .map(|(hydra, redis)| hydra / redis)
                .collect::<Vec<_>>();
            let hydra_spread = robust_spread(&hydracache);
            let redis_spread = robust_spread(&redis);
            let ratio_spread = robust_spread(&ratios);
            let hydra_first_median = median(&ratio_hydra_first);
            let redis_first_median = median(&ratio_redis_first);
            let order_bias_ratio = if hydra_first_median > 0.0 && redis_first_median > 0.0 {
                (hydra_first_median.max(redis_first_median)
                    - hydra_first_median.min(redis_first_median))
                    / hydra_first_median.min(redis_first_median)
            } else {
                f64::INFINITY
            };
            let mut reasons = BTreeSet::new();
            for (label, spread) in [
                ("HydraCache", hydra_spread),
                ("Redis", redis_spread),
                ("paired ratio", ratio_spread),
                ("AB/BA order bias", order_bias_ratio),
            ] {
                if !spread.is_finite() || spread > scenario.max_robust_spread_ratio {
                    reasons.insert(format!(
                        "pipeline {pipeline} operation {operation} {label} spread {spread:.6} exceeds {:.6}",
                        scenario.max_robust_spread_ratio
                    ));
                }
            }
            output.push(ComparisonAggregate {
                pipeline: *pipeline,
                operation: operation.to_owned(),
                ratio_definition: "hydracache_requests_per_second / redis_requests_per_second"
                    .to_owned(),
                hydracache_requests_per_second: hydracache.clone(),
                redis_requests_per_second: redis.clone(),
                hydracache_over_redis_ratio: ratios.clone(),
                hydracache_median_requests_per_second: median(&hydracache),
                redis_median_requests_per_second: median(&redis),
                median_hydracache_over_redis_ratio: median(&ratios),
                hydracache_robust_spread_ratio: hydra_spread,
                redis_robust_spread_ratio: redis_spread,
                ratio_robust_spread_ratio: ratio_spread,
                median_ratio_hydracache_first: hydra_first_median,
                median_ratio_redis_first: redis_first_median,
                order_bias_ratio,
                stable: reasons.is_empty(),
                stability_reasons: reasons.into_iter().collect(),
            });
        }
    }
    Ok(output)
}

fn case_rps(
    repeat: &ComparisonRepeatEvidence,
    pipeline: u32,
    operation: &str,
    system: ComparisonSystem,
) -> Result<f64, RedisComparisonError> {
    repeat
        .cases
        .iter()
        .find(|case| case.pipeline == pipeline && case.system == system)
        .and_then(|case| case.rows.iter().find(|row| row.name == operation))
        .map(RedisBenchmarkCsvRow::requests_per_second_f64)
        .ok_or_else(|| {
            RedisComparisonError::Evidence(format!(
                "repeat {} has no {system:?}/pipeline-{pipeline}/{operation} sample",
                repeat.repeat
            ))
        })
}

fn aggregate_lists_equal(left: &[ComparisonAggregate], right: &[ComparisonAggregate]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter().zip(right).all(|(left, right)| {
        left.pipeline == right.pipeline
            && left.operation == right.operation
            && left.ratio_definition == right.ratio_definition
            && floats_equal(
                &left.hydracache_requests_per_second,
                &right.hydracache_requests_per_second,
            )
            && floats_equal(
                &left.redis_requests_per_second,
                &right.redis_requests_per_second,
            )
            && floats_equal(
                &left.hydracache_over_redis_ratio,
                &right.hydracache_over_redis_ratio,
            )
            && same_f64(
                left.hydracache_median_requests_per_second,
                right.hydracache_median_requests_per_second,
            )
            && same_f64(
                left.redis_median_requests_per_second,
                right.redis_median_requests_per_second,
            )
            && same_f64(
                left.median_hydracache_over_redis_ratio,
                right.median_hydracache_over_redis_ratio,
            )
            && same_f64(
                left.hydracache_robust_spread_ratio,
                right.hydracache_robust_spread_ratio,
            )
            && same_f64(
                left.redis_robust_spread_ratio,
                right.redis_robust_spread_ratio,
            )
            && same_f64(
                left.ratio_robust_spread_ratio,
                right.ratio_robust_spread_ratio,
            )
            && same_f64(
                left.median_ratio_hydracache_first,
                right.median_ratio_hydracache_first,
            )
            && same_f64(
                left.median_ratio_redis_first,
                right.median_ratio_redis_first,
            )
            && same_f64(left.order_bias_ratio, right.order_bias_ratio)
            && left.stable == right.stable
            && left.stability_reasons == right.stability_reasons
    })
}

fn raw_command_matches(
    evidence: &RawCommandEvidence,
    identity: &ExecutableIdentity,
    argv: &[String],
) -> bool {
    evidence.validate_integrity()
        && &evidence.executable == identity
        && evidence.argv == argv
        && evidence.execution_environment == COMMAND_ENVIRONMENT
}

fn floats_equal(left: &[f64], right: &[f64]) -> bool {
    left.len() == right.len() && left.iter().zip(right).all(|(a, b)| same_f64(*a, *b))
}

fn same_f64(left: f64, right: f64) -> bool {
    left.to_bits() == right.to_bits()
}

fn median(samples: &[f64]) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    if sorted.len().is_multiple_of(2) {
        let upper = sorted.len() / 2;
        (sorted[upper - 1] + sorted[upper]) / 2.0
    } else {
        sorted[sorted.len() / 2]
    }
}

fn robust_spread(samples: &[f64]) -> f64 {
    let median = median(samples);
    let minimum = samples.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum = samples.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if median > 0.0 {
        (maximum - minimum) / median
    } else {
        f64::INFINITY
    }
}

#[derive(Debug, Clone)]
struct PreparedTools {
    redis_benchmark: ResolvedExternalTool,
    redis_benchmark_version: String,
    redis_benchmark_version_probe: RawCommandEvidence,
    docker: ResolvedExternalTool,
    docker_version: String,
    docker_version_probe: RawCommandEvidence,
}

enum ToolPreparation {
    Ready(Box<PreparedTools>),
    Skip(ComparisonSkipLoud),
}

/// Execute the full W8 reference measurement. HydraCache is launched from the
/// exact prebuild manifest, while Redis is launched from the immutable OCI
/// reference in the committed scenario. The canonical four-artifact W3 suite
/// must already exist; a smoke report, missing suite seal, live predecessor
/// PID, or caller-authored endpoint cannot enter this path.
pub async fn run_same_box_redis_comparison(
    repo_root: &Path,
    w3_artifacts: &W3ReferenceArtifactSet,
    context: &ValidatedRespReferenceContext,
    external_tool_prebuild: &ExternalToolPrebuildReceipt,
    run_mode: RedisComparisonRunMode,
) -> Result<RedisComparisonOutcome, RedisComparisonError> {
    let canonical_root = fs::canonicalize(repo_root).map_err(|error| {
        RedisComparisonError::Prerequisite(format!(
            "unable to canonicalize repository root {}: {error}",
            repo_root.display()
        ))
    })?;
    if canonical_root != context.repo_root {
        return Err(RedisComparisonError::Prerequisite(format!(
            "W8 repository root {} differs from the validated W3/W7 root {}",
            canonical_root.display(),
            context.repo_root.display()
        )));
    }
    let scenario = RedisComparisonScenario::load(&repo_root.join(W8_SCENARIO_RELATIVE_PATH))?;
    if run_mode == RedisComparisonRunMode::MandatoryReference
        && std::env::var("HYDRACACHE_RUN_PERF_RESP").as_deref() != Ok("1")
    {
        return Err(RedisComparisonError::Prerequisite(
            "mandatory W8 evidence requires HYDRACACHE_RUN_PERF_RESP=1".to_owned(),
        ));
    }
    context
        .verify_binaries_unchanged()
        .map_err(|error| RedisComparisonError::Prerequisite(error.to_string()))?;
    let prepared = prepare_tools(
        repo_root,
        &scenario,
        context,
        external_tool_prebuild,
        run_mode,
    )?;
    let prepared = match prepared {
        ToolPreparation::Ready(prepared) => *prepared,
        ToolPreparation::Skip(skip) => {
            return Ok(RedisComparisonOutcome::SkippedLoud(skip));
        }
    };
    let w3_reference =
        W3ReferenceBinding::load(repo_root, w3_artifacts, context, external_tool_prebuild)?;

    let ports = RespReferencePorts::select_available()
        .map_err(|error| RedisComparisonError::Prerequisite(error.to_string()))?;
    let launch = RespDaemonLaunch::for_repeat(repo_root, W8_DAEMON_REPEAT_INDEX, ports);
    let daemon = start_reference_daemon(context, &launch)
        .await
        .map_err(|error| RedisComparisonError::Prerequisite(error.to_string()))?;
    let capability = daemon.endpoint_capability().clone();
    let capability_sha256 = daemon
        .endpoint_capability_digest()
        .map_err(|error| RedisComparisonError::Prerequisite(error.to_string()))?;
    validate_live_hydracache_binding(context, &capability, &w3_reference)?;
    let hydra_endpoint = daemon.resp_endpoint();
    let blocking_scenario = scenario.clone();
    let blocking_prepared = prepared.clone();
    let blocking = tokio::task::spawn_blocking(move || {
        run_blocking_comparison(&blocking_scenario, &blocking_prepared, hydra_endpoint)
    })
    .await
    .map_err(|error| RedisComparisonError::Process {
        phase: "blocking-worker".to_owned(),
        detail: error.to_string(),
    })?;
    let lifecycle = daemon
        .stop()
        .await
        .map_err(|error| RedisComparisonError::Prerequisite(error.to_string()));
    let measured = blocking?;
    let lifecycle = lifecycle?;
    context
        .verify_binaries_unchanged()
        .map_err(|error| RedisComparisonError::Prerequisite(error.to_string()))?;

    let aggregates = derive_aggregates(&measured.repeats, &scenario)?;
    let mut stability_reasons = aggregates
        .iter()
        .flat_map(|aggregate| aggregate.stability_reasons.iter().cloned())
        .collect::<Vec<_>>();
    stability_reasons.sort();
    stability_reasons.dedup();
    let measurements_stable = stability_reasons.is_empty();
    let ship_evidence_eligible =
        run_mode == RedisComparisonRunMode::MandatoryReference && measurements_stable;
    let report = SameBoxRedisComparisonReport {
        schema_version: W8_REPORT_SCHEMA_VERSION,
        release: "0.67.0".to_owned(),
        report_id: "same-box-redis-vs-hydracache-resp-reference-v1".to_owned(),
        scenario_id: scenario.scenario_id.clone(),
        scenario_sha256: scenario.digest(),
        measurement_id: W8_MEASUREMENT_ID.to_owned(),
        methodology: W8_METHOD.to_owned(),
        claim_scope: W8_CLAIM_SCOPE.to_owned(),
        interpretation: W8_INTERPRETATION.to_owned(),
        run_mode,
        source: context.source.clone(),
        build: context.build.clone(),
        runner_profile: context.profile.clone(),
        observed_runner: context.runner.clone(),
        host_fingerprint: context.runner.fingerprint.clone(),
        w3_reference,
        hydracache_endpoint_capability: capability,
        hydracache_endpoint_capability_sha256: capability_sha256,
        hydracache_daemon_lifecycle: lifecycle,
        redis_benchmark_identity: ExecutableIdentity::from(&prepared.redis_benchmark),
        redis_benchmark_prebuild_receipt_sha256: external_tool_prebuild.receipt_sha256.clone(),
        redis_benchmark_version: prepared.redis_benchmark_version,
        redis_benchmark_version_probe: prepared.redis_benchmark_version_probe,
        docker: measured.docker,
        repeats: measured.repeats,
        aggregates,
        measurements_stable,
        ship_evidence_eligible,
        stability_reasons,
    };
    let problems = report.validation_problems(&scenario);
    if !problems.is_empty() {
        return Err(RedisComparisonError::Evidence(format!(
            "produced W8 report is invalid: {problems:?}"
        )));
    }
    Ok(RedisComparisonOutcome::Completed(Box::new(report)))
}

/// Execute W8 and publish `compare-redis.json` only after the complete report
/// validates. The destination is create-new and the final name appears via an
/// atomic hard-link, so neither a local skip nor any failed/partial run can
/// leave a promotable report artifact behind.
#[allow(clippy::too_many_arguments)]
pub async fn run_and_write_same_box_redis_comparison(
    repo_root: &Path,
    w3_artifacts: &W3ReferenceArtifactSet,
    context: &ValidatedRespReferenceContext,
    external_tool_prebuild: &ExternalToolPrebuildReceipt,
    run_mode: RedisComparisonRunMode,
    report_path: &Path,
) -> Result<RedisComparisonOutcome, RedisComparisonError> {
    let expected_report = canonical_w8_report_path(repo_root)?;
    if !report_path.is_absolute() || report_path != expected_report {
        return Err(RedisComparisonError::Contract(format!(
            "W8 report must use the canonical create-new destination {}; got {}",
            expected_report.display(),
            report_path.display()
        )));
    }
    if report_path.exists() {
        return Err(RedisComparisonError::Evidence(format!(
            "refusing to overwrite stale W8 evidence {}",
            report_path.display()
        )));
    }
    let outcome = run_same_box_redis_comparison(
        repo_root,
        w3_artifacts,
        context,
        external_tool_prebuild,
        run_mode,
    )
    .await?;
    match &outcome {
        RedisComparisonOutcome::Completed(report) => {
            let scenario = RedisComparisonScenario::load(
                &fs::canonicalize(repo_root)?.join(W8_SCENARIO_RELATIVE_PATH),
            )?;
            let mut bytes = report.to_pretty_json(&scenario)?;
            bytes.push(b'\n');
            write_new_bytes_atomic(report_path, &bytes)?;
        }
        RedisComparisonOutcome::SkippedLoud(skip) => {
            if run_mode == RedisComparisonRunMode::MandatoryReference {
                return Err(RedisComparisonError::Evidence(
                    "mandatory W8 execution returned a local-only skip".to_owned(),
                ));
            }
            eprintln!(
                "hydracache-loadgen: W8 local skip [{}]: {}; no {} artifact was produced",
                skip.code,
                skip.message,
                report_path.display()
            );
            if report_path.exists() {
                return Err(RedisComparisonError::Evidence(
                    "W8 local skip unexpectedly left a report artifact".to_owned(),
                ));
            }
        }
    }
    Ok(outcome)
}

pub fn canonical_w8_report_path(repo_root: &Path) -> Result<PathBuf, RedisComparisonError> {
    let root = fs::canonicalize(repo_root).map_err(|error| {
        RedisComparisonError::Contract(format!(
            "unable to canonicalize W8 repository root {}: {error}",
            repo_root.display()
        ))
    })?;
    Ok(root.join(W8_REPORT_RELATIVE_PATH))
}

fn write_new_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), RedisComparisonError> {
    if !path.is_absolute() || bytes.is_empty() {
        return Err(RedisComparisonError::Evidence(
            "W8 atomic report requires an absolute path and non-empty canonical bytes".to_owned(),
        ));
    }
    if path.exists() {
        return Err(RedisComparisonError::Evidence(format!(
            "refusing to overwrite stale W8 evidence {}",
            path.display()
        )));
    }
    let parent = path.parent().ok_or_else(|| {
        RedisComparisonError::Evidence("W8 report path has no parent directory".to_owned())
    })?;
    fs::create_dir_all(parent)?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            RedisComparisonError::Evidence("W8 report file name must be UTF-8".to_owned())
        })?;
    let sequence = W8_ARTIFACT_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    let temporary = parent.join(format!(
        ".{name}.{}.{}-atomic.tmp",
        std::process::id(),
        sequence
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::hard_link(&temporary, path).map_err(|error| {
            RedisComparisonError::Evidence(format!(
                "cannot atomically publish create-new W8 report {}: {error}",
                path.display()
            ))
        })?;
        fs::remove_file(&temporary)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn validate_live_hydracache_binding(
    context: &ValidatedRespReferenceContext,
    capability: &RespEndpointCapability,
    w3: &W3ReferenceBinding,
) -> Result<(), RedisComparisonError> {
    let exact = capability.digest().is_ok()
        && capability.source_commit == context.source.git_commit
        && capability.prebuild_manifest_sha256 == context.manifest_sha256
        && capability.prebuild_contract_digest == context.build.prebuild_contract_digest
        && capability.server_binary_sha256 == context.server.sha256
        && capability.loadgen_binary_sha256 == context.loadgen.sha256
        && w3.source == context.source
        && w3.build == context.build
        && w3.runner_profile == context.profile.name
        && w3.observed_runner_fingerprint == context.runner.fingerprint
        && capability.pid != w3.archived_pid
        && capability.started_unix_nanos > w3.archived_started_unix_nanos;
    if !exact {
        return Err(RedisComparisonError::Prerequisite(
            "live W8 HydraCache endpoint is not the exact W3/W7 source, build, runner, and binary context"
                .to_owned(),
        ));
    }
    Ok(())
}

fn prepare_tools(
    repo_root: &Path,
    scenario: &RedisComparisonScenario,
    context: &ValidatedRespReferenceContext,
    receipt: &ExternalToolPrebuildReceipt,
    run_mode: RedisComparisonRunMode,
) -> Result<ToolPreparation, RedisComparisonError> {
    let platform = current_platform_key();
    if platform != "linux-x86_64-gnu" {
        return match run_mode {
            RedisComparisonRunMode::LocalInformational => Ok(ToolPreparation::Skip(
                ComparisonSkipLoud {
                    code: "w8-platform-local-skip-loud".to_owned(),
                    message: format!(
                        "W8 has an immutable Redis/tool contract only for linux-x86_64-gnu; observed {platform}"
                    ),
                    missing_program: "docker+redis-benchmark".to_owned(),
                },
            )),
            RedisComparisonRunMode::MandatoryReference => Err(
                RedisComparisonError::Prerequisite(format!(
                    "mandatory W8 runner platform must be linux-x86_64-gnu; observed {platform}"
                )),
            ),
        };
    }
    let registry = ExternalToolProvenanceRegistry::load(
        &repo_root.join(REDIS_BENCHMARK_PROVENANCE_REGISTRY_PATH),
    )
    .map_err(|error| RedisComparisonError::Prerequisite(error.to_string()))?;
    let provenance = registry
        .approved_entry(platform, &scenario.tool.required_provenance_id)
        .ok_or_else(|| {
            RedisComparisonError::Prerequisite(
                "W8 has no approved exact redis-benchmark provenance row".to_owned(),
            )
        })?;
    let payload = &receipt.payload;
    let receipt_exact = receipt.receipt_sha256 == typed_digest(payload)
        && payload.schema_version == 1
        && payload.platform_key == platform
        && payload.provenance_id == scenario.tool.required_provenance_id
        && payload.provenance_registry_sha256 == registry.digest()
        && payload.source_archive_sha256.as_deref()
            == provenance.provenance.source_archive_sha256()
        && payload.tool_binary_id == scenario.tool.program
        && payload.tool_canonical_path.is_absolute()
        && is_sha256(&payload.tool_binary_sha256)
        && payload.prebuild_manifest_sha256 == context.manifest_sha256;
    if !receipt_exact {
        return Err(RedisComparisonError::Prerequisite(
            "redis-benchmark receipt is not the exact W3 external-tool prebuild identity"
                .to_owned(),
        ));
    }
    let executor = SystemToolExecutor;
    let redis_benchmark = match executor
        .resolve_exact(&scenario.tool.program, &payload.tool_canonical_path)
    {
        Ok(tool) => tool,
        Err(error)
            if error.kind == LaunchErrorKind::MissingProgram
                && run_mode == RedisComparisonRunMode::LocalInformational =>
        {
            return Ok(ToolPreparation::Skip(ComparisonSkipLoud {
                code: "w8-redis-benchmark-local-skip-loud".to_owned(),
                message: format!(
                    "exact receipt-bound redis-benchmark is unavailable; no comparison artifact was produced: {}",
                    error.message
                ),
                missing_program: scenario.tool.program.clone(),
            }));
        }
        Err(error) => {
            return Err(RedisComparisonError::Process {
                phase: "redis-benchmark-resolution".to_owned(),
                detail: error.message,
            })
        }
    };
    if redis_benchmark.binary_sha256 != payload.tool_binary_sha256 {
        return Err(RedisComparisonError::Prerequisite(
            "redis-benchmark changed after the W3 prebuild receipt was sealed".to_owned(),
        ));
    }
    let docker = match executor.resolve(&scenario.docker.program) {
        Ok(tool) => tool,
        Err(error)
            if error.kind == LaunchErrorKind::MissingProgram
                && run_mode == RedisComparisonRunMode::LocalInformational =>
        {
            return Ok(ToolPreparation::Skip(ComparisonSkipLoud {
                code: "w8-docker-local-skip-loud".to_owned(),
                message: format!(
                    "Docker is unavailable; no comparison artifact was produced: {}",
                    error.message
                ),
                missing_program: scenario.docker.program.clone(),
            }));
        }
        Err(error) => {
            return Err(RedisComparisonError::Process {
                phase: "docker-resolution".to_owned(),
                detail: error.message,
            })
        }
    };
    let redis_version_probe = execute_checked(
        &redis_benchmark,
        &scenario.tool.version_args,
        Duration::from_secs(scenario.version_timeout_seconds),
        scenario,
        "redis-benchmark-version",
    )?;
    if !redis_version_probe.stderr.is_empty() {
        return Err(RedisComparisonError::Process {
            phase: "redis-benchmark-version".to_owned(),
            detail: format!("unexpected stderr: {:?}", redis_version_probe.stderr),
        });
    }
    let redis_version = exact_single_line(&redis_version_probe.stdout).ok_or_else(|| {
        RedisComparisonError::Process {
            phase: "redis-benchmark-version".to_owned(),
            detail: "version output is not exactly one non-empty line".to_owned(),
        }
    })?;
    if redis_version != scenario.tool.expected_version {
        return Err(RedisComparisonError::Prerequisite(format!(
            "redis-benchmark version mismatch: expected {:?}, observed {redis_version:?}",
            scenario.tool.expected_version
        )));
    }
    let docker_version_probe = execute_checked(
        &docker,
        &scenario.docker.version_args,
        Duration::from_secs(scenario.version_timeout_seconds),
        scenario,
        "docker-version",
    )?;
    let docker_version = exact_single_line(&docker_version_probe.stdout).ok_or_else(|| {
        RedisComparisonError::Process {
            phase: "docker-version".to_owned(),
            detail: "Docker version output is not exactly one non-empty line".to_owned(),
        }
    })?;
    if !docker_version.starts_with(&scenario.docker.version_prefix) {
        return Err(RedisComparisonError::Prerequisite(format!(
            "unexpected Docker version output {docker_version:?}"
        )));
    }
    verify_tool_unchanged(&redis_benchmark)?;
    verify_tool_unchanged(&docker)?;
    Ok(ToolPreparation::Ready(Box::new(PreparedTools {
        redis_benchmark,
        redis_benchmark_version: redis_version,
        redis_benchmark_version_probe: redis_version_probe,
        docker,
        docker_version,
        docker_version_probe,
    })))
}

struct BlockingComparison {
    docker: DockerRedisEvidence,
    repeats: Vec<ComparisonRepeatEvidence>,
}

struct RunningRedis {
    docker: ResolvedExternalTool,
    docker_version: String,
    version_probe: RawCommandEvidence,
    pull: RawCommandEvidence,
    manifest_inspect: RawCommandEvidence,
    image_inspect: RawCommandEvidence,
    container_run: RawCommandEvidence,
    port_inspect: RawCommandEvidence,
    container_inspect: RawCommandEvidence,
    container_name: String,
    container_id: String,
    endpoint: SocketAddr,
    readiness_request_sha256: String,
    readiness_response_sha256: String,
    readiness_attempts: u32,
    image: DockerImageIdentity,
    scenario: RedisComparisonScenario,
    stopped: bool,
}

/// Armed before `docker run`: even a timeout, malformed container id, or any
/// failure before `RunningRedis` is fully constructed gets a name-scoped
/// stop/remove attempt.
struct PendingRedisContainer {
    docker_path: PathBuf,
    container_name: String,
    armed: bool,
}

impl PendingRedisContainer {
    fn new(docker: &ResolvedExternalTool, container_name: &str) -> Self {
        Self {
            docker_path: docker.canonical_path.clone(),
            container_name: container_name.to_owned(),
            armed: true,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for PendingRedisContainer {
    fn drop(&mut self) {
        if self.armed {
            cleanup_redis_container(&self.docker_path, &self.container_name);
            self.armed = false;
        }
    }
}

impl RunningRedis {
    fn stop(mut self) -> Result<DockerRedisEvidence, RedisComparisonError> {
        let argv = vec![
            "stop".to_owned(),
            "--time".to_owned(),
            "10".to_owned(),
            self.container_name.clone(),
        ];
        let stop = execute_checked(
            &self.docker,
            &argv,
            Duration::from_secs(30),
            &self.scenario,
            "docker-stop",
        )?;
        self.stopped = true;
        verify_tool_unchanged(&self.docker)?;
        Ok(DockerRedisEvidence {
            docker_identity: ExecutableIdentity::from(&self.docker),
            docker_version: self.docker_version.clone(),
            version_probe: self.version_probe.clone(),
            pull: self.pull.clone(),
            manifest_inspect: self.manifest_inspect.clone(),
            image_inspect: self.image_inspect.clone(),
            container_run: self.container_run.clone(),
            port_inspect: self.port_inspect.clone(),
            container_inspect: self.container_inspect.clone(),
            container_stop: stop,
            container_name: self.container_name.clone(),
            container_id: self.container_id.clone(),
            redis_endpoint: self.endpoint,
            readiness_request_sha256: self.readiness_request_sha256.clone(),
            readiness_response_sha256: self.readiness_response_sha256.clone(),
            readiness_attempts: self.readiness_attempts,
            image: self.image.clone(),
        })
    }
}

impl Drop for RunningRedis {
    fn drop(&mut self) {
        if self.stopped {
            return;
        }
        cleanup_redis_container(&self.docker.canonical_path, &self.container_name);
        self.stopped = true;
    }
}

fn cleanup_redis_container(docker_path: &Path, container_name: &str) {
    let run = |args: &[&str]| {
        std::process::Command::new(docker_path)
            .args(args)
            .env_clear()
            .env("LANG", "C")
            .env("LC_ALL", "C")
            .env("TZ", "UTC")
            .output()
    };
    let stopped =
        run(&["stop", "--time", "1", container_name]).is_ok_and(|output| output.status.success());
    if !stopped {
        let _ = run(&["rm", "--force", container_name]);
    }
}

fn run_blocking_comparison(
    scenario: &RedisComparisonScenario,
    tools: &PreparedTools,
    hydracache_endpoint: SocketAddr,
) -> Result<BlockingComparison, RedisComparisonError> {
    verify_tool_unchanged(&tools.redis_benchmark)?;
    verify_tool_unchanged(&tools.docker)?;
    let redis = start_redis_container(scenario, tools)?;
    let redis_endpoint = redis.endpoint;
    let mut repeats = Vec::with_capacity(usize::from(scenario.repeats));
    for repeat in 1..=scenario.repeats {
        let order = ExecutionOrder::for_repeat(repeat);
        let mut cases = Vec::with_capacity(scenario.pipelines.len() * 2);
        for pipeline in &scenario.pipelines {
            for system in order.systems() {
                let endpoint = match system {
                    ComparisonSystem::Hydracache => hydracache_endpoint,
                    ComparisonSystem::Redis => redis_endpoint,
                };
                verify_tool_unchanged(&tools.redis_benchmark)?;
                let initial_state = reset_and_preload(endpoint, scenario)?;
                let argv = scenario.benchmark_argv(endpoint, *pipeline);
                let process = execute_checked(
                    &tools.redis_benchmark,
                    &argv,
                    Duration::from_secs(scenario.case_timeout_seconds),
                    scenario,
                    &format!("benchmark-repeat-{repeat}-pipeline-{pipeline}-{system:?}"),
                )?;
                verify_tool_unchanged(&tools.redis_benchmark)?;
                if !process.stderr.is_empty() {
                    return Err(RedisComparisonError::Process {
                        phase: format!("benchmark-repeat-{repeat}-pipeline-{pipeline}-{system:?}"),
                        detail: format!("unexpected stderr: {:?}", process.stderr),
                    });
                }
                let expected = ["GET".to_owned(), "SET".to_owned()];
                let rows = parse_redis_benchmark_csv(process.stdout.as_bytes(), &expected)
                    .map_err(|error| RedisComparisonError::Evidence(format!(
                        "invalid redis-benchmark CSV for repeat {repeat}, pipeline {pipeline}, {system:?}: {error}"
                    )))?;
                cases.push(SystemCaseEvidence {
                    system,
                    pipeline: *pipeline,
                    endpoint,
                    initial_state,
                    process,
                    rows,
                });
            }
        }
        repeats.push(ComparisonRepeatEvidence {
            repeat,
            order,
            cases,
        });
    }
    verify_tool_unchanged(&tools.redis_benchmark)?;
    let docker = redis.stop()?;
    Ok(BlockingComparison { docker, repeats })
}

fn start_redis_container(
    scenario: &RedisComparisonScenario,
    tools: &PreparedTools,
) -> Result<RunningRedis, RedisComparisonError> {
    let image_reference = scenario.image_reference();
    let pull_argv = vec![
        "pull".to_owned(),
        "--platform".to_owned(),
        scenario.docker.platform.clone(),
        image_reference.clone(),
    ];
    let pull = execute_checked(
        &tools.docker,
        &pull_argv,
        Duration::from_secs(900),
        scenario,
        "docker-pull-pinned-redis",
    )?;
    verify_tool_unchanged(&tools.docker)?;
    let manifest_argv = vec![
        "manifest".to_owned(),
        "inspect".to_owned(),
        "--verbose".to_owned(),
        image_reference.clone(),
    ];
    let manifest_inspect = execute_checked(
        &tools.docker,
        &manifest_argv,
        Duration::from_secs(120),
        scenario,
        "docker-manifest-inspect",
    )?;
    if !manifest_output_contains_platform_digest(
        &manifest_inspect.stdout,
        &scenario.docker.image_platform_manifest_digest,
        "linux",
        "amd64",
    ) {
        return Err(RedisComparisonError::Prerequisite(
            "Docker registry manifest does not contain the committed linux/amd64 Redis manifest digest"
                .to_owned(),
        ));
    }
    let inspect_argv = vec![
        "image".to_owned(),
        "inspect".to_owned(),
        image_reference.clone(),
    ];
    let image_inspect = execute_checked(
        &tools.docker,
        &inspect_argv,
        Duration::from_secs(30),
        scenario,
        "docker-image-inspect",
    )?;
    let inspected = parse_image_inspect(&image_inspect.stdout, scenario)?;
    let container_name = unique_container_name()?;
    let mut run_argv = vec![
        "run".to_owned(),
        "--detach".to_owned(),
        "--rm".to_owned(),
        "--name".to_owned(),
        container_name.clone(),
        "--platform".to_owned(),
        scenario.docker.platform.clone(),
        "--publish".to_owned(),
        format!("127.0.0.1::{}/tcp", scenario.docker.container_port),
        image_reference.clone(),
    ];
    run_argv.extend(scenario.docker.server_argv.clone());
    let mut pending_container = PendingRedisContainer::new(&tools.docker, &container_name);
    let container_run = execute_checked(
        &tools.docker,
        &run_argv,
        Duration::from_secs(60),
        scenario,
        "docker-run-pinned-redis",
    )?;
    let container_id =
        exact_single_line(&container_run.stdout).ok_or_else(|| RedisComparisonError::Process {
            phase: "docker-run-pinned-redis".to_owned(),
            detail: "docker run did not return exactly one container id".to_owned(),
        })?;
    if container_id.len() != 64
        || !container_id
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(RedisComparisonError::Process {
            phase: "docker-run-pinned-redis".to_owned(),
            detail: format!("invalid container id {container_id:?}"),
        });
    }
    let mut guard = RunningRedis {
        docker: tools.docker.clone(),
        docker_version: tools.docker_version.clone(),
        version_probe: tools.docker_version_probe.clone(),
        pull,
        manifest_inspect,
        image_inspect,
        container_run,
        port_inspect: empty_raw_evidence(&tools.docker),
        container_inspect: empty_raw_evidence(&tools.docker),
        container_name,
        container_id,
        endpoint: SocketAddr::from((Ipv4Addr::LOCALHOST, 1)),
        readiness_request_sha256: String::new(),
        readiness_response_sha256: String::new(),
        readiness_attempts: 0,
        image: DockerImageIdentity {
            requested_reference: image_reference,
            image_name: scenario.docker.image_name.clone(),
            image_version: scenario.docker.image_version.clone(),
            index_digest: scenario.docker.image_index_digest.clone(),
            platform: scenario.docker.platform.clone(),
            platform_manifest_digest: scenario.docker.image_platform_manifest_digest.clone(),
            local_image_id: inspected.local_image_id,
            container_image_id: String::new(),
            repo_digests: inspected.repo_digests,
        },
        scenario: scenario.clone(),
        stopped: false,
    };
    pending_container.disarm();
    let port_argv = vec![
        "port".to_owned(),
        guard.container_name.clone(),
        format!("{}/tcp", scenario.docker.container_port),
    ];
    guard.port_inspect = execute_checked(
        &tools.docker,
        &port_argv,
        Duration::from_secs(30),
        scenario,
        "docker-port-inspect",
    )?;
    guard.endpoint = parse_published_endpoint(&guard.port_inspect.stdout)?;
    let container_inspect_argv = vec![
        "container".to_owned(),
        "inspect".to_owned(),
        guard.container_name.clone(),
    ];
    guard.container_inspect = execute_checked(
        &tools.docker,
        &container_inspect_argv,
        Duration::from_secs(30),
        scenario,
        "docker-container-inspect",
    )?;
    let container_image_id = parse_container_image_id(&guard.container_inspect.stdout)?;
    if container_image_id != guard.image.local_image_id {
        return Err(RedisComparisonError::Prerequisite(format!(
            "container image id {container_image_id} differs from pinned local image id {}",
            guard.image.local_image_id
        )));
    }
    guard.image.container_image_id = container_image_id;
    let readiness = wait_for_redis_ready(
        guard.endpoint,
        Duration::from_secs(scenario.container_start_timeout_seconds),
    )?;
    guard.readiness_request_sha256 = sha256(b"*1\r\n$4\r\nPING\r\n");
    guard.readiness_response_sha256 = sha256(b"+PONG\r\n");
    guard.readiness_attempts = readiness;
    verify_tool_unchanged(&tools.docker)?;
    Ok(guard)
}

struct ImageInspectFacts {
    local_image_id: String,
    repo_digests: Vec<String>,
}

fn manifest_output_contains_platform_digest(
    stdout: &str,
    digest: &str,
    os: &str,
    architecture: &str,
) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(stdout) else {
        return false;
    };
    fn walk(value: &serde_json::Value, digest: &str, os: &str, architecture: &str) -> bool {
        match value {
            serde_json::Value::Array(values) => values
                .iter()
                .any(|value| walk(value, digest, os, architecture)),
            serde_json::Value::Object(map) => {
                let direct_digest = map
                    .get("digest")
                    .or_else(|| map.get("Digest"))
                    .and_then(serde_json::Value::as_str);
                let platform = map.get("platform").or_else(|| map.get("Platform"));
                let direct_match = direct_digest == Some(digest)
                    && platform.is_some_and(|platform| {
                        let observed_os = platform
                            .get("os")
                            .or_else(|| platform.get("OS"))
                            .and_then(serde_json::Value::as_str);
                        let observed_arch = platform
                            .get("architecture")
                            .or_else(|| platform.get("Architecture"))
                            .and_then(serde_json::Value::as_str);
                        observed_os == Some(os) && observed_arch == Some(architecture)
                    });
                direct_match
                    || map
                        .values()
                        .any(|value| walk(value, digest, os, architecture))
            }
            _ => false,
        }
    }
    walk(&value, digest, os, architecture)
}

fn parse_image_inspect(
    stdout: &str,
    scenario: &RedisComparisonScenario,
) -> Result<ImageInspectFacts, RedisComparisonError> {
    let value: serde_json::Value = serde_json::from_str(stdout).map_err(|error| {
        RedisComparisonError::Evidence(format!("invalid docker image inspect JSON: {error}"))
    })?;
    let rows = value.as_array().ok_or_else(|| {
        RedisComparisonError::Evidence("docker image inspect root is not an array".to_owned())
    })?;
    if rows.len() != 1 {
        return Err(RedisComparisonError::Evidence(
            "docker image inspect must return exactly one image".to_owned(),
        ));
    }
    let row = &rows[0];
    let id = row
        .get("Id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            RedisComparisonError::Evidence("docker image inspect has no Id".to_owned())
        })?;
    let architecture = row.get("Architecture").and_then(serde_json::Value::as_str);
    let os = row.get("Os").and_then(serde_json::Value::as_str);
    let repo_digests = row
        .get("RepoDigests")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            RedisComparisonError::Evidence("docker image inspect has no RepoDigests".to_owned())
        })?
        .iter()
        .map(|value| {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                RedisComparisonError::Evidence(
                    "docker image RepoDigests contains a non-string".to_owned(),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !is_oci_digest(id)
        || architecture != Some("amd64")
        || os != Some("linux")
        || !repo_digests
            .iter()
            .any(|digest| digest == &scenario.image_reference())
    {
        return Err(RedisComparisonError::Prerequisite(format!(
            "local Redis image does not match pinned linux/amd64 index: id={id:?}, arch={architecture:?}, os={os:?}, repo_digests={repo_digests:?}"
        )));
    }
    Ok(ImageInspectFacts {
        local_image_id: id.to_owned(),
        repo_digests,
    })
}

fn parse_container_image_id(stdout: &str) -> Result<String, RedisComparisonError> {
    let value: serde_json::Value = serde_json::from_str(stdout).map_err(|error| {
        RedisComparisonError::Evidence(format!("invalid docker container inspect JSON: {error}"))
    })?;
    let rows = value.as_array().ok_or_else(|| {
        RedisComparisonError::Evidence("docker container inspect root is not an array".to_owned())
    })?;
    if rows.len() != 1 {
        return Err(RedisComparisonError::Evidence(
            "docker container inspect must return exactly one container".to_owned(),
        ));
    }
    let image = rows[0]
        .get("Image")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            RedisComparisonError::Evidence("container inspect has no Image id".to_owned())
        })?;
    if !is_oci_digest(image) {
        return Err(RedisComparisonError::Evidence(format!(
            "container Image is not a sha256 identity: {image:?}"
        )));
    }
    Ok(image.to_owned())
}

fn parse_published_endpoint(stdout: &str) -> Result<SocketAddr, RedisComparisonError> {
    let line = exact_single_line(stdout).ok_or_else(|| {
        RedisComparisonError::Evidence(
            "docker port must return exactly one published endpoint".to_owned(),
        )
    })?;
    let endpoint = line.parse::<SocketAddr>().map_err(|error| {
        RedisComparisonError::Evidence(format!(
            "docker port returned invalid endpoint {line:?}: {error}"
        ))
    })?;
    if endpoint.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST) || endpoint.port() == 0 {
        return Err(RedisComparisonError::Evidence(format!(
            "Docker Redis must be published only on explicit IPv4 loopback; got {endpoint}"
        )));
    }
    Ok(endpoint)
}

fn wait_for_redis_ready(
    endpoint: SocketAddr,
    timeout: Duration,
) -> Result<u32, RedisComparisonError> {
    let deadline = Instant::now() + timeout;
    let request = b"*1\r\n$4\r\nPING\r\n";
    let mut attempts = 0_u32;
    let mut last_error = String::new();
    while Instant::now() < deadline {
        attempts = attempts.saturating_add(1);
        match exchange_one_resp(endpoint, request, Duration::from_millis(500)) {
            Ok((Resp2Value::Simple(value), raw)) if value == b"PONG" && raw == b"+PONG\r\n" => {
                return Ok(attempts)
            }
            Ok((value, raw)) => {
                last_error = format!("unexpected readiness reply {value:?}, raw={raw:?}")
            }
            Err(error) => last_error = error.to_string(),
        }
        thread::sleep(Duration::from_millis(25));
    }
    Err(RedisComparisonError::Process {
        phase: "redis-container-readiness".to_owned(),
        detail: format!(
            "Redis at {endpoint} did not return exact +PONG within {timeout:?} after {attempts} attempts: {last_error}"
        ),
    })
}

fn reset_and_preload(
    endpoint: SocketAddr,
    scenario: &RedisComparisonScenario,
) -> Result<RespResetPreloadEvidence, RedisComparisonError> {
    let key = scenario.key.as_bytes();
    let payload = vec![b'R'; scenario.payload_bytes as usize];
    let mut request = Vec::new();
    request.extend_from_slice(&encode_resp2_command([b"DEL".as_slice(), key]));
    request.extend_from_slice(&encode_resp2_command([
        b"SET".as_slice(),
        key,
        payload.as_slice(),
    ]));
    request.extend_from_slice(&encode_resp2_command([b"GET".as_slice(), key]));
    let replies = exchange_exact_resp(endpoint, &request, 3, Duration::from_secs(5))?;
    if !matches!(replies[0].0, Resp2Value::Integer(value) if value >= 0)
        || !matches!(&replies[1].0, Resp2Value::Simple(value) if value == b"OK")
        || !matches!(&replies[2].0, Resp2Value::Bulk(Some(value)) if value == &payload)
    {
        return Err(RedisComparisonError::Evidence(format!(
            "RESP reset/preload/verify failed on {endpoint}: {:?}",
            replies.iter().map(|(value, _)| value).collect::<Vec<_>>()
        )));
    }
    Ok(RespResetPreloadEvidence {
        endpoint,
        method: "resp-del-set-get-exact-single-benchmark-key".to_owned(),
        key: scenario.key.clone(),
        payload_bytes: scenario.payload_bytes,
        payload_sha256: sha256(&payload),
        reset_reply_sha256: sha256(&replies[0].1),
        preload_reply_sha256: sha256(&replies[1].1),
        verification_reply_sha256: sha256(&replies[2].1),
        logical_state_sha256: logical_state_digest(key, &payload),
    })
}

fn exchange_one_resp(
    endpoint: SocketAddr,
    request: &[u8],
    timeout: Duration,
) -> Result<(Resp2Value, Vec<u8>), RedisComparisonError> {
    let mut replies = exchange_exact_resp(endpoint, request, 1, timeout)?;
    Ok(replies.remove(0))
}

fn exchange_exact_resp(
    endpoint: SocketAddr,
    request: &[u8],
    expected: usize,
    timeout: Duration,
) -> Result<Vec<(Resp2Value, Vec<u8>)>, RedisComparisonError> {
    let mut stream = TcpStream::connect_timeout(&endpoint, timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    stream.write_all(request)?;
    stream.flush()?;
    let mut buffer = Vec::new();
    let mut offset = 0_usize;
    let mut replies = Vec::with_capacity(expected);
    while replies.len() < expected {
        while offset < buffer.len() && replies.len() < expected {
            match parse_resp2(&buffer[offset..], Resp2Limits::default()).map_err(|error| {
                RedisComparisonError::Evidence(format!(
                    "invalid RESP reply from {endpoint}: {error}"
                ))
            })? {
                Resp2ParseStatus::Incomplete => break,
                Resp2ParseStatus::Complete { value, consumed } => {
                    let raw = buffer[offset..offset + consumed].to_vec();
                    offset += consumed;
                    replies.push((value, raw));
                }
            }
        }
        if replies.len() == expected {
            break;
        }
        if buffer.len() >= 2 * 1024 * 1024 {
            return Err(RedisComparisonError::Evidence(
                "RESP reset/readiness reply exceeded 2 MiB".to_owned(),
            ));
        }
        let mut chunk = [0_u8; 8192];
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Err(RedisComparisonError::Evidence(format!(
                "RESP reply from {endpoint} was truncated after {} of {expected} values",
                replies.len()
            )));
        }
        buffer.extend_from_slice(&chunk[..read]);
    }
    if offset != buffer.len() {
        return Err(RedisComparisonError::Evidence(format!(
            "RESP endpoint {endpoint} returned surplus bytes after {expected} replies"
        )));
    }
    Ok(replies)
}

fn execute_checked(
    tool: &ResolvedExternalTool,
    argv: &[String],
    timeout: Duration,
    scenario: &RedisComparisonScenario,
    phase: &str,
) -> Result<RawCommandEvidence, RedisComparisonError> {
    let max_stdout_bytes = usize::try_from(scenario.max_stdout_bytes).map_err(|_| {
        RedisComparisonError::Contract("stdout capture limit does not fit usize".to_owned())
    })?;
    let max_stderr_bytes = usize::try_from(scenario.max_stderr_bytes).map_err(|_| {
        RedisComparisonError::Contract("stderr capture limit does not fit usize".to_owned())
    })?;
    let capture = SystemToolExecutor
        .execute(
            tool,
            argv,
            ProcessLimits {
                timeout,
                max_stdout_bytes,
                max_stderr_bytes,
            },
        )
        .map_err(|error| RedisComparisonError::Process {
            phase: phase.to_owned(),
            detail: error.message,
        })?;
    let stdout =
        String::from_utf8(capture.stdout).map_err(|error| RedisComparisonError::Process {
            phase: phase.to_owned(),
            detail: format!("stdout is not UTF-8: {error}"),
        })?;
    let stderr =
        String::from_utf8(capture.stderr).map_err(|error| RedisComparisonError::Process {
            phase: phase.to_owned(),
            detail: format!("stderr is not UTF-8: {error}"),
        })?;
    let exit_code = capture
        .exit_code
        .ok_or_else(|| RedisComparisonError::Process {
            phase: phase.to_owned(),
            detail: "process exited without a numeric status".to_owned(),
        })?;
    if capture.timed_out || exit_code != 0 {
        return Err(RedisComparisonError::Process {
            phase: phase.to_owned(),
            detail: format!(
                "exit_code={exit_code}, timed_out={}, stdout={stdout:?}, stderr={stderr:?}",
                capture.timed_out
            ),
        });
    }
    Ok(RawCommandEvidence {
        executable: ExecutableIdentity::from(tool),
        argv: argv.to_vec(),
        execution_environment: COMMAND_ENVIRONMENT
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        exit_code,
        timed_out: capture.timed_out,
        stdout_bytes: stdout.len() as u64,
        stderr_bytes: stderr.len() as u64,
        stdout_sha256: sha256(stdout.as_bytes()),
        stderr_sha256: sha256(stderr.as_bytes()),
        stdout,
        stderr,
    })
}

fn empty_raw_evidence(tool: &ResolvedExternalTool) -> RawCommandEvidence {
    RawCommandEvidence {
        executable: ExecutableIdentity::from(tool),
        argv: Vec::new(),
        execution_environment: COMMAND_ENVIRONMENT
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        exit_code: -1,
        timed_out: false,
        stdout: String::new(),
        stderr: String::new(),
        stdout_bytes: 0,
        stderr_bytes: 0,
        stdout_sha256: sha256(b""),
        stderr_sha256: sha256(b""),
    }
}

fn verify_tool_unchanged(tool: &ResolvedExternalTool) -> Result<(), RedisComparisonError> {
    let canonical = fs::canonicalize(&tool.canonical_path).map_err(|error| {
        RedisComparisonError::Prerequisite(format!(
            "external executable {} disappeared: {error}",
            tool.canonical_path.display()
        ))
    })?;
    let actual = sha256_file(&canonical)?;
    if canonical != tool.canonical_path || actual != tool.binary_sha256 {
        return Err(RedisComparisonError::Prerequisite(format!(
            "external executable {} changed during W8: expected {}, observed {}",
            tool.canonical_path.display(),
            tool.binary_sha256,
            actual
        )));
    }
    Ok(())
}

fn unique_container_name() -> Result<String, RedisComparisonError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| RedisComparisonError::Prerequisite(error.to_string()))?
        .as_nanos();
    Ok(format!("hydracache-perf-w8-{}-{nanos}", std::process::id()))
}

fn exact_single_line(value: &str) -> Option<String> {
    let normalized = value.strip_suffix('\n').unwrap_or(value);
    let normalized = normalized.strip_suffix('\r').unwrap_or(normalized);
    if normalized.is_empty() || normalized.contains(['\r', '\n']) {
        None
    } else {
        Some(normalized.to_owned())
    }
}

fn encode_bulk_reply(payload: &[u8]) -> Vec<u8> {
    let mut reply = format!("${}\r\n", payload.len()).into_bytes();
    reply.extend_from_slice(payload);
    reply.extend_from_slice(b"\r\n");
    reply
}

fn logical_state_digest(key: &[u8], payload: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"hydracache-w8-same-box-repeat-state-v1");
    hasher.update((key.len() as u64).to_le_bytes());
    hasher.update(key);
    hasher.update((payload.len() as u64).to_le_bytes());
    hasher.update(payload);
    hex_digest(hasher.finalize().as_ref())
}

fn portable_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn typed_digest<T: Serialize>(value: &T) -> String {
    sha256(&serde_json::to_vec(value).expect("typed W8 serialization cannot fail"))
}

fn sha256_file(path: &Path) -> Result<String, RedisComparisonError> {
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
    Ok(hex_digest(hasher.finalize().as_ref()))
}

fn sha256(bytes: &[u8]) -> String {
    hex_digest(Sha256::digest(bytes).as_ref())
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
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

/// Fast falsifiability probe for the W8 comparison boundary. It deliberately
/// supplies both a different host and a tag/mutable image identity. A guard
/// that accepts either defect makes the canary green and is therefore broken.
pub fn w8_boundary_canary_red() -> Result<(), String> {
    let scenario = RedisComparisonScenario::parse_toml(include_str!(
        "../../../docs/testing/perf-scenarios/0.67/compare-redis-v1.toml"
    ))
    .map_err(|error| error.to_string())?;
    let mut unpinned = scenario.clone();
    unpinned.docker.image_index_digest = "redis:7.2.5".to_owned();
    let image_rejected = unpinned.validate().is_err();
    let host_rejected = same_host_contract("runner-a", "runner-b").is_err();
    if image_rejected && host_rejected {
        Err(format!(
            "{W8_CANARY_MARKER} mismatched host and unpinned Redis image were rejected"
        ))
    } else {
        Ok(())
    }
}

fn same_host_contract(left: &str, right: &str) -> Result<(), RedisComparisonError> {
    if left.is_empty() || left != right {
        return Err(RedisComparisonError::Evidence(
            "same-box comparison requires identical non-empty runner fingerprints".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::RespDaemonConfigIdentity;
    use crate::tiers::resp::RespReferenceSuiteReceiptPayload;
    use crate::tiers::resp_reference::RespPingEvidence;

    struct LifecycleFixture {
        root: PathBuf,
        capability: RespEndpointCapability,
        lifecycle: RespDaemonEvidence,
        server: VerifiedBinary,
        loadgen: VerifiedBinary,
    }

    impl Drop for LifecycleFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn temporary_root(label: &str) -> PathBuf {
        let sequence = W8_ARTIFACT_SEQUENCE.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!(
            "hydracache-w8-{label}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::canonicalize(root).unwrap()
    }

    fn lifecycle_fixture() -> LifecycleFixture {
        let root = temporary_root("lifecycle");
        let data_dir = root.join("data");
        fs::create_dir(&data_dir).unwrap();
        let data_dir = fs::canonicalize(data_dir).unwrap();
        let server_path = root.join(if cfg!(windows) {
            "hydracache-server.exe"
        } else {
            "hydracache-server"
        });
        let loadgen_path = root.join(if cfg!(windows) {
            "hydracache-loadgen.exe"
        } else {
            "hydracache-loadgen"
        });
        let stdout_path = root.join("daemon.stdout.log");
        let stderr_path = root.join("daemon.stderr.log");
        fs::write(&server_path, b"server-binary").unwrap();
        fs::write(&loadgen_path, b"loadgen-binary").unwrap();
        fs::write(&stdout_path, b"ready\n").unwrap();
        fs::write(&stderr_path, b"").unwrap();
        let server_path = fs::canonicalize(server_path).unwrap();
        let loadgen_path = fs::canonicalize(loadgen_path).unwrap();
        let stdout_path = fs::canonicalize(stdout_path).unwrap();
        let stderr_path = fs::canonicalize(stderr_path).unwrap();
        let server_sha256 = sha256(b"server-binary");
        let loadgen_sha256 = sha256(b"loadgen-binary");
        let resp_endpoint = SocketAddr::from((Ipv4Addr::LOCALHOST, 31_810));
        let admin_endpoint = SocketAddr::from((Ipv4Addr::LOCALHOST, 31_811));
        let capability = RespEndpointCapability {
            schema_version: 1,
            pid: u32::MAX,
            started_unix_nanos: 1,
            repeat_index: 0,
            direct_prebuilt_exec: true,
            fresh_data_dir: true,
            config: RespDaemonConfigIdentity {
                role: "local".to_owned(),
                listen_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
                cluster_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
                storage_dir: data_dir.clone(),
                admin_enabled: true,
                admin_addr: admin_endpoint,
                redis_enabled: true,
                redis_addr: resp_endpoint,
                redis_auth_required: false,
                rediss_enabled: false,
            },
            selected_endpoint: format!("hydracache-server@{resp_endpoint}"),
            server_binary_sha256: server_sha256.clone(),
            loadgen_binary_sha256: loadgen_sha256.clone(),
            prebuild_manifest_sha256: sha256(b"prebuild"),
            prebuild_contract_digest: sha256(b"contract"),
            source_commit: "a".repeat(40),
        };
        let capability_sha256 = capability.digest().unwrap();
        let lifecycle = RespDaemonEvidence {
            repeat_index: 0,
            pid: capability.pid,
            direct_prebuilt_exec: true,
            server_binary_path: server_path.clone(),
            server_binary_sha256: server_sha256.clone(),
            loadgen_binary_path: loadgen_path.clone(),
            loadgen_binary_sha256: loadgen_sha256.clone(),
            binaries_verified_after_measurement: true,
            resp_endpoint,
            admin_endpoint,
            selected_endpoint: capability.selected_endpoint.clone(),
            endpoint_capability_digest: capability_sha256,
            data_dir,
            readiness: RespPingEvidence {
                request_sha256: sha256(RESP_PING_FRAME),
                response_sha256: sha256(RESP_PONG_FRAME),
                attempts: 1,
                selected_endpoint: resp_endpoint,
                exact_response: RESP_PONG_DISPLAY.to_owned(),
            },
            killed_and_waited: true,
            exit_code: None,
            stdout_log: LogEvidence {
                canonical_path: stdout_path,
                bytes: b"ready\n".len() as u64,
                sha256: sha256(b"ready\n"),
            },
            stderr_log: LogEvidence {
                canonical_path: stderr_path,
                bytes: 0,
                sha256: sha256(b""),
            },
        };
        LifecycleFixture {
            root,
            capability,
            lifecycle,
            server: VerifiedBinary {
                id: "hydracache-server".to_owned(),
                canonical_path: server_path,
                sha256: server_sha256,
            },
            loadgen: VerifiedBinary {
                id: "hydracache-loadgen".to_owned(),
                canonical_path: loadgen_path,
                sha256: loadgen_sha256,
            },
        }
    }

    fn scenario() -> RedisComparisonScenario {
        RedisComparisonScenario::parse_toml(include_str!(
            "../../../docs/testing/perf-scenarios/0.67/compare-redis-v1.toml"
        ))
        .unwrap()
    }

    fn row(name: &str, rps: f64) -> RedisBenchmarkCsvRow {
        RedisBenchmarkCsvRow {
            name: name.to_owned(),
            requests_per_second: format!("{rps:.2}"),
            average_latency_ms: "1.00".to_owned(),
            minimum_latency_ms: "0.50".to_owned(),
            p50_latency_ms: "1.00".to_owned(),
            p95_latency_ms: "1.50".to_owned(),
            p99_latency_ms: "2.00".to_owned(),
            maximum_latency_ms: "3.00".to_owned(),
        }
    }

    fn dummy_process() -> RawCommandEvidence {
        let path = if cfg!(windows) {
            PathBuf::from(r"C:\perf\redis-benchmark.exe")
        } else {
            PathBuf::from("/opt/perf/redis-benchmark")
        };
        RawCommandEvidence {
            executable: ExecutableIdentity {
                logical_name: "redis-benchmark".to_owned(),
                canonical_path: path,
                sha256: sha256(b"tool"),
            },
            argv: Vec::new(),
            execution_environment: COMMAND_ENVIRONMENT
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            exit_code: 0,
            timed_out: false,
            stdout: String::new(),
            stderr: String::new(),
            stdout_bytes: 0,
            stderr_bytes: 0,
            stdout_sha256: sha256(b""),
            stderr_sha256: sha256(b""),
        }
    }

    fn dummy_initial(endpoint: SocketAddr) -> RespResetPreloadEvidence {
        RespResetPreloadEvidence {
            endpoint,
            method: String::new(),
            key: String::new(),
            payload_bytes: 0,
            payload_sha256: String::new(),
            reset_reply_sha256: String::new(),
            preload_reply_sha256: String::new(),
            verification_reply_sha256: String::new(),
            logical_state_sha256: String::new(),
        }
    }

    fn repeats() -> Vec<ComparisonRepeatEvidence> {
        let hydra = SocketAddr::from((Ipv4Addr::LOCALHOST, 6380));
        let redis = SocketAddr::from((Ipv4Addr::LOCALHOST, 6381));
        (1_u8..=5)
            .map(|repeat| {
                let order = ExecutionOrder::for_repeat(repeat);
                let mut cases = Vec::new();
                for pipeline in [1_u32, 10] {
                    for system in order.systems() {
                        let endpoint = match system {
                            ComparisonSystem::Hydracache => hydra,
                            ComparisonSystem::Redis => redis,
                        };
                        let base = if system == ComparisonSystem::Hydracache {
                            80_000.0
                        } else {
                            100_000.0
                        };
                        let jitter = f64::from(repeat - 1) * 100.0;
                        cases.push(SystemCaseEvidence {
                            system,
                            pipeline,
                            endpoint,
                            initial_state: dummy_initial(endpoint),
                            process: dummy_process(),
                            rows: vec![
                                row("GET", base + jitter + f64::from(pipeline)),
                                row("SET", base - 1_000.0 + jitter + f64::from(pipeline)),
                            ],
                        });
                    }
                }
                ComparisonRepeatEvidence {
                    repeat,
                    order,
                    cases,
                }
            })
            .collect()
    }

    #[test]
    fn committed_contract_pins_tool_image_and_method() {
        let scenario = scenario();
        scenario.validate().unwrap();
        assert_eq!(scenario.repeats, 5);
        assert_eq!(scenario.pipelines, [1, 10]);
        assert_eq!(scenario.operations, ["get", "set"]);
        assert!(scenario.image_reference().contains("@sha256:"));

        let mut mutable = scenario;
        mutable.docker.image_index_digest = "redis:7.2.5".to_owned();
        assert!(mutable.validate().is_err());
    }

    #[test]
    fn aggregation_is_paired_and_preserves_both_orders() {
        let aggregates = derive_aggregates(&repeats(), &scenario()).unwrap();
        assert_eq!(aggregates.len(), 4);
        assert!(aggregates.iter().all(|aggregate| aggregate.stable));
        assert!(aggregates.iter().all(|aggregate| {
            aggregate.hydracache_requests_per_second.len() == 5
                && aggregate.redis_requests_per_second.len() == 5
                && aggregate.hydracache_over_redis_ratio.len() == 5
                && aggregate.median_ratio_hydracache_first > 0.0
                && aggregate.median_ratio_redis_first > 0.0
        }));
    }

    #[test]
    fn aggregation_rejects_incomplete_or_unpaired_runs() {
        let mut evidence = repeats();
        evidence[0]
            .cases
            .retain(|case| case.system != ComparisonSystem::Redis || case.pipeline != 10);
        let error = derive_aggregates(&evidence, &scenario()).unwrap_err();
        assert!(error.to_string().contains("Redis/pipeline-10"));
    }

    #[test]
    fn docker_inspect_requires_exact_index_and_platform() {
        let scenario = scenario();
        let exact = format!(
            r#"[{{"Id":"sha256:{}","RepoDigests":["{}"],"Architecture":"amd64","Os":"linux"}}]"#,
            "a".repeat(64),
            scenario.image_reference()
        );
        let facts = parse_image_inspect(&exact, &scenario).unwrap();
        assert_eq!(facts.repo_digests, [scenario.image_reference()]);

        let wrong = exact.replace("amd64", "arm64");
        assert!(parse_image_inspect(&wrong, &scenario).is_err());

        let manifest = format!(
            r#"[{{"Descriptor":{{"digest":"{}","platform":{{"os":"linux","architecture":"amd64"}}}}}}]"#,
            scenario.docker.image_platform_manifest_digest
        );
        assert!(manifest_output_contains_platform_digest(
            &manifest,
            &scenario.docker.image_platform_manifest_digest,
            "linux",
            "amd64"
        ));
        assert!(!manifest_output_contains_platform_digest(
            &manifest.replace("amd64", "arm64"),
            &scenario.docker.image_platform_manifest_digest,
            "linux",
            "amd64"
        ));
    }

    #[test]
    fn boundary_canary_is_discriminating() {
        let error = w8_boundary_canary_red().unwrap_err();
        assert!(error.contains(W8_CANARY_MARKER));
        assert!(same_host_contract("runner-a", "runner-a").is_ok());
        assert!(same_host_contract("runner-a", "runner-b").is_err());
    }

    #[test]
    fn canonical_w3_set_rejects_missing_lifecycle_or_suite_receipt() {
        let root = temporary_root("artifact-set");
        let paths = W3ReferenceArtifactSet::canonical(&root).unwrap();
        for path in [
            &paths.open_loop_report,
            &paths.daemon_lifecycle,
            &paths.external_report,
            &paths.suite_receipt,
        ] {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"{}\n").unwrap();
        }
        paths.read_canonical(&root).unwrap();

        fs::remove_file(&paths.daemon_lifecycle).unwrap();
        assert!(paths.read_canonical(&root).is_err());
        fs::write(&paths.daemon_lifecycle, b"{}\n").unwrap();
        fs::remove_file(&paths.suite_receipt).unwrap();
        assert!(paths.read_canonical(&root).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn suite_seal_rejects_stale_lifecycle_and_tampered_receipt() {
        let open_loop = b"open-loop";
        let external = b"external";
        let lifecycle = b"lifecycle";
        let payload = RespReferenceSuiteReceiptPayload {
            schema_version: 1,
            source_commit: "a".repeat(40),
            prebuild_manifest_sha256: sha256(b"manifest"),
            selected_endpoint: "hydracache-server@127.0.0.1:6380".to_owned(),
            endpoint_capability_sha256: sha256(b"capability"),
            open_loop_report_sha256: sha256(open_loop),
            external_report_sha256: sha256(external),
            daemon_lifecycle_sha256: sha256(lifecycle),
        };
        let exact = RespReferenceSuiteReceipt {
            receipt_sha256: typed_digest(&payload),
            payload,
        };
        validate_suite_artifact_seal(&exact, open_loop, external, lifecycle).unwrap();
        assert!(
            validate_suite_artifact_seal(&exact, open_loop, external, b"stale-lifecycle").is_err()
        );
        let mut tampered = exact;
        tampered.receipt_sha256 = sha256(b"caller-authored-suite");
        assert!(validate_suite_artifact_seal(&tampered, open_loop, external, lifecycle).is_err());
    }

    #[test]
    fn archived_lifecycle_rejects_stale_logs_missing_files_and_live_pid_reuse() {
        let fixture = lifecycle_fixture();
        validate_archived_lifecycle(
            &fixture.capability,
            &fixture.lifecycle,
            &fixture.server,
            &fixture.loadgen,
        )
        .unwrap();

        fs::write(&fixture.lifecycle.stdout_log.canonical_path, b"tampered\n").unwrap();
        assert!(validate_archived_lifecycle(
            &fixture.capability,
            &fixture.lifecycle,
            &fixture.server,
            &fixture.loadgen,
        )
        .is_err());
        fs::write(&fixture.lifecycle.stdout_log.canonical_path, b"ready\n").unwrap();

        fs::remove_file(&fixture.lifecycle.stderr_log.canonical_path).unwrap();
        assert!(validate_archived_lifecycle(
            &fixture.capability,
            &fixture.lifecycle,
            &fixture.server,
            &fixture.loadgen,
        )
        .is_err());
        fs::write(&fixture.lifecycle.stderr_log.canonical_path, b"").unwrap();

        let mut reused_capability = fixture.capability.clone();
        reused_capability.pid = std::process::id();
        let mut reused_lifecycle = fixture.lifecycle.clone();
        reused_lifecycle.pid = reused_capability.pid;
        reused_lifecycle.endpoint_capability_digest = reused_capability.digest().unwrap();
        assert!(validate_archived_lifecycle(
            &reused_capability,
            &reused_lifecycle,
            &fixture.server,
            &fixture.loadgen,
        )
        .is_err());
    }

    #[test]
    fn atomic_report_publication_is_create_new_and_never_leaves_partial_output() {
        let root = temporary_root("atomic-report");
        let path = root.join("compare-redis.json");
        assert!(write_new_bytes_atomic(&path, b"").is_err());
        assert!(!path.exists());

        write_new_bytes_atomic(&path, b"complete-report\n").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"complete-report\n");
        assert!(write_new_bytes_atomic(&path, b"replacement\n").is_err());
        assert_eq!(fs::read(&path).unwrap(), b"complete-report\n");
        assert!(fs::read_dir(&root).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("-atomic.tmp")
        }));
        fs::remove_dir_all(root).unwrap();
    }
}
