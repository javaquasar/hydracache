use std::collections::BTreeMap;
use std::error::Error;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::fast_suite::{self, FastSuiteEntry};
use crate::gated_tests::CommandSpec;

const REPORT_PATH: &str = "target/test-evidence/0.64/determinism-sweep.json";

#[derive(Serialize)]
struct DeterminismReport {
    schema_version: u32,
    release: String,
    source_commit: String,
    dirty_worktree: bool,
    suites: Vec<SuiteReport>,
}

#[derive(Serialize)]
struct SuiteReport {
    id: String,
    first_digest: String,
    repeated_digest: String,
    serial_digest: String,
    matched: bool,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let registry = fast_suite::load_registry(&options.root)?;
    let suites = registry
        .suite
        .iter()
        .filter(|suite| suite.deterministic)
        .collect::<Vec<_>>();
    if suites.is_empty() {
        return Err("determinism-sweep found no deterministic suites".into());
    }

    let mut reports = Vec::new();
    let mut failures = Vec::new();
    for suite in suites {
        let first = run_and_digest(&options.root, suite, RunnerMode::Normal, "first")?;
        let repeated = run_and_digest(&options.root, suite, RunnerMode::Normal, "repeat")?;
        let serial = run_and_digest(&options.root, suite, RunnerMode::Serial, "serial")?;
        let matched = digests_match([&first, &repeated, &serial]);
        if !matched {
            failures.push(suite.id.clone());
        }
        reports.push(SuiteReport {
            id: suite.id.clone(),
            first_digest: first,
            repeated_digest: repeated,
            serial_digest: serial,
            matched,
        });
    }

    let (source_commit, dirty_worktree) = git_identity(&options.root);
    let report = DeterminismReport {
        schema_version: 1,
        release: registry.release,
        source_commit,
        dirty_worktree,
        suites: reports,
    };
    let path = options.root.join(REPORT_PATH);
    fs::create_dir_all(path.parent().expect("report parent"))?;
    fs::write(&path, serde_json::to_vec_pretty(&report)?)?;
    if failures.is_empty() {
        println!(
            "determinism-sweep: OK ({} suites, repeated + serial digests match)",
            report.suites.len()
        );
        Ok(())
    } else {
        Err(format!(
            "determinism-sweep found logical drift in {}",
            failures.join(", ")
        )
        .into())
    }
}

pub fn logical_digest(value: &Value) -> Result<String, Box<dyn Error>> {
    validate_logical_shape(value)?;
    let canonical =
        canonicalize(value).ok_or("logical evidence became empty after normalization")?;
    Ok(sha256(&serde_json::to_vec(&canonical)?))
}

pub fn digests_match<'a>(digests: impl IntoIterator<Item = &'a String>) -> bool {
    let mut digests = digests.into_iter();
    let Some(first) = digests.next() else {
        return false;
    };
    digests.all(|digest| digest == first)
}

fn run_and_digest(
    root: &Path,
    suite: &FastSuiteEntry,
    mode: RunnerMode,
    label: &str,
) -> Result<String, Box<dyn Error>> {
    let artifact = root.join(&suite.logical_digest_artifact);
    if artifact.is_file() {
        fs::remove_file(&artifact)?;
    }
    let mut command = suite.command.clone();
    if mode == RunnerMode::Serial {
        add_serial_test_arg(&mut command.args);
    }
    let result = execute(root, &command, suite.timeout_seconds, &suite.id, label)?;
    if result.timed_out {
        return Err(format!("{} {label} run timed out", suite.id).into());
    }
    if result.exit_code != Some(0) {
        return Err(format!(
            "{} {label} run failed with {:?}: {}{}",
            suite.id, result.exit_code, result.stdout, result.stderr
        )
        .into());
    }
    if !artifact.is_file() {
        return Err(format!(
            "{} did not write logical digest artifact {}",
            suite.id, suite.logical_digest_artifact
        )
        .into());
    }
    let value: Value = serde_json::from_slice(&fs::read(&artifact)?)?;
    logical_digest(&value)
}

fn validate_logical_shape(value: &Value) -> Result<(), Box<dyn Error>> {
    let object = value
        .as_object()
        .ok_or("logical digest artifact must be a JSON object")?;
    for field in [
        "seed",
        "schedule",
        "operations",
        "invariant_verdicts",
        "final_state",
    ] {
        if !object.contains_key(field) {
            return Err(format!("logical digest artifact is missing {field}").into());
        }
    }
    Ok(())
}

fn canonicalize(value: &Value) -> Option<Value> {
    match value {
        Value::Object(object) => {
            let mut canonical = BTreeMap::new();
            for (key, value) in object {
                if ephemeral_key(key) {
                    continue;
                }
                if let Some(value) = canonicalize(value) {
                    canonical.insert(key.clone(), value);
                }
            }
            let map: Map<String, Value> = canonical.into_iter().collect();
            Some(Value::Object(map))
        }
        Value::Array(values) => Some(Value::Array(
            values.iter().filter_map(canonicalize).collect(),
        )),
        other => Some(other.clone()),
    }
}

fn ephemeral_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "wall_clock",
        "timestamp",
        "started_at",
        "ended_at",
        "duration",
        "elapsed",
        "absolute_path",
        "workspace_path",
        "temp_path",
        "port",
        "thread_id",
        "process_id",
    ]
    .iter()
    .any(|ephemeral| key == *ephemeral || key.starts_with(&format!("{ephemeral}_")))
}

fn add_serial_test_arg(args: &mut Vec<String>) {
    if let Some(separator) = args.iter().position(|arg| arg == "--") {
        args.insert(separator + 1, "--test-threads=1".to_owned());
    } else {
        args.push("--".to_owned());
        args.push("--test-threads=1".to_owned());
    }
}

struct ProcessResult {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    timed_out: bool,
}

fn execute(
    root: &Path,
    command: &CommandSpec,
    timeout_seconds: u64,
    suite: &str,
    label: &str,
) -> Result<ProcessResult, Box<dyn Error>> {
    if command.platform != "any" && command.platform != std::env::consts::OS {
        return Err(format!("suite {suite} does not support this platform").into());
    }
    let directory = root.join("target/determinism-sweep/process");
    fs::create_dir_all(&directory)?;
    let stem = format!("{}-{}-{}", std::process::id(), sanitize(suite), label);
    let stdout_path = directory.join(format!("{stem}.stdout"));
    let stderr_path = directory.join(format!("{stem}.stderr"));
    let mut child = Command::new(&command.program)
        .args(&command.args)
        .envs(&command.env)
        .current_dir(root.join(&command.cwd))
        .stdout(Stdio::from(File::create(&stdout_path)?))
        .stderr(Stdio::from(File::create(&stderr_path)?))
        .spawn()?;
    let deadline = Instant::now() + Duration::from_secs(timeout_seconds);
    let (exit_code, timed_out) = loop {
        if let Some(status) = child.try_wait()? {
            break (status.code(), false);
        }
        if Instant::now() >= deadline {
            child.kill()?;
            let status = child.wait()?;
            break (status.code(), true);
        }
        thread::sleep(Duration::from_millis(25));
    };
    let stdout = fs::read_to_string(&stdout_path).unwrap_or_default();
    let stderr = fs::read_to_string(&stderr_path).unwrap_or_default();
    let _ = fs::remove_file(stdout_path);
    let _ = fs::remove_file(stderr_path);
    Ok(ProcessResult {
        exit_code,
        stdout,
        stderr,
        timed_out,
    })
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn git_identity(root: &Path) -> (String, bool) {
    let commit = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_owned())
        .unwrap_or_else(|| "unknown".to_owned());
    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
        .map(|output| !output.stdout.is_empty())
        .unwrap_or(true);
    (commit, dirty)
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RunnerMode {
    Normal,
    Serial,
}

struct Options {
    root: PathBuf,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut root = crate::doc_check::find_repo_root()?;
        let mut release = None;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--root" => root = PathBuf::from(args.next().ok_or("--root requires a path")?),
                "--release" => release = Some(args.next().ok_or("--release requires a value")?),
                other => return Err(format!("unknown determinism-sweep argument: {other}").into()),
            }
        }
        let release = release.ok_or("determinism-sweep requires --release")?;
        if release != "0.64" && release != "0.64.0" {
            return Err(format!("unsupported determinism release {release}").into());
        }
        Ok(Self { root })
    }
}
