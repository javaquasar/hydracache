//! Bounded read-only manager observation. No unit mutation or cleanup authority.
use crate::diagnostic_lease::{validate_identity, DiagnosticIdentity, SURFACES};
use crate::diagnostic_loaded::{decode_settings, InvocationGuard, LoadedSettings, LoadedSnapshot};
use crate::{canonical_json, is_hash};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use zbus::blocking::{connection::Builder, Connection, Proxy};
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

const REQUEST_BYTES: usize = 4096;
const OUTPUT_BYTES: usize = 65_536;
const OPERATION_TIMEOUT: Duration = Duration::from_millis(2000);
const CLEANUP_TIMEOUT: Duration = Duration::from_millis(1000);
const POLL_INTERVAL: Duration = Duration::from_millis(5);
const METHOD_TIMEOUT: Duration = Duration::from_millis(250);
const ADDRESS_SPACE_BYTES: u64 = 536_870_912;
const CPU_SECONDS: u64 = 2;
const BUS_ADDRESS: &str = "unix:path=/run/dbus/system_bus_socket";
const DESTINATION: &str = "org.freedesktop.systemd1";
const MANAGER_PATH: &str = "/org/freedesktop/systemd1";
const MANAGER_INTERFACE: &str = "org.freedesktop.systemd1.Manager";
const UNIT_INTERFACE: &str = "org.freedesktop.systemd1.Unit";
const SERVICE_INTERFACE: &str = "org.freedesktop.systemd1.Service";

#[path = "diagnostic_worker_account.rs"]
mod account;
pub use account::{account_worker_main, WorkerAccountRead, WorkerAccountSnapshot};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManagerScope {
    schema_version: u32,
    lease_id: String,
    boot_id: String,
    surface: String,
}
impl<'de> Deserialize<'de> for ManagerScope {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct WireScope {
            schema_version: u32,
            lease_id: String,
            boot_id: String,
            surface: String,
        }
        let wire = WireScope::deserialize(deserializer)?;
        let scope = Self {
            schema_version: wire.schema_version,
            lease_id: wire.lease_id,
            boot_id: wire.boot_id,
            surface: wire.surface,
        };
        scope.validate().map_err(serde::de::Error::custom)?;
        Ok(scope)
    }
}
impl ManagerScope {
    pub fn new(lease: &str, boot: &str, surface: &str) -> Result<Self, String> {
        let scope = Self {
            schema_version: 1,
            lease_id: lease.into(),
            boot_id: boot.into(),
            surface: surface.into(),
        };
        scope.validate()?;
        Ok(scope)
    }
    fn validate(&self) -> Result<(), String> {
        let identity = DiagnosticIdentity {
            lease_id: self.lease_id.clone(),
            boot_id: self.boot_id.clone(),
            // Syntax only: this scope makes no assertion about an artifact.
            binary_sha256: "0".repeat(64),
            build_provenance_sha256: "0".repeat(64),
        };
        validate_identity(&identity).map_err(|_| "invalid manager scope")?;
        if self.schema_version != 1 || !SURFACES.contains(&self.surface.as_str()) {
            return Err("invalid manager scope".into());
        }
        Ok(())
    }
    pub fn unit_name(&self) -> String {
        let index = SURFACES
            .iter()
            .position(|v| *v == self.surface)
            .expect("validated private scope")
            + 1;
        format!(
            "hydracache-diagnostic-074-{}-{index}.service",
            self.lease_id
        )
    }
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        canonical_json(self).map_err(|_| "scope serialization failed".into())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > REQUEST_BYTES {
            return Err("scope byte budget".into());
        }
        let scope: Self = serde_json::from_slice(bytes).map_err(|_| "invalid scope document")?;
        if scope.encode()? != bytes {
            return Err("noncanonical scope".into());
        }
        Ok(scope)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedUnit {
    pub unit_name: String,
    pub object_path: String,
    pub invocation_id: String,
    pub active_state: String,
    pub sub_state: String,
    pub main_pid: u32,
    pub control_group: String,
    pub result: String,
    pub transient: bool,
}

/// An observation of a pinned manager owner; not unit-policy or release proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagerSnapshot {
    schema_version: u32,
    scope: ManagerScope,
    manager_owner: String,
    manager_uid: u32,
    manager_pid: u32,
    unit: Option<ObservedUnit>,
}
impl ManagerSnapshot {
    pub(crate) fn scope(&self) -> &ManagerScope {
        &self.scope
    }
    pub(crate) fn same_manager(&self, other: &Self) -> bool {
        self.scope == other.scope
            && self.manager_owner == other.manager_owner
            && self.manager_uid == other.manager_uid
            && self.manager_pid == other.manager_pid
    }
    pub fn unit(&self) -> Option<&ObservedUnit> {
        self.unit.as_ref()
    }
    pub(crate) fn validate(&self, scope: &ManagerScope) -> Result<(), String> {
        scope.validate()?;
        if self.schema_version != 1
            || &self.scope != scope
            || self.manager_uid != 0
            || self.manager_pid != 1
            || !unique_owner(&self.manager_owner)
        {
            return Err("manager owner or scope mismatch".into());
        }
        if let Some(unit) = &self.unit {
            let expected_group = format!("/system.slice/{}", scope.unit_name());
            if unit.unit_name != scope.unit_name()
                || !unit
                    .object_path
                    .starts_with("/org/freedesktop/systemd1/unit/")
                || unit.object_path.len() > 512
                || !unit
                    .object_path
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'/')
                || unit.invocation_id.len() != 32
                || !is_hash(&format!("{}{}", unit.invocation_id, unit.invocation_id))
                || !word(&unit.active_state)
                || !word(&unit.sub_state)
                || !word(&unit.result)
                || unit.main_pid > i32::MAX as u32
                || !(unit.control_group == expected_group
                    || (unit.control_group.is_empty() && unit.main_pid == 0))
            {
                return Err("unit observation mismatch".into());
            }
        }
        Ok(())
    }
    fn decode(bytes: &[u8], scope: &ManagerScope) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > OUTPUT_BYTES {
            return Err("snapshot byte budget".into());
        }
        let snapshot: Self =
            serde_json::from_slice(bytes).map_err(|_| "invalid manager snapshot")?;
        snapshot.validate(scope)?;
        if canonical_json(&snapshot).map_err(|_| "snapshot encoding")? != bytes {
            return Err("noncanonical manager snapshot".into());
        }
        Ok(snapshot)
    }
}
fn word(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
fn unique_owner(value: &str) -> bool {
    let Some(value) = value.strip_prefix(':') else {
        return false;
    };
    value.len() <= 64
        && value.split('.').count() == 2
        && value
            .split('.')
            .all(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerFailureKind {
    Invalid,
    Io,
    Deadline,
    OutputBudget,
    Exit,
    PendingCleanup,
}
#[derive(Debug)]
pub struct WorkerFailure {
    pub kind: WorkerFailureKind,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub cleanup_confirmed: bool,
}

#[derive(Default)]
pub struct ManagerClient {
    pending: Option<Child>,
}
impl ManagerClient {
    /// Every failure latches the original guard, including an unavailable or
    /// malformed observation. A later valid read cannot erase that failure.
    pub fn inspect_original(
        &mut self,
        guard: &mut InvocationGuard,
    ) -> Result<LoadedSnapshot, WorkerFailure> {
        if guard.is_refused() {
            return Err(WorkerFailure {
                kind: WorkerFailureKind::Invalid,
                stdout: vec![],
                stderr: vec![],
                cleanup_confirmed: self.pending.is_none(),
            });
        }
        let snapshot = match self.inspect_loaded(guard.scope()) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                guard.refuse();
                return Err(error);
            }
        };
        if guard.revalidate(&snapshot).is_err() {
            return Err(WorkerFailure {
                kind: WorkerFailureKind::Invalid,
                // The decoded document was required to match canonical bytes.
                stdout: canonical_json(&snapshot).unwrap_or_default(),
                stderr: vec![],
                cleanup_confirmed: true,
            });
        }
        Ok(snapshot)
    }
    /// Separate read-only projection; never complete policy or launch proof.
    pub fn inspect_loaded(
        &mut self,
        scope: &ManagerScope,
    ) -> Result<LoadedSnapshot, WorkerFailure> {
        let request = scope
            .encode()
            .map_err(|_| failure(WorkerFailureKind::Invalid))?;
        let mut command = Command::new("/proc/self/exe");
        command.arg("diagnostic-loaded-worker");
        let stdout = self.run(command, &request, OPERATION_TIMEOUT)?;
        LoadedSnapshot::decode(&stdout, scope).map_err(|_| WorkerFailure {
            kind: WorkerFailureKind::Invalid,
            stdout,
            stderr: vec![],
            cleanup_confirmed: true,
        })
    }
    /// No retries. The caller must retain failure evidence and its reservation.
    pub fn inspect(&mut self, scope: &ManagerScope) -> Result<ManagerSnapshot, WorkerFailure> {
        let request = scope
            .encode()
            .map_err(|_| failure(WorkerFailureKind::Invalid))?;
        let mut command = Command::new("/proc/self/exe");
        command.arg("diagnostic-manager-worker");
        let stdout = self.run(command, &request, OPERATION_TIMEOUT)?;
        ManagerSnapshot::decode(&stdout, scope).map_err(|_| WorkerFailure {
            kind: WorkerFailureKind::Invalid,
            stdout,
            stderr: vec![],
            cleanup_confirmed: true,
        })
    }
    /// Reap only an already retained helper. Never starts or mutates a unit.
    pub fn poll_pending_cleanup(&mut self) -> Result<bool, std::io::Error> {
        if let Some(child) = &mut self.pending {
            if child.try_wait()?.is_none() {
                return Ok(false);
            }
            self.pending = None;
        }
        Ok(true)
    }
    fn run(
        &mut self,
        mut command: Command,
        request: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>, WorkerFailure> {
        if !self.poll_pending_cleanup().map_err(|_| WorkerFailure {
            kind: WorkerFailureKind::Io,
            stdout: vec![],
            stderr: vec![],
            cleanup_confirmed: self.pending.is_none(),
        })? {
            return Err(failure(WorkerFailureKind::PendingCleanup));
        }
        if request.is_empty() || request.len() > REQUEST_BYTES {
            return Err(failure(WorkerFailureKind::Invalid));
        }
        let deadline = Instant::now() + timeout;
        #[cfg(test)]
        let fixture = command
            .get_envs()
            .find(|(k, _)| *k == "HC_DIAGNOSTIC_WORKER_FIXTURE")
            .and_then(|(_, v)| v.map(|v| v.to_owned()));
        command
            .env_clear()
            .env("TOKIO_WORKER_THREADS", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(test)]
        if let Some(fixture) = fixture {
            command.env("HC_DIAGNOSTIC_WORKER_FIXTURE", fixture);
        }
        // exec's diagnostic channel and every inherited non-stdio descriptor are
        // CLOEXEC. This executes no allocator/locking code in the forked child.
        use std::os::unix::process::CommandExt;
        unsafe {
            command.pre_exec(|| {
                if libc::syscall(
                    libc::SYS_close_range,
                    3u32,
                    u32::MAX,
                    libc::CLOSE_RANGE_CLOEXEC,
                ) < 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command
            .spawn()
            .map_err(|_| failure(WorkerFailureKind::Io))?;
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut reaped = false;
        let result = (|| -> Result<(), WorkerFailureKind> {
            let mut input = child.stdin.take().ok_or(WorkerFailureKind::Io)?;
            let mut out = child.stdout.take().ok_or(WorkerFailureKind::Io)?;
            let mut err = child.stderr.take().ok_or(WorkerFailureKind::Io)?;
            for fd in [input.as_raw_fd(), out.as_raw_fd(), err.as_raw_fd()] {
                nonblocking(fd).map_err(|_| WorkerFailureKind::Io)?;
            }
            let mut sent = 0;
            while sent < request.len() {
                check_deadline(deadline)?;
                match input.write(&request[sent..]) {
                    Ok(0) => return Err(WorkerFailureKind::Io),
                    Ok(n) => sent += n,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(POLL_INTERVAL)
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(_) => return Err(WorkerFailureKind::Io),
                }
            }
            drop(input);
            let mut out_eof = false;
            let mut err_eof = false;
            let mut exited = None;
            loop {
                check_deadline(deadline)?;
                drain(&mut out, &mut stdout, stderr.len(), &mut out_eof, deadline)?;
                drain(&mut err, &mut stderr, stdout.len(), &mut err_eof, deadline)?;
                if exited.is_none() {
                    exited = child.try_wait().map_err(|_| WorkerFailureKind::Io)?;
                    reaped = exited.is_some();
                }
                if let Some(status) = exited {
                    if !status.success() {
                        return Err(WorkerFailureKind::Exit);
                    }
                    if out_eof && err_eof {
                        if stderr.is_empty() {
                            return Ok(());
                        }
                        return Err(WorkerFailureKind::Invalid);
                    }
                }
                std::thread::sleep(POLL_INTERVAL);
            }
        })();
        match result {
            Ok(()) => Ok(stdout),
            Err(kind) => {
                let cleanup_confirmed = if reaped {
                    true
                } else {
                    cleanup_child(&mut child, CLEANUP_TIMEOUT)
                };
                if !cleanup_confirmed {
                    self.pending = Some(child);
                }
                Err(WorkerFailure {
                    kind,
                    stdout,
                    stderr,
                    cleanup_confirmed,
                })
            }
        }
    }
}
impl Drop for ManagerClient {
    fn drop(&mut self) {
        if let Some(child) = &mut self.pending {
            // Best effort only. A retained uncertain operation must not be
            // translated into reservation release by a caller on Drop.
            let _ = child.kill();
            let _ = child.try_wait();
        }
    }
}
fn failure(kind: WorkerFailureKind) -> WorkerFailure {
    WorkerFailure {
        kind,
        stdout: vec![],
        stderr: vec![],
        cleanup_confirmed: kind != WorkerFailureKind::PendingCleanup,
    }
}
fn check_deadline(deadline: Instant) -> Result<(), WorkerFailureKind> {
    if Instant::now() >= deadline {
        Err(WorkerFailureKind::Deadline)
    } else {
        Ok(())
    }
}
fn nonblocking(fd: i32) -> std::io::Result<()> {
    // SAFETY: a live owned stdio pipe descriptor, neither closed nor retained here.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}
fn drain(
    reader: &mut impl Read,
    output: &mut Vec<u8>,
    other_len: usize,
    eof: &mut bool,
    deadline: Instant,
) -> Result<(), WorkerFailureKind> {
    if *eof {
        return Ok(());
    }
    let mut buffer = [0u8; 4096];
    loop {
        check_deadline(deadline)?;
        match reader.read(&mut buffer) {
            Ok(0) => {
                *eof = true;
                return Ok(());
            }
            Ok(n) => {
                let remaining = OUTPUT_BYTES.saturating_sub(output.len() + other_len);
                output.extend_from_slice(&buffer[..n.min(remaining)]);
                if n > remaining {
                    return Err(WorkerFailureKind::OutputBudget);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return Err(WorkerFailureKind::Io),
        }
    }
}
fn cleanup_child(child: &mut Child, budget: Duration) -> bool {
    // Child has not been reaped, so its PID cannot be reused. Kill is scoped to
    // that direct helper, never a process group, diagnostic unit or service.
    let _ = child.kill();
    let deadline = Instant::now() + budget;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Err(_) => return false,
            Ok(None) => {}
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Fixed worker entry point: canonical stdin, canonical stdout, bounded error.
/// Use only in a separately bounded child; blocking DBus setup is intentional.
pub fn worker_main() -> u8 {
    worker(false)
}
pub fn loaded_worker_main() -> u8 {
    worker(true)
}
fn worker(loaded: bool) -> u8 {
    let result = (|| -> Result<Vec<u8>, String> {
        restrict_worker_resources()?;
        let mut request = Vec::new();
        std::io::stdin()
            .take((REQUEST_BYTES + 1) as u64)
            .read_to_end(&mut request)
            .map_err(|_| "worker input")?;
        let scope = ManagerScope::decode(&request)?;
        let (snapshot, settings) = inspect_in_worker(&scope, loaded)?;
        if loaded {
            canonical_json(&LoadedSnapshot::new(snapshot, settings))
        } else {
            canonical_json(&snapshot)
        }
        .map_err(|_| "worker encoding".into())
    })();
    match result {
        Ok(bytes) if bytes.len() <= OUTPUT_BYTES => {
            if std::io::stdout().write_all(&bytes).is_ok() {
                0
            } else {
                9
            }
        }
        Err(error) => {
            // Internal categories only: no DBus error bodies, request fields,
            // paths or credentials are included in these bounded literals.
            eprintln!("diagnostic manager observation refused: {error}");
            9
        }
        Ok(_) => {
            eprintln!("diagnostic manager observation refused");
            9
        }
    }
}
fn restrict_worker_resources() -> Result<(), String> {
    for (resource, maximum) in [
        (libc::RLIMIT_AS, ADDRESS_SPACE_BYTES),
        (libc::RLIMIT_CPU, CPU_SECONDS),
        (libc::RLIMIT_CORE, 0),
    ] {
        let limits = libc::rlimit {
            rlim_cur: maximum as libc::rlim_t,
            rlim_max: maximum as libc::rlim_t,
        };
        // SAFETY: this worker has not opened DBus or spawned threads; the
        // initialized structure lives throughout this self-only syscall.
        if unsafe { libc::setrlimit(resource, &limits) } != 0 {
            return Err("worker resource limits refused".into());
        }
    }
    Ok(())
}
fn fixed_bus_socket() -> Result<(), String> {
    for path in ["/", "/run", "/run/dbus"] {
        let metadata = fs::symlink_metadata(path).map_err(|_| "system bus ancestor")?;
        if !metadata.is_dir() || metadata.uid() != 0 || metadata.mode() & 0o022 != 0 {
            return Err("untrusted system bus ancestor".into());
        }
    }
    let socket =
        fs::symlink_metadata("/run/dbus/system_bus_socket").map_err(|_| "system bus socket")?;
    if !socket.file_type().is_socket() || socket.uid() != 0 {
        return Err("untrusted system bus socket".into());
    }
    Ok(())
}
fn proxy<'a>(
    connection: &Connection,
    owner: &'a str,
    path: &'a str,
    interface: &'a str,
) -> Result<Proxy<'a>, String> {
    zbus::blocking::proxy::Builder::new(connection)
        .destination(owner)
        .map_err(|_| "proxy destination")?
        .path(path)
        .map_err(|_| "proxy path")?
        .interface(interface)
        .map_err(|_| "proxy interface")?
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .map_err(|_| "manager proxy".into())
}
fn manager_owner(bus: &Proxy<'_>) -> Result<String, String> {
    let owner: String = checked_call(bus, "org.freedesktop.DBus", "GetNameOwner", &(DESTINATION,))
        .map_err(|_| "manager owner unavailable")?;
    let uid: u32 = checked_call(
        bus,
        "org.freedesktop.DBus",
        "GetConnectionUnixUser",
        &(owner.as_str(),),
    )
    .map_err(|_| "manager user unavailable")?;
    let pid: u32 = checked_call(
        bus,
        "org.freedesktop.DBus",
        "GetConnectionUnixProcessID",
        &(owner.as_str(),),
    )
    .map_err(|_| "manager pid unavailable")?;
    if !unique_owner(&owner) || uid != 0 || pid != 1 {
        return Err("foreign manager owner".into());
    }
    Ok(owner)
}
#[derive(Debug, PartialEq, Eq)]
enum ReplyError {
    NoSuchUnit,
    Refused,
}
fn sender_matches(sender: Option<&str>, expected: &str) -> bool {
    sender == Some(expected) && (unique_owner(expected) || expected == "org.freedesktop.DBus")
}
fn classify_error(name: &str, sender: Option<&str>, owner: &str) -> ReplyError {
    if sender_matches(sender, owner) && name == "org.freedesktop.systemd1.NoSuchUnit" {
        ReplyError::NoSuchUnit
    } else {
        ReplyError::Refused
    }
}
fn decode_reply<R: for<'de> zbus::zvariant::DynamicDeserialize<'de>>(
    message: &zbus::Message,
    owner: &str,
) -> Result<R, ReplyError> {
    if !sender_matches(message.header().sender().map(|v| v.as_str()), owner) {
        return Err(ReplyError::Refused);
    }
    message
        .body()
        .deserialize()
        .map_err(|_| ReplyError::Refused)
}
fn checked_call<B, R>(
    proxy: &Proxy<'_>,
    owner: &str,
    method: &str,
    body: &B,
) -> Result<R, ReplyError>
where
    B: Serialize + zbus::zvariant::DynamicType,
    R: for<'de> zbus::zvariant::DynamicDeserialize<'de>,
{
    let message = match proxy.call_method(method, body) {
        Ok(message) => message,
        Err(zbus::Error::MethodError(name, _, message)) => {
            return Err(classify_error(
                name.as_str(),
                message.header().sender().map(|v| v.as_str()),
                owner,
            ));
        }
        Err(_) => return Err(ReplyError::Refused),
    };
    decode_reply(&message, owner)
}
fn boot_matches(scope: &ManagerScope) -> Result<(), String> {
    let boot =
        fs::read_to_string("/proc/sys/kernel/random/boot_id").map_err(|_| "boot unavailable")?;
    if boot != format!("{}\n", scope.boot_id) {
        return Err("manager boot mismatch".into());
    }
    Ok(())
}
fn inspect_in_worker(
    scope: &ManagerScope,
    loaded: bool,
) -> Result<(ManagerSnapshot, Option<LoadedSettings>), String> {
    scope.validate()?;
    fixed_bus_socket()?;
    boot_matches(scope)?;
    let connection = Builder::address(BUS_ADDRESS)
        .map_err(|_| "system bus address")?
        .method_timeout(METHOD_TIMEOUT)
        .build()
        .map_err(|_| "system bus connection")?;
    let bus = proxy(
        &connection,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
    )?;
    let owner = manager_owner(&bus)?;
    let manager = proxy(&connection, &owner, MANAGER_PATH, MANAGER_INTERFACE)?;
    let name = scope.unit_name();
    let path: Result<OwnedObjectPath, ReplyError> =
        checked_call(&manager, &owner, "GetUnit", &(name.as_str(),));
    let (unit, settings) = match path {
        Ok(path) => {
            let first = read_unit(&connection, &owner, path.as_str(), loaded)?;
            if read_unit(&connection, &owner, path.as_str(), loaded)? != first {
                return Err("unit observation changed".into());
            }
            (Some(first.0), first.1)
        }
        Err(ReplyError::NoSuchUnit) => (None, None),
        Err(_) => return Err("unit inspection failed".into()),
    };
    if manager_owner(&bus)? != owner {
        return Err("manager owner changed".into());
    }
    boot_matches(scope)?;
    let snapshot = ManagerSnapshot {
        schema_version: 1,
        scope: scope.clone(),
        manager_owner: owner.clone(),
        manager_uid: 0,
        manager_pid: 1,
        unit,
    };
    snapshot.validate(scope)?;
    Ok((snapshot, settings))
}
fn read_unit(
    connection: &Connection,
    owner: &str,
    path: &str,
    loaded: bool,
) -> Result<(ObservedUnit, Option<LoadedSettings>), String> {
    let properties = proxy(connection, owner, path, "org.freedesktop.DBus.Properties")?;
    let mut unit: HashMap<String, OwnedValue> =
        checked_call(&properties, owner, "GetAll", &(UNIT_INTERFACE,))
            .map_err(|_| "unit properties")?;
    let mut service: HashMap<String, OwnedValue> =
        checked_call(&properties, owner, "GetAll", &(SERVICE_INTERFACE,))
            .map_err(|_| "service properties")?;
    if loaded {
        // Description belongs to Unit; never guess from a Service fallback.
        service.insert(
            "Description".into(),
            unit.remove("Description")
                .ok_or("missing unit description")?,
        );
        let mut identity = HashMap::new();
        for field in ["MainPID", "ControlGroup", "Result"] {
            identity.insert(
                field.into(),
                service.remove(field).ok_or("missing service identity")?,
            );
        }
        Ok((
            decode_unit_properties(path, unit, identity)?,
            Some(decode_settings(service)?),
        ))
    } else {
        Ok((decode_unit_properties(path, unit, service)?, None))
    }
}
fn decode_unit_properties(
    path: &str,
    mut unit: HashMap<String, OwnedValue>,
    mut service: HashMap<String, OwnedValue>,
) -> Result<ObservedUnit, String> {
    let id: Vec<u8> = take(&mut unit, "InvocationID")?;
    if id.len() != 16 {
        return Err("unit invocation identity".into());
    }
    Ok(ObservedUnit {
        unit_name: take(&mut unit, "Id")?,
        object_path: path.into(),
        invocation_id: id.iter().map(|v| format!("{v:02x}")).collect(),
        active_state: take(&mut unit, "ActiveState")?,
        sub_state: take(&mut unit, "SubState")?,
        main_pid: take(&mut service, "MainPID")?,
        control_group: take(&mut service, "ControlGroup")?,
        result: take(&mut service, "Result")?,
        transient: take(&mut unit, "Transient")?,
    })
}
pub(crate) fn take<T: TryFrom<OwnedValue>>(
    values: &mut HashMap<String, OwnedValue>,
    name: &str,
) -> Result<T, String> {
    values
        .remove(name)
        .ok_or_else(|| "missing unit property".to_string())?
        .try_into()
        .map_err(|_| "unit property type".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> ManagerScope {
        ManagerScope::new(
            &"a".repeat(64),
            "12345678-1234-1234-1234-123456789abc",
            "resp2",
        )
        .unwrap()
    }
    #[test]
    fn reply_origin_and_error_name_are_checked_before_absence_or_body_decode() {
        let method = zbus::Message::method_call("/org/freedesktop/systemd1", "GetUnit")
            .unwrap()
            .build(&("scope",))
            .unwrap();
        let reply = zbus::Message::method_return(&method.header())
            .unwrap()
            .sender(":1.9")
            .unwrap()
            .build(&7u32)
            .unwrap();
        assert_eq!(decode_reply::<u32>(&reply, ":1.9"), Ok(7));
        assert_eq!(
            decode_reply::<u32>(&reply, ":1.8"),
            Err(ReplyError::Refused)
        );
        assert_eq!(
            decode_reply::<String>(&reply, ":1.9"),
            Err(ReplyError::Refused)
        );
        for sender in [
            None,
            Some(":1.8"),
            Some("org.freedesktop.DBus"),
            Some("org.freedesktop.systemd1"),
        ] {
            assert_eq!(
                classify_error("org.freedesktop.systemd1.NoSuchUnit", sender, ":1.9"),
                ReplyError::Refused
            );
        }
        assert_eq!(
            classify_error("org.freedesktop.systemd1.NoSuchUnit", Some(":1.9"), ":1.9"),
            ReplyError::NoSuchUnit
        );
        for name in [
            "org.freedesktop.DBus.Error.AccessDenied",
            "org.freedesktop.DBus.Error.Timeout",
            "org.freedesktop.systemd1.UnitNotFound",
            "NoSuchUnit",
        ] {
            assert_eq!(
                classify_error(name, Some(":1.9"), ":1.9"),
                ReplyError::Refused
            );
        }
        assert!(sender_matches(
            Some("org.freedesktop.DBus"),
            "org.freedesktop.DBus"
        ));
        assert!(!sender_matches(Some(":1.9"), "org.freedesktop.DBus"));
    }
    fn snapshot() -> ManagerSnapshot {
        ManagerSnapshot {
            schema_version: 1,
            scope: scope(),
            manager_owner: ":1.5".into(),
            manager_uid: 0,
            manager_pid: 1,
            unit: None,
        }
    }
    fn fixture(mode: &str) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args([
            "--exact",
            "diagnostic_manager::tests::worker_fixture",
            "--nocapture",
        ]);
        command.env("HC_DIAGNOSTIC_WORKER_FIXTURE", mode);
        command
    }
    #[test]
    fn worker_fixture() {
        let Ok(mode) = std::env::var("HC_DIAGNOSTIC_WORKER_FIXTURE") else {
            return;
        };
        if mode == "retained" {
            // This child is adopted directly with null stdin, not run(request).
            std::thread::sleep(Duration::from_secs(10));
            return;
        }
        // Isolate output/exit scenarios from a broken-input-pipe race. The
        // parent closes stdin after sending; a fixture must accept that request
        // before exercising its intended failure. Production still refuses IO.
        let mut bytes = vec![];
        std::io::stdin().read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"scope");
        match mode.as_str() {
            "ok" => {
                assert_eq!(std::env::var("TOKIO_WORKER_THREADS").unwrap(), "1");
                assert!(std::env::var_os("DBUS_SYSTEM_BUS_ADDRESS").is_none());
                assert!(std::env::var_os("LD_PRELOAD").is_none());
            }
            "exit" => {
                print!("partial-failed");
                std::io::stdout().flush().unwrap();
                eprint!("failed");
                std::process::exit(7);
            }
            "stderr" => {
                eprint!("unexpected warning");
            }
            "limits" => {
                restrict_worker_resources().unwrap();
                for (resource, expected) in [
                    (libc::RLIMIT_AS, ADDRESS_SPACE_BYTES),
                    (libc::RLIMIT_CPU, CPU_SECONDS),
                    (libc::RLIMIT_CORE, 0),
                ] {
                    let mut limit = libc::rlimit {
                        rlim_cur: 0,
                        rlim_max: 0,
                    };
                    // SAFETY: initialized output storage for a self-only query.
                    assert_eq!(unsafe { libc::getrlimit(resource, &mut limit) }, 0);
                    assert_eq!(limit.rlim_cur, expected);
                    assert_eq!(limit.rlim_max, expected);
                }
            }
            "flood" => {
                let _ = std::io::stdout().write_all(&vec![b'x'; OUTPUT_BYTES * 4]);
                std::thread::sleep(Duration::from_secs(10));
            }
            "stderr-flood" => {
                let _ = std::io::stderr().write_all(&vec![b'x'; OUTPUT_BYTES * 4]);
                std::thread::sleep(Duration::from_secs(10));
            }
            "stall" => {
                print!("partial-before-stall");
                std::io::stdout().flush().unwrap();
                std::thread::sleep(Duration::from_secs(10));
            }
            _ => panic!("unknown fixture"),
        }
    }
    #[test]
    fn bounded_worker_success_closes_input_drains_and_reaps() {
        let mut client = ManagerClient::default();
        let mut command = fixture("ok");
        command.env("DBUS_SYSTEM_BUS_ADDRESS", "unix:path=/foreign/bus");
        command.env("LD_PRELOAD", "/not/a/library");
        let bytes = client.run(command, b"scope", OPERATION_TIMEOUT).unwrap();
        assert!(String::from_utf8(bytes)
            .unwrap()
            .contains("test result: ok"));
        assert!(client.poll_pending_cleanup().unwrap());
    }
    #[test]
    fn worker_resource_limits_apply_only_in_owned_test_child() {
        ManagerClient::default()
            .run(fixture("limits"), b"scope", OPERATION_TIMEOUT)
            .unwrap();
    }
    #[test]
    fn stalled_worker_retains_prefix_and_is_reaped_without_wait() {
        let mut client = ManagerClient::default();
        let error = client
            .run(fixture("stall"), b"scope", Duration::from_millis(200))
            .unwrap_err();
        assert_eq!(error.kind, WorkerFailureKind::Deadline);
        assert!(error.cleanup_confirmed);
        assert!(String::from_utf8(error.stdout)
            .unwrap()
            .contains("partial-before-stall"));
        assert!(client.poll_pending_cleanup().unwrap());
    }
    #[test]
    fn both_output_streams_share_one_budget_and_overflow_is_retained() {
        for mode in ["flood", "stderr-flood"] {
            let mut client = ManagerClient::default();
            let error = client
                .run(fixture(mode), b"scope", OPERATION_TIMEOUT)
                .unwrap_err();
            assert_eq!(error.kind, WorkerFailureKind::OutputBudget);
            assert_eq!(error.stdout.len() + error.stderr.len(), OUTPUT_BYTES);
            assert!(error.cleanup_confirmed);
        }
    }
    #[test]
    fn failed_exit_and_unexpected_stderr_are_not_success() {
        for (mode, kind) in [
            ("exit", WorkerFailureKind::Exit),
            ("stderr", WorkerFailureKind::Invalid),
        ] {
            let error = ManagerClient::default()
                .run(fixture(mode), b"scope", OPERATION_TIMEOUT)
                .unwrap_err();
            assert_eq!(error.kind, kind);
            assert!(!error.stderr.is_empty());
            assert!(error.cleanup_confirmed);
        }
    }
    #[test]
    fn retained_live_helper_refuses_second_operation() {
        let mut client = ManagerClient::default();
        let mut command = fixture("retained");
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        client.pending = Some(command.spawn().unwrap());
        let error = client
            .run(fixture("ok"), b"scope", OPERATION_TIMEOUT)
            .unwrap_err();
        assert_eq!(error.kind, WorkerFailureKind::PendingCleanup);
        assert!(!error.cleanup_confirmed);
        assert!(cleanup_child(
            client.pending.as_mut().unwrap(),
            CLEANUP_TIMEOUT
        ));
        assert!(client.poll_pending_cleanup().unwrap());
    }
    #[test]
    fn invalid_input_and_spawn_failure_do_not_adopt_a_helper() {
        let mut client = ManagerClient::default();
        assert_eq!(
            client
                .run(fixture("ok"), &[], OPERATION_TIMEOUT)
                .unwrap_err()
                .kind,
            WorkerFailureKind::Invalid
        );
        assert_eq!(
            client
                .run(
                    fixture("ok"),
                    &vec![b'x'; REQUEST_BYTES + 1],
                    OPERATION_TIMEOUT
                )
                .unwrap_err()
                .kind,
            WorkerFailureKind::Invalid
        );
        assert_eq!(
            client
                .run(
                    Command::new("/not/a/diagnostic-helper"),
                    b"scope",
                    OPERATION_TIMEOUT
                )
                .unwrap_err()
                .kind,
            WorkerFailureKind::Io
        );
        assert!(client.pending.is_none());
    }
    #[test]
    fn deadline_is_checked_inside_unending_output_drain() {
        struct Interrupted;
        impl Read for Interrupted {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::Interrupted.into())
            }
        }
        assert_eq!(
            drain(
                &mut Interrupted,
                &mut vec![],
                0,
                &mut false,
                Instant::now() - Duration::from_millis(1)
            ),
            Err(WorkerFailureKind::Deadline)
        );
        let mut output = vec![];
        assert_eq!(
            drain(
                &mut &b"abc"[..],
                &mut output,
                OUTPUT_BYTES - 2,
                &mut false,
                Instant::now() + OPERATION_TIMEOUT
            ),
            Err(WorkerFailureKind::OutputBudget)
        );
        assert_eq!(output, b"ab");
    }
    #[test]
    fn snapshot_principal_scope_schema_and_canonical_binding_fail_closed() {
        let good = snapshot();
        let bytes = canonical_json(&good).unwrap();
        assert_eq!(ManagerSnapshot::decode(&bytes, &scope()).unwrap(), good);
        for mutate in [
            |v: &mut ManagerSnapshot| v.manager_pid = 2,
            |v: &mut ManagerSnapshot| v.manager_uid = 1000,
            |v: &mut ManagerSnapshot| v.schema_version = 2,
            |v: &mut ManagerSnapshot| v.manager_owner = "org.freedesktop.systemd1".into(),
            |v: &mut ManagerSnapshot| v.scope.surface = "resp3".into(),
        ] {
            let mut bad = good.clone();
            mutate(&mut bad);
            assert!(ManagerSnapshot::decode(&canonical_json(&bad).unwrap(), &scope()).is_err());
        }
        let mut noncanonical = bytes.clone();
        noncanonical.push(b'\n');
        assert!(ManagerSnapshot::decode(&noncanonical, &scope()).is_err());
        assert!(ManagerSnapshot::decode(&vec![b'x'; OUTPUT_BYTES + 1], &scope()).is_err());
        for owner in ["", ":", ":1", ":1.2.3", ":1.x", "1.5", ":1..2"] {
            assert!(!unique_owner(owner));
        }
    }
    #[test]
    fn loaded_unit_identity_never_accepts_foreign_cgroup_or_generation() {
        let mut good = snapshot();
        good.unit = Some(ObservedUnit {
            unit_name: scope().unit_name(),
            object_path: "/org/freedesktop/systemd1/unit/example".into(),
            invocation_id: "ab".repeat(16),
            active_state: "active".into(),
            sub_state: "running".into(),
            main_pid: 9,
            control_group: format!("/system.slice/{}", scope().unit_name()),
            result: "success".into(),
            transient: true,
        });
        assert!(good.validate(&scope()).is_ok());
        for mutate in [
            |u: &mut ObservedUnit| u.unit_name = "foreign.service".into(),
            |u: &mut ObservedUnit| u.control_group = "/system.slice/foreign.service".into(),
            |u: &mut ObservedUnit| u.control_group.clear(),
            |u: &mut ObservedUnit| u.invocation_id = "z".repeat(32),
            |u: &mut ObservedUnit| u.object_path = "/foreign/object".into(),
            |u: &mut ObservedUnit| u.main_pid = u32::MAX,
            |u: &mut ObservedUnit| u.result = "SUCCESS".into(),
        ] {
            let mut bad = good.clone();
            mutate(bad.unit.as_mut().unwrap());
            assert!(bad.validate(&scope()).is_err());
        }
    }
    #[test]
    fn dbus_property_type_missing_and_invocation_size_refuse() {
        let mut values = HashMap::new();
        values.insert("MainPID".into(), OwnedValue::from(9u32));
        assert_eq!(take::<u32>(&mut values, "MainPID").unwrap(), 9);
        assert!(take::<u32>(&mut values, "MainPID").is_err());
        values.insert("MainPID".into(), OwnedValue::from(true));
        assert!(take::<u32>(&mut values, "MainPID").is_err());
    }
    #[test]
    fn full_unit_property_decode_uses_service_cgroup_and_exact_invocation() {
        use zbus::zvariant::Value;
        fn text(value: &str) -> OwnedValue {
            OwnedValue::try_from(Value::from(value)).unwrap()
        }
        fn properties(id_len: usize) -> (HashMap<String, OwnedValue>, HashMap<String, OwnedValue>) {
            let unit = HashMap::from([
                ("Id".into(), text(&scope().unit_name())),
                (
                    "InvocationID".into(),
                    OwnedValue::try_from(Value::from(vec![1u8; id_len])).unwrap(),
                ),
                ("ActiveState".into(), text("active")),
                ("SubState".into(), text("running")),
                ("Transient".into(), OwnedValue::from(true)),
            ]);
            let service = HashMap::from([
                ("MainPID".into(), OwnedValue::from(9u32)),
                (
                    "ControlGroup".into(),
                    text(&format!("/system.slice/{}", scope().unit_name())),
                ),
                ("Result".into(), text("success")),
            ]);
            (unit, service)
        }
        let (unit, service) = properties(16);
        let observed =
            decode_unit_properties("/org/freedesktop/systemd1/unit/example", unit, service)
                .unwrap();
        assert_eq!(observed.invocation_id, "01".repeat(16));
        let mut snapshot = snapshot();
        snapshot.unit = Some(observed);
        snapshot.validate(&scope()).unwrap();
        for length in [0, 15, 17] {
            let (unit, service) = properties(length);
            assert!(decode_unit_properties(
                "/org/freedesktop/systemd1/unit/example",
                unit,
                service
            )
            .is_err());
        }
        let (mut unit, mut service) = properties(16);
        // Putting the cgroup property on the wrong interface is not a fallback.
        unit.insert(
            "ControlGroup".into(),
            service.remove("ControlGroup").unwrap(),
        );
        assert!(
            decode_unit_properties("/org/freedesktop/systemd1/unit/example", unit, service)
                .is_err()
        );
    }
}
