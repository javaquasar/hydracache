//! Continuous local process segments. Budgets use elapsed time; map correctness uses logical data.
use crate::imap_build_process_075::Cancellation;
use crate::imap_proof_process_075::{self as process, ExecutionPolicy, ProcessBounds};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    process::Command,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
#[cfg(windows)]
const WORKER_MEMORY_BYTES: usize = 1024 * 1024 * 1024;
#[cfg(target_os = "linux")]
const WORKER_MEMORY_BYTES: usize = 8usize * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cohort {
    Reference,
    Native,
    Daemon,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoakBounds {
    pub active_ms: u64,
    pub period_ms: u64,
    pub maximum_rounds: u64,
    pub warmup_rounds: u64,
    pub startup_ms: u64,
    pub drain_ms: u64,
    pub hard_ms: u64,
    pub host_remaining_ms: u64,
}
impl SoakBounds {
    pub fn smoke() -> Self {
        Self {
            active_ms: 200,
            period_ms: 10,
            maximum_rounds: 64,
            warmup_rounds: 2,
            startup_ms: 30000,
            drain_ms: 10000,
            hard_ms: 45000,
            host_remaining_ms: 60000,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.active_ms < 2
            || self.active_ms > 86400000
            || self.period_ms == 0
            || self.period_ms > 60000
            || self.active_ms / self.period_ms < 2
            || !(2..=8192).contains(&self.maximum_rounds)
            || self.active_ms.div_ceil(self.period_ms) + 2 > self.maximum_rounds
            || !(1..=16).contains(&self.warmup_rounds)
            || self.startup_ms < 100
            || self.startup_ms > 60000
            || self.drain_ms < 100
            || self.drain_ms > 30000
            || self.hard_ms < self.active_ms + self.startup_ms + self.drain_ms
            || self.hard_ms > 93600000
            || self.host_remaining_ms < self.hard_ms + 5000
        {
            return Err("invalid/ineligible continuous soak budget".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundArtifact {
    pub path: String,
    pub sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoakProfile {
    pub schema: String,
    pub source_sha: String,
    pub cohort: Cohort,
    pub seed: u64,
    pub bounds: SoakBounds,
    pub runtime: String,
    pub artifacts: Vec<BoundArtifact>,
}
impl SoakProfile {
    pub fn validate(&self) -> Result<(), String> {
        self.bounds.validate()?;
        if self.schema != "imap075-soak-profile-v1"
            || !crate::imap_qualification_075::hex(&self.source_sha, 40)
            || self.runtime.is_empty()
            || self.runtime.len() > 4096
            || self.artifacts.is_empty()
            || self.artifacts.len() > 8
        {
            return Err("invalid soak profile/source/runtime/artifacts".into());
        }
        let mut paths = std::collections::BTreeSet::new();
        for artifact in &self.artifacts {
            if artifact.path.is_empty()
                || artifact.path.contains('\\')
                || artifact.path.contains(':')
                || artifact.path.len() > 512
                || !paths.insert(&artifact.path)
                || Path::new(&artifact.path)
                    .components()
                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
                || !crate::imap_qualification_075::hex(&artifact.sha256, 64)
            {
                return Err("invalid soak artifact binding".into());
            }
        }
        Ok(())
    }
    fn check_artifacts(&self, root: &Path) -> Result<(), Box<dyn Error>> {
        if current_runtime(self.cohort)? != self.runtime {
            return Err("soak runtime profile changed".into());
        }
        for artifact in &self.artifacts {
            let path = root.join(&artifact.path);
            if !path.canonicalize()?.starts_with(root.canonicalize()?)
                || crate::imap_daemon_probe_075::binary_digest(&path)? != artifact.sha256
            {
                return Err("soak bound artifact changed".into());
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Ready,
    Warmup,
    Active,
    Round,
    Drained,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Owners {
    pub unit: String,
    #[serde(deserialize_with = "required_option")]
    pub values: Option<u64>,
    #[serde(deserialize_with = "required_option")]
    pub outcomes: Option<u64>,
    #[serde(deserialize_with = "required_option")]
    pub pending: Option<u64>,
    #[serde(deserialize_with = "required_option")]
    pub listeners: Option<u64>,
    #[serde(deserialize_with = "required_option")]
    pub streams: Option<u64>,
    #[serde(deserialize_with = "required_option")]
    pub resync: Option<u64>,
    #[serde(deserialize_with = "required_option")]
    pub queued: Option<u64>,
    #[serde(deserialize_with = "required_option")]
    pub correlations: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub schema: String,
    pub phase: Phase,
    pub pid: u32,
    pub incarnation: String,
    pub cohort: Cohort,
    pub seed: u64,
    pub source_sha: String,
    pub round: u64,
    pub active_elapsed_ms: u64,
    #[serde(deserialize_with = "required_option")]
    pub digest: Option<String>,
    pub owners: Owners,
    #[serde(deserialize_with = "required_option")]
    pub heap_bytes: Option<u64>,
    #[serde(deserialize_with = "required_option")]
    pub rss_bytes: Option<u64>,
    #[serde(deserialize_with = "required_option")]
    pub gc_collections: Option<u64>,
    #[serde(deserialize_with = "required_option")]
    pub cpu_ns: Option<u64>,
    #[serde(deserialize_with = "required_option")]
    pub threads: Option<u64>,
    #[serde(deserialize_with = "required_option")]
    pub reason: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerIdentity {
    pub pid: u32,
    pub platform_birth: String,
    pub incarnation: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentityCheckpoint {
    schema: String,
    segment: String,
    worker: WorkerIdentity,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Terminal {
    Complete,
    Cancelled,
    TimedOut,
    Failed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoakReport {
    pub schema: String,
    pub scope: String,
    pub production_enabled: bool,
    pub profile: SoakProfile,
    pub segment: String,
    #[serde(deserialize_with = "required_option")]
    pub parent_segment: Option<String>,
    pub controller: WorkerIdentity,
    #[serde(deserialize_with = "required_option")]
    pub worker: Option<WorkerIdentity>,
    pub terminal: Terminal,
    pub rounds: u64,
    pub active_elapsed_ms: u64,
    pub transcript_sha256: String,
    pub transcript_bytes: u64,
    pub process_reaped: bool,
    #[serde(deserialize_with = "required_option")]
    pub remaining_members: Option<usize>,
    pub containment: String,
    pub detail: String,
}
fn required_option<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(deserializer)
}
pub struct SegmentState {
    profile: SoakProfile,
    pub worker: Option<WorkerIdentity>,
    phase: Option<Phase>,
    pub rounds: u64,
    pub active_elapsed_ms: u64,
    pub terminal: Option<Terminal>,
    last_digest: Option<String>,
    baseline_values: Option<u64>,
}
impl SegmentState {
    pub fn new(profile: SoakProfile) -> Result<Self, String> {
        profile.validate()?;
        Ok(Self {
            profile,
            worker: None,
            phase: None,
            rounds: 0,
            active_elapsed_ms: 0,
            terminal: None,
            last_digest: None,
            baseline_values: None,
        })
    }
    pub fn accept(&mut self, frame: &Frame, pid: u32, birth: &str) -> Result<(), String> {
        let result = self.accept_frame(frame, pid, birth);
        if result.is_err() {
            self.terminal = Some(Terminal::Failed);
        }
        result
    }
    fn accept_frame(&mut self, frame: &Frame, pid: u32, birth: &str) -> Result<(), String> {
        if self.terminal.is_some()
            || frame.schema != "imap075-soak-frame-v1"
            || frame.pid != pid
            || frame.cohort != self.profile.cohort
            || frame.seed != self.profile.seed
            || frame.source_sha != self.profile.source_sha
            || frame.incarnation.is_empty()
            || frame.incarnation.len() > 128
            || frame.incarnation.chars().any(char::is_control)
            || birth.is_empty()
            || birth.len() > 256
            || frame.active_elapsed_ms < self.active_elapsed_ms
            || frame.active_elapsed_ms > self.profile.bounds.hard_ms
        {
            return Err("soak frame identity/runtime/progress mismatch".into());
        }
        let identity = WorkerIdentity {
            pid,
            platform_birth: birth.into(),
            incarnation: frame.incarnation.clone(),
        };
        if self
            .worker
            .as_ref()
            .is_some_and(|previous| previous != &identity)
        {
            return Err("worker replacement ends continuous segment".into());
        }
        match (&self.phase, &frame.phase) {
            (None, Phase::Ready) => {
                if frame.round != 0 || frame.active_elapsed_ms != 0 || frame.digest.is_some() {
                    return Err("invalid soak readiness".into());
                }
                self.worker = Some(identity);
            }
            (Some(Phase::Ready), Phase::Warmup) => {
                if frame.round != 0 || frame.active_elapsed_ms != 0 {
                    return Err("invalid soak warmup".into());
                }
            }
            (Some(Phase::Warmup), Phase::Active) => {
                if frame.round != 0 || frame.active_elapsed_ms != 0 {
                    return Err("invalid soak active cut".into());
                }
                self.baseline_values = frame.owners.values;
            }
            (Some(Phase::Active | Phase::Round), Phase::Round) => {
                if frame.round != self.rounds + 1
                    || frame.round > self.profile.bounds.maximum_rounds
                {
                    return Err("soak round sequence/bound".into());
                }
                self.rounds = frame.round;
            }
            (Some(Phase::Ready | Phase::Warmup | Phase::Active | Phase::Round), Phase::Drained) => {
                if frame.round != self.rounds || frame.digest != self.last_digest {
                    return Err("soak drain changed logical state".into());
                }
                self.terminal = Some(match frame.reason.as_deref() {
                    Some("duration_complete")
                        if self.rounds >= 2
                            && frame.active_elapsed_ms >= self.profile.bounds.active_ms =>
                    {
                        Terminal::Complete
                    }
                    Some("cancelled") => Terminal::Cancelled,
                    _ => return Err("incomplete soak drain".into()),
                });
            }
            _ => return Err("invalid soak phase transition".into()),
        }
        if frame.phase != Phase::Ready
            && !frame
                .digest
                .as_ref()
                .is_some_and(|v| crate::imap_qualification_075::hex(v, 64))
        {
            return Err("missing soak logical digest".into());
        }
        if frame.phase != Phase::Ready
            && frame.digest.as_deref() != Some(expected_digest(&self.profile, frame.round).as_str())
        {
            return Err("soak logical seed/position digest differs".into());
        }
        if frame.phase != Phase::Drained && frame.reason.is_some() {
            return Err("premature soak terminal reason".into());
        }
        self.validate_owners(frame)?;
        self.phase = Some(frame.phase.clone());
        self.active_elapsed_ms = frame.active_elapsed_ms;
        self.last_digest = frame.digest.clone();
        Ok(())
    }
    fn validate_owners(&self, frame: &Frame) -> Result<(), String> {
        let o = &frame.owners;
        match self.profile.cohort {
            Cohort::Reference => {
                if o.unit != "reference_owner_bytes"
                    || [
                        o.pending,
                        o.listeners,
                        o.streams,
                        o.resync,
                        o.queued,
                        o.correlations,
                    ]
                    .iter()
                    .any(|v| *v != Some(0))
                    || o.outcomes != Some(if frame.phase == Phase::Ready { 0 } else { 28 })
                    || o.values.is_none()
                    || frame.phase == Phase::Ready && o.values != Some(0)
                    || matches!(frame.phase, Phase::Warmup | Phase::Active | Phase::Round)
                        && o.values == Some(0)
                    || matches!(frame.phase, Phase::Round) && o.values != self.baseline_values
                {
                    return Err("reference quiescent owner leak".into());
                }
            }
            Cohort::Native => {
                let closed = matches!(frame.phase, Phase::Ready | Phase::Drained);
                if o.unit != "native_fixture_counts"
                    || o.values != Some(u64::from(!closed))
                    || o.listeners != Some(u64::from(frame.phase != Phase::Drained))
                    || o.queued != Some(0)
                    || o.correlations != Some(0)
                    || [o.outcomes, o.pending, o.streams, o.resync]
                        .iter()
                        .any(Option::is_some)
                {
                    return Err("native quiescent fixture owner leak".into());
                }
            }
            Cohort::Daemon => {
                if o.unit != "daemon_protocol_only"
                    || o.values != Some(0)
                    || [
                        o.outcomes,
                        o.pending,
                        o.listeners,
                        o.streams,
                        o.resync,
                        o.queued,
                        o.correlations,
                    ]
                    .iter()
                    .any(Option::is_some)
                {
                    return Err("daemon owner scope fabricated".into());
                }
            }
        }
        Ok(())
    }
}

/// The process remains live behind the Ready/start handshake while birth identity is observed.
pub fn process_identity(pid: u32) -> Result<String, Box<dyn Error>> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, FILETIME},
            System::Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
        };
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            return Err(std::io::Error::last_os_error().into());
        }
        let mut creation = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let (mut exit, mut kernel, mut user) = (creation, creation, creation);
        let ok =
            unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) };
        // Read a failed GetProcessTimes error before CloseHandle can change it.
        let result = windows_process_birth(
            ok != 0,
            creation.dwHighDateTime,
            creation.dwLowDateTime,
            std::io::Error::last_os_error,
        );
        unsafe { CloseHandle(handle) };
        result
    }
    #[cfg(target_os = "linux")]
    {
        let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
        let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id")?;
        Ok(format!("linux-{}-{}", boot.trim(), linux_start(&stat)?))
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = pid;
        Err("platform birth identity unavailable".into())
    }
}
pub fn linux_start(stat: &str) -> Result<u64, Box<dyn Error>> {
    if stat.len() > 8192 {
        return Err("process stat bound".into());
    }
    let end = stat.rfind(')').ok_or("invalid process stat")?;
    Ok(stat[end + 1..]
        .split_whitespace()
        .nth(19)
        .ok_or("missing process birth")?
        .parse()?)
}
fn nonce() -> String {
    format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    )
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let mut f = OpenOptions::new().create_new(true).write(true).open(path)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    Ok(())
}
fn publish_marker(control: &Path, name: &str) -> Result<(), Box<dyn Error>> {
    marker_name(name)?;
    let mut pending = tempfile::NamedTempFile::new_in(control)?;
    pending.write_all(format!("{name}-v1").as_bytes())?;
    pending.as_file().sync_all()?;
    complete_marker(
        control,
        name,
        pending
            .persist_noclobber(control.join(name))
            .map(|_| ())
            .map_err(|e| e.error),
    )
}
fn marker_name(name: &str) -> Result<(), Box<dyn Error>> {
    if !["start", "cancel"].contains(&name) {
        return Err("unknown soak control marker".into());
    }
    Ok(())
}
fn complete_marker(
    control: &Path,
    name: &str,
    result: std::io::Result<()>,
) -> Result<(), Box<dyn Error>> {
    match result {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if !marker(control, name)? {
                return Err("soak marker disappeared during publication".into());
            }
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

pub fn state_digest(value: &[u8]) -> String {
    let mut bytes = vec![0, 0, 0, 1, 97];
    bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
    bytes.extend_from_slice(value);
    bytes.push(0);
    crate::imap_qualification_075::digest(&bytes)
}
pub fn expected_digest(profile: &SoakProfile, round: u64) -> String {
    if profile.cohort == Cohort::Daemon {
        return crate::imap_qualification_075::digest(&[]);
    }
    let position = (profile.bounds.warmup_rounds + round)
        * if profile.cohort == Cohort::Native {
            2
        } else {
            1
        };
    state_digest(&(profile.seed ^ position).to_be_bytes())
}
pub(crate) fn worker_arguments(profile: &SoakProfile, control: &Path) -> Vec<String> {
    vec![
        profile.bounds.active_ms.to_string(),
        profile.bounds.period_ms.to_string(),
        profile.bounds.maximum_rounds.to_string(),
        profile.bounds.warmup_rounds.to_string(),
        profile.bounds.startup_ms.to_string(),
        profile.seed.to_string(),
        profile.source_sha.clone(),
        control.to_string_lossy().into_owned(),
    ]
}
pub fn marker(control: &Path, name: &str) -> Result<bool, Box<dyn Error>> {
    marker_using(control, name, fs::File::open)
}
fn marker_using(
    control: &Path,
    name: &str,
    open: impl FnOnce(std::path::PathBuf) -> std::io::Result<fs::File>,
) -> Result<bool, Box<dyn Error>> {
    marker_name(name)?;
    use std::io::Read;
    let path = control.join(name);
    let input = match open(path) {
        Ok(input) => input,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.into()),
    };
    let mut bytes = Vec::new();
    input.take(17).read_to_end(&mut bytes)?;
    if bytes != format!("{name}-v1").as_bytes() {
        return Err("invalid bounded soak control marker".into());
    }
    Ok(true)
}
fn plain_control_component(redirected: bool) -> Result<(), Box<dyn Error>> {
    if redirected {
        return Err("soak control symlink/reparse refused".into());
    }
    Ok(())
}
fn control_scope(root: &Path, canonical: &Path) -> Result<(), Box<dyn Error>> {
    if !canonical.starts_with(root) {
        return Err("soak control canonical escape".into());
    }
    Ok(())
}
fn prepare_control(root: &Path, control: &Path) -> Result<(), Box<dyn Error>> {
    let relative = control.strip_prefix(root)?.to_path_buf();
    let root = root.canonicalize()?;
    if relative
        .components()
        .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err("soak control path escapes root".into());
    }
    let mut parent = root.clone();
    for component in relative.components() {
        parent.push(component);
        if parent.exists() {
            let metadata = fs::symlink_metadata(&parent)?;
            let redirected = metadata.file_type().is_symlink();
            #[cfg(windows)]
            let redirected = {
                use std::os::windows::fs::MetadataExt;
                redirected || metadata.file_attributes() & 0x400 != 0
            };
            plain_control_component(redirected)?;
            control_scope(&root, &parent.canonicalize()?)?;
        }
    }
    fs::create_dir(control)?;
    control_scope(&root, &control.canonicalize()?)?;
    Ok(())
}
enum ProtocolEnd {
    Worker {
        line_complete: bool,
        terminal: Option<Terminal>,
        cancelled_before_active: bool,
    },
    Daemon {
        terminal: Option<Terminal>,
    },
}
/// Classify protocol completion separately from the observed process outcome.
/// This function neither executes processes nor publishes evidence.
fn finalize_process(
    report: &mut SoakReport,
    result: Result<process::ProcessOutput, process::ProcessFailure>,
    protocol: ProtocolEnd,
    cancelled: bool,
) {
    match result {
        Ok(output) => {
            report.process_reaped = output.direct_child_reaped;
            report.remaining_members = Some(output.remaining_live_members);
            report.containment = output.scope.into();
            report.terminal = Terminal::Failed;
            match protocol {
                ProtocolEnd::Worker {
                    line_complete,
                    terminal,
                    cancelled_before_active,
                } => {
                    if !line_complete {
                        report.detail = "incomplete soak protocol line".into();
                    } else if let Some(terminal) = terminal {
                        report.terminal = terminal;
                    } else if cancelled_before_active {
                        report.terminal = Terminal::Cancelled;
                        report.detail = "cancelled before active admission".into();
                    } else {
                        report.detail = "worker exited without drain".into();
                    }
                }
                ProtocolEnd::Daemon { terminal } => {
                    report.terminal = terminal.unwrap_or(Terminal::Failed);
                    if report.terminal == Terminal::Failed {
                        report.detail = "daemon exited before drain".into();
                    }
                }
            }
        }
        Err(error) => {
            report.process_reaped = error.direct_child_reaped;
            report.remaining_members = error.remaining_live_members;
            report.detail = error.to_string();
            report.terminal = if error.kind == process::FailureKind::Timeout {
                Terminal::TimedOut
            } else if cancelled && report.detail.contains("cancellation drain") {
                Terminal::Cancelled
            } else {
                Terminal::Failed
            };
        }
    }
}
fn finalize_segment(
    report: &mut SoakReport,
    timeout_drain: bool,
    artifacts: Result<(), Box<dyn Error>>,
    cleanup_complete: bool,
    cleanup_detail: &str,
) {
    if timeout_drain && matches!(report.terminal, Terminal::Complete | Terminal::Cancelled) {
        report.terminal = Terminal::TimedOut;
        report.detail = "hard deadline requested bounded drain".into();
    }
    if let Err(error) = artifacts {
        report.terminal = Terminal::Failed;
        report.detail = error.to_string();
    }
    if !cleanup_complete {
        report.terminal = Terminal::Failed;
        report.detail.push_str(cleanup_detail);
    }
}

struct WorkerFrames {
    parsed: usize,
    state: SegmentState,
    birth: Option<String>,
}
impl WorkerFrames {
    fn accept(
        &mut self,
        observation: &process::ProcessObservation<'_>,
        control: &Path,
        segment: &str,
        mut read_birth: impl FnMut(u32) -> Result<String, Box<dyn Error>>,
    ) -> Result<(), String> {
        if self.parsed > observation.stdout.len() {
            return Err("soak stdout observation regressed".into());
        }
        while let Some(end) = observation.stdout[self.parsed..]
            .iter()
            .position(|b| *b == b'\n')
        {
            let line = &observation.stdout[self.parsed..self.parsed + end];
            if line.len() > 4096 {
                return Err("soak frame byte bound".into());
            }
            self.parsed += end + 1;
            let frame: Frame = serde_json::from_slice(line).map_err(|e| e.to_string())?;
            if self.birth.is_none() {
                if observation.root_exited {
                    return Err("soak worker died before owned readiness".into());
                }
                self.birth = Some(read_birth(observation.pid).map_err(|e| e.to_string())?);
            }
            self.state.accept(
                &frame,
                observation.pid,
                self.birth.as_deref().expect("observed process birth"),
            )?;
            if frame.phase == Phase::Ready {
                write_new(
                    &control.join("identity.json"),
                    &serde_json::to_vec(&IdentityCheckpoint {
                        schema: "imap075-soak-identity-v1".into(),
                        segment: segment.into(),
                        worker: self.state.worker.clone().expect("accepted worker identity"),
                    })
                    .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                publish_marker(control, "start").map_err(|e| e.to_string())?;
            }
            println!(
                "imap-soak: {:?} {:?} round={} active_ms={}",
                self.state.profile.cohort, frame.phase, frame.round, frame.active_elapsed_ms
            );
        }
        if observation.stdout.len() - self.parsed > 4096 {
            return Err("unterminated soak frame bound".into());
        }
        Ok(())
    }
}
struct WorkerControl<'a> {
    root: &'a Path,
    profile: &'a SoakProfile,
    control: &'a Path,
    cancel: &'a Cancellation,
    started: Instant,
    cancellation_at: Option<Instant>,
    timeout_drain: bool,
    last_binding: Instant,
}
impl WorkerControl<'_> {
    fn observe(
        &mut self,
        observation: &process::ProcessObservation<'_>,
        ready: bool,
        mut clock: impl FnMut() -> Instant,
        check_bindings: impl FnOnce(&SoakProfile, &Path) -> Result<(), Box<dyn Error>>,
    ) -> Result<(), String> {
        if !ready
            && clock().saturating_duration_since(self.started)
                > Duration::from_millis(self.profile.bounds.startup_ms)
        {
            return Err("soak readiness deadline".into());
        }
        let requested = marker(self.control, "cancel").map_err(|e| e.to_string())?;
        let budget_drain = observation.elapsed
            >= Duration::from_millis(self.profile.bounds.hard_ms - self.profile.bounds.drain_ms);
        if (self.cancel.is_cancelled() || requested || budget_drain)
            && !observation.root_exited
            && self.cancellation_at.is_none()
        {
            if !requested {
                publish_marker(self.control, "cancel").map_err(|e| e.to_string())?;
            }
            self.timeout_drain = budget_drain && !self.cancel.is_cancelled() && !requested;
            self.cancellation_at = Some(clock());
        }
        if self.cancellation_at.is_some_and(|at| {
            clock().saturating_duration_since(at)
                > Duration::from_millis(self.profile.bounds.drain_ms)
        }) && !observation.root_exited
        {
            return Err("soak cancellation drain deadline".into());
        }
        refresh_soak_bindings(
            self.root,
            self.profile,
            &mut self.last_binding,
            &mut clock,
            check_bindings,
        )?;
        Ok(())
    }
}
/// Code constructs the command. The profile/checkpoint can never choose a program or arguments.
pub fn execute_worker(
    root: &Path,
    profile: SoakProfile,
    command: Command,
    control: &Path,
    cancel: Cancellation,
    parent: Option<String>,
) -> Result<SoakReport, Box<dyn Error>> {
    execute_worker_using(root, profile, command, control, cancel, parent, write_new)
}
fn execute_worker_using(
    root: &Path,
    profile: SoakProfile,
    mut command: Command,
    control: &Path,
    cancel: Cancellation,
    parent: Option<String>,
    mut write: impl FnMut(&Path, &[u8]) -> Result<(), Box<dyn Error>>,
) -> Result<SoakReport, Box<dyn Error>> {
    profile.validate()?;
    profile.check_artifacts(root)?;
    prepare_control(root, control)?;
    let segment = nonce();
    let controller = WorkerIdentity {
        pid: std::process::id(),
        platform_birth: process_identity(std::process::id())?,
        incarnation: nonce(),
    };
    let mut report = SoakReport {
        schema: "imap075-soak-report-v1".into(),
        scope: "continuous_local_development_segment".into(),
        production_enabled: false,
        profile: profile.clone(),
        segment,
        parent_segment: parent,
        controller,
        worker: None,
        terminal: Terminal::Failed,
        rounds: 0,
        active_elapsed_ms: 0,
        transcript_sha256: crate::imap_qualification_075::digest(&[]),
        transcript_bytes: 0,
        process_reaped: false,
        remaining_members: None,
        containment: String::new(),
        detail: String::new(),
    };
    write(
        &control.join("checkpoint.json"),
        &serde_json::to_vec(&report)?,
    )?;
    command.args(worker_arguments(&profile, control));
    let mut stdout = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(control.join("stdout.jsonl"))?;
    let mut stderr = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(control.join("stderr.txt"))?;
    let (mut saved, mut saved_err) = (0usize, 0usize);
    let mut frames = WorkerFrames {
        parsed: 0,
        state: SegmentState::new(profile.clone())?,
        birth: None,
    };
    let started = Instant::now();
    let mut controls = WorkerControl {
        root,
        profile: &profile,
        control,
        cancel: &cancel,
        started,
        cancellation_at: None,
        timeout_drain: false,
        last_binding: started,
    };
    let result = process::execute_managed(
        command,
        &[],
        ProcessBounds {
            timeout: Duration::from_millis(profile.bounds.hard_ms),
            cleanup_timeout: Duration::from_secs(5),
            stdout_bytes: 8 * 1024 * 1024,
            stderr_bytes: 1024 * 1024,
            memory_bytes: WORKER_MEMORY_BYTES,
            ..Default::default()
        },
        ExecutionPolicy::Soak,
        |o| {
            stdout
                .write_all(&o.stdout[saved..])
                .map_err(|e| e.to_string())?;
            saved = o.stdout.len();
            stderr
                .write_all(&o.stderr[saved_err..])
                .map_err(|e| e.to_string())?;
            saved_err = o.stderr.len();
            frames.accept(&o, control, &report.segment, process_identity)?;
            controls.observe(
                &o,
                frames.state.phase.is_some(),
                Instant::now,
                SoakProfile::check_artifacts,
            )
        },
    );
    stdout.sync_all()?;
    stderr.sync_all()?;
    report.worker = frames.state.worker;
    report.rounds = frames.state.rounds;
    report.active_elapsed_ms = frames.state.active_elapsed_ms;
    report.transcript_bytes = saved as u64;
    report.transcript_sha256 =
        crate::imap_qualification_075::digest(&fs::read(control.join("stdout.jsonl"))?);
    finalize_process(
        &mut report,
        result,
        ProtocolEnd::Worker {
            line_complete: frames.parsed == saved,
            terminal: frames.state.terminal,
            cancelled_before_active: controls.cancellation_at.is_some() && frames.state.rounds == 0,
        },
        controls.cancellation_at.is_some(),
    );
    let cleanup_complete = report.remaining_members == Some(0) && report.process_reaped;
    finalize_segment(
        &mut report,
        controls.timeout_drain,
        profile.check_artifacts(root),
        cleanup_complete,
        "; incomplete process cleanup",
    );
    write(
        &control.join("report.json"),
        &serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}
#[cfg(test)]
#[path = "../tests/support/imap_soak_completion_075.rs"]
mod completion_tests;
#[cfg(test)]
#[path = "../tests/support/imap_soak_control_075.rs"]
mod control_tests;
#[cfg(test)]
#[path = "../tests/support/imap_soak_frames_075.rs"]
mod frame_tests;
pub fn validate_report(report: &SoakReport) -> Result<(), Box<dyn Error>> {
    report.profile.validate()?;
    if report.schema != "imap075-soak-report-v1"
        || report.production_enabled
        || report.scope != "continuous_local_development_segment"
        || report.segment.is_empty()
        || report.segment.len() > 128
        || !crate::imap_qualification_075::hex(&report.transcript_sha256, 64)
        || report.transcript_bytes > 8 * 1024 * 1024
        || report.rounds > report.profile.bounds.maximum_rounds
        || report.active_elapsed_ms > report.profile.bounds.hard_ms
        || report.controller.platform_birth.is_empty()
        || report.controller.pid == 0
        || report.controller.incarnation.is_empty()
        || report.worker.as_ref().is_some_and(|w| {
            w.pid == 0
                || w.platform_birth.is_empty()
                || w.platform_birth.len() > 256
                || w.incarnation.is_empty()
                || w.incarnation.len() > 128
        })
        || report.terminal == Terminal::Complete
            && (report.worker.is_none()
                || !report.process_reaped
                || report.remaining_members != Some(0)
                || report.rounds < 2
                || report.active_elapsed_ms < report.profile.bounds.active_ms
                || !report.detail.is_empty())
    {
        return Err("invalid/incomplete continuous soak report".into());
    }
    Ok(())
}
pub fn resume_parent(report: &SoakReport, profile: &SoakProfile) -> Result<String, Box<dyn Error>> {
    validate_report(report)?;
    profile.validate()?;
    let mut previous = report.profile.clone();
    let mut current = profile.clone();
    previous.bounds.host_remaining_ms = 0;
    current.bounds.host_remaining_ms = 0;
    if previous != current {
        return Err("soak restart source/runtime/profile/artifact mismatch".into());
    }
    if !report.process_reaped {
        if let Some(worker) = &report.worker {
            if identity_is_live(worker)? {
                return Err(
                    "previous worker still live; ownership cannot transfer across controllers"
                        .into(),
                );
            }
        } else {
            return Err("previous worker cleanup/identity unknown".into());
        }
    }
    Ok(report.segment.clone())
}
fn identity_is_live(identity: &WorkerIdentity) -> Result<bool, Box<dyn Error>> {
    identity_is_live_using(identity, process_identity, cfg!(windows))
}
fn identity_is_live_using(
    identity: &WorkerIdentity,
    read: impl FnOnce(u32) -> Result<String, Box<dyn Error>>,
    windows: bool,
) -> Result<bool, Box<dyn Error>> {
    match read(identity.pid) {
        Ok(birth) => Ok(birth == identity.platform_birth),
        Err(error) => {
            if error.downcast_ref::<std::io::Error>().is_some_and(|e| {
                e.kind() == std::io::ErrorKind::NotFound
                    || windows && matches!(e.raw_os_error(), Some(87 | 1168))
            }) {
                Ok(false)
            } else {
                Err(error)
            }
        }
    }
}

pub fn current_runtime(cohort: Cohort) -> Result<String, Box<dyn Error>> {
    runtime_using(
        cohort,
        || Ok(crate::imap_api_differential::java_executable().canonicalize()?),
        runtime_observation,
        crate::imap_daemon_probe_075::binary_digest,
    )
}
type RuntimeObservation = (Vec<u8>, Vec<u8>);
fn runtime_observation(command: Command) -> Result<RuntimeObservation, Box<dyn Error>> {
    let output = process::execute(
        command,
        &[],
        ProcessBounds {
            stdout_bytes: 8192,
            stderr_bytes: 8192,
            ..Default::default()
        },
    )?;
    Ok((output.stdout, output.stderr))
}
fn runtime_using(
    cohort: Cohort,
    mut launcher: impl FnMut() -> Result<std::path::PathBuf, Box<dyn Error>>,
    observe: impl FnOnce(Command) -> Result<RuntimeObservation, Box<dyn Error>>,
    hash: impl FnOnce(&Path) -> Result<String, Box<dyn Error>>,
) -> Result<String, Box<dyn Error>> {
    let command = if cohort == Cohort::Native {
        let mut c = Command::new(launcher()?);
        c.args(["-Xms16m", "-Xmx64m", "--version"]);
        c
    } else {
        let mut c = Command::new("rustc");
        c.arg("--version");
        c
    };
    let (mut bytes, stderr) = observe(command)?;
    bytes.extend(stderr);
    let text = String::from_utf8(bytes)?;
    let launcher = if cohort == Cohort::Native {
        hash(&launcher()?)?
    } else {
        "static-worker-artifact".into()
    };
    Ok(format!(
        "{}-{};{};launcher={launcher}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        text.trim()
    ))
}
fn binary(root: &Path, name: &str) -> std::path::PathBuf {
    root.join("target/debug")
        .join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
}
pub fn trusted_profile(
    root: &Path,
    source: &str,
    cohort: Cohort,
    bounds: SoakBounds,
) -> Result<SoakProfile, Box<dyn Error>> {
    trusted_profile_using(
        root,
        source,
        cohort,
        bounds,
        |root| {
            crate::imap_extended_receipt_075::artifact(
                root,
                &std::env::current_exe()?,
                512 * 1024 * 1024,
            )
        },
        current_runtime,
    )
}
fn trusted_profile_using(
    root: &Path,
    source: &str,
    cohort: Cohort,
    bounds: SoakBounds,
    tooling: impl FnOnce(&Path) -> Result<crate::imap_extended_receipt_075::Artifact, Box<dyn Error>>,
    runtime: impl FnOnce(Cohort) -> Result<String, Box<dyn Error>>,
) -> Result<SoakProfile, Box<dyn Error>> {
    let paths = match cohort {
        Cohort::Reference => vec![binary(root, "imap-soak-worker-075")],
        Cohort::Daemon => vec![binary(root, "hydracache-server")],
        Cohort::Native => vec![
            root.join("tests/java-imap-benchmark/target/java-imap-benchmark-0.75.0-SNAPSHOT.jar"),
            root.join(
                "tests/java-imap-hazelcast-075/target/java-imap-hazelcast-075-0.75.0-SNAPSHOT.jar",
            ),
            root.join("tests/java-imap-hazelcast-075/target/dependencies/hazelcast-5.5.0.jar"),
        ],
    };
    let mut artifacts = Vec::new();
    for path in paths {
        artifacts.push(BoundArtifact {
            path: path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/"),
            sha256: crate::imap_daemon_probe_075::binary_digest(&path)?,
        });
    }
    if cohort == Cohort::Native && artifacts[2].sha256 != crate::imap_pre074_075::HAZELCAST_SHA256 {
        return Err("soak native pin differs".into());
    }
    let tooling = tooling(root)?;
    artifacts.push(BoundArtifact {
        path: tooling.path,
        sha256: tooling.sha256,
    });
    let profile = SoakProfile {
        schema: "imap075-soak-profile-v1".into(),
        source_sha: source.into(),
        cohort,
        seed: 117,
        bounds,
        runtime: runtime(cohort)?,
        artifacts,
    };
    profile.validate()?;
    Ok(profile)
}
pub fn run_prepared(
    root: &Path,
    profile: SoakProfile,
    control: &Path,
    cancel: Cancellation,
    parent: Option<String>,
) -> Result<SoakReport, Box<dyn Error>> {
    match profile.cohort {
        Cohort::Daemon => execute_daemon(
            root,
            profile,
            &binary(root, "hydracache-server"),
            control,
            cancel,
            parent,
        ),
        Cohort::Reference => execute_worker(
            root,
            profile,
            Command::new(binary(root, "imap-soak-worker-075")),
            control,
            cancel,
            parent,
        ),
        Cohort::Native => {
            let mut command =
                Command::new(crate::imap_api_differential::java_executable().canonicalize()?);
            command.arg("-Xmx128m").arg(format!("-Djdk.net.unixdomain.tmpdir={}",root.join("tests/java-imap-hazelcast-075/target").display()))
                .arg("-cp").arg(std::env::join_paths([root.join("tests/java-imap-benchmark/target/java-imap-benchmark-0.75.0-SNAPSHOT.jar"),root.join("tests/java-imap-hazelcast-075/target/java-imap-hazelcast-075-0.75.0-SNAPSHOT.jar"),root.join("tests/java-imap-hazelcast-075/target/dependencies/hazelcast-5.5.0.jar")])?)
                .arg("io.hydracache.imap.hazelcast.NativeSoakWorker075");
            execute_worker(root, profile, command, control, cancel, parent)
        }
    }
}
pub fn check_saved(root: &Path, path: &Path) -> Result<SoakReport, Box<dyn Error>> {
    check_saved_using(root, path, &ReplayBindings::real())
}
fn check_saved_using(
    root: &Path,
    path: &Path,
    bindings: &ReplayBindings,
) -> Result<SoakReport, Box<dyn Error>> {
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    let bytes = crate::imap_qualification_075::read_relative(root, &relative, 256 * 1024)?;
    let report: SoakReport = serde_json::from_slice(&bytes)?;
    validate_report(&report)?;
    if report.profile.source_sha != (bindings.source)(root)? {
        return Err("soak archived source differs".into());
    }
    (bindings.artifacts)(&report.profile, root)?;
    let current = (bindings.profile)(
        root,
        &report.profile.source_sha,
        report.profile.cohort,
        report.profile.bounds.clone(),
    )?;
    if current != report.profile {
        return Err("soak producer/artifact profile differs from current trusted producer".into());
    }
    let transcript_path = Path::new(&relative)
        .parent()
        .ok_or("soak receipt parent missing")?
        .join("stdout.jsonl")
        .to_string_lossy()
        .replace('\\', "/");
    let transcript =
        crate::imap_qualification_075::read_relative(root, &transcript_path, 8 * 1024 * 1024)?;
    if transcript.len() as u64 != report.transcript_bytes
        || crate::imap_qualification_075::digest(&transcript) != report.transcript_sha256
    {
        return Err("soak transcript binding differs".into());
    }
    if report.terminal == Terminal::Complete {
        let identity = report
            .worker
            .as_ref()
            .ok_or("soak worker identity missing")?;
        replay_saved_progress(
            &report.profile,
            identity,
            &transcript,
            report.rounds,
            report.active_elapsed_ms,
        )?;
    }
    Ok(report)
}
pub fn check_checkpoint(root: &Path, path: &Path) -> Result<SoakReport, Box<dyn Error>> {
    check_checkpoint_using(root, path, &ReplayBindings::real())
}
fn check_checkpoint_using(
    root: &Path,
    path: &Path,
    bindings: &ReplayBindings,
) -> Result<SoakReport, Box<dyn Error>> {
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    if Path::new(&relative).file_name().and_then(|v| v.to_str()) != Some("checkpoint.json") {
        return Err("fixed controller checkpoint name required".into());
    }
    let mut report: SoakReport = serde_json::from_slice(
        &crate::imap_qualification_075::read_relative(root, &relative, 256 * 1024)?,
    )?;
    validate_report(&report)?;
    if report.profile.source_sha != (bindings.source)(root)? || (bindings.live)(&report.controller)?
    {
        return Err("checkpoint source differs or previous controller still lives".into());
    }
    let current = (bindings.profile)(
        root,
        &report.profile.source_sha,
        report.profile.cohort,
        report.profile.bounds.clone(),
    )?;
    if current != report.profile {
        return Err("checkpoint runtime/artifact/profile differs".into());
    }
    let directory = Path::new(&relative)
        .parent()
        .ok_or("missing checkpoint parent")?;
    let identity: IdentityCheckpoint =
        serde_json::from_slice(&crate::imap_qualification_075::read_relative(
            root,
            &directory
                .join("identity.json")
                .to_string_lossy()
                .replace('\\', "/"),
            65536,
        )?)?;
    if identity.schema != "imap075-soak-identity-v1"
        || identity.segment != report.segment
        || (bindings.live)(&identity.worker)?
    {
        return Err("checkpoint identity differs or previous worker still lives".into());
    }
    let bytes = crate::imap_qualification_075::read_relative(
        root,
        &directory
            .join("stdout.jsonl")
            .to_string_lossy()
            .replace('\\', "/"),
        8 * 1024 * 1024,
    )?;
    if !bytes.ends_with(b"\n") {
        return Err("corrupt/partial controller progress checkpoint".into());
    }
    let mut progress = SegmentState::new(report.profile.clone())?;
    for line in bytes.split(|b| *b == b'\n').filter(|line| !line.is_empty()) {
        if line.len() > 4096 {
            return Err("checkpoint frame bound".into());
        }
        progress.accept(
            &serde_json::from_slice(line)?,
            identity.worker.pid,
            &identity.worker.platform_birth,
        )?;
    }
    report.worker = Some(identity.worker);
    report.rounds = progress.rounds;
    report.active_elapsed_ms = progress.active_elapsed_ms;
    report.transcript_bytes = bytes.len() as u64;
    report.transcript_sha256 = crate::imap_qualification_075::digest(&bytes);
    report.terminal = Terminal::Failed;
    report.detail = "controller restarted; prior segment incomplete; no duration stitching".into();
    Ok(report)
}
pub fn preparation_smoke(
    root: &Path,
    source: &str,
    directory: &Path,
) -> Result<Vec<crate::imap_extended_receipt_075::Artifact>, Box<dyn Error>> {
    preparation_smoke_using(root, source, directory, &mut ActualSoakCommands)
}
fn preparation_smoke_using(
    root: &Path,
    source: &str,
    directory: &Path,
    commands: &mut impl SoakCommands,
) -> SoakResult<Vec<crate::imap_extended_receipt_075::Artifact>> {
    let mut reports = Vec::new();
    for cohort in [Cohort::Reference, Cohort::Native, Cohort::Daemon] {
        let mut bounds = SoakBounds::smoke();
        bounds.host_remaining_ms =
            crate::imap_build_process_075::remaining_budget()?.as_millis() as u64;
        let profile = commands.profile(root, source, cohort, bounds)?;
        let control = directory.join(format!("soak-{cohort:?}").to_lowercase());
        let report = commands.execute(root, profile, &control, None)?;
        validate_report(&report)?;
        commands.completed(&report, "preparation soak incomplete")?;
        commands.check(root, &control.join("report.json"))?;
        reports.push(commands.artifact(root, &control.join("report.json"))?);
    }
    Ok(reports)
}
pub fn check_manual_workflow(root: &Path) -> Result<(), Box<dyn Error>> {
    use serde_yaml::Value;
    let bytes = crate::imap_qualification_075::read_relative(
        root,
        ".github/workflows/imap-soak-075.yml",
        16384,
    )?;
    let workflow: Value = serde_yaml::from_slice(&bytes)?;
    let names = |value: &Value| -> Result<Vec<String>, Box<dyn Error>> {
        value
            .as_mapping()
            .ok_or("workflow mapping required")?
            .keys()
            .map(|v| {
                v.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| "workflow string key required".into())
            })
            .collect()
    };
    if names(&workflow["on"])? != ["workflow_dispatch"]
        || workflow["permissions"]["contents"].as_str() != Some("read")
        || names(&workflow["permissions"])?.len() != 1
        || workflow["concurrency"]["group"].as_str()
            != Some("imap075-soak-${{ github.workflow }}-${{ github.ref }}")
        || workflow["concurrency"]["cancel-in-progress"].as_bool() != Some(false)
        || names(&workflow["jobs"])? != ["hour"]
    {
        return Err("manual soak trigger/permissions/topology weakened".into());
    }
    let inputs = &workflow["on"]["workflow_dispatch"]["inputs"];
    if names(inputs)? != ["cohort"]
        || inputs["cohort"]["required"].as_bool() != Some(true)
        || inputs["cohort"]["type"].as_str() != Some("choice")
        || inputs["cohort"]["default"].as_str() != Some("reference")
        || inputs["cohort"]["options"] != serde_yaml::to_value(["reference", "native", "daemon"])?
    {
        return Err("manual soak cohort inputs weakened".into());
    }
    let job = &workflow["jobs"]["hour"];
    if job["if"].as_str()!=Some("github.repository == 'javaquasar/hydracache' && github.ref == 'refs/heads/feat/0.75-distributed-imap-foundation' && github.event_name == 'workflow_dispatch'")
        || job["runs-on"]!=serde_yaml::to_value(["self-hosted","linux","x64","hydracache-imap075"])? || job["timeout-minutes"].as_u64()!=Some(120) || job["env"]["CARGO_BUILD_JOBS"].as_u64()!=Some(2) {return Err("manual soak trusted source/runner/budget weakened".into());}
    let steps = job["steps"]
        .as_sequence()
        .ok_or("manual soak steps required")?;
    if steps.len() != 7
        || steps[1]["uses"].as_str()
            != Some("actions/checkout@fbc6f3992d24b796d5a048ff273f7fcc4a7b6c09")
        || steps[1]["with"]["fetch-depth"].as_u64() != Some(0)
        || steps[1]["with"]["persist-credentials"].as_bool() != Some(false)
        || steps[2]["uses"].as_str()
            != Some("actions/setup-java@cf277c60eb25467037889841efdb72551f06f6c3")
        || steps[2]["with"]["distribution"].as_str() != Some("temurin")
        || steps[2]["with"]["java-version"].as_u64() != Some(21)
    {
        return Err("manual soak actions/history/transfer policy weakened".into());
    }
    let expected=[(0,"Host eligibility budget",None,r#"echo "IMAP075_HOST_DEADLINE=$(( $(date +%s) + 115 * 60 ))" >> "$GITHUB_ENV"
echo "HYDRACACHE_IMAP075_BUILD_DEADLINE_MS=$(( ($(date +%s) + 40 * 60) * 1000 ))" >> "$GITHUB_ENV""#),
        (3,"Install exact Rust toolchain",Some(10),"rustup toolchain install 1.94.0 --profile minimal\nrustup default 1.94.0"),
        (4,"Prepare bounded development artifacts",Some(43),"cargo xtask imap-pre074-tools --release 0.75"),
        (5,"Continuous one-hour soak",Some(75),r#"set -euo pipefail
case "$COHORT" in reference|native|daemon) ;; *) exit 2 ;; esac
remaining=$(( IMAP075_HOST_DEADLINE - $(date +%s) ))
cargo xtask imap-soak --release 0.75 --cohort "$COHORT" --profile hour --host-budget-seconds "$remaining" --output "target/imap-075/hour-${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}""#),
        (6,"Local archive location",None,"echo 'Reports remain in the owned target/imap-075/hour-* directory on this runner; no automatic artifact transfer is configured.'")];
    for (index, name, timeout, command) in expected {
        if steps[index]["name"].as_str() != Some(name)
            || steps[index]["timeout-minutes"].as_u64() != timeout
            || steps[index]["run"].as_str().map(str::trim) != Some(command)
            || steps[index]["continue-on-error"].as_bool() == Some(true)
        {
            return Err("manual soak command/budget weakened".into());
        }
    }
    if steps[5]["env"]["COHORT"].as_str() != Some("${{ inputs.cohort }}")
        || steps[6]["if"].as_str() != Some("always()")
    {
        return Err("manual soak input/diagnostic wiring weakened".into());
    }
    Ok(())
}
pub fn request_cancel(root: &Path, checkpoint: &Path) -> Result<(), Box<dyn Error>> {
    let relative = checkpoint
        .strip_prefix(root)
        .unwrap_or(checkpoint)
        .to_string_lossy()
        .replace('\\', "/");
    if !relative.starts_with("target/imap-075/")
        || Path::new(&relative).file_name().and_then(|v| v.to_str()) != Some("checkpoint.json")
    {
        return Err("owned soak checkpoint path required for cancellation".into());
    }
    let report: SoakReport = serde_json::from_slice(
        &crate::imap_qualification_075::read_relative(root, &relative, 256 * 1024)?,
    )?;
    validate_report(&report)?;
    if !identity_is_live(&report.controller)? {
        return Err("soak controller no longer lives".into());
    }
    let control = root
        .join(&relative)
        .parent()
        .ok_or("missing owned control parent")?
        .to_path_buf();
    if !marker(&control, "cancel")? {
        publish_marker(&control, "cancel")?;
    }
    Ok(())
}
pub fn run_cancel(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.len() != 4 || args[0] != "--release" || args[1] != "0.75" || args[2] != "--checkpoint" {
        return Err("requires --release 0.75 --checkpoint <owned-path>".into());
    }
    request_cancel(&crate::doc_check::find_repo_root()?, Path::new(&args[3]))?;
    println!("imap-soak: cancellation requested for the owned segment");
    Ok(())
}
pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    run_using(args, &mut ActualSoakCommands)
}
fn run_using(args: Vec<String>, commands: &mut impl SoakCommands) -> SoakResult<()> {
    let mut fields = std::collections::BTreeMap::new();
    if !args.len().is_multiple_of(2) {
        return Err("soak options require values".into());
    }
    for pair in args.chunks_exact(2) {
        if ![
            "--release",
            "--cohort",
            "--profile",
            "--host-budget-seconds",
            "--output",
            "--resume",
        ]
        .contains(&pair[0].as_str())
            || fields.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("unknown/duplicate soak option".into());
        }
    }
    if fields.remove("--release") != Some("0.75") {
        return Err("requires --release 0.75".into());
    }
    let cohorts = match fields.remove("--cohort").unwrap_or("all") {
        "all" => vec![Cohort::Reference, Cohort::Native, Cohort::Daemon],
        "reference" => vec![Cohort::Reference],
        "native" => vec![Cohort::Native],
        "daemon" => vec![Cohort::Daemon],
        _ => return Err("unknown soak cohort".into()),
    };
    let mut bounds = SoakBounds::smoke();
    match fields.remove("--profile").unwrap_or("smoke") {
        "smoke" => {}
        "hour" => {
            bounds.active_ms = 3600000;
            bounds.period_ms = 1000;
            bounds.maximum_rounds = 4096;
        }
        "six-hours" => {
            bounds.active_ms = 21600000;
            bounds.period_ms = 10000;
            bounds.maximum_rounds = 4096;
        }
        "day" => {
            bounds.active_ms = 86400000;
            bounds.period_ms = 60000;
            bounds.maximum_rounds = 2048;
        }
        _ => return Err("unknown bounded soak profile".into()),
    }
    bounds.hard_ms = bounds.active_ms + bounds.startup_ms + bounds.drain_ms;
    let budget: u64 = fields
        .remove("--host-budget-seconds")
        .ok_or("explicit host budget required")?
        .parse()?;
    bounds.host_remaining_ms = budget.checked_mul(1000).ok_or("host budget overflow")?;
    bounds.validate()?;
    if cohorts.len() != 1 && bounds.active_ms > 200 {
        return Err("long profiles require one explicit cohort".into());
    }
    let root = commands.root()?;
    let source = commands.source(&root)?;
    let output = fields.remove("--output").ok_or("--output required")?;
    let directory = root.join(output);
    require_soak_output(output)?;
    prepare_control(&root, &directory)?;
    let parent = fields
        .remove("--resume")
        .map(|path| {
            if Path::new(path).file_name().and_then(|v| v.to_str()) == Some("checkpoint.json") {
                check_checkpoint(&root, Path::new(path))
            } else {
                check_saved(&root, Path::new(path))
            }
        })
        .transpose()?;
    let started = Instant::now();
    for cohort in cohorts {
        let mut current = bounds.clone();
        current.host_remaining_ms = current
            .host_remaining_ms
            .saturating_sub(started.elapsed().as_millis() as u64);
        let profile = commands.profile(&root, &source, cohort, current)?;
        let parent_segment = parent
            .as_ref()
            .map(|saved| resume_parent(saved, &profile))
            .transpose()?;
        let report = commands.execute(
            &root,
            profile,
            &directory.join(format!("{cohort:?}").to_lowercase()),
            parent_segment,
        )?;
        validate_report(&report)?;
        commands.completed(&report, &format!("soak {cohort:?}"))?;
        commands.check(
            &root,
            &directory
                .join(format!("{cohort:?}").to_lowercase())
                .join("report.json"),
        )?;
    }
    require_soak_source(&source, &commands.source(&root)?)?;
    println!("imap-soak: complete local segment(s); no production admission");
    Ok(())
}

pub fn execute_daemon(
    root: &Path,
    profile: SoakProfile,
    binary: &Path,
    control: &Path,
    cancel: Cancellation,
    parent: Option<String>,
) -> Result<SoakReport, Box<dyn Error>> {
    execute_daemon_using(root, profile, binary, control, cancel, parent, write_new)
}
fn execute_daemon_using(
    root: &Path,
    profile: SoakProfile,
    binary: &Path,
    control: &Path,
    cancel: Cancellation,
    parent: Option<String>,
    write: impl FnMut(&Path, &[u8]) -> Result<(), Box<dyn Error>>,
) -> Result<SoakReport, Box<dyn Error>> {
    execute_daemon_bound(
        root,
        profile,
        binary,
        control,
        cancel,
        parent,
        DaemonBindings {
            write,
            clock: Instant::now,
            check: SoakProfile::check_artifacts,
        },
    )
}
fn execute_daemon_bound<W, C>(
    root: &Path,
    profile: SoakProfile,
    binary: &Path,
    control: &Path,
    cancel: Cancellation,
    parent: Option<String>,
    mut bindings: DaemonBindings<W, C>,
) -> SoakResult<SoakReport>
where
    W: FnMut(&Path, &[u8]) -> SoakResult<()>,
    C: FnMut() -> Instant,
{
    profile.validate()?;
    if profile.cohort != Cohort::Daemon {
        return Err("daemon soak cohort required".into());
    }
    profile.check_artifacts(root)?;
    prepare_control(root, control)?;
    let crate::imap_daemon_probe_075::SoakDaemon {
        command,
        addrs,
        storage: _storage,
    } = crate::imap_daemon_probe_075::soak_command(binary, control)?;
    let mut report = SoakReport {
        schema: "imap075-soak-report-v1".into(),
        scope: "continuous_local_development_segment".into(),
        production_enabled: false,
        profile: profile.clone(),
        segment: nonce(),
        parent_segment: parent,
        controller: WorkerIdentity {
            pid: std::process::id(),
            platform_birth: process_identity(std::process::id())?,
            incarnation: nonce(),
        },
        worker: None,
        terminal: Terminal::Failed,
        rounds: 0,
        active_elapsed_ms: 0,
        transcript_sha256: crate::imap_qualification_075::digest(&[]),
        transcript_bytes: 0,
        process_reaped: false,
        remaining_members: None,
        containment: String::new(),
        detail: String::new(),
    };
    (bindings.write)(
        &control.join("checkpoint.json"),
        &serde_json::to_vec(&report)?,
    )?;
    let mut transcript = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(control.join("stdout.jsonl"))?;
    let mut raw_out = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(control.join("daemon-stdout.log"))?;
    let mut raw_err = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(control.join("stderr.txt"))?;
    let mut state = SegmentState::new(profile.clone())?;
    let (mut saved, mut errors) = (0usize, 0usize);
    let mut active = None;
    let mut next = Instant::now();
    let mut drain = None;
    let mut birth = String::new();
    let incarnation = nonce();
    let started = (bindings.clock)();
    let mut last_binding = (bindings.clock)();
    let mut record =
        |state: &mut SegmentState, frame: &Frame, pid: u32, birth: &str| -> Result<(), String> {
            state.accept(frame, pid, birth)?;
            if frame.phase == Phase::Ready {
                write_new(
                    &control.join("identity.json"),
                    &serde_json::to_vec(&IdentityCheckpoint {
                        schema: "imap075-soak-identity-v1".into(),
                        segment: report.segment.clone(),
                        worker: state.worker.clone().expect("accepted daemon identity"),
                    })
                    .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
            }
            serde_json::to_writer(&mut transcript, frame).map_err(|e| e.to_string())?;
            writeln!(transcript).map_err(|e| e.to_string())?;
            Ok(())
        };
    let mut frame = Frame {
        schema: "imap075-soak-frame-v1".into(),
        phase: Phase::Ready,
        pid: 0,
        incarnation,
        cohort: Cohort::Daemon,
        seed: profile.seed,
        source_sha: profile.source_sha.clone(),
        round: 0,
        active_elapsed_ms: 0,
        digest: None,
        owners: Owners {
            unit: "daemon_protocol_only".into(),
            values: Some(0),
            ..Default::default()
        },
        heap_bytes: None,
        rss_bytes: None,
        gc_collections: None,
        cpu_ns: None,
        threads: None,
        reason: None,
    };
    let result = process::execute_managed(
        command,
        &[],
        ProcessBounds {
            timeout: Duration::from_millis(profile.bounds.hard_ms),
            cleanup_timeout: Duration::from_secs(5),
            stdout_bytes: 8 * 1024 * 1024,
            stderr_bytes: 1024 * 1024,
            memory_bytes: WORKER_MEMORY_BYTES,
            ..Default::default()
        },
        ExecutionPolicy::Soak,
        |o| {
            raw_out
                .write_all(&o.stdout[saved..])
                .map_err(|e| e.to_string())?;
            saved = o.stdout.len();
            raw_err
                .write_all(&o.stderr[errors..])
                .map_err(|e| e.to_string())?;
            errors = o.stderr.len();
            if state.phase.is_none()
                && o.stdout.starts_with(b"{\"status\":\"ok\"}")
                && !o.root_exited
            {
                crate::imap_daemon_probe_075::soak_ready(&addrs).map_err(|e| e.to_string())?;
                birth = process_identity(o.pid).map_err(|e| e.to_string())?;
                frame.pid = o.pid;
                record(&mut state, &frame, o.pid, &birth)?;
                for position in 1..=profile.bounds.warmup_rounds {
                    crate::imap_daemon_probe_075::soak_round(&addrs, profile.seed, position)
                        .map_err(|e| e.to_string())?;
                }
                frame.digest = Some(expected_digest(&profile, 0));
                frame.phase = Phase::Warmup;
                record(&mut state, &frame, o.pid, &birth)?;
                frame.phase = Phase::Active;
                record(&mut state, &frame, o.pid, &birth)?;
                active = Some(Instant::now());
                next = Instant::now();
            }
            if let Some(at) = active {
                let requested = cancel.is_cancelled()
                    || marker(control, "cancel").map_err(|e| e.to_string())?;
                if drain.is_none()
                    && (requested
                        || at.elapsed() >= Duration::from_millis(profile.bounds.active_ms)
                            && frame.round >= 2)
                {
                    crate::imap_daemon_probe_075::soak_drain(&addrs).map_err(|e| e.to_string())?;
                    frame.phase = Phase::Drained;
                    frame.active_elapsed_ms = at.elapsed().as_millis() as u64;
                    frame.reason = Some(daemon_drain_reason(requested).into());
                    record(&mut state, &frame, o.pid, &birth)?;
                    drain = Some(Instant::now());
                } else if drain.is_none() && Instant::now() >= next && !o.root_exited {
                    daemon_round_budget(frame.round, profile.bounds.maximum_rounds)?;
                    crate::imap_daemon_probe_075::soak_round(
                        &addrs,
                        profile.seed,
                        profile.bounds.warmup_rounds + frame.round + 1,
                    )
                    .map_err(|e| e.to_string())?;
                    frame.round += 1;
                    frame.phase = Phase::Round;
                    frame.active_elapsed_ms = at.elapsed().as_millis() as u64;
                    record(&mut state, &frame, o.pid, &birth)?;
                    next = Instant::now() + Duration::from_millis(profile.bounds.period_ms);
                    println!(
                        "imap-soak: daemon round={} active_ms={}",
                        frame.round, frame.active_elapsed_ms
                    );
                }
            }
            daemon_deadlines(
                state.phase.is_some(),
                o.root_exited,
                &profile.bounds,
                || (bindings.clock)().saturating_duration_since(started),
                || drain.map(|at| at.elapsed()),
            )?;
            refresh_soak_bindings(
                root,
                &profile,
                &mut last_binding,
                &mut bindings.clock,
                bindings.check,
            )?;
            Ok(())
        },
    );
    transcript.sync_all()?;
    raw_out.sync_all()?;
    raw_err.sync_all()?;
    report.worker = state.worker;
    report.rounds = state.rounds;
    report.active_elapsed_ms = state.active_elapsed_ms;
    let transcript = fs::read(control.join("stdout.jsonl"))?;
    report.transcript_bytes = transcript.len() as u64;
    report.transcript_sha256 = crate::imap_qualification_075::digest(&transcript);
    finalize_process(
        &mut report,
        result,
        ProtocolEnd::Daemon {
            terminal: state.terminal,
        },
        false,
    );
    let artifacts = profile.check_artifacts(root);
    let cleanup_complete = report.process_reaped
        && report.remaining_members == Some(0)
        && addrs
            .iter()
            .all(|addr| std::net::TcpListener::bind(addr).is_ok());
    finalize_segment(
        &mut report,
        false,
        artifacts,
        cleanup_complete,
        "; daemon cleanup/listeners incomplete",
    );
    (bindings.write)(
        &control.join("report.json"),
        &serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}

fn refresh_soak_bindings(
    root: &Path,
    profile: &SoakProfile,
    last: &mut Instant,
    mut clock: impl FnMut() -> Instant,
    check: impl FnOnce(&SoakProfile, &Path) -> Result<(), Box<dyn Error>>,
) -> Result<(), String> {
    if clock().saturating_duration_since(*last) >= Duration::from_secs(10) {
        check(profile, root).map_err(|e| e.to_string())?;
        *last = clock();
    }
    Ok(())
}
fn daemon_drain_reason(cancelled: bool) -> &'static str {
    if cancelled {
        "cancelled"
    } else {
        "duration_complete"
    }
}
fn daemon_round_budget(round: u64, maximum: u64) -> Result<(), String> {
    if round == maximum {
        return Err("daemon soak round limit".into());
    }
    Ok(())
}
fn daemon_deadlines(
    ready: bool,
    exited: bool,
    bounds: &SoakBounds,
    startup: impl FnOnce() -> Duration,
    drain: impl FnOnce() -> Option<Duration>,
) -> Result<(), String> {
    if !ready && startup() > Duration::from_millis(bounds.startup_ms) {
        return Err("daemon soak readiness deadline".into());
    }
    if drain().is_some_and(|at| at > Duration::from_millis(bounds.drain_ms)) && !exited {
        return Err("daemon soak drain deadline".into());
    }
    Ok(())
}

fn require_completed_segment(
    terminal: &Terminal,
    detail: &str,
    context: &str,
) -> Result<(), Box<dyn Error>> {
    if *terminal != Terminal::Complete {
        return Err(format!("{context}: {terminal:?}: {detail}").into());
    }
    Ok(())
}
fn require_soak_output(output: &str) -> Result<(), Box<dyn Error>> {
    if !output.replace('\\', "/").starts_with("target/imap-075/") {
        return Err("soak output must be under target/imap-075".into());
    }
    Ok(())
}
fn require_soak_source(expected: &str, current: &str) -> Result<(), Box<dyn Error>> {
    if current != expected {
        return Err("source changed during soak".into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/support/imap_soak_publication_075.rs"]
mod publication_tests;

#[cfg(test)]
#[path = "../tests/support/imap_soak_replay_errors_075.rs"]
mod replay_error_tests;

type SoakResult<T> = Result<T, Box<dyn Error>>;
type ReplayProfile = fn(&Path, &str, Cohort, SoakBounds) -> SoakResult<SoakProfile>;
struct ReplayBindings {
    source: fn(&Path) -> SoakResult<String>,
    profile: ReplayProfile,
    artifacts: fn(&SoakProfile, &Path) -> SoakResult<()>,
    live: fn(&WorkerIdentity) -> SoakResult<bool>,
}
impl ReplayBindings {
    fn real() -> Self {
        Self {
            source: crate::imap_ci_075::source_state,
            profile: trusted_profile,
            artifacts: SoakProfile::check_artifacts,
            live: identity_is_live,
        }
    }
}
fn replay_saved_progress(
    profile: &SoakProfile,
    identity: &WorkerIdentity,
    transcript: &[u8],
    rounds: u64,
    active_ms: u64,
) -> SoakResult<()> {
    let mut state = SegmentState::new(profile.clone())?;
    for line in transcript
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
    {
        if line.len() > 4096 {
            return Err("saved soak frame bound".into());
        }
        state.accept(
            &serde_json::from_slice(line)?,
            identity.pid,
            &identity.platform_birth,
        )?;
    }
    if state.terminal != Some(Terminal::Complete)
        || state.rounds != rounds
        || state.active_elapsed_ms != active_ms
    {
        return Err("saved soak progress incomplete".into());
    }
    Ok(())
}

#[cfg(windows)]
fn windows_process_birth(
    ok: bool,
    high: u32,
    low: u32,
    error: impl FnOnce() -> std::io::Error,
) -> SoakResult<String> {
    if !ok {
        return Err(error().into());
    }
    Ok(format!("windows-filetime-{high:08x}{low:08x}"))
}

trait SoakCommands {
    fn root(&mut self) -> SoakResult<std::path::PathBuf>;
    fn source(&mut self, root: &Path) -> SoakResult<String>;
    fn profile(
        &mut self,
        root: &Path,
        source: &str,
        cohort: Cohort,
        bounds: SoakBounds,
    ) -> SoakResult<SoakProfile>;
    fn execute(
        &mut self,
        root: &Path,
        profile: SoakProfile,
        control: &Path,
        parent: Option<String>,
    ) -> SoakResult<SoakReport>;
    fn completed(&mut self, report: &SoakReport, context: &str) -> SoakResult<()>;
    fn check(&mut self, root: &Path, path: &Path) -> SoakResult<SoakReport>;
    fn artifact(
        &mut self,
        root: &Path,
        path: &Path,
    ) -> SoakResult<crate::imap_extended_receipt_075::Artifact>;
}
struct ActualSoakCommands;
impl SoakCommands for ActualSoakCommands {
    fn root(&mut self) -> SoakResult<std::path::PathBuf> {
        crate::doc_check::find_repo_root()
    }
    fn source(&mut self, root: &Path) -> SoakResult<String> {
        crate::imap_ci_075::source_state(root)
    }
    fn profile(
        &mut self,
        root: &Path,
        source: &str,
        cohort: Cohort,
        bounds: SoakBounds,
    ) -> SoakResult<SoakProfile> {
        trusted_profile(root, source, cohort, bounds)
    }
    fn execute(
        &mut self,
        root: &Path,
        profile: SoakProfile,
        control: &Path,
        parent: Option<String>,
    ) -> SoakResult<SoakReport> {
        run_prepared(root, profile, control, Cancellation::default(), parent)
    }
    fn completed(&mut self, report: &SoakReport, context: &str) -> SoakResult<()> {
        require_completed_segment(&report.terminal, &report.detail, context)
    }
    fn check(&mut self, root: &Path, path: &Path) -> SoakResult<SoakReport> {
        check_saved(root, path)
    }
    fn artifact(
        &mut self,
        root: &Path,
        path: &Path,
    ) -> SoakResult<crate::imap_extended_receipt_075::Artifact> {
        crate::imap_extended_receipt_075::artifact(root, path, 256 * 1024)
    }
}
#[cfg(test)]
#[path = "../tests/support/imap_soak_orchestration_errors_075.rs"]
mod orchestration_error_tests;

struct DaemonBindings<W, C> {
    write: W,
    clock: C,
    check: fn(&SoakProfile, &Path) -> SoakResult<()>,
}
