use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use flate2::read::GzDecoder;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tar::Archive;

const DEFAULT_MANIFEST: &str = "docs/testing/hc2-compat/v0.68-preview.1.json";
const TARGET_DIR: &str = "target/hc2-compat";
const PEER_TARGET_DIR: &str = "target/hc2-compat/peer";
const PEER_CRATE: &str = "hydracache-client-plane-spike";
const RUST_ARTIFACT_ID: &str = "rust-h17-preview";
const JAVA_JAR_ID: &str = "java-h17-preview-jar";
const JAVA_POM_ID: &str = "java-h17-preview-pom";
const JAVA_CONSUMER: &str = "tests/java-hc2-consumer/pom.xml";

const REQUIRED_SCENARIOS: &[&str] = &[
    "retained-rust-client-current-peer",
    "retained-java-client-current-peer",
    "old-client-new-daemon",
    "current-client-old-daemon",
    "hc1-hc2-concurrent-listeners",
    "rolling-upgrade",
    "capability-negotiation",
    "unknown-fields",
    "planned-deprecation",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompatibilityManifest {
    schema_version: u32,
    baseline: Baseline,
    artifacts: Vec<Artifact>,
    rows: Vec<MatrixRow>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Baseline {
    id: String,
    protocol_generation: u32,
    producer_commit: String,
    producer_tree: String,
    contract_blob: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    id: String,
    kind: String,
    path: String,
    sha256: String,
    size_bytes: u64,
    producer_commit: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MatrixRow {
    id: String,
    scenario: String,
    status: String,
    evidence: Vec<String>,
    blockers: Vec<String>,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mut manifest_only = false;
    let mut require_complete = false;
    for argument in args {
        match argument.as_str() {
            "--manifest-only" => manifest_only = true,
            "--require-complete" => require_complete = true,
            _ => {
                return Err(
                    format!("unsupported client-plane-compat-check argument: {argument}").into(),
                )
            }
        }
    }
    check_at_root(&workspace_root()?, manifest_only, require_complete)
}

pub fn check_at_root(
    root: &Path,
    manifest_only: bool,
    require_complete: bool,
) -> Result<(), Box<dyn Error>> {
    let manifest_path = root.join(DEFAULT_MANIFEST);
    let manifest: CompatibilityManifest = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    validate_manifest(root, &manifest)?;
    if !manifest_only {
        run_baseline_smoke(root, &manifest)?;
    }
    report_matrix(&manifest);
    if require_complete {
        let incomplete: Vec<_> = manifest
            .rows
            .iter()
            .filter(|row| row.status != "pass")
            .map(|row| format!("{}={}", row.id, row.status))
            .collect();
        if !incomplete.is_empty() {
            return Err(format!(
                "HC/2 compatibility matrix is incomplete: {}",
                incomplete.join(", ")
            )
            .into());
        }
    }
    let execution = if manifest_only {
        "manifest-only; baseline smoke not executed"
    } else {
        "baseline smoke executed"
    };
    println!(
        "client-plane-compat-check: OK (retained artifacts verified; {execution}; incomplete rows reported)"
    );
    Ok(())
}

fn validate_manifest(root: &Path, manifest: &CompatibilityManifest) -> Result<(), Box<dyn Error>> {
    if manifest.schema_version != 1 || manifest.baseline.protocol_generation != 5 {
        return Err("unsupported HC/2 compatibility manifest schema or generation".into());
    }
    validate_hex("producer_commit", &manifest.baseline.producer_commit, 40)?;
    validate_hex("producer_tree", &manifest.baseline.producer_tree, 40)?;
    validate_hex("contract_blob", &manifest.baseline.contract_blob, 40)?;
    run_git(
        root,
        &[
            "cat-file",
            "-e",
            &format!("{}^{{commit}}", manifest.baseline.producer_commit),
        ],
        "retained producer commit",
    )?;
    run_git(
        root,
        &[
            "merge-base",
            "--is-ancestor",
            &manifest.baseline.producer_commit,
            "HEAD",
        ],
        "retained producer ancestry",
    )?;
    let tree = git_output(
        root,
        &[
            "rev-parse",
            &format!("{}^{{tree}}", manifest.baseline.producer_commit),
        ],
    )?;
    if tree != manifest.baseline.producer_tree {
        return Err("retained producer tree does not match producer commit".into());
    }
    let baseline_blob = git_output(
        root,
        &[
            "rev-parse",
            &format!(
                "{}:crates/hydracache-client-hc2/proto/hc2_contract.proto",
                manifest.baseline.producer_commit
            ),
        ],
    )?;
    if baseline_blob != manifest.baseline.contract_blob {
        return Err("retained contract blob does not match producer commit".into());
    }

    let mut artifact_ids = BTreeSet::new();
    for artifact in &manifest.artifacts {
        if !artifact_ids.insert(artifact.id.as_str()) {
            return Err(format!("duplicate compatibility artifact id: {}", artifact.id).into());
        }
        if artifact.producer_commit != manifest.baseline.producer_commit {
            return Err(format!("artifact {} has a different producer commit", artifact.id).into());
        }
        if !matches!(
            artifact.kind.as_str(),
            "rust-crate" | "java-jar" | "maven-pom"
        ) {
            return Err(format!(
                "artifact {} has unsupported kind {}",
                artifact.id, artifact.kind
            )
            .into());
        }
        validate_relative_path(&artifact.path)?;
        validate_hex("artifact sha256", &artifact.sha256, 64)?;
        let path = root.join(&artifact.path);
        let bytes = fs::read(&path)
            .map_err(|error| format!("reading retained artifact {}: {error}", path.display()))?;
        if bytes.len() as u64 != artifact.size_bytes {
            return Err(format!("retained artifact {} size mismatch", artifact.id).into());
        }
        let actual: String = Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        if actual != artifact.sha256 {
            return Err(format!("retained artifact {} digest mismatch", artifact.id).into());
        }
    }
    for required in [RUST_ARTIFACT_ID, JAVA_JAR_ID, JAVA_POM_ID] {
        if !artifact_ids.contains(required) {
            return Err(format!("required retained artifact is absent: {required}").into());
        }
    }

    let mut rows = BTreeSet::new();
    let mut scenarios = BTreeSet::new();
    let current_contract = git_output(
        root,
        &[
            "hash-object",
            "crates/hydracache-client-hc2/proto/hc2_contract.proto",
        ],
    )?;
    for row in &manifest.rows {
        if !rows.insert(row.id.as_str()) {
            return Err(format!("duplicate compatibility row id: {}", row.id).into());
        }
        scenarios.insert(row.scenario.as_str());
        if !matches!(row.status.as_str(), "pass" | "baseline-smoke" | "blocked") {
            return Err(format!("row {} has forbidden status {}", row.id, row.status).into());
        }
        match row.status.as_str() {
            "blocked" if row.blockers.is_empty() => {
                return Err(format!("blocked row {} lacks blockers", row.id).into())
            }
            "pass" | "baseline-smoke" if row.evidence.is_empty() => {
                return Err(format!("evidenced row {} lacks evidence", row.id).into())
            }
            _ => {}
        }
        if row.status == "pass" && current_contract == manifest.baseline.contract_blob {
            return Err(format!(
                "row {} claims cross-version pass against the identical contract blob",
                row.id
            )
            .into());
        }
    }
    for scenario in REQUIRED_SCENARIOS {
        if !scenarios.contains(scenario) {
            return Err(format!("required compatibility scenario is absent: {scenario}").into());
        }
    }
    Ok(())
}

fn run_baseline_smoke(root: &Path, manifest: &CompatibilityManifest) -> Result<(), Box<dyn Error>> {
    run_checked(
        root,
        "cargo",
        &[
            "build",
            "--locked",
            "-p",
            PEER_CRATE,
            "--bin",
            "hc2_java_interop_server",
            "--target-dir",
            PEER_TARGET_DIR,
        ],
        &[],
        "HC/2 compatibility conformance peer",
    )?;
    let peer = peer_path(root);
    if !peer.is_file() {
        return Err(format!("HC/2 compatibility peer is absent: {}", peer.display()).into());
    }

    let rust_artifact = artifact_path(root, manifest, RUST_ARTIFACT_ID)?;
    let unpack_root = root.join(TARGET_DIR).join("retained-rust");
    if unpack_root.exists() {
        fs::remove_dir_all(&unpack_root)?;
    }
    fs::create_dir_all(&unpack_root)?;
    Archive::new(GzDecoder::new(fs::File::open(rust_artifact)?)).unpack(&unpack_root)?;
    let crate_root = unpack_root.join("hydracache-client-hc2-0.67.0");
    let vcs: serde_json::Value =
        serde_json::from_slice(&fs::read(crate_root.join(".cargo_vcs_info.json"))?)?;
    if vcs.pointer("/git/sha1").and_then(serde_json::Value::as_str)
        != Some(manifest.baseline.producer_commit.as_str())
    {
        return Err("retained Rust crate VCS identity does not match the manifest".into());
    }
    let packaged_contract = git_output(
        root,
        &[
            "hash-object",
            &crate_root
                .join("proto/hc2_contract.proto")
                .to_string_lossy(),
        ],
    )?;
    if packaged_contract != manifest.baseline.contract_blob {
        return Err("retained Rust crate contract differs from the producer contract blob".into());
    }
    // Cargo discovers the repository workspace through parent directories even
    // for an extracted published crate. Isolate this temporary build tree
    // without changing any retained source or the checksummed archive.
    fs::OpenOptions::new()
        .append(true)
        .open(crate_root.join("Cargo.toml"))?
        .write_all(b"\n[workspace]\n")?;
    let peer_text = peer.to_string_lossy().into_owned();
    let crate_manifest = crate_root.join("Cargo.toml").to_string_lossy().into_owned();
    let rust_target = root
        .join(TARGET_DIR)
        .join("retained-rust-target")
        .to_string_lossy()
        .into_owned();
    run_checked(
        root,
        "cargo",
        &[
            "test",
            "--locked",
            "--manifest-path",
            &crate_manifest,
            "--target-dir",
            &rust_target,
        ],
        &[("HC2_RUST_INTEROP_SERVER", &peer_text)],
        "retained Rust HC/2 client artifact",
    )?;
    run_checked(
        root,
        "cargo",
        &[
            "test",
            "--locked",
            "--manifest-path",
            "tests/rust-hc2-compat-consumer/Cargo.toml",
            "--target-dir",
            &rust_target,
        ],
        &[],
        "retained Rust HC/2 additive-field consumer",
    )?;

    let m2 = root.join(TARGET_DIR).join("m2");
    fs::create_dir_all(&m2)?;
    let m2_arg = format!("-Dmaven.repo.local={}", m2.to_string_lossy());
    let jar = artifact_path(root, manifest, JAVA_JAR_ID)?
        .to_string_lossy()
        .into_owned();
    let pom = artifact_path(root, manifest, JAVA_POM_ID)?
        .to_string_lossy()
        .into_owned();
    let file_arg = format!("-Dfile={jar}");
    let pom_arg = format!("-DpomFile={pom}");
    run_checked(
        root,
        maven_program(),
        &[
            "-B",
            "-ntp",
            &m2_arg,
            "org.apache.maven.plugins:maven-install-plugin:3.1.2:install-file",
            &file_arg,
            &pom_arg,
        ],
        &[],
        "installing retained Java HC/2 artifact into isolated repository",
    )?;
    run_checked(
        root,
        maven_program(),
        &[
            "-B",
            "-ntp",
            &m2_arg,
            "-f",
            JAVA_CONSUMER,
            "clean",
            "verify",
        ],
        &[("HC2_JAVA_INTEROP_SERVER", &peer_text)],
        "retained Java HC/2 client artifact",
    )?;
    Ok(())
}

fn report_matrix(manifest: &CompatibilityManifest) {
    let mut totals = BTreeMap::<&str, usize>::new();
    for row in &manifest.rows {
        *totals.entry(&row.status).or_default() += 1;
        if row.status == "blocked" {
            println!("BLOCKED {}: {}", row.id, row.blockers.join("; "));
        } else {
            println!("{} {}", row.status.to_uppercase(), row.id);
        }
    }
    println!(
        "HC/2 compatibility baseline {}: pass={}, baseline-smoke={}, blocked={}",
        manifest.baseline.id,
        totals.get("pass").copied().unwrap_or_default(),
        totals.get("baseline-smoke").copied().unwrap_or_default(),
        totals.get("blocked").copied().unwrap_or_default()
    );
}

fn artifact_path<'a>(
    root: &'a Path,
    manifest: &'a CompatibilityManifest,
    id: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.id == id)
        .map(|artifact| root.join(&artifact.path))
        .ok_or_else(|| format!("retained artifact is absent: {id}").into())
}

fn validate_relative_path(value: &str) -> Result<(), Box<dyn Error>> {
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("artifact path must be a normalized relative path: {value}").into());
    }
    Ok(())
}

fn validate_hex(label: &str, value: &str, length: usize) -> Result<(), Box<dyn Error>> {
    if value.len() != length || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{label} must be {length} hexadecimal characters").into());
    }
    Ok(())
}

fn peer_path(root: &Path) -> PathBuf {
    root.join(PEER_TARGET_DIR)
        .join("debug")
        .join(if cfg!(windows) {
            "hc2_java_interop_server.exe"
        } else {
            "hc2_java_interop_server"
        })
}

fn maven_program() -> &'static str {
    if cfg!(windows) {
        "mvn.cmd"
    } else {
        "mvn"
    }
}

fn run_checked(
    root: &Path,
    program: &str,
    args: &[&str],
    environment: &[(&str, &str)],
    label: &str,
) -> Result<(), Box<dyn Error>> {
    let status = Command::new(program)
        .args(args)
        .envs(environment.iter().copied())
        .current_dir(root)
        .status()
        .map_err(|error| format!("starting {label}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{label} failed with {status}").into())
    }
}

fn run_git(root: &Path, args: &[&str], label: &str) -> Result<(), Box<dyn Error>> {
    run_checked(root, "git", args, &[], label)
}

fn git_output(root: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git").args(args).current_dir(root).output()?;
    if !output.status.success() {
        return Err(format!("git {} failed", args.join(" ")).into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn workspace_root() -> Result<PathBuf, Box<dyn Error>> {
    let mut candidate = std::env::current_dir()?;
    loop {
        if candidate.join("Cargo.toml").is_file() && candidate.join(DEFAULT_MANIFEST).is_file() {
            return Ok(candidate);
        }
        if !candidate.pop() {
            return Err("could not locate HydraCache workspace root".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_manifest_is_structurally_valid() {
        let root = workspace_root().unwrap();
        let manifest: CompatibilityManifest =
            serde_json::from_slice(&fs::read(root.join(DEFAULT_MANIFEST)).unwrap()).unwrap();
        validate_manifest(&root, &manifest).unwrap();
    }

    #[test]
    fn traversal_and_skip_status_are_not_accepted() {
        assert!(validate_relative_path("../artifact.jar").is_err());
        assert!(validate_relative_path("C:/artifact.jar").is_err());
        assert!(!matches!("skip", "pass" | "baseline-smoke" | "blocked"));
    }
}
