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
//! - a shipped `0.43.0` entry must explicitly confirm that the networked control
//!   plane is wired, so a modeled-vs-networked gap cannot be marked shipped by
//!   accident.
//! - every in-prose `V0_*.md` plan reference under `docs/plans/` resolves to a
//!   real plan file.
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
    #[serde(default)]
    networked_control_plane: Option<bool>,
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

        if r.status == "shipped" && r.networked_control_plane == Some(false) {
            problems.push(format!(
                "{}: shipped release cannot set networked_control_plane = false",
                r.file
            ));
        }

        if r.version == "0.43.0" && r.status == "shipped" && r.networked_control_plane != Some(true)
        {
            problems.push(format!(
                "{}: shipped 0.43.0 must set networked_control_plane = true",
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

    problems.extend(check_in_prose_plan_links(root)?);

    Ok(problems)
}

fn check_in_prose_plan_links(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let plan_dir = root.join("docs/plans");
    if !plan_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut problems = Vec::new();
    for entry in
        fs::read_dir(&plan_dir).map_err(|err| format!("reading {}: {err}", plan_dir.display()))?
    {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("reading {}: {err}", path.display()))?;
        let source = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string();
        let mut seen = HashSet::new();
        for link in extract_plan_links(&text) {
            if !seen.insert(link.clone()) {
                continue;
            }
            if !plan_dir.join(&link).is_file() {
                problems.push(format!("{source}: references missing plan '{link}'"));
            }
        }
    }

    Ok(problems)
}

fn extract_plan_links(text: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut search_from = 0;

    while let Some(offset) = text[search_from..].find("V0_") {
        let start = search_from + offset;
        let rest = &text[start..];
        let Some(md_offset) = rest.find(".md") else {
            break;
        };
        let end = start + md_offset + ".md".len();
        let candidate = &text[start..end];
        if is_plan_filename(candidate) {
            links.push(candidate.to_owned());
            search_from = end;
        } else {
            search_from = start + "V0_".len();
        }
    }

    links
}

fn is_plan_filename(candidate: &str) -> bool {
    candidate.ends_with(".md")
        && candidate
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
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
        Err(format!("doc-check found {} problem(s)", problems.len()).into())
    }
}
