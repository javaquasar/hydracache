use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::{Child, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::Deserialize;

const BORROWED_MANIFESTS: [&str; 2] = [
    "docs/integrations/hazelcast_borrowed_suite.json",
    "docs/integrations/cache_semantics_borrowed.json",
];
const LEGACY_MANIFEST: &str = "docs/testing/compat/legacy-clients.toml";
const ALLOWED_OUTCOMES: [&str; 4] = [
    "pass",
    "divergence-documented",
    "unsupported-documented",
    "skipped",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BorrowedManifest {
    schema_version: u32,
    suite: String,
    source: SourcePin,
    rows: Vec<BorrowedRow>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourcePin {
    project: String,
    repository: String,
    version: String,
    commit: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BorrowedRow {
    id: String,
    source_test: String,
    expectation: String,
    expected: String,
    hydracache_test: String,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    proofs: Vec<BorrowedProof>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BorrowedProof {
    layer: String,
    source: String,
    test: String,
    language: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyManifest {
    schema_version: u32,
    suite: String,
    clients: Vec<LegacyClient>,
    hc2_prerequisite: Hc2Prerequisite,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyClient {
    id: String,
    tag: String,
    commit: String,
    protocol: String,
    consumer_fixture: String,
    surface: Vec<String>,
    expected: String,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Hc2Prerequisite {
    manifest: String,
    command: String,
    required_rows: usize,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.as_slice() != ["--structural"] {
        return Err("usage: migration-conformance-check --structural".into());
    }
    let root = repository_root();
    validate_at_root(&root)?;
    println!("migration conformance structural check: PASS");
    Ok(())
}

pub fn run_borrowed(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.as_slice() != ["--suite", "hazelcast"] {
        return Err("usage: borrowed-suite-check --suite hazelcast".into());
    }
    let root = repository_root();
    validate_at_root(&root)?;
    if std::env::var("HYDRACACHE_RUN_JVM_COMPAT").as_deref() != Ok("1") {
        return Err(
            "SKIP-LOUD: set HYDRACACHE_RUN_JVM_COMPAT=1 to execute the JVM borrowed suite".into(),
        );
    }
    for (package, binary) in [
        ("hydracache-client-plane-spike", "hc2_java_interop_server"),
        ("hydracache-server", "hydracache-server"),
    ] {
        let status = Command::new("cargo")
            .current_dir(&root)
            .args(["build", "-p", package, "--bin", binary, "--locked"])
            .status()?;
        if !status.success() {
            return Err(
                format!("failed to build live facade dependency {binary}: {status}").into(),
            );
        }
    }
    let executable_suffix = if cfg!(windows) { ".exe" } else { "" };
    let target_dir = cargo_target_dir(&root);
    let fixture = target_dir
        .join("debug")
        .join(format!("hc2_java_interop_server{executable_suffix}"));
    let daemon = target_dir
        .join("debug")
        .join(format!("hydracache-server{executable_suffix}"));
    if !fixture.is_file() || !daemon.is_file() {
        return Err("live facade binaries were not produced at their deterministic paths".into());
    }
    let lease_status = Command::new("cargo")
        .current_dir(&root)
        .args([
            "test",
            "-p",
            "hydracache-server",
            "--test",
            "lock_endpoint",
            "lease_renew_extends_then_expiry_frees",
            "--locked",
        ])
        .status()?;
    if !lease_status.success() {
        return Err(format!("production lock lease contract failed with {lease_status}").into());
    }
    let maven = if cfg!(windows) { "mvn.cmd" } else { "mvn" };
    let status = Command::new(maven)
        .current_dir(&root)
        .env("HC2_JAVA_INTEROP_SERVER", &fixture)
        .env("HC2_JAVA_DAEMON", &daemon)
        .args([
            "-q",
            "-f",
            "sdks/java/pom.xml",
            "-pl",
            "hydracache-hazelcast-facade",
            "-Dtest=BorrowedHazelcastExpectationsTest,BorrowedHazelcastLiveInteropTest",
            "test",
        ])
        .status()?;
    if !status.success() {
        return Err(format!("borrowed Hazelcast expectation suite failed with {status}").into());
    }
    retain_report(
        &root,
        "sdks/java/hydracache-hazelcast-facade/target/surefire-reports/TEST-io.hydracache.hazelcast.BorrowedHazelcastExpectationsTest.xml",
        "target/test-evidence/0.69/hazelcast-borrowed-expectations.xml",
    )?;
    retain_report(
        &root,
        "sdks/java/hydracache-hazelcast-facade/target/surefire-reports/TEST-io.hydracache.hazelcast.BorrowedHazelcastLiveInteropTest.xml",
        "target/test-evidence/0.69/hazelcast-live-interop.xml",
    )?;
    let session_status = Command::new(maven)
        .current_dir(&root)
        .env("HC2_JAVA_INTEROP_SERVER", &fixture)
        .args([
            "-q",
            "-f",
            "sdks/java/pom.xml",
            "-pl",
            "hydracache-client-hc2",
            "-Dtest=GrpcHydraCacheClientInteropTest#javaRecoveryReplacesADeadRustProcessRepairsSubscriptionAndLosesSession",
            "test",
        ])
        .status()?;
    if !session_status.success() {
        return Err(format!("HC/2 session-loss contract failed with {session_status}").into());
    }
    retain_report(
        &root,
        "sdks/java/hydracache-client-hc2/target/surefire-reports/TEST-io.hydracache.client.hc2.GrpcHydraCacheClientInteropTest.xml",
        "target/test-evidence/0.69/hc2-session-loss.xml",
    )?;
    println!("borrowed Hazelcast expectation suite: PASS");
    Ok(())
}

fn retain_report(root: &Path, source: &str, destination: &str) -> Result<(), Box<dyn Error>> {
    let source = root.join(source);
    if !source.is_file() {
        return Err(format!("expected test report is missing: {}", source.display()).into());
    }
    let destination = root.join(destination);
    let parent = destination
        .parent()
        .ok_or("retained test report has no parent directory")?;
    fs::create_dir_all(parent)?;
    fs::copy(&source, &destination)?;
    Ok(())
}

pub fn run_legacy(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.as_slice() != ["--matrix", "hc1"] {
        return Err("usage: legacy-client-check --matrix hc1".into());
    }
    let root = repository_root();
    validate_at_root(&root)?;
    let legacy: LegacyManifest = toml::from_str(&fs::read_to_string(root.join(LEGACY_MANIFEST))?)?;
    validate_git_tags(&root, &legacy)?;
    crate::client_plane_compat::run(vec!["--manifest-only".to_owned()])?;
    if std::env::var("HYDRACACHE_RUN_LEGACY_CLIENTS").as_deref() != Ok("1") {
        return Err(
            "SKIP-LOUD: set HYDRACACHE_RUN_LEGACY_CLIENTS=1 to build and run HC/1 tag consumers"
                .into(),
        );
    }

    run_command(
        Command::new("cargo").current_dir(&root).args([
            "build",
            "-p",
            "hydracache-server",
            "--bin",
            "hydracache-server",
            "--locked",
        ]),
        "build current server",
    )?;
    let address = reserve_loopback_address()?;
    let server_exe = cargo_target_dir(&root)
        .join("debug")
        .join(if cfg!(windows) {
            "hydracache-server.exe"
        } else {
            "hydracache-server"
        });
    let child = Command::new(server_exe)
        .current_dir(&root)
        .env("HYDRACACHE_LISTEN_ADDR", &address)
        .env("HYDRACACHE_CLIENT_API_ENABLED", "true")
        .env("HYDRACACHE_ADMIN_API_ENABLED", "false")
        .env("HYDRACACHE_DRAIN_TIMEOUT_MS", "1000")
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()?;
    let mut server = ChildGuard(child);
    wait_for_listener(&address, &mut server.0)?;

    let mut executed = BTreeSet::new();
    for client in &legacy.clients {
        run_legacy_consumer(&root, client, &address)?;
        executed.insert(client.id.clone());
        println!(
            "legacy client {} ({}) against current server: PASS",
            client.tag, client.commit
        );
    }
    validate_legacy_execution(
        &legacy
            .clients
            .iter()
            .map(|client| client.id.clone())
            .collect::<Vec<_>>(),
        &executed,
    )?;
    Ok(())
}

/// Reject a legacy matrix that reports green without executing every manifest row.
pub fn validate_legacy_execution(
    expected: &[String],
    executed: &BTreeSet<String>,
) -> Result<(), Box<dyn Error>> {
    let expected = expected.iter().cloned().collect::<BTreeSet<_>>();
    if expected != *executed {
        return Err(format!(
            "HC-CANARY-RED:W3 legacy execution set mismatch: expected={expected:?}, executed={executed:?}"
        )
        .into());
    }
    Ok(())
}

pub fn run_postgres(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mode = match args.as_slice() {
        [flag, mode]
            if flag == "--mode" && matches!(mode.as_str(), "happy" | "canary" | "soak") =>
        {
            mode.as_str()
        }
        _ => return Err("usage: postgres-conformance-check --mode <happy|canary|soak>".into()),
    };
    if std::env::var("HYDRACACHE_TEST_POSTGRES_URL").is_err() {
        return Err("SKIP-LOUD: HYDRACACHE_TEST_POSTGRES_URL is required".into());
    }
    let root = repository_root();
    let evidence = root.join("target/test-evidence/0.69");
    fs::create_dir_all(&evidence)?;
    let series = std::env::var("HYDRACACHE_POSTGRES_SERIES")
        .map_err(|_| "SKIP-LOUD: HYDRACACHE_POSTGRES_SERIES is required")?;
    if !matches!(series.as_str(), "16" | "18") {
        return Err("HYDRACACHE_POSTGRES_SERIES must be 16 or 18".into());
    }
    let image = std::env::var("HYDRACACHE_POSTGRES_IMAGE_ID")
        .map_err(|_| "SKIP-LOUD: HYDRACACHE_POSTGRES_IMAGE_ID is required")?;
    let (test, log) = if mode == "happy" {
        fs::write(
            evidence.join(format!("postgres-{series}-image.txt")),
            format!("{image}\n"),
        )?;
        fs::write(
            evidence.join(format!("postgres-{series}-seeds.txt")),
            "0x69_2026 0x69_2027 0x69_2028\n",
        )?;
        let test = if series == "18" {
            "postgres_18_cached_reads_match_direct_queries_through_the_real_outbox"
        } else {
            "postgres_cached_reads_match_direct_queries_through_the_real_outbox"
        };
        (
            test,
            evidence.join(format!("postgres-{series}-differential.log")),
        )
    } else if mode == "canary" {
        (
            "canary_postgres_differential_rejects_a_dropped_invalidation",
            evidence.join(format!("postgres-{series}-canary.log")),
        )
    } else {
        fs::write(
            evidence.join(format!("postgres-{series}-soak-seeds.txt")),
            (0..24)
                .map(|index| format!("{:#x}\n", 0x69_5000_u64 + index))
                .collect::<String>(),
        )?;
        (
            "postgres_commit_scoped_wait_soak_stays_within_budget",
            evidence.join(format!("postgres-{series}-soak.log")),
        )
    };
    let mut command = Command::new("cargo");
    command.current_dir(&root).args([
        "test",
        "-p",
        "hydracache-db",
        "--features",
        "sqlx-outbox",
        "--test",
        "cached_vs_direct_postgres",
        "--locked",
        test,
        "--",
        "--ignored",
        "--exact",
        "--nocapture",
    ]);
    if mode == "canary" {
        command.env("HYDRACACHE_CANARY_DEFECT", "W4_PG_DROP");
    }
    let output = command.output()?;
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    fs::write(&log, combined.as_bytes())?;
    print!("{combined}");
    if mode == "happy" && !output.status.success() {
        return Err(format!("PostgreSQL differential failed with {}", output.status).into());
    }
    if mode == "canary" && !postgres_canary_is_expected_red(output.status.success(), &combined) {
        return Err(
            "PostgreSQL dropped-invalidation canary did not fail with HC-CANARY-RED:W4-PG".into(),
        );
    }
    if mode == "soak" && !output.status.success() {
        return Err(format!("PostgreSQL bounded soak failed with {}", output.status).into());
    }
    Ok(())
}

fn postgres_canary_is_expected_red(success: bool, output: &str) -> bool {
    !success && output.contains("HC-CANARY-RED:W4-PG")
}

fn cargo_target_dir(root: &Path) -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                root.join(path)
            }
        })
        .unwrap_or_else(|| root.join("target"))
}

fn validate_git_tags(root: &Path, manifest: &LegacyManifest) -> Result<(), Box<dyn Error>> {
    for client in &manifest.clients {
        let output = Command::new("git")
            .current_dir(root)
            .args(["rev-list", "-n", "1", &client.tag])
            .output()?;
        require(
            output.status.success(),
            LEGACY_MANIFEST,
            &format!("cannot resolve tag {}", client.tag),
        )?;
        let actual = String::from_utf8(output.stdout)?.trim().to_owned();
        require(
            actual == client.commit,
            LEGACY_MANIFEST,
            &format!(
                "tag {} resolved to {actual}, expected {}",
                client.tag, client.commit
            ),
        )?;
    }
    Ok(())
}

fn run_legacy_consumer(
    root: &Path,
    client: &LegacyClient,
    address: &str,
) -> Result<(), Box<dyn Error>> {
    // Keep nested tag workspaces outside the repository. Cargo/rustfmt discover nested
    // manifests under the repository even when they are below target/, which makes a
    // post-conformance `cargo fmt --all` traverse the historical workspace on Windows.
    let directory = std::env::temp_dir()
        .join("hydracache-legacy-client-matrix")
        .join(&client.id);
    fs::create_dir_all(directory.join("src"))?;
    let repository_url = format!("file:///{}", root.to_string_lossy().replace('\\', "/"));
    let package_id = client.id.replace('.', "-");
    let manifest = format!(
        "[package]\nname = \"legacy-consumer-{}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n\n[dependencies]\nbytes = \"1\"\ntokio = {{ version = \"1\", features = [\"macros\", \"rt-multi-thread\"] }}\nhydracache-client = {{ git = \"{}\", rev = \"{}\" }}\nhydracache-client-protocol = {{ git = \"{}\", rev = \"{}\" }}\n",
        package_id, repository_url, client.commit, repository_url, client.commit
    );
    fs::write(directory.join("Cargo.toml"), manifest)?;
    fs::write(directory.join("src/main.rs"), LEGACY_CONSUMER_SOURCE)?;
    run_command(
        Command::new("cargo")
            .current_dir(&directory)
            .env("HYDRACACHE_LEGACY_SERVER", format!("http://{address}"))
            .env("HYDRACACHE_LEGACY_SURFACE", client.surface.join(","))
            .args(["run", "--quiet"]),
        &format!("run legacy consumer {}", client.tag),
    )
}

const LEGACY_CONSUMER_SOURCE: &str = r#"
use bytes::Bytes;
use hydracache_client::{ClientIdentity, HttpClientTransport, HydraClient, HydraClientConfig};
use hydracache_client_protocol::{Namespace, StructuredKey, PROTOCOL_VERSION};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let base = std::env::var("HYDRACACHE_LEGACY_SERVER").expect("server URL");
    let identity = ClientIdentity::new("legacy-069", "compat").unwrap();
    let client = HydraClient::connect(
        HttpClientTransport::new(base),
        HydraClientConfig::new(identity),
    ).await.expect("legacy handshake");
    assert!(client.negotiated_version() <= PROTOCOL_VERSION);
    let namespace = Namespace::new("legacy").unwrap();
    let key = StructuredKey::new(vec!["client".to_owned(), "069".to_owned()]).unwrap();
    client.put(namespace.clone(), key.clone(), Bytes::from_static(b"value"), None)
        .await.expect("legacy put");
    assert_eq!(client.get(namespace.clone(), key.clone()).await.expect("legacy get"),
        Some(Bytes::from_static(b"value")));
    let surface = std::env::var("HYDRACACHE_LEGACY_SURFACE").expect("surface");
    if surface.split(',').any(|operation| operation == "ttl") {
        client.put(namespace.clone(), key.clone(), Bytes::from_static(b"ttl"),
            Some(Duration::from_secs(30))).await.expect("legacy ttl put");
        assert_eq!(client.get(namespace.clone(), key.clone()).await.expect("legacy ttl get"),
            Some(Bytes::from_static(b"ttl")));
    }
    if surface.split(',').any(|operation| operation == "lock") {
        let lock_key = StructuredKey::new(vec!["lock".to_owned(), "069".to_owned()]).unwrap();
        let guard = client.try_lock(namespace.clone(), lock_key, Duration::from_secs(5))
            .await.expect("legacy lock request").expect("legacy lock acquisition");
        assert!(guard.fence() > 0);
        guard.unlock().await.expect("legacy unlock");
    }
    client.invalidate(namespace.clone(), key.clone()).await.expect("legacy invalidate");
    assert_eq!(client.get(namespace, key).await.expect("legacy absent get"), None);
}
"#;

fn run_command(command: &mut Command, description: &str) -> Result<(), Box<dyn Error>> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{description} failed with {status}").into())
    }
}

fn reserve_loopback_address() -> Result<String, Box<dyn Error>> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?.to_string();
    drop(listener);
    Ok(address)
}

fn wait_for_listener(address: &str, child: &mut Child) -> Result<(), Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait()? {
            return Err(format!("current server exited before HC/1 readiness: {status}").into());
        }
        if std::net::TcpStream::connect(address).is_ok() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err(format!("current server did not listen on {address} within 20 seconds").into())
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub fn validate_at_root(root: &Path) -> Result<(), Box<dyn Error>> {
    for relative in BORROWED_MANIFESTS {
        let path = root.join(relative);
        validate_borrowed_text_at_root(root, relative, &fs::read_to_string(&path)?)?;
    }

    let legacy_path = root.join(LEGACY_MANIFEST);
    let legacy: LegacyManifest = toml::from_str(&fs::read_to_string(&legacy_path)?)?;
    validate_legacy(root, &legacy)?;
    Ok(())
}

/// Validate one borrowed-suite document against repository-local proof sources.
pub fn validate_borrowed_text_at_root(
    root: &Path,
    path: &str,
    text: &str,
) -> Result<(), Box<dyn Error>> {
    let manifest: BorrowedManifest = serde_json::from_str(text)?;
    validate_borrowed(root, path, &manifest)
}

fn validate_borrowed(
    root: &Path,
    path: &str,
    manifest: &BorrowedManifest,
) -> Result<(), Box<dyn Error>> {
    require(
        matches!(manifest.schema_version, 1 | 2),
        path,
        "schema_version must be 1 or 2",
    )?;
    nonempty(path, "suite", &manifest.suite)?;
    nonempty(path, "source.project", &manifest.source.project)?;
    require(
        manifest.source.repository.starts_with("https://"),
        path,
        "source.repository must be an https URL",
    )?;
    nonempty(path, "source.version", &manifest.source.version)?;
    validate_commit(path, &manifest.source.commit)?;
    require(!manifest.rows.is_empty(), path, "rows must not be empty")?;

    let mut ids = BTreeSet::new();
    let mut tests = BTreeSet::new();
    for row in &manifest.rows {
        require(
            ids.insert(row.id.as_str()),
            path,
            &format!("duplicate row id {}", row.id),
        )?;
        require(
            tests.insert(row.hydracache_test.as_str()),
            path,
            &format!("duplicate hydracache_test {}", row.hydracache_test),
        )?;
        nonempty(path, "row.source_test", &row.source_test)?;
        require(
            row.source_test.contains('#'),
            path,
            &format!("row {} source_test must identify a symbol with #", row.id),
        )?;
        nonempty(path, "row.expectation", &row.expectation)?;
        nonempty(path, "row.hydracache_test", &row.hydracache_test)?;
        validate_outcome(path, &row.id, &row.expected, row.reason.as_deref())?;
        if manifest.schema_version == 2 {
            require(
                !row.proofs.is_empty(),
                path,
                &format!("row {} must declare at least one proof", row.id),
            )?;
        }
        let mut proofs = BTreeSet::new();
        for proof in &row.proofs {
            require(
                matches!(
                    proof.layer.as_str(),
                    "adapted-unit" | "live-daemon" | "server-state-machine" | "recovery-interop"
                ),
                path,
                &format!("row {} has unknown proof layer {}", row.id, proof.layer),
            )?;
            require(
                matches!(proof.language.as_str(), "rust" | "java" | "python"),
                path,
                &format!(
                    "row {} has unknown proof language {}",
                    row.id, proof.language
                ),
            )?;
            nonempty(path, "row.proof.source", &proof.source)?;
            nonempty(path, "row.proof.test", &proof.test)?;
            require(
                proofs.insert((
                    proof.layer.as_str(),
                    proof.source.as_str(),
                    proof.test.as_str(),
                )),
                path,
                &format!("row {} has a duplicate proof", row.id),
            )?;
            let proof_source = root.join(&proof.source);
            require(
                proof_source.is_file(),
                path,
                &format!("row {} proof source {} is missing", row.id, proof.source),
            )?;
            let proof_text = fs::read_to_string(&proof_source)?;
            require(
                proof_text.contains(&proof.test),
                path,
                &format!(
                    "row {} proof selector {} is absent from {}",
                    row.id, proof.test, proof.source
                ),
            )?;
        }
        if manifest.suite == "hazelcast-java-facade-adapted-expectations" && row.expected == "pass"
        {
            require(
                row.proofs.iter().any(|proof| {
                    matches!(
                        proof.layer.as_str(),
                        "live-daemon" | "server-state-machine" | "recovery-interop"
                    )
                }),
                path,
                &format!("passing Hazelcast row {} lacks a non-double proof", row.id),
            )?;
        }
    }
    Ok(())
}

fn validate_legacy(root: &Path, manifest: &LegacyManifest) -> Result<(), Box<dyn Error>> {
    require(
        manifest.schema_version == 1,
        LEGACY_MANIFEST,
        "schema_version must be 1",
    )?;
    nonempty(LEGACY_MANIFEST, "suite", &manifest.suite)?;
    require(
        !manifest.clients.is_empty(),
        LEGACY_MANIFEST,
        "clients must not be empty",
    )?;
    let mut ids = BTreeSet::new();
    let mut tags = BTreeSet::new();
    for client in &manifest.clients {
        require(
            ids.insert(client.id.as_str()),
            LEGACY_MANIFEST,
            &format!("duplicate client id {}", client.id),
        )?;
        require(
            tags.insert(client.tag.as_str()),
            LEGACY_MANIFEST,
            &format!("duplicate client tag {}", client.tag),
        )?;
        validate_commit(LEGACY_MANIFEST, &client.commit)?;
        require(
            client.tag.starts_with('v'),
            LEGACY_MANIFEST,
            &format!("{} tag must start with v", client.id),
        )?;
        require(
            client.protocol == "HC/1",
            LEGACY_MANIFEST,
            &format!("{} must remain HC/1", client.id),
        )?;
        nonempty(
            LEGACY_MANIFEST,
            "client.consumer_fixture",
            &client.consumer_fixture,
        )?;
        require(
            !client.surface.is_empty(),
            LEGACY_MANIFEST,
            &format!("{} surface is empty", client.id),
        )?;
        validate_outcome(
            LEGACY_MANIFEST,
            &client.id,
            &client.expected,
            client.reason.as_deref(),
        )?;
    }
    require(
        root.join(&manifest.hc2_prerequisite.manifest).is_file(),
        LEGACY_MANIFEST,
        "HC/2 prerequisite manifest does not exist",
    )?;
    require(
        manifest
            .hc2_prerequisite
            .command
            .contains("client-plane-compat-check --manifest-only"),
        LEGACY_MANIFEST,
        "HC/2 prerequisite must execute the retained 0.68 checker",
    )?;
    require(
        manifest.hc2_prerequisite.required_rows == 9,
        LEGACY_MANIFEST,
        "HC/2 prerequisite must retain all nine 0.68 rows",
    )?;
    Ok(())
}

fn validate_outcome(
    path: &str,
    id: &str,
    expected: &str,
    reason: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    require(
        ALLOWED_OUTCOMES.contains(&expected),
        path,
        &format!("row {id} has unknown outcome {expected}"),
    )?;
    if expected != "pass" {
        require(
            reason.is_some_and(|value| !value.trim().is_empty()),
            path,
            &format!("row {id} outcome {expected} requires a reason"),
        )?;
    }
    Ok(())
}

fn validate_commit(path: &str, commit: &str) -> Result<(), Box<dyn Error>> {
    require(
        commit.len() == 40 && commit.bytes().all(|byte| byte.is_ascii_hexdigit()),
        path,
        "source commit must be a full 40-character hexadecimal object id",
    )
}

fn nonempty(path: &str, field: &str, value: &str) -> Result<(), Box<dyn Error>> {
    require(
        !value.trim().is_empty(),
        path,
        &format!("{field} must not be empty"),
    )
}

fn require(condition: bool, path: &str, message: &str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(format!("{path}: {message}").into())
    }
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("xtask is nested under crates")
        .to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_manifests_are_structurally_valid() {
        validate_at_root(&repository_root()).expect("0.69 manifests should be valid");
    }

    #[test]
    fn non_pass_outcome_requires_a_reason() {
        let error = validate_outcome("fixture", "row", "skipped", None).unwrap_err();
        assert!(error.to_string().contains("requires a reason"));
    }

    #[test]
    fn unknown_outcome_fails_closed() {
        let error = validate_outcome("fixture", "row", "maybe", Some("fixture")).unwrap_err();
        assert!(error.to_string().contains("unknown outcome"));
    }

    #[test]
    fn postgres_canary_requires_both_failure_and_the_semantic_marker() {
        assert!(postgres_canary_is_expected_red(
            false,
            "HC-CANARY-RED:W4-PG divergence"
        ));
        assert!(!postgres_canary_is_expected_red(
            true,
            "HC-CANARY-RED:W4-PG divergence"
        ));
        assert!(!postgres_canary_is_expected_red(false, "unrelated failure"));
    }
}
