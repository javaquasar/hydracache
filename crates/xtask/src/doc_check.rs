//! Documentation-consistency checks for the release manifest.
//!
//! Validates `docs/plans/releases.toml` (the authoritative release registry, see
//! `docs/RULES.md` R-11):
//!
//! - every `file` referenced by an entry exists on disk;
//! - no two non-draft / non-superseded entries share a `version`;
//! - every `depends_on` value resolves to a `version` present in the manifest;
//! - `status` is one of the allowed values;
//! - `version = "TBD"` is only allowed for `draft` / `superseded` entries.
//!
//! This turns the "release sequencing is recorded, not implied" rule into an
//! executable gate so doc drift (e.g. two plans claiming the same version, or a
//! plan referencing a sibling that no longer exists) fails CI instead of silently
//! rotting.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

const ALLOWED_STATUS: [&str; 5] = ["shipped", "in-progress", "planned", "draft", "superseded"];

#[derive(serde::Deserialize)]
struct Manifest {
    #[serde(default)]
    release: Vec<Release>,
}

#[derive(serde::Deserialize)]
struct Release {
    version: String,
    file: String,
    status: String,
    #[serde(default)]
    #[allow(dead_code)]
    theme: String,
    #[serde(default)]
    depends_on: Vec<String>,
}

/// Locate the repository root by ascending from the cargo manifest dir and the
/// current directory until `docs/plans/releases.toml` is found.
pub fn find_repo_root() -> Result<PathBuf, Box<dyn Error>> {
    let mut starts: Vec<PathBuf> = Vec::new();
    if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        starts.push(PathBuf::from(dir));
    }
    if let Ok(dir) = std::env::current_dir() {
        starts.push(dir);
    }
    for start in starts {
        let mut dir: &Path = start.as_path();
        loop {
            if dir.join("docs/plans/releases.toml").is_file() {
                return Ok(dir.to_path_buf());
            }
            match dir.parent() {
                Some(parent) => dir = parent,
                None => break,
            }
        }
    }
    Err("could not locate repo root (docs/plans/releases.toml not found)".into())
}

/// Validate the manifest under `root`. Returns the list of problems found (empty =
/// consistent). `Err` is reserved for IO / parse failures.
pub fn check(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let manifest_path = root.join("docs/plans/releases.toml");
    let text = fs::read_to_string(&manifest_path)
        .map_err(|err| format!("reading {}: {err}", manifest_path.display()))?;
    let manifest: Manifest = toml::from_str(&text)
        .map_err(|err| format!("parsing {}: {err}", manifest_path.display()))?;

    let mut problems = Vec::new();
    let known_versions: HashSet<&str> = manifest
        .release
        .iter()
        .map(|r| r.version.as_str())
        .collect();
    let mut active_versions: HashMap<&str, ()> = HashMap::new();

    for r in &manifest.release {
        if !ALLOWED_STATUS.contains(&r.status.as_str()) {
            problems.push(format!(
                "{}: invalid status '{}' (allowed: {})",
                r.file,
                r.status,
                ALLOWED_STATUS.join(", ")
            ));
        }
        let is_draftish = r.status == "draft" || r.status == "superseded";

        if r.version == "TBD" && !is_draftish {
            problems.push(format!(
                "{}: version 'TBD' is only allowed for draft/superseded entries",
                r.file
            ));
        }

        if !root.join(&r.file).is_file() {
            problems.push(format!("{}: file does not exist", r.file));
        }

        if r.version != "TBD"
            && !is_draftish
            && active_versions.insert(r.version.as_str(), ()).is_some()
        {
            problems.push(format!(
                "duplicate version '{}' among active (non-draft) plans",
                r.version
            ));
        }

        for dep in &r.depends_on {
            if !known_versions.contains(dep.as_str()) {
                problems.push(format!(
                    "{}: depends_on '{}' does not match any version in the manifest",
                    r.file, dep
                ));
            }
        }
    }

    Ok(problems)
}

/// CLI entry point: `cargo xtask doc-check [--root <path>]`.
pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mut root: Option<PathBuf> = None;
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--root" => {
                root = Some(PathBuf::from(
                    it.next().ok_or("--root requires a path argument")?,
                ))
            }
            other => return Err(format!("unknown doc-check argument: {other}").into()),
        }
    }

    let root = match root {
        Some(root) => root,
        None => find_repo_root()?,
    };

    let problems = check(&root)?;
    if problems.is_empty() {
        println!("doc-check: OK (releases.toml is consistent)");
        Ok(())
    } else {
        for problem in &problems {
            eprintln!("doc-check: {problem}");
        }
        Err(format!("doc-check fo