use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration as StdDuration, Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::doc_check;
use crate::fast_suite;
use crate::gated_tests::{self, CommandSpec, GateEntry};

pub const DEFAULT_RECEIPTS_DIR: &str = "target/release-evidence/receipts";
const MAX_DIAGNOSTIC_CHARS_PER_STREAM: usize = 32_000;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceOutcome {
    Pass,
    Fail,
    Timeout,
    Skip,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceReceipt {
    pub schema_version: u32,
    pub release: String,
    pub gate_id: String,
    pub source_commit: String,
    pub dirty_worktree: bool,
    pub command_digest: String,
    pub registry_digest: String,
    pub input_digest: String,
    pub toolchain: String,
    pub container_identity: BTreeMap<String, String>,
    pub platform: String,
    pub started_at: String,
    pub ended_at: String,
    pub duration_ms: u64,
    pub outcome: EvidenceOutcome,
    pub exit_code: Option<i32>,
    pub normalized_result: NormalizedResult,
    pub stdout: String,
    pub stderr: String,
    pub artifacts: Vec<ArtifactDigest>,
    pub missing_artifacts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedResult {
    pub outcome: EvidenceOutcome,
    pub exit_code: Option<i32>,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactDigest {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug)]
pub struct ExecutionResult {
    pub receipt_path: PathBuf,
    pub receipt: EvidenceReceipt,
}

#[derive(Debug, Clone)]
pub struct ExpectedDigests {
    pub command: String,
    pub registry: String,
    pub input: String,
}

pub fn run(args: Vec<String>) -> Result<i32, Box<dyn Error>> {
    let options = Options::parse(args)?;
    let result = execute_gate(
        &options.root,
        &options.release,
        &options.gate_id,
        &options.receipts_dir,
    )?;
    println!(
        "evidence-run: {:?} gate={} receipt={}",
        result.receipt.outcome,
        result.receipt.gate_id,
        result.receipt_path.display()
    );
    if let Some(diagnostics) = failure_diagnostics(&result.receipt) {
        eprintln!("{diagnostics}");
    }
    Ok(exit_code_for(&result.receipt))
}

pub fn failure_diagnostics(receipt: &EvidenceReceipt) -> Option<String> {
    if receipt.outcome == EvidenceOutcome::Pass {
        return None;
    }
    let mut output = format!(
        "evidence-run: captured output for {:?} gate={}",
        receipt.outcome, receipt.gate_id
    );
    append_diagnostic_stream(&mut output, "stdout", &receipt.stdout);
    append_diagnostic_stream(&mut output, "stderr", &receipt.stderr);
    Some(output)
}

fn append_diagnostic_stream(output: &mut String, name: &str, value: &str) {
    if value.trim().is_empty() {
        return;
    }
    let chars = value.chars().count();
    let visible = if chars > MAX_DIAGNOSTIC_CHARS_PER_STREAM {
        let tail = value
            .chars()
            .skip(chars - MAX_DIAGNOSTIC_CHARS_PER_STREAM)
            .collect::<String>();
        format!("[truncated to last {MAX_DIAGNOSTIC_CHARS_PER_STREAM} characters]\n{tail}")
    } else {
        value.to_owned()
    };
    let _ = write!(output, "\n--- {name} ---\n{}", visible.trim_end());
}

pub fn execute_gate(
    root: &Path,
    release: &str,
    gate_id: &str,
    receipts_dir: &Path,
) -> Result<ExecutionResult, Box<dyn Error>> {
    validate_identifier(gate_id)?;
    let receipts_dir = resolve_receipts_dir(root, receipts_dir)?;
    let gate = resolve_registered_gate(root, release, gate_id)?;
    validate_command(&gate.command)?;
    let artifact_paths = validate_artifact_paths(root, &gate.artifacts)?;

    let expected = expected_digests_for(root, gate.registry_path, &gate.id, &gate.command)?;
    let (source_commit, dirty_worktree) = git_identity(root)?;
    remove_stale_declared_artifacts(&artifact_paths)?;
    let started = OffsetDateTime::now_utc();
    let timer = Instant::now();

    let process = if platform_matches(&gate.command.platform) {
        execute_command(root, &gate.command, gate.timeout_seconds)
    } else {
        ProcessResult {
            outcome: EvidenceOutcome::Skip,
            exit_code: None,
            stdout: String::new(),
            stderr: format!(
                "gate platform {} does not match {}",
                gate.command.platform,
                std::env::consts::OS
            ),
        }
    };

    let mut artifacts = Vec::new();
    let mut missing_artifacts = Vec::new();
    for (declared, path) in artifact_paths {
        if path.is_file() {
            let bytes = fs::read(&path)?;
            artifacts.push(ArtifactDigest {
                path: declared,
                sha256: sha256(&bytes),
                bytes: bytes.len() as u64,
            });
        } else {
            missing_artifacts.push(declared);
        }
    }

    let mut outcome = process.outcome;
    let mut stderr = process.stderr;
    if !missing_artifacts.is_empty() && outcome == EvidenceOutcome::Pass {
        outcome = EvidenceOutcome::Fail;
        stderr.push_str("\nmissing declared artifact(s)");
    }
    let ended = OffsetDateTime::now_utc();
    let receipt = EvidenceReceipt {
        schema_version: 1,
        release: normalize_release(release),
        gate_id: gate_id.to_owned(),
        source_commit,
        dirty_worktree,
        command_digest: expected.command,
        registry_digest: expected.registry,
        input_digest: expected.input,
        toolchain: command_output("rustc", &["--version"]).unwrap_or_else(|| "unknown".to_owned()),
        container_identity: container_identity(),
        platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        started_at: started.format(&Rfc3339)?,
        ended_at: ended.format(&Rfc3339)?,
        duration_ms: timer.elapsed().as_millis().try_into().unwrap_or(u64::MAX),
        outcome,
        exit_code: process.exit_code,
        normalized_result: NormalizedResult {
            outcome,
            exit_code: process.exit_code,
            stdout_sha256: sha256(process.stdout.as_bytes()),
            stderr_sha256: sha256(stderr.as_bytes()),
        },
        stdout: process.stdout,
        stderr,
        artifacts,
        missing_artifacts,
    };
    let receipt_path = write_receipt_atomic(&receipts_dir, &receipt)?;
    Ok(ExecutionResult {
        receipt_path,
        receipt,
    })
}

pub fn expected_digests(root: &Path, gate: &GateEntry) -> Result<ExpectedDigests, Box<dyn Error>> {
    expected_digests_for(root, gated_tests::REGISTRY_PATH, &gate.id, &gate.command)
}

pub fn expected_fast_digests(
    root: &Path,
    suite: &fast_suite::FastSuiteEntry,
) -> Result<ExpectedDigests, Box<dyn Error>> {
    expected_digests_for(root, fast_suite::REGISTRY_PATH, &suite.id, &suite.command)
}

fn expected_digests_for(
    root: &Path,
    registry_path: &str,
    id: &str,
    command_spec: &CommandSpec,
) -> Result<ExpectedDigests, Box<dyn Error>> {
    let registry = sha256(&fs::read(root.join(registry_path))?);
    let command = sha256(&serde_json::to_vec(command_spec)?);
    let input = sha256(format!("{id}\n{registry}\n{command}").as_bytes());
    Ok(ExpectedDigests {
        command,
        registry,
        input,
    })
}

pub fn exit_code_for(receipt: &EvidenceReceipt) -> i32 {
    match receipt.outcome {
        EvidenceOutcome::Pass => 0,
        EvidenceOutcome::Fail => receipt.exit_code.filter(|code| *code != 0).unwrap_or(1),
        EvidenceOutcome::Timeout => 124,
        EvidenceOutcome::Skip => 125,
    }
}

fn execute_command(root: &Path, command_spec: &CommandSpec, timeout_seconds: u64) -> ProcessResult {
    let cwd = match resolve_cwd(root, &command_spec.cwd) {
        Ok(cwd) => cwd,
        Err(error) => {
            return ProcessResult {
                outcome: EvidenceOutcome::Fail,
                exit_code: None,
                stdout: String::new(),
                stderr: error.to_string(),
            };
        }
    };
    let mut command = Command::new(&command_spec.program);
    command
        .args(&command_spec.args)
        .envs(&command_spec.env)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_process_group(&mut command);

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return ProcessResult {
                outcome: EvidenceOutcome::Skip,
                exit_code: None,
                stdout: String::new(),
                stderr: format!("failed to start registered program: {error}"),
            };
        }
    };
    let stdout = child.stdout.take().map(read_pipe);
    let stderr = child.stderr.take().map(read_pipe);
    let deadline = Instant::now() + StdDuration::from_secs(timeout_seconds.max(1));
    let (outcome, status) = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let outcome = if status.success() {
                    EvidenceOutcome::Pass
                } else {
                    EvidenceOutcome::Fail
                };
                break (outcome, Some(status));
            }
            Ok(None) if Instant::now() >= deadline => {
                terminate_process_tree(&mut child);
                let status = child.wait().ok();
                break (EvidenceOutcome::Timeout, status);
            }
            Ok(None) => thread::sleep(StdDuration::from_millis(10)),
            Err(error) => {
                terminate_process_tree(&mut child);
                return ProcessResult {
                    outcome: EvidenceOutcome::Fail,
                    exit_code: None,
                    stdout: join_pipe(stdout),
                    stderr: format!(
                        "failed while waiting for child: {error}\n{}",
                        join_pipe(stderr)
                    ),
                };
            }
        }
    };
    ProcessResult {
        outcome,
        exit_code: status.and_then(exit_status_code),
        stdout: join_pipe(stdout),
        stderr: join_pipe(stderr),
    }
}

struct ProcessResult {
    outcome: EvidenceOutcome,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn read_pipe<R: Read + Send + 'static>(mut pipe: R) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = pipe.read_to_end(&mut bytes);
        String::from_utf8_lossy(&bytes).into_owned()
    })
}

fn join_pipe(handle: Option<thread::JoinHandle<String>>) -> String {
    handle
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default()
}

fn exit_status_code(status: ExitStatus) -> Option<i32> {
    status.code()
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_process_group(_command: &mut Command) {}

#[cfg(unix)]
fn terminate_process_tree(child: &mut std::process::Child) {
    let _ = Command::new("kill")
        .args(["-KILL", "--", &format!("-{}", child.id())])
        .status();
    let _ = child.kill();
}

#[cfg(windows)]
fn terminate_process_tree(child: &mut std::process::Child) {
    let _ = Command::new("taskkill")
        .args(["/PID", &child.id().to_string(), "/T", "/F"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = child.kill();
}

#[cfg(not(any(unix, windows)))]
fn terminate_process_tree(child: &mut std::process::Child) {
    let _ = child.kill();
}

fn validate_command(command: &CommandSpec) -> Result<(), Box<dyn Error>> {
    if command.program.trim().is_empty() {
        return Err("registered command program is empty".into());
    }
    let name = Path::new(&command.program)
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or(&command.program)
        .to_ascii_lowercase();
    if [
        "sh",
        "bash",
        "cmd",
        "cmd.exe",
        "powershell",
        "powershell.exe",
        "pwsh",
        "pwsh.exe",
    ]
    .contains(&name.as_str())
    {
        return Err(format!("unreviewed shell program is forbidden: {}", command.program).into());
    }
    Ok(())
}

fn validate_identifier(id: &str) -> Result<(), Box<dyn Error>> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(format!("unsafe gate id {id:?}").into());
    }
    Ok(())
}

fn resolve_cwd(root: &Path, cwd: &str) -> Result<PathBuf, Box<dyn Error>> {
    let relative = safe_relative_path(cwd)?;
    let resolved = root.join(relative);
    if !resolved.is_dir() {
        return Err(format!("registered cwd does not exist: {}", resolved.display()).into());
    }
    Ok(resolved)
}

fn validate_artifact_paths(
    root: &Path,
    artifacts: &[String],
) -> Result<Vec<(String, PathBuf)>, Box<dyn Error>> {
    artifacts
        .iter()
        .map(|declared| {
            let relative = safe_relative_path(declared)?;
            if relative.components().next() != Some(Component::Normal(OsStr::new("target"))) {
                return Err(format!(
                    "artifact path must be inside the repository target directory: {declared}"
                )
                .into());
            }
            Ok((declared.replace('\\', "/"), root.join(relative)))
        })
        .collect()
}

fn remove_stale_declared_artifacts(
    artifact_paths: &[(String, PathBuf)],
) -> Result<(), Box<dyn Error>> {
    for (declared, path) in artifact_paths {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "unable to remove stale declared artifact {declared}: {error}"
                )
                .into());
            }
        }
    }
    Ok(())
}

struct RegisteredGate {
    id: String,
    command: CommandSpec,
    timeout_seconds: u64,
    artifacts: Vec<String>,
    registry_path: &'static str,
}

fn resolve_registered_gate(
    root: &Path,
    release: &str,
    gate_id: &str,
) -> Result<RegisteredGate, Box<dyn Error>> {
    let gated = gated_tests::load_registry(root)?;
    if !registry_covers_release(&gated.release, release) {
        return Err(format!(
            "gated registry release {} does not match requested release {release}",
            gated.release
        )
        .into());
    }
    if let Some(gate) = gated.gate.into_iter().find(|gate| gate.id == gate_id) {
        return Ok(RegisteredGate {
            id: gate.id,
            command: gate.command,
            timeout_seconds: gate.timeout_seconds,
            artifacts: gate.artifacts,
            registry_path: gated_tests::REGISTRY_PATH,
        });
    }

    let fast = fast_suite::load_registry(root)?;
    if !registry_covers_release(&fast.release, release) {
        return Err(format!(
            "fast-suite registry release {} does not match requested release {release}",
            fast.release
        )
        .into());
    }
    if let Some(suite) = fast.suite.into_iter().find(|suite| suite.id == gate_id) {
        return Ok(RegisteredGate {
            id: suite.id,
            command: suite.command,
            timeout_seconds: suite.timeout_seconds,
            artifacts: suite.artifacts,
            registry_path: fast_suite::REGISTRY_PATH,
        });
    }
    Err(format!("unknown gate id {gate_id}").into())
}

fn safe_relative_path(value: &str) -> Result<PathBuf, Box<dyn Error>> {
    let path = Path::new(value);
    if value.trim().is_empty() || path.is_absolute() {
        return Err(format!("path must be non-empty and relative: {value:?}").into());
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(format!("path traversal is forbidden: {value}").into());
    }
    Ok(path.to_path_buf())
}

fn resolve_receipts_dir(root: &Path, value: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let relative = safe_relative_path(value.to_string_lossy().as_ref())?;
    if relative.components().next() != Some(Component::Normal(OsStr::new("target"))) {
        return Err("receipts directory must be inside target".into());
    }
    Ok(root.join(relative))
}

fn write_receipt_atomic(
    receipts_dir: &Path,
    receipt: &EvidenceReceipt,
) -> Result<PathBuf, Box<dyn Error>> {
    fs::create_dir_all(receipts_dir)?;
    let suffix = OffsetDateTime::now_utc().unix_timestamp_nanos();
    let destination = receipts_dir.join(format!("{}-{suffix}.json", receipt.gate_id));
    let temporary = receipts_dir.join(format!(
        ".{}.{}.{suffix}.tmp",
        receipt.gate_id,
        std::process::id()
    ));
    fs::write(&temporary, serde_json::to_vec_pretty(receipt)?)?;
    fs::rename(&temporary, &destination)?;
    Ok(destination)
}

fn git_identity(root: &Path) -> Result<(String, bool), Box<dyn Error>> {
    let commit = command_output_in(root, "git", &["rev-parse", "HEAD"])
        .ok_or("unable to resolve source commit")?;
    let status = command_output_in(
        root,
        "git",
        &["status", "--porcelain", "--untracked-files=normal"],
    )
    .ok_or("unable to inspect worktree status")?;
    Ok((commit, !status.trim().is_empty()))
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn command_output_in(root: &Path, program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            let _ = write!(output, "{byte:02x}");
            output
        })
}

fn container_identity() -> BTreeMap<String, String> {
    [
        "CI",
        "GITHUB_ACTIONS",
        "GITHUB_RUN_ID",
        "GITHUB_RUN_ATTEMPT",
        "GITHUB_SHA",
        "HYDRACACHE_BUILD_IMAGE",
    ]
    .into_iter()
    .filter_map(|name| {
        std::env::var(name)
            .ok()
            .map(|value| (name.to_owned(), value))
    })
    .collect()
}

fn platform_matches(required: &str) -> bool {
    required == "any" || required.eq_ignore_ascii_case(std::env::consts::OS)
}

fn registry_covers_release(registry_release: &str, requested: &str) -> bool {
    let parse = |release: &str| -> Option<(u64, u64, u64)> {
        let normalized = normalize_release(release);
        let mut parts = normalized.split('.').map(str::parse::<u64>);
        let version = (
            parts.next()?.ok()?,
            parts.next()?.ok()?,
            parts.next()?.ok()?,
        );
        parts.next().is_none().then_some(version)
    };
    matches!(
        (parse(registry_release), parse(requested)),
        (Some(registry), Some(candidate)) if registry.0 == candidate.0 && registry <= candidate
    )
}

fn normalize_release(release: &str) -> String {
    if release.matches('.').count() == 1 {
        format!("{release}.0")
    } else {
        release.to_owned()
    }
}

struct Options {
    root: PathBuf,
    release: String,
    gate_id: String,
    receipts_dir: PathBuf,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut root = doc_check::find_repo_root()?;
        let mut release = None;
        let mut gate_id = None;
        let mut receipts_dir = PathBuf::from(DEFAULT_RECEIPTS_DIR);
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--root" => root = PathBuf::from(args.next().ok_or("--root requires a path")?),
                "--release" => release = Some(args.next().ok_or("--release requires a value")?),
                "--gate" => gate_id = Some(args.next().ok_or("--gate requires an id")?),
                "--receipts-dir" => {
                    receipts_dir =
                        PathBuf::from(args.next().ok_or("--receipts-dir requires a path")?)
                }
                other => return Err(format!("unknown evidence-run argument: {other}").into()),
            }
        }
        Ok(Self {
            root,
            release: release.ok_or("evidence-run requires --release")?,
            gate_id: gate_id.ok_or("evidence-run requires --gate")?,
            receipts_dir,
        })
    }
}
