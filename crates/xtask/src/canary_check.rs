use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::doc_check;

const REGISTRY_PATH: &str = "docs/testing/canary-registry.json";
const REQUIRED_W_ITEMS: &[&str] = &[
    "W1", "W2", "W3", "W4", "W5", "W7", "W8", "W9", "W10", "W11", "W12", "W13", "W14", "W15",
    "W16", "W17", "W18", "W19", "W20", "W21", "W23",
];

#[derive(Debug, Deserialize)]
struct CanaryRegistry {
    version: u32,
    release: String,
    entries: Vec<CanaryEntry>,
}

#[derive(Debug, Deserialize)]
struct CanaryEntry {
    w_item: String,
    guard: FunctionRef,
    canary: FunctionRef,
    red_evidence: String,
    makes_guard_fail: bool,
}

#[derive(Debug, Deserialize)]
struct FunctionRef {
    file: String,
    function: String,
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
    let mut problems = Vec::new();

    if registry.version != 1 {
        problems.push(format!(
            "{}: unsupported version {}",
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
        registered.insert(entry.w_item.as_str());
        validate_entry(root, entry, &mut problems);
    }

    for item in REQUIRED_W_ITEMS {
        if !registered.contains(item) {
            problems.push(format!("{REGISTRY_PATH}: missing canary entry for {item}"));
        }
    }

    problems.extend(plan_canary_problems(root, &registered)?);
    Ok(problems)
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
    if !entry.makes_guard_fail {
        problems.push(format!(
            "{}: {} canary {} is inert (makes_guard_fail=false)",
            REGISTRY_PATH, entry.w_item, entry.canary.function
        ));
    }
    validate_function_ref(root, &entry.guard, "guard", &entry.w_item, problems);
    validate_function_ref(root, &entry.canary, "canary", &entry.w_item, problems);
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
    registered: &BTreeSet<&str>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let plan_path =
        root.join("docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md");
    let plan = fs::read_to_string(&plan_path)?;
    let mut problems = Vec::new();
    for item in REQUIRED_W_ITEMS {
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

fn load_registry(root: &Path) -> Result<CanaryRegistry, Box<dyn Error>> {
    let path = root.join(REGISTRY_PATH);
    let text = fs::read_to_string(&path).map_err(|error| {
        format!(
            "reading {}: {error}",
            path.strip_prefix(root).unwrap_or(&path).display()
        )
    })?;
    let registry = serde_json::from_str(&text).map_err(|error| {
        format!(
            "parsing {}: {error}",
            path.strip_prefix(root).unwrap_or(&path).display()
        )
    })?;
    Ok(registry)
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
