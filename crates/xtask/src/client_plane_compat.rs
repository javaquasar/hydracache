use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use flate2::read::GzDecoder;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tar::Archive;

const DEFAULT_MANIFEST: &str = "docs/testing/hc2-compat/v0.68-preview.1.json";
const TARGET_DIR: &str = "target/hc2-compat";
const PEER_TARGET_DIR: &str = "target/hc2-compat/peer";
const CURRENT_RUST_OLD_TARGET_DIR: &str = "target/hc2-compat/current-rust-old";
const CURRENT_RUST_ROLLING_TARGET_DIR: &str = "target/hc2-compat/current-rust-rolling";
const PEER_CRATE: &str = "hydracache-client-plane-spike";
const RUST_ARTIFACT_ID: &str = "rust-h17-preview";
const JAVA_JAR_ID: &str = "java-h17-preview-jar";
const JAVA_POM_ID: &str = "java-h17-preview-pom";
const OLD_DAEMON_LINUX_ID: &str = "daemon-generation5-linux-x86_64";
const OLD_DAEMON_WINDOWS_ID: &str = "daemon-generation5-windows-x86_64";
const NEW_DAEMON_LINUX_ID: &str = "daemon-generation6-linux-x86_64";
const NEW_DAEMON_WINDOWS_ID: &str = "daemon-generation6-windows-x86_64";
const JAVA_CONSUMER: &str = "tests/java-hc2-consumer/pom.xml";
const MAVEN_MAX_ATTEMPTS: u8 = 3;
const MAVEN_RETRY_DELAY_SECONDS: u64 = 5;

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
    #[serde(default)]
    producer_tree: Option<String>,
    #[serde(default)]
    contract_blob: Option<String>,
    #[serde(default)]
    protocol_generation: Option<u32>,
    #[serde(default)]
    platform: Option<String>,
    #[serde(default)]
    executable_path: Option<String>,
    #[serde(default)]
    executable_sha256: Option<String>,
    #[serde(default)]
    executable_size_bytes: Option<u64>,
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
        "manifest-only; cross-version scenarios not executed"
    } else {
        "cross-version scenarios executed"
    };
    let matrix = if manifest.rows.iter().all(|row| row.status == "pass") {
        "matrix complete"
    } else {
        "incomplete rows reported"
    };
    println!("client-plane-compat-check: OK (retained artifacts verified; {execution}; {matrix})");
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
    require_git_ancestor(
        root,
        &manifest.baseline.producer_commit,
        "HEAD",
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
        if !matches!(
            artifact.kind.as_str(),
            "rust-crate" | "java-jar" | "maven-pom" | "production-daemon-tar-gz"
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
        let bytes = canonical_retained_artifact_bytes(&artifact.kind, bytes);
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
        if artifact.kind == "production-daemon-tar-gz" {
            validate_daemon_artifact(root, artifact, &bytes)?;
        } else if artifact.producer_commit != manifest.baseline.producer_commit {
            return Err(format!("artifact {} has a different producer commit", artifact.id).into());
        }
    }
    for required in [
        RUST_ARTIFACT_ID,
        JAVA_JAR_ID,
        JAVA_POM_ID,
        OLD_DAEMON_LINUX_ID,
        OLD_DAEMON_WINDOWS_ID,
        NEW_DAEMON_LINUX_ID,
        NEW_DAEMON_WINDOWS_ID,
    ] {
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

fn canonical_retained_artifact_bytes(kind: &str, bytes: Vec<u8>) -> Vec<u8> {
    if kind != "maven-pom" || !bytes.windows(2).any(|pair| pair == b"\r\n") {
        return bytes;
    }

    let mut canonical = Vec::with_capacity(bytes.len());
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\r' && bytes.get(cursor + 1) == Some(&b'\n') {
            cursor += 1;
        }
        canonical.push(bytes[cursor]);
        cursor += 1;
    }
    canonical
}

fn validate_daemon_artifact(
    root: &Path,
    artifact: &Artifact,
    archive_bytes: &[u8],
) -> Result<(), Box<dyn Error>> {
    let producer_tree = artifact
        .producer_tree
        .as_deref()
        .ok_or("retained daemon lacks producer_tree")?;
    let contract_blob = artifact
        .contract_blob
        .as_deref()
        .ok_or("retained daemon lacks contract_blob")?;
    let protocol_generation = artifact
        .protocol_generation
        .ok_or("retained daemon lacks protocol_generation")?;
    let platform = artifact
        .platform
        .as_deref()
        .ok_or("retained daemon lacks platform")?;
    let executable_path = artifact
        .executable_path
        .as_deref()
        .ok_or("retained daemon lacks executable_path")?;
    let executable_sha256 = artifact
        .executable_sha256
        .as_deref()
        .ok_or("retained daemon lacks executable_sha256")?;
    let executable_size_bytes = artifact
        .executable_size_bytes
        .ok_or("retained daemon lacks executable_size_bytes")?;
    validate_hex("daemon producer_commit", &artifact.producer_commit, 40)?;
    validate_hex("daemon producer_tree", producer_tree, 40)?;
    validate_hex("daemon contract_blob", contract_blob, 40)?;
    validate_hex("daemon executable_sha256", executable_sha256, 64)?;
    validate_relative_path(executable_path)?;
    if !matches!(protocol_generation, 5 | 6)
        || !matches!(platform, "linux-x86_64" | "windows-x86_64")
    {
        return Err(format!("retained daemon {} has an invalid identity", artifact.id).into());
    }
    run_git(
        root,
        &[
            "cat-file",
            "-e",
            &format!("{}^{{commit}}", artifact.producer_commit),
        ],
        "retained daemon producer commit",
    )?;
    require_git_ancestor(
        root,
        &artifact.producer_commit,
        "HEAD",
        "retained daemon producer ancestry",
    )?;
    if git_output(
        root,
        &[
            "rev-parse",
            &format!("{}^{{tree}}", artifact.producer_commit),
        ],
    )? != producer_tree
    {
        return Err(format!("retained daemon {} producer tree mismatch", artifact.id).into());
    }
    if git_output(
        root,
        &[
            "rev-parse",
            &format!(
                "{}:crates/hydracache-client-hc2/proto/hc2_contract.proto",
                artifact.producer_commit
            ),
        ],
    )? != contract_blob
    {
        return Err(format!("retained daemon {} contract blob mismatch", artifact.id).into());
    }

    let mut executable = None;
    let mut receipt = None;
    for entry in Archive::new(GzDecoder::new(archive_bytes)).entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().replace('\\', "/");
        if path == executable_path {
            if !entry.header().entry_type().is_file() {
                return Err("retained daemon executable is not a regular file".into());
            }
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut bytes)?;
            executable = Some(bytes);
        } else if path.ends_with("/receipt.txt") {
            let mut text = String::new();
            std::io::Read::read_to_string(&mut entry, &mut text)?;
            receipt = Some(text);
        }
    }
    let executable = executable.ok_or("retained daemon archive lacks its executable")?;
    if executable.len() as u64 != executable_size_bytes
        || sha256_hex(&executable) != executable_sha256
    {
        return Err(format!("retained daemon {} executable mismatch", artifact.id).into());
    }
    let receipt = receipt.ok_or("retained daemon archive lacks its receipt")?;
    for expected in [
        format!("producer_commit={}", artifact.producer_commit),
        format!("producer_tree={producer_tree}"),
        format!("contract_blob={contract_blob}"),
        format!("platform={platform}"),
        format!("executable_sha256={executable_sha256}"),
        format!("executable_size_bytes={executable_size_bytes}"),
    ] {
        if !receipt.lines().any(|line| line == expected) {
            return Err(format!("retained daemon {} receipt mismatch", artifact.id).into());
        }
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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
    let daemon = extract_daemon(
        root,
        manifest,
        platform_daemon_id(NEW_DAEMON_LINUX_ID, NEW_DAEMON_WINDOWS_ID),
        "retained-daemon-generation6",
    )?;
    let daemon_text = daemon.to_string_lossy().into_owned();
    let retained_daemon = extract_daemon(
        root,
        manifest,
        platform_daemon_id(OLD_DAEMON_LINUX_ID, OLD_DAEMON_WINDOWS_ID),
        "retained-daemon-generation5",
    )?;
    let retained_daemon_text = retained_daemon.to_string_lossy().into_owned();
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
        &[
            ("HC2_COMPAT_INTEROP_SERVER", &peer_text),
            ("HC2_COMPAT_PRODUCTION_DAEMON", &daemon_text),
        ],
        "retained Rust HC/2 additive-field consumer",
    )?;
    run_checked(
        root,
        "cargo",
        &[
            "test",
            "--locked",
            "-p",
            "hydracache-client-hc2",
            "--test",
            "grpc_process",
            "--target-dir",
            CURRENT_RUST_OLD_TARGET_DIR,
        ],
        &[
            ("HC2_RUST_INTEROP_SERVER", &peer_text),
            ("HC2_RUST_PRODUCTION_DAEMON", &retained_daemon_text),
            ("HC2_RUST_PROTOCOL_GENERATION", "5"),
        ],
        "current Rust HC/2 client against retained generation-5 daemon",
    )?;
    run_checked(
        root,
        "cargo",
        &[
            "test",
            "--locked",
            "-p",
            "hydracache-client-hc2",
            "--test",
            "grpc_process",
            "generation_five_client_rolls_from_retained_to_current_production_daemon",
            "--target-dir",
            CURRENT_RUST_ROLLING_TARGET_DIR,
            "--",
            "--exact",
        ],
        &[
            ("HC2_RUST_INTEROP_SERVER", &peer_text),
            ("HC2_RUST_RETAINED_PRODUCTION_DAEMON", &retained_daemon_text),
            ("HC2_RUST_PRODUCTION_DAEMON", &daemon_text),
        ],
        "rolling replacement from retained generation-5 to current generation-6 daemon",
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
            "-Dhc2.compat.production.required=true",
            "-Dhc2.expected.protocol.generation=5",
            "clean",
            "verify",
        ],
        &[
            ("HC2_JAVA_INTEROP_SERVER", &peer_text),
            ("HC2_COMPAT_INTEROP_SERVER", &peer_text),
            ("HC2_COMPAT_PRODUCTION_DAEMON", &daemon_text),
        ],
        "retained Java HC/2 client artifact",
    )?;
    run_checked(
        root,
        maven_program(),
        &[
            "-B",
            "-ntp",
            "-f",
            "sdks/java/hydracache-client-hc2/pom.xml",
            "-Dtest=GrpcHydraCacheClientInteropTest",
            "test",
        ],
        &[
            ("HC2_JAVA_INTEROP_SERVER", &peer_text),
            ("HC2_JAVA_DAEMON", &retained_daemon_text),
            ("HC2_JAVA_PROTOCOL_GENERATION", "5"),
        ],
        "current Java HC/2 client against retained generation-5 daemon",
    )?;
    run_checked(
        root,
        maven_program(),
        &[
            "-B",
            "-ntp",
            "-f",
            "sdks/java/hydracache-client-hc2/pom.xml",
            "-Dtest=GrpcHydraCacheClientInteropTest#javaSdkExecutesAgainstTheProductionDaemonAndDrainsCleanly",
            "test",
        ],
        &[
            ("HC2_JAVA_INTEROP_SERVER", &peer_text),
            ("HC2_JAVA_DAEMON", &daemon_text),
            ("HC2_JAVA_PROTOCOL_GENERATION", "5"),
            ("HC2_JAVA_EXPECTED_PREFERRED_GENERATION", "6"),
            ("HC2_JAVA_EXPECTED_DEPRECATED", "true"),
        ],
        "current Java generation-5 client inside the generation-6 deprecation window",
    )?;
    run_checked(
        root,
        "cargo",
        &[
            "test",
            "--locked",
            "-p",
            "hydracache-server",
            "--test",
            "hc2_daemon_process",
            "real_daemon_shares_hc1_hc2_dispatch_and_exits_on_drain",
            "--target-dir",
            PEER_TARGET_DIR,
            "--",
            "--exact",
        ],
        &[],
        "concurrent HC/1 and HC/2 production listeners",
    )?;
    Ok(())
}

fn platform_daemon_id<'a>(linux: &'a str, windows: &'a str) -> &'a str {
    if cfg!(windows) {
        windows
    } else {
        linux
    }
}

fn extract_daemon(
    root: &Path,
    manifest: &CompatibilityManifest,
    id: &str,
    unpack_directory: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    let artifact = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.id == id)
        .ok_or_else(|| format!("retained daemon artifact is absent: {id}"))?;
    let executable_path = artifact
        .executable_path
        .as_deref()
        .ok_or("retained daemon lacks executable_path")?;
    let unpack_root = root.join(TARGET_DIR).join(unpack_directory);
    if unpack_root.exists() {
        fs::remove_dir_all(&unpack_root)?;
    }
    fs::create_dir_all(&unpack_root)?;
    Archive::new(GzDecoder::new(fs::File::open(root.join(&artifact.path))?))
        .unpack(&unpack_root)?;
    let executable = unpack_root.join(executable_path);
    if !executable.is_file() {
        return Err(format!(
            "retained daemon executable is absent after extraction: {}",
            executable.display()
        )
        .into());
    }
    Ok(executable)
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
        || value.is_empty()
        || value.contains(['\\', ':'])
        || value
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
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
    let max_attempts = if is_maven_program(program) {
        MAVEN_MAX_ATTEMPTS
    } else {
        1
    };
    for attempt in 1..=max_attempts {
        let status = Command::new(program)
            .args(args)
            .envs(environment.iter().copied())
            .current_dir(root)
            .status()
            .map_err(|error| format!("starting {label}: {error}"))?;
        if status.success() {
            return Ok(());
        }
        if attempt == max_attempts {
            return Err(format!("{label} failed with {status} after {attempt} attempt(s)").into());
        }
        eprintln!(
            "{label} failed with {status}; retrying bounded Maven execution ({}/{max_attempts})",
            attempt + 1
        );
        thread::sleep(Duration::from_secs(MAVEN_RETRY_DELAY_SECONDS));
    }
    unreachable!("bounded attempt range is non-empty")
}

fn is_maven_program(program: &str) -> bool {
    Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.eq_ignore_ascii_case("mvn") || name.eq_ignore_ascii_case("mvn.cmd")
        })
}

fn run_git(root: &Path, args: &[&str], label: &str) -> Result<(), Box<dyn Error>> {
    let output = Command::new("git").args(args).current_dir(root).output()?;
    if output.status.success() {
        return Ok(());
    }
    let head = git_output(root, &["rev-parse", "HEAD"])
        .unwrap_or_else(|error| format!("unavailable ({error})"));
    Err(format!(
        "{label} failed with {} (cwd={}, head={head}, command=git {}, stdout={}, stderr={})",
        output.status,
        root.display(),
        args.join(" "),
        String::from_utf8_lossy(&output.stdout).trim(),
        String::from_utf8_lossy(&output.stderr).trim(),
    )
    .into())
}

fn require_git_ancestor(
    root: &Path,
    ancestor: &str,
    descendant: &str,
    label: &str,
) -> Result<(), Box<dyn Error>> {
    let merge_base = Command::new("git")
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .current_dir(root)
        .output()?;
    if merge_base.status.success() {
        return Ok(());
    }

    let reachable = Command::new("git")
        .args(["rev-list", descendant])
        .current_dir(root)
        .output()?;
    if reachable.status.success() && reachable_commits_include(&reachable.stdout, ancestor) {
        eprintln!(
            "{label}: git merge-base returned {}; exact reachability via git rev-list succeeded",
            merge_base.status
        );
        return Ok(());
    }

    let head = git_output(root, &["rev-parse", descendant])
        .unwrap_or_else(|error| format!("unavailable ({error})"));
    let shallow = git_output(root, &["rev-parse", "--is-shallow-repository"])
        .unwrap_or_else(|error| format!("unavailable ({error})"));
    Err(format!(
        "{label} failed (cwd={}, descendant={head}, ancestor={ancestor}, shallow={shallow}, merge_base_status={}, merge_base_stdout={}, merge_base_stderr={}, rev_list_status={}, rev_list_stderr={})",
        root.display(),
        merge_base.status,
        String::from_utf8_lossy(&merge_base.stdout).trim(),
        String::from_utf8_lossy(&merge_base.stderr).trim(),
        reachable.status,
        String::from_utf8_lossy(&reachable.stderr).trim(),
    )
    .into())
}

fn reachable_commits_include(output: &[u8], expected: &str) -> bool {
    String::from_utf8_lossy(output)
        .lines()
        .any(|commit| commit == expected)
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
    fn retained_maven_pom_identity_is_stable_across_checkout_line_endings() {
        let lf = b"<project>\n  <version>1</version>\n</project>\n".to_vec();
        let crlf = b"<project>\r\n  <version>1</version>\r\n</project>\r\n".to_vec();

        assert_eq!(canonical_retained_artifact_bytes("maven-pom", crlf), lf);
        assert_eq!(
            canonical_retained_artifact_bytes("java-jar", b"a\r\nb".to_vec()),
            b"a\r\nb"
        );
    }

    #[test]
    fn traversal_and_skip_status_are_not_accepted() {
        assert!(validate_relative_path("../artifact.jar").is_err());
        assert!(validate_relative_path("C:/artifact.jar").is_err());
        assert!(!matches!("skip", "pass" | "baseline-smoke" | "blocked"));
    }

    #[test]
    fn reachability_requires_an_exact_commit_line() {
        let reachable = format!("{}\n{}\n", "a".repeat(40), "b".repeat(40));
        assert!(reachable_commits_include(
            reachable.as_bytes(),
            &"a".repeat(40)
        ));
        assert!(!reachable_commits_include(
            reachable.as_bytes(),
            &"a".repeat(39)
        ));
    }

    #[test]
    fn bounded_retry_is_scoped_only_to_maven_executables() {
        assert!(is_maven_program("mvn"));
        assert!(is_maven_program("mvn.cmd"));
        assert!(is_maven_program("/opt/apache-maven/bin/mvn"));
        #[cfg(windows)]
        assert!(is_maven_program(r"C:\tools\maven\mvn.cmd"));
        assert!(!is_maven_program("cargo"));
        assert!(!is_maven_program("java"));
        assert!(!is_maven_program("mvn-wrapper"));
    }
}
