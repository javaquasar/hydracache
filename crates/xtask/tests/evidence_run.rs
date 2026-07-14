use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use xtask::evidence_run::{EvidenceOutcome, EvidenceReceipt};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
const CHILD_ENV: &str = "HYDRACACHE_EVIDENCE_CHILD";

#[test]
fn evidence_child_helper() {
    match std::env::var(CHILD_ENV).as_deref() {
        Ok("pass") => println!("logical pass"),
        Ok("fail") => panic!("intentional evidence failure"),
        Ok("timeout") => std::thread::sleep(Duration::from_secs(10)),
        _ => {}
    }
}

fn temp_root(name: &str) -> PathBuf {
    let serial = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "hydracache-evidence-{name}-{}-{serial}",
        std::process::id()
    ));
    fs::create_dir_all(root.join("docs/testing")).unwrap();
    fs::create_dir_all(root.join("target/receipts")).unwrap();
    run(&root, "git", &["init", "-q"]);
    run(
        &root,
        "git",
        &["config", "user.email", "evidence@example.invalid"],
    );
    run(&root, "git", &["config", "user.name", "Evidence Test"]);
    root
}

fn run(root: &Path, program: &str, args: &[&str]) {
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "{program} {args:?} failed");
}

fn write_registry(root: &Path, mode: &str, timeout_seconds: u64, artifacts: Vec<String>) {
    let mut env = BTreeMap::new();
    env.insert(CHILD_ENV.to_owned(), mode.to_owned());
    let registry = xtask::gated_tests::GatedTestRegistry {
        schema_version: 1,
        release: "0.64.0".to_owned(),
        gate: vec![xtask::gated_tests::GateEntry {
            id: format!("test.{mode}"),
            kind: xtask::gated_tests::GateKind::CfgTestTarget,
            source: "tests/evidence_run.rs".to_owned(),
            package: "xtask".to_owned(),
            target: "evidence_run".to_owned(),
            test: "evidence_child_helper".to_owned(),
            cfg: String::new(),
            env: String::new(),
            reason: "executor fixture".to_owned(),
            tier: xtask::gated_tests::GateTier::Fast,
            required_features: vec![],
            required_env: vec![],
            required_tools: vec![],
            timeout_seconds,
            owner_release: "0.64.0".to_owned(),
            ship_mandatory: true,
            artifacts,
            ci: xtask::gated_tests::CiRegistration {
                workflow: ".github/workflows/ci.yml".to_owned(),
                job: "evidence".to_owned(),
                step: "run".to_owned(),
            },
            command: xtask::gated_tests::CommandSpec {
                program: std::env::current_exe()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                args: vec![
                    "--exact".to_owned(),
                    "evidence_child_helper".to_owned(),
                    "--nocapture".to_owned(),
                ],
                env,
                cwd: ".".to_owned(),
                platform: "any".to_owned(),
            },
        }],
    };
    fs::write(
        root.join(xtask::gated_tests::REGISTRY_PATH),
        toml::to_string_pretty(&registry).unwrap(),
    )
    .unwrap();
    run(root, "git", &["add", "."]);
    run(root, "git", &["commit", "-q", "-m", "fixture"]);
}

fn execute(mode: &str, timeout_seconds: u64) -> (PathBuf, xtask::evidence_run::ExecutionResult) {
    let root = temp_root(mode);
    write_registry(&root, mode, timeout_seconds, vec![]);
    let result = xtask::evidence_run::execute_gate(
        &root,
        "0.64",
        &format!("test.{mode}"),
        Path::new("target/receipts"),
    )
    .unwrap();
    (root, result)
}

#[test]
fn evidence_executor_writes_atomic_receipts_for_success_failure_and_timeout() {
    for (mode, expected) in [
        ("pass", EvidenceOutcome::Pass),
        ("fail", EvidenceOutcome::Fail),
        ("timeout", EvidenceOutcome::Timeout),
    ] {
        let (root, result) = execute(mode, 1);
        assert_eq!(result.receipt.outcome, expected);
        assert!(result.receipt_path.is_file());
        assert!(!result.receipt.dirty_worktree);
        assert_eq!(result.receipt.source_commit.len(), 40);
        assert_eq!(result.receipt.command_digest.len(), 64);
        assert_eq!(result.receipt.registry_digest.len(), 64);
        assert_eq!(result.receipt.input_digest.len(), 64);
        let parsed: EvidenceReceipt =
            serde_json::from_slice(&fs::read(&result.receipt_path).unwrap()).unwrap();
        assert_eq!(parsed.outcome, expected);
        assert!(fs::read_dir(root.join("target/receipts"))
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")));
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn evidence_executor_propagates_child_exit_and_captures_output() {
    let (pass_root, pass) = execute("pass", 5);
    assert_eq!(xtask::evidence_run::exit_code_for(&pass.receipt), 0);
    assert!(pass.receipt.stdout.contains("logical pass"));
    fs::remove_dir_all(pass_root).unwrap();

    let (fail_root, fail) = execute("fail", 5);
    assert_ne!(xtask::evidence_run::exit_code_for(&fail.receipt), 0);
    assert!(fail.receipt.stderr.contains("intentional evidence failure"));
    fs::remove_dir_all(fail_root).unwrap();

    let (timeout_root, timeout) = execute("timeout", 1);
    assert_eq!(xtask::evidence_run::exit_code_for(&timeout.receipt), 124);
    fs::remove_dir_all(timeout_root).unwrap();
}

#[test]
fn evidence_executor_rejects_shells_path_traversal_and_artifacts_outside_target() {
    let root = temp_root("unsafe");
    write_registry(&root, "unsafe", 1, vec!["../secret".to_owned()]);
    let error = xtask::evidence_run::execute_gate(
        &root,
        "0.64",
        "test.unsafe",
        Path::new("target/receipts"),
    )
    .unwrap_err();
    assert!(error.to_string().contains("path traversal"));

    let registry_path = root.join(xtask::gated_tests::REGISTRY_PATH);
    let mut registry: xtask::gated_tests::GatedTestRegistry =
        toml::from_str(&fs::read_to_string(&registry_path).unwrap()).unwrap();
    registry.gate[0].artifacts = vec!["docs/not-an-artifact.json".to_owned()];
    fs::write(&registry_path, toml::to_string_pretty(&registry).unwrap()).unwrap();
    let error = xtask::evidence_run::execute_gate(
        &root,
        "0.64",
        "test.unsafe",
        Path::new("target/receipts"),
    )
    .unwrap_err();
    assert!(error.to_string().contains("target directory"));

    registry.gate[0].artifacts.clear();
    registry.gate[0].command.program = "cmd.exe".to_owned();
    fs::write(&registry_path, toml::to_string_pretty(&registry).unwrap()).unwrap();
    let error = xtask::evidence_run::execute_gate(
        &root,
        "0.64",
        "test.unsafe",
        Path::new("target/receipts"),
    )
    .unwrap_err();
    assert!(error.to_string().contains("shell program is forbidden"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn evidence_executor_runs_a_registered_fast_suite_without_shell_indirection() {
    let root = temp_root("fast-suite");
    let gated = xtask::gated_tests::GatedTestRegistry {
        schema_version: 1,
        release: "0.64.0".to_owned(),
        gate: vec![],
    };
    fs::write(
        root.join(xtask::gated_tests::REGISTRY_PATH),
        toml::to_string_pretty(&gated).unwrap(),
    )
    .unwrap();
    let mut env = BTreeMap::new();
    env.insert(CHILD_ENV.to_owned(), "pass".to_owned());
    let fast = xtask::fast_suite::FastSuiteRegistry {
        schema_version: 1,
        release: "0.64.0".to_owned(),
        nextest_version: "0.9.137".to_owned(),
        aggregate_budget_seconds: 1_500,
        suite: vec![xtask::fast_suite::FastSuiteEntry {
            id: "fast.fixture".to_owned(),
            work_items: vec!["W1".to_owned()],
            timeout_seconds: 5,
            budget_seconds: 5,
            deterministic: false,
            artifacts: vec![],
            logical_digest_artifact: String::new(),
            baseline: xtask::fast_suite::Baseline {
                status: xtask::fast_suite::BaselineStatus::Unmeasured,
                commit: String::new(),
                toolchain: String::new(),
                linux_ci_median_seconds: 0,
                noise_allowance_seconds: 0,
            },
            command: xtask::gated_tests::CommandSpec {
                program: std::env::current_exe()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                args: vec![
                    "--exact".to_owned(),
                    "evidence_child_helper".to_owned(),
                    "--nocapture".to_owned(),
                ],
                env,
                cwd: ".".to_owned(),
                platform: "any".to_owned(),
            },
        }],
    };
    fs::write(
        root.join(xtask::fast_suite::REGISTRY_PATH),
        toml::to_string_pretty(&fast).unwrap(),
    )
    .unwrap();
    run(&root, "git", &["add", "."]);
    run(&root, "git", &["commit", "-q", "-m", "fast fixture"]);

    let result = xtask::evidence_run::execute_gate(
        &root,
        "0.64",
        "fast.fixture",
        Path::new("target/receipts"),
    )
    .unwrap();
    assert_eq!(result.receipt.outcome, EvidenceOutcome::Pass);
    assert!(result.receipt.stdout.contains("logical pass"));
    fs::remove_dir_all(root).unwrap();
}
