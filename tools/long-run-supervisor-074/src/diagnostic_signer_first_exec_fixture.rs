//! Test-build only. Kernel exec stops for owned fixtures, never signing authority.
use std::ffi::CString;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Write};
use std::marker::PhantomData;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

const PROGRAM: &str = "printf 'ready\\n'; while IFS= read -r line; do case \"$line\" in other) exec /bin/cat;; same) exec \"$SELF\" -c \"$PROGRAM\";; *) printf 'ack\\n';; esac; done";
const DEADLINE: Duration = Duration::from_secs(5);
const EXEC_STOP: i32 = (libc::PTRACE_EVENT_EXEC << 16) | (libc::SIGTRAP << 8) | 0x7f;

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "owned first-exec fixture refused",
    )
}
fn is_exec_stop(status: i32) -> bool {
    status == EXEC_STOP
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Stamp {
    dev: u64,
    ino: u64,
    uid: u32,
    gid: u32,
    mode: u32,
    links: u64,
    len: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}
impl Stamp {
    fn of(m: &Metadata) -> Self {
        Self {
            dev: m.dev(),
            ino: m.ino(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode(),
            links: m.nlink(),
            len: m.len(),
            modified: (m.mtime(), m.mtime_nsec()),
            changed: (m.ctime(), m.ctime_nsec()),
        }
    }
}
fn selected(file: &File) -> io::Result<Stamp> {
    // SAFETY: flag query on a borrowed live descriptor, no ownership transfer.
    let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
    let m = file.metadata()?;
    // SAFETY: effective identity queries have no pointer arguments or mutation.
    let uid = unsafe { libc::geteuid() };
    if flags < 0
        || flags & libc::O_ACCMODE != libc::O_RDONLY
        || flags & libc::O_PATH != 0
        || !m.is_file()
        || m.nlink() != 1
        || m.len() < 64
        || m.mode() & 0o7777 != 0o555
        || uid == 0
        || m.uid() != uid
    {
        return Err(invalid());
    }
    let mut header = [0u8; 64];
    file.read_exact_at(&mut header, 0)?;
    if &header[..7] != b"\x7fELF\x02\x01\x01"
        || ![2, 3].contains(&u16::from_le_bytes([header[16], header[17]]))
        || u16::from_le_bytes([header[18], header[19]]) != 62
        || u32::from_le_bytes(header[20..24].try_into().unwrap()) != 1
        || u16::from_le_bytes([header[52], header[53]]) != 64
    {
        return Err(invalid());
    }
    let stamp = Stamp::of(&m);
    if Stamp::of(&file.metadata()?) != stamp {
        return Err(invalid());
    }
    Ok(stamp)
}

struct Fixture {
    _temp: tempfile::TempDir,
    binary: PathBuf,
    output: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let binary = temp.path().join("owned-shell");
        fs::copy("/bin/sh", &binary).unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o555)).unwrap();
        Self {
            output: temp.path().join("output"),
            binary,
            _temp: temp,
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
enum Phase {
    Initial,
    FirstExecStopped,
    Running,
    Refused,
    Terminal,
}
enum Control {
    Ack,
    Same,
    Other,
}
// No Send/Sync: Linux tracing belongs to the creating tracer thread.
struct OwnedExec {
    pid: libc::pid_t,
    pidfd: Option<OwnedFd>,
    expected: File,
    stamp: Stamp,
    input: Option<File>,
    output: File,
    phase: Phase,
    reaped: bool,
    last_status: Option<i32>,
    _thread_bound: PhantomData<Rc<()>>,
}
fn pipe() -> io::Result<(OwnedFd, OwnedFd)> {
    let mut fds = [-1; 2];
    // SAFETY: valid two-int output storage; success creates two owned FDs.
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: each successful pipe descriptor is transferred exactly once.
    let pair = unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
    if fds.iter().any(|fd| *fd < 3) {
        return Err(invalid());
    }
    Ok(pair)
}
fn trace(request: libc::c_uint, pid: libc::pid_t, data: usize) -> io::Result<()> {
    // SAFETY: only ptrace operations with unused addr and scalar data are used.
    if unsafe { libc::ptrace(request, pid, std::ptr::null_mut::<libc::c_void>(), data) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
impl OwnedExec {
    fn launch(fixture: &Fixture) -> io::Result<Self> {
        let mut owner = Self::stopped(fixture)?;
        if let Err(error) = owner.first_exec() {
            owner.refuse()?;
            return Err(error);
        }
        Ok(owner)
    }
    fn stopped(fixture: &Fixture) -> io::Result<Self> {
        let expected = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&fixture.binary)?;
        let stamp = selected(&expected)?;
        let writer = OpenOptions::new()
            .create_new(true)
            .append(true)
            .mode(0o600)
            .custom_flags(libc::O_CLOEXEC)
            .open(&fixture.output)?;
        let output = File::open(&fixture.output)?;
        let (input_read, input_write) = pipe()?;
        let program = CString::new(PROGRAM).unwrap();
        let program_env = CString::new(format!("PROGRAM={PROGRAM}")).unwrap();
        let args = [
            c"owned-shell".as_ptr(),
            c"-c".as_ptr(),
            program.as_ptr(),
            std::ptr::null(),
        ];
        let env = [
            c"SELF=/proc/self/exe".as_ptr(),
            program_env.as_ptr(),
            std::ptr::null(),
        ];
        let child_fds = [
            input_read.as_raw_fd(),
            input_write.as_raw_fd(),
            writer.as_raw_fd(),
            output.as_raw_fd(),
            expected.as_raw_fd(),
        ];
        if child_fds.iter().any(|fd| *fd < 3) {
            return Err(invalid());
        }
        // SAFETY: all child inputs are prepared; child_exec uses only libc calls
        // and _exit, never allocation, locks, unwinding or Rust destructors.
        let pid = unsafe { libc::fork() };
        if pid < 0 {
            return Err(io::Error::last_os_error());
        }
        if pid == 0 {
            child_exec(child_fds, &args, &env);
        }
        drop(input_read);
        drop(writer);
        let mut owner = Self {
            pid,
            pidfd: None,
            expected,
            stamp,
            input: Some(File::from(input_write)),
            output,
            phase: Phase::Initial,
            reaped: false,
            last_status: None,
            _thread_bound: PhantomData,
        };
        // SAFETY: PID belongs to our still-unreaped direct child; no adoption.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0u32) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful syscall creates a new owned descriptor.
        owner.pidfd = Some(unsafe { OwnedFd::from_raw_fd(fd as i32) });
        let status = owner.wait_status()?;
        if status != (libc::SIGSTOP << 8) | 0x7f {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("owned initial stop refused: {status:#x}"),
            ));
        }
        Ok(owner)
    }
    fn first_exec(&mut self) -> io::Result<()> {
        let result = self.first_exec_inner();
        if result.is_err() {
            self.refuse()?;
        }
        result
    }
    fn first_exec_inner(&mut self) -> io::Result<()> {
        if self.phase != Phase::Initial {
            return Err(invalid());
        }
        trace(
            libc::PTRACE_SETOPTIONS,
            self.pid,
            (libc::PTRACE_O_TRACEEXEC | libc::PTRACE_O_EXITKILL) as usize,
        )?;
        trace(libc::PTRACE_CONT, self.pid, 0)?;
        let status = self.wait_status()?;
        if !is_exec_stop(status) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("owned first exec stop refused: {status:#x}"),
            ));
        }
        self.check_object(&self.expected).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("owned stopped executable refused: {error}"),
            )
        })?;
        self.phase = Phase::FirstExecStopped;
        Ok(())
    }
    fn check_object(&self, selected_file: &File) -> io::Result<()> {
        let current = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_CLOEXEC)
            .open(format!("/proc/{}/exe", self.pid))?;
        if Stamp::of(&current.metadata()?) != self.stamp
            || selected(selected_file)? != self.stamp
            || Stamp::of(&current.metadata()?) != self.stamp
        {
            return Err(invalid());
        }
        Ok(())
    }
    fn poll_status(&mut self) -> io::Result<Option<i32>> {
        if self.reaped {
            return Err(invalid());
        }
        let mut status = 0;
        // SAFETY: exact owned child PID, valid status storage; never wait-any.
        let result = unsafe { libc::waitpid(self.pid, &mut status, libc::WNOHANG | libc::__WALL) };
        if result < 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::EINTR) {
                return Ok(None);
            }
            return Err(error);
        }
        if result == 0 {
            return Ok(None);
        }
        if result != self.pid {
            return Err(invalid());
        }
        self.last_status = Some(status);
        self.reaped = libc::WIFEXITED(status) || libc::WIFSIGNALED(status);
        Ok(Some(status))
    }
    fn wait_status(&mut self) -> io::Result<i32> {
        let deadline = Instant::now() + DEADLINE;
        loop {
            if let Some(status) = self.poll_status()? {
                return Ok(status);
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "owned fixture stalled",
                ));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    fn resume(&mut self) -> io::Result<()> {
        if self.phase != Phase::FirstExecStopped {
            return Err(invalid());
        }
        if self
            .check_object(&self.expected)
            .and_then(|()| trace(libc::PTRACE_CONT, self.pid, 0))
            .is_err()
        {
            self.refuse()?;
            return Err(invalid());
        }
        self.phase = Phase::Running;
        Ok(())
    }
    fn command(&mut self, command: Control) -> io::Result<()> {
        if self.phase != Phase::Running {
            return Err(invalid());
        }
        let bytes: &[u8] = match command {
            Control::Ack => b"ack\n",
            Control::Same => b"same\n",
            Control::Other => b"other\n",
        };
        if self
            .input
            .as_mut()
            .ok_or_else(invalid)?
            .write_all(bytes)
            .is_err()
        {
            self.refuse()?;
            return Err(invalid());
        }
        Ok(())
    }
    fn wait_marker(&mut self, marker: &[u8]) -> io::Result<()> {
        if self.phase != Phase::Running {
            return Err(invalid());
        }
        let result = self.marker_inner(marker);
        if result.is_err() {
            self.refuse()?;
        }
        result
    }
    fn marker_inner(&mut self, marker: &[u8]) -> io::Result<()> {
        let deadline = Instant::now() + DEADLINE;
        loop {
            if self.output.metadata()?.len() > 128 {
                return Err(invalid());
            }
            let mut bytes = [0; 128];
            let n = self.output.read_at(&mut bytes, 0)?;
            if bytes[..n].windows(marker.len()).any(|part| part == marker) {
                return Ok(());
            }
            if self.poll_status()?.is_some() || Instant::now() >= deadline {
                return Err(invalid());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    fn reject_next_exec(&mut self) -> io::Result<()> {
        if self.phase != Phase::Running {
            return Err(invalid());
        }
        let result = self.wait_status().map(is_exec_stop);
        self.refuse()?;
        match result {
            Ok(true) => Ok(()),
            _ => Err(invalid()),
        }
    }
    fn finish(&mut self) -> io::Result<()> {
        if self.phase != Phase::Running {
            return Err(invalid());
        }
        self.input.take();
        match self.wait_status() {
            Ok(0) => {
                self.phase = Phase::Terminal;
                Ok(())
            }
            _ => {
                self.refuse()?;
                Err(invalid())
            }
        }
    }
    fn refuse(&mut self) -> io::Result<()> {
        self.phase = Phase::Refused;
        self.terminate()
    }
    fn terminate(&mut self) -> io::Result<()> {
        self.input.take();
        if self.reaped {
            return Ok(());
        }
        // SAFETY: retained pidfd is exclusively our original child. Before
        // acquisition failure only, PID is our own unreaped fork result.
        let result = unsafe {
            match &self.pidfd {
                Some(fd) => libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    fd.as_raw_fd(),
                    libc::SIGKILL,
                    std::ptr::null::<libc::siginfo_t>(),
                    0u32,
                ),
                None => libc::kill(self.pid, libc::SIGKILL) as libc::c_long,
            }
        };
        if result < 0 && io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
            return Err(io::Error::last_os_error());
        }
        let deadline = Instant::now() + DEADLINE;
        while !self.reaped {
            self.poll_status()?;
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "owned fixture reap stalled",
                ));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        Ok(())
    }
}
impl Drop for OwnedExec {
    fn drop(&mut self) {
        // No service/cgroup/adopted-process cleanup; exactly our own child.
        if let Err(error) = self.terminate() {
            eprintln!("owned first-exec fixture cleanup failed: {error}");
        }
    }
}

fn child_exec(fds: [i32; 5], args: &[*const libc::c_char; 4], env: &[*const libc::c_char; 3]) -> ! {
    // SAFETY: post-fork fixed prepared descriptors/C strings; no Rust allocation,
    // locks, unwinding or destructors. Every error takes async-signal-safe _exit.
    unsafe {
        libc::close(fds[1]);
        libc::close(fds[3]);
        if libc::dup2(fds[0], libc::STDIN_FILENO) < 0
            || libc::dup2(fds[2], libc::STDOUT_FILENO) < 0
            || libc::dup2(fds[2], libc::STDERR_FILENO) < 0
        {
            libc::_exit(70);
        }
        libc::close(fds[0]);
        libc::close(fds[2]);
        // CLOEXEC alone leaves other threads' inherited descriptors alive at
        // the pre-exec stop. Close the copied child table now, preserving only
        // stdio and the original selected executable. Never share/unshare or
        // alter the parent's descriptor table; unsupported kernels refuse.
        let executable_fd = fds[4] as u32;
        if (executable_fd > 3
            && libc::syscall(libc::SYS_close_range, 3u32, executable_fd - 1, 0u32) < 0)
            || libc::syscall(libc::SYS_close_range, executable_fd + 1, u32::MAX, 0u32) < 0
        {
            libc::_exit(73);
        }
        if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) < 0
            || libc::ptrace(
                libc::PTRACE_TRACEME,
                0,
                std::ptr::null_mut::<libc::c_void>(),
                0usize,
            ) < 0
            || libc::kill(libc::getpid(), libc::SIGSTOP) < 0
        {
            libc::_exit(71);
        }
        libc::syscall(
            libc::SYS_execveat,
            fds[4],
            c"".as_ptr(),
            args.as_ptr(),
            env.as_ptr(),
            libc::AT_EMPTY_PATH,
        );
        // Fixed bounded diagnostic exit: distinguish kernel exec errno without
        // formatting, allocation or calling Rust IO in the post-fork child.
        let errno = *libc::__errno_location();
        libc::_exit(if (1..128).contains(&errno) {
            128 + errno
        } else {
            72
        });
    }
}

#[path = "diagnostic_signer_first_exec_fixture_tests.rs"]
mod tests;
