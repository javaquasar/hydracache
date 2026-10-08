//! Retained read-only process-generation observation, never cleanup authority.
use crate::diagnostic_tree::DiagnosticTreeScope;
use std::ffi::CString;
use std::fs::{File, Metadata, OpenOptions};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};
use thiserror::Error;

const DOCUMENT_BYTES: usize = 65_536;
const MAX_DEPTH: usize = 8;
const MAX_COMPONENT: usize = 128;
const PROC_MAGIC: libc::c_long = 0x9fa0;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("diagnostic process scope or proc document is invalid")]
    Invalid,
    #[error("diagnostic process document exceeded its bound")]
    Budget,
    #[error("original process generation, boot, group, path or descriptor drifted")]
    Drift,
    #[error("original pidfd is terminal or uncertain; live admission refused")]
    NotLive,
    #[error("read-only process observation failed: {0}")]
    Io(#[from] std::io::Error),
}

/// Caller assertion only; authentication of the original start remains external.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedGeneration {
    pub boot_id: String,
    pub pid: u32,
    pub start_ticks: u64,
    pub cgroup_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationObservation {
    expected: ExpectedGeneration,
    process_group: u32,
}
impl GenerationObservation {
    pub fn expected(&self) -> &ExpectedGeneration {
        &self.expected
    }
    pub fn process_group(&self) -> u32 {
        self.process_group
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileId(u64, u64);
impl FileId {
    fn of(meta: &Metadata) -> Self {
        Self(meta.dev(), meta.ino())
    }
}
struct Document {
    name: String,
    file: File,
    id: FileId,
}
impl Document {
    fn open(directory: &File, name: &str) -> Result<Self, ProcessError> {
        let file = child(directory, name, false)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.dev() != directory.metadata()?.dev() {
            return Err(ProcessError::Invalid);
        }
        Ok(Self {
            name: name.into(),
            id: FileId::of(&metadata),
            file,
        })
    }
    fn read(&self, directory: &File) -> Result<Vec<u8>, ProcessError> {
        let named = child(directory, &self.name, false)?;
        if FileId::of(&named.metadata()?) != self.id
            || FileId::of(&self.file.metadata()?) != self.id
        {
            return Err(ProcessError::Drift);
        }
        let result = bounded(&self.file)?;
        if FileId::of(&self.file.metadata()?) != self.id {
            return Err(ProcessError::Drift);
        }
        Ok(result)
    }
}
struct ProcFiles {
    root: File,
    directory: File,
    id: FileId,
    pid: u32,
    stat: Document,
    cgroup: Document,
}
impl ProcFiles {
    fn open(pid: u32) -> Result<Self, ProcessError> {
        valid_pid(pid)?;
        let root = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open("/proc")?;
        require_procfs(&root)?;
        let directory = child(&root, &pid.to_string(), true)?;
        if directory.metadata()?.dev() != root.metadata()?.dev() {
            return Err(ProcessError::Invalid);
        }
        let id = FileId::of(&directory.metadata()?);
        let stat = Document::open(&directory, "stat")?;
        let cgroup = Document::open(&directory, "cgroup")?;
        Ok(Self {
            root,
            directory,
            id,
            pid,
            stat,
            cgroup,
        })
    }
    fn inspect(&self) -> Result<IdentityFields, ProcessError> {
        require_procfs(&self.root)?;
        let named_root = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open("/proc")?;
        if FileId::of(&named_root.metadata()?) != FileId::of(&self.root.metadata()?) {
            return Err(ProcessError::Drift);
        }
        let named = child(&self.root, &self.pid.to_string(), true)?;
        if FileId::of(&named.metadata()?) != self.id
            || FileId::of(&self.directory.metadata()?) != self.id
        {
            return Err(ProcessError::Drift);
        }
        let fields = parse_stat(&self.stat.read(&self.directory)?)?;
        if fields.pid != self.pid {
            return Err(ProcessError::Drift);
        }
        let cgroup = parse_cgroup(&self.cgroup.read(&self.directory)?)?;
        // A fresh read of the original stat brackets cgroup collection. Counters
        // and runnable/sleeping state may change, identity-bearing fields may not.
        let after = parse_stat(&self.stat.read(&self.directory)?)?;
        if fields != after {
            return Err(ProcessError::Drift);
        }
        Ok(IdentityFields {
            stat: fields,
            cgroup,
        })
    }
}

struct KernelProbe {
    files: ProcFiles,
    pidfd: OwnedFd,
}
trait Probe {
    fn boot(&self) -> Result<String, ProcessError>;
    fn inspect(&self) -> Result<IdentityFields, ProcessError>;
    fn live(&self) -> Result<(), ProcessError>;
}
impl Probe for KernelProbe {
    fn boot(&self) -> Result<String, ProcessError> {
        read_boot(&self.files.root)
    }
    fn inspect(&self) -> Result<IdentityFields, ProcessError> {
        self.files.inspect()
    }
    fn live(&self) -> Result<(), ProcessError> {
        let mut p = libc::pollfd {
            fd: self.pidfd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: a single initialized pollfd containing a retained descriptor.
        let n = unsafe { libc::poll(&mut p, 1, 0) };
        if n < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        if n != 0 || p.revents != 0 {
            return Err(ProcessError::NotLive);
        }
        if pidfd_pid(&self.files.root, &self.pidfd)? != self.files.pid {
            return Err(ProcessError::Drift);
        }
        Ok(())
    }
}

pub struct ProcessRead {
    observation: GenerationObservation,
    probe: KernelProbe,
}
impl ProcessRead {
    pub fn observation(&self) -> &GenerationObservation {
        &self.observation
    }
    /// Rechecks the original retained generation; never captures a replacement.
    pub fn revalidate(&self) -> Result<(), ProcessError> {
        check(&self.probe, &self.observation)
    }
}

pub fn pin_kernel_process(
    scope: &DiagnosticTreeScope,
    expected: ExpectedGeneration,
) -> Result<ProcessRead, ProcessError> {
    validate_scope(scope, &expected)?;
    pin(expected)
}
fn pin(expected: ExpectedGeneration) -> Result<ProcessRead, ProcessError> {
    let files = ProcFiles::open(expected.pid)?;
    if read_boot(&files.root)? != expected.boot_id {
        return Err(ProcessError::Drift);
    }
    let before = files.inspect()?;
    if before.stat.pid != expected.pid
        || before.stat.start_ticks != expected.start_ticks
        || before.cgroup != expected.cgroup_path
    {
        return Err(ProcessError::Drift);
    }
    // Open the pidfd only after pinning and reading the original proc generation.
    // The subsequent read through original descriptors must still succeed: these
    // descriptors never redirect to a PID replacement during the acquisition gap.
    // SAFETY: valid positive pid_t, zero flags, syscall returns a new owned FD.
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, expected.pid as libc::pid_t, 0u32) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: successful syscall returned an unowned descriptor.
    let pidfd = unsafe { OwnedFd::from_raw_fd(fd as i32) };
    let observation = GenerationObservation {
        expected,
        process_group: before.stat.group,
    };
    let result = ProcessRead {
        observation,
        probe: KernelProbe { files, pidfd },
    };
    result.revalidate()?;
    Ok(result)
}
fn check(probe: &impl Probe, observation: &GenerationObservation) -> Result<(), ProcessError> {
    probe.live()?;
    if probe.boot()? != observation.expected.boot_id {
        return Err(ProcessError::Drift);
    }
    let actual = probe.inspect()?;
    if actual.stat.pid != observation.expected.pid
        || actual.stat.start_ticks != observation.expected.start_ticks
        || actual.stat.group != observation.process_group
        || actual.cgroup != observation.expected.cgroup_path
    {
        return Err(ProcessError::Drift);
    }
    if probe.boot()? != observation.expected.boot_id {
        return Err(ProcessError::Drift);
    }
    probe.live()
}

fn validate_scope(
    scope: &DiagnosticTreeScope,
    expected: &ExpectedGeneration,
) -> Result<(), ProcessError> {
    valid_pid(expected.pid)?;
    if expected.start_ticks == 0 || expected.boot_id != scope.boot_id() {
        return Err(ProcessError::Invalid);
    }
    let root = scope
        .path()
        .strip_prefix("/sys/fs/cgroup")
        .map_err(|_| ProcessError::Invalid)?;
    let root = format!("/{}", root.to_str().ok_or(ProcessError::Invalid)?);
    if expected.cgroup_path.len() > root.len() + MAX_DEPTH * (MAX_COMPONENT + 1) {
        return Err(ProcessError::Invalid);
    }
    if expected.cgroup_path == root {
        return Ok(());
    }
    let descendant = expected
        .cgroup_path
        .strip_prefix(root.as_str())
        .and_then(|v| v.strip_prefix('/'))
        .ok_or(ProcessError::Invalid)?;
    let components: Vec<_> = descendant.split('/').collect();
    if components.len() > MAX_DEPTH || components.iter().any(|v| !safe_component(v)) {
        return Err(ProcessError::Invalid);
    }
    Ok(())
}
fn safe_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_COMPONENT
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}
fn valid_pid(pid: u32) -> Result<(), ProcessError> {
    if pid == 0 || pid > i32::MAX as u32 {
        Err(ProcessError::Invalid)
    } else {
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct StatFields {
    pid: u32,
    group: u32,
    start_ticks: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct IdentityFields {
    stat: StatFields,
    cgroup: String,
}
fn parse_stat(bytes: &[u8]) -> Result<StatFields, ProcessError> {
    if bytes.len() > DOCUMENT_BYTES {
        return Err(ProcessError::Budget);
    }
    if bytes.contains(&0) || !bytes.ends_with(b"\n") {
        return Err(ProcessError::Invalid);
    }
    let open = bytes
        .iter()
        .position(|b| *b == b'(')
        .ok_or(ProcessError::Invalid)?;
    let close = bytes
        .iter()
        .rposition(|b| *b == b')')
        .ok_or(ProcessError::Invalid)?;
    if close <= open || open < 2 || bytes[open - 1] != b' ' || bytes.get(close + 1) != Some(&b' ') {
        return Err(ProcessError::Invalid);
    }
    let pid = number(&bytes[..open - 1])?;
    let pid = u32::try_from(pid).map_err(|_| ProcessError::Invalid)?;
    valid_pid(pid)?;
    let tail = std::str::from_utf8(&bytes[close + 2..bytes.len() - 1])
        .map_err(|_| ProcessError::Invalid)?;
    if tail.contains(['\n', '\r', '\t']) {
        return Err(ProcessError::Invalid);
    }
    let fields: Vec<_> = tail.split(' ').collect();
    if fields.len() < 20 || fields.len() > 128 || fields.iter().any(|s| s.is_empty()) {
        return Err(ProcessError::Invalid);
    }
    match fields[0] {
        "Z" | "X" | "x" => return Err(ProcessError::NotLive),
        "R" | "S" | "D" | "T" | "t" | "W" | "I" => {}
        _ => return Err(ProcessError::Invalid),
    }
    let group = u32::try_from(number(fields[2].as_bytes())?).map_err(|_| ProcessError::Invalid)?;
    valid_pid(group)?;
    let start_ticks = number(fields[19].as_bytes())?;
    if start_ticks == 0 {
        return Err(ProcessError::Invalid);
    }
    Ok(StatFields {
        pid,
        group,
        start_ticks,
    })
}
fn number(bytes: &[u8]) -> Result<u64, ProcessError> {
    if bytes.is_empty()
        || bytes.len() > 20
        || !bytes.iter().all(u8::is_ascii_digit)
        || (bytes.len() > 1 && bytes[0] == b'0')
    {
        return Err(ProcessError::Invalid);
    }
    std::str::from_utf8(bytes)
        .map_err(|_| ProcessError::Invalid)?
        .parse()
        .map_err(|_| ProcessError::Invalid)
}
fn parse_cgroup(bytes: &[u8]) -> Result<String, ProcessError> {
    if bytes.len() > DOCUMENT_BYTES {
        return Err(ProcessError::Budget);
    }
    let value = std::str::from_utf8(bytes).map_err(|_| ProcessError::Invalid)?;
    let path = value
        .strip_prefix("0::")
        .and_then(|v| v.strip_suffix('\n'))
        .ok_or(ProcessError::Invalid)?;
    if !path.starts_with('/')
        || path.contains(['\n', '\r', '\0'])
        || (path != "/" && path[1..].split('/').any(|v| !safe_component(v)))
    {
        return Err(ProcessError::Invalid);
    }
    Ok(path.into())
}
fn bounded(file: &File) -> Result<Vec<u8>, ProcessError> {
    if file.metadata()?.len() > DOCUMENT_BYTES as u64 {
        return Err(ProcessError::Budget);
    }
    let mut bytes = Vec::new();
    let mut chunk = [0; 4096];
    while bytes.len() <= DOCUMENT_BYTES {
        let limit = chunk.len().min(DOCUMENT_BYTES + 1 - bytes.len());
        let n = file.read_at(&mut chunk[..limit], bytes.len() as u64)?;
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..n]);
    }
    if bytes.len() > DOCUMENT_BYTES {
        return Err(ProcessError::Budget);
    }
    if bytes.is_empty() {
        return Err(ProcessError::Invalid);
    }
    Ok(bytes)
}
fn child(directory: &File, name: &str, is_dir: bool) -> Result<File, ProcessError> {
    let name = CString::new(name).map_err(|_| ProcessError::Invalid)?;
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if is_dir { libc::O_DIRECTORY } else { 0 };
    // SAFETY: retained directory FD and live NUL-terminated single component.
    let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: openat returned a new owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn require_procfs(file: &File) -> Result<(), ProcessError> {
    // SAFETY: initialized writable struct and live descriptor.
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(file.as_raw_fd(), &mut stat) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if stat.f_type != PROC_MAGIC {
        return Err(ProcessError::Invalid);
    }
    Ok(())
}
// pidfd_open resolves a PID in the caller's namespace; procfs may be mounted
// for another namespace. fdinfo reports the pidfd task in this procfs instance.
fn pidfd_pid(root: &File, fd: &OwnedFd) -> Result<u32, ProcessError> {
    let mut name = [0u8; 32];
    let link = c"self";
    // SAFETY: retained procfs root, valid string and writable bounded buffer.
    let n = unsafe {
        libc::readlinkat(
            root.as_raw_fd(),
            link.as_ptr(),
            name.as_mut_ptr().cast(),
            name.len(),
        )
    };
    if n < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if n as usize >= name.len() {
        return Err(ProcessError::Invalid);
    }
    let own = u32::try_from(number(&name[..n as usize])?).map_err(|_| ProcessError::Invalid)?;
    valid_pid(own)?;
    let own_directory = child(root, &own.to_string(), true)?;
    let info_directory = child(&own_directory, "fdinfo", true)?;
    let info = Document::open(&info_directory, &fd.as_raw_fd().to_string())?;
    parse_pidfd_info(&info.read(&info_directory)?)
}
fn parse_pidfd_info(bytes: &[u8]) -> Result<u32, ProcessError> {
    if bytes.len() > DOCUMENT_BYTES {
        return Err(ProcessError::Budget);
    }
    let value = std::str::from_utf8(bytes).map_err(|_| ProcessError::Invalid)?;
    if !value.ends_with('\n') || value.contains(['\0', '\r']) {
        return Err(ProcessError::Invalid);
    }
    let mut matches = value.lines().filter_map(|v| v.strip_prefix("Pid:\t"));
    let pid = matches.next().ok_or(ProcessError::Invalid)?;
    if matches.next().is_some() {
        return Err(ProcessError::Invalid);
    }
    let pid = u32::try_from(number(pid.as_bytes())?).map_err(|_| ProcessError::Invalid)?;
    valid_pid(pid)?;
    Ok(pid)
}
fn read_boot(root: &File) -> Result<String, ProcessError> {
    let mut directory = root.try_clone()?;
    for name in ["sys", "kernel", "random"] {
        directory = child(&directory, name, true)?;
    }
    let file = Document::open(&directory, "boot_id")?;
    let bytes = file.read(&directory)?;
    let value = std::str::from_utf8(&bytes).map_err(|_| ProcessError::Invalid)?;
    let boot = value.strip_suffix('\n').ok_or(ProcessError::Invalid)?;
    if boot.len() != 36
        || !boot.bytes().enumerate().all(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
    {
        return Err(ProcessError::Invalid);
    }
    Ok(boot.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic_lease::DiagnosticIdentity;
    use std::cell::Cell;
    use std::io::{self, Read, Write};
    use std::os::unix::fs::symlink;
    use std::process::{Child, Command, Stdio};

    fn stat(pid: u32, group: u32, ticks: u64, state: &str) -> Vec<u8> {
        let mut fields = vec!["0".to_owned(); 50];
        fields[0] = state.into();
        fields[2] = group.to_string();
        fields[19] = ticks.to_string();
        format!("{pid} (worker ) with spaces) {}\n", fields.join(" ")).into_bytes()
    }
    fn expected() -> ExpectedGeneration {
        ExpectedGeneration {
            boot_id: "12345678-1234-1234-1234-123456789abc".into(),
            pid: 17,
            start_ticks: 31,
            cgroup_path: "/system.slice/owned.service/child".into(),
        }
    }
    fn observation() -> GenerationObservation {
        GenerationObservation {
            expected: expected(),
            process_group: 19,
        }
    }
    struct Fake {
        boot: String,
        identity: IdentityFields,
        live: bool,
        calls: Cell<usize>,
        fail: Option<usize>,
        late_boot_drift: bool,
    }
    impl Fake {
        fn new() -> Self {
            Self {
                boot: expected().boot_id,
                identity: IdentityFields {
                    stat: StatFields {
                        pid: 17,
                        group: 19,
                        start_ticks: 31,
                    },
                    cgroup: expected().cgroup_path,
                },
                live: true,
                calls: Cell::new(0),
                fail: None,
                late_boot_drift: false,
            }
        }
        fn step(&self) -> Result<usize, ProcessError> {
            let n = self.calls.get() + 1;
            self.calls.set(n);
            if self.fail == Some(n) {
                return Err(io::Error::from_raw_os_error(libc::EIO).into());
            }
            Ok(n)
        }
    }
    impl Probe for Fake {
        fn live(&self) -> Result<(), ProcessError> {
            self.step()?;
            if self.live {
                Ok(())
            } else {
                Err(ProcessError::NotLive)
            }
        }
        fn boot(&self) -> Result<String, ProcessError> {
            let n = self.step()?;
            Ok(if self.late_boot_drift && n == 4 {
                "changed".into()
            } else {
                self.boot.clone()
            })
        }
        fn inspect(&self) -> Result<IdentityFields, ProcessError> {
            self.step()?;
            Ok(self.identity.clone())
        }
    }
    #[test]
    fn stat_identity_handles_comm_bytes_and_ignores_changing_counters() {
        let mut document = stat(17, 19, 31, "S");
        document[4] = 0xff; // comm is not necessarily UTF-8.
        assert_eq!(
            parse_stat(&document).unwrap(),
            StatFields {
                pid: 17,
                group: 19,
                start_ticks: 31
            }
        );
        assert_eq!(
            parse_stat(&stat(17, 19, 31, "R")).unwrap(),
            parse_stat(&document).unwrap()
        );
    }
    #[test]
    fn stat_dead_unknown_malformed_and_overflow_documents_fail_closed() {
        for state in ["Z", "X", "x"] {
            assert!(matches!(
                parse_stat(&stat(1, 2, 3, state)),
                Err(ProcessError::NotLive)
            ));
        }
        for state in ["Q", "", "RR"] {
            assert!(parse_stat(&stat(1, 2, 3, state)).is_err());
        }
        for document in [
            b"1 (x) S\n".to_vec(),
            stat(0, 2, 3, "S"),
            stat(1, 0, 3, "S"),
            stat(1, 2, 0, "S"),
            stat(u32::MAX, 2, 3, "S"),
            vec![b'1'; DOCUMENT_BYTES + 1],
            b"01 (x) S\n".to_vec(),
        ] {
            assert!(parse_stat(&document).is_err());
        }
        for value in [
            b"+1".as_slice(),
            b"01",
            b"-1",
            b"18446744073709551616",
            b"",
            b"1 ",
        ] {
            assert!(number(value).is_err());
        }
        assert_eq!(number(b"18446744073709551615").unwrap(), u64::MAX);
    }
    #[test]
    fn unified_cgroup_is_single_canonical_and_bounded() {
        assert_eq!(
            parse_cgroup(b"0::/system.slice/test.service/child\n").unwrap(),
            "/system.slice/test.service/child"
        );
        assert_eq!(parse_cgroup(b"0::/\n").unwrap(), "/");
        for value in [
            "0::/a\n0::/b\n",
            "1::/a\n",
            "0::/a",
            "0:://a\n",
            "0::/a/../b\n",
            "0::/a/./b\n",
            "0::/a/\n",
            "0::/a\r\n",
            "0::/a\0\n",
        ] {
            assert!(parse_cgroup(value.as_bytes()).is_err(), "{value:?}");
        }
        assert!(parse_cgroup(&vec![b'x'; DOCUMENT_BYTES + 1]).is_err());
    }
    #[test]
    fn diagnostic_scope_refuses_foreign_prefix_boot_and_depth() {
        let identity = DiagnosticIdentity {
            lease_id: "a".repeat(64),
            boot_id: expected().boot_id,
            binary_sha256: "b".repeat(64),
            build_provenance_sha256: "c".repeat(64),
        };
        let scope = DiagnosticTreeScope::new(&identity, "embedded").unwrap();
        let root = format!(
            "/{}",
            scope
                .path()
                .strip_prefix("/sys/fs/cgroup")
                .unwrap()
                .display()
        );
        let mut e = expected();
        e.cgroup_path = root.clone();
        validate_scope(&scope, &e).unwrap();
        e.cgroup_path = format!("{root}/{}", ["child"; MAX_DEPTH].join("/"));
        validate_scope(&scope, &e).unwrap();
        for tail in ["-suffix", "/../other", "/a//b", "/a/", "/.", "/bad:name"] {
            e.cgroup_path = format!("{root}{tail}");
            assert!(pin_kernel_process(&scope, e.clone()).is_err());
        }
        e.cgroup_path = format!("{root}/{}", ["child"; MAX_DEPTH + 1].join("/"));
        assert!(validate_scope(&scope, &e).is_err());
        e.cgroup_path = root;
        e.boot_id = "other".into();
        assert!(validate_scope(&scope, &e).is_err());
    }
    #[test]
    fn original_generation_group_cgroup_and_boot_drift_never_refresh() {
        check(&Fake::new(), &observation()).unwrap();
        for case in 0..6 {
            let mut f = Fake::new();
            match case {
                0 => f.identity.stat.pid += 1,
                1 => f.identity.stat.start_ticks += 1,
                2 => f.identity.stat.group += 1,
                3 => f.identity.cgroup = "/other".into(),
                4 => f.boot = "reboot".into(),
                _ => f.late_boot_drift = true,
            }
            assert!(matches!(
                check(&f, &observation()),
                Err(ProcessError::Drift)
            ));
            assert_eq!(observation().expected.start_ticks, 31);
        }
    }
    #[test]
    fn every_read_poll_failure_and_terminal_state_refuses_without_retry() {
        for n in 1..=5 {
            let mut f = Fake::new();
            f.fail = Some(n);
            assert!(matches!(
                check(&f, &observation()),
                Err(ProcessError::Io(_))
            ));
            assert_eq!(f.calls.get(), n);
        }
        let mut f = Fake::new();
        f.live = false;
        assert!(matches!(
            check(&f, &observation()),
            Err(ProcessError::NotLive)
        ));
        assert_eq!(f.calls.get(), 1);
    }
    #[test]
    fn synthetic_documents_cannot_pass_procfs_gate_and_substitution_is_detected() {
        let temp = tempfile::tempdir().unwrap();
        let directory = File::open(temp.path()).unwrap();
        assert!(matches!(
            require_procfs(&directory),
            Err(ProcessError::Invalid)
        ));
        let path = temp.path().join("stat");
        std::fs::write(&path, stat(1, 2, 3, "S")).unwrap();
        let doc = Document::open(&directory, "stat").unwrap();
        doc.read(&directory).unwrap();
        std::fs::rename(&path, temp.path().join("old")).unwrap();
        std::fs::write(&path, stat(1, 2, 3, "S")).unwrap();
        assert!(matches!(doc.read(&directory), Err(ProcessError::Drift)));
        std::fs::remove_file(&path).unwrap();
        symlink("old", &path).unwrap();
        assert!(Document::open(&directory, "stat").is_err());
        let file = File::open(temp.path().join("old")).unwrap();
        assert!(child(&file, "stat", false).is_err());
    }
    #[test]
    fn bounded_document_reads_accept_limit_and_refuse_overflow_or_empty() {
        let mut f = tempfile::tempfile().unwrap();
        f.write_all(&vec![b'a'; DOCUMENT_BYTES]).unwrap();
        assert_eq!(bounded(&f).unwrap().len(), DOCUMENT_BYTES);
        f.write_all(b"x").unwrap();
        assert!(matches!(bounded(&f), Err(ProcessError::Budget)));
        f.set_len(0).unwrap();
        assert!(bounded(&f).is_err());
        assert!(safe_component(&"x".repeat(MAX_COMPONENT)));
        assert!(!safe_component(&"x".repeat(MAX_COMPONENT + 1)));
    }
    #[test]
    fn seeded_stat_generations_keep_identity_fields_exact() {
        let mut seed = 740074u64;
        for _ in 0..256 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let pid = ((seed >> 32) as u32 % 1_000_000) + 1;
            let ticks = seed.max(1);
            let fields = parse_stat(&stat(pid, pid, ticks, "S")).unwrap();
            assert_eq!(
                fields,
                StatFields {
                    pid,
                    group: pid,
                    start_ticks: ticks
                }
            );
        }
    }
    #[test]
    fn pidfd_namespace_mapping_is_exact_and_missing_or_dead_is_refused() {
        assert_eq!(
            parse_pidfd_info(b"pos:\t0\nflags:\t02000002\nPid:\t17\nNSpid:\t17\t1\n").unwrap(),
            17
        );
        for value in [
            "Pid:\t0\n",
            "Pid:\t-1\n",
            "Pid:\t01\n",
            "Pid:\t17\nPid:\t18\n",
            "NSpid:\t17\n",
            "Pid: 17\n",
            "Pid:\t2147483648\n",
            "Pid:\t17",
            "Pid:\t17\r\n",
        ] {
            assert!(parse_pidfd_info(value.as_bytes()).is_err());
        }
        assert!(parse_pidfd_info(&vec![b'x'; DOCUMENT_BYTES + 1]).is_err());
    }
    fn live_expected(pid: u32) -> ExpectedGeneration {
        let f = ProcFiles::open(pid).unwrap();
        let i = f.inspect().unwrap();
        ExpectedGeneration {
            boot_id: read_boot(&f.root).unwrap(),
            pid,
            start_ticks: i.stat.start_ticks,
            cgroup_path: i.cgroup,
        }
    }
    #[test]
    fn local_self_pidfd_and_original_proc_descriptors_revalidate() {
        let expected = live_expected(std::process::id());
        let r = pin(expected.clone()).unwrap();
        r.revalidate().unwrap();
        assert_eq!(r.observation().expected(), &expected);
        let mut wrong = expected;
        wrong.start_ticks += 1;
        assert!(matches!(pin(wrong), Err(ProcessError::Drift)));
        assert!(ProcFiles::open(0).is_err());
        assert!(ProcFiles::open(u32::MAX).is_err());
        let flags = unsafe { libc::fcntl(r.probe.pidfd.as_raw_fd(), libc::F_GETFD) };
        assert_ne!(flags & libc::FD_CLOEXEC, 0);
    }
    struct Helper(Child);
    impl Drop for Helper {
        fn drop(&mut self) {
            self.0.stdin.take();
            let _ = self.0.wait();
        }
    }
    #[test]
    fn local_nonproduct_helper_exit_cannot_recapture_original_generation() {
        let mut helper = Helper(
            Command::new("/bin/cat")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap(),
        );
        helper.0.stdin.as_mut().unwrap().write_all(b"x").unwrap();
        let mut byte = [0];
        helper
            .0
            .stdout
            .as_mut()
            .unwrap()
            .read_exact(&mut byte)
            .unwrap();
        assert_eq!(byte, [b'x']);
        let r = pin(live_expected(helper.0.id())).unwrap();
        r.revalidate().unwrap();
        helper.0.stdin.take();
        assert!(helper.0.wait().unwrap().success());
        assert!(matches!(r.revalidate(), Err(ProcessError::NotLive)));
        assert!(r.probe.files.inspect().is_err());
        assert!(pin(r.observation.expected.clone()).is_err());
    }
    #[test]
    fn parallel_readonly_revalidation_has_no_shared_seek_offset() {
        let r = pin(live_expected(std::process::id())).unwrap();
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let r = &r;
                scope.spawn(move || {
                    for _ in 0..16 {
                        r.revalidate().unwrap();
                    }
                });
            }
        });
    }
}
