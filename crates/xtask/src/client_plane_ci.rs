use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

const CONTRACT_PATH: &str = "docs/testing/hc2-ci/h22-gates.json";
const WORKFLOW_PATH: &str = ".github/workflows/hc2-client-plane.yml";
const RECEIPT_SCHEMA: &str = "hydracache.hc2.ci-receipt.v1";
const CONTRACT_SCHEMA: &str = "hydracache.hc2.ci-gates.v1";
const REQUIRED_LANES: [&str; 4] = [
    "linux-required",
    "docker-interop",
    "fuzz",
    "fixed-host-soak",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GateContract {
    schema_version: String,
    workflow: String,
    artifact_retention_days: u16,
    rust_toolchain: String,
    java_version: String,
    python_version: String,
    interop_image: String,
    rust_image: String,
    maven_image: String,
    fixed_host: FixedHostContract,
    action_pins: BTreeMap<String, String>,
    lanes: Vec<GateLane>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixedHostContract {
    profile: String,
    os_id: String,
    os_version_id: String,
    architecture: String,
    labels: Vec<String>,
    preflight_script: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GateLane {
    id: String,
    job_name: String,
    runner: String,
    trigger: String,
    timeout_minutes: u16,
    release_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CiReceipt {
    pub schema_version: String,
    pub lane: String,
    pub outcome: String,
    pub commit: String,
    pub run_id: String,
    pub run_attempt: String,
    pub runner_os: String,
    pub runner_arch: String,
    pub runner_name: String,
    pub profile: String,
    pub seed: Option<u64>,
    pub iterations: Option<u32>,
    pub image: Option<String>,
}

pub fn run_check(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if !args.is_empty() {
        return Err("client-plane-ci-check does not accept arguments".into());
    }
    check_contract_at(&workspace_root()?)?;
    println!(
        "client-plane-ci-check: OK (four fail-closed H22 lanes, pinned workflow, bounded artifacts)"
    );
    Ok(())
}

pub fn run_receipt(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = parse_options(args)?;
    let lane = required(&options, "--lane")?;
    let output = PathBuf::from(required(&options, "--output")?);
    let outcome = options
        .get("--outcome")
        .map(String::as_str)
        .unwrap_or("pass");
    let seed = optional_parse::<u64>(&options, "--seed")?;
    let iterations = optional_parse::<u32>(&options, "--iterations")?;
    let image = options.get("--image").cloned();
    if lane == "docker-interop" {
        let contract = load_contract_at(&workspace_root()?)?;
        if image.as_deref() != Some(contract.interop_image.as_str()) {
            return Err("Docker interop receipt image differs from the reviewed H22 digest".into());
        }
    }
    let receipt = receipt_from_environment(lane, outcome, seed, iterations, image)?;
    write_receipt(&output, &receipt)?;
    println!("wrote HC/2 CI receipt: {}", output.display());
    Ok(())
}

pub fn run_admission(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = parse_options(args)?;
    let receipts = PathBuf::from(required(&options, "--receipts")?);
    let expected_commit = options.get("--commit").map(String::as_str);
    let root = workspace_root()?;
    check_contract_at(&root)?;
    let contract = load_contract_at(&root)?;
    let admitted = admit_receipts(&receipts, expected_commit, &contract.interop_image)?;
    println!("client-plane-ci-admission: OK (commit {admitted})");
    Ok(())
}

fn check_contract_at(root: &Path) -> Result<(), Box<dyn Error>> {
    let contract = load_contract_at(root)?;
    if contract.schema_version != CONTRACT_SCHEMA {
        return Err(format!("unexpected H22 gate schema: {}", contract.schema_version).into());
    }
    if contract.workflow != WORKFLOW_PATH {
        return Err(format!("H22 workflow must be {WORKFLOW_PATH}").into());
    }
    if !(7..=90).contains(&contract.artifact_retention_days) {
        return Err("H22 artifact retention must be between 7 and 90 days".into());
    }
    if contract.rust_toolchain != "1.94.0"
        || contract.java_version != "17"
        || contract.python_version != "3.12"
    {
        return Err("H22 language toolchains are not pinned to the reviewed versions".into());
    }
    require_digest(&contract.interop_image, "H22 interop image")?;
    require_digest(&contract.rust_image, "H22 Rust image")?;
    require_digest(&contract.maven_image, "H22 Maven image")?;
    let expected_labels = ["self-hosted", "linux", "x64", "hydracache-hc2-soak-v1"];
    if contract.fixed_host.profile != "hc2-fixed-soak-v1"
        || contract.fixed_host.os_id != "ubuntu"
        || contract.fixed_host.os_version_id != "24.04"
        || contract.fixed_host.architecture != "x86_64"
        || contract
            .fixed_host
            .labels
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            != expected_labels
        || contract.fixed_host.preflight_script != "scripts/hc2/verify-fixed-host.sh"
    {
        return Err("H22 fixed-host contract differs from the reviewed profile".into());
    }
    let expected_actions = [
        "checkout",
        "download-artifact",
        "rust-toolchain",
        "setup-java",
        "setup-python",
        "upload-artifact",
    ];
    if contract
        .action_pins
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != expected_actions.into_iter().collect()
    {
        return Err("H22 action pin set differs from the reviewed contract".into());
    }
    for (name, pin) in &contract.action_pins {
        if !is_hex_sha(pin) {
            return Err(format!("H22 action {name} is not pinned to a 40-hex commit").into());
        }
    }
    if contract.lanes.len() != REQUIRED_LANES.len() {
        return Err("H22 must define exactly four evidence lanes".into());
    }
    let mut ids = BTreeSet::new();
    for lane in &contract.lanes {
        if !REQUIRED_LANES.contains(&lane.id.as_str()) || !ids.insert(lane.id.as_str()) {
            return Err(format!("unknown or duplicate H22 lane: {}", lane.id).into());
        }
        if lane.job_name.trim().is_empty()
            || lane.runner.trim().is_empty()
            || lane.trigger.trim().is_empty()
            || !(5..=120).contains(&lane.timeout_minutes)
            || !lane.release_required
        {
            return Err(format!("invalid H22 lane contract: {}", lane.id).into());
        }
    }
    let workflow = fs::read_to_string(root.join(&contract.workflow))?;
    let fixed_host_preflight =
        fs::read_to_string(root.join(&contract.fixed_host.preflight_script))?;
    for action in contract.action_pins.values() {
        require_text(&workflow, action, "pinned action")?;
    }
    require_text(&workflow, &contract.interop_image, "pinned interop image")?;
    require_text(
        &workflow,
        &contract.fixed_host.preflight_script,
        "fixed-host preflight",
    )?;
    for required in [
        "hc2-fixed-soak-v1",
        "Ubuntu version must be 24.04",
        "checkout does not match GITHUB_SHA",
        "runner service must not execute as root",
        "rustc must be pinned to 1.94.0",
        "cargo must be pinned to 1.94.0",
    ] {
        require_text(&fixed_host_preflight, required, "fixed-host invariant")?;
    }
    let dockerfile = fs::read_to_string(root.join("scripts/hc2/Dockerfile.interop"))?;
    for image in [
        &contract.interop_image,
        &contract.rust_image,
        &contract.maven_image,
    ] {
        require_text(&dockerfile, image, "pinned Dockerfile image")?;
    }
    for lane in &contract.lanes {
        require_text(
            &workflow,
            &format!("name: {}", lane.job_name),
            "lane job name",
        )?;
        require_text(
            &workflow,
            &format!("timeout-minutes: {}", lane.timeout_minutes),
            "lane timeout",
        )?;
        require_text(
            &workflow,
            &format!("client-plane-ci-receipt --lane {}", lane.id),
            "lane receipt command",
        )?;
    }
    for required in [
        "cancel-in-progress: true",
        "client-plane-ci-check",
        "client-plane-ci-admission",
        "fuzz_hc2_client_plane",
        "timeout --signal=INT --kill-after=10s 180s",
        "fuzz_status=${PIPESTATUS[0]}",
        "\"$fuzz_status\" -ne 124",
        "-timeout=5",
        "-error_exitcode=77",
        "-timeout_exitcode=70",
        "find artifacts/fuzz_hc2_client_plane -type f",
        "hydracache-hc2-soak-v1",
        "retention-days:",
    ] {
        require_text(&workflow, required, "workflow invariant")?;
    }
    Ok(())
}

fn load_contract_at(root: &Path) -> Result<GateContract, Box<dyn Error>> {
    Ok(serde_json::from_slice(&fs::read(
        root.join(CONTRACT_PATH),
    )?)?)
}

fn receipt_from_environment(
    lane: &str,
    outcome: &str,
    seed: Option<u64>,
    iterations: Option<u32>,
    image: Option<String>,
) -> Result<CiReceipt, Box<dyn Error>> {
    validate_lane(lane)?;
    if outcome != "pass" && outcome != "fail" {
        return Err("receipt outcome must be pass or fail".into());
    }
    if lane == "fuzz" && seed.is_none() {
        return Err("fuzz receipt requires --seed".into());
    }
    if lane == "fixed-host-soak" && iterations.filter(|value| *value > 0).is_none() {
        return Err("fixed-host-soak receipt requires positive --iterations".into());
    }
    if lane == "docker-interop" {
        require_digest(
            image.as_deref().unwrap_or_default(),
            "Docker interop receipt image",
        )?;
    }
    let commit = env_or_git("GITHUB_SHA")?;
    validate_commit(&commit)?;
    let profile = if lane == "fixed-host-soak" {
        "hc2-fixed-soak-v1"
    } else {
        "hc2-correctness-v1"
    };
    Ok(CiReceipt {
        schema_version: RECEIPT_SCHEMA.to_owned(),
        lane: lane.to_owned(),
        outcome: outcome.to_owned(),
        commit,
        run_id: bounded_env("GITHUB_RUN_ID", "local", 64)?,
        run_attempt: bounded_env("GITHUB_RUN_ATTEMPT", "1", 16)?,
        runner_os: bounded_env("RUNNER_OS", env::consts::OS, 32)?,
        runner_arch: bounded_env("RUNNER_ARCH", env::consts::ARCH, 32)?,
        runner_name: bounded_env("RUNNER_NAME", "local", 128)?,
        profile: profile.to_owned(),
        seed,
        iterations,
        image,
    })
}

fn write_receipt(path: &Path, receipt: &CiReceipt) -> Result<(), Box<dyn Error>> {
    validate_receipt(receipt)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut bytes = serde_json::to_vec_pretty(receipt)?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}

fn admit_receipts(
    directory: &Path,
    expected_commit: Option<&str>,
    expected_interop_image: &str,
) -> Result<String, Box<dyn Error>> {
    let mut by_lane = BTreeMap::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".receipt.json"))
        {
            continue;
        }
        let receipt: CiReceipt = serde_json::from_slice(&fs::read(&path)?)?;
        validate_receipt(&receipt)?;
        if by_lane.insert(receipt.lane.clone(), receipt).is_some() {
            return Err(format!("duplicate H22 receipt lane in {}", path.display()).into());
        }
    }
    let missing = REQUIRED_LANES
        .iter()
        .filter(|lane| !by_lane.contains_key(**lane))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "HC/2 release admission is missing lanes: {}",
            missing.join(", ")
        )
        .into());
    }
    let commits = by_lane
        .values()
        .map(|receipt| receipt.commit.as_str())
        .collect::<BTreeSet<_>>();
    if commits.len() != 1 {
        return Err("HC/2 release admission receipts do not share one commit".into());
    }
    let commit = (*commits.iter().next().ok_or("no H22 receipts")?).to_owned();
    if expected_commit.is_some_and(|expected| expected != commit) {
        return Err(format!("HC/2 receipt commit {commit} does not match expected commit").into());
    }
    let red = by_lane
        .values()
        .filter(|receipt| receipt.outcome != "pass")
        .map(|receipt| receipt.lane.as_str())
        .collect::<Vec<_>>();
    if !red.is_empty() {
        return Err(format!("HC/2 release admission has red lanes: {}", red.join(", ")).into());
    }
    if by_lane
        .get("docker-interop")
        .and_then(|receipt| receipt.image.as_deref())
        != Some(expected_interop_image)
    {
        return Err("HC/2 Docker receipt does not bind the reviewed image digest".into());
    }
    Ok(commit)
}

fn validate_receipt(receipt: &CiReceipt) -> Result<(), Box<dyn Error>> {
    if receipt.schema_version != RECEIPT_SCHEMA {
        return Err(format!("unexpected HC/2 receipt schema: {}", receipt.schema_version).into());
    }
    validate_lane(&receipt.lane)?;
    validate_commit(&receipt.commit)?;
    if receipt.outcome != "pass" && receipt.outcome != "fail" {
        return Err("HC/2 receipt outcome must be pass or fail".into());
    }
    for (label, value, limit) in [
        ("run_id", receipt.run_id.as_str(), 64),
        ("run_attempt", receipt.run_attempt.as_str(), 16),
        ("runner_os", receipt.runner_os.as_str(), 32),
        ("runner_arch", receipt.runner_arch.as_str(), 32),
        ("runner_name", receipt.runner_name.as_str(), 128),
        ("profile", receipt.profile.as_str(), 64),
    ] {
        if value.is_empty() || value.len() > limit || value.chars().any(char::is_control) {
            return Err(format!("invalid HC/2 receipt {label}").into());
        }
    }
    let expected_profile = if receipt.lane == "fixed-host-soak" {
        "hc2-fixed-soak-v1"
    } else {
        "hc2-correctness-v1"
    };
    if receipt.profile != expected_profile {
        return Err("HC/2 receipt uses the wrong evidence profile".into());
    }
    match receipt.lane.as_str() {
        "linux-required"
            if receipt.seed.is_none()
                && receipt.iterations.is_none()
                && receipt.image.is_none() =>
        {
            Ok(())
        }
        "docker-interop"
            if receipt.seed.is_none()
                && receipt.iterations.is_none()
                && receipt.image.is_some() =>
        {
            require_digest(
                receipt.image.as_deref().unwrap_or_default(),
                "Docker interop receipt image",
            )
        }
        "fuzz"
            if receipt.seed.is_some()
                && receipt.iterations.is_none()
                && receipt.image.is_none() =>
        {
            Ok(())
        }
        "fixed-host-soak"
            if receipt.seed.is_none()
                && receipt.iterations.is_some_and(|value| value > 0)
                && receipt.image.is_none() =>
        {
            Ok(())
        }
        _ => Err("HC/2 receipt has missing or extraneous lane metadata".into()),
    }
}

fn parse_options(args: Vec<String>) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    if !args.len().is_multiple_of(2) {
        return Err("HC/2 CI options must be --name value pairs".into());
    }
    let mut options = BTreeMap::new();
    for pair in args.chunks_exact(2) {
        if !pair[0].starts_with("--") || options.insert(pair[0].clone(), pair[1].clone()).is_some()
        {
            return Err(format!("invalid or duplicate HC/2 CI option: {}", pair[0]).into());
        }
    }
    Ok(options)
}

fn required<'a>(
    options: &'a BTreeMap<String, String>,
    key: &str,
) -> Result<&'a str, Box<dyn Error>> {
    options
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("missing required option {key}").into())
}

fn optional_parse<T: std::str::FromStr>(
    options: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<T>, Box<dyn Error>>
where
    T::Err: Error + 'static,
{
    options
        .get(key)
        .map(|value| value.parse())
        .transpose()
        .map_err(Into::into)
}

fn validate_lane(lane: &str) -> Result<(), Box<dyn Error>> {
    if REQUIRED_LANES.contains(&lane) {
        Ok(())
    } else {
        Err(format!("unsupported H22 lane: {lane}").into())
    }
}

fn validate_commit(commit: &str) -> Result<(), Box<dyn Error>> {
    if commit.len() == 40 && commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err("HC/2 CI commit must be a full 40-hex SHA".into())
    }
}

fn is_hex_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn require_digest(value: &str, label: &str) -> Result<(), Box<dyn Error>> {
    let Some((name, digest)) = value.rsplit_once("@sha256:") else {
        return Err(format!("{label} is not digest pinned").into());
    };
    if name.is_empty() || digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(format!("{label} has an invalid digest").into());
    }
    Ok(())
}

fn require_text(haystack: &str, needle: &str, label: &str) -> Result<(), Box<dyn Error>> {
    if haystack.contains(needle) {
        Ok(())
    } else {
        Err(format!("H22 {label} is absent: {needle}").into())
    }
}

fn bounded_env(name: &str, default: &str, limit: usize) -> Result<String, Box<dyn Error>> {
    let value = env::var(name).unwrap_or_else(|_| default.to_owned());
    if value.is_empty() || value.len() > limit || value.chars().any(char::is_control) {
        return Err(format!("invalid environment metadata {name}").into());
    }
    Ok(value)
}

fn env_or_git(name: &str) -> Result<String, Box<dyn Error>> {
    if let Ok(value) = env::var(name) {
        return Ok(value);
    }
    let root = workspace_root()?;
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err("git rev-parse HEAD failed while constructing HC/2 receipt".into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn workspace_root() -> Result<PathBuf, Box<dyn Error>> {
    let mut candidate = env::current_dir()?;
    loop {
        if candidate.join("Cargo.toml").is_file() && candidate.join("crates/xtask").is_dir() {
            return Ok(candidate);
        }
        if !candidate.pop() {
            return Err("could not locate HydraCache workspace root".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt(lane: &str, outcome: &str, commit: &str) -> CiReceipt {
        CiReceipt {
            schema_version: RECEIPT_SCHEMA.to_owned(),
            lane: lane.to_owned(),
            outcome: outcome.to_owned(),
            commit: commit.to_owned(),
            run_id: "42".to_owned(),
            run_attempt: "1".to_owned(),
            runner_os: "Linux".to_owned(),
            runner_arch: "X64".to_owned(),
            runner_name: "bounded-test-runner".to_owned(),
            profile: if lane == "fixed-host-soak" {
                "hc2-fixed-soak-v1".to_owned()
            } else {
                "hc2-correctness-v1".to_owned()
            },
            seed: (lane == "fuzz").then_some(22),
            iterations: (lane == "fixed-host-soak").then_some(8),
            image: (lane == "docker-interop")
                .then_some(format!("ubuntu:24.04@sha256:{}", "a".repeat(64))),
        }
    }

    #[test]
    fn checked_in_contract_matches_workflow() {
        check_contract_at(&workspace_root().unwrap()).unwrap();
    }

    #[test]
    fn receipt_validation_is_bounded_and_lane_specific() {
        let sha = "1".repeat(40);
        for lane in REQUIRED_LANES {
            validate_receipt(&receipt(lane, "pass", &sha)).unwrap();
        }
        let mut invalid = receipt("fuzz", "pass", &sha);
        invalid.seed = None;
        assert!(validate_receipt(&invalid).is_err());
        invalid = receipt("docker-interop", "pass", &sha);
        invalid.image = Some("ubuntu:latest".to_owned());
        assert!(validate_receipt(&invalid).is_err());
    }
}
