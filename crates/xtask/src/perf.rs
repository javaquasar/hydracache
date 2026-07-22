//! Fail-closed prebuild gate for release-0.67 performance evidence.
//!
//! The gate is deliberately separate from every measurement command. It builds
//! the two frozen release targets once, hashes them, and publishes a manifest
//! plus the RESP reference inputs as a commit-marker pair. Consumers only read
//! these artifacts and execute the recorded binaries directly.

use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use hydracache_loadgen::profile::{PerformanceProfile, RunnerFingerprint};
use hydracache_loadgen::resp_external::{
    current_platform_key, ExternalToolExecutor, ExternalToolPrebuildReceipt,
    ExternalToolPrebuildReceiptPayload, ExternalToolProvenanceRegistry, ProcessLimits,
    SystemToolExecutor, EXTERNAL_PREBUILD_RECEIPT_VERSION, PINNED_REDIS_BENCHMARK_VERSION,
};
use hydracache_loadgen::tiers::resp::{
    RespReferenceRunInputs, RESP_REFERENCE_RUN_INPUTS_RELATIVE_PATH,
};
use hydracache_loadgen::tiers::resp_reference::{
    PerfPrebuildManifest, PrebuildSource, PrebuiltBinary, ReferenceBuildContract,
    ReferencePrerequisites, LOADGEN_BINARY_ID, PREBUILD_MANIFEST_RELATIVE_PATH,
    PREBUILD_MANIFEST_SCHEMA_VERSION, REFERENCE_PROFILE, SERVER_BINARY_ID,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::perf_budget::{Enforcement, ProfileContract, RELEASE};

pub const PROFILE_RELATIVE_PATH: &str = "docs/testing/perf-profiles/reference-v1.toml";
pub const REDIS_PROVENANCE_RELATIVE_PATH: &str =
    "docs/testing/perf-scenarios/0.67/redis-benchmark-provenance-v1.toml";
pub const REFERENCE_AUTHORIZATION_ENV: &str = "HYDRACACHE_RUN_PERF_REFERENCE";

const RELEASE_ARGUMENT: &str = "0.67";
const REQUIRED_PLATFORM: &str = "linux-x86_64";
const REDIS_PROVENANCE_ID: &str = "redis-benchmark-7.2.5-linux-x86_64-gnu-source-v1";
const MAX_CONTRACT_BYTES: u64 = 1024 * 1024;
const BUILD_ARGS: [&str; 7] = [
    "build",
    "-p",
    LOADGEN_BINARY_ID,
    "-p",
    SERVER_BINARY_ID,
    "--release",
    "--locked",
];

#[derive(Debug)]
pub struct PerfPrebuildError(String);

impl PerfPrebuildError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for PerfPrebuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for PerfPrebuildError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrebuildCommandOutput {
    pub success: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedExternalTool {
    pub platform_key: String,
    pub canonical_path: PathBuf,
    pub sha256: String,
    pub version: String,
}

/// Injectable host seam. Tests prove the ordering and failure behavior without
/// recursively invoking Cargo or depending on a particular CI machine.
pub trait PrebuildHost {
    fn run_command(
        &mut self,
        cwd: &Path,
        program: &str,
        args: &[&str],
    ) -> Result<PrebuildCommandOutput, String>;

    fn reference_platform_key(&self) -> String;

    fn environment_variable_names(&self) -> Vec<String>;

    fn observe_runner(&mut self, profile: &PerformanceProfile)
        -> Result<RunnerFingerprint, String>;

    fn observe_external_tool(&mut self, program: &str) -> Result<ObservedExternalTool, String>;
}

#[derive(Debug, Default)]
pub struct SystemPrebuildHost;

impl PrebuildHost for SystemPrebuildHost {
    fn run_command(
        &mut self,
        cwd: &Path,
        program: &str,
        args: &[&str],
    ) -> Result<PrebuildCommandOutput, String> {
        let output = Command::new(program)
            .args(args)
            .current_dir(cwd)
            .output()
            .map_err(|error| format!("unable to execute {program}: {error}"))?;
        Ok(PrebuildCommandOutput {
            success: output.status.success(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }

    fn reference_platform_key(&self) -> String {
        format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
    }

    fn environment_variable_names(&self) -> Vec<String> {
        std::env::vars_os()
            .filter_map(|(name, _)| name.into_string().ok())
            .collect()
    }

    fn observe_runner(
        &mut self,
        profile: &PerformanceProfile,
    ) -> Result<RunnerFingerprint, String> {
        observe_linux_reference_runner(profile)
    }

    fn observe_external_tool(&mut self, program: &str) -> Result<ObservedExternalTool, String> {
        let executor = SystemToolExecutor;
        let resolved = executor
            .resolve(program)
            .map_err(|error| format!("unable to resolve {program}: {}", error.message))?;
        let capture = executor
            .execute(
                &resolved,
                &["--version".to_owned()],
                ProcessLimits {
                    timeout: Duration::from_secs(10),
                    max_stdout_bytes: 4096,
                    max_stderr_bytes: 4096,
                },
            )
            .map_err(|error| format!("unable to probe {program}: {}", error.message))?;
        if capture.timed_out
            || capture.exit_code != Some(0)
            || !capture.stderr.is_empty()
            || capture.stdout.is_empty()
        {
            return Err(format!(
                "{program} version probe was not an exact successful bounded execution"
            ));
        }
        let version = String::from_utf8(capture.stdout)
            .map_err(|error| format!("{program} version is not UTF-8: {error}"))?
            .trim()
            .to_owned();
        Ok(ObservedExternalTool {
            platform_key: current_platform_key().to_owned(),
            canonical_path: resolved.canonical_path,
            sha256: resolved.binary_sha256,
            version,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrebuildOutcome {
    pub manifest_path: PathBuf,
    pub manifest_sha256: String,
    pub run_inputs_path: PathBuf,
    pub run_inputs_sha256: String,
    pub build_elapsed_millis: u64,
}

#[derive(Debug, Clone)]
pub struct VerifiedPrebuildBundle {
    pub manifest: PerfPrebuildManifest,
    pub manifest_sha256: String,
    pub run_inputs: RespReferenceRunInputs,
}

pub fn run(args: Vec<String>) -> Result<(), PerfPrebuildError> {
    let (release, profile) = parse_args(args)?;
    if std::env::var(REFERENCE_AUTHORIZATION_ENV).as_deref() != Ok("1") {
        return Err(PerfPrebuildError::new(format!(
            "{REFERENCE_AUTHORIZATION_ENV}=1 is required; reference prebuilds are dedicated-lane only"
        )));
    }
    let root = crate::doc_check::find_repo_root()
        .map_err(|error| PerfPrebuildError::new(error.to_string()))?;
    let mut host = SystemPrebuildHost;
    let outcome = execute_prebuild_with(&root, &release, &profile, &mut host)?;
    println!(
        "0.67 performance prebuild complete: manifest={} sha256={} inputs={} build_ms={}",
        outcome.manifest_path.display(),
        outcome.manifest_sha256,
        outcome.run_inputs_path.display(),
        outcome.build_elapsed_millis
    );
    Ok(())
}

pub fn execute_prebuild_with<H: PrebuildHost>(
    root: &Path,
    release: &str,
    profile_name: &str,
    host: &mut H,
) -> Result<PrebuildOutcome, PerfPrebuildError> {
    validate_request(release, profile_name)?;
    let root = fs::canonicalize(root).map_err(|error| {
        PerfPrebuildError::new(format!(
            "unable to canonicalize repository root {}: {error}",
            root.display()
        ))
    })?;
    let manifest_path = root.join(PREBUILD_MANIFEST_RELATIVE_PATH);
    let run_inputs_path = root.join(RESP_REFERENCE_RUN_INPUTS_RELATIVE_PATH);
    reject_existing_outputs(&manifest_path, &run_inputs_path)?;

    let profile = load_profile(&root)?;
    validate_profile_contract(&profile, profile_name, &host.reference_platform_key())?;
    reject_build_affecting_environment(host)?;
    let build_contract = reference_build_contract(&profile)?;
    let before = capture_source_snapshot(host, &root)?;
    if before.toolchain_identity != profile.prebuild.toolchain_identity
        || before.cargo_identity != "cargo-1.94.0"
    {
        return Err(PerfPrebuildError::new(format!(
            "live rustc/cargo toolchain {}/{} differs from the receipt-bound toolchain {}/cargo-1.94.0",
            before.toolchain_identity, before.cargo_identity, profile.prebuild.toolchain_identity
        )));
    }

    let build_started = Instant::now();
    checked_command(host, &root, "cargo", &BUILD_ARGS)?;
    let build_elapsed_millis = build_started
        .elapsed()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);

    let after_build = capture_source_snapshot(host, &root)?;
    ensure_unchanged_source(&before, &after_build, "during the release build")?;
    let runner = host
        .observe_runner(&profile.runner)
        .map_err(|error| PerfPrebuildError::new(format!("runner observation failed: {error}")))?;
    let validation = profile.runner.validate(&runner);
    if !validation.eligible || !is_sha256(&runner.fingerprint) {
        return Err(PerfPrebuildError::new(format!(
            "reference runner is ineligible: {:?}",
            validation.reasons
        )));
    }

    let binaries = collect_binaries(&root)?;
    let manifest = PerfPrebuildManifest {
        schema_version: PREBUILD_MANIFEST_SCHEMA_VERSION,
        source: PrebuildSource {
            git_commit: before.git_commit.clone(),
            cargo_lock_sha256: before.cargo_lock_sha256.clone(),
        },
        toolchain_identity: before.toolchain_identity.clone(),
        target_set: profile.prebuild.target_set.clone(),
        features: profile.prebuild.features.clone(),
        cargo_profile: profile.prebuild.cargo_profile.clone(),
        flags: profile.prebuild.flags.clone(),
        build_recipe: profile.prebuild.build_recipe.clone(),
        build_contract_digest: build_contract
            .digest()
            .map_err(|error| PerfPrebuildError::new(error.to_string()))?,
        runner_profile: profile.name.clone(),
        runner_fingerprint: runner.fingerprint.clone(),
        platform_key: profile.required_platform_key.clone(),
        binaries,
    };
    let manifest_bytes = json_bytes(&manifest, "prebuild manifest")?;
    let manifest_sha256 = sha256_bytes(&manifest_bytes);

    let external = host
        .observe_external_tool("redis-benchmark")
        .map_err(|error| {
            PerfPrebuildError::new(format!("external-tool prebuild failed: {error}"))
        })?;
    let external_sha = sha256_file(&external.canonical_path)?;
    if external.version != PINNED_REDIS_BENCHMARK_VERSION
        || external.sha256 != external_sha
        || external.platform_key != "linux-x86_64-gnu"
        || !external.canonical_path.is_absolute()
    {
        return Err(PerfPrebuildError::new(
            "redis-benchmark path, hash, version, or platform differs from the frozen contract",
        ));
    }
    let registry_path = root.join(REDIS_PROVENANCE_RELATIVE_PATH);
    let registry = ExternalToolProvenanceRegistry::load(&registry_path)
        .map_err(|error| PerfPrebuildError::new(error.to_string()))?;
    let provenance = registry
        .approved_entry(&external.platform_key, REDIS_PROVENANCE_ID)
        .ok_or_else(|| {
            PerfPrebuildError::new(
                "the exact platform has no approved redis-benchmark provenance row",
            )
        })?;
    let external_tool_prebuild =
        ExternalToolPrebuildReceipt::seal(ExternalToolPrebuildReceiptPayload {
            schema_version: EXTERNAL_PREBUILD_RECEIPT_VERSION,
            platform_key: external.platform_key,
            provenance_id: provenance.provenance_id.clone(),
            provenance_registry_sha256: registry.digest(),
            source_archive_sha256: provenance
                .provenance
                .source_archive_sha256()
                .map(str::to_owned),
            tool_binary_id: "redis-benchmark".to_owned(),
            tool_canonical_path: external.canonical_path,
            tool_binary_sha256: external_sha,
            prebuild_manifest_sha256: manifest_sha256.clone(),
        });
    let run_inputs = RespReferenceRunInputs {
        prerequisites: ReferencePrerequisites {
            profile: profile.runner.clone(),
            observed_runner: runner,
            expected_build_contract: build_contract,
        },
        external_tool_prebuild,
    };
    let run_inputs_bytes = json_bytes(&run_inputs, "RESP reference run inputs")?;
    let run_inputs_sha256 = sha256_bytes(&run_inputs_bytes);

    let before_publish = capture_source_snapshot(host, &root)?;
    ensure_unchanged_source(&before, &before_publish, "before artifact publication")?;
    verify_binary_entries(&root, &manifest.binaries)?;
    publish_pair_commit_marker(
        &root,
        &manifest_path,
        &manifest_bytes,
        &run_inputs_path,
        &run_inputs_bytes,
    )?;

    let verified = verify_published_bundle(&root).inspect_err(|_| {
        let _ = remove_published_pair(&manifest_path, &run_inputs_path);
    })?;
    if verified.manifest_sha256 != manifest_sha256 {
        let _ = remove_published_pair(&manifest_path, &run_inputs_path);
        return Err(PerfPrebuildError::new(
            "published manifest digest differs from the prepared commit marker",
        ));
    }
    let after_publish = capture_source_snapshot(host, &root)?;
    if let Err(error) = ensure_unchanged_source(&before, &after_publish, "after publication") {
        let _ = remove_published_pair(&manifest_path, &run_inputs_path);
        return Err(error);
    }

    Ok(PrebuildOutcome {
        manifest_path,
        manifest_sha256,
        run_inputs_path,
        run_inputs_sha256,
        build_elapsed_millis,
    })
}

/// Read-only consumer validation. It never creates, deletes, rebuilds, or
/// rewrites either prebuild artifact.
pub fn verify_published_bundle(root: &Path) -> Result<VerifiedPrebuildBundle, PerfPrebuildError> {
    let root = fs::canonicalize(root).map_err(|error| {
        PerfPrebuildError::new(format!("unable to canonicalize consumer root: {error}"))
    })?;
    let manifest_path = root.join(PREBUILD_MANIFEST_RELATIVE_PATH);
    let inputs_path = root.join(RESP_REFERENCE_RUN_INPUTS_RELATIVE_PATH);
    let manifest_bytes = read_bounded(&manifest_path)?;
    let inputs_bytes = read_bounded(&inputs_path)?;
    let manifest: PerfPrebuildManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| PerfPrebuildError::new(format!("invalid prebuild manifest: {error}")))?;
    let run_inputs: RespReferenceRunInputs =
        serde_json::from_slice(&inputs_bytes).map_err(|error| {
            PerfPrebuildError::new(format!("invalid RESP reference run inputs: {error}"))
        })?;
    let manifest_sha256 = sha256_bytes(&manifest_bytes);
    let profile = load_profile(&root)?;
    validate_profile_contract(&profile, REFERENCE_PROFILE, REQUIRED_PLATFORM)?;
    let expected_contract = reference_build_contract(&profile)?;
    let resealed =
        ExternalToolPrebuildReceipt::seal(run_inputs.external_tool_prebuild.payload.clone());
    let registry = ExternalToolProvenanceRegistry::load(&root.join(REDIS_PROVENANCE_RELATIVE_PATH))
        .map_err(|error| PerfPrebuildError::new(error.to_string()))?;
    let tool = &run_inputs.external_tool_prebuild.payload;
    let provenance = registry
        .approved_entry(&tool.platform_key, REDIS_PROVENANCE_ID)
        .ok_or_else(|| PerfPrebuildError::new("published inputs lost approved tool provenance"))?;
    if manifest.schema_version != PREBUILD_MANIFEST_SCHEMA_VERSION
        || !is_git_commit(&manifest.source.git_commit)
        || !is_sha256(&manifest.source.cargo_lock_sha256)
        || manifest.toolchain_identity != profile.prebuild.toolchain_identity
        || manifest.platform_key != profile.required_platform_key
        || manifest.target_set != [LOADGEN_BINARY_ID, SERVER_BINARY_ID]
        || manifest.features != profile.prebuild.features
        || manifest.cargo_profile != profile.prebuild.cargo_profile
        || manifest.flags != profile.prebuild.flags
        || manifest.build_recipe != profile.prebuild.build_recipe
        || manifest.build_contract_digest != profile.prebuild.digest
        || manifest.build_contract_digest
            != expected_contract
                .digest()
                .map_err(|error| PerfPrebuildError::new(error.to_string()))?
        || manifest.runner_profile != profile.name
        || manifest.runner_fingerprint != run_inputs.prerequisites.observed_runner.fingerprint
        || run_inputs.prerequisites.profile != profile.runner
        || run_inputs.prerequisites.expected_build_contract != expected_contract
        || !run_inputs
            .prerequisites
            .profile
            .validate(&run_inputs.prerequisites.observed_runner)
            .eligible
        || run_inputs
            .external_tool_prebuild
            .payload
            .prebuild_manifest_sha256
            != manifest_sha256
        || run_inputs.external_tool_prebuild != resealed
        || tool.schema_version != EXTERNAL_PREBUILD_RECEIPT_VERSION
        || tool.platform_key != provenance.platform_key
        || tool.provenance_id != provenance.provenance_id
        || tool.provenance_registry_sha256 != registry.digest()
        || tool.source_archive_sha256.as_deref() != provenance.provenance.source_archive_sha256()
        || tool.tool_binary_id != "redis-benchmark"
        || !tool.tool_canonical_path.is_absolute()
        || sha256_file(&root.join("Cargo.lock"))? != manifest.source.cargo_lock_sha256
    {
        return Err(PerfPrebuildError::new(
            "published prebuild artifacts are incomplete, mismatched, or not receipt-bound",
        ));
    }
    verify_binary_entries(&root, &manifest.binaries)?;
    if sha256_file(&tool.tool_canonical_path)? != tool.tool_binary_sha256 {
        return Err(PerfPrebuildError::new(
            "recorded redis-benchmark binary changed after prebuild",
        ));
    }
    Ok(VerifiedPrebuildBundle {
        manifest,
        manifest_sha256,
        run_inputs,
    })
}

fn parse_args(args: Vec<String>) -> Result<(String, String), PerfPrebuildError> {
    let mut release = None;
    let mut profile = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--release" => release = Some(next_value(&mut args, "--release")?),
            "--profile" => profile = Some(next_value(&mut args, "--profile")?),
            _ => return Err(PerfPrebuildError::new(format!("unknown argument {arg:?}"))),
        }
    }
    let release = release.ok_or_else(|| PerfPrebuildError::new("--release is required"))?;
    let profile = profile.ok_or_else(|| PerfPrebuildError::new("--profile is required"))?;
    validate_request(&release, &profile)?;
    Ok((release, profile))
}

fn next_value(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<String, PerfPrebuildError> {
    args.next()
        .ok_or_else(|| PerfPrebuildError::new(format!("{flag} requires a value")))
}

fn validate_request(release: &str, profile: &str) -> Result<(), PerfPrebuildError> {
    if release != RELEASE_ARGUMENT || profile != REFERENCE_PROFILE {
        return Err(PerfPrebuildError::new(format!(
            "perf-prebuild supports only --release {RELEASE_ARGUMENT} --profile {REFERENCE_PROFILE}"
        )));
    }
    Ok(())
}

fn load_profile(root: &Path) -> Result<ProfileContract, PerfPrebuildError> {
    let path = root.join(PROFILE_RELATIVE_PATH);
    let bytes = read_bounded(&path)?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|error| PerfPrebuildError::new(format!("profile is not UTF-8: {error}")))?;
    toml::from_str(text)
        .map_err(|error| PerfPrebuildError::new(format!("invalid reference profile: {error}")))
}

fn validate_profile_contract(
    profile: &ProfileContract,
    requested: &str,
    platform: &str,
) -> Result<(), PerfPrebuildError> {
    let exact = profile.schema_version == 1
        && profile.release == RELEASE
        && profile.name == requested
        && profile.name == REFERENCE_PROFILE
        && profile.enforcement == Enforcement::Ship
        && profile.required_platform_key == REQUIRED_PLATFORM
        && platform == REQUIRED_PLATFORM
        && profile.runner.name == REFERENCE_PROFILE
        && profile.runner.required_runner_class == REFERENCE_PROFILE
        && profile.runner.minimum_logical_cores == 8
        && profile.runner.required_cpu_affinity == "dedicated-cpuset"
        && profile.runner.required_cgroup_cpu_quota == "unlimited"
        && profile.runner.require_dedicated
        && (profile.runner.maximum_calibration_score - 0.05).abs() < f64::EPSILON
        && profile.prebuild.schema_version == 1
        && profile.prebuild.toolchain_identity == "rustc-1.94.0"
        && profile.prebuild.target_set == [LOADGEN_BINARY_ID, SERVER_BINARY_ID]
        && profile.prebuild.features.is_empty()
        && profile.prebuild.cargo_profile == "release"
        && profile.prebuild.flags == ["--locked", "--release"]
        && profile.prebuild.build_recipe
            == ["cargo build -p hydracache-loadgen -p hydracache-server --release --locked"]
        && profile.prebuild.computed_digest() == profile.prebuild.digest
        && is_sha256(&profile.prebuild.digest)
        && profile.runner.contract_problems().is_empty();
    if !exact {
        return Err(PerfPrebuildError::new(
            "reference-v1 profile changed the frozen release/platform/build contract",
        ));
    }
    Ok(())
}

fn reference_build_contract(
    profile: &ProfileContract,
) -> Result<ReferenceBuildContract, PerfPrebuildError> {
    let contract = ReferenceBuildContract::new(
        profile.prebuild.toolchain_identity.clone(),
        profile.prebuild.target_set.clone(),
        profile.prebuild.features.clone(),
        profile.prebuild.cargo_profile.clone(),
        profile.prebuild.flags.clone(),
        profile.prebuild.build_recipe.clone(),
    );
    let digest = contract
        .digest()
        .map_err(|error| PerfPrebuildError::new(error.to_string()))?;
    if digest != profile.prebuild.digest {
        return Err(PerfPrebuildError::new(
            "loadgen and xtask build-contract digests disagree",
        ));
    }
    Ok(contract)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceSnapshot {
    git_commit: String,
    cargo_lock_sha256: String,
    toolchain_identity: String,
    cargo_identity: String,
}

fn capture_source_snapshot<H: PrebuildHost>(
    host: &mut H,
    root: &Path,
) -> Result<SourceSnapshot, PerfPrebuildError> {
    let top = checked_command(host, root, "git", &["rev-parse", "--show-toplevel"])?;
    let top = canonical_output_path(&top.stdout, "git top-level")?;
    if top != root {
        return Err(PerfPrebuildError::new(
            "requested root differs from the exact git top-level",
        ));
    }
    let commit = checked_text(host, root, "git", &["rev-parse", "HEAD"])?;
    if !is_git_commit(&commit) {
        return Err(PerfPrebuildError::new(
            "git HEAD is not a canonical full commit",
        ));
    }
    let status = checked_command(
        host,
        root,
        "git",
        &[
            "status",
            "--porcelain=v1",
            "--untracked-files=normal",
            "--ignore-submodules=none",
        ],
    )?;
    if !status.stdout.is_empty() {
        return Err(PerfPrebuildError::new(
            "reference prebuild requires an exactly clean working tree",
        ));
    }
    let rustc = checked_text(host, root, "rustc", &["--version"])?;
    let toolchain_identity = canonical_toolchain_identity(&rustc)?;
    let cargo = checked_text(host, root, "cargo", &["--version"])?;
    let cargo_identity = canonical_cargo_identity(&cargo)?;
    Ok(SourceSnapshot {
        git_commit: commit,
        cargo_lock_sha256: sha256_file(&root.join("Cargo.lock"))?,
        toolchain_identity,
        cargo_identity,
    })
}

fn reject_build_affecting_environment(host: &impl PrebuildHost) -> Result<(), PerfPrebuildError> {
    const EXACT: [&str; 8] = [
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTC",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_BUILD_RUSTC",
        "CARGO_BUILD_TARGET",
        "CARGO_TARGET_DIR",
    ];
    let mut rejected = host
        .environment_variable_names()
        .into_iter()
        .filter(|name| {
            EXACT.contains(&name.as_str())
                || name.starts_with("CARGO_PROFILE_RELEASE_")
                || (name.starts_with("CARGO_TARGET_") && name.ends_with("_RUSTFLAGS"))
        })
        .collect::<Vec<_>>();
    rejected.sort();
    rejected.dedup();
    if rejected.is_empty() {
        Ok(())
    } else {
        Err(PerfPrebuildError::new(format!(
            "reference prebuild rejects ambient build-affecting environment variables: {}",
            rejected.join(", ")
        )))
    }
}

fn ensure_unchanged_source(
    expected: &SourceSnapshot,
    observed: &SourceSnapshot,
    phase: &str,
) -> Result<(), PerfPrebuildError> {
    if expected != observed {
        return Err(PerfPrebuildError::new(format!(
            "commit, Cargo.lock, or toolchain changed {phase}"
        )));
    }
    Ok(())
}

fn collect_binaries(root: &Path) -> Result<Vec<PrebuiltBinary>, PerfPrebuildError> {
    [LOADGEN_BINARY_ID, SERVER_BINARY_ID]
        .into_iter()
        .map(|id| {
            let expected = root.join("target").join("release").join(id);
            let canonical = fs::canonicalize(&expected).map_err(|error| {
                PerfPrebuildError::new(format!(
                    "required prebuilt binary {} is unavailable: {error}",
                    expected.display()
                ))
            })?;
            if canonical != expected || !canonical.is_file() || !canonical.starts_with(root) {
                return Err(PerfPrebuildError::new(format!(
                    "prebuilt {id} path is not the exact canonical target/release path"
                )));
            }
            Ok(PrebuiltBinary {
                id: id.to_owned(),
                sha256: sha256_file(&canonical)?,
                canonical_path: canonical,
            })
        })
        .collect()
}

fn verify_binary_entries(
    root: &Path,
    binaries: &[PrebuiltBinary],
) -> Result<(), PerfPrebuildError> {
    if binaries.len() != 2 {
        return Err(PerfPrebuildError::new(
            "prebuild manifest must contain exactly loadgen and server",
        ));
    }
    for (binary, expected_id) in binaries.iter().zip([LOADGEN_BINARY_ID, SERVER_BINARY_ID]) {
        let expected = root.join("target").join("release").join(expected_id);
        if binary.id != expected_id
            || binary.canonical_path != expected
            || fs::canonicalize(&binary.canonical_path).ok().as_ref() != Some(&expected)
            || sha256_file(&binary.canonical_path)? != binary.sha256
        {
            return Err(PerfPrebuildError::new(format!(
                "prebuilt binary {expected_id} no longer matches its exact path/hash receipt"
            )));
        }
    }
    Ok(())
}

fn publish_pair_commit_marker(
    root: &Path,
    manifest_path: &Path,
    manifest_bytes: &[u8],
    inputs_path: &Path,
    inputs_bytes: &[u8],
) -> Result<(), PerfPrebuildError> {
    reject_existing_outputs(manifest_path, inputs_path)?;
    let parent = manifest_path
        .parent()
        .ok_or_else(|| PerfPrebuildError::new("manifest path has no parent"))?;
    if inputs_path.parent() != Some(parent) {
        return Err(PerfPrebuildError::new(
            "prebuild artifacts must share one exact publication directory",
        ));
    }
    fs::create_dir_all(parent).map_err(|error| {
        PerfPrebuildError::new(format!("unable to create evidence directory: {error}"))
    })?;
    let canonical_parent = fs::canonicalize(parent).map_err(|error| {
        PerfPrebuildError::new(format!(
            "unable to canonicalize evidence directory: {error}"
        ))
    })?;
    if canonical_parent != parent || !canonical_parent.starts_with(root) {
        return Err(PerfPrebuildError::new(
            "prebuild artifact directory is a symlink or escapes the canonical repository root",
        ));
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| PerfPrebuildError::new(error.to_string()))?
        .as_nanos();
    let manifest_tmp = parent.join(format!(
        ".prebuild-manifest.{}.{nonce}.tmp",
        std::process::id()
    ));
    let inputs_tmp = parent.join(format!(
        ".resp-reference-inputs.{}.{nonce}.tmp",
        std::process::id()
    ));
    let result = (|| {
        write_create_new(&manifest_tmp, manifest_bytes)?;
        write_create_new(&inputs_tmp, inputs_bytes)?;
        // Inputs are promoted first. The manifest is the commit marker and is
        // visible only after the complete, manifest-bound input file exists.
        promote_create_new(&inputs_tmp, inputs_path, "run inputs")?;
        if let Err(error) =
            promote_create_new(&manifest_tmp, manifest_path, "manifest commit marker")
        {
            let _ = fs::remove_file(inputs_path);
            return Err(error);
        }
        if let Err(error) = sync_directory(parent) {
            let _ = fs::remove_file(manifest_path);
            let _ = fs::remove_file(inputs_path);
            return Err(error);
        }
        Ok(())
    })();
    let _ = fs::remove_file(&manifest_tmp);
    let _ = fs::remove_file(&inputs_tmp);
    result
}

fn promote_create_new(
    temporary: &Path,
    destination: &Path,
    label: &str,
) -> Result<(), PerfPrebuildError> {
    // `rename` replaces an existing destination on Unix. A same-directory hard
    // link gives us an atomic create-new name instead, so a racing/stale
    // artifact can never be overwritten.
    fs::hard_link(temporary, destination).map_err(|error| {
        PerfPrebuildError::new(format!(
            "unable to promote {label} without overwrite: {error}"
        ))
    })?;
    if let Err(error) = fs::remove_file(temporary) {
        let _ = fs::remove_file(destination);
        return Err(PerfPrebuildError::new(format!(
            "unable to retire staged {label}: {error}"
        )));
    }
    Ok(())
}

fn reject_existing_outputs(manifest: &Path, inputs: &Path) -> Result<(), PerfPrebuildError> {
    if manifest.exists() || inputs.exists() {
        return Err(PerfPrebuildError::new(
            "prebuild outputs already exist; stale evidence is never overwritten",
        ));
    }
    Ok(())
}

fn remove_published_pair(manifest: &Path, inputs: &Path) -> Result<(), PerfPrebuildError> {
    for path in [manifest, inputs] {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(PerfPrebuildError::new(format!(
                    "unable to remove failed publication {}: {error}",
                    path.display()
                )))
            }
        }
    }
    Ok(())
}

fn write_create_new(path: &Path, bytes: &[u8]) -> Result<(), PerfPrebuildError> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| PerfPrebuildError::new(format!("creating {}: {error}", path.display())))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| PerfPrebuildError::new(format!("writing {}: {error}", path.display())))
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), PerfPrebuildError> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| PerfPrebuildError::new(format!("syncing {}: {error}", path.display())))
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), PerfPrebuildError> {
    Ok(())
}

fn checked_text<H: PrebuildHost>(
    host: &mut H,
    cwd: &Path,
    program: &str,
    args: &[&str],
) -> Result<String, PerfPrebuildError> {
    let output = checked_command(host, cwd, program, args)?;
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|error| PerfPrebuildError::new(format!("{program} output is not UTF-8: {error}")))
}

fn checked_command<H: PrebuildHost>(
    host: &mut H,
    cwd: &Path,
    program: &str,
    args: &[&str],
) -> Result<PrebuildCommandOutput, PerfPrebuildError> {
    let output = host
        .run_command(cwd, program, args)
        .map_err(PerfPrebuildError::new)?;
    if !output.success {
        return Err(PerfPrebuildError::new(format!(
            "{program} {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output)
}

fn canonical_output_path(bytes: &[u8], label: &str) -> Result<PathBuf, PerfPrebuildError> {
    let value = String::from_utf8(bytes.to_vec())
        .map_err(|error| PerfPrebuildError::new(format!("{label} is not UTF-8: {error}")))?;
    fs::canonicalize(value.trim())
        .map_err(|error| PerfPrebuildError::new(format!("invalid {label}: {error}")))
}

fn canonical_toolchain_identity(raw: &str) -> Result<String, PerfPrebuildError> {
    let mut fields = raw.split_whitespace();
    if fields.next() != Some("rustc") {
        return Err(PerfPrebuildError::new("rustc version probe is malformed"));
    }
    let version = fields
        .next()
        .ok_or_else(|| PerfPrebuildError::new("rustc version is absent"))?;
    let parts = version.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(PerfPrebuildError::new(
            "rustc version is not exact semantic version",
        ));
    }
    Ok(format!("rustc-{version}"))
}

fn canonical_cargo_identity(raw: &str) -> Result<String, PerfPrebuildError> {
    let mut fields = raw.split_whitespace();
    if fields.next() != Some("cargo") {
        return Err(PerfPrebuildError::new("cargo version probe is malformed"));
    }
    let version = fields
        .next()
        .ok_or_else(|| PerfPrebuildError::new("cargo version is absent"))?;
    let parts = version.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(PerfPrebuildError::new(
            "cargo version is not exact semantic version",
        ));
    }
    Ok(format!("cargo-{version}"))
}

fn json_bytes<T: Serialize>(value: &T, label: &str) -> Result<Vec<u8>, PerfPrebuildError> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| PerfPrebuildError::new(format!("serializing {label}: {error}")))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, PerfPrebuildError> {
    let metadata = fs::metadata(path).map_err(|error| {
        PerfPrebuildError::new(format!("reading metadata for {}: {error}", path.display()))
    })?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_CONTRACT_BYTES {
        return Err(PerfPrebuildError::new(format!(
            "{} must be a regular bounded non-empty file",
            path.display()
        )));
    }
    fs::read(path)
        .map_err(|error| PerfPrebuildError::new(format!("reading {}: {error}", path.display())))
}

pub fn sha256_file(path: &Path) -> Result<String, PerfPrebuildError> {
    let mut file = File::open(path)
        .map_err(|error| PerfPrebuildError::new(format!("opening {}: {error}", path.display())))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            PerfPrebuildError::new(format!("reading {}: {error}", path.display()))
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_digest(&hasher.finalize()))
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex_digest(&hasher.finalize())
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn is_git_commit(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn observe_linux_reference_runner(
    profile: &PerformanceProfile,
) -> Result<RunnerFingerprint, String> {
    if std::env::consts::OS != "linux" || std::env::consts::ARCH != "x86_64" {
        return Err("reference-v1 requires Linux x86_64".to_owned());
    }
    if std::env::var(REFERENCE_AUTHORIZATION_ENV).as_deref() != Ok("1") {
        return Err("reference lane authorization is absent".to_owned());
    }
    let cpu_model = proc_value("/proc/cpuinfo", "model name")?;
    let cpu_affinity_raw = proc_value("/proc/self/status", "Cpus_allowed_list")?;
    let logical_cores = count_cpu_list(&cpu_affinity_raw)?;
    if logical_cores < profile.minimum_logical_cores {
        return Err("dedicated cpuset is below the profile minimum".to_owned());
    }
    let quota_raw = read_cpu_quota()?;
    if quota_raw != "max" && quota_raw != "-1" {
        return Err(format!(
            "reference cgroup CPU quota is limited: {quota_raw}"
        ));
    }
    let ram_kib = proc_value("/proc/meminfo", "MemTotal")?
        .split_whitespace()
        .next()
        .ok_or_else(|| "MemTotal is malformed".to_owned())?
        .parse::<u64>()
        .map_err(|error| format!("MemTotal is malformed: {error}"))?;
    let ram_bytes = ram_kib
        .checked_mul(1024)
        .ok_or_else(|| "MemTotal overflow".to_owned())?;
    let kernel = fs::read_to_string("/proc/sys/kernel/osrelease")
        .map_err(|error| format!("kernel probe failed: {error}"))?
        .trim()
        .to_owned();
    let governor = read_required_trimmed(
        "/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor",
        "CPU governor",
    )?;
    let turbo = read_turbo_state()?;
    let calibration_score = calibration_score();
    #[derive(Serialize)]
    struct StableFingerprint<'a> {
        schema_version: u32,
        runner_class: &'a str,
        cpu_model: &'a str,
        logical_cores: u32,
        ram_bytes: u64,
        os: &'a str,
        kernel: &'a str,
        cpu_affinity_raw: &'a str,
        cgroup_cpu_quota_raw: &'a str,
        governor: &'a str,
        turbo: &'a str,
        shared_hardware: bool,
    }
    let stable = StableFingerprint {
        schema_version: 1,
        runner_class: &profile.required_runner_class,
        cpu_model: &cpu_model,
        logical_cores,
        ram_bytes,
        os: "linux",
        kernel: &kernel,
        cpu_affinity_raw: &cpu_affinity_raw,
        cgroup_cpu_quota_raw: &quota_raw,
        governor: &governor,
        turbo: &turbo,
        shared_hardware: false,
    };
    let stable_bytes = serde_json::to_vec(&stable).map_err(|error| error.to_string())?;
    Ok(RunnerFingerprint {
        runner_class: profile.required_runner_class.clone(),
        fingerprint: sha256_bytes(&stable_bytes),
        cpu_model,
        logical_cores,
        ram_bytes,
        os: "linux".to_owned(),
        kernel,
        cpu_affinity: profile.required_cpu_affinity.clone(),
        cgroup_cpu_quota: profile.required_cgroup_cpu_quota.clone(),
        governor,
        turbo,
        shared_hardware: false,
        calibration_score,
    })
}

fn proc_value(path: &str, key: &str) -> Result<String, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("reading {path}: {error}"))?;
    text.lines()
        .find_map(|line| {
            let (candidate, value) = line.split_once(':')?;
            (candidate.trim() == key).then(|| value.trim().to_owned())
        })
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{key} is absent from {path}"))
}

fn count_cpu_list(value: &str) -> Result<u32, String> {
    let mut count = 0_u32;
    for item in value.split(',') {
        let (start, end) = match item.split_once('-') {
            Some((start, end)) => (start, end),
            None => (item, item),
        };
        let start = start
            .parse::<u32>()
            .map_err(|error| format!("invalid CPU list {value:?}: {error}"))?;
        let end = end
            .parse::<u32>()
            .map_err(|error| format!("invalid CPU list {value:?}: {error}"))?;
        if end < start {
            return Err(format!("invalid CPU list range {item:?}"));
        }
        count = count
            .checked_add(end - start + 1)
            .ok_or_else(|| "CPU list count overflow".to_owned())?;
    }
    Ok(count)
}

fn read_cpu_quota() -> Result<String, String> {
    if let Ok(value) = fs::read_to_string("/sys/fs/cgroup/cpu.max") {
        return value
            .split_whitespace()
            .next()
            .map(str::to_owned)
            .ok_or_else(|| "cgroup v2 cpu.max is empty".to_owned());
    }
    read_required_trimmed("/sys/fs/cgroup/cpu/cpu.cfs_quota_us", "cgroup v1 CPU quota")
}

fn read_turbo_state() -> Result<String, String> {
    for (path, prefix) in [
        (
            "/sys/devices/system/cpu/intel_pstate/no_turbo",
            "intel-no-turbo",
        ),
        ("/sys/devices/system/cpu/cpufreq/boost", "cpufreq-boost"),
    ] {
        if let Ok(value) = fs::read_to_string(path) {
            let value = value.trim();
            if !value.is_empty() {
                return Ok(format!("{prefix}:{value}"));
            }
        }
    }
    Err("CPU turbo/boost state is unavailable".to_owned())
}

fn read_required_trimmed(path: &str, label: &str) -> Result<String, String> {
    let value = fs::read_to_string(path)
        .map_err(|error| format!("{label} probe failed at {path}: {error}"))?;
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{label} probe is empty"));
    }
    Ok(value.to_owned())
}

fn calibration_score() -> f64 {
    let mut samples = Vec::with_capacity(7);
    for repeat in 0_u64..7 {
        let started = Instant::now();
        let mut state = 0x9e37_79b9_7f4a_7c15_u64 ^ repeat;
        for index in 0_u64..1_000_000 {
            state = state
                .wrapping_add(index ^ 0xa076_1d64_78bd_642f)
                .rotate_left(17)
                .wrapping_mul(0xe703_7ed1_a0b4_28db);
        }
        std::hint::black_box(state);
        samples.push(started.elapsed().as_nanos() as f64);
    }
    samples.sort_by(f64::total_cmp);
    let median = samples[samples.len() / 2];
    let mut deviations = samples
        .iter()
        .map(|sample| (sample - median).abs())
        .collect::<Vec<_>>();
    deviations.sort_by(f64::total_cmp);
    if median <= 0.0 {
        f64::INFINITY
    } else {
        deviations[deviations.len() / 2] / median
    }
}
