use crate::imap_value_plane_model::sha256_file;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const RELEASE: &str = "0.75";
const UPSTREAM_SHA: &str = "880efabcd8eabd0c2595749ee87f495c6a45895c";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceMap {
    release: String,
    status: String,
    upstream_url: String,
    upstream_sha: String,
    upstream_license: String,
    sources: Vec<SourceEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceEntry {
    path: String,
    sha256: String,
    principle: String,
    hydracache_target: String,
    adapted_test_ids: Vec<String>,
    non_adopted: String,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let (release, upstream) = parse_args(&args)?;
    let root = crate::doc_check::find_repo_root()?;
    let problems = check_at_root(&root, &release, upstream.as_deref())?;
    if problems.is_empty() {
        println!("imap-hazelcast-source-check {release}: OK (13 pinned sources)");
        return Ok(());
    }
    for problem in &problems {
        eprintln!("imap-hazelcast-source-check {release}: {problem}");
    }
    Err(format!("Hazelcast source check found {} problem(s)", problems.len()).into())
}

pub fn check_at_root(
    root: &Path,
    release: &str,
    upstream: Option<&Path>,
) -> Result<Vec<String>, Box<dyn Error>> {
    if release != RELEASE {
        return Ok(vec![format!(
            "unsupported release {release}; expected {RELEASE}"
        )]);
    }
    let path = root.join("docs/testing/imap/0.75/hazelcast-source-map.toml");
    check_map_file(root, &path, upstream)
}

pub fn check_map_file(
    root: &Path,
    path: &Path,
    upstream: Option<&Path>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let map: SourceMap = toml::from_str(&fs::read_to_string(path)?)?;
    let mut problems = Vec::new();
    if map.release != RELEASE || map.status != "provisional" {
        problems.push("source map must remain provisional for release 0.75".into());
    }
    if map.upstream_sha != UPSTREAM_SHA || map.upstream_sha.len() != 40 {
        problems.push("source map has an unexpected upstream SHA".into());
    }
    if map.upstream_url != "https://github.com/hazelcast/hazelcast"
        || map.upstream_license != "Apache-2.0"
    {
        problems.push("source map must identify the licensed Hazelcast upstream".into());
    }
    if map.sources.len() != 13 {
        problems.push("source map must contain exactly 13 reviewed source anchors".into());
    }
    let mut paths = BTreeSet::new();
    for entry in &map.sources {
        if !paths.insert(&entry.path) {
            problems.push(format!("duplicate source path {}", entry.path));
        }
        if Path::new(&entry.path).is_absolute() || entry.path.contains("..") {
            problems.push(format!("unsafe source path {}", entry.path));
        }
        if entry.sha256.len() != 64 || !entry.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            problems.push(format!("source {} has invalid SHA-256", entry.path));
        }
        for (label, value) in [
            ("principle", &entry.principle),
            ("HydraCache target", &entry.hydracache_target),
            ("non-adopted note", &entry.non_adopted),
        ] {
            if value.trim().is_empty() {
                problems.push(format!("source {} has no {label}", entry.path));
            }
        }
        if entry.adapted_test_ids.is_empty()
            || entry.adapted_test_ids.iter().any(|id| id.trim().is_empty())
        {
            problems.push(format!("source {} has no adapted test id", entry.path));
        }
        if !root.join(&entry.hydracache_target).exists() {
            problems.push(format!(
                "source {} points to missing HydraCache target {}",
                entry.path, entry.hydracache_target
            ));
        }
    }
    if let Some(upstream) = upstream {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(upstream)
            .output()?;
        let actual = String::from_utf8(output.stdout)?.trim().to_owned();
        if !output.status.success() || actual != map.upstream_sha {
            problems.push(format!(
                "upstream checkout is {actual}, expected {}",
                map.upstream_sha
            ));
        }
        for entry in &map.sources {
            let source = upstream.join(&entry.path);
            if !source.is_file() {
                problems.push(format!("upstream source is missing: {}", entry.path));
            } else if sha256_file(&source)? != entry.sha256 {
                problems.push(format!("upstream source digest changed: {}", entry.path));
            }
        }
    }
    Ok(problems)
}

fn parse_args(args: &[String]) -> Result<(String, Option<PathBuf>), Box<dyn Error>> {
    let mut release = None;
    let mut upstream = None;
    let mut index = 0;
    while index < args.len() {
        let name = args[index].as_str();
        index += 1;
        let value = args
            .get(index)
            .ok_or_else(|| format!("{name} requires a value"))?;
        match name {
            "--release" => release = Some(value.clone()),
            "--upstream" => upstream = Some(PathBuf::from(value)),
            other => return Err(format!("unknown source-check argument: {other}").into()),
        }
        index += 1;
    }
    Ok((
        release.ok_or("source-check requires --release 0.75")?,
        upstream,
    ))
}
