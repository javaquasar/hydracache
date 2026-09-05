use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "hydracache-ci-reliability-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temp directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn python() -> &'static str {
    if cfg!(windows) {
        "python"
    } else {
        "python3"
    }
}

fn run_watchdog(temp: &TempDir, timeout: f64, command: &[&str]) -> (Output, Value) {
    let receipt = temp.path().join("receipt.json");
    let log = temp.path().join("child.log");
    let output = Command::new(python())
        .arg(repo_root().join("scripts/ci/run-with-heartbeat.py"))
        .args([
            "--timeout-seconds",
            &timeout.to_string(),
            "--heartbeat-seconds",
            "0.05",
            "--status-json",
        ])
        .arg(&receipt)
        .arg("--log-file")
        .arg(&log)
        .args(["--attempt-id", "fixture-attempt", "--"])
        .args(command)
        .output()
        .expect("run watchdog");
    let receipt = serde_json::from_slice(&fs::read(receipt).expect("watchdog receipt"))
        .expect("valid watchdog JSON");
    (output, receipt)
}

fn write_fixture(temp: &TempDir, workflow: &str, topology: Value) -> PathBuf {
    let workflow_dir = temp.path().join(".github/workflows");
    fs::create_dir_all(&workflow_dir).expect("workflow directory");
    fs::write(workflow_dir.join("fixture.yml"), workflow).expect("fixture workflow");
    let topology_path = temp.path().join("topology.json");
    fs::write(
        &topology_path,
        serde_json::to_vec_pretty(&topology).expect("serialize topology"),
    )
    .expect("fixture topology");
    topology_path
}

fn topology(classes: Value, producer: &str, triggers: &[&str]) -> Value {
    json!({
        "schema_version": 1,
        "release": "0.71",
        "publication_producer": producer,
        "workflows": [{
            "path": ".github/workflows/fixture.yml",
            "purpose": "focused reliability fixture",
            "triggers": triggers,
            "concurrency_markers": ["github.sha"],
            "classes": classes
        }]
    })
}

fn classes(core: &[&str], release: &[&str], publish: &[&str]) -> Value {
    json!({
        "core": core,
        "release-only": release,
        "scheduled-diagnostic": [],
        "manual-protected": [],
        "publish": publish
    })
}

#[test]
fn checked_in_topology_is_closed_and_valid() {
    xtask::ci_topology::check(&repo_root(), "0.71").expect("checked-in topology must pass");
}

#[test]
fn topology_rejects_missing_timeout_and_duplicate_branch_tag_execution() {
    let temp = TempDir::new("trigger-timeout");
    let workflow = r#"
name: fixture
on:
  push:
    branches: [main]
    tags: ['v*']
concurrency:
  group: fixture-${{ github.sha }}
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - run: cargo publish --dry-run
"#;
    let path = write_fixture(
        &temp,
        workflow,
        topology(
            classes(&["check"], &[], &[]),
            ".github/workflows/fixture.yml#check",
            &["push_branch", "push_tag"],
        ),
    );
    let error = xtask::ci_topology::check_with_path(temp.path(), "0.71", &path)
        .expect_err("invalid fixture")
        .to_string();
    assert!(error.contains("must declare a positive timeout-minutes"));
    assert!(error.contains("without an explicit reuse/rerun disposition"));
}

#[test]
fn topology_rejects_install_step_without_a_bounded_timeout() {
    let temp = TempDir::new("install-step-timeout");
    let workflow = r#"
name: fixture
on: workflow_dispatch
concurrency:
  group: fixture-${{ github.sha }}
jobs:
  check:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - name: Install management console dependencies
        run: npm ci --prefix console
"#;
    let path = write_fixture(
        &temp,
        workflow,
        topology(
            classes(&["check"], &[], &[]),
            ".github/workflows/fixture.yml#check",
            &["workflow_dispatch"],
        ),
    );
    let error = xtask::ci_topology::check_with_path(temp.path(), "0.71", &path)
        .expect_err("unbounded install step must fail")
        .to_string();
    assert!(error.contains(
        "Install management console dependencies\") needs a positive timeout-minutes smaller than job timeout 10"
    ));
}

#[test]
fn topology_rejects_mixed_artifact_identity() {
    let temp = TempDir::new("artifact");
    let workflow = r#"
name: fixture
on: workflow_dispatch
concurrency:
  group: fixture-${{ github.sha }}
jobs:
  publish:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - run: cargo publish --dry-run
      - uses: actions/upload-artifact@v6
        with:
          name: proof-${{ github.sha }}-${{ github.run_id }}
          path: receipt.json
"#;
    let path = write_fixture(
        &temp,
        workflow,
        topology(
            classes(&[], &[], &["publish"]),
            ".github/workflows/fixture.yml#publish",
            &["workflow_dispatch"],
        ),
    );
    let error = xtask::ci_topology::check_with_path(temp.path(), "0.71", &path)
        .expect_err("missing run attempt must fail")
        .to_string();
    assert!(error.contains("github.run_attempt"));
}

#[test]
fn topology_rejects_two_crate_publishers() {
    let temp = TempDir::new("publishers");
    let workflow = r#"
name: fixture
on: workflow_dispatch
concurrency:
  group: fixture-${{ github.sha }}
jobs:
  first:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - run: cargo publish --dry-run
  second:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - run: cargo publish --dry-run
"#;
    let path = write_fixture(
        &temp,
        workflow,
        topology(
            classes(&[], &[], &["first", "second"]),
            ".github/workflows/fixture.yml#first",
            &["workflow_dispatch"],
        ),
    );
    let error = xtask::ci_topology::check_with_path(temp.path(), "0.71", &path)
        .expect_err("two publishers must fail")
        .to_string();
    assert!(error.contains("exactly one producer"));
    assert!(error.contains("#first"));
    assert!(error.contains("#second"));
}

#[test]
fn watchdog_classifies_silent_success_and_product_failure() {
    let success = TempDir::new("watchdog-success");
    let (output, receipt) = run_watchdog(
        &success,
        5.0,
        &[python(), "-c", "import time; time.sleep(0.15)"],
    );
    assert!(output.status.success());
    assert_eq!(receipt["classification"], "success");
    assert!(String::from_utf8_lossy(&output.stdout).contains("watchdog heartbeat"));

    let failure = TempDir::new("watchdog-failure");
    let (output, receipt) = run_watchdog(
        &failure,
        5.0,
        &[
            python(),
            "-c",
            "print('provisioning failed', flush=True); raise SystemExit(23)",
        ],
    );
    assert_eq!(output.status.code(), Some(23));
    assert_eq!(receipt["classification"], "product-failure");
    assert_eq!(receipt["exit_code"], 23);
    assert!(fs::read_to_string(failure.path().join("child.log"))
        .expect("retained child log")
        .contains("provisioning failed"));
}

#[test]
fn watchdog_times_out_and_kills_descendant_processes() {
    let temp = TempDir::new("watchdog-descendant");
    let marker = temp.path().join("descendant-survived.txt");
    let outer = concat!(
        "import subprocess,sys,time;",
        "subprocess.Popen([sys.executable,'-c',",
        "'import pathlib,sys,time;time.sleep(1.5);pathlib.Path(sys.argv[1]).write_text(\"alive\")',",
        "sys.argv[1]]);time.sleep(10)"
    );
    let marker_arg = marker.to_string_lossy().into_owned();
    let (output, receipt) =
        run_watchdog(&temp, 0.35, &[python(), "-c", outer, marker_arg.as_str()]);
    assert_eq!(output.status.code(), Some(124));
    assert_eq!(receipt["classification"], "timeout");
    std::thread::sleep(Duration::from_secs(2));
    assert!(!marker.exists(), "descendant escaped the process-tree kill");
}

#[test]
fn watchdog_classifies_missing_provisioning_tool() {
    let temp = TempDir::new("watchdog-tool-unavailable");
    let missing = format!("hydracache-missing-tool-{}", std::process::id());
    let (output, receipt) = run_watchdog(&temp, 5.0, &[missing.as_str()]);
    assert_eq!(output.status.code(), Some(127));
    assert_eq!(receipt["classification"], "tool-unavailable");
}
