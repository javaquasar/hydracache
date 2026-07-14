use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::doc_check;

pub const REGISTRY_PATH: &str = "docs/testing/canary-registry.json";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CanaryRegistry {
    pub version: u32,
    pub release: String,
    pub entries: Vec<CanaryEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CanaryEntry {
    pub w_item: String,
    pub guard: FunctionRef,
    pub canary: FunctionRef,
    pub guard_command: CanaryCommand,
    pub canary_command: CanaryCommand,
    pub defect_id: String,
    pub expected_failure: String,
    pub timeout_seconds: u64,
    pub tier: CanaryTier,
    pub artifacts: Vec<String>,
    pub red_evidence: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionRef {
    pub file: String,
    pub function: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CanaryCommand {
    pub program: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    pub cwd: String,
    pub platform: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanaryTier {
    Fast,
    Nightly,
    External,
}

#[derive(Debug, Deserialize)]
struct ReleasesManifest {
    release: Vec<ReleaseRow>,
}

#[derive(Debug, Deserialize)]
struct ReleaseRow {
    version: String,
    #[serde(default)]
    work_items: Vec<String>,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let root = parse_root(args)?;
    let problems = check_canary_registry(&root)?;
    if problems.is_empty() {
        println!("canary-check: OK");
        Ok(())
    } else {
        for problem in &problems {
            eprintln!("canary-check: {problem}");
        }
        Err(format!("canary-check found {} problem(s)", problems.len()).into())
    }
}

pub fn check_canary_registry(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let registry = load_registry(root)?;
    let required = required_work_items(root, &registry.release)?;
    let mut problems = Vec::new();

    if registry.version != 2 {
        problems.push(format!(
            "{}: unsupported version {}; schema v2 is required",
            REGISTRY_PATH, registry.version
        ));
    }
    if registry.release != "0.64.0" {
        problems.push(format!(
            "{}: release must be 0.64.0, got {}",
            REGISTRY_PATH, registry.release
        ));
    }

    let mut registered = BTreeSet::new();
    for entry in &registry.entries {
        if !registered.insert(entry.w_item.clone()) {
            problems.push(format!(
                "{}: duplicate canary entry for {}",
                REGISTRY_PATH, entry.w_item
            ));
        }
        validate_entry(root, entry, &mut problems);
    }

    for item in &required {
        if !registered.contains(item) {
            problems.push(format!("{REGISTRY_PATH}: missing canary entry for {item}"));
        }
    }
    for item in registered.difference(&required) {
        problems.push(format!(
            "{REGISTRY_PATH}: stale canary entry {item} is not in releases.toml"
        ));
    }

    problems.extend(plan_canary_problems(root, &required, &registered)?);
    Ok(problems)
}

pub fn load_registry(root: &Path) -> Result<CanaryRegistry, Box<dyn Error>> {
    let path = root.join(REGISTRY_PATH);
    let text = fs::read_to_string(&path).map_err(|error| {
        format!(
            "reading {}: {error}",
            path.strip_prefix(root).unwrap_or(&path).display()
        )
    })?;
    serde_json::from_str(&text).map_err(|error| {
        format!(
            "parsing {}: {error}",
            path.strip_prefix(root).unwrap_or(&path).display()
        )
        .into()
    })
}

pub fn required_work_items(root: &Path, release: &str) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let text = fs::read_to_string(root.join("docs/plans/releases.toml"))?;
    let releases: ReleasesManifest = toml::from_str(&text)?;
    let normalized = normalize_release(release);
    let row = releases
        .release
        .into_iter()
        .find(|row| normalize_release(&row.version) == normalized)
        .ok_or_else(|| format!("release {release} is not registered in releases.toml"))?;
    if row.work_items.is_empty() {
        return Err(format!("release {release} has no work_items in releases.toml").into());
    }
    Ok(row.work_items.into_iter().collect())
}

fn validate_entry(root: &Path, entry: &CanaryEntry, problems: &mut Vec<String>) {
    if entry.guard.file == entry.canary.file && entry.guard.function == entry.canary.function {
        problems.push(format!(
            "{}: {} guard and canary both reference {}",
            REGISTRY_PATH, entry.w_item, entry.guard.function
        ));
    }
    if entry.red_evidence.trim().is_empty() {
        problems.push(format!(
            "{}: {} is missing red_evidence",
            REGISTRY_PATH, entry.w_item
        ));
    }
    if entry.defect_id.trim().is_empty() || entry.expected_failure.trim().is_empty() {
        problems.push(format!(
            "{}: {} requires defect_id and expected_failure",
            REGISTRY_PATH, entry.w_item
        ));
    }
    if entry.timeout_seconds == 0 || entry.timeout_seconds > 3_600 {
        problems.push(format!(
            "{}: {} timeout_seconds must be in 1..=3600",
            REGISTRY_PATH, entry.w_item
        ));
    }
    if entry.artifacts.is_empty() {
        problems.push(format!(
            "{}: {} must declare a dynamic evidence artifact",
            REGISTRY_PATH, entry.w_item
        ));
    }
    for artifact in &entry.artifacts {
        if !safe_relative_path(artifact) {
            problems.push(format!(
                "{}: {} artifact must be a safe relative path: {artifact}",
                REGISTRY_PATH, entry.w_item
            ));
        }
    }
    validate_command(&entry.guard_command, "guard_command", entry, problems);
    validate_command(&entry.canary_command, "canary_command", entry, problems);
    if entry.guard_command.program == entry.canary_command.program
        && entry.guard_command.args == entry.canary_command.args
        && entry.guard_command.env == entry.canary_command.env
    {
        problems.push(format!(
            "{}: {} guard and canary commands are identical",
            REGISTRY_PATH, entry.w_item
        ));
    }
    validate_function_ref(root, &entry.guard, "guard", &entry.w_item, problems);
    validate_function_ref(root, &entry.canary, "canary", &entry.w_item, problems);
}

fn validate_command(
    command: &CanaryCommand,
    role: &str,
    entry: &CanaryEntry,
    problems: &mut Vec<String>,
) {
    if command.program.trim().is_empty() || command.args.is_empty() {
        problems.push(format!(
            "{}: {} {role} requires program and args",
            REGISTRY_PATH, entry.w_item
        ));
    }
    if !safe_relative_path(&command.cwd) {
        problems.push(format!(
            "{}: {} {role}.cwd must be a safe relative path",
            REGISTRY_PATH, entry.w_item
        ));
    }
    if !matches!(command.platform.as_str(), "any" | "linux" | "windows") {
        problems.push(format!(
            "{}: {} {role}.platform must be any, linux, or windows",
            REGISTRY_PATH, entry.w_item
        ));
    }
}

fn validate_function_ref(
    root: &Path,
    reference: &FunctionRef,
    role: &str,
    w_item: &str,
    problems: &mut Vec<String>,
) {
    let path = root.join(&reference.file);
    match fs::read_to_string(&path) {
        Ok(text) => {
            if !function_exists(&text, &reference.function) {
                problems.push(format!(
                    "{}: {w_item} {role} function `{}` not found in {}",
                    REGISTRY_PATH, reference.function, reference.file
                ));
            }
        }
        Err(error) => problems.push(format!(
            "{}: {w_item} {role} file {} cannot be read: {error}",
            REGISTRY_PATH, reference.file
        )),
    }
}

fn function_exists(text: &str, function: &str) -> bool {
    let patterns = [
        format!("fn {function}("),
        format!("fn {function}<"),
        format!("async fn {function}("),
        format!("async fn {function}<"),
    ];
    patterns.iter().any(|pattern| text.contains(pattern))
}

fn plan_canary_problems(
    root: &Path,
    required: &BTreeSet<String>,
    registered: &BTreeSet<String>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let plan = fs::read_to_string(
        root.join("docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md"),
    )?;
    let mut problems = Vec::new();
    for item in required {
        if !plan.contains(&format!("## {item}.")) && !plan.contains(&format!("### {item}.")) {
            problems.push(format!(
                "{}: {item} is registered but has no release-plan section",
                REGISTRY_PATH
            ));
        }
        if !registered.contains(item) {
            problems.push(format!(
                "{}: {item} has no registered canary for the release-plan section",
                REGISTRY_PATH
            ));
        }
    }
    Ok(problems)
}

fn safe_relative_path(value: &str) -> bool {
    let path = Path::new(value);
    !value.trim().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| !matches!(component, Component::ParentDir | Component::Prefix(_)))
}

fn normalize_release(value: &str) -> &str {
    value.strip_suffix(".0").unwrap_or(value)
}

fn parse_root(args: Vec<String>) -> Result<PathBuf, Box<dyn Error>> {
    let mut root: Option<PathBuf> = None;
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--root" => {
                root = Some(PathBuf::from(
                    it.next().ok_or("--root requires a path argument")?,
                ))
            }
            other => return Err(format!("unknown canary-check argument: {other}").into()),
        }
    }
    match root {
        Some(root) => Ok(root),
        None => doc_check::find_repo_root(),
    }
}
