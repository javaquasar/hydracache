use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use hydracache_loadgen::profile::{PerformanceProfile, RunnerFingerprint};
use hydracache_loadgen::tiers::resp::RESP_REFERENCE_RUN_INPUTS_RELATIVE_PATH;
use hydracache_loadgen::tiers::resp_reference::{
    LOADGEN_BINARY_ID, PREBUILD_MANIFEST_RELATIVE_PATH, SERVER_BINARY_ID,
};
use xtask::perf::{
    execute_prebuild_with, sha256_file, verify_published_bundle, ObservedExternalTool,
    PrebuildCommandOutput, PrebuildHost,
};

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const FINGERPRINT: &str = "abababababababababababababababababababababababababababababababab";

fn temp_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "hydracache-perf-prebuild-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(root.join("docs/testing/perf-profiles")).unwrap();
    fs::create_dir_all(root.join("docs/testing/perf-scenarios/0.67")).unwrap();
    fs::create_dir_all(root.join("tools")).unwrap();
    fs::write(
        root.join("docs/testing/perf-profiles/reference-v1.toml"),
        include_bytes!("../../../docs/testing/perf-profiles/reference-v1.toml"),
    )
    .unwrap();
    fs::write(
        root.join("docs/testing/perf-scenarios/0.67/redis-benchmark-provenance-v1.toml"),
        include_bytes!(
            "../../../docs/testing/perf-scenarios/0.67/redis-benchmark-provenance-v1.toml"
        ),
    )
    .unwrap();
    fs::write(root.join("Cargo.lock"), b"fixture-lock-v1\n").unwrap();
    fs::write(
        root.join("tools/redis-benchmark"),
        b"fixture-redis-benchmark",
    )
    .unwrap();
    fs::canonicalize(root).unwrap()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuildMutation {
    None,
    CargoLock,
    Head,
    OmitServer,
}

#[derive(Debug)]
struct FakeHost {
    root: PathBuf,
    head: String,
    dirty: bool,
    rustc: String,
    cargo: String,
    mutation: BuildMutation,
    calls: Vec<(String, Vec<String>)>,
}

impl FakeHost {
    fn new(root: PathBuf) -> Self {
        Self {
            root,
            head: COMMIT.to_owned(),
            dirty: false,
            rustc: "rustc 1.94.0 (fixture 2026-01-01)".to_owned(),
            cargo: "cargo 1.94.0 (fixture 2026-01-01)".to_owned(),
            mutation: BuildMutation::None,
            calls: Vec::new(),
        }
    }

    fn successful(program: &str, stdout: impl Into<Vec<u8>>) -> PrebuildCommandOutput {
        let _ = program;
        PrebuildCommandOutput {
            success: true,
            stdout: stdout.into(),
            stderr: Vec::new(),
        }
    }

    fn cargo_calls(&self) -> Vec<&[String]> {
        self.calls
            .iter()
            .filter_map(|(program, args)| {
                (program == "cargo" && args.first().map(String::as_str) == Some("build"))
                    .then_some(args.as_slice())
            })
            .collect()
    }
}

impl PrebuildHost for FakeHost {
    fn run_command(
        &mut self,
        cwd: &Path,
        program: &str,
        args: &[&str],
    ) -> Result<PrebuildCommandOutput, String> {
        assert_eq!(cwd, self.root);
        self.calls.push((
            program.to_owned(),
            args.iter().map(|arg| (*arg).to_owned()).collect(),
        ));
        match (program, args) {
            ("git", ["rev-parse", "--show-toplevel"]) => Ok(Self::successful(
                program,
                format!("{}\n", self.root.display()),
            )),
            ("git", ["rev-parse", "HEAD"]) => {
                Ok(Self::successful(program, format!("{}\n", self.head)))
            }
            (
                "git",
                ["status", "--porcelain=v1", "--untracked-files=normal", "--ignore-submodules=none"],
            ) => Ok(Self::successful(
                program,
                if self.dirty { " M fixture\n" } else { "" },
            )),
            ("rustc", ["--version"]) => Ok(Self::successful(program, format!("{}\n", self.rustc))),
            ("cargo", ["--version"]) => Ok(Self::successful(program, format!("{}\n", self.cargo))),
            (
                "cargo",
                ["build", "-p", "hydracache-loadgen", "-p", "hydracache-server", "--release", "--locked"],
            ) => {
                let release = self.root.join("target/release");
                fs::create_dir_all(&release).map_err(|error| error.to_string())?;
                fs::write(release.join(LOADGEN_BINARY_ID), b"loadgen-v1")
                    .map_err(|error| error.to_string())?;
                if self.mutation != BuildMutation::OmitServer {
                    fs::write(release.join(SERVER_BINARY_ID), b"server-v1")
                        .map_err(|error| error.to_string())?;
                }
                match self.mutation {
                    BuildMutation::CargoLock => {
                        fs::write(self.root.join("Cargo.lock"), b"changed-lock\n")
                            .map_err(|error| error.to_string())?;
                    }
                    BuildMutation::Head => self.head = "f".repeat(40),
                    BuildMutation::None | BuildMutation::OmitServer => {}
                }
                Ok(Self::successful(program, Vec::new()))
            }
            _ => Err(format!("unexpected command: {program} {args:?}")),
        }
    }

    fn reference_platform_key(&self) -> String {
        "linux-x86_64".to_owned()
    }

    fn observe_runner(
        &mut self,
        profile: &PerformanceProfile,
    ) -> Result<RunnerFingerprint, String> {
        Ok(RunnerFingerprint {
            runner_class: profile.required_runner_class.clone(),
            fingerprint: FINGERPRINT.to_owned(),
            cpu_model: "fixture-cpu".to_owned(),
            logical_cores: 16,
            ram_bytes: 64 * 1024 * 1024 * 1024,
            os: "linux".to_owned(),
            kernel: "fixture-kernel".to_owned(),
            cpu_affinity: profile.required_cpu_affinity.clone(),
            cgroup_cpu_quota: profile.required_cgroup_cpu_quota.clone(),
            governor: "performance".to_owned(),
            turbo: "intel-no-turbo:1".to_owned(),
            shared_hardware: false,
            calibration_score: 0.01,
        })
    }

    fn observe_external_tool(&mut self, program: &str) -> Result<ObservedExternalTool, String> {
        assert_eq!(program, "redis-benchmark");
        let canonical_path = fs::canonicalize(self.root.join("tools/redis-benchmark"))
            .map_err(|error| error.to_string())?;
        let sha256 = sha256_file(&canonical_path).map_err(|error| error.to_string())?;
        Ok(ObservedExternalTool {
            platform_key: "linux-x86_64-gnu".to_owned(),
            canonical_path,
            sha256,
            version: "redis-benchmark 7.2.5".to_owned(),
        })
    }
}

fn build(root: &Path) -> (FakeHost, xtask::perf::PrebuildOutcome) {
    let mut host = FakeHost::new(root.to_path_buf());
    let outcome = execute_prebuild_with(root, "0.67", "reference-v1", &mut host).unwrap();
    (host, outcome)
}

#[test]
fn prebuild_publishes_atomic_two_artifact_commit_marker() {
    let root = temp_root("atomic-pair");
    let (host, outcome) = build(&root);
    assert!(outcome.manifest_path.is_file());
    assert!(outcome.run_inputs_path.is_file());
    assert_eq!(host.cargo_calls().len(), 1);

    let bundle = verify_published_bundle(&root).unwrap();
    assert_eq!(bundle.manifest_sha256, outcome.manifest_sha256);
    assert_eq!(
        bundle
            .run_inputs
            .external_tool_prebuild
            .payload
            .prebuild_manifest_sha256,
        outcome.manifest_sha256
    );
    assert_eq!(
        bundle
            .manifest
            .binaries
            .iter()
            .map(|binary| binary.id.as_str())
            .collect::<Vec<_>>(),
        [LOADGEN_BINARY_ID, SERVER_BINARY_ID]
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn prebuild_rejects_dirty_or_changed_source_without_artifacts() {
    for (label, configure) in [
        ("dirty", BuildMutation::None),
        ("lock", BuildMutation::CargoLock),
        ("head", BuildMutation::Head),
    ] {
        let root = temp_root(label);
        let mut host = FakeHost::new(root.clone());
        host.mutation = configure;
        host.dirty = label == "dirty";
        let error = execute_prebuild_with(&root, "0.67", "reference-v1", &mut host)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("clean working tree") || error.contains("changed during"),
            "unexpected error: {error}"
        );
        assert!(!root.join(PREBUILD_MANIFEST_RELATIVE_PATH).exists());
        assert!(!root.join(RESP_REFERENCE_RUN_INPUTS_RELATIVE_PATH).exists());
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn exact_toolchain_and_target_set_are_fail_closed() {
    let root = temp_root("wrong-toolchain");
    let mut host = FakeHost::new(root.clone());
    host.rustc = "rustc 1.93.0 (fixture)".to_owned();
    let error = execute_prebuild_with(&root, "0.67", "reference-v1", &mut host)
        .unwrap_err()
        .to_string();
    assert!(error.contains("toolchain") || error.contains("receipt-bound"));
    assert!(host.cargo_calls().is_empty());
    fs::remove_dir_all(root).unwrap();

    let root = temp_root("wrong-cargo");
    let mut host = FakeHost::new(root.clone());
    host.cargo = "cargo 1.93.0 (fixture)".to_owned();
    let error = execute_prebuild_with(&root, "0.67", "reference-v1", &mut host)
        .unwrap_err()
        .to_string();
    assert!(error.contains("cargo-1.93.0"));
    assert!(host.cargo_calls().is_empty());
    fs::remove_dir_all(root).unwrap();

    let root = temp_root("missing-target");
    let mut host = FakeHost::new(root.clone());
    host.mutation = BuildMutation::OmitServer;
    let error = execute_prebuild_with(&root, "0.67", "reference-v1", &mut host)
        .unwrap_err()
        .to_string();
    assert!(error.contains("hydracache-server"));
    assert!(!root.join(PREBUILD_MANIFEST_RELATIVE_PATH).exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn measurement_refuses_missing_or_mismatched_prebuild_manifest() {
    let root = temp_root("missing-manifest");
    let (_host, _outcome) = build(&root);
    let manifest_path = root.join(PREBUILD_MANIFEST_RELATIVE_PATH);
    let original = fs::read(&manifest_path).unwrap();
    fs::remove_file(&manifest_path).unwrap();
    assert!(verify_published_bundle(&root).is_err());
    fs::write(&manifest_path, &original).unwrap();

    let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
    value["runner_fingerprint"] = serde_json::json!("cd".repeat(32));
    fs::write(&manifest_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    assert!(verify_published_bundle(&root).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn prebuilt_binary_mutation_invalidates_the_prebuild_bundle() {
    let root = temp_root("binary-binding");
    let (_host, _outcome) = build(&root);
    fs::write(root.join("target/release/hydracache-loadgen"), b"mutated").unwrap();
    let error = verify_published_bundle(&root).unwrap_err().to_string();
    assert!(error.contains("path/hash receipt"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn prebuild_manifest_hash_binds_resp_reference_inputs() {
    let root = temp_root("receipt-binding");
    let (_host, outcome) = build(&root);
    let bundle = verify_published_bundle(&root).unwrap();
    assert_eq!(
        bundle
            .run_inputs
            .external_tool_prebuild
            .payload
            .prebuild_manifest_sha256,
        outcome.manifest_sha256
    );
    assert_eq!(
        sha256_file(&outcome.run_inputs_path).unwrap(),
        outcome.run_inputs_sha256
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn compile_time_is_excluded_from_measurement_window() {
    let root = temp_root("build-window");
    let (host, _outcome) = build(&root);
    let exact = vec![
        "build",
        "-p",
        "hydracache-loadgen",
        "-p",
        "hydracache-server",
        "--release",
        "--locked",
    ];
    assert_eq!(host.cargo_calls(), [exact.as_slice()]);
    let calls_before_consumer = host.calls.len();
    verify_published_bundle(&root).unwrap();
    assert_eq!(host.calls.len(), calls_before_consumer);
    fs::remove_dir_all(root).unwrap();

    let repo_root = xtask::doc_check::find_repo_root().unwrap();
    let registry = xtask::gated_tests::load_registry(&repo_root).unwrap();
    let prebuild = registry
        .gate
        .iter()
        .find(|gate| gate.id == "tool.perf-prebuild-067")
        .unwrap();
    assert_eq!(prebuild.command.program, "cargo");
    assert!(prebuild
        .command
        .args
        .iter()
        .any(|arg| arg == "perf-prebuild"));
    for gate_id in [
        "env.hydracache-run-067-perf-core",
        "env.hydracache-run-067-perf-resp",
        "env.hydracache-run-067-perf-control-plane",
    ] {
        let consumer = registry
            .gate
            .iter()
            .find(|gate| gate.id == gate_id)
            .unwrap();
        assert_eq!(
            consumer.command.program,
            "target/release/hydracache-loadgen"
        );
        assert!(!consumer.command.args.iter().any(|arg| arg == "cargo"));
    }
}

#[test]
fn consumer_gate_does_not_delete_the_prebuild_manifest() {
    let root = temp_root("consumer-preserves");
    let (mut host, outcome) = build(&root);
    let manifest_before = fs::read(&outcome.manifest_path).unwrap();
    let inputs_before = fs::read(&outcome.run_inputs_path).unwrap();
    verify_published_bundle(&root).unwrap();
    verify_published_bundle(&root).unwrap();
    assert_eq!(fs::read(&outcome.manifest_path).unwrap(), manifest_before);
    assert_eq!(fs::read(&outcome.run_inputs_path).unwrap(), inputs_before);

    let error = execute_prebuild_with(&root, "0.67", "reference-v1", &mut host)
        .unwrap_err()
        .to_string();
    assert!(error.contains("never overwritten"));
    assert_eq!(fs::read(&outcome.manifest_path).unwrap(), manifest_before);
    assert_eq!(fs::read(&outcome.run_inputs_path).unwrap(), inputs_before);
    fs::remove_dir_all(root).unwrap();
}
