//! Receipt-bound reference harness for the release-0.67 RESP tier.
//!
//! This module is intentionally separate from the fast in-process RESP fixture.
//! It accepts only a clean, exact prebuild receipt, launches the recorded server
//! binary directly, and exposes identities that can make a selected node-local
//! daemon capacity claim. There is no build fallback and no `cargo` invocation.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener as StdTcpListener};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::profile::{PerformanceProfile, RunnerFingerprint};
use crate::report::{
    BuildIdentity, RespDaemonConfigIdentity, RespEndpointCapability, SourceIdentity,
    SurfaceIdentity,
};
use crate::targets::resp::{
    encode_resp2_command, parse_resp2, Resp2Limits, Resp2ParseStatus, Resp2Value,
    RespEndpointIdentity,
};

pub const PREBUILD_MANIFEST_RELATIVE_PATH: &str =
    "target/test-evidence/0.67/prebuild-manifest.json";
pub const PREBUILD_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const REFERENCE_BUILD_CONTRACT_SCHEMA_VERSION: u32 = 1;
pub const REFERENCE_PROFILE: &str = "reference-v1";
pub const SERVER_BINARY_ID: &str = "hydracache-server";
pub const LOADGEN_BINARY_ID: &str = "hydracache-loadgen";
pub const RESP_SURFACE_KIND: &str = "node-resp";
pub const RESP_EXECUTION_MODE: &str = "real-daemon-tcp-resp-open-loop";
pub const RESP_STATE_SCOPE: &str = "node-local";
pub const RESP_NETWORK_BOUNDARY: &str = "loopback-tcp";
pub const RESP_CLAIM_SCOPE: &str = "selected-endpoint-capacity";
pub const RESP_PING_FRAME: &[u8] = b"*1\r\n$4\r\nPING\r\n";
pub const RESP_PONG_FRAME: &[u8] = b"+PONG\r\n";
pub const RESP_PONG_DISPLAY: &str = "+PONG\\r\\n";

const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_READINESS_RESPONSE_BYTES: usize = 1024;
const MIN_STARTUP_TIMEOUT: Duration = Duration::from_millis(100);
const MAX_STARTUP_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_PING_INTERVAL: Duration = Duration::from_secs(1);
const PING_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(1);

static RUN_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Exact schema emitted by the mandatory W10 prebuild gate.
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

/// Stable baseline-eligibility contract. Source SHA, per-run fingerprint, file
/// paths, and binary hashes intentionally remain outside this digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceBuildContract {
    pub schema_version: u32,
    pub toolchain_identity: String,
    pub target_set: Vec<String>,
    pub features: Vec<String>,
    pub cargo_profile: String,
    pub flags: Vec<String>,
    pub build_recipe: Vec<String>,
}

impl ReferenceBuildContract {
    pub fn new(
        toolchain_identity: impl Into<String>,
        target_set: Vec<String>,
        features: Vec<String>,
        cargo_profile: impl Into<String>,
        flags: Vec<String>,
        build_recipe: Vec<String>,
    ) -> Self {
        Self {
            schema_version: REFERENCE_BUILD_CONTRACT_SCHEMA_VERSION,
            toolchain_identity: toolchain_identity.into(),
            target_set,
            features,
            cargo_profile: cargo_profile.into(),
            flags,
            build_recipe,
        }
    }

    pub fn digest(&self) -> Result<String, RespReferenceError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|error| {
            RespReferenceError::Contract(format!(
                "unable to serialize reference build contract: {error}"
            ))
        })?;
        Ok(sha256_bytes(&bytes))
    }

    pub fn validate(&self) -> Result<(), RespReferenceError> {
        let expected_binary_ids = [LOADGEN_BINARY_ID, SERVER_BINARY_ID];
        if self.schema_version != REFERENCE_BUILD_CONTRACT_SCHEMA_VERSION
            || canonical_toolchain_identity(&self.toolchain_identity).is_err()
            || self.cargo_profile != "release"
            || self.flags.is_empty()
            || self.flags.iter().any(|flag| flag.trim().is_empty())
            || self.flags.iter().collect::<BTreeSet<_>>().len() != self.flags.len()
            || self
                .features
                .iter()
                .any(|feature| feature.trim().is_empty())
            || self.build_recipe.is_empty()
            || self.build_recipe.iter().any(|step| step.trim().is_empty())
            || !self.flags.iter().any(|flag| flag == "--release")
            || !self.flags.iter().any(|flag| flag == "--locked")
            || self
                .target_set
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                != expected_binary_ids
        {
            return Err(RespReferenceError::Contract(
                "reference build contract must match the W7 toolchain/target/features/profile/flags/recipe contract and exact loadgen/server target set"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

/// W7-owned runner/profile facts plus the committed build contract. There is no
/// permissive default: callers that do not have W7 evidence cannot construct a
/// reference context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferencePrerequisites {
    pub profile: PerformanceProfile,
    pub observed_runner: RunnerFingerprint,
    pub expected_build_contract: ReferenceBuildContract,
}

impl ReferencePrerequisites {
    pub fn validate(&self) -> Result<(), RespReferenceError> {
        self.expected_build_contract.validate()?;
        if self.profile.name != REFERENCE_PROFILE
            || self.observed_runner.fingerprint.trim().is_empty()
        {
            return Err(RespReferenceError::Contract(
                "W7 prerequisites must provide the exact reference-v1 profile, observed runner fingerprint, and matching build contract"
                    .to_owned(),
            ));
        }
        let profile_problems = self.profile.contract_problems();
        if !profile_problems.is_empty() {
            return Err(RespReferenceError::Contract(format!(
                "W7 reference profile is invalid: {profile_problems:?}"
            )));
        }
        let validation = self.profile.validate(&self.observed_runner);
        if !validation.eligible {
            return Err(RespReferenceError::Contract(format!(
                "observed runner is not eligible for reference-v1: {:?}",
                validation.reasons
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedBinary {
    pub id: String,
    pub canonical_path: PathBuf,
    pub sha256: String,
}

/// Fully validated receipt context used by a real daemon repeat.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedRespReferenceContext {
    pub repo_root: PathBuf,
    pub manifest_path: PathBuf,
    pub manifest_sha256: String,
    pub source: SourceIdentity,
    pub build: BuildIdentity,
    pub profile: PerformanceProfile,
    pub runner: RunnerFingerprint,
    pub surface: SurfaceIdentity,
    pub server: VerifiedBinary,
    pub loadgen: VerifiedBinary,
}

impl ValidatedRespReferenceContext {
    pub fn endpoint_identity(
        &self,
        address: SocketAddr,
    ) -> Result<RespEndpointIdentity, RespReferenceError> {
        validate_loopback_endpoint("RESP", address)?;
        Ok(RespEndpointIdentity {
            address,
            selected_endpoint: format!("hydracache-server@{address}"),
            endpoint_kind: RESP_SURFACE_KIND.to_owned(),
            state_scope: RESP_STATE_SCOPE.to_owned(),
        })
    }

    fn verify_binaries_unchanged(&self) -> Result<(), RespReferenceError> {
        verify_recorded_binary(&self.server)?;
        verify_recorded_binary(&self.loadgen)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RespReferenceError {
    #[error("reference-v1 is blocked until W7 supplies a validated profile, runner fingerprint, and build contract")]
    MissingW7Prerequisites,
    #[error("prebuild manifest {path} is unavailable: {source}")]
    ManifestRead {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("prebuild manifest {path} is invalid: {detail}")]
    ManifestInvalid { path: PathBuf, detail: String },
    #[error("reference contract validation failed: {0}")]
    Contract(String),
    #[error("reference system probe failed: {0}")]
    SystemProbe(String),
    #[error("reference daemon IO failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("reference daemon readiness failed: {detail}; logs={logs:?}")]
    Readiness {
        detail: String,
        logs: Box<RespReadinessFailureLogs>,
    },
    #[error("reference daemon lifecycle failed: {detail}; evidence={evidence:?}")]
    DaemonLifecycle {
        detail: String,
        evidence: Box<RespDaemonEvidence>,
    },
}

#[derive(Debug, Clone)]
struct LiveReferenceFacts {
    git_commit: String,
    git_clean: bool,
    cargo_lock_sha256: String,
    toolchain_identity: String,
    current_exe: PathBuf,
}

/// Load and validate the mandatory manifest at its exact release-evidence path.
/// `None` is an intentional fail-closed seam used before W7 has landed.
pub fn load_reference_context(
    repo_root: &Path,
    prerequisites: Option<&ReferencePrerequisites>,
) -> Result<ValidatedRespReferenceContext, RespReferenceError> {
    let prerequisites = prerequisites.ok_or(RespReferenceError::MissingW7Prerequisites)?;
    prerequisites.validate()?;
    let repo_root = fs::canonicalize(repo_root).map_err(|error| {
        RespReferenceError::SystemProbe(format!(
            "unable to canonicalize repository root {}: {error}",
            repo_root.display()
        ))
    })?;
    let manifest_path = repo_root.join(PREBUILD_MANIFEST_RELATIVE_PATH);
    let bytes = read_bounded_manifest(&manifest_path)?;
    let manifest = parse_prebuild_manifest(&bytes, &manifest_path)?;
    let facts = observe_live_facts(&repo_root)?;
    validate_manifest(
        &repo_root,
        manifest_path,
        &bytes,
        manifest,
        prerequisites,
        &facts,
    )
}

pub fn parse_prebuild_manifest(
    bytes: &[u8],
    path: &Path,
) -> Result<PerfPrebuildManifest, RespReferenceError> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(RespReferenceError::ManifestInvalid {
            path: path.to_path_buf(),
            detail: format!(
                "manifest size must be 1..={MAX_MANIFEST_BYTES} bytes, got {}",
                bytes.len()
            ),
        });
    }
    serde_json::from_slice(bytes).map_err(|error| RespReferenceError::ManifestInvalid {
        path: path.to_path_buf(),
        detail: error.to_string(),
    })
}

fn read_bounded_manifest(path: &Path) -> Result<Vec<u8>, RespReferenceError> {
    let metadata = fs::metadata(path).map_err(|source| RespReferenceError::ManifestRead {
        path: path.to_path_buf(),
        source,
    })?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_MANIFEST_BYTES {
        return Err(RespReferenceError::ManifestInvalid {
            path: path.to_path_buf(),
            detail: format!(
                "manifest must be a regular 1..={MAX_MANIFEST_BYTES}-byte file, got {} bytes",
                metadata.len()
            ),
        });
    }
    fs::read(path).map_err(|source| RespReferenceError::ManifestRead {
        path: path.to_path_buf(),
        source,
    })
}

fn observe_live_facts(repo_root: &Path) -> Result<LiveReferenceFacts, RespReferenceError> {
    let top_level = run_probe(
        "git",
        &["-C", path_text(repo_root)?, "rev-parse", "--show-toplevel"],
    )?;
    let observed_root = fs::canonicalize(Path::new(top_level.trim())).map_err(|error| {
        RespReferenceError::SystemProbe(format!(
            "unable to canonicalize git top-level {top_level:?}: {error}"
        ))
    })?;
    if observed_root != repo_root {
        return Err(RespReferenceError::SystemProbe(format!(
            "requested repository root {} differs from git top-level {}",
            repo_root.display(),
            observed_root.display()
        )));
    }
    let git_commit = run_probe("git", &["-C", path_text(repo_root)?, "rev-parse", "HEAD"])?
        .trim()
        .to_owned();
    let status = run_probe(
        "git",
        &[
            "-C",
            path_text(repo_root)?,
            "status",
            "--porcelain=v1",
            "--untracked-files=normal",
            "--ignore-submodules=none",
        ],
    )?;
    let cargo_lock_sha256 = sha256_file(&repo_root.join("Cargo.lock"))?;
    let toolchain_identity = canonical_toolchain_identity(&run_probe("rustc", &["-V"])?)?;
    let current_exe = fs::canonicalize(std::env::current_exe().map_err(|error| {
        RespReferenceError::SystemProbe(format!("unable to resolve current executable: {error}"))
    })?)
    .map_err(|error| {
        RespReferenceError::SystemProbe(format!(
            "unable to canonicalize current executable: {error}"
        ))
    })?;
    Ok(LiveReferenceFacts {
        git_commit,
        git_clean: status.trim().is_empty(),
        cargo_lock_sha256,
        toolchain_identity,
        current_exe,
    })
}

fn validate_manifest(
    repo_root: &Path,
    manifest_path: PathBuf,
    manifest_bytes: &[u8],
    manifest: PerfPrebuildManifest,
    prerequisites: &ReferencePrerequisites,
    facts: &LiveReferenceFacts,
) -> Result<ValidatedRespReferenceContext, RespReferenceError> {
    let invalid = |detail: String| RespReferenceError::ManifestInvalid {
        path: manifest_path.clone(),
        detail,
    };
    if manifest.schema_version != PREBUILD_MANIFEST_SCHEMA_VERSION {
        return Err(invalid(format!(
            "unsupported schema_version {}; expected {PREBUILD_MANIFEST_SCHEMA_VERSION}",
            manifest.schema_version
        )));
    }
    if !valid_git_commit(&manifest.source.git_commit)
        || !valid_sha256(&manifest.source.cargo_lock_sha256)
        || !valid_sha256(&manifest.build_contract_digest)
    {
        return Err(invalid(
            "source commit and all recorded digests must be canonical lowercase hex".to_owned(),
        ));
    }
    if !facts.git_clean {
        return Err(invalid(
            "working tree is dirty; reference evidence requires the exact clean prebuilt commit"
                .to_owned(),
        ));
    }
    if manifest.source.git_commit != facts.git_commit {
        return Err(invalid(format!(
            "manifest commit {} differs from checked-out commit {}",
            manifest.source.git_commit, facts.git_commit
        )));
    }
    if manifest.source.cargo_lock_sha256 != facts.cargo_lock_sha256 {
        return Err(invalid(
            "manifest Cargo.lock SHA-256 differs from the checked-out Cargo.lock".to_owned(),
        ));
    }
    if manifest.runner_profile != REFERENCE_PROFILE
        || manifest.runner_profile != prerequisites.profile.name
        || manifest.runner_fingerprint != prerequisites.observed_runner.fingerprint
    {
        return Err(invalid(
            "manifest runner profile/fingerprint differs from the W7-observed reference runner"
                .to_owned(),
        ));
    }
    let expected_contract = &prerequisites.expected_build_contract;
    if manifest.toolchain_identity != expected_contract.toolchain_identity
        || manifest.toolchain_identity != facts.toolchain_identity
    {
        return Err(invalid(
            "manifest, committed contract, and live rustc toolchain identities differ".to_owned(),
        ));
    }
    if manifest.target_set != expected_contract.target_set
        || manifest.features != expected_contract.features
        || manifest.cargo_profile != expected_contract.cargo_profile
        || manifest.flags != expected_contract.flags
        || manifest.build_recipe != expected_contract.build_recipe
        || manifest.platform_key.trim().is_empty()
    {
        return Err(invalid(
            "manifest target/features/profile/flags/recipe differ from the committed W7 prebuild contract"
                .to_owned(),
        ));
    }
    let expected_contract_digest = expected_contract.digest()?;
    let manifest_contract = ReferenceBuildContract {
        schema_version: REFERENCE_BUILD_CONTRACT_SCHEMA_VERSION,
        toolchain_identity: manifest.toolchain_identity.clone(),
        target_set: manifest.target_set.clone(),
        features: manifest.features.clone(),
        cargo_profile: manifest.cargo_profile.clone(),
        flags: manifest.flags.clone(),
        build_recipe: manifest.build_recipe.clone(),
    };
    if manifest.build_contract_digest != expected_contract_digest
        || manifest.build_contract_digest != manifest_contract.digest()?
    {
        return Err(invalid(
            "manifest build_contract_digest does not bind the exact committed contract".to_owned(),
        ));
    }

    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut verified = BTreeMap::new();
    for binary in &manifest.binaries {
        if !valid_identifier(&binary.id)
            || !ids.insert(binary.id.clone())
            || !valid_sha256(&binary.sha256)
        {
            return Err(invalid(format!(
                "binary entries require unique portable ids and lowercase SHA-256: {:?}",
                binary.id
            )));
        }
        if !binary.canonical_path.is_absolute() {
            return Err(invalid(format!(
                "binary {} path is not absolute: {}",
                binary.id,
                binary.canonical_path.display()
            )));
        }
        let canonical = fs::canonicalize(&binary.canonical_path).map_err(|error| {
            invalid(format!(
                "unable to canonicalize binary {} at {}: {error}",
                binary.id,
                binary.canonical_path.display()
            ))
        })?;
        if canonical != binary.canonical_path || !paths.insert(canonical.clone()) {
            return Err(invalid(format!(
                "binary {} path must already be canonical and unique: {}",
                binary.id,
                binary.canonical_path.display()
            )));
        }
        if !fs::metadata(&canonical)
            .map_err(|error| invalid(format!("unable to stat {}: {error}", canonical.display())))?
            .is_file()
        {
            return Err(invalid(format!(
                "binary {} is not a regular file",
                canonical.display()
            )));
        }
        let observed_sha256 = sha256_file(&canonical)?;
        if observed_sha256 != binary.sha256 {
            return Err(invalid(format!(
                "binary {} SHA-256 differs from the prebuild manifest",
                binary.id
            )));
        }
        verified.insert(
            binary.id.clone(),
            VerifiedBinary {
                id: binary.id.clone(),
                canonical_path: canonical,
                sha256: observed_sha256,
            },
        );
    }
    let server = verified
        .remove(SERVER_BINARY_ID)
        .ok_or_else(|| invalid("prebuild manifest has no hydracache-server binary".to_owned()))?;
    let loadgen = verified
        .remove(LOADGEN_BINARY_ID)
        .ok_or_else(|| invalid("prebuild manifest has no hydracache-loadgen binary".to_owned()))?;
    if !verified.is_empty() {
        return Err(invalid(format!(
            "prebuild manifest contains unexpected binaries: {:?}",
            verified.keys().collect::<Vec<_>>()
        )));
    }
    validate_required_binary_path(repo_root, &server, SERVER_BINARY_ID)?;
    validate_required_binary_path(repo_root, &loadgen, LOADGEN_BINARY_ID)?;
    if facts.current_exe != loadgen.canonical_path {
        return Err(invalid(format!(
            "running loadgen {} is not the receipt-bound binary {}",
            facts.current_exe.display(),
            loadgen.canonical_path.display()
        )));
    }

    let manifest_sha256 = sha256_bytes(manifest_bytes);
    let source = SourceIdentity {
        git_commit: manifest.source.git_commit,
        cargo_lock_sha256: manifest.source.cargo_lock_sha256,
        toolchain: manifest.toolchain_identity,
        build_flags: manifest.flags,
    };
    let build = BuildIdentity {
        prebuild_contract_digest: manifest.build_contract_digest,
        prebuild_manifest_sha256: manifest_sha256.clone(),
        binary_sha256: vec![
            (LOADGEN_BINARY_ID.to_owned(), loadgen.sha256.clone()),
            (SERVER_BINARY_ID.to_owned(), server.sha256.clone()),
        ],
    };
    Ok(ValidatedRespReferenceContext {
        repo_root: repo_root.to_path_buf(),
        manifest_path,
        manifest_sha256,
        source,
        build,
        profile: prerequisites.profile.clone(),
        runner: prerequisites.observed_runner.clone(),
        surface: SurfaceIdentity {
            surface_kind: RESP_SURFACE_KIND.to_owned(),
            execution_mode: RESP_EXECUTION_MODE.to_owned(),
            state_scope: RESP_STATE_SCOPE.to_owned(),
            network_boundary: RESP_NETWORK_BOUNDARY.to_owned(),
            claim_scope: RESP_CLAIM_SCOPE.to_owned(),
        },
        server,
        loadgen,
    })
}

fn validate_required_binary_path(
    repo_root: &Path,
    binary: &VerifiedBinary,
    expected_id: &str,
) -> Result<(), RespReferenceError> {
    let expected = repo_root
        .join("target")
        .join("release")
        .join(format!("{expected_id}{}", std::env::consts::EXE_SUFFIX));
    let expected = fs::canonicalize(&expected).map_err(|error| {
        RespReferenceError::Contract(format!(
            "expected prebuilt binary {} is unavailable: {error}",
            expected.display()
        ))
    })?;
    if binary.canonical_path != expected {
        return Err(RespReferenceError::Contract(format!(
            "{} must resolve to the canonical target/release binary {}, got {}",
            binary.id,
            expected.display(),
            binary.canonical_path.display()
        )));
    }
    Ok(())
}

fn verify_recorded_binary(binary: &VerifiedBinary) -> Result<(), RespReferenceError> {
    let canonical = fs::canonicalize(&binary.canonical_path).map_err(|error| {
        RespReferenceError::Contract(format!(
            "receipt-bound binary {} disappeared: {error}",
            binary.canonical_path.display()
        ))
    })?;
    if canonical != binary.canonical_path || sha256_file(&canonical)? != binary.sha256 {
        return Err(RespReferenceError::Contract(format!(
            "receipt-bound binary {} changed after manifest validation",
            binary.id
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespReferencePorts {
    pub resp: SocketAddr,
    pub admin: SocketAddr,
}

impl RespReferencePorts {
    pub fn validate(self) -> Result<(), RespReferenceError> {
        validate_loopback_endpoint("RESP", self.resp)?;
        validate_loopback_endpoint("admin", self.admin)?;
        if self.resp == self.admin {
            return Err(RespReferenceError::Contract(
                "reference RESP and admin endpoints must be distinct".to_owned(),
            ));
        }
        Ok(())
    }

    /// Pick currently free explicit loopback ports. The caller should launch
    /// immediately; the daemon still fails closed if either bind is lost.
    pub fn select_available() -> Result<Self, RespReferenceError> {
        let resp = select_available_loopback()?;
        let mut admin = select_available_loopback()?;
        while admin == resp {
            admin = select_available_loopback()?;
        }
        let ports = Self { resp, admin };
        ports.validate()?;
        Ok(ports)
    }
}

#[derive(Debug, Clone)]
pub struct RespDaemonLaunch {
    pub repeat_index: u32,
    pub ports: RespReferencePorts,
    pub evidence_root: PathBuf,
    pub startup_timeout: Duration,
    pub ping_interval: Duration,
}

impl RespDaemonLaunch {
    pub fn for_repeat(repo_root: &Path, repeat_index: u32, ports: RespReferencePorts) -> Self {
        Self {
            repeat_index,
            ports,
            evidence_root: repo_root.join("target/test-evidence/0.67/resp-daemon-repeats"),
            startup_timeout: Duration::from_secs(20),
            ping_interval: Duration::from_millis(25),
        }
    }

    fn validate(&self) -> Result<(), RespReferenceError> {
        self.ports.validate()?;
        if !(MIN_STARTUP_TIMEOUT..=MAX_STARTUP_TIMEOUT).contains(&self.startup_timeout)
            || self.ping_interval.is_zero()
            || self.ping_interval > MAX_PING_INTERVAL
            || self.ping_interval >= self.startup_timeout
        {
            return Err(RespReferenceError::Contract(
                "reference readiness timeout/interval must be bounded and non-zero".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespPingEvidence {
    pub request_sha256: String,
    pub response_sha256: String,
    pub attempts: u32,
    pub selected_endpoint: SocketAddr,
    pub exact_response: String,
}

impl RespEndpointCapability {
    pub fn digest(&self) -> Result<String, RespReferenceError> {
        if self.schema_version != 1
            || self.pid == 0
            || self.started_unix_nanos == 0
            || !self.direct_prebuilt_exec
            || !self.fresh_data_dir
            || self.config.role != "local"
            || self.config.listen_addr != SocketAddr::from((Ipv4Addr::LOCALHOST, 0))
            || self.config.cluster_addr != SocketAddr::from((Ipv4Addr::LOCALHOST, 0))
            || !self.config.admin_enabled
            || !self.config.redis_enabled
            || self.config.redis_auth_required
            || self.config.rediss_enabled
            || self.config.redis_addr == self.config.admin_addr
            || self.selected_endpoint != format!("hydracache-server@{}", self.config.redis_addr)
            || !self.config.storage_dir.is_absolute()
            || !valid_sha256(&self.server_binary_sha256)
            || !valid_sha256(&self.loadgen_binary_sha256)
            || !valid_sha256(&self.prebuild_manifest_sha256)
            || !valid_sha256(&self.prebuild_contract_digest)
            || !valid_git_commit(&self.source_commit)
        {
            return Err(RespReferenceError::Contract(
                "RESP endpoint capability is incomplete or does not bind the exact direct-daemon configuration"
                    .to_owned(),
            ));
        }
        validate_loopback_endpoint("capability RESP", self.config.redis_addr)?;
        validate_loopback_endpoint("capability admin", self.config.admin_addr)?;
        let payload = serde_json::to_vec(self).map_err(|error| {
            RespReferenceError::Contract(format!(
                "unable to serialize RESP endpoint capability: {error}"
            ))
        })?;
        Ok(sha256_bytes(&payload))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogEvidence {
    pub canonical_path: PathBuf,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespReadinessFailureLogs {
    pub stdout: LogEvidence,
    pub stderr: LogEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespDaemonEvidence {
    pub repeat_index: u32,
    pub pid: u32,
    pub direct_prebuilt_exec: bool,
    pub server_binary_path: PathBuf,
    pub server_binary_sha256: String,
    pub loadgen_binary_path: PathBuf,
    pub loadgen_binary_sha256: String,
    pub binaries_verified_after_measurement: bool,
    pub resp_endpoint: SocketAddr,
    pub admin_endpoint: SocketAddr,
    pub selected_endpoint: String,
    pub endpoint_capability_digest: String,
    pub data_dir: PathBuf,
    pub readiness: RespPingEvidence,
    pub killed_and_waited: bool,
    pub exit_code: Option<i32>,
    pub stdout_log: LogEvidence,
    pub stderr_log: LogEvidence,
}

/// Owned direct-child daemon. `stop` captures hashed lifecycle evidence. Drop
/// still kills and waits, so cancellation cannot leak a daemon or zombie.
pub struct RespDaemonFixture {
    child: Option<Child>,
    pid: u32,
    repeat_index: u32,
    server: VerifiedBinary,
    loadgen: VerifiedBinary,
    ports: RespReferencePorts,
    data_dir: PathBuf,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
    readiness: RespPingEvidence,
    endpoint_capability: RespEndpointCapability,
}

/// Owns a just-spawned daemon until every fallible capability/readiness step
/// has completed. Any early `?` still kills and waits the direct child.
struct SpawnedChildGuard {
    child: Option<Child>,
}

impl SpawnedChildGuard {
    fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    fn child_mut(&mut self) -> &mut Child {
        self.child.as_mut().expect("spawned child guard is armed")
    }

    fn disarm(mut self) -> Child {
        self.child.take().expect("spawned child guard is armed")
    }

    fn kill_and_wait(&mut self) -> Result<(), String> {
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| "spawned child guard was already disarmed".to_owned())?;
        let mut problems = Vec::new();
        if let Err(error) = child.kill() {
            problems.push(format!("kill failed: {error}"));
        }
        match child.wait() {
            Ok(_) => {
                self.child.take();
            }
            Err(error) => problems.push(format!("wait/reap failed: {error}")),
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems.join("; "))
        }
    }
}

impl Drop for SpawnedChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl RespDaemonFixture {
    pub fn resp_endpoint(&self) -> SocketAddr {
        self.ports.resp
    }

    pub fn admin_endpoint(&self) -> SocketAddr {
        self.ports.admin
    }

    pub fn endpoint_identity(&self) -> RespEndpointIdentity {
        RespEndpointIdentity {
            address: self.ports.resp,
            selected_endpoint: format!("hydracache-server@{}", self.ports.resp),
            endpoint_kind: RESP_SURFACE_KIND.to_owned(),
            state_scope: RESP_STATE_SCOPE.to_owned(),
        }
    }

    /// This single capability is shared by all open-loop/raw/matrix/stall and
    /// supplemental external evidence emitted for the started endpoint.
    pub fn endpoint_capability(&self) -> &RespEndpointCapability {
        &self.endpoint_capability
    }

    pub fn endpoint_capability_digest(&self) -> Result<String, RespReferenceError> {
        self.endpoint_capability.digest()
    }

    pub async fn stop(mut self) -> Result<RespDaemonEvidence, RespReferenceError> {
        let mut child = self.child.take().ok_or_else(|| {
            RespReferenceError::Contract("reference daemon was already reaped".to_owned())
        })?;
        let mut problems = Vec::new();
        let mut status = None;
        let was_running = match child.try_wait() {
            Ok(Some(observed)) => {
                status = Some(observed);
                problems.push("prebuilt daemon exited before harness cleanup".to_owned());
                false
            }
            Ok(None) => true,
            Err(error) => {
                problems.push(format!("initial daemon status check failed: {error}"));
                true
            }
        };
        let mut kill_succeeded = false;
        let mut wait_succeeded = status.is_some();
        if status.is_none() {
            match child.kill() {
                Ok(()) => kill_succeeded = true,
                Err(error) => problems.push(format!("daemon kill failed: {error}")),
            }
            match child.wait() {
                Ok(observed) => {
                    status = Some(observed);
                    wait_succeeded = true;
                }
                Err(error) => problems.push(format!("daemon wait/reap failed: {error}")),
            }
        }
        let server_verified =
            verify_recorded_binary(&self.server).map_err(|error| error.to_string());
        let loadgen_verified =
            verify_recorded_binary(&self.loadgen).map_err(|error| error.to_string());
        if let Err(error) = &server_verified {
            problems.push(format!("server post-run verification failed: {error}"));
        }
        if let Err(error) = &loadgen_verified {
            problems.push(format!("loadgen post-run verification failed: {error}"));
        }
        let binaries_verified = server_verified.is_ok() && loadgen_verified.is_ok();
        let evidence = self.evidence(
            status.as_ref(),
            was_running && kill_succeeded && wait_succeeded,
            binaries_verified,
        )?;
        if !problems.is_empty() {
            return Err(RespReferenceError::DaemonLifecycle {
                detail: problems.join("; "),
                evidence: Box::new(evidence),
            });
        }
        Ok(evidence)
    }

    fn evidence(
        &self,
        status: Option<&ExitStatus>,
        killed_and_waited: bool,
        binaries_verified_after_measurement: bool,
    ) -> Result<RespDaemonEvidence, RespReferenceError> {
        Ok(RespDaemonEvidence {
            repeat_index: self.repeat_index,
            pid: self.pid,
            direct_prebuilt_exec: true,
            server_binary_path: self.server.canonical_path.clone(),
            server_binary_sha256: self.server.sha256.clone(),
            loadgen_binary_path: self.loadgen.canonical_path.clone(),
            loadgen_binary_sha256: self.loadgen.sha256.clone(),
            binaries_verified_after_measurement,
            resp_endpoint: self.ports.resp,
            admin_endpoint: self.ports.admin,
            selected_endpoint: format!("hydracache-server@{}", self.ports.resp),
            endpoint_capability_digest: self.endpoint_capability.digest()?,
            data_dir: self.data_dir.clone(),
            readiness: self.readiness.clone(),
            killed_and_waited,
            exit_code: status.and_then(ExitStatus::code),
            stdout_log: log_evidence(&self.stdout_path)?,
            stderr_log: log_evidence(&self.stderr_path)?,
        })
    }
}

impl Drop for RespDaemonFixture {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub async fn start_reference_daemon(
    context: &ValidatedRespReferenceContext,
    launch: &RespDaemonLaunch,
) -> Result<RespDaemonFixture, RespReferenceError> {
    launch.validate()?;
    context.verify_binaries_unchanged()?;
    ensure_ports_are_available(launch.ports)?;
    let run_dir = create_unique_run_dir(&launch.evidence_root, launch.repeat_index)?;
    let data_dir = fs::canonicalize(run_dir.join("data")).map_err(|error| {
        RespReferenceError::SystemProbe(format!("unable to canonicalize fresh data dir: {error}"))
    })?;
    let stdout_path = run_dir.join("hydracache-server.stdout.log");
    let stderr_path = run_dir.join("hydracache-server.stderr.log");
    let stdout = File::create(&stdout_path)?;
    let stderr = File::create(&stderr_path)?;

    let mut command = Command::new(&context.server.canonical_path);
    command
        .current_dir(&context.repo_root)
        .env_clear()
        .env("HYDRACACHE_ROLE", "local")
        .env("HYDRACACHE_LISTEN_ADDR", "127.0.0.1:0")
        .env("HYDRACACHE_CLUSTER_ADDR", "127.0.0.1:0")
        .env("HYDRACACHE_STORAGE_DIR", &data_dir)
        .env("HYDRACACHE_ADMIN_API_ENABLED", "true")
        .env("HYDRACACHE_ADMIN_ADDR", launch.ports.admin.to_string())
        .env("HYDRACACHE_REDIS_API_ENABLED", "true")
        .env("HYDRACACHE_REDIS_ADDR", launch.ports.resp.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    let child = command.spawn().map_err(|error| {
        RespReferenceError::SystemProbe(format!(
            "unable to execute prebuilt server {} directly: {error}",
            context.server.canonical_path.display()
        ))
    })?;
    let mut child_guard = SpawnedChildGuard::new(child);
    let pid = child_guard.child_mut().id();
    let endpoint_capability = RespEndpointCapability {
        schema_version: 1,
        pid,
        started_unix_nanos: unix_nanos_now()?,
        repeat_index: launch.repeat_index,
        direct_prebuilt_exec: true,
        fresh_data_dir: true,
        config: RespDaemonConfigIdentity {
            role: "local".to_owned(),
            listen_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            cluster_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            storage_dir: data_dir.clone(),
            admin_enabled: true,
            admin_addr: launch.ports.admin,
            redis_enabled: true,
            redis_addr: launch.ports.resp,
            redis_auth_required: false,
            rediss_enabled: false,
        },
        selected_endpoint: format!("hydracache-server@{}", launch.ports.resp),
        server_binary_sha256: context.server.sha256.clone(),
        loadgen_binary_sha256: context.loadgen.sha256.clone(),
        prebuild_manifest_sha256: context.manifest_sha256.clone(),
        prebuild_contract_digest: context.build.prebuild_contract_digest.clone(),
        source_commit: context.source.git_commit.clone(),
    };
    endpoint_capability.digest()?;
    let readiness = wait_for_strict_ping(
        child_guard.child_mut(),
        launch.ports.resp,
        launch.startup_timeout,
        launch.ping_interval,
    )
    .await;
    match readiness {
        Ok(readiness) => Ok(RespDaemonFixture {
            child: Some(child_guard.disarm()),
            pid,
            repeat_index: launch.repeat_index,
            server: context.server.clone(),
            loadgen: context.loadgen.clone(),
            ports: launch.ports,
            data_dir,
            stdout_path,
            stderr_path,
            readiness: RespPingEvidence {
                attempts: readiness.attempts,
                ..readiness
            },
            endpoint_capability,
        }),
        Err(detail) => {
            let cleanup = child_guard.kill_and_wait();
            let stdout_log = log_evidence(&stdout_path)?;
            let stderr_log = log_evidence(&stderr_path)?;
            Err(RespReferenceError::Readiness {
                detail: match cleanup {
                    Ok(()) => format!("pid {pid}: {detail}; child killed and reaped"),
                    Err(cleanup) => {
                        format!("pid {pid}: {detail}; cleanup also failed: {cleanup}")
                    }
                },
                logs: Box::new(RespReadinessFailureLogs {
                    stdout: stdout_log,
                    stderr: stderr_log,
                }),
            })
        }
    }
}

fn create_unique_run_dir(
    evidence_root: &Path,
    repeat_index: u32,
) -> Result<PathBuf, RespReferenceError> {
    fs::create_dir_all(evidence_root)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| RespReferenceError::SystemProbe(error.to_string()))?
        .as_nanos();
    let sequence = RUN_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let run_dir = evidence_root.join(format!(
        "repeat-{repeat_index}-pid-{}-nanos-{nonce}-seq-{sequence}",
        std::process::id()
    ));
    fs::create_dir(&run_dir)?;
    fs::create_dir(run_dir.join("data"))?;
    Ok(fs::canonicalize(run_dir)?)
}

fn ensure_ports_are_available(ports: RespReferencePorts) -> Result<(), RespReferenceError> {
    let resp = StdTcpListener::bind(ports.resp).map_err(|error| {
        RespReferenceError::Contract(format!(
            "selected RESP endpoint {} is unavailable before launch: {error}",
            ports.resp
        ))
    })?;
    let admin = StdTcpListener::bind(ports.admin).map_err(|error| {
        RespReferenceError::Contract(format!(
            "selected admin endpoint {} is unavailable before launch: {error}",
            ports.admin
        ))
    })?;
    drop(admin);
    drop(resp);
    Ok(())
}

fn select_available_loopback() -> Result<SocketAddr, RespReferenceError> {
    let listener = StdTcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    Ok(listener.local_addr()?)
}

fn unix_nanos_now() -> Result<u64, RespReferenceError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| RespReferenceError::SystemProbe(error.to_string()))?
        .as_nanos();
    u64::try_from(nanos).map_err(|_| {
        RespReferenceError::SystemProbe(
            "system clock nanoseconds do not fit the capability contract".to_owned(),
        )
    })
}

async fn wait_for_strict_ping(
    child: &mut Child,
    endpoint: SocketAddr,
    startup_timeout: Duration,
    ping_interval: Duration,
) -> Result<RespPingEvidence, String> {
    let started = Instant::now();
    let mut attempts = 0_u32;
    let mut last_retry = "RESP listener has not accepted a connection".to_owned();
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("unable to inspect daemon status: {error}"))?
        {
            return Err(format!(
                "daemon exited before strict PING readiness with status {status}"
            ));
        }
        if started.elapsed() >= startup_timeout {
            return Err(format!(
                "strict PING readiness timed out after {startup_timeout:?}: {last_retry}"
            ));
        }
        attempts = attempts.saturating_add(1);
        match strict_ping_once(endpoint).await {
            Ok((request, response)) => {
                return Ok(RespPingEvidence {
                    request_sha256: sha256_bytes(&request),
                    response_sha256: sha256_bytes(&response),
                    attempts,
                    selected_endpoint: endpoint,
                    exact_response: RESP_PONG_DISPLAY.to_owned(),
                });
            }
            Err(PingAttemptError::Retryable(detail)) => last_retry = detail,
            Err(PingAttemptError::Fatal(detail)) => return Err(detail),
        }
        tokio::time::sleep(ping_interval).await;
    }
}

#[derive(Debug)]
enum PingAttemptError {
    Retryable(String),
    Fatal(String),
}

async fn strict_ping_once(endpoint: SocketAddr) -> Result<(Vec<u8>, Vec<u8>), PingAttemptError> {
    let request = encode_resp2_command([b"PING".as_slice()]);
    debug_assert_eq!(request, RESP_PING_FRAME);
    let future = async {
        let mut stream = TcpStream::connect(endpoint)
            .await
            .map_err(|error| PingAttemptError::Retryable(error.to_string()))?;
        stream
            .write_all(&request)
            .await
            .map_err(|error| PingAttemptError::Retryable(error.to_string()))?;
        stream
            .flush()
            .await
            .map_err(|error| PingAttemptError::Retryable(error.to_string()))?;
        let mut response = Vec::new();
        let mut chunk = [0_u8; 64];
        loop {
            let read = stream
                .read(&mut chunk)
                .await
                .map_err(|error| PingAttemptError::Retryable(error.to_string()))?;
            if read == 0 {
                return Err(PingAttemptError::Fatal(
                    "RESP readiness connection closed before one exact PONG".to_owned(),
                ));
            }
            response.extend_from_slice(&chunk[..read]);
            if response.len() > MAX_READINESS_RESPONSE_BYTES {
                return Err(PingAttemptError::Fatal(format!(
                    "RESP readiness response exceeded {MAX_READINESS_RESPONSE_BYTES} bytes"
                )));
            }
            match parse_resp2(&response, Resp2Limits::default()) {
                Ok(Resp2ParseStatus::Incomplete) => {}
                Ok(Resp2ParseStatus::Complete { value, consumed }) => {
                    if consumed != response.len()
                        || value != Resp2Value::Simple(b"PONG".to_vec())
                        || response != RESP_PONG_FRAME
                    {
                        return Err(PingAttemptError::Fatal(format!(
                            "RESP readiness requires the exact +PONG reply, got {response:?}"
                        )));
                    }
                    return Ok((request, response));
                }
                Err(error) => {
                    return Err(PingAttemptError::Fatal(format!(
                        "RESP readiness returned an invalid frame: {error}"
                    )));
                }
            }
        }
    };
    tokio::time::timeout(PING_ATTEMPT_TIMEOUT, future)
        .await
        .map_err(|_| PingAttemptError::Retryable("strict PING attempt timed out".to_owned()))?
}

fn log_evidence(path: &Path) -> Result<LogEvidence, RespReferenceError> {
    let canonical_path = fs::canonicalize(path)?;
    let metadata = fs::metadata(&canonical_path)?;
    if !metadata.is_file() {
        return Err(RespReferenceError::Contract(format!(
            "daemon log is not a regular file: {}",
            canonical_path.display()
        )));
    }
    Ok(LogEvidence {
        canonical_path: canonical_path.clone(),
        bytes: metadata.len(),
        sha256: sha256_file(&canonical_path)?,
    })
}

fn validate_loopback_endpoint(label: &str, endpoint: SocketAddr) -> Result<(), RespReferenceError> {
    if endpoint.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST) || endpoint.port() == 0 {
        return Err(RespReferenceError::Contract(format!(
            "reference {label} endpoint must be explicit 127.0.0.1 with a non-zero port, got {endpoint}"
        )));
    }
    Ok(())
}

fn run_probe(program: &str, args: &[&str]) -> Result<String, RespReferenceError> {
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| {
            RespReferenceError::SystemProbe(format!(
                "unable to run {program} with exact probe arguments: {error}"
            ))
        })?;
    if !output.status.success() {
        return Err(RespReferenceError::SystemProbe(format!(
            "{program} probe failed with status {}; stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8(output.stdout).map_err(|error| {
        RespReferenceError::SystemProbe(format!("{program} probe emitted non-UTF-8: {error}"))
    })
}

fn canonical_toolchain_identity(output: &str) -> Result<String, RespReferenceError> {
    let normalized = output.trim();
    if let Some(version) = normalized.strip_prefix("rustc-") {
        if valid_rustc_semver(version) {
            return Ok(normalized.to_owned());
        }
    }
    let mut fields = normalized.split_whitespace();
    let program = fields.next();
    let version = fields.next();
    let valid_version = version.is_some_and(valid_rustc_semver);
    if program != Some("rustc") || !valid_version {
        return Err(RespReferenceError::SystemProbe(
            "toolchain probe must expose an exact rustc semantic version".to_owned(),
        ));
    }
    Ok(format!(
        "rustc-{}",
        version.expect("validated version exists")
    ))
}

fn valid_rustc_semver(version: &str) -> bool {
    let parts = version.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

fn path_text(path: &Path) -> Result<&str, RespReferenceError> {
    path.to_str().ok_or_else(|| {
        RespReferenceError::SystemProbe(format!(
            "repository path is not valid UTF-8: {}",
            path.display()
        ))
    })
}

fn sha256_file(path: &Path) -> Result<String, RespReferenceError> {
    let mut file = File::open(path)?;
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

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex_digest(hasher.finalize().as_ref())
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_git_commit(value: &str) -> bool {
    (40..=64).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ContractFixture {
        root: PathBuf,
        manifest_path: PathBuf,
        server: PathBuf,
        loadgen: PathBuf,
        contract: ReferenceBuildContract,
        prerequisites: ReferencePrerequisites,
        facts: LiveReferenceFacts,
        manifest: PerfPrebuildManifest,
    }

    impl ContractFixture {
        fn new(label: &str) -> Self {
            let nonce = RUN_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::current_dir()
                .unwrap()
                .join("target/test-hydracache-loadgen/resp-reference")
                .join(format!("{label}-{}-{nonce}", std::process::id()));
            fs::create_dir_all(root.join("target/release")).unwrap();
            fs::create_dir_all(root.join("target/test-evidence/0.67")).unwrap();
            fs::write(root.join("Cargo.lock"), b"fixture-lock").unwrap();
            let server = root.join("target/release").join(format!(
                "{SERVER_BINARY_ID}{}",
                std::env::consts::EXE_SUFFIX
            ));
            let loadgen = root.join("target/release").join(format!(
                "{LOADGEN_BINARY_ID}{}",
                std::env::consts::EXE_SUFFIX
            ));
            fs::write(&server, b"server-binary").unwrap();
            fs::write(&loadgen, b"loadgen-binary").unwrap();
            let root = fs::canonicalize(root).unwrap();
            let server = fs::canonicalize(server).unwrap();
            let loadgen = fs::canonicalize(loadgen).unwrap();
            let toolchain_identity = "rustc-1.94.0".to_owned();
            let contract = ReferenceBuildContract::new(
                toolchain_identity.clone(),
                vec![LOADGEN_BINARY_ID.to_owned(), SERVER_BINARY_ID.to_owned()],
                Vec::new(),
                "release",
                vec!["--locked".to_owned(), "--release".to_owned()],
                vec![
                    "cargo build --locked --release -p hydracache-loadgen -p hydracache-server"
                        .to_owned(),
                ],
            );
            let runner = RunnerFingerprint {
                runner_class: "reference-v1".to_owned(),
                fingerprint: "fixture-runner-sha".to_owned(),
                cpu_model: "fixture-cpu".to_owned(),
                logical_cores: 8,
                ram_bytes: 8 * 1024 * 1024 * 1024,
                os: "fixture-os".to_owned(),
                kernel: "fixture-kernel".to_owned(),
                cpu_affinity: "0-7".to_owned(),
                cgroup_cpu_quota: "max".to_owned(),
                governor: "performance".to_owned(),
                turbo: "disabled".to_owned(),
                shared_hardware: false,
                calibration_score: 0.01,
            };
            let profile = PerformanceProfile {
                name: REFERENCE_PROFILE.to_owned(),
                required_runner_class: "reference-v1".to_owned(),
                allowed_fingerprints: vec![runner.fingerprint.clone()],
                minimum_logical_cores: 8,
                required_cpu_affinity: "0-7".to_owned(),
                required_cgroup_cpu_quota: "max".to_owned(),
                require_dedicated: true,
                maximum_calibration_score: 0.02,
            };
            let prerequisites = ReferencePrerequisites {
                profile,
                observed_runner: runner.clone(),
                expected_build_contract: contract.clone(),
            };
            let commit = "ab".repeat(20);
            let manifest = PerfPrebuildManifest {
                schema_version: PREBUILD_MANIFEST_SCHEMA_VERSION,
                source: PrebuildSource {
                    git_commit: commit.clone(),
                    cargo_lock_sha256: sha256_file(&root.join("Cargo.lock")).unwrap(),
                },
                toolchain_identity: toolchain_identity.clone(),
                target_set: contract.target_set.clone(),
                features: contract.features.clone(),
                cargo_profile: contract.cargo_profile.clone(),
                flags: contract.flags.clone(),
                build_recipe: contract.build_recipe.clone(),
                build_contract_digest: contract.digest().unwrap(),
                runner_profile: REFERENCE_PROFILE.to_owned(),
                runner_fingerprint: runner.fingerprint.clone(),
                platform_key: "fixture-platform".to_owned(),
                binaries: vec![
                    PrebuiltBinary {
                        id: SERVER_BINARY_ID.to_owned(),
                        canonical_path: server.clone(),
                        sha256: sha256_file(&server).unwrap(),
                    },
                    PrebuiltBinary {
                        id: LOADGEN_BINARY_ID.to_owned(),
                        canonical_path: loadgen.clone(),
                        sha256: sha256_file(&loadgen).unwrap(),
                    },
                ],
            };
            let facts = LiveReferenceFacts {
                git_commit: commit,
                git_clean: true,
                cargo_lock_sha256: manifest.source.cargo_lock_sha256.clone(),
                toolchain_identity,
                current_exe: loadgen.clone(),
            };
            let manifest_path = root.join(PREBUILD_MANIFEST_RELATIVE_PATH);
            Self {
                root,
                manifest_path,
                server,
                loadgen,
                contract,
                prerequisites,
                facts,
                manifest,
            }
        }

        fn validate(&self) -> Result<ValidatedRespReferenceContext, RespReferenceError> {
            let bytes = serde_json::to_vec_pretty(&self.manifest).unwrap();
            validate_manifest(
                &self.root,
                self.manifest_path.clone(),
                &bytes,
                self.manifest.clone(),
                &self.prerequisites,
                &self.facts,
            )
        }
    }

    impl Drop for ContractFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn valid_manifest_produces_exact_ship_identities_without_invented_hashes() {
        let fixture = ContractFixture::new("valid");
        let context = fixture.validate().unwrap();
        assert_eq!(context.source.git_commit, fixture.facts.git_commit);
        assert_eq!(
            context.build.prebuild_contract_digest,
            fixture.contract.digest().unwrap()
        );
        assert_eq!(context.server.canonical_path, fixture.server);
        assert_eq!(context.loadgen.canonical_path, fixture.loadgen);
        assert_eq!(context.build.binary_sha256.len(), 2);
        assert_eq!(context.surface.surface_kind, RESP_SURFACE_KIND);
        assert_eq!(context.surface.execution_mode, RESP_EXECUTION_MODE);
        assert_eq!(context.surface.state_scope, RESP_STATE_SCOPE);
        assert_eq!(context.surface.network_boundary, RESP_NETWORK_BOUNDARY);
        assert_eq!(context.surface.claim_scope, RESP_CLAIM_SCOPE);
    }

    #[test]
    fn missing_w7_prerequisites_blocks_before_manifest_or_git_probe() {
        let missing = load_reference_context(Path::new("does-not-exist"), None).unwrap_err();
        assert!(matches!(
            missing,
            RespReferenceError::MissingW7Prerequisites
        ));
    }

    #[test]
    fn dirty_commit_and_mismatched_receipt_fields_fail_closed() {
        let mut dirty = ContractFixture::new("dirty");
        dirty.facts.git_clean = false;
        assert!(dirty.validate().unwrap_err().to_string().contains("dirty"));

        let mut lock = ContractFixture::new("lock");
        lock.manifest.source.cargo_lock_sha256 = "cd".repeat(32);
        assert!(lock
            .validate()
            .unwrap_err()
            .to_string()
            .contains("Cargo.lock"));

        let mut fingerprint = ContractFixture::new("fingerprint");
        fingerprint.manifest.runner_fingerprint = "different-runner".to_owned();
        assert!(fingerprint
            .validate()
            .unwrap_err()
            .to_string()
            .contains("fingerprint"));

        let mut contract = ContractFixture::new("contract");
        contract.manifest.build_contract_digest = "ef".repeat(32);
        assert!(contract
            .validate()
            .unwrap_err()
            .to_string()
            .contains("build_contract_digest"));
    }

    #[test]
    fn missing_swapped_or_noncanonical_binaries_fail_closed() {
        let mut missing = ContractFixture::new("missing-binary");
        missing
            .manifest
            .binaries
            .retain(|binary| binary.id != SERVER_BINARY_ID);
        assert!(missing
            .validate()
            .unwrap_err()
            .to_string()
            .contains(SERVER_BINARY_ID));

        let swapped = ContractFixture::new("swapped-binary");
        fs::write(&swapped.server, b"changed-after-prebuild").unwrap();
        assert!(swapped
            .validate()
            .unwrap_err()
            .to_string()
            .contains("SHA-256"));

        let mut noncanonical = ContractFixture::new("noncanonical-binary");
        noncanonical.manifest.binaries[0].canonical_path = PathBuf::from(format!(
            "target/release/{SERVER_BINARY_ID}{}",
            std::env::consts::EXE_SUFFIX
        ));
        assert!(noncanonical
            .validate()
            .unwrap_err()
            .to_string()
            .contains("not absolute"));
    }

    #[test]
    fn manifest_parser_rejects_unknown_fields_and_oversize_input() {
        let fixture = ContractFixture::new("strict-json");
        let mut value = serde_json::to_value(&fixture.manifest).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("unbound_claim".to_owned(), serde_json::json!(true));
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(parse_prebuild_manifest(&bytes, &fixture.manifest_path).is_err());
        assert!(parse_prebuild_manifest(
            &vec![b' '; MAX_MANIFEST_BYTES as usize + 1],
            &fixture.manifest_path
        )
        .is_err());
    }

    #[tokio::test]
    async fn readiness_accepts_only_one_exact_pong_frame() {
        for response in [
            b"+OK\r\n".as_slice(),
            b"+PONG\r\n+EXTRA\r\n".as_slice(),
            b"-ERR nope\r\n".as_slice(),
        ] {
            let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
                .await
                .unwrap();
            let endpoint = listener.local_addr().unwrap();
            let response = response.to_vec();
            let server = tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = [0_u8; 64];
                let read = stream.read(&mut request).await.unwrap();
                assert_eq!(&request[..read], b"*1\r\n$4\r\nPING\r\n");
                stream.write_all(&response).await.unwrap();
            });
            assert!(matches!(
                strict_ping_once(endpoint).await,
                Err(PingAttemptError::Fatal(_))
            ));
            server.await.unwrap();
        }

        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let endpoint = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 64];
            let read = stream.read(&mut request).await.unwrap();
            assert_eq!(&request[..read], b"*1\r\n$4\r\nPING\r\n");
            stream.write_all(b"+PONG\r\n").await.unwrap();
        });
        let (_, response) = strict_ping_once(endpoint).await.unwrap();
        assert_eq!(response, b"+PONG\r\n");
        server.await.unwrap();
    }

    #[test]
    fn repeat_directories_are_fresh_and_unique() {
        let fixture = ContractFixture::new("repeat-dir");
        let root = fixture.root.join("evidence");
        let first = create_unique_run_dir(&root, 3).unwrap();
        let second = create_unique_run_dir(&root, 3).unwrap();
        assert_ne!(first, second);
        assert!(first.join("data").is_dir());
        assert!(second.join("data").is_dir());
    }

    #[test]
    fn one_endpoint_capability_digest_binds_process_config_and_prebuild_receipt() {
        let fixture = ContractFixture::new("capability");
        let resp = SocketAddr::from((Ipv4Addr::LOCALHOST, 16_701));
        let admin = SocketAddr::from((Ipv4Addr::LOCALHOST, 16_702));
        let capability = RespEndpointCapability {
            schema_version: 1,
            pid: 42,
            started_unix_nanos: 67,
            repeat_index: 2,
            direct_prebuilt_exec: true,
            fresh_data_dir: true,
            config: RespDaemonConfigIdentity {
                role: "local".to_owned(),
                listen_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
                cluster_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
                storage_dir: fixture.root.join("fresh-data"),
                admin_enabled: true,
                admin_addr: admin,
                redis_enabled: true,
                redis_addr: resp,
                redis_auth_required: false,
                rediss_enabled: false,
            },
            selected_endpoint: format!("hydracache-server@{resp}"),
            server_binary_sha256: fixture.manifest.binaries[0].sha256.clone(),
            loadgen_binary_sha256: fixture.manifest.binaries[1].sha256.clone(),
            prebuild_manifest_sha256: "cd".repeat(32),
            prebuild_contract_digest: fixture.contract.digest().unwrap(),
            source_commit: fixture.facts.git_commit.clone(),
        };
        let digest = capability.digest().unwrap();
        assert!(valid_sha256(&digest));
        assert_eq!(capability.digest().unwrap(), digest);

        let mut different_process = capability.clone();
        different_process.pid += 1;
        assert_ne!(different_process.digest().unwrap(), digest);

        let mut forged_receipt = capability;
        forged_receipt.prebuild_manifest_sha256 = "not-a-sha".to_owned();
        assert!(forged_receipt.digest().is_err());
    }
}
