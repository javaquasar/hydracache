//! Trusted-manual qualification gate for a prospective 0.67.1 reference host.
//!
//! Qualification proves that a host is suitable for later bootstrap sampling.
//! It deliberately cannot produce ship or bootstrap-eligible evidence.

use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::Command;

use hydracache_loadgen::profile::{reference_attestation_problems, REFERENCE_RUNNER_CLASS};
use hydracache_loadgen::tiers::resp::RESP_REFERENCE_RUN_INPUTS_RELATIVE_PATH;
use serde::{Deserialize, Serialize};

use crate::perf::{
    sha256_file, verify_published_bundle, MachineAttestationReceipt, RunnerPreflightReport,
    ATTESTATION_V2_RELATIVE_PATH, RUNNER_PREFLIGHT_RELATIVE_PATH,
};

pub const QUALIFICATION_RECEIPT_RELATIVE_PATH: &str =
    "target/test-evidence/0.67.1/qualification.json";
pub const QUALIFICATION_LOCAL_RELATIVE_PATH: &str =
    "target/test-evidence/0.67.1/qualification/local-smoke.json";
pub const QUALIFICATION_CLIENT_RELATIVE_PATH: &str =
    "target/test-evidence/0.67.1/qualification/client-surface-smoke.json";

const RELEASE: &str = "0.67.1";
const PROFILE: &str = "reference-v1";
const EXPECTED_REPOSITORY: &str = "javaquasar/hydracache";
const EXPECTED_REF: &str = "refs/heads/main";
const EXPECTED_WORKFLOW_REF: &str =
    "javaquasar/hydracache/.github/workflows/ci.yml@refs/heads/main";
const MAX_JSON_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualificationContext {
    pub github_actions: String,
    pub event_name: String,
    pub git_ref: String,
    pub repository: String,
    pub head_repository: Option<String>,
    pub workflow_ref: String,
    pub performance_mode: String,
    pub candidate_release: String,
    pub runner_class: String,
    pub github_sha: String,
    pub git_head: String,
    pub github_run_id: String,
    pub clean_worktree: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationArtifactDigest {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationReceipt {
    pub schema_version: u32,
    pub release: String,
    pub profile: String,
    pub mode: String,
    pub source_commit: String,
    pub github_run_id: String,
    pub runner_fingerprint: String,
    pub attestation_sha256: String,
    pub preflight_sha256: String,
    pub prebuild_manifest_sha256: String,
    pub resp_run_inputs_sha256: String,
    pub bounded_diagnostics: Vec<QualificationArtifactDigest>,
    pub passed: bool,
    pub bootstrap_eligible: bool,
    pub ship_evidence_eligible: bool,
}

pub fn qualification_context_problems(context: &QualificationContext) -> Vec<String> {
    let mut problems = Vec::new();
    if context.github_actions != "true" {
        problems.push("qualification is restricted to GitHub Actions".to_owned());
    }
    if context.event_name != "workflow_dispatch" {
        problems.push("qualification requires an explicit workflow_dispatch".to_owned());
    }
    if context.git_ref != EXPECTED_REF {
        problems.push("qualification requires refs/heads/main".to_owned());
    }
    if context.repository != EXPECTED_REPOSITORY
        || context
            .head_repository
            .as_deref()
            .is_some_and(|repository| repository != EXPECTED_REPOSITORY)
    {
        problems.push("qualification refuses fork or foreign-repository context".to_owned());
    }
    if context.workflow_ref != EXPECTED_WORKFLOW_REF {
        problems
            .push("qualification requires the workflow definition from trusted main".to_owned());
    }
    if context.performance_mode != "qualify" {
        problems.push("qualification requires the explicit qualify mode".to_owned());
    }
    if context.candidate_release != RELEASE {
        problems.push("qualification candidate release is not 0.67.1".to_owned());
    }
    if context.runner_class != REFERENCE_RUNNER_CLASS {
        problems.push("custom label cannot substitute for the bare-metal runner class".to_owned());
    }
    if !is_git_commit(&context.github_sha) || context.github_sha != context.git_head {
        problems.push("qualification checkout does not match the dispatched commit".to_owned());
    }
    if context.github_run_id.parse::<u64>().is_err() {
        problems.push("qualification GitHub run id is missing or malformed".to_owned());
    }
    if !context.clean_worktree {
        problems.push("qualification requires an exactly clean worktree".to_owned());
    }
    problems
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let phase = parse_options(args)?;
    let root = crate::doc_check::find_repo_root()?;
    let context = observe_context(&root)?;
    let problems = qualification_context_problems(&context);
    if !problems.is_empty() {
        return Err(format!("qualification context rejected: {problems:?}").into());
    }
    if phase == "context" {
        println!("0.67.1 qualification context: OK");
        return Ok(());
    }

    let receipt = build_receipt(&root, &context)?;
    let output = root.join(QUALIFICATION_RECEIPT_RELATIVE_PATH);
    write_create_new_json(&output, &receipt)?;
    println!(
        "0.67.1 qualification passed without promotion eligibility: {}",
        output.display()
    );
    Ok(())
}

fn parse_options(args: Vec<String>) -> Result<String, Box<dyn Error>> {
    let mut release = None;
    let mut profile = None;
    let mut phase = None;
    let mut args = args.into_iter();
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--release" => release = args.next(),
            "--profile" => profile = args.next(),
            "--phase" => phase = args.next(),
            other => return Err(format!("unknown perf-qualification argument: {other}").into()),
        }
    }
    if release.as_deref() != Some(RELEASE)
        || profile.as_deref() != Some(PROFILE)
        || !matches!(phase.as_deref(), Some("context" | "finalize"))
    {
        return Err(
            "usage: perf-qualification --release 0.67.1 --profile reference-v1 --phase <context|finalize>"
                .into(),
        );
    }
    Ok(phase.expect("phase was checked"))
}

fn observe_context(root: &Path) -> Result<QualificationContext, Box<dyn Error>> {
    let git_head = git_text(root, &["rev-parse", "HEAD"])?;
    let status = Command::new("git")
        .args([
            "status",
            "--porcelain=v1",
            "--untracked-files=normal",
            "--ignore-submodules=none",
        ])
        .current_dir(root)
        .output()?;
    let clean_worktree = status.status.success() && status.stdout.is_empty();
    Ok(QualificationContext {
        github_actions: env("GITHUB_ACTIONS"),
        event_name: env("GITHUB_EVENT_NAME"),
        git_ref: env("GITHUB_REF"),
        repository: env("GITHUB_REPOSITORY"),
        head_repository: std::env::var("GITHUB_HEAD_REPOSITORY").ok(),
        workflow_ref: env("GITHUB_WORKFLOW_REF"),
        performance_mode: env("HYDRACACHE_PERFORMANCE_0671_MODE"),
        candidate_release: env("HYDRACACHE_CANDIDATE_RELEASE"),
        runner_class: env("HYDRACACHE_PERF_RUNNER_CLASS"),
        github_sha: env("GITHUB_SHA"),
        git_head,
        github_run_id: env("GITHUB_RUN_ID"),
        clean_worktree,
    })
}

fn build_receipt(
    root: &Path,
    context: &QualificationContext,
) -> Result<QualificationReceipt, Box<dyn Error>> {
    let attestation_path = root.join(ATTESTATION_V2_RELATIVE_PATH);
    let preflight_path = root.join(RUNNER_PREFLIGHT_RELATIVE_PATH);
    let attestation: MachineAttestationReceipt = read_json(&attestation_path)?;
    let preflight: RunnerPreflightReport = read_json(&preflight_path)?;
    let attestation_problems =
        reference_attestation_problems(&attestation.observed_runner.attestation);
    if attestation.schema_version != 2
        || attestation.release != RELEASE
        || attestation.profile != PROFILE
        || !attestation.passed
        || attestation.ship_evidence_eligible
        || attestation.observed_runner.runner_class != REFERENCE_RUNNER_CLASS
        || attestation.observed_runner.shared_hardware
        || !attestation_problems.is_empty()
    {
        return Err(format!(
            "machine attestation is not qualification-eligible: {attestation_problems:?}"
        )
        .into());
    }
    if preflight.release != "0.67.0"
        || preflight.profile != PROFILE
        || !preflight.passed
        || preflight.observed_runner.fingerprint != attestation.observed_runner.fingerprint
    {
        return Err("runner preflight does not bind the accepted attestation".into());
    }

    let bundle = verify_published_bundle(root)?;
    if bundle.manifest.source.git_commit != context.git_head
        || bundle.manifest.runner_fingerprint != attestation.observed_runner.fingerprint
    {
        return Err("prebuild bundle does not bind checkout and runner fingerprint".into());
    }

    let diagnostic_paths = [
        root.join(QUALIFICATION_LOCAL_RELATIVE_PATH),
        root.join(QUALIFICATION_CLIENT_RELATIVE_PATH),
    ];
    let mut bounded_diagnostics = Vec::new();
    for path in diagnostic_paths {
        validate_smoke_diagnostic(&path)?;
        bounded_diagnostics.push(QualificationArtifactDigest {
            path: relative_path(root, &path)?,
            sha256: sha256_file(&path)?,
        });
    }

    Ok(QualificationReceipt {
        schema_version: 1,
        release: RELEASE.to_owned(),
        profile: PROFILE.to_owned(),
        mode: "qualification-only".to_owned(),
        source_commit: context.git_head.clone(),
        github_run_id: context.github_run_id.clone(),
        runner_fingerprint: attestation.observed_runner.fingerprint,
        attestation_sha256: sha256_file(&attestation_path)?,
        preflight_sha256: sha256_file(&preflight_path)?,
        prebuild_manifest_sha256: bundle.manifest_sha256,
        resp_run_inputs_sha256: sha256_file(&root.join(RESP_REFERENCE_RUN_INPUTS_RELATIVE_PATH))?,
        bounded_diagnostics,
        passed: true,
        bootstrap_eligible: false,
        ship_evidence_eligible: false,
    })
}

fn validate_smoke_diagnostic(path: &Path) -> Result<(), Box<dyn Error>> {
    let report: serde_json::Value = read_json(path)?;
    if report.get("run_mode").and_then(serde_json::Value::as_str) != Some("smoke")
        || report.get("stable").and_then(serde_json::Value::as_bool) != Some(false)
        || report
            .get("runner_profile")
            .and_then(serde_json::Value::as_str)
            != Some("smoke-v1")
    {
        return Err(format!(
            "qualification diagnostic {} is not explicit non-ship smoke evidence",
            path.display()
        )
        .into());
    }
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, Box<dyn Error>> {
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 || metadata.len() > MAX_JSON_BYTES {
        return Err(format!("{} is empty or exceeds the JSON bound", path.display()).into());
    }
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn write_create_new_json(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(value)?;
    let mut output = OpenOptions::new().write(true).create_new(true).open(path)?;
    output.write_all(&bytes)?;
    output.write_all(b"\n")?;
    output.sync_all()?;
    Ok(())
}

fn git_text(root: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git").args(args).current_dir(root).output()?;
    if !output.status.success() || !output.stderr.is_empty() {
        return Err(format!("git {args:?} failed").into());
    }
    Ok(std::str::from_utf8(&output.stdout)?.trim().to_owned())
}

fn relative_path(root: &Path, path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(path
        .strip_prefix(root)?
        .to_string_lossy()
        .replace('\\', "/"))
}

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

fn is_git_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
