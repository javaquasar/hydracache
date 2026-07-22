use std::error::Error;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::doc_check;

pub const CONFIG_PATH: &str = "docs/testing/coverage-ratchet.toml";
pub const MINIMUM_FLOOR_PERCENT: f64 = 88.0;
const REVIEWED_IGNORED_SOURCE_REGEX: &str = "(^|/)crates/(xtask|hydracache-loadgen)/";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageRatchet {
    pub schema_version: u32,
    pub release: String,
    pub tool: String,
    pub tool_version: String,
    pub minimum_floor_percent: f64,
    pub configured_floor_percent: f64,
    pub baseline_status: BaselineStatus,
    pub baseline_commit: String,
    pub baseline_toolchain: String,
    pub baseline_lines_percent: f64,
    pub ignored_source_regex: String,
    pub raw_report_artifact: String,
    pub evidence_artifact: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BaselineStatus {
    Unmeasured,
    Measured,
}

#[derive(Debug, Serialize)]
struct CoverageEvidence {
    schema_version: u32,
    release: String,
    source_commit: String,
    tool_version: String,
    rustc_version: String,
    configured_floor_percent: f64,
    measured_lines_percent: f64,
    ignored_source_regex: String,
    raw_report_artifact: String,
    profile_steps: Vec<CoverageStepEvidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageStepKind {
    Clean,
    DefaultTests,
    AdditiveTests,
    Report,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageStep {
    pub id: &'static str,
    pub kind: CoverageStepKind,
    pub args: Vec<String>,
    pub environment: Vec<(&'static str, &'static str)>,
}

#[derive(Debug, Serialize)]
struct CoverageStepEvidence {
    id: &'static str,
    kind: CoverageStepKind,
    command: Vec<String>,
    environment: Vec<(&'static str, &'static str)>,
    status: &'static str,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let (root, execute) = parse_args(args)?;
    let config = load_config(&root)?;
    let problems = validate_contract(&root, &config)?;
    if !problems.is_empty() {
        for problem in &problems {
            eprintln!("coverage-ratchet-check: {problem}");
        }
        return Err(format!("coverage-ratchet-check found {} problem(s)", problems.len()).into());
    }
    if execute {
        execute_measurement(&root, &config)?;
    } else {
        println!(
            "coverage-ratchet-check: OK (structural, floor {:.0}%, baseline {:?})",
            config.configured_floor_percent, config.baseline_status
        );
    }
    Ok(())
}

pub fn load_config(root: &Path) -> Result<CoverageRatchet, Box<dyn Error>> {
    let path = root.join(CONFIG_PATH);
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("reading {}: {error}", path.display()))?;
    toml::from_str(&text).map_err(|error| format!("parsing {CONFIG_PATH}: {error}").into())
}

pub fn loadgen_manifest_is_development_only(text: &str) -> bool {
    let Ok(manifest) = toml::from_str::<toml::Value>(text) else {
        return false;
    };
    manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .and_then(|package| package.get("publish"))
        .and_then(toml::Value::as_bool)
        == Some(false)
}

pub fn validate_contract(
    root: &Path,
    config: &CoverageRatchet,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut problems = Vec::new();
    if config.schema_version != 1 {
        problems.push("schema_version must be 1".to_owned());
    }
    if normalize_release(&config.release) != "0.64.0" {
        problems.push("release must be 0.64.0".to_owned());
    }
    if config.tool != "cargo-llvm-cov" || config.tool_version != "0.8.7" {
        problems.push("cargo-llvm-cov must remain pinned to reviewed version 0.8.7".to_owned());
    }
    if config.minimum_floor_percent != MINIMUM_FLOOR_PERCENT
        || config.configured_floor_percent < MINIMUM_FLOOR_PERCENT
    {
        problems.push("coverage floor may not decrease below 88%".to_owned());
    }
    if config.ignored_source_regex != REVIEWED_IGNORED_SOURCE_REGEX {
        problems.push(format!(
            "coverage exclusion must remain exactly {REVIEWED_IGNORED_SOURCE_REGEX:?}"
        ));
    }
    let loadgen_manifest = fs::read_to_string(root.join("crates/hydracache-loadgen/Cargo.toml"))?;
    if !loadgen_manifest_is_development_only(&loadgen_manifest) {
        problems.push(
            "hydracache-loadgen may be excluded only while package.publish is false".to_owned(),
        );
    }

    match config.baseline_status {
        BaselineStatus::Unmeasured => {
            if !config.baseline_commit.is_empty()
                || !config.baseline_toolchain.is_empty()
                || config.baseline_lines_percent != 0.0
                || config.configured_floor_percent != MINIMUM_FLOOR_PERCENT
            {
                problems.push(
                    "unmeasured baseline must keep empty provenance and the existing 88% floor"
                        .to_owned(),
                );
            }
        }
        BaselineStatus::Measured => {
            let expected = MINIMUM_FLOOR_PERCENT.max(config.baseline_lines_percent.floor());
            if config.baseline_commit.len() != 40
                || config.baseline_toolchain.trim().is_empty()
                || config.baseline_lines_percent < MINIMUM_FLOOR_PERCENT
                || config.configured_floor_percent != expected
            {
                problems.push(
                    "measured baseline must have exact provenance and floor=max(88,floor(lines))"
                        .to_owned(),
                );
            }
        }
    }
    for artifact in [&config.raw_report_artifact, &config.evidence_artifact] {
        if !is_target_relative(artifact) {
            problems.push(format!("coverage artifact path is unsafe: {artifact}"));
        }
    }

    problems.extend(validate_measurement_plan(&measurement_plan(config), config));

    let workflow = fs::read_to_string(root.join(".github/workflows/ci.yml"))?;
    for required in [
        "tool: cargo-llvm-cov@0.8.7",
        "evidence-run --release 0.64 --gate tool.coverage-ratchet",
        "--ignore-filename-regex '(^|/)crates/(xtask|hydracache-loadgen)/'",
        "target/test-evidence/0.64/coverage-*.json",
        "postgres:16.4-alpine",
        "HYDRACACHE_TEST_POSTGRES_URL",
    ] {
        if !workflow.contains(required) {
            problems.push(format!("coverage CI wiring is missing `{required}`"));
        }
    }
    Ok(problems)
}

pub fn measured_line_percent(document: &Value) -> Option<f64> {
    document
        .get("data")?
        .as_array()?
        .first()?
        .get("totals")?
        .get("lines")?
        .get("percent")?
        .as_f64()
}

pub fn enforce_floor(measured: f64, configured_floor: f64) -> Result<(), String> {
    if measured < configured_floor {
        return Err(format!(
            "measured line coverage {measured:.2}% is below {configured_floor:.2}%"
        ));
    }
    Ok(())
}

pub fn measurement_plan(config: &CoverageRatchet) -> Vec<CoverageStep> {
    vec![
        CoverageStep {
            id: "clean",
            kind: CoverageStepKind::Clean,
            args: strings(&["llvm-cov", "clean", "--workspace"]),
            environment: Vec::new(),
        },
        CoverageStep {
            id: "default-workspace",
            kind: CoverageStepKind::DefaultTests,
            args: strings(&[
                "llvm-cov",
                "--workspace",
                "--all-targets",
                "--locked",
                "--no-report",
            ]),
            environment: Vec::new(),
        },
        CoverageStep {
            id: "raft-sled-log-store",
            kind: CoverageStepKind::AdditiveTests,
            args: strings(&[
                "llvm-cov",
                "-p",
                "hydracache-cluster-raft",
                "--features",
                "sled-log-store",
                "--test",
                "snapshot_corruption",
                "--test",
                "sled_log_store",
                "--test",
                "durable_recovery_corpus",
                "--locked",
                "--no-report",
            ]),
            environment: Vec::new(),
        },
        CoverageStep {
            id: "raft-test-failpoints",
            kind: CoverageStepKind::AdditiveTests,
            args: strings(&[
                "llvm-cov",
                "-p",
                "hydracache-cluster-raft",
                "--features",
                "test-failpoints",
                "--test",
                "failpoints_crash_safety",
                "--test",
                "rejoin_after_compaction",
                "--test",
                "snapshot_resource_faults",
                "--locked",
                "--no-report",
                "--",
                "--test-threads=1",
            ]),
            environment: Vec::new(),
        },
        CoverageStep {
            id: "db-postgres-outbox",
            kind: CoverageStepKind::AdditiveTests,
            args: strings(&[
                "llvm-cov",
                "-p",
                "hydracache-db",
                "--features",
                "sqlx-outbox",
                "--test",
                "outbox_postgres",
                "--locked",
                "--no-report",
                "--",
                "--ignored",
                "--test-threads=1",
            ]),
            environment: Vec::new(),
        },
        CoverageStep {
            id: "server-networked-daemon",
            kind: CoverageStepKind::AdditiveTests,
            args: strings(&[
                "llvm-cov",
                "-p",
                "hydracache-server",
                "--test",
                "grid_host",
                "--locked",
                "--no-report",
                "--",
                "multi_node",
                "--nocapture",
                "--test-threads=1",
            ]),
            environment: vec![("HYDRACACHE_RUN_NETWORKED_DAEMON_E2E", "1")],
        },
        CoverageStep {
            id: "report",
            kind: CoverageStepKind::Report,
            args: vec![
                "llvm-cov".to_owned(),
                "report".to_owned(),
                "--ignore-filename-regex".to_owned(),
                config.ignored_source_regex.clone(),
                "--json".to_owned(),
                "--output-path".to_owned(),
                config.raw_report_artifact.clone(),
            ],
            environment: Vec::new(),
        },
    ]
}

pub fn validate_measurement_plan(plan: &[CoverageStep], config: &CoverageRatchet) -> Vec<String> {
    let mut problems = Vec::new();
    let required_ids = [
        "clean",
        "default-workspace",
        "raft-sled-log-store",
        "raft-test-failpoints",
        "db-postgres-outbox",
        "server-networked-daemon",
        "report",
    ];
    let actual_ids = plan.iter().map(|step| step.id).collect::<Vec<_>>();
    if actual_ids != required_ids {
        problems.push(format!(
            "coverage profile must run required steps in order {required_ids:?}, got {actual_ids:?}"
        ));
    }

    let clean_count = plan
        .iter()
        .filter(|step| step.kind == CoverageStepKind::Clean)
        .count();
    if clean_count != 1 {
        problems.push(format!(
            "coverage profile must contain exactly one clean step, got {clean_count}"
        ));
    }
    let report_count = plan
        .iter()
        .filter(|step| step.kind == CoverageStepKind::Report)
        .count();
    if report_count != 1 || plan.last().map(|step| step.kind) != Some(CoverageStepKind::Report) {
        problems.push("coverage profile must contain exactly one final report step".to_owned());
    }

    for step in plan {
        match step.kind {
            CoverageStepKind::Clean => {
                if !step.environment.is_empty() {
                    problems
                        .push("coverage clean step may not set environment variables".to_owned());
                }
                if step.args != strings(&["llvm-cov", "clean", "--workspace"]) {
                    problems.push(
                        "coverage clean step must clean the workspace exactly once".to_owned(),
                    );
                }
            }
            CoverageStepKind::DefaultTests | CoverageStepKind::AdditiveTests => {
                for required in ["--no-report", "--locked"] {
                    if !has_arg(&step.args, required) {
                        problems.push(format!(
                            "coverage test step {} is missing {required}",
                            step.id
                        ));
                    }
                }
                if has_arg(&step.args, "--no-clean") {
                    problems.push(format!(
                        "coverage test step {} combines incompatible --no-clean and --no-report flags",
                        step.id
                    ));
                }
                if has_arg(&step.args, "clean") || has_arg(&step.args, "report") {
                    problems.push(format!(
                        "coverage test step {} may not clean or report the shared profile",
                        step.id
                    ));
                }
                if step.id == "server-networked-daemon"
                    && step.environment != vec![("HYDRACACHE_RUN_NETWORKED_DAEMON_E2E", "1")]
                {
                    problems.push(
                        "server networked coverage tier must enable its reviewed E2E gate"
                            .to_owned(),
                    );
                }
            }
            CoverageStepKind::Report => {
                if !step.environment.is_empty() {
                    problems
                        .push("coverage report step may not set environment variables".to_owned());
                }
                for required in ["report", "--json", "--output-path"] {
                    if !has_arg(&step.args, required) {
                        problems.push(format!(
                            "coverage report step is missing required argument {required}"
                        ));
                    }
                }
                let exclusion = step
                    .args
                    .windows(2)
                    .find(|window| window[0] == "--ignore-filename-regex")
                    .map(|window| window[1].as_str());
                if exclusion != Some(config.ignored_source_regex.as_str()) {
                    problems.push(
                        "coverage report must use the reviewed development-harness source exclusion"
                            .to_owned(),
                    );
                }
            }
        }
    }
    problems
}

fn execute_measurement(root: &Path, config: &CoverageRatchet) -> Result<(), Box<dyn Error>> {
    let tool_version = command_text(root, "cargo", &["llvm-cov", "--version"])?;
    if !tool_version.contains(&config.tool_version) {
        return Err(format!(
            "expected cargo-llvm-cov {}, got {tool_version}",
            config.tool_version
        )
        .into());
    }
    let plan = measurement_plan(config);
    let plan_problems = validate_measurement_plan(&plan, config);
    if !plan_problems.is_empty() {
        return Err(format!("invalid coverage measurement plan: {plan_problems:?}").into());
    }
    let raw_path = root.join(&config.raw_report_artifact);
    for step in &plan {
        if step.kind == CoverageStepKind::Report {
            if let Some(parent) = raw_path.parent() {
                fs::create_dir_all(parent)?;
            }
        }
        let status = Command::new("cargo")
            .args(&step.args)
            .envs(step.environment.iter().copied())
            .env("CARGO_BUILD_JOBS", "2")
            .current_dir(root)
            .status()?;
        if !status.success() {
            return Err(format!(
                "coverage profile step {} failed with status {status}",
                step.id
            )
            .into());
        }
    }
    let document: Value = serde_json::from_slice(&fs::read(&raw_path)?)?;
    let measured = measured_line_percent(&document)
        .ok_or("coverage JSON is missing data[0].totals.lines.percent")?;
    enforce_floor(measured, config.configured_floor_percent)?;
    let evidence = CoverageEvidence {
        schema_version: 2,
        release: config.release.clone(),
        source_commit: command_text(root, "git", &["rev-parse", "HEAD"])?,
        tool_version,
        rustc_version: command_text(root, "rustc", &["--version"])?,
        configured_floor_percent: config.configured_floor_percent,
        measured_lines_percent: measured,
        ignored_source_regex: config.ignored_source_regex.clone(),
        raw_report_artifact: config.raw_report_artifact.clone(),
        profile_steps: plan
            .into_iter()
            .map(|step| CoverageStepEvidence {
                id: step.id,
                kind: step.kind,
                command: std::iter::once("cargo".to_owned())
                    .chain(step.args)
                    .collect(),
                environment: step.environment,
                status: "passed",
            })
            .collect(),
    };
    let evidence_path = root.join(&config.evidence_artifact);
    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(evidence_path, serde_json::to_vec_pretty(&evidence)?)?;
    println!(
        "coverage-ratchet-check: OK ({measured:.2}% >= {:.2}%)",
        config.configured_floor_percent
    );
    Ok(())
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn has_arg(args: &[String], expected: &str) -> bool {
    args.iter().any(|arg| arg == expected)
}

fn command_text(root: &Path, program: &str, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(format!("{program} {} failed", args.join(" ")).into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn is_target_relative(value: &str) -> bool {
    let path = Path::new(value);
    !path.is_absolute()
        && path.components().next() == Some(Component::Normal(std::ffi::OsStr::new("target")))
        && !path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
}

fn normalize_release(release: &str) -> String {
    if release.matches('.').count() == 1 {
        format!("{release}.0")
    } else {
        release.to_owned()
    }
}

fn parse_args(args: Vec<String>) -> Result<(PathBuf, bool), Box<dyn Error>> {
    let mut root = doc_check::find_repo_root()?;
    let mut execute = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = PathBuf::from(args.next().ok_or("--root requires a path")?),
            "--run" => execute = true,
            "--structural" => {}
            other => return Err(format!("unknown coverage-ratchet-check argument: {other}").into()),
        }
    }
    Ok((root, execute))
}
