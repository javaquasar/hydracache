use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::doc_check;
use crate::evidence_run::{EvidenceOutcome, EvidenceReceipt};
use crate::gated_tests::CommandSpec;

pub const REGISTRY_PATH: &str = "docs/testing/fast-suite-registry.toml";
pub const NEXTTEST_CONFIG_PATH: &str = ".config/nextest.toml";
pub const PR_BUDGET_SECONDS: u64 = 1_500;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FastSuiteRegistry {
    pub schema_version: u32,
    pub release: String,
    pub nextest_version: String,
    pub aggregate_budget_seconds: u64,
    #[serde(default)]
    pub suite: Vec<FastSuiteEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FastSuiteEntry {
    pub id: String,
    pub work_items: Vec<String>,
    pub timeout_seconds: u64,
    pub budget_seconds: u64,
    pub deterministic: bool,
    pub artifacts: Vec<String>,
    pub logical_digest_artifact: String,
    pub baseline: Baseline,
    pub command: CommandSpec,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Baseline {
    pub status: BaselineStatus,
    pub commit: String,
    pub toolchain: String,
    pub linux_ci_median_seconds: u64,
    pub noise_allowance_seconds: u64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BaselineStatus {
    Unmeasured,
    Measured,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let (root, release, receipts_dir) = parse_args(args)?;
    let registry = load_registry(&root)?;
    let problems = validate_registry(&root, &registry, &release, receipts_dir.as_deref())?;
    if problems.is_empty() {
        let measured = registry
            .suite
            .iter()
            .filter(|suite| suite.baseline.status == BaselineStatus::Measured)
            .count();
        println!(
            "fast-suite-check: OK ({} suites, {measured} measured baselines, aggregate budget {}s)",
            registry.suite.len(),
            registry.aggregate_budget_seconds
        );
        Ok(())
    } else {
        for problem in &problems {
            eprintln!("fast-suite-check: {problem}");
        }
        Err(format!("fast-suite-check found {} problem(s)", problems.len()).into())
    }
}

pub fn load_registry(root: &Path) -> Result<FastSuiteRegistry, Box<dyn Error>> {
    let path = root.join(REGISTRY_PATH);
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("reading {}: {error}", path.display()))?;
    toml::from_str(&text).map_err(|error| format!("parsing {REGISTRY_PATH}: {error}").into())
}

pub fn validate_registry(
    root: &Path,
    registry: &FastSuiteRegistry,
    release: &str,
    receipts_dir: Option<&Path>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut problems = Vec::new();
    if registry.schema_version != 1 {
        problems.push("schema_version must be 1".to_owned());
    }
    if !registry_covers_release(&registry.release, release) {
        problems.push("release mismatch".to_owned());
    }
    if registry.nextest_version != "0.9.137" {
        problems.push("cargo-nextest must remain pinned to reviewed version 0.9.137".to_owned());
    }
    if registry.aggregate_budget_seconds != PR_BUDGET_SECONDS {
        problems.push(format!(
            "aggregate_budget_seconds must be the reviewed {}s PR ceiling",
            PR_BUDGET_SECONDS
        ));
    }
    let budget_sum: u64 = registry
        .suite
        .iter()
        .map(|suite| suite.budget_seconds)
        .sum();
    if budget_sum > registry.aggregate_budget_seconds {
        problems.push(format!(
            "suite budgets total {budget_sum}s, above {}s",
            registry.aggregate_budget_seconds
        ));
    }
    let config = fs::read_to_string(root.join(NEXTTEST_CONFIG_PATH))?;
    let config: toml::Value = toml::from_str(&config)
        .map_err(|error| format!("parsing {NEXTTEST_CONFIG_PATH}: {error}"))?;
    let ci_profile = config.get("profile").and_then(|profile| profile.get("ci"));
    if ci_profile
        .and_then(|profile| profile.get("slow-timeout"))
        .is_none()
    {
        problems.push(format!(
            "{NEXTTEST_CONFIG_PATH} profile.ci is missing slow-timeout"
        ));
    }
    if config
        .get("test-groups")
        .and_then(|groups| groups.get("trybuild"))
        .and_then(|group| group.get("max-threads"))
        .and_then(toml::Value::as_integer)
        != Some(1)
    {
        problems.push(format!(
            "{NEXTTEST_CONFIG_PATH} must serialize the trybuild group with max-threads = 1"
        ));
    }
    let trybuild_override = ci_profile
        .and_then(|profile| profile.get("overrides"))
        .and_then(toml::Value::as_array)
        .and_then(|overrides| {
            overrides.iter().find(|entry| {
                entry.get("test-group").and_then(toml::Value::as_str) == Some("trybuild")
            })
        });
    let filter = trybuild_override
        .and_then(|entry| entry.get("filter"))
        .and_then(toml::Value::as_str)
        .unwrap_or_default();
    if !filter.contains("cacheable_macro_compile_tests")
        || !filter.contains("proc_macro_compile_tests")
    {
        problems.push(format!(
            "{NEXTTEST_CONFIG_PATH} trybuild override must select both compile-test harnesses"
        ));
    }
    let trybuild_timeout = trybuild_override.and_then(|entry| entry.get("slow-timeout"));
    if trybuild_timeout
        .and_then(|timeout| timeout.get("period"))
        .and_then(toml::Value::as_str)
        != Some("120s")
        || trybuild_timeout
            .and_then(|timeout| timeout.get("terminate-after"))
            .and_then(toml::Value::as_integer)
            != Some(3)
    {
        problems.push(format!(
            "{NEXTTEST_CONFIG_PATH} trybuild override must use bounded slow-timeout 120s x 3"
        ));
    }

    let mut ids = BTreeSet::new();
    let mut previous = None;
    for suite in &registry.suite {
        if !ids.insert(suite.id.as_str()) {
            problems.push(format!("duplicate suite id {}", suite.id));
        }
        if previous.is_some_and(|previous: &str| previous >= suite.id.as_str()) {
            problems.push("suite ids must use deterministic ascending order".to_owned());
        }
        previous = Some(&suite.id);
        if suite.id.trim().is_empty()
            || suite.timeout_seconds == 0
            || suite.budget_seconds == 0
            || suite.budget_seconds > suite.timeout_seconds
            || suite.work_items.is_empty()
            || suite.command.program.trim().is_empty()
        {
            problems.push(format!(
                "suite {} has an incomplete execution contract",
                suite.id
            ));
        }
        if suite.deterministic && suite.logical_digest_artifact.is_empty() {
            problems.push(format!(
                "deterministic suite {} must name a logical digest artifact",
                suite.id
            ));
        }
        for artifact in suite
            .artifacts
            .iter()
            .chain(std::iter::once(&suite.logical_digest_artifact))
            .filter(|artifact| !artifact.is_empty())
        {
            if !is_target_relative(artifact) {
                problems.push(format!("suite {} has unsafe artifact {artifact}", suite.id));
            }
        }
        match suite.baseline.status {
            BaselineStatus::Unmeasured => {
                if !suite.baseline.commit.is_empty()
                    || !suite.baseline.toolchain.is_empty()
                    || suite.baseline.linux_ci_median_seconds != 0
                    || suite.baseline.noise_allowance_seconds != 0
                {
                    problems.push(format!(
                        "suite {} labels its baseline unmeasured but contains invented measurements",
                        suite.id
                    ));
                }
            }
            BaselineStatus::Measured => {
                if suite.baseline.commit.len() != 40
                    || suite.baseline.toolchain.is_empty()
                    || suite.baseline.linux_ci_median_seconds == 0
                    || suite.budget_seconds
                        < suite.baseline.linux_ci_median_seconds
                            + suite.baseline.noise_allowance_seconds
                {
                    problems.push(format!(
                        "suite {} has an invalid measured baseline",
                        suite.id
                    ));
                }
            }
        }
    }

    if let Some(receipts_dir) = receipts_dir {
        let path = root.join(receipts_dir);
        if !is_target_path(receipts_dir) || !path.is_dir() {
            problems.push("receipts directory must exist inside target".to_owned());
        } else {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
                    continue;
                }
                let receipt: EvidenceReceipt =
                    match serde_json::from_slice(&fs::read(entry.path())?) {
                        Ok(receipt) => receipt,
                        Err(error) => {
                            problems.push(format!(
                                "invalid receipt {}: {error}",
                                entry.path().display()
                            ));
                            continue;
                        }
                    };
                if let Some(suite) = registry
                    .suite
                    .iter()
                    .find(|suite| suite.id == receipt.gate_id)
                {
                    if receipt.outcome == EvidenceOutcome::Pass
                        && receipt.duration_ms > suite.budget_seconds.saturating_mul(1_000)
                    {
                        problems.push(format!(
                            "suite {} took {}ms, above its {}s budget",
                            suite.id, receipt.duration_ms, suite.budget_seconds
                        ));
                    }
                }
            }
        }
    }
    Ok(problems)
}

fn is_target_relative(value: &str) -> bool {
    is_target_path(Path::new(value))
}

fn is_target_path(value: &Path) -> bool {
    !value.is_absolute()
        && value.components().next() == Some(Component::Normal(std::ffi::OsStr::new("target")))
        && !value.components().any(|component| {
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

fn registry_covers_release(registry_release: &str, requested: &str) -> bool {
    let parse = |release: &str| -> Option<(u64, u64, u64)> {
        let normalized = normalize_release(release);
        let mut parts = normalized.split('.').map(str::parse::<u64>);
        let version = (
            parts.next()?.ok()?,
            parts.next()?.ok()?,
            parts.next()?.ok()?,
        );
        parts.next().is_none().then_some(version)
    };
    matches!(
        (parse(registry_release), parse(requested)),
        (Some(registry), Some(candidate)) if registry.0 == candidate.0 && registry <= candidate
    )
}

fn parse_args(args: Vec<String>) -> Result<(PathBuf, String, Option<PathBuf>), Box<dyn Error>> {
    let mut root = doc_check::find_repo_root()?;
    let mut release = None;
    let mut receipts_dir = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = PathBuf::from(args.next().ok_or("--root requires a path")?),
            "--release" => release = Some(args.next().ok_or("--release requires a value")?),
            "--receipts-dir" => {
                receipts_dir = Some(PathBuf::from(
                    args.next().ok_or("--receipts-dir requires a path")?,
                ))
            }
            other => return Err(format!("unknown fast-suite-check argument: {other}").into()),
        }
    }
    Ok((
        root,
        release.ok_or("fast-suite-check requires --release")?,
        receipts_dir,
    ))
}
