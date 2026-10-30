//! Development proof workers: one execution/I/O deadline and owned process-tree cleanup.
//! This is not a general sandbox. Linux contains inherited process-group members;
//! Windows assigns a suspended child to a kill-on-close Job before any worker runs.
use std::fmt;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

#[cfg(windows)]
#[path = "imap_proof_process_windows_075.rs"]
mod platform;
#[cfg(target_os = "linux")]
#[path = "imap_proof_process_linux_075.rs"]
mod platform;

pub(crate) const PROCESS_SCOPE: &str = platform::SCOPE;
#[cfg(test)]
#[path = "../tests/support/imap_proof_checkpoints_075.rs"]
mod checkpoint_tests;
#[cfg(all(test, any(windows, target_os = "linux")))]
#[path = "../tests/support/imap_proof_pipe_setup_075.rs"]
mod pipe_setup_tests;

/// Fault-worker announcement only; callers still own process containment/deadlines.
/// The fixture exposes checkpoint errors without changing the LLVM environment.
#[doc(hidden)]
pub fn announce_fixture(
    file: &std::path::Path,
    exit_after: bool,
    mut checkpoint: impl FnMut() -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut ready = false;
    loop {
        std::thread::park_timeout(Duration::from_millis(2));
        if !ready {
            checkpoint()?;
            std::fs::write(file, std::process::id().to_string())?;
            ready = true;
        }
        if exit_after {
            checkpoint()?;
            return Ok(());
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ProcessBounds {
    pub timeout: Duration,
    pub cleanup_timeout: Duration,
    pub input_bytes: usize,
    pub stdout_bytes: usize,
    pub stderr_bytes: usize,
    pub memory_bytes: usize,
    /// Windows job process cap. Linux requires zero: no per-group count claim.
    pub maximum_processes: u32,
}
impl Default for ProcessBounds {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(15),
            cleanup_timeout: Duration::from_secs(2),
            input_bytes: 2 * 1024 * 1024,
            stdout_bytes: 16 * 1024 * 1024,
            stderr_bytes: 64 * 1024,
            memory_bytes: if cfg!(windows) {
                512 * 1024 * 1024
            } else {
                8usize * 1024 * 1024 * 1024
            },
            maximum_processes: if cfg!(windows) { 8 } else { 0 },
        }
    }
}
impl ProcessBounds {
    fn validate(self) -> Result<(), ProcessFailure> {
        self.validate_policy(ExecutionPolicy::Proof)
    }
    fn validate_policy(self, policy: ExecutionPolicy) -> Result<(), ProcessFailure> {
        if cfg!(target_os = "linux") && self.maximum_processes != 0 {
            return Err(ProcessFailure::new(FailureKind::UnsupportedPlatform,"Linux process-count isolation is unavailable; zero explicitly declares no count cap"));
        }
        if self.timeout.is_zero()
            || self.timeout > policy.maximum_duration()
            || self.cleanup_timeout.is_zero()
            || self.cleanup_timeout > Duration::from_secs(5)
            || self.input_bytes == 0
            || self.input_bytes > 2 * 1024 * 1024
            || self.stdout_bytes == 0
            || self.stdout_bytes > policy.maximum_output()
            || self.stderr_bytes == 0
            || self.stderr_bytes > 1024 * 1024
            || self.memory_bytes < 16 * 1024 * 1024
            || self.memory_bytes > 16usize * 1024 * 1024 * 1024
            || (cfg!(windows) && !(1..=32).contains(&self.maximum_processes))
        {
            return Err(ProcessFailure::new(
                FailureKind::InvalidBounds,
                "invalid proof process bounds",
            ));
        }
        Ok(())
    }
}

/// Internal, closed policies. Public proof calls always retain their 60-second cap.
#[derive(Clone, Copy)]
pub(crate) enum ExecutionPolicy {
    Proof,
    Build,
    Soak,
}
impl ExecutionPolicy {
    fn maximum_duration(self) -> Duration {
        Duration::from_secs(match self {
            Self::Proof => 60,
            Self::Build => 2 * 60 * 60,
            Self::Soak => 26 * 60 * 60,
        })
    }
    fn maximum_output(self) -> usize {
        match self {
            Self::Proof => 16 * 1024 * 1024,
            Self::Build => 64 * 1024 * 1024,
            Self::Soak => 8 * 1024 * 1024,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    InvalidBounds,
    InputLimit,
    Spawn,
    Containment,
    Timeout,
    StdoutLimit,
    StderrLimit,
    Io,
    Exit,
    Cleanup,
    UnsupportedPlatform,
    Observation,
}
#[derive(Debug)]
pub struct ProcessFailure {
    pub kind: FailureKind,
    pub detail: String,
    pub direct_child_reaped: bool,
    pub remaining_live_members: Option<usize>,
}
impl ProcessFailure {
    fn new(kind: FailureKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
            direct_child_reaped: false,
            remaining_live_members: None,
        }
    }
    fn cleaned(mut self, remaining: usize) -> Self {
        self.direct_child_reaped = true;
        self.remaining_live_members = Some(remaining);
        self
    }
}
impl fmt::Display for ProcessFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "proof process {:?}: {}", self.kind, self.detail)
    }
}
impl std::error::Error for ProcessFailure {}
fn io_error(kind: FailureKind, error: io::Error) -> ProcessFailure {
    ProcessFailure::new(kind, error.to_string())
}

#[derive(Debug)]
pub struct ProcessOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: ExitStatus,
    pub scope: &'static str,
    pub direct_child_reaped: bool,
    pub remaining_live_members: usize,
    pub io_threads: usize,
}

pub struct ProcessObservation<'a> {
    pub stdout: &'a [u8],
    pub stderr: &'a [u8],
    pub root_exited: bool,
    pub pid: u32,
    pub elapsed: Duration,
}

pub fn execute(
    command: Command,
    input: &[u8],
    bounds: ProcessBounds,
) -> Result<ProcessOutput, ProcessFailure> {
    execute_observed(command, input, bounds, |_| Ok(()))
}

#[cfg(any(windows, target_os = "linux"))]
pub fn execute_observed(
    command: Command,
    input: &[u8],
    bounds: ProcessBounds,
    observe: impl FnMut(ProcessObservation<'_>) -> Result<(), String>,
) -> Result<ProcessOutput, ProcessFailure> {
    bounds.validate()?;
    execute_managed(command, input, bounds, ExecutionPolicy::Proof, observe)
}

#[cfg(any(windows, target_os = "linux"))]
pub(crate) fn execute_managed(
    command: Command,
    input: &[u8],
    bounds: ProcessBounds,
    policy: ExecutionPolicy,
    observe: impl FnMut(ProcessObservation<'_>) -> Result<(), String>,
) -> Result<ProcessOutput, ProcessFailure> {
    execute_with_pipe_setup(command, input, bounds, policy, observe, prepare_pipes)
}
#[cfg(any(windows, target_os = "linux"))]
fn prepare_pipes(
    stdout: &std::process::ChildStdout,
    stderr: &std::process::ChildStderr,
) -> io::Result<()> {
    platform::prepare_pipe(stdout).and_then(|()| platform::prepare_pipe(stderr))
}
#[cfg(any(windows, target_os = "linux"))]
fn execute_with_pipe_setup(
    mut command: Command,
    input: &[u8],
    bounds: ProcessBounds,
    policy: ExecutionPolicy,
    mut observe: impl FnMut(ProcessObservation<'_>) -> Result<(), String>,
    prepare: impl FnOnce(&std::process::ChildStdout, &std::process::ChildStderr) -> io::Result<()>,
) -> Result<ProcessOutput, ProcessFailure> {
    bounds.validate_policy(policy)?;
    if input.len() > bounds.input_bytes {
        return Err(ProcessFailure::new(
            FailureKind::InputLimit,
            "proof input exceeds byte bound",
        ));
    }
    // Fixed launch arguments and artifact hashes must not be bypassed by inherited
    // JVM agents, classpaths or dynamic-loader injection. This includes fingerprints.
    for variable in [
        "JAVA_TOOL_OPTIONS",
        "JDK_JAVA_OPTIONS",
        "_JAVA_OPTIONS",
        "CLASSPATH",
        "LD_PRELOAD",
        "LD_AUDIT",
        "LD_LIBRARY_PATH",
    ] {
        command.env_remove(variable);
    }
    let started = Instant::now();
    // A bounded anonymous file avoids a potentially blocked stdin writer thread.
    // No path, credentials, or input payload is exposed in process arguments.
    let mut stdin = tempfile::tempfile().map_err(|e| io_error(FailureKind::Io, e))?;
    stdin
        .write_all(input)
        .map_err(|e| io_error(FailureKind::Io, e))?;
    stdin
        .seek(SeekFrom::Start(0))
        .map_err(|e| io_error(FailureKind::Io, e))?;
    command
        .stdin(Stdio::from(stdin))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if started.elapsed() >= bounds.timeout {
        return Err(ProcessFailure::new(
            FailureKind::Timeout,
            "input preparation exceeded deadline",
        ));
    }
    let (mut tree, mut child) = platform::Tree::spawn(&mut command, bounds).map_err(|e| {
        io_error(
            if e.kind() == io::ErrorKind::NotFound {
                FailureKind::Spawn
            } else {
                FailureKind::Containment
            },
            e,
        )
    })?;
    let mut stdout = child.stdout.take().expect("piped proof stdout");
    let mut stderr = child.stderr.take().expect("piped proof stderr");
    let preparation = prepare(&stdout, &stderr);
    if let Err(error) = preparation {
        let remaining = cleanup(&mut tree, &mut child, bounds.cleanup_timeout)?;
        return Err(io_error(FailureKind::Containment, error).cleaned(remaining));
    }
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let (mut out_closed, mut err_closed) = (false, false);
    let mut status = None;
    let mut terminated = false;
    let outcome = (|| {
        loop {
            if !out_closed {
                out_closed = drain(
                    &mut stdout,
                    &mut out,
                    bounds.stdout_bytes,
                    FailureKind::StdoutLimit,
                )?;
            }
            if !err_closed {
                err_closed = drain(
                    &mut stderr,
                    &mut err,
                    bounds.stderr_bytes,
                    FailureKind::StderrLimit,
                )?;
            }
            if status.is_none() {
                status = tree
                    .poll_exit(&mut child)
                    .map_err(|e| io_error(FailureKind::Io, e))?;
            }
            observe(ProcessObservation {
                stdout: &out,
                stderr: &err,
                root_exited: status.is_some(),
                pid: child.id(),
                elapsed: started.elapsed(),
            })
            .map_err(|reason| {
                ProcessFailure::new(
                    FailureKind::Observation,
                    reason.chars().take(512).collect::<String>(),
                )
            })?;
            if started.elapsed() >= bounds.timeout {
                return Err(ProcessFailure::new(
                    FailureKind::Timeout,
                    "observer or process exceeded deadline",
                ));
            }
            if status.is_some() && !terminated {
                tree.terminate()
                    .map_err(|e| io_error(FailureKind::Cleanup, e))?;
                terminated = true;
            }
            if status.is_some() && out_closed && err_closed {
                break;
            }
            if started.elapsed() >= bounds.timeout {
                return Err(ProcessFailure::new(
                    FailureKind::Timeout,
                    "execution or inherited pipe exceeded deadline",
                ));
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        Ok(())
    })();
    // Every return after spawn kills inherited descendants and reaps our direct child.
    // I/O uses nonblocking polls; there are no reader/writer threads to detach or join.
    let remaining = cleanup(&mut tree, &mut child, bounds.cleanup_timeout)?;
    outcome.map_err(|failure| failure.cleaned(remaining))?;
    let status = status
        .ok_or_else(|| ProcessFailure::new(FailureKind::Cleanup, "missing child exit status"))?;
    if !status.success() {
        return Err(ProcessFailure::new(
            FailureKind::Exit,
            format!("{status}: {}", String::from_utf8_lossy(&err)),
        )
        .cleaned(remaining));
    }
    Ok(ProcessOutput {
        stdout: out,
        stderr: err,
        status,
        scope: platform::SCOPE,
        direct_child_reaped: true,
        remaining_live_members: remaining,
        io_threads: 0,
    })
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn execute_observed(
    _: Command,
    _: &[u8],
    bounds: ProcessBounds,
    _: impl FnMut(ProcessObservation<'_>) -> Result<(), String>,
) -> Result<ProcessOutput, ProcessFailure> {
    bounds.validate()?;
    Err(ProcessFailure::new(
        FailureKind::UnsupportedPlatform,
        "proof containment supports Windows Job Objects and Linux inherited process groups",
    ))
}

#[cfg(not(any(windows, target_os = "linux")))]
pub(crate) fn execute_managed(
    command: Command,
    input: &[u8],
    bounds: ProcessBounds,
    policy: ExecutionPolicy,
    observe: impl FnMut(ProcessObservation<'_>) -> Result<(), String>,
) -> Result<ProcessOutput, ProcessFailure> {
    bounds.validate_policy(policy)?;
    execute_observed(command, input, bounds, observe)
}

#[cfg(any(windows, target_os = "linux"))]
fn drain(
    source: &mut (impl Read + platform::Pipe),
    output: &mut Vec<u8>,
    maximum: usize,
    kind: FailureKind,
) -> Result<bool, ProcessFailure> {
    // A bounded quantum lets stderr, exit and deadline checks progress under a flood.
    let mut block = [0u8; 8192];
    for _ in 0..8 {
        let available = platform::available(source).map_err(|e| io_error(FailureKind::Io, e))?;
        let Some(available) = available else {
            return Ok(true);
        };
        if available == 0 {
            return Ok(false);
        }
        let count = available
            .min(block.len())
            .min(maximum.saturating_sub(output.len()).saturating_add(1));
        match source.read(&mut block[..count]) {
            Ok(0) => return Ok(true),
            Ok(n) => {
                if n > maximum - output.len() {
                    return Err(ProcessFailure::new(
                        kind,
                        "worker output exceeds byte bound",
                    ));
                }
                output.extend_from_slice(&block[..n]);
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(false),
            Err(error) => return Err(io_error(FailureKind::Io, error)),
        }
    }
    Ok(false)
}
#[cfg(any(windows, target_os = "linux"))]
fn cleanup(
    tree: &mut platform::Tree,
    child: &mut Child,
    timeout: Duration,
) -> Result<usize, ProcessFailure> {
    tree.terminate()
        .map_err(|e| io_error(FailureKind::Cleanup, e))?;
    let started = Instant::now();
    loop {
        // Repeat owned-session cleanup: cooperative inner controllers may have
        // forked a group while the first process inventory was being collected.
        tree.terminate()
            .map_err(|e| io_error(FailureKind::Cleanup, e))?;
        let remaining = tree
            .live_members()
            .map_err(|e| io_error(FailureKind::Cleanup, e))?;
        let reaped = remaining == 0
            && child
                .try_wait()
                .map_err(|e| io_error(FailureKind::Cleanup, e))?
                .is_some();
        if reaped && remaining == 0 {
            return Ok(0);
        }
        if started.elapsed() >= timeout {
            return Err(ProcessFailure::new(
                FailureKind::Cleanup,
                "process group did not quiesce within cleanup deadline",
            ));
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
