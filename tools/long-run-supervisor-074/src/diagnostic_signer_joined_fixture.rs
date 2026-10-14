//! Test-only fresh GNU helper exec followed by exact filter installation.
use super::diagnostic_signer_first_exec_fixture::{selected, Stamp};
use super::diagnostic_signer_seccomp_fixture::{decode, encoded, policy, Event};
use sha2::{Digest, Sha256};
use std::ffi::CStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::marker::PhantomData;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const ARCH: u32 = 0xc000_003e;
const EXEC_STOP: i32 = (libc::PTRACE_EVENT_EXEC << 16) | (libc::SIGTRAP << 8) | 0x7f;
const SYSCALL_STOP: i32 = ((libc::SIGTRAP | 0x80) << 8) | 0x7f;
const DEADLINE: Duration = Duration::from_secs(5);
const HELPER: &str = include_str!("diagnostic_signer_joined_helper.rs");
const MAX_IMAGE: usize = 8 * 1024 * 1024;

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "owned joined fixture refused")
}
fn read_image(path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_IMAGE as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() < 64 || bytes.len() > MAX_IMAGE {
        return Err(invalid());
    }
    Ok(bytes)
}
fn compiled_helper() -> io::Result<Vec<u8>> {
    let home = std::env::var_os("HOME").ok_or_else(invalid)?;
    let rustup = PathBuf::from(&home).join(".cargo/bin/rustup");
    let tool = Command::new(rustup)
        .args(["which", "--toolchain", "1.94.0", "rustc"])
        .env_clear()
        .env("HOME", home)
        .output()?;
    if !tool.status.success() {
        return Err(invalid());
    }
    let compiler = PathBuf::from(
        String::from_utf8(tool.stdout)
            .map_err(|_| invalid())?
            .trim(),
    );
    if !compiler.is_absolute() || !compiler.is_file() {
        return Err(invalid());
    }
    let version = Command::new(&compiler)
        .arg("--version")
        .env_clear()
        .output()?;
    if !version.status.success() || !version.stdout.starts_with(b"rustc 1.94.0 ") {
        return Err(invalid());
    }
    let temp = tempfile::tempdir()?;
    let source = temp.path().join("helper.rs");
    let binary = temp.path().join("helper");
    fs::write(&source, HELPER)?;
    let result = Command::new(compiler)
        .arg(&source)
        .args([
            "--edition=2021",
            "--target=x86_64-unknown-linux-gnu",
            "-Dwarnings",
            "-Copt-level=3",
            "-Cpanic=abort",
            "-Cdebuginfo=0",
        ])
        .arg(format!(
            "--remap-path-prefix={}=/owned-joined-fixture",
            temp.path().display()
        ))
        .arg("-o")
        .arg(&binary)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("TMPDIR", temp.path())
        .output()?;
    if !result.status.success() {
        return Err(io::Error::other(format!(
            "fixed helper compile failed: {}",
            String::from_utf8_lossy(&result.stderr[..result.stderr.len().min(8192)])
        )));
    }
    let bytes = read_image(&binary)?;
    let digest: String = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    println!("fixture_gnu_helper_sha256={digest} compiler=Rust1.94 profile=O3_abort runtime_admission=false");
    Ok(bytes)
}
struct Fixture {
    _snapshot: Option<Snapshot>,
    _temp: tempfile::TempDir,
    binary: PathBuf,
    output: PathBuf,
}
impl Fixture {
    fn new() -> io::Result<Self> {
        Self::new_with_snapshot(false)
    }
    fn new_with_snapshot(snapshot: bool) -> io::Result<Self> {
        // SAFETY: no-argument identity query; never launch these fixtures as root.
        if unsafe { libc::geteuid() } == 0 {
            return Err(invalid());
        }
        static COMPILED: OnceLock<Result<Vec<u8>, String>> = OnceLock::new();
        let bytes = COMPILED.get_or_init(|| compiled_helper().map_err(|e| e.to_string()));
        let bytes = bytes.as_ref().map_err(|e| io::Error::other(e.clone()))?;
        let temp = tempfile::tempdir()?;
        let binary = temp.path().join("owned-joined-helper");
        // The parent must never hold a writable executable-file description:
        // an in-flight raw fork can retain it before reaching close_range.
        let mut copier = Command::new("/usr/bin/tee")
            .arg("--")
            .arg(&binary)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        let written = copier.stdin.take().ok_or_else(invalid)?.write_all(bytes);
        let copied = copier.wait()?;
        written?;
        if !copied.success() || read_image(&binary)? != *bytes {
            return Err(invalid());
        }
        let snapshot = if snapshot {
            Some(Snapshot::new(None)?)
        } else {
            None
        };
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o555))?;
        Ok(Self {
            _snapshot: snapshot,
            output: temp.path().join("output"),
            binary,
            _temp: temp,
        })
    }
}

struct Snapshot {
    pid: libc::pid_t,
    pidfd: Option<OwnedFd>,
    reaped: bool,
}
impl Snapshot {
    fn new(writer: Option<i32>) -> io::Result<Self> {
        if writer.is_some_and(|fd| fd < 3) {
            return Err(invalid());
        }
        // SAFETY: child only closes its copied table, stops and _exits. No Rust IO/locks.
        let pid = unsafe { libc::fork() };
        if pid < 0 {
            return Err(io::Error::last_os_error());
        }
        if pid == 0 {
            // SAFETY: preserve only fixed selected fixture writer, if present.
            unsafe {
                let keep = writer.map(|fd| fd as u32);
                let failed = match keep {
                    Some(fd) => {
                        (fd > 3 && libc::syscall(libc::SYS_close_range, 3u32, fd - 1, 0u32) < 0)
                            || libc::syscall(libc::SYS_close_range, fd + 1, u32::MAX, 0u32) < 0
                    }
                    None => libc::syscall(libc::SYS_close_range, 3u32, u32::MAX, 0u32) < 0,
                };
                if failed {
                    libc::_exit(80);
                }
                libc::kill(libc::getpid(), libc::SIGSTOP);
                libc::_exit(81);
            }
        }
        let mut child = Self {
            pid,
            pidfd: None,
            reaped: false,
        };
        // SAFETY: original owned unreaped snapshot child, never supplied/adopted.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0u32) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful syscall created one owned descriptor.
        child.pidfd = Some(unsafe { OwnedFd::from_raw_fd(fd as i32) });
        if child.wait()? != (libc::SIGSTOP << 8) | 0x7f {
            return Err(invalid());
        }
        Ok(child)
    }
    fn wait(&mut self) -> io::Result<i32> {
        let deadline = Instant::now() + DEADLINE;
        loop {
            let mut status = 0;
            // SAFETY: initialized output and exact own unreaped child, never wait-any.
            let found =
                unsafe { libc::waitpid(self.pid, &mut status, libc::WNOHANG | libc::WUNTRACED) };
            if found == self.pid {
                self.reaped = libc::WIFEXITED(status) || libc::WIFSIGNALED(status);
                return Ok(status);
            }
            if found < 0 && io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
                return Err(io::Error::last_os_error());
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "owned snapshot stalled",
                ));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    fn terminate(&mut self) -> io::Result<()> {
        if self.reaped {
            return Ok(());
        }
        // SAFETY: exact retained own pidfd; PID fallback only for failed acquisition.
        let result = unsafe {
            match &self.pidfd {
                Some(fd) => libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    fd.as_raw_fd(),
                    libc::SIGKILL,
                    0usize,
                    0u32,
                ),
                None => libc::kill(self.pid, libc::SIGKILL) as libc::c_long,
            }
        };
        if result < 0 && io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
            return Err(io::Error::last_os_error());
        }
        while !self.reaped {
            self.wait()?;
        }
        Ok(())
    }
}
impl Drop for Snapshot {
    fn drop(&mut self) {
        if let Err(error) = self.terminate() {
            eprintln!("owned snapshot cleanup failed: {error}");
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    SelfExec,
    OtherExec,
    FdExec,
    Flags,
    Policy,
    Length,
    Header,
    Pointer,
    EarlyExec,
}
impl Case {
    fn argument(self) -> &'static CStr {
        match self {
            Self::SelfExec => c"self",
            Self::OtherExec => c"other",
            Self::FdExec => c"fd",
            Self::Flags => c"flags",
            Self::Policy => c"policy",
            Self::Length => c"length",
            Self::Header => c"header",
            Self::Pointer => c"pointer",
            Self::EarlyExec => c"early-exec",
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
enum Phase {
    Initial,
    FirstExecStopped,
    Installed,
    Refused,
    Terminal,
}
struct Joined {
    pid: libc::pid_t,
    pidfd: Option<OwnedFd>,
    expected: File,
    stamp: Stamp,
    output: File,
    case: Case,
    phase: Phase,
    reaped: bool,
    _thread: PhantomData<Rc<()>>,
}
fn stack_bounds(raw: &str) -> io::Result<(usize, usize)> {
    if raw.len() > 65536 {
        return Err(invalid());
    }
    let mut found = None;
    for line in raw.lines().filter(|line| line.ends_with("[stack]")) {
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() != 6
            || parts[1] != "rw-p"
            || parts[2] != "00000000"
            || parts[3] != "00:00"
            || parts[4] != "0"
            || found.is_some()
        {
            return Err(invalid());
        }
        let (lo, hi) = parts[0].split_once('-').ok_or_else(invalid)?;
        let lo = usize::from_str_radix(lo, 16).map_err(|_| invalid())?;
        let hi = usize::from_str_radix(hi, 16).map_err(|_| invalid())?;
        if lo == 0 || lo >= hi || lo % 4096 != 0 || hi % 4096 != 0 {
            return Err(invalid());
        }
        found = Some((lo, hi));
    }
    found.ok_or_else(invalid)
}
fn within(bounds: (usize, usize), address: usize, count: usize) -> bool {
    count > 0
        && count <= 272
        && address >= bounds.0
        && address
            .checked_add(count)
            .is_some_and(|end| end <= bounds.1)
}
fn startup_allowed(nr: u64) -> bool {
    nr < 1024
        && ![
            libc::SYS_execve,
            libc::SYS_execveat,
            libc::SYS_fork,
            libc::SYS_vfork,
            libc::SYS_clone,
            libc::SYS_clone3,
            libc::SYS_ptrace,
            libc::SYS_prctl,
            libc::SYS_seccomp,
            libc::SYS_exit,
            libc::SYS_exit_group,
        ]
        .contains(&(nr as i64))
}
impl Joined {
    fn launch(fixture: &Fixture, case: Case) -> io::Result<Self> {
        let expected = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&fixture.binary)?;
        let stamp = selected(&expected)?;
        let input = File::open("/dev/null")?;
        let writer = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_CLOEXEC)
            .open(&fixture.output)?;
        let output = File::open(&fixture.output)?;
        let fds = [input.as_raw_fd(), writer.as_raw_fd(), expected.as_raw_fd()];
        if fds.iter().any(|fd| *fd < 3) {
            return Err(invalid());
        }
        let argv = [
            c"owned-joined-helper".as_ptr(),
            case.argument().as_ptr(),
            std::ptr::null(),
        ];
        let env = [std::ptr::null::<libc::c_char>()];
        // SAFETY: prepared fixed inputs; child_exec uses libc/_exit only after fork.
        let pid = unsafe { libc::fork() };
        if pid < 0 {
            return Err(io::Error::last_os_error());
        }
        if pid == 0 {
            child_exec(fds, &argv, &env);
        }
        drop(input);
        drop(writer);
        let mut owner = Self {
            pid,
            pidfd: None,
            expected,
            stamp,
            output,
            case,
            phase: Phase::Initial,
            reaped: false,
            _thread: PhantomData,
        };
        // SAFETY: original own direct child, unreaped, never an adopted PID.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0u32) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful syscall created this unowned descriptor.
        owner.pidfd = Some(unsafe { OwnedFd::from_raw_fd(fd as i32) });
        if owner.wait()? != (libc::SIGSTOP << 8) | 0x7f {
            return Err(invalid());
        }
        owner.trace(
            libc::PTRACE_SETOPTIONS,
            (libc::PTRACE_O_TRACEEXEC | libc::PTRACE_O_TRACESYSGOOD | libc::PTRACE_O_EXITKILL)
                as usize,
        )?;
        owner.trace(libc::PTRACE_CONT, 0)?;
        let status = owner.wait()?;
        if status != EXEC_STOP {
            return Err(io::Error::other(format!(
                "joined first exec refused: {status:#x}"
            )));
        }
        owner.check_object(&owner.expected)?;
        owner.one_task()?;
        let mut names = fs::read_dir(format!("/proc/{pid}/fd"))?
            .take(4)
            .map(|e| e.map(|e| e.file_name()))
            .collect::<io::Result<Vec<_>>>()?;
        names.sort();
        if names != ["0", "1", "2"].map(std::ffi::OsString::from)
            || owner.output.metadata()?.len() != 0
        {
            return Err(invalid());
        }
        owner.phase = Phase::FirstExecStopped;
        Ok(owner)
    }
    fn check_object(&self, expected: &File) -> io::Result<()> {
        let current = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_CLOEXEC)
            .open(format!("/proc/{}/exe", self.pid))?;
        if selected(expected)? != self.stamp
            || Stamp::of(&current.metadata()?) != self.stamp
            || Stamp::of(&current.metadata()?) != self.stamp
        {
            return Err(invalid());
        }
        Ok(())
    }
    fn one_task(&self) -> io::Result<()> {
        let entries = fs::read_dir(format!("/proc/{}/task", self.pid))?
            .take(2)
            .map(|e| e.map(|e| e.file_name()))
            .collect::<io::Result<Vec<_>>>()?;
        if entries != [std::ffi::OsString::from(self.pid.to_string())] {
            return Err(invalid());
        }
        Ok(())
    }
    fn wait(&mut self) -> io::Result<i32> {
        let deadline = Instant::now() + DEADLINE;
        loop {
            if self.reaped {
                return Err(invalid());
            }
            let mut status = 0;
            // SAFETY: exact own child and initialized output, never wait-any.
            let found =
                unsafe { libc::waitpid(self.pid, &mut status, libc::WNOHANG | libc::__WALL) };
            if found == self.pid {
                self.reaped = libc::WIFEXITED(status) || libc::WIFSIGNALED(status);
                return Ok(status);
            }
            if found < 0 && io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
                return Err(io::Error::last_os_error());
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "joined fixture stalled",
                ));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    fn trace(&self, op: u32, data: usize) -> io::Result<()> {
        // SAFETY: fixed scalar ptrace operation on the exact stopped original.
        if unsafe { libc::ptrace(op, self.pid, 0usize, data) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    fn event(&mut self) -> io::Result<Event> {
        self.trace(libc::PTRACE_SYSCALL, 0)?;
        let status = self.wait()?;
        if status != SYSCALL_STOP {
            return Err(io::Error::other(format!(
                "joined syscall stop refused: {status:#x}"
            )));
        }
        // SAFETY: zero integral/union storage valid, sized kernel output buffer.
        let mut info: libc::ptrace_syscall_info = unsafe { std::mem::zeroed() };
        // SAFETY: original stopped child, writable initialized correctly sized buffer.
        let size = unsafe {
            libc::ptrace(
                libc::PTRACE_GET_SYSCALL_INFO,
                self.pid,
                std::mem::size_of_val(&info),
                &mut info,
            )
        };
        if size < 0 {
            return Err(io::Error::last_os_error());
        }
        decode(&info, size as usize)
    }
    fn read_stack(
        &self,
        bounds: (usize, usize),
        address: usize,
        count: usize,
    ) -> io::Result<Vec<u8>> {
        if !within(bounds, address, count) {
            return Err(invalid());
        }
        let start = address & !7;
        let end = address.checked_add(count).ok_or_else(invalid)?;
        let padded_end = end.checked_add(7).ok_or_else(invalid)? & !7;
        if start < bounds.0 || padded_end > bounds.1 {
            return Err(invalid());
        }
        let mut out = Vec::with_capacity(count + 14);
        for cursor in (start..padded_end).step_by(8) {
            // SAFETY: tracer-thread errno and bounded original stopped stack read.
            unsafe {
                *libc::__errno_location() = 0;
            }
            // SAFETY: word entirely in the selected original stack interval.
            let word = unsafe { libc::ptrace(libc::PTRACE_PEEKDATA, self.pid, cursor, 0usize) };
            // SAFETY: same tracer-thread errno; valid -1 data is not an error.
            let errno = unsafe { *libc::__errno_location() };
            if word == -1 && errno != 0 {
                return Err(io::Error::from_raw_os_error(errno));
            }
            out.extend_from_slice(&word.to_ne_bytes());
        }
        let offset = address - start;
        Ok(out[offset..offset + count].to_vec())
    }
    fn memory(&self, bounds: (usize, usize), header: usize, program: usize) -> io::Result<()> {
        let raw = self.read_stack(bounds, header, 16)?;
        if u16::from_le_bytes(raw[..2].try_into().unwrap()) != 34
            || u64::from_le_bytes(raw[8..16].try_into().unwrap()) != program as u64
            || self.read_stack(bounds, program, 272)? != encoded(&policy())
        {
            return Err(invalid());
        }
        Ok(())
    }
    fn install(&mut self) -> io::Result<()> {
        let result = self.install_inner();
        if result.is_err() {
            self.refuse()?;
        }
        result
    }
    fn install_inner(&mut self) -> io::Result<()> {
        if self.phase != Phase::FirstExecStopped || self.output.metadata()?.len() != 0 {
            return Err(invalid());
        }
        self.check_object(&self.expected)?;
        self.one_task()?;
        let deadline = Instant::now() + DEADLINE;
        let mut event = self.event()?;
        // Only the successful original descriptor-exec exit may precede startup.
        if event
            == (Event::Exit {
                arch: ARCH,
                value: 0,
                error: false,
            })
        {
            event = self.event()?;
        }
        let mut installer = None;
        for _ in 0..256 {
            if Instant::now() >= deadline {
                return Err(invalid());
            }
            match event {
                Event::Entry {
                    arch: ARCH,
                    nr,
                    args,
                } if nr == libc::SYS_seccomp as u64 => {
                    if args[0] != libc::SECCOMP_SET_MODE_FILTER as u64
                        || args[1] != libc::SECCOMP_FILTER_FLAG_TSYNC
                    {
                        return Err(invalid());
                    }
                    installer = Some(usize::try_from(args[2]).map_err(|_| invalid())?);
                    break;
                }
                Event::Entry { arch: ARCH, nr, .. } => {
                    if !startup_allowed(nr) {
                        return Err(invalid());
                    }
                    if !matches!(self.event()?, Event::Exit { arch: ARCH, .. }) {
                        return Err(invalid());
                    }
                    event = self.event()?;
                }
                _ => return Err(invalid()),
            }
        }
        let header = installer.ok_or_else(invalid)?;
        self.one_task()?;
        let mut maps = String::new();
        File::open(format!("/proc/{}/maps", self.pid))?
            .take(65537)
            .read_to_string(&mut maps)?;
        let bounds = stack_bounds(&maps)?;
        let raw = self.read_stack(bounds, header, 16)?;
        let program = usize::try_from(u64::from_le_bytes(raw[8..16].try_into().unwrap()))
            .map_err(|_| invalid())?;
        self.memory(bounds, header, program)?;
        if self.event()?
            != (Event::Exit {
                arch: ARCH,
                value: 0,
                error: false,
            })
        {
            return Err(invalid());
        }
        self.memory(bounds, header, program)?;
        self.one_task()?;
        self.check_object(&self.expected)?;
        match self.event()? {
            Event::Entry {
                arch: ARCH,
                nr,
                args,
            } if nr == libc::SYS_prctl as u64
                && args[0] == libc::PR_SET_DUMPABLE as u64
                && args[1] == 0 => {}
            _ => return Err(invalid()),
        }
        if self.event()?
            != (Event::Exit {
                arch: ARCH,
                value: 0,
                error: false,
            })
        {
            return Err(invalid());
        }
        self.phase = Phase::Installed;
        Ok(())
    }
    fn denied_probe(&mut self, case: Case) -> io::Result<()> {
        let result = self.denied_inner(case);
        if result.is_err() {
            self.refuse()?;
        }
        result
    }
    fn denied_inner(&mut self, case: Case) -> io::Result<()> {
        if self.phase != Phase::Installed || self.case != case {
            return Err(invalid());
        }
        let wanted = match case {
            Case::SelfExec | Case::OtherExec => libc::SYS_execve,
            Case::FdExec => libc::SYS_execveat,
            _ => return Err(invalid()),
        };
        match self.event()? {
            Event::Entry { arch: ARCH, nr, .. } if nr == wanted as u64 => {}
            _ => return Err(invalid()),
        }
        if self.event()?
            != (Event::Exit {
                arch: ARCH,
                value: -(libc::EPERM as i64),
                error: true,
            })
        {
            return Err(invalid());
        }
        match self.event()? {
            Event::Entry {
                arch: ARCH,
                nr,
                args,
            } if nr == libc::SYS_exit_group as u64 && args[0] == 0 => {}
            _ => return Err(invalid()),
        }
        self.trace(libc::PTRACE_SYSCALL, 0)?;
        if self.wait()? != 0 || self.output.metadata()?.len() != 0 {
            return Err(invalid());
        }
        self.phase = Phase::Terminal;
        Ok(())
    }
    fn signal_original(&self) -> io::Result<()> {
        // SAFETY: exact owned pidfd; fallback only if original fork acquisition failed.
        let result = unsafe {
            match &self.pidfd {
                Some(fd) => libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    fd.as_raw_fd(),
                    libc::SIGKILL,
                    0usize,
                    0u32,
                ),
                None => libc::kill(self.pid, libc::SIGKILL) as libc::c_long,
            }
        };
        if result < 0 && io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    fn refuse(&mut self) -> io::Result<()> {
        self.phase = Phase::Refused;
        self.terminate()
    }
    fn terminate(&mut self) -> io::Result<()> {
        if self.reaped {
            return Ok(());
        }
        self.signal_original()?;
        while !self.reaped {
            self.wait()?;
        }
        Ok(())
    }
}
impl Drop for Joined {
    fn drop(&mut self) {
        if let Err(error) = self.terminate() {
            eprintln!("joined fixture cleanup failed: {error}");
        }
    }
}
fn child_exec(fds: [i32; 3], argv: &[*const libc::c_char; 3], env: &[*const libc::c_char; 1]) -> ! {
    // SAFETY: prepared descriptors/static strings; no Rust allocation/locks/unwind
    // after raw fork. Fresh helper Rust startup occurs only after successful exec.
    unsafe {
        let limit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if libc::dup2(fds[0], 0) < 0 || libc::dup2(fds[1], 1) < 0 || libc::dup2(fds[1], 2) < 0 {
            libc::_exit(70);
        }
        let keep = fds[2] as u32;
        if (keep > 3 && libc::syscall(libc::SYS_close_range, 3u32, keep - 1, 0u32) < 0)
            || libc::syscall(libc::SYS_close_range, keep + 1, u32::MAX, 0u32) < 0
            || libc::setrlimit(libc::RLIMIT_CORE, &limit) < 0
            || libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1usize, 0usize, 0usize, 0usize) < 0
            || libc::ptrace(libc::PTRACE_TRACEME, 0, 0usize, 0usize) < 0
            || libc::kill(libc::getpid(), libc::SIGSTOP) < 0
        {
            libc::_exit(71);
        }
        libc::syscall(
            libc::SYS_execveat,
            fds[2],
            c"".as_ptr(),
            argv.as_ptr(),
            env.as_ptr(),
            libc::AT_EMPTY_PATH,
        );
        let errno = *libc::__errno_location();
        libc::_exit(if (1..128).contains(&errno) {
            128 + errno
        } else {
            72
        });
    }
}

#[path = "diagnostic_signer_joined_fixture_tests.rs"]
mod tests;
