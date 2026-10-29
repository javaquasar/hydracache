//! Predecessor-independent integration preparation. Never produces a production receipt.
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::imap_daemon_probe_075::{self as daemon, DaemonProbe};
use crate::imap_extended_receipt_075::{artifact, Artifact};
use crate::imap_qualification_075::{digest, read_relative, MAX_ARTIFACT_BYTES};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const CONTRACT: &str = "docs/testing/imap/0.75/pre074-integration-contract.json";
const JAVA_MODULE: &str = "tests/java-imap-hazelcast-075";
const JAVA_CONTRACT: &str =
    "tests/java-imap-benchmark/target/java-imap-benchmark-0.75.0-SNAPSHOT.jar";
const JAVA_ADAPTER: &str =
    "tests/java-imap-hazelcast-075/target/java-imap-hazelcast-075-0.75.0-SNAPSHOT.jar";
const HAZELCAST: &str = "tests/java-imap-hazelcast-075/target/dependencies/hazelcast-5.5.0.jar";
const HAZELCAST_POM: &str = "tests/java-imap-hazelcast-075/target/dependencies/hazelcast-5.5.0.pom";
pub const HAZELCAST_SHA256: &str =
    "cf560b40b988526e780e4fae12cb6fe01c5158bb9e00839882857f32b544a19e";
const BLOCKERS: &[&str] = &[
    "w0_closure",
    "production_backend_extraction_and_routing",
    "final_wire_and_durable_identities",
    "production_sdk_generation",
    "accepted_074_non_regression",
    "real_cluster_s2_and_release_admission",
];
const PRODUCTION_OBSERVATIONS: &[&str] = &[
    "authoritative_partition_epoch_and_owner",
    "exact_required_replica_proof_before_public_ack",
    "durable_value_expiry_tombstone_dedup_atomic_commit",
    "listener_partition_watermark_gap_and_resync_cut",
    "authenticated_member_tenant_generation_and_trust_epoch",
    "readiness_from_actual_replica_and_repair_state",
    "drain_and_namespace_reclamation_zero_old_owners",
    "bounded_data_safety_repair_lanes_and_noisy_tenant_progress",
];

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegrationContract {
    pub release: String,
    pub scope: String,
    pub production_enabled: bool,
    pub predecessor_required: String,
    pub profiles: Vec<String>,
    pub production_observations_required: Vec<String>,
    pub blocked_until_074: Vec<String>,
    pub tests: Vec<String>,
}
pub fn validate_contract(
    root: &Path,
    contract: &IntegrationContract,
) -> Result<(), Box<dyn Error>> {
    if contract.release != "0.75"
        || contract.scope != "pre074_preparation"
        || contract.production_enabled
        || contract.predecessor_required != "v0.74.0"
        || contract.blocked_until_074 != BLOCKERS
        || contract.production_observations_required != PRODUCTION_OBSERVATIONS
        || contract.profiles
            != [
                "current_daemon_local_characterization",
                "reference_backend_contract",
                "existing_durable_v1_characterization",
                "official_hazelcast_diagnostic",
            ]
        || contract.tests.len() != 10
        || contract.tests.iter().collect::<BTreeSet<_>>().len() != 10
    {
        return Err("pre-0.74 contract weakened, incomplete or production-enabled".into());
    }
    for test in &contract.tests {
        read_relative(root, test, MAX_ARTIFACT_BYTES)?;
    }
    Ok(())
}

/// Follow normal/build workspace edges, including optional/target-specific ones, but not dev edges.
pub fn validate_production_dependencies(metadata: &Value) -> Result<(), Box<dyn Error>> {
    let packages = metadata["packages"]
        .as_array()
        .ok_or("cargo packages missing")?;
    if packages.len() > 512 {
        return Err("workspace package bound".into());
    }
    let mut by_name = BTreeMap::new();
    for package in packages {
        let name = package["name"]
            .as_str()
            .ok_or("cargo package name missing")?;
        if by_name.insert(name, package).is_some() {
            return Err("duplicate workspace package name".into());
        }
    }
    let mut queue = VecDeque::from([
        "hydracache-server",
        "hydracache-client-hc2",
        "hydracache-client-transport-axum",
    ]);
    let mut seen = BTreeSet::new();
    while let Some(name) = queue.pop_front() {
        if !seen.insert(name) {
            continue;
        }
        if matches!(name, "hydracache-cluster-testkit" | "xtask") {
            return Err(
                "development testkit/xtask is reachable from production dependencies".into(),
            );
        }
        let package = by_name
            .get(name)
            .ok_or("production workspace package missing")?;
        for dependency in package["dependencies"]
            .as_array()
            .ok_or("cargo dependencies missing")?
        {
            let kind = dependency["kind"].as_str();
            if !matches!(kind, None | Some("dev" | "build")) {
                return Err("unknown dependency kind".into());
            }
            if kind == Some("dev") {
                continue;
            }
            let dependency = dependency["name"]
                .as_str()
                .ok_or("dependency name missing")?;
            if by_name.contains_key(dependency) {
                queue.push_back(dependency);
            }
        }
    }
    Ok(())
}

fn metadata(root: &Path) -> Result<Value, Box<dyn Error>> {
    let output = Command::new("cargo")
        .current_dir(root)
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--locked",
            "--offline",
        ])
        .output()?;
    if !output.status.success() || output.stdout.len() > MAX_ARTIFACT_BYTES {
        return Err("cargo metadata failed or exceeded bound".into());
    }
    let value = serde_json::from_slice(&output.stdout)?;
    validate_production_dependencies(&value)?;
    Ok(value)
}
pub fn check_contract(root: &Path) -> Result<(), Box<dyn Error>> {
    let contract = serde_json::from_slice(&read_relative(root, CONTRACT, MAX_ARTIFACT_BYTES)?)?;
    validate_contract(root, &contract)?;
    crate::imap_ci_075::check_workflow(root)?;
    metadata(root)?;
    let status: Value = serde_json::from_slice(&read_relative(
        root,
        "docs/testing/imap/0.75/status.json",
        MAX_ARTIFACT_BYTES,
    )?)?;
    if status["w0_disposition"] != "open"
        || status["production_capability"] != "disabled_fail_closed"
    {
        return Err("preparation requires the explicit provisional production boundary".into());
    }
    Ok(())
}

fn command(
    builds: &mut crate::imap_build_process_075::BuildController,
    root: &Path,
    stage: crate::imap_build_process_075::BuildStage,
    program: &str,
    args: &[&str],
) -> Result<(), Box<dyn Error>> {
    let mut invocation = Command::new(program);
    invocation.current_dir(root).args(args);
    builds.run(stage, invocation)?;
    Ok(())
}

fn rust_consumer(
    root: &Path,
    builds: &mut crate::imap_build_process_075::BuildController,
) -> Result<(), Box<dyn Error>> {
    let scratch = root.join("target/imap-075");
    fs::create_dir_all(&scratch)?;
    let temp = tempfile::tempdir_in(&scratch)?;
    fs::create_dir(temp.path().join("src"))?;
    fs::copy(
        root.join("tests/rust-imap-contract-consumer-075/src/main.rs"),
        temp.path().join("src/main.rs"),
    )?;
    let dependency = root
        .join("crates/hydracache-cluster-testkit")
        .to_string_lossy()
        .replace('\\', "/");
    let manifest = toml::to_string(
        &json!({"package":{"name":"imap-contract-consumer-075","version":"0.0.0","edition":"2021","publish":false},"workspace":{},"dependencies":{"hydracache-cluster-testkit":{"path":dependency}}}),
    )?;
    fs::write(temp.path().join("Cargo.toml"), manifest)?;
    fs::copy(root.join("Cargo.lock"), temp.path().join("Cargo.lock"))?;
    for action in ["test", "build"] {
        let mut invocation = Command::new("cargo");
        invocation
            .current_dir(temp.path())
            .args([action, "--offline"])
            .env("CARGO_TARGET_DIR", root.join("target"));
        if action == "build" {
            invocation.arg("--locked");
        }
        builds.run(
            if action == "test" {
                crate::imap_build_process_075::BuildStage::RustConsumerTests
            } else {
                crate::imap_build_process_075::BuildStage::RustConsumerBuild
            },
            invocation,
        )?;
        validate_consumer_lock(
            &fs::read_to_string(root.join("Cargo.lock"))?,
            &fs::read_to_string(temp.path().join("Cargo.lock"))?,
        )?;
    }
    Ok(())
}

pub fn validate_consumer_lock(seed: &str, resolved: &str) -> Result<(), Box<dyn Error>> {
    if seed.len() > MAX_ARTIFACT_BYTES || resolved.len() > MAX_ARTIFACT_BYTES {
        return Err("consumer lock byte bound".into());
    }
    type LockIdentity = (String, String, Option<String>, Option<String>);
    fn identities(source: &str) -> Result<BTreeSet<LockIdentity>, Box<dyn Error>> {
        let document: toml::Value = toml::from_str(source)?;
        if document.get("version").and_then(toml::Value::as_integer) != Some(4) {
            return Err("unsupported consumer lock format".into());
        }
        let packages = document
            .get("package")
            .and_then(toml::Value::as_array)
            .ok_or("lock package table missing")?;
        if packages.len() > 4096 {
            return Err("consumer lock package bound".into());
        }
        let mut result = BTreeSet::new();
        for package in packages {
            let identity = (
                package
                    .get("name")
                    .and_then(toml::Value::as_str)
                    .ok_or("lock package name missing")?
                    .to_owned(),
                package
                    .get("version")
                    .and_then(toml::Value::as_str)
                    .ok_or("lock package version missing")?
                    .to_owned(),
                package
                    .get("source")
                    .map(|value| {
                        value
                            .as_str()
                            .map(String::from)
                            .ok_or("invalid lock source")
                    })
                    .transpose()?,
                package
                    .get("checksum")
                    .map(|value| {
                        value
                            .as_str()
                            .map(String::from)
                            .ok_or("invalid lock checksum")
                    })
                    .transpose()?,
            );
            if !result.insert(identity) {
                return Err("duplicate consumer lock identity".into());
            }
        }
        Ok(result)
    }
    let seed = identities(seed)?;
    for identity in identities(resolved)? {
        if identity
            == (
                "imap-contract-consumer-075".into(),
                "0.0.0".into(),
                None,
                None,
            )
        {
            continue;
        }
        if !seed.contains(&identity) {
            return Err("external consumer resolved an unpinned dependency".into());
        }
    }
    Ok(())
}

/// Build commands run under the enclosing CI job deadline. Runtime probes have separate managed
/// process envelopes. No command or executable can be selected by a retained receipt.
pub fn prepare_tools(
    root: &Path,
) -> Result<Vec<crate::imap_package_reproduction_075::JavaPackageProof>, Box<dyn Error>> {
    check_contract(root)?;
    use crate::imap_build_process_075::{BuildController, BuildStage};
    let mut builds = BuildController::new(root)?;
    // Cargo's integration-test build provides CARGO_BIN_EXE_hydracache-server for
    // hc2_daemon_process; avoid a separate build with a different dev-feature union.
    command(
        &mut builds,
        root,
        BuildStage::ServerTests,
        "cargo",
        &[
            "test",
            "-p",
            "hydracache-server",
            "--test",
            "imap_dispatch_characterization_075",
            "--test",
            "hc2_daemon_process",
            "--locked",
        ],
    )?;
    rust_consumer(root, &mut builds)?;
    let maven = if cfg!(windows) { "mvn.cmd" } else { "mvn" };
    command(
        &mut builds,
        root,
        BuildStage::JavaContract,
        maven,
        &["-B", "-f", "tests/java-imap-benchmark/pom.xml", "install"],
    )?;
    command(
        &mut builds,
        root,
        BuildStage::NativeJava,
        maven,
        &[
            "-B",
            "-f",
            "tests/java-imap-hazelcast-075/pom.xml",
            "verify",
        ],
    )?;
    command(
        &mut builds,
        root,
        BuildStage::NativeDependencyJar,
        maven,
        &[
            "-B",
            "-f",
            "tests/java-imap-hazelcast-075/pom.xml",
            "org.apache.maven.plugins:maven-dependency-plugin:3.8.1:copy-dependencies",
            "-DincludeGroupIds=com.hazelcast",
            "-DincludeScope=runtime",
            "-DoutputDirectory=target/dependencies",
        ],
    )?;
    command(
        &mut builds,
        root,
        BuildStage::NativeDependencyPom,
        maven,
        &[
            "-B",
            "-f",
            "tests/java-imap-hazelcast-075/pom.xml",
            "org.apache.maven.plugins:maven-dependency-plugin:3.8.1:copy",
            "-Dartifact=com.hazelcast:hazelcast:5.5.0:pom",
            "-DoutputDirectory=target/dependencies",
        ],
    )?;
    crate::imap_package_reproduction_075::reproduce_with_controller(root, &mut builds)
}

fn binary(root: &Path, name: &str) -> PathBuf {
    root.join("target/debug").join(if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.into()
    })
}
fn java_probe(root: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let classpath = std::env::join_paths([
        root.join(JAVA_CONTRACT),
        root.join(JAVA_ADAPTER),
        root.join(HAZELCAST),
    ])?;
    let mut invocation = Command::new(crate::imap_api_differential::java_executable());
    invocation
        .arg("-Xmx128m")
        .arg(format!(
            "-Djdk.net.unixdomain.tmpdir={}",
            root.join(JAVA_MODULE).join("target").display()
        ))
        .arg("-cp")
        .arg(classpath)
        .arg("io.hydracache.imap.hazelcast.HazelcastProbeMain075");
    Ok(crate::imap_proof_process_075::execute(
        invocation,
        &[],
        crate::imap_proof_process_075::ProcessBounds {
            timeout: Duration::from_secs(40),
            stdout_bytes: 65536,
            stderr_bytes: 65536,
            ..Default::default()
        },
    )?
    .stdout)
}
fn rust_probe(root: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(crate::imap_proof_process_075::execute(
        Command::new(binary(root, "imap-contract-consumer-075")),
        &[],
        Default::default(),
    )?
    .stdout)
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JavaStep {
    pub operation: String,
    pub kind: String,
    pub value: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JavaDiagnostics {
    pub scope: String,
    pub operation: String,
    pub warmup_operations: usize,
    pub samples_ns: Vec<u64>,
    pub heap_used_bytes: u64,
    pub heap_committed_bytes: u64,
    pub non_heap_used_bytes: u64,
    pub threads: u64,
    pub gc_collections: u64,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JavaProbe {
    pub schema: String,
    pub profile: String,
    pub artifact_sha256: String,
    pub steps: Vec<JavaStep>,
    pub native_events: Vec<String>,
    pub snapshot_digest: String,
    pub typed_port_kind: String,
    pub typed_port_value: String,
    pub operation_count: usize,
    pub diagnostics: JavaDiagnostics,
}
pub fn validate_java_probe(probe: &JavaProbe) -> Result<(), Box<dyn Error>> {
    let diagnostic = &probe.diagnostics;
    if diagnostic.scope != "single_jvm_member_client_diagnostic"
        || diagnostic.operation != "native_get_absent"
        || diagnostic.warmup_operations != 8
        || diagnostic.samples_ns.len() != 16
        || diagnostic.heap_used_bytes == 0
        || diagnostic.heap_used_bytes > diagnostic.heap_committed_bytes
        || diagnostic.heap_committed_bytes > 256 * 1024 * 1024
        || diagnostic.non_heap_used_bytes > 256 * 1024 * 1024
        || !(1..=512).contains(&diagnostic.threads)
    {
        return Err("Java diagnostic resource/sample shape exceeds its declared profile".into());
    }
    let expected = [
        ("absent", "absent", None),
        ("put", "inserted", None),
        ("present", "present", Some("0080ff")),
        ("mismatch", "mismatch", Some("0080ff")),
        ("replace", "replaced", Some("0080ff")),
        ("remove", "removed", Some("09")),
    ];
    let rows = expected
        .into_iter()
        .map(|(operation, kind, value)| JavaStep {
            operation: operation.into(),
            kind: kind.into(),
            value: value.map(String::from),
        })
        .collect::<Vec<_>>();
    if probe.schema != "imap075-packaged-hazelcast-probe-v1"
        || probe.profile != "exclusive_eternal_binary_no_tls_no_backups"
        || probe.artifact_sha256 != HAZELCAST_SHA256
        || probe.steps != rows
        || probe.native_events != ["added", "updated", "removed"]
        || probe.snapshot_digest != digest(&[])
        || probe.typed_port_kind != "inserted"
        || probe.typed_port_value != "0080ff"
        || probe.operation_count != 40
    {
        return Err(
            "actual Java/Hazelcast observations differ from the fixed independent expectation"
                .into(),
        );
    }
    Ok(())
}
fn validate_rust_probe(bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let value: Value = serde_json::from_slice(bytes)?;
    if value
        != json!({"schema":"imap075-external-rust-consumer-v1","kind":"inserted","value":"0080ff","operation_count":40})
    {
        return Err("external Rust observation differs".into());
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreparationRuntime {
    pub rust_fingerprint: String,
    pub java_fingerprint: String,
    pub java_launcher_sha256: String,
}
fn runtime() -> Result<PreparationRuntime, Box<dyn Error>> {
    fn fingerprint(mut command: Command) -> Result<String, Box<dyn Error>> {
        command.arg("--version");
        let output = crate::imap_proof_process_075::execute(
            command,
            &[],
            crate::imap_proof_process_075::ProcessBounds {
                stdout_bytes: 4096,
                stderr_bytes: 4096,
                ..Default::default()
            },
        )?;
        Ok(String::from_utf8(output.stdout)?.trim().to_owned())
    }
    let java = crate::imap_api_differential::java_executable();
    if fs::metadata(&java)?.len() > 64 * 1024 * 1024 {
        return Err("Java launcher byte bound".into());
    }
    let mut java_version = Command::new(&java);
    // Version probes start a JVM too. Ergonomic heap sizing can exceed the
    // managed process envelope before the launcher prints its fingerprint.
    java_version.args(["-Xms16m", "-Xmx64m"]);
    Ok(PreparationRuntime {
        rust_fingerprint: fingerprint(Command::new("rustc"))?,
        java_fingerprint: fingerprint(java_version)?,
        java_launcher_sha256: daemon::binary_digest(&java)?,
    })
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreparationReceipt {
    pub schema: String,
    pub release: String,
    pub scope: String,
    pub source_sha: String,
    pub production_enabled: bool,
    pub tooling: Artifact,
    pub runtime: PreparationRuntime,
    pub reproduced_packages: Vec<crate::imap_package_reproduction_075::JavaPackageProof>,
    pub inputs: Vec<Artifact>,
    pub daemon: Artifact,
    pub storage: Artifact,
    pub java: Artifact,
    pub rust: Artifact,
    pub workspace_metadata: Artifact,
    pub blocked_until_074: Vec<String>,
}

fn inputs(root: &Path) -> Result<Vec<Artifact>, Box<dyn Error>> {
    let mut result = Vec::new();
    for path in [
        "Cargo.lock",
        CONTRACT,
        "tests/java-imap-benchmark/pom.xml",
        "tests/java-imap-hazelcast-075/pom.xml",
        "tests/rust-imap-contract-consumer-075/src/main.rs",
        JAVA_CONTRACT,
        JAVA_ADAPTER,
        HAZELCAST,
        HAZELCAST_POM,
    ] {
        result.push(artifact(
            root,
            &root.join(path),
            if path == HAZELCAST {
                32 * 1024 * 1024
            } else {
                MAX_ARTIFACT_BYTES
            },
        )?);
    }
    if result[7].sha256 != HAZELCAST_SHA256
        || result[8].sha256 != "b84a92ffc6798bcf0e94f38df11c4445e2a9a4f6fd31082a057b179ee404db23"
    {
        return Err("Hazelcast GA dependency artifact/POM differs from its pin".into());
    }
    result.push(artifact(
        root,
        &binary(root, "hydracache-server"),
        512 * 1024 * 1024,
    )?);
    result.push(artifact(
        root,
        &binary(root, "imap-contract-consumer-075"),
        512 * 1024 * 1024,
    )?);
    Ok(result)
}

fn new_output(root: &Path, output: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let relative = if output.is_absolute() {
        output.strip_prefix(root)?
    } else {
        output
    };
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
        || relative.components().next() != Some(Component::Normal(std::ffi::OsStr::new("target")))
        || relative.components().count() < 2
    {
        return Err("preparation output requires a new normal directory under target/".into());
    }
    let mut existing = root.to_path_buf();
    for component in relative.components() {
        existing.push(component);
        if existing.exists() && fs::symlink_metadata(&existing)?.file_type().is_symlink() {
            return Err("symlinked preparation output refused".into());
        }
    }
    let output = root.join(relative);
    let parent = output.parent().ok_or("missing preparation parent")?;
    fs::create_dir_all(parent)?;
    if !parent.canonicalize()?.starts_with(root.canonicalize()?) {
        return Err("preparation output escapes root".into());
    }
    fs::create_dir(&output)?;
    Ok(output)
}
fn retain(
    root: &Path,
    directory: &Path,
    name: &str,
    bytes: &[u8],
) -> Result<Artifact, Box<dyn Error>> {
    if bytes.len() > MAX_ARTIFACT_BYTES {
        return Err("preparation artifact byte bound".into());
    }
    let path = directory.join(name);
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    output.write_all(bytes)?;
    output.sync_all()?;
    artifact(root, &path, MAX_ARTIFACT_BYTES)
}

pub fn produce(root: &Path, output: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let source = crate::imap_ci_075::source_state(root)?;
    check_contract(root)?;
    let directory = new_output(root, output)?;
    // Building is part of the producer; archived booleans cannot stand in for test execution.
    let reproduced_packages = prepare_tools(root)?;
    let before = inputs(root)?;
    let tooling = artifact(root, &std::env::current_exe()?, 512 * 1024 * 1024)?;
    let observed_runtime = runtime()?;
    let scratch = root.join("target/imap-075");
    fs::create_dir_all(&scratch)?;
    let observed_daemon = daemon::probe(&binary(root, "hydracache-server"), &scratch)?;
    let observed_storage = crate::imap_storage_probe_075::characterize(&scratch)?;
    let observed_java = java_probe(root)?;
    let parsed_java: JavaProbe = serde_json::from_slice(&observed_java)?;
    validate_java_probe(&parsed_java)?;
    let observed_rust = rust_probe(root)?;
    validate_rust_probe(&observed_rust)?;
    let workspace_metadata = metadata(root)?;
    let receipt = PreparationReceipt {
        schema: "hydracache.imap.pre074-preparation.v1".into(),
        release: "0.75".into(),
        scope: "pre074_preparation".into(),
        source_sha: source.clone(),
        production_enabled: false,
        tooling,
        runtime: observed_runtime,
        reproduced_packages,
        inputs: before.clone(),
        daemon: retain(
            root,
            &directory,
            "daemon.json",
            &serde_json::to_vec(&observed_daemon)?,
        )?,
        storage: retain(
            root,
            &directory,
            "storage.json",
            &serde_json::to_vec(&observed_storage)?,
        )?,
        java: retain(root, &directory, "java.json", &observed_java)?,
        rust: retain(root, &directory, "rust.json", &observed_rust)?,
        workspace_metadata: retain(
            root,
            &directory,
            "workspace-metadata.json",
            &serde_json::to_vec(&workspace_metadata)?,
        )?,
        blocked_until_074: BLOCKERS.iter().map(|s| (*s).into()).collect(),
    };
    if crate::imap_ci_075::source_state(root)? != source
        || inputs(root)? != before
        || runtime()? != receipt.runtime
    {
        return Err("preparation source, artifacts or runtime changed during execution".into());
    }
    let path = directory.join("receipt.json");
    retain(
        root,
        &directory,
        "receipt.json",
        &serde_json::to_vec_pretty(&receipt)?,
    )?;
    Ok(path)
}

fn read_artifact(root: &Path, reference: &Artifact) -> Result<Vec<u8>, Box<dyn Error>> {
    let bytes = read_relative(root, &reference.path, MAX_ARTIFACT_BYTES)?;
    if bytes.len() as u64 != reference.bytes || digest(&bytes) != reference.sha256 {
        return Err("preparation artifact checksum/size mismatch".into());
    }
    Ok(bytes)
}
pub fn validate_receipt_header(
    receipt: &PreparationReceipt,
    source: &str,
) -> Result<(), Box<dyn Error>> {
    if receipt.schema != "hydracache.imap.pre074-preparation.v1"
        || receipt.release != "0.75"
        || receipt.scope != "pre074_preparation"
        || receipt.source_sha != source
        || !crate::imap_qualification_075::hex(source, 40)
        || receipt.production_enabled
        || receipt.blocked_until_074 != BLOCKERS
        || receipt.inputs.len() != 11
        || !receipt.runtime.rust_fingerprint.starts_with("rustc ")
        || receipt.runtime.rust_fingerprint.len() > 4096
        || receipt.runtime.java_fingerprint.is_empty()
        || receipt.runtime.java_fingerprint.len() > 4096
        || !crate::imap_qualification_075::hex(&receipt.runtime.java_launcher_sha256, 64)
        || receipt.reproduced_packages.len() != 2
        || receipt
            .reproduced_packages
            .iter()
            .zip(["java-imap-benchmark", "java-imap-hazelcast-075"])
            .any(|(proof, component)| {
                proof.component != component
                    || !crate::imap_qualification_075::hex(&proof.sha256, 64)
            })
    {
        return Err("preparation receipt identity/source/scope/blockers differ".into());
    }
    Ok(())
}

pub fn verify(root: &Path, path: &Path) -> Result<PreparationReceipt, Box<dyn Error>> {
    let relative = if path.is_absolute() {
        path.strip_prefix(root)?
    } else {
        path
    };
    let relative = relative.to_string_lossy().replace('\\', "/");
    let bytes = read_relative(root, &relative, 256 * 1024)?;
    let receipt: PreparationReceipt = serde_json::from_slice(&bytes)?;
    let source = crate::imap_ci_075::source_state(root)?;
    validate_receipt_header(&receipt, &source)?;
    check_contract(root)?;
    if receipt.inputs != inputs(root)?
        || receipt.tooling != artifact(root, &std::env::current_exe()?, 512 * 1024 * 1024)?
        || runtime()? != receipt.runtime
    {
        return Err("preparation current trusted artifacts differ".into());
    }
    for (proof, input) in receipt
        .reproduced_packages
        .iter()
        .zip(&receipt.inputs[5..7])
    {
        if proof.sha256 != input.sha256 {
            return Err("package reproduction is not bound to the current package".into());
        }
    }
    let directory = Path::new(&relative)
        .parent()
        .ok_or("missing preparation receipt parent")?;
    let references = [
        (&receipt.daemon, "daemon.json"),
        (&receipt.storage, "storage.json"),
        (&receipt.java, "java.json"),
        (&receipt.rust, "rust.json"),
        (&receipt.workspace_metadata, "workspace-metadata.json"),
    ];
    for (reference, name) in references {
        if reference.path != directory.join(name).to_string_lossy().replace('\\', "/") {
            return Err("preparation artifacts are missing, aliased or foreign".into());
        }
    }
    let observed_daemon: DaemonProbe =
        serde_json::from_slice(&read_artifact(root, &receipt.daemon)?)?;
    let observed_storage: crate::imap_storage_probe_075::StorageProbe =
        serde_json::from_slice(&read_artifact(root, &receipt.storage)?)?;
    let observed_java: JavaProbe = serde_json::from_slice(&read_artifact(root, &receipt.java)?)?;
    validate_java_probe(&observed_java)?;
    let observed_rust = read_artifact(root, &receipt.rust)?;
    validate_rust_probe(&observed_rust)?;
    let observed_metadata: Value =
        serde_json::from_slice(&read_artifact(root, &receipt.workspace_metadata)?)?;
    validate_production_dependencies(&observed_metadata)?;
    let scratch = root.join("target/imap-075");
    fs::create_dir_all(&scratch)?;
    let fresh = daemon::probe(&binary(root, "hydracache-server"), &scratch)?;
    if observed_daemon.scope != fresh.scope
        || observed_daemon.binary_sha256 != fresh.binary_sha256
        || observed_daemon.config_sha256 != fresh.config_sha256
        || observed_daemon.cluster_backend != "disabled_blocked_by_074"
        || !observed_daemon.root_reaped
        || observed_daemon.remaining_live_members != 0
        || !observed_daemon.released_listeners
        || observed_daemon.containment != fresh.containment
        || observed_daemon.exchanges.len() != fresh.exchanges.len()
    {
        return Err("daemon observations or lifecycle are incomplete/foreign".into());
    }
    for (index, (saved, actual)) in observed_daemon
        .exchanges
        .iter()
        .zip(&fresh.exchanges)
        .enumerate()
    {
        if saved.scenario != actual.scenario || saved.request != actual.request {
            return Err("daemon scenario/request changed".into());
        }
        daemon::verify_exchange(index, &saved.response)?;
    }
    let mut fresh_java: JavaProbe = serde_json::from_slice(&java_probe(root)?)?;
    validate_java_probe(&fresh_java)?;
    // Wall-clock/GC/heap samples are diagnostics, not deterministic correctness observations.
    fresh_java.diagnostics = observed_java.diagnostics.clone();
    if fresh_java != observed_java
        || rust_probe(root)? != observed_rust
        || crate::imap_storage_probe_075::characterize(&scratch)? != observed_storage
        || metadata(root)? != observed_metadata
        || crate::imap_package_reproduction_075::reproduce(root)? != receipt.reproduced_packages
    {
        return Err("preparation replay differs from retained raw observations".into());
    }
    if crate::imap_ci_075::source_state(root)? != source
        || receipt.inputs != inputs(root)?
        || runtime()? != receipt.runtime
    {
        return Err("preparation source/artifacts changed during replay".into());
    }
    Ok(receipt)
}

pub fn run(args: Vec<String>, mode: &str) -> Result<(), Box<dyn Error>> {
    let root = crate::doc_check::find_repo_root()?;
    match (mode, args.as_slice()) {
        ("tools", [release, version]) if release == "--release" && version == "0.75" => {
            prepare_tools(&root)?;
        }
        ("produce", [release, version, flag, path])
            if release == "--release" && version == "0.75" && flag == "--output" =>
        {
            let path = produce(&root, Path::new(path))?;
            verify(&root, &path)?;
            println!(
                "pre074 preparation receipt: {} (production disabled)",
                path.display()
            );
        }
        ("verify", [release, version, flag, path])
            if release == "--release" && version == "0.75" && flag == "--receipt" =>
        {
            verify(&root, Path::new(path))?;
        }
        _ => {
            return Err(
                "requires --release 0.75 and the command-specific --output/--receipt option".into(),
            )
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_fingerprint_launches_real_java_within_the_managed_memory_envelope() {
        let observed = runtime().unwrap();
        assert!(observed.rust_fingerprint.starts_with("rustc "));
        assert!(
            observed.java_fingerprint.starts_with("java ")
                || observed.java_fingerprint.starts_with("openjdk ")
        );
        assert!(crate::imap_qualification_075::hex(
            &observed.java_launcher_sha256,
            64
        ));
    }

    #[test]
    fn artifact_writes_are_bounded_non_overwriting_and_tamper_evident() {
        let root = tempfile::tempdir().unwrap();
        for path in ["outside", "target", "target/../escape"] {
            assert!(new_output(root.path(), Path::new(path)).is_err());
        }
        let dir = new_output(root.path(), Path::new("target/new/evidence")).unwrap();
        assert!(new_output(root.path(), Path::new("target/new/evidence")).is_err());
        let reference = retain(root.path(), &dir, "raw.json", b"original").unwrap();
        assert_eq!(read_artifact(root.path(), &reference).unwrap(), b"original");
        assert!(retain(root.path(), &dir, "raw.json", b"replacement").is_err());
        fs::write(root.path().join(&reference.path), b"changed").unwrap();
        assert!(read_artifact(root.path(), &reference).is_err());
        assert!(retain(
            root.path(),
            &dir,
            "oversized",
            &vec![0; MAX_ARTIFACT_BYTES + 1]
        )
        .is_err());
        assert!(validate_rust_probe(b"{\"passed\":true}").is_err());
    }
}
