//! Development build orchestration. Receipts never choose commands.
use crate::imap_proof_process_075::{self as process, ExecutionPolicy, FailureKind, ProcessBounds};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
#[cfg(windows)]
const BUILD_CONTAINMENT: &str = "windows_job_per_process_commit_limit";
#[cfg(target_os = "linux")]
const BUILD_CONTAINMENT: &str =
    "owned_session_nested_groups_per_process_address_space_no_count_cap";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildStage {
    ServerTests,
    HelperContractTests,
    SoakWorkerBuild,
    RustConsumerTests,
    RustConsumerBuild,
    JavaContract,
    NativeJava,
    NativeDependencyJar,
    NativeDependencyPom,
    ReproduceContract,
    ReproduceNative,
    ToolingTests,
    ProvenanceTools,
    ProvenanceMetadata,
    CoverageRunner,
    CoverageExecution,
    CoverageReport,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildBounds {
    pub phase_timeout: Duration,
    pub cleanup_reserve: Duration,
    pub stdout_bytes: usize,
    pub stderr_bytes: usize,
    pub maximum_events: usize,
    pub maximum_stages: u64,
    pub aggregate_output_bytes: usize,
    pub memory_bytes: usize,
    pub maximum_processes: u32,
    pub heartbeat: Duration,
}
impl Default for BuildBounds {
    fn default() -> Self {
        Self {
            phase_timeout: Duration::from_secs(20 * 60),
            cleanup_reserve: Duration::from_secs(20),
            stdout_bytes: 16 * 1024 * 1024,
            stderr_bytes: 1024 * 1024,
            maximum_events: 8192,
            maximum_stages: 32,
            aggregate_output_bytes: 64 * 1024 * 1024,
            memory_bytes: 8usize * 1024 * 1024 * 1024,
            maximum_processes: if cfg!(windows) { 32 } else { 0 },
            heartbeat: Duration::from_secs(1),
        }
    }
}
impl BuildBounds {
    fn validate(self) -> Result<(), String> {
        if self.phase_timeout.is_zero()
            || self.phase_timeout > Duration::from_secs(7200)
            || self.cleanup_reserve.is_zero()
            || self.cleanup_reserve > Duration::from_secs(300)
            || self.stdout_bytes == 0
            || self.stdout_bytes > 64 * 1024 * 1024
            || self.stderr_bytes == 0
            || self.stderr_bytes > 1024 * 1024
            || !(2..=65536).contains(&self.maximum_events)
            || self.maximum_stages == 0
            || self.maximum_stages > 256
            || self.aggregate_output_bytes == 0
            || self.aggregate_output_bytes > 256 * 1024 * 1024
            || self.memory_bytes < 64 * 1024 * 1024
            || self.memory_bytes > 16usize * 1024 * 1024 * 1024
            || (cfg!(windows) && !(1..=32).contains(&self.maximum_processes))
            || (cfg!(target_os = "linux") && self.maximum_processes != 0)
            || self.heartbeat.is_zero()
            || self.heartbeat > Duration::from_secs(60)
        {
            return Err("invalid build bounds".into());
        }
        Ok(())
    }
}
#[derive(Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildOutcome {
    Running,
    Completed,
    Failed,
    TimedOut,
    Cancelled,
    OutputLimit,
    JournalFailed,
    CleanupFailed,
    Refused,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildEvent {
    pub sequence: u64,
    pub stage: BuildStage,
    pub outcome: BuildOutcome,
    pub elapsed_ms: u64,
    pub stdout_bytes: usize,
    pub stderr_bytes: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildReport {
    pub bounds: BuildBounds,
    pub scope: String,
    pub stage: BuildStage,
    pub outcome: BuildOutcome,
    pub elapsed_ms: u64,
    pub effective_timeout_ms: u64,
    pub stdout_path: String,
    pub stderr_path: String,
    pub stdout_bytes: usize,
    pub stderr_bytes: usize,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub pid: Option<u32>,
    pub root_reaped: bool,
    pub remaining_live_members: Option<usize>,
    pub containment: String,
    pub journal_complete: bool,
    pub logs_complete: bool,
}
#[derive(Debug)]
pub struct BuildError {
    pub report: Box<BuildReport>,
    pub detail: String,
}
impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "build {:?}: {:?}: {}",
            self.report.stage, self.report.outcome, self.detail
        )
    }
}
impl std::error::Error for BuildError {}
fn ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}
fn hash(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn create(path: &Path) -> io::Result<File> {
    OpenOptions::new().create_new(true).write(true).open(path)
}
struct RetainedLog<W> {
    writer: W,
    bytes: usize,
    hash: Sha256,
}
impl<W: Write> RetainedLog<W> {
    fn new(writer: W) -> Self {
        Self {
            writer,
            bytes: 0,
            hash: Sha256::new(),
        }
    }
    fn append(&mut self, mut bytes: &[u8]) -> io::Result<()> {
        while !bytes.is_empty() {
            match self.writer.write(bytes) {
                Ok(0) => return Err(io::Error::from(io::ErrorKind::WriteZero)),
                Ok(n) => {
                    self.hash.update(&bytes[..n]);
                    self.bytes += n;
                    bytes = &bytes[n..];
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}
struct LogCapture<O, E> {
    out: RetainedLog<O>,
    err: RetainedLog<E>,
}
#[derive(Debug)]
struct LogFailure {
    journal: bool,
    aggregate: bool,
    detail: String,
}
impl<O: Write, E: Write> LogCapture<O, E> {
    fn new(out: O, err: E) -> Self {
        Self {
            out: RetainedLog::new(out),
            err: RetainedLog::new(err),
        }
    }
    fn append(&mut self, out: &[u8], err: &[u8], remaining: usize) -> Result<(), LogFailure> {
        if out.len() + err.len() > remaining {
            return Err(LogFailure {
                journal: false,
                aggregate: true,
                detail: "aggregate build output bound".into(),
            });
        }
        self.out
            .append(out)
            .and_then(|()| self.err.append(err))
            .map_err(|error| LogFailure {
                journal: true,
                aggregate: false,
                detail: error.to_string(),
            })
    }
    fn hashes(self) -> (String, String) {
        (
            hex(&self.out.hash.finalize()),
            hex(&self.err.hash.finalize()),
        )
    }
}
fn failure_outcome(
    kind: FailureKind,
    journal: bool,
    cancelled: bool,
    aggregate: bool,
) -> BuildOutcome {
    if kind == FailureKind::Cleanup {
        BuildOutcome::CleanupFailed
    } else if journal {
        BuildOutcome::JournalFailed
    } else if cancelled {
        BuildOutcome::Cancelled
    } else if aggregate {
        BuildOutcome::OutputLimit
    } else {
        match kind {
            FailureKind::Timeout => BuildOutcome::TimedOut,
            FailureKind::StdoutLimit | FailureKind::StderrLimit => BuildOutcome::OutputLimit,
            _ => BuildOutcome::Failed,
        }
    }
}
#[cfg(test)]
#[path = "../tests/support/imap_build_logs_075.rs"]
mod log_tests;

fn plain_build_component(symlink: bool, reparse: bool) -> Result<(), Box<dyn std::error::Error>> {
    if symlink {
        return Err("symlinked build output refused".into());
    }
    if reparse {
        return Err("reparse build output refused".into());
    }
    Ok(())
}
#[cfg(windows)]
pub(crate) fn reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}
#[cfg(not(windows))]
pub(crate) fn reparse(_: &fs::Metadata) -> bool {
    false
}
fn build_output_scope(output: &Path, root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !output.starts_with(root) {
        return Err("build output escapes root".into());
    }
    Ok(())
}

/// A controller owns a fresh directory and an absolute monotonic budget across all its phases.
pub struct BuildController {
    directory: PathBuf,
    deadline: Instant,
    bounds: BuildBounds,
    cancellation: Cancellation,
    journal: Box<dyn Write>,
    sequence: u64,
    stages: u64,
    stopped: bool,
    output_bytes: usize,
}
impl BuildController {
    pub fn new(root: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let remaining = remaining_budget()?;
        Self::with_budget(
            root,
            remaining,
            BuildBounds::default(),
            Cancellation::default(),
        )
    }
    pub fn with_budget(
        root: &Path,
        remaining: Duration,
        bounds: BuildBounds,
        cancellation: Cancellation,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        bounds.validate()?;
        if remaining.is_zero() || remaining > Duration::from_secs(7200) {
            return Err("invalid outer build budget".into());
        }
        let deadline = Instant::now() + remaining;
        let mut parent = root.to_path_buf();
        for name in ["target", "imap-075", "builds"] {
            parent.push(name);
            if let Ok(metadata) = fs::symlink_metadata(&parent) {
                plain_build_component(metadata.file_type().is_symlink(), reparse(&metadata))?;
            }
        }
        fs::create_dir_all(&parent)?;
        build_output_scope(&parent.canonicalize()?, &root.canonicalize()?)?;
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let directory = parent.join(format!(
            "{}-{}-{}",
            std::process::id(),
            stamp,
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory)?;
        let journal = Box::new(create(&directory.join("events.jsonl"))?);
        Ok(Self {
            directory,
            deadline,
            bounds,
            cancellation,
            journal,
            sequence: 0,
            stages: 0,
            stopped: false,
            output_bytes: 0,
        })
    }
    /// A failing sink is also useful to independently test journal failure cleanup.
    pub fn replace_journal(&mut self, journal: Box<dyn Write>) {
        self.journal = journal;
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    pub fn cancellation(&self) -> Cancellation {
        self.cancellation.clone()
    }
    pub fn events(&self) -> u64 {
        self.sequence
    }
    fn record(&mut self, event: &mut BuildEvent) -> io::Result<()> {
        if self.sequence >= self.bounds.maximum_events as u64 {
            return Err(io::Error::other("build event bound"));
        }
        event.sequence = self.sequence + 1;
        let mut line = serde_json::to_vec(event)?;
        line.push(b'\n');
        self.journal.write_all(&line)?;
        self.journal.flush()?;
        self.sequence += 1;
        Ok(())
    }
    /// Only trusted repository code constructs the command. It is never read from a receipt.
    pub fn run(&mut self, stage: BuildStage, command: Command) -> Result<BuildReport, BuildError> {
        let started = Instant::now();
        let effective = self
            .deadline
            .saturating_duration_since(started)
            .saturating_sub(self.bounds.cleanup_reserve)
            .min(self.bounds.phase_timeout);
        self.stages = self.stages.saturating_add(1);
        let prefix = format!("{}-{:?}", self.stages, stage);
        let stdout_path = self.directory.join(format!("{prefix}.stdout"));
        let stderr_path = self.directory.join(format!("{prefix}.stderr"));
        let mut report = BuildReport {
            bounds: self.bounds,
            scope: "pre074_development_build".into(),
            stage,
            outcome: BuildOutcome::Refused,
            elapsed_ms: 0,
            effective_timeout_ms: ms(effective),
            stdout_path: stdout_path.to_string_lossy().into(),
            stderr_path: stderr_path.to_string_lossy().into(),
            stdout_bytes: 0,
            stderr_bytes: 0,
            stdout_sha256: hash(&[]),
            stderr_sha256: hash(&[]),
            pid: None,
            root_reaped: false,
            remaining_live_members: None,
            containment: BUILD_CONTAINMENT.into(),
            journal_complete: false,
            logs_complete: false,
        };
        let error = |report: BuildReport, detail: String| BuildError {
            report: Box::new(report),
            detail: detail.chars().take(512).collect(),
        };
        if self.stopped {
            return Err(error(
                report,
                "previous build stage did not complete".into(),
            ));
        }
        self.stopped = true;
        if self.stages > self.bounds.maximum_stages
            || self.output_bytes >= self.bounds.aggregate_output_bytes
        {
            return Err(error(report, "aggregate build work/output bound".into()));
        }
        if self.cancellation.is_cancelled() {
            report.outcome = BuildOutcome::Cancelled;
            return Err(error(report, "cancelled before spawn".into()));
        }
        if effective < Duration::from_millis(10) {
            return Err(error(
                report,
                "insufficient outer budget before spawn".into(),
            ));
        }
        let mut out = create(&stdout_path).map_err(|e| error(report.clone(), e.to_string()))?;
        let mut err = create(&stderr_path).map_err(|e| error(report.clone(), e.to_string()))?;
        let mut event = BuildEvent {
            sequence: 0,
            stage,
            outcome: BuildOutcome::Running,
            elapsed_ms: 0,
            stdout_bytes: 0,
            stderr_bytes: 0,
        };
        self.record(&mut event).map_err(|e| {
            report.outcome = BuildOutcome::JournalFailed;
            error(report.clone(), e.to_string())
        })?;
        println!(
            "imap-build: {:?} started (budget {} ms)",
            stage,
            ms(effective)
        );
        let mut journal_failed = false;
        let mut aggregate_limit = false;
        let mut cancelled = false;
        let mut next_heartbeat = self.bounds.heartbeat;
        let mut logs = LogCapture::new(&mut out, &mut err);
        let remaining_output = self.bounds.aggregate_output_bytes - self.output_bytes;
        let timeout = effective.saturating_sub(started.elapsed());
        if timeout.is_zero() {
            report.outcome = BuildOutcome::TimedOut;
            return Err(error(
                report,
                "build setup consumed the remaining phase budget".into(),
            ));
        }
        let bounds = ProcessBounds {
            timeout,
            cleanup_timeout: Duration::from_secs(3),
            input_bytes: 1,
            stdout_bytes: self.bounds.stdout_bytes.min(remaining_output),
            stderr_bytes: self.bounds.stderr_bytes.min(remaining_output),
            memory_bytes: self.bounds.memory_bytes,
            maximum_processes: self.bounds.maximum_processes,
        };
        let result = process::execute_managed(
            command,
            &[],
            bounds,
            ExecutionPolicy::Build,
            |observation| {
                report.pid = Some(observation.pid);
                if self.cancellation.is_cancelled() && !observation.root_exited {
                    cancelled = true;
                    return Err("build cancellation accepted".into());
                }
                let a = &observation.stdout[report.stdout_bytes..];
                let b = &observation.stderr[report.stderr_bytes..];
                let previous = report.stdout_bytes + report.stderr_bytes;
                let retained =
                    logs.append(a, b, self.bounds.aggregate_output_bytes - self.output_bytes);
                report.stdout_bytes = logs.out.bytes;
                report.stderr_bytes = logs.err.bytes;
                self.output_bytes += report.stdout_bytes + report.stderr_bytes - previous;
                if let Err(failure) = retained {
                    aggregate_limit = failure.aggregate;
                    journal_failed = failure.journal;
                    return Err(failure.detail);
                }
                if observation.elapsed >= next_heartbeat {
                    event.elapsed_ms = ms(observation.elapsed);
                    event.stdout_bytes = report.stdout_bytes;
                    event.stderr_bytes = report.stderr_bytes;
                    if let Err(failure) = self.record(&mut event) {
                        journal_failed = true;
                        return Err(failure.to_string());
                    }
                    println!(
                        "imap-build: {:?} heartbeat {} ms (out={}, err={})",
                        stage, event.elapsed_ms, event.stdout_bytes, event.stderr_bytes
                    );
                    next_heartbeat = observation.elapsed + self.bounds.heartbeat;
                }
                Ok(())
            },
        );
        report.elapsed_ms = ms(started.elapsed());
        (report.stdout_sha256, report.stderr_sha256) = logs.hashes();
        let detail = match result {
            Ok(output) => {
                report.outcome = BuildOutcome::Completed;
                report.logs_complete = true;
                report.root_reaped = output.direct_child_reaped;
                report.remaining_live_members = Some(output.remaining_live_members);
                None
            }
            Err(failure) => {
                report.root_reaped = failure.direct_child_reaped;
                report.remaining_live_members = failure.remaining_live_members;
                report.outcome =
                    failure_outcome(failure.kind, journal_failed, cancelled, aggregate_limit);
                Some(failure.to_string())
            }
        };
        event.outcome = report.outcome;
        event.elapsed_ms = report.elapsed_ms;
        event.stdout_bytes = report.stdout_bytes;
        event.stderr_bytes = report.stderr_bytes;
        let final_write = self
            .record(&mut event)
            .and_then(|()| out.sync_all())
            .and_then(|()| err.sync_all());
        report.journal_complete = final_write.is_ok();
        if let Err(failure) = final_write {
            report.outcome = BuildOutcome::JournalFailed;
            return Err(error(report, failure.to_string()));
        }
        create(&self.directory.join(format!("{prefix}.json")))
            .and_then(|mut file| file.write_all(&serde_json::to_vec_pretty(&report)?))
            .map_err(|failure| {
                report.outcome = BuildOutcome::JournalFailed;
                error(report.clone(), failure.to_string())
            })?;
        println!(
            "imap-build: {:?} {:?} ({} ms; cleanup={})",
            stage, report.outcome, report.elapsed_ms, report.root_reaped
        );
        match detail {
            Some(detail) => Err(error(report, detail)),
            None => {
                self.stopped = false;
                Ok(report)
            }
        }
    }
}
/// The CI wrapper supplies its absolute harness deadline. This clock is used for budgets only.
pub fn remaining_budget() -> Result<Duration, Box<dyn std::error::Error>> {
    remaining_budget_from(
        std::env::var("HYDRACACHE_IMAP075_BUILD_DEADLINE_MS"),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
    )
}
fn remaining_budget_from(
    value: Result<String, std::env::VarError>,
    now: u128,
) -> Result<Duration, Box<dyn std::error::Error>> {
    let raw = match value {
        Ok(raw) => Some(raw),
        Err(std::env::VarError::NotPresent) => None,
        Err(failure) => return Err(failure.into()),
    };
    remaining_budget_at(raw.as_deref(), now)
}
pub fn remaining_budget_at(
    raw: Option<&str>,
    now: u128,
) -> Result<Duration, Box<dyn std::error::Error>> {
    match raw {
        Some(raw) => {
            let deadline: u128 = raw.parse()?;
            let remaining = deadline
                .checked_sub(now)
                .ok_or("outer build deadline expired")?;
            let millis = u64::try_from(remaining)?;
            if millis == 0 || millis > 7_200_000 {
                return Err("outer build budget out of bounds".into());
            }
            Ok(Duration::from_millis(millis))
        }
        None => Ok(Duration::from_secs(40 * 60)),
    }
}
