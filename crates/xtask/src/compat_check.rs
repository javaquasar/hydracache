use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use sha2::{Digest, Sha256};

const MANIFEST: &str = "docs/testing/compat/v0.63.0.json";
const REQUIRED_KINDS: &[&str] = &["raft-wire-message", "raft-conf-state", "raft-snapshot"];

#[derive(Debug, Deserialize)]
struct CompatManifest {
    schema_version: u32,
    producer_release: String,
    producer_tag: String,
    producer_commit: String,
    artifacts: Vec<CompatArtifact>,
    api_baseline: ApiBaseline,
}

#[derive(Debug, Deserialize)]
struct CompatArtifact {
    kind: String,
    path: String,
    format_version: u32,
    sha256: String,
    expected: String,
    write_back_supported: bool,
}

#[derive(Debug, Deserialize)]
struct ApiBaseline {
    tool: String,
    tool_version: String,
    baseline_release: String,
    packages: Vec<String>,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let (root, mode) = parse_args(args)?;
    let problems = match mode {
        Mode::PreflightOnly => preflight_check(&root)?,
        Mode::ManifestOnly => manifest_check(&root)?,
        Mode::Full => {
            let mut problems = manifest_check(&root)?;
            problems.extend(preflight_check(&root)?);
            problems
        }
    };
    if problems.is_empty() {
        println!("compat-check: OK ({mode:?})");
        Ok(())
    } else {
        for problem in &problems {
            eprintln!("compat-check: {problem}");
        }
        Err(format!("compat-check found {} problem(s)", problems.len()).into())
    }
}

#[derive(Clone, Copy, Debug)]
enum Mode {
    Full,
    PreflightOnly,
    ManifestOnly,
}

pub fn manifest_check(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let manifest: CompatManifest = serde_json::from_str(&fs::read_to_string(root.join(MANIFEST))?)?;
    let mut problems = Vec::new();
    if manifest.schema_version != 1 {
        problems.push(format!(
            "unsupported manifest schema {}",
            manifest.schema_version
        ));
    }
    if manifest.producer_release != "0.63.0" || manifest.producer_tag != "v0.63.0" {
        problems.push("previous fixture provenance must be the shipped v0.63.0 release".to_owned());
    }
    let tag_commit = git_output(root, ["rev-list", "-n", "1", "v0.63.0"])?;
    if tag_commit != manifest.producer_commit {
        problems.push(format!(
            "producer commit mismatch: manifest={}, tag={tag_commit}",
            manifest.producer_commit
        ));
    }

    let mut kinds = BTreeSet::new();
    for artifact in &manifest.artifacts {
        if !kinds.insert(artifact.kind.as_str()) {
            problems.push(format!(
                "duplicate compatibility artifact kind {}",
                artifact.kind
            ));
        }
        if artifact.format_version == 0 || artifact.expected.trim().is_empty() {
            problems.push(format!(
                "artifact {} has an incomplete semantic contract",
                artifact.kind
            ));
        }
        if artifact.kind == "raft-wire-message" && artifact.write_back_supported {
            problems.push("previous wire fixture must be read-only".to_owned());
        }
        let path = root.join(&artifact.path);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                problems.push(format!("cannot read {}: {error}", path.display()));
                continue;
            }
        };
        let actual = Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if actual != artifact.sha256 {
            problems.push(format!(
                "fixture hash mismatch for {}: expected {}, got {actual}",
                artifact.path, artifact.sha256
            ));
        }
    }
    for required in REQUIRED_KINDS {
        if !kinds.contains(required) {
            problems.push(format!(
                "missing required compatibility artifact {required}"
            ));
        }
    }
    if manifest.api_baseline.tool != "cargo-semver-checks"
        || manifest.api_baseline.tool_version != "0.48.0"
        || manifest.api_baseline.baseline_release != "0.63.0"
        || manifest.api_baseline.packages.is_empty()
    {
        problems.push("public API baseline is incomplete or not pinned".to_owned());
    }
    Ok(problems)
}

pub fn preflight_check(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let ancestor = Command::new("git")
        .args(["merge-base", "--is-ancestor", "v0.63.0", "HEAD"])
        .current_dir(root)
        .status()?
        .success();
    let root_manifest = fs::read_to_string(root.join("Cargo.toml"))?;
    let mut problems = validate_internal_versions(&root_manifest)?;
    if !ancestor {
        problems.insert(0, "v0.63.0 is not an ancestor of HEAD".to_owned());
    }
    Ok(problems)
}

pub fn validate_internal_versions(root_manifest: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let manifest: toml::Value = toml::from_str(root_manifest)?;
    let mut problems = Vec::new();
    let workspace_version = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("version"))
        .and_then(toml::Value::as_str)
        .unwrap_or("<missing>");
    if !allowed_workspace_version(workspace_version) {
        problems.push(format!(
            "workspace version {workspace_version} is not 0.63.0 or 0.64.0-dev"
        ));
    }
    if let Some(dependencies) = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(toml::Value::as_table)
    {
        for (name, dependency) in dependencies {
            let Some(table) = dependency.as_table() else {
                continue;
            };
            if !name.starts_with("hydracache") || !table.contains_key("path") {
                continue;
            }
            if let Some(version) = table.get("version").and_then(toml::Value::as_str) {
                if !allowed_workspace_version(version) {
                    problems.push(format!("stale internal dependency {name}={version}"));
                }
            }
        }
    }
    Ok(problems)
}

fn allowed_workspace_version(version: &str) -> bool {
    version == "0.63.0" || version == "0.64.0-dev" || version.starts_with("=0.64.0-dev")
}

fn git_output<const N: usize>(root: &Path, args: [&str; N]) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git").args(args).current_dir(root).output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr)
            .trim()
            .to_owned()
            .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn parse_args(args: Vec<String>) -> Result<(PathBuf, Mode), Box<dyn Error>> {
    let mut root = crate::doc_check::find_repo_root()?;
    let mut mode = Mode::Full;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = PathBuf::from(args.next().ok_or("--root requires a path")?),
            "--preflight-only" => mode = Mode::PreflightOnly,
            "--manifest-only" => mode = Mode::ManifestOnly,
            other => return Err(format!("unknown compat-check argument: {other}").into()),
        }
    }
    Ok((root, mode))
}

#[cfg(test)]
mod tests {
    use super::validate_internal_versions;

    #[test]
    fn compat_preflight_rejects_a_branch_without_v063_ancestry_or_with_stale_internal_versions() {
        let stale = r#"
[workspace]
[workspace.package]
version = "0.62.0"
[workspace.dependencies]
hydracache = { path = "crates/hydracache", version = "0.62.0" }
sqlparser = "0.62.0"
"#;
        let problems = validate_internal_versions(stale).unwrap();
        assert!(problems
            .iter()
            .any(|problem| problem.contains("workspace version")));
        assert!(problems
            .iter()
            .any(|problem| problem.contains("stale internal dependency hydracache")));
        assert!(!problems.iter().any(|problem| problem.contains("sqlparser")));
    }
}
