//! `cargo xtask set-version X.Y.Z`: bump every place that pins the release
//! version in one step.
//!
//! Updated together:
//!
//! - `Cargo.toml` `[workspace.package] version`;
//! - every `[workspace.dependencies]` entry that points at an in-repo crate
//!   (`path = "crates/..."`) and carries the old version;
//! - `deploy/helm/hydracache/Chart.yaml` `version` and `appVersion` (the Helm
//!   image tag defaults to `appVersion`);
//! - the `hydracache-server` image in `deploy/k8s/statefulset.yaml`.
//!
//! The edit is textual so comments and layout survive. After writing, the
//! deploy-image part of `doc-check` runs against the result; any leftover drift
//! fails the command instead of being reported as success. `Cargo.lock`,
//! release notes and `docs/plans/releases.toml` stay manual release steps.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use crate::doc_check;

const SERVER_IMAGE_PREFIX: &str = "ghcr.io/javaquasar/hydracache-server:";

/// One file rewritten by [`set_version`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionEdit {
    /// Path relative to the repository root.
    pub file: String,
    /// Number of lines changed in that file.
    pub lines: usize,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mut root: Option<PathBuf> = None;
    let mut version: Option<String> = None;
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--root" => {
                root = Some(PathBuf::from(
                    it.next().ok_or("--root requires a path argument")?,
                ))
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown set-version argument: {other}").into())
            }
            other if version.is_none() => version = Some(other.to_owned()),
            other => return Err(format!("unexpected set-version argument: {other}").into()),
        }
    }
    let version = version.ok_or("usage: cargo xtask set-version X.Y.Z [--root PATH]")?;
    let root = match root {
        Some(root) => root,
        None => doc_check::find_repo_root()?,
    };

    let (old, edits) = set_version(&root, &version)?;
    for edit in &edits {
        println!("set-version: {} ({} line(s))", edit.file, edit.lines);
    }
    println!("set-version: {old} -> {version}");
    println!("next: cargo update --workspace --offline   # refresh Cargo.lock");
    Ok(())
}

/// Rewrite every release-version pin from the current workspace version to
/// `new_version`. Returns the previous version and the files changed.
pub fn set_version(
    root: &Path,
    new_version: &str,
) -> Result<(String, Vec<VersionEdit>), Box<dyn Error>> {
    if !is_release_version(new_version) {
        return Err(format!(
            "invalid version {new_version:?}: expected MAJOR.MINOR.PATCH with an optional -pre.release suffix"
        )
        .into());
    }

    let cargo_path = root.join("Cargo.toml");
    let cargo_text = fs::read_to_string(&cargo_path)
        .map_err(|err| format!("reading {}: {err}", cargo_path.display()))?;
    let old_version =
        workspace_version(&cargo_text).ok_or("Cargo.toml: no `version` in [workspace.package]")?;

    let (cargo_new, cargo_lines) = rewrite_cargo_toml(&cargo_text, &old_version, new_version)?;

    let chart_rel = "deploy/helm/hydracache/Chart.yaml";
    let chart_text = read(root, chart_rel)?;
    let (chart_new, chart_lines) = rewrite_chart(&chart_text, new_version)?;

    let k8s_rel = "deploy/k8s/statefulset.yaml";
    let k8s_text = read(root, k8s_rel)?;
    let (k8s_new, k8s_lines) = rewrite_server_image(&k8s_text, new_version)?;

    // Write only after every file has been rewritten in memory, so a malformed
    // file cannot leave a half-bumped tree behind.
    fs::write(&cargo_path, cargo_new)?;
    fs::write(root.join(chart_rel), chart_new)?;
    fs::write(root.join(k8s_rel), k8s_new)?;

    let leftover = doc_check::check_deploy_image_versions(root)?;
    if !leftover.is_empty() {
        return Err(format!(
            "set-version wrote the files but deploy checks still fail:\n- {}",
            leftover.join("\n- ")
        )
        .into());
    }

    Ok((
        old_version,
        vec![
            VersionEdit {
                file: "Cargo.toml".to_owned(),
                lines: cargo_lines,
            },
            VersionEdit {
                file: chart_rel.to_owned(),
                lines: chart_lines,
            },
            VersionEdit {
                file: k8s_rel.to_owned(),
                lines: k8s_lines,
            },
        ],
    ))
}

fn read(root: &Path, relative: &str) -> Result<String, Box<dyn Error>> {
    fs::read_to_string(root.join(relative))
        .map_err(|err| format!("reading {relative}: {err}").into())
}

fn is_release_version(version: &str) -> bool {
    let (core, pre) = match version.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (version, None),
    };
    let numeric = core.split('.').collect::<Vec<_>>();
    numeric.len() == 3
        && numeric
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
        && pre.is_none_or(|pre| {
            !pre.is_empty()
                && pre.split('.').all(|part| {
                    !part.is_empty() && part.chars().all(|ch| ch.is_ascii_alphanumeric())
                })
        })
}

fn workspace_version(cargo_toml: &str) -> Option<String> {
    let value: toml::Value = toml::from_str(cargo_toml).ok()?;
    value["workspace"]["package"]["version"]
        .as_str()
        .map(str::to_owned)
}

/// Current TOML table header, e.g. `workspace.package`, for a trimmed line.
fn table_header(line: &str) -> Option<&str> {
    let inner = line.strip_prefix('[')?.strip_suffix(']')?;
    (!inner.starts_with('[')).then(|| inner.trim())
}

fn rewrite_cargo_toml(text: &str, old: &str, new: &str) -> Result<(String, usize), Box<dyn Error>> {
    let old_field = format!("version = \"{old}\"");
    let new_field = format!("version = \"{new}\"");
    let mut table = String::new();
    let mut package_lines = 0;
    let mut dependency_lines = 0;
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if let Some(header) = table_header(trimmed) {
            table = header.to_owned();
        }
        let rewritten = if table == "workspace.package" && trimmed == old_field {
            package_lines += 1;
            line.replacen(&old_field, &new_field, 1)
        } else if table == "workspace.dependencies"
            && trimmed.contains("path = \"crates/")
            && trimmed.contains(&old_field)
        {
            dependency_lines += 1;
            line.replacen(&old_field, &new_field, 1)
        } else {
            line.to_owned()
        };
        out.push_str(&rewritten);
    }
    if package_lines != 1 {
        return Err(format!(
            "Cargo.toml: expected exactly one `{old_field}` line in [workspace.package], found {package_lines}"
        )
        .into());
    }
    if dependency_lines == 0 {
        return Err(format!(
            "Cargo.toml: no in-repo [workspace.dependencies] entry carries `{old_field}`"
        )
        .into());
    }
    Ok((out, package_lines + dependency_lines))
}

fn rewrite_chart(text: &str, new: &str) -> Result<(String, usize), Box<dyn Error>> {
    let mut version = 0;
    let mut app_version = 0;
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let ending = &line[line.trim_end_matches(['\r', '\n']).len()..];
        if line.starts_with("version:") {
            version += 1;
            out.push_str(&format!("version: {new}{ending}"));
        } else if line.starts_with("appVersion:") {
            app_version += 1;
            out.push_str(&format!("appVersion: \"{new}\"{ending}"));
        } else {
            out.push_str(line);
        }
    }
    if version != 1 || app_version != 1 {
        return Err(format!(
            "deploy/helm/hydracache/Chart.yaml: expected one top-level `version:` and one `appVersion:`, found {version} and {app_version}"
        )
        .into());
    }
    Ok((out, version + app_version))
}

fn rewrite_server_image(text: &str, new: &str) -> Result<(String, usize), Box<dyn Error>> {
    let mut count = 0;
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        match line.find(SERVER_IMAGE_PREFIX) {
            Some(start) => {
                let tag_start = start + SERVER_IMAGE_PREFIX.len();
                let tag_end = line[tag_start..]
                    .find(|ch: char| ch.is_whitespace() || ch == '"' || ch == '\'')
                    .map_or(line.len(), |offset| tag_start + offset);
                count += 1;
                out.push_str(&line[..tag_start]);
                out.push_str(new);
                out.push_str(&line[tag_end..]);
            }
            None => out.push_str(line),
        }
    }
    if count == 0 {
        return Err(format!(
            "deploy/k8s/statefulset.yaml: no `{SERVER_IMAGE_PREFIX}<tag>` image found"
        )
        .into());
    }
    Ok((out, count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_release_and_prerelease_versions_only() {
        for ok in ["0.74.2", "1.0.0", "0.75.0-rc.1"] {
            assert!(is_release_version(ok), "{ok}");
        }
        for bad in ["0.74", "v0.74.2", "0.74.2-", "0.74.x", "0.74.2-rc..1", ""] {
            assert!(!is_release_version(bad), "{bad}");
        }
    }

    #[test]
    fn server_image_rewrite_keeps_quotes_and_suffix() {
        let (out, count) = rewrite_server_image(
            "  image: \"ghcr.io/javaquasar/hydracache-server:0.1.0\"\n",
            "0.2.0",
        )
        .unwrap();
        assert_eq!(count, 1);
        assert_eq!(
            out,
            "  image: \"ghcr.io/javaquasar/hydracache-server:0.2.0\"\n"
        );
    }
}
