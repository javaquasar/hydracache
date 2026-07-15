use std::error::Error;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::canary_check::{self, CanaryCommand, CanaryEntry, CanaryTier};

pub const RECEIPTS_DIR: &str = "target/release-evidence/canaries";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanaryOutcome {
    ExpectedRed,
    StayedGreen,
    Timeout,
    CompileFailure,
    WrongFailure,
    GuardFailed,
    Skipped,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CanaryReceipt {
    pub schema_version: u32,
    pub release: String,
    pub w_item: String,
    pub source_commit: String,
    pub dirty_worktree: bool,
    pub defect_id: String,
    pub command_digest: String,
    pub registry_digest: String,
    pub expected_failure: String,
    pub started_at: String,
    pub ended_at: String,
    pub duration_ms: u64,
    pub exit_code: Option<i32>,
    pub outcome: CanaryOutcome,
    pub output_sha256: String,
}

#[derive(Debug)]
pub struct ProcessResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub skipped: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SweepTier {
    Fast,
    All,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let static_problems = canary_check::check_canary_registry(&options.root)?;
    if !static_problems.is_empty() {
        return Err(format!("canary registry is invalid: {}", static_problems.join("; ")).into());
    }
    let registry = canary_check::load_registry(&options.root)?;
    if normalize_release(&registry.release) != normalize_release(&options.release) {
        return Err(format!(
            "canary registry release {} does not match {}",
            registry.release, options.release
        )
        .into());
    }

    let mut failures = Vec::new();
    let mut executed = 0usize;
    for entry in registry.entries.iter().filter(|entry| {
        (options.tier == SweepTier::All || entry.tier == CanaryTier::Fast)
            && options
                .w_item
                .as_ref()
                .is_none_or(|w_item| w_item == &entry.w_item)
    }) {
        executed += 1;
        let receipt = execute_entry(&options.root, &registry, entry)?;
        println!(
            "canary-sweep: {} {:?} ({} ms)",
            entry.w_item, receipt.outcome, receipt.duration_ms
        );
        if receipt.outcome != CanaryOutcome::ExpectedRed {
            failures.push(format!("{}={:?}", entry.w_item, receipt.outcome));
        }
    }
    if executed == 0 {
        return Err("canary-sweep selected no registry entries".into());
    }
    if failures.is_empty() {
        println!("canary-sweep: OK ({executed} expected-red proofs)");
        Ok(())
    } else {
        Err(format!("canary-sweep rejected: {}", failures.join(", ")).into())
    }
}

pub fn execute_entry(
    root: &Path,
    registry: &canary_check::CanaryRegistry,
    entry: &CanaryEntry,
) -> Result<CanaryReceipt, Box<dyn Error>> {
    let (source_commit, dirty_worktree) = git_identity(root);
    let started_at = OffsetDateTime::now_utc();
    let timer = Instant::now();
    let guard = execute_command(
        root,
        &entry.guard_command,
        entry.timeout_seconds,
        &entry.w_item,
    )?;
    let guard_failed =
        !guard.skipped && (guard.timed_out || guard.exit_code != Some(0) || no_tests_ran(&guard));
    let result = if guard.skipped {
        ProcessResult {
            exit_code: None,
            stdout: guard.stdout,
            stderr: guard.stderr,
            timed_out: false,
            skipped: true,
        }
    } else if guard_failed {
        ProcessResult {
            exit_code: guard.exit_code,
            stdout: guard.stdout,
            stderr: guard.stderr,
            timed_out: guard.timed_out,
            skipped: false,
        }
    } else {
        execute_command(
            root,
            &entry.canary_command,
            entry.timeout_seconds,
            &entry.w_item,
        )?
    };
    let outcome = if guard_failed {
        CanaryOutcome::GuardFailed
    } else {
        classify_canary_result(&result, &entry.expected_failure)
    };
    if outcome != CanaryOutcome::ExpectedRed {
        eprintln!(
            "canary-sweep: {} diagnostic:\n{}{}",
            entry.w_item, result.stdout, result.stderr
        );
    }
    let output = format!("{}{}", result.stdout, result.stderr);
    let receipt = CanaryReceipt {
        schema_version: 1,
        release: registry.release.clone(),
        w_item: entry.w_item.clone(),
        source_commit,
        dirty_worktree,
        defect_id: entry.defect_id.clone(),
        command_digest: command_digest(entry),
        registry_digest: sha256(&serde_json::to_vec(registry)?),
        expected_failure: entry.expected_failure.clone(),
        started_at: started_at.format(&Rfc3339)?,
        ended_at: OffsetDateTime::now_utc().format(&Rfc3339)?,
        duration_ms: timer.elapsed().as_millis() as u64,
        exit_code: result.exit_code,
        outcome,
        output_sha256: sha256(output.as_bytes()),
    };
    write_receipt(root, &receipt)?;
    Ok(receipt)
}

pub fn classify_canary_result(result: &ProcessResult, expected: &str) -> CanaryOutcome {
    if result.skipped {
        return CanaryOutcome::Skipped;
    }
    if result.timed_out {
        return CanaryOutcome::Timeout;
    }
    if result.exit_code == Some(0) {
        return CanaryOutcome::StayedGreen;
    }
    let output = format!("{}{}", result.stdout, result.stderr);
    if looks_like_compile_failure(&output) {
        return CanaryOutcome::CompileFailure;
    }
    if output.contains(expected) {
        CanaryOutcome::ExpectedRed
    } else {
        CanaryOutcome::WrongFailure
    }
}

pub fn receipt_problems(
    _root: &Path,
    registry: &canary_check::CanaryRegistry,
    entry: &CanaryEntry,
    receipt: &CanaryReceipt,
    source_commit: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    if receipt.schema_version != 1 {
        problems.push("unsupported canary receipt schema".to_owned());
    }
    if normalize_release(&receipt.release) != normalize_release(&registry.release) {
        problems.push("wrong release".to_owned());
    }
    if receipt.w_item != entry.w_item {
        problems.push("wrong work item".to_owned());
    }
    if receipt.source_commit != source_commit {
        problems.push("wrong source commit".to_owned());
    }
    if receipt.dirty_worktree {
        problems.push("canary receipt was produced from a dirty worktree".to_owned());
    }
    if receipt.defect_id != entry.defect_id {
        problems.push("stale defect id".to_owned());
    }
    if receipt.command_digest != command_digest(entry) {
        problems.push("stale canary command digest".to_owned());
    }
    match serde_json::to_vec(registry) {
        Ok(bytes) if receipt.registry_digest != sha256(&bytes) => {
            problems.push("stale canary registry digest".to_owned())
        }
        Err(error) => problems.push(format!("cannot hash canary registry: {error}")),
        _ => {}
    }
    if receipt.expected_failure != entry.expected_failure {
        problems.push("stale expected failure signature".to_owned());
    }
    if receipt.outcome != CanaryOutcome::ExpectedRed {
        problems.push(format!("canary outcome is {:?}", receipt.outcome));
    }
    problems
}

pub fn load_receipts(root: &Path) -> Result<Vec<CanaryReceipt>, Box<dyn Error>> {
    let directory = root.join(RECEIPTS_DIR);
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut receipts = Vec::new();
    for path in fs::read_dir(directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
    {
        receipts.push(serde_json::from_slice(&fs::read(path)?)?);
    }
    Ok(receipts)
}

fn execute_command(
    root: &Path,
    command: &CanaryCommand,
    timeout_seconds: u64,
    identity: &str,
) -> Result<ProcessResult, Box<dyn Error>> {
    if !platform_matches(&command.platform) {
        return Ok(ProcessResult {
            exit_code: None,
            stdout: String::new(),
            stderr: format!("platform {} does not match", command.platform),
            timed_out: false,
            skipped: true,
        });
    }
    let directory = root.join("target/canary-sweep/process");
    fs::create_dir_all(&directory)?;
    let base = format!("{}-{}", std::process::id(), sanitize(identity));
    let stdout_path = directory.join(format!("{base}.stdout"));
    let stderr_path = directory.join(format!("{base}.stderr"));
    let stdout_file = File::create(&stdout_path)?;
    let stderr_file = File::create(&stderr_path)?;
    let mut child = Command::new(&command.program)
        .args(&command.args)
        .envs(&command.env)
        .current_dir(root.join(&command.cwd))
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
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
        skipped: false,
    })
}

fn looks_like_compile_failure(output: &str) -> bool {
    output.contains("could not compile")
        || output.contains("error[E")
        || output.contains("linking with `")
        || output.contains("linker `") && output.contains("not found")
}

fn no_tests_ran(result: &ProcessResult) -> bool {
    let output = format!("{}{}", result.stdout, result.stderr);
    output.contains("running 0 tests") || output.contains("0 passed; 0 failed")
}

fn write_receipt(root: &Path, receipt: &CanaryReceipt) -> Result<(), Box<dyn Error>> {
    let directory = root.join(RECEIPTS_DIR);
    fs::create_dir_all(&directory)?;
    fs::write(
        directory.join(format!("{}.json", receipt.w_item)),
        serde_json::to_vec_pretty(receipt)?,
    )?;
    Ok(())
}

fn command_digest(entry: &CanaryEntry) -> String {
    sha256(
        &serde_json::to_vec(&(
            &entry.guard_command,
            &entry.canary_command,
            &entry.defect_id,
            &entry.expected_failure,
            entry.timeout_seconds,
        ))
        .expect("serializable canary command"),
    )
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

fn platform_matches(platform: &str) -> bool {
    platform == "any" || platform == std::env::consts::OS
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

fn normalize_release(value: &str) -> &str {
    value.strip_suffix(".0").unwrap_or(value)
}

struct Options {
    root: PathBuf,
    release: String,
    tier: SweepTier,
    w_item: Option<String>,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut root = crate::doc_check::find_repo_root()?;
        let mut release = None;
        let mut tier = None;
        let mut w_item = None;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--root" => root = PathBuf::from(args.next().ok_or("--root requires a path")?),
                "--release" => release = Some(args.next().ok_or("--release requires a value")?),
                "--tier" => {
                    tier = Some(match args.next().as_deref() {
                        Some("fast") => SweepTier::Fast,
                        Some("all") => SweepTier::All,
                        value => return Err(format!("invalid canary tier: {value:?}").into()),
                    })
                }
                "--w-item" => w_item = Some(args.next().ok_or("--w-item requires a value")?),
                other => return Err(format!("unknown canary-sweep argument: {other}").into()),
            }
        }
        Ok(Self {
            root,
            release: release.ok_or("canary-sweep requires --release")?,
            tier: tier.ok_or("canary-sweep requires --tier")?,
            w_item,
        })
    }
}
