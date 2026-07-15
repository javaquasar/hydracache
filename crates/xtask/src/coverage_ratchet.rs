use std::error::Error;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::doc_check;

pub const CONFIG_PATH: &str = "docs/testing/coverage-ratchet.toml";
pub const MINIMUM_FLOOR_PERCENT: f64 = 88.0;
const REVIEWED_IGNORED_SOURCE_REGEX: &str = "(^|/)crates/xtask/";

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

    let workflow = fs::read_to_string(root.join(".github/workflows/ci.yml"))?;
    for required in [
        "tool: cargo-llvm-cov@0.8.7",
        "evidence-run --release 0.64 --gate tool.coverage-ratchet",
        "--ignore-filename-regex '(^|/)crates/xtask/'",
        "target/test-evidence/0.64/coverage-*.json",
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

pub fn measurement_args(config: &CoverageRatchet) -> Vec<String> {
    vec![
        "llvm-cov".to_owned(),
        "--workspace".to_owned(),
        "--all-targets".to_owned(),
        "--locked".to_owned(),
        "--ignore-filename-regex".to_owned(),
        config.ignored_source_regex.clone(),
        "--json".to_owned(),
        "--output-path".to_owned(),
        config.raw_report_artifact.clone(),
    ]
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
    let raw_path = root.join(&config.raw_report_artifact);
    if let Some(parent) = raw_path.parent() {
        fs::create_dir_all(parent)?;
    }
    // The ratchet parses the report itself so a failed floor stays actionable.
    let status = Command::new("cargo")
        .args(measurement_args(config))
        .env("CARGO_BUILD_JOBS", "2")
        .current_dir(root)
        .status()?;
    if !status.success() {
        return Err(format!("cargo llvm-cov failed with status {status}").into());
    }
    let document: Value = serde_json::from_slice(&fs::read(&raw_path)?)?;
    let measured = measured_line_percent(&document)
        .ok_or("coverage JSON is missing data[0].totals.lines.percent")?;
    enforce_floor(measured, config.configured_floor_percent)?;
    let evidence = CoverageEvidence {
        schema_version: 1,
        release: config.release.clone(),
        source_commit: command_text(root, "git", &["rev-parse", "HEAD"])?,
        tool_version,
        rustc_version: command_text(root, "rustc", &["--version"])?,
        configured_floor_percent: config.configured_floor_percent,
        measured_lines_percent: measured,
        ignored_source_regex: config.ignored_source_regex.clone(),
        raw_report_artifact: config.raw_report_artifact.clone(),
    };
    let evidence_path = root.join(&config.evidence_artifact);
    fs::write(evidence_path, serde_json::to_vec_pretty(&evidence)?)?;
    println!(
        "coverage-ratchet-check: OK ({measured:.2}% >= {:.2}%)",
        config.configured_floor_percent
    );
    Ok(())
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
