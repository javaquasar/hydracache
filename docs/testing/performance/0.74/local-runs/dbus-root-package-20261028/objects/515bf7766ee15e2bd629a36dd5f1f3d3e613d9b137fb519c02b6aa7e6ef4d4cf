//! Binary-private original self-exec ownership; no selected root/runtime admission.
use hydracache_long_run_supervisor_074::diagnostic_signer_isolation::{
    run_fixed_probe, FixedProbe,
};
use std::fs::{File, Metadata, OpenOptions};
use std::io::{self, Read};
use std::marker::PhantomData;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};
use std::rc::Rc;
use std::time::{Duration, Instant};

const LIMIT: Duration = Duration::from_secs(5);
const CHILD_LINE: &[u8] = b"fixed child isolation probe passed; admission=false\n";
const INITIAL: i32 = (libc::SIGSTOP << 8) | 0x7f;
const EXEC: i32 = (libc::PTRACE_EVENT_EXEC << 16) | (libc::SIGTRAP << 8) | 0x7f;
const SYS_STOP: i32 = (libc::SIGSYS << 8) | 0x7f;
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "fixed owned probe refused")
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    Native,
    X32,
    I386,
}
impl Case {
    fn arg(self) -> &'static std::ffi::CStr {
        match self {
            Self::Native => c"--child-native",
            Self::X32 => c"--child-x32",
            Self::I386 => c"--child-i386",
        }
    }
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
    let metadata = file.metadata()?;
    // SAFETY: fixed queries on this borrowed live descriptor and current numeric identity.
    let (flags, descriptor, uid) = unsafe {
        (
            libc::fcntl(file.as_raw_fd(), libc::F_GETFL),
            libc::fcntl(file.as_raw_fd(), libc::F_GETFD),
            libc::getuid(),
        )
    };
    if flags < 0
        || flags & (libc::O_ACCMODE | libc::O_PATH) != libc::O_RDONLY
        || descriptor < 0
        || descriptor & libc::FD_CLOEXEC == 0
        || uid == 0
        || !metadata.is_file()
        || metadata.uid() != uid
        || metadata.nlink() != 1
        || metadata.mode() & 0o7777 != 0o555
        || !(64..=134217728).contains(&metadata.len())
    {
        return Err(invalid());
    }
    let stamp = Stamp::of(&metadata);
    let mut header = [0; 64];
    file.read_exact_at(&mut header, 0)?;
    if &header[..7] != b"\x7fELF\x02\x01\x01"
        || ![2, 3].contains(&u16::from_le_bytes([header[16], header[17]]))
        || u16::from_le_bytes([header[18], header[19]]) != 62
        || u32::from_le_bytes(header[20..24].try_into().unwrap()) != 1
        || u16::from_le_bytes([header[52], header[53]]) != 64
        || Stamp::of(&file.metadata()?) != stamp
    {
        return Err(invalid());
    }
    Ok(stamp)
}
fn parent_context() -> io::Result<()> {
    // SAFETY: current identity only. The normal entry supports a fresh main-thread owner.
    let (uid, gid, pid, tid) = unsafe {
        (
            libc::getuid(),
            libc::getgid(),
            libc::getpid(),
            libc::syscall(libc::SYS_gettid),
        )
    };
    if uid == 0
        || gid == 0
        || unsafe { libc::geteuid() } != uid
        || unsafe { libc::getegid() } != gid
        || tid != pid as libc::c_long
        || std::env::vars_os().next().is_some()
    {
        return Err(invalid());
    }
    let mut bytes = Vec::new();
    File::open("/proc/thread-self/status")?
        .take(65537)
        .read_to_end(&mut bytes)?;
    if !single_thread_status(&bytes) {
        return Err(invalid());
    }
    Ok(())
}
fn supports_reaping(handler: libc::sighandler_t, flags: libc::c_int) -> bool {
    handler == libc::SIG_DFL && flags & libc::SA_NOCLDWAIT == 0
}
fn reaping_policy() -> io::Result<()> {
    // SAFETY: query only; never replace the invoking process's signal disposition.
    let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
    if unsafe { libc::sigaction(libc::SIGCHLD, std::ptr::null(), &mut action) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if !supports_reaping(action.sa_sigaction, action.sa_flags) {
        return Err(invalid());
    }
    Ok(())
}
fn single_thread_status(bytes: &[u8]) -> bool {
    if bytes.is_empty() || bytes.len() > 65536 {
        return false;
    }
    let Ok(raw) = std::str::from_utf8(bytes) else {
        return false;
    };
    let mut threads = raw.lines().filter_map(|line| line.strip_prefix("Threads:"));
    threads.next().map(str::trim) == Some("1") && threads.next().is_none()
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Initial,
    ExecStopped,
    Running,
    Terminal,
    Refused,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    Initial,
    Exec,
    ExitZero,
    SysDeath,
    SysDelivery,
    Other,
}
fn event(status: i32) -> Event {
    match status {
        INITIAL => Event::Initial,
        EXEC => Event::Exec,
        0 => Event::ExitZero,
        libc::SIGSYS => Event::SysDeath,
        SYS_STOP => Event::SysDelivery,
        _ => Event::Other,
    }
}
fn correct_output(raw: &[u8], case: Case) -> bool {
    raw.len() <= 128
        && raw
            == if case == Case::Native {
                CHILD_LINE
            } else {
                b""
            }
}
struct Owner {
    pid: libc::pid_t,
    pidfd: Option<OwnedFd>,
    expected: File,
    stamp: Stamp,
    output: File,
    phase: Phase,
    reaped: bool,
    case: Case,
    _thread: PhantomData<Rc<()>>,
}
fn trace(request: libc::c_uint, pid: libc::pid_t, signal: usize) -> io::Result<()> {
    // SAFETY: exact direct child owned by this tracing thread; no arbitrary memory IO.
    if unsafe { libc::ptrace(request, pid, std::ptr::null_mut::<libc::c_void>(), signal) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
impl Owner {
    // Private only: ordinary caller supplies its own retained /proc/self/exe;
    // unit tests alone construct other fixed owned nonproduct fixture files.
    fn create(expected: File, case: Case) -> io::Result<Self> {
        // Auto-reaping could invalidate an unreaped-fork PID before pidfd_open;
        // refuse it before any child exists, including the private unit-test seam.
        reaping_policy()?;
        let stamp = selected(&expected)?;
        let null = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_CLOEXEC)
            .open("/dev/null")?;
        let mut raw = [-1; 2];
        // SAFETY: initialized two-int output; successful descriptors transferred exactly once.
        if unsafe { libc::pipe2(raw.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let (read, write) = unsafe { (OwnedFd::from_raw_fd(raw[0]), OwnedFd::from_raw_fd(raw[1])) };
        let fds = [null.as_raw_fd(), write.as_raw_fd(), expected.as_raw_fd()];
        if raw.iter().chain(fds.iter()).any(|fd| *fd < 3) {
            return Err(invalid());
        }
        let argv = [
            c"owned-original-probe".as_ptr(),
            case.arg().as_ptr(),
            std::ptr::null(),
        ];
        let env = [std::ptr::null::<libc::c_char>()];
        // SAFETY: original current parent, captured before its own fork.
        let parent = unsafe { libc::getpid() };
        // SAFETY: all child inputs prepared; child() never allocates/unwinds or takes Rust locks.
        let pid = unsafe { libc::fork() };
        if pid < 0 {
            return Err(io::Error::last_os_error());
        }
        if pid == 0 {
            child(fds, parent, &argv, &env);
        }
        drop(write);
        drop(null);
        // Establish cleanup ownership before the first fallible parent-side acquisition.
        let mut own = Self {
            pid,
            pidfd: None,
            expected,
            stamp,
            output: File::from(read),
            phase: Phase::Initial,
            reaped: false,
            case,
            _thread: PhantomData,
        };
        // SAFETY: exact original unreaped fork child, never PID adoption.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0u32) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        own.pidfd = Some(unsafe { OwnedFd::from_raw_fd(fd as i32) });
        if event(own.wait()?) != Event::Initial {
            return Err(invalid());
        }
        Ok(own)
    }
    fn object(&self) -> io::Result<()> {
        if self.reaped || !matches!(self.phase, Phase::Initial | Phase::ExecStopped) {
            return Err(invalid());
        }
        // Deliberate fixed proc magic link for exactly the retained stopped child.
        let current = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_CLOEXEC)
            .open(format!("/proc/{}/exe", self.pid))?;
        if Stamp::of(&current.metadata()?) != self.stamp
            || selected(&self.expected)? != self.stamp
            || Stamp::of(&current.metadata()?) != self.stamp
        {
            return Err(invalid());
        }
        Ok(())
    }
    fn first(&mut self) -> io::Result<()> {
        let result = (|| {
            if self.phase != Phase::Initial || self.reaped {
                return Err(invalid());
            }
            trace(
                libc::PTRACE_SETOPTIONS,
                self.pid,
                (libc::PTRACE_O_TRACEEXEC | libc::PTRACE_O_EXITKILL) as usize,
            )?;
            trace(libc::PTRACE_CONT, self.pid, 0)?;
            if event(self.wait()?) != Event::Exec {
                return Err(invalid());
            }
            self.object()?;
            self.phase = Phase::ExecStopped;
            Ok(())
        })();
        self.checked(result)
    }
    fn resume(&mut self) -> io::Result<()> {
        let result = (|| {
            if self.phase != Phase::ExecStopped || self.reaped {
                return Err(invalid());
            }
            self.object()?;
            trace(libc::PTRACE_CONT, self.pid, 0)?;
            self.phase = Phase::Running;
            Ok(())
        })();
        self.checked(result)
    }
    fn finish(&mut self) -> io::Result<()> {
        let result = (|| {
            if self.phase != Phase::Running || self.reaped {
                return Err(invalid());
            }
            let mut observed = event(self.wait()?);
            if self.case != Case::Native && observed == Event::SysDelivery {
                trace(libc::PTRACE_CONT, self.pid, libc::SIGSYS as usize)?;
                observed = event(self.wait()?);
            }
            if observed
                != if self.case == Case::Native {
                    Event::ExitZero
                } else {
                    Event::SysDeath
                }
                || !self.reaped
            {
                return Err(invalid());
            }
            let mut bytes = Vec::new();
            self.output.by_ref().take(129).read_to_end(&mut bytes)?;
            if !correct_output(&bytes, self.case) {
                return Err(invalid());
            }
            self.phase = Phase::Terminal;
            Ok(())
        })();
        self.checked(result)
    }
    fn checked(&mut self, result: io::Result<()>) -> io::Result<()> {
        if result.is_err() {
            self.phase = Phase::Refused;
            self.terminate()?;
        }
        result
    }
    fn poll(&mut self) -> io::Result<Option<i32>> {
        if self.reaped {
            return Err(invalid());
        }
        let mut status = 0;
        // SAFETY: exact owned child only, bounded nonblocking wait, never wait-any.
        let result = unsafe { libc::waitpid(self.pid, &mut status, libc::WNOHANG | libc::__WALL) };
        if result < 0 {
            let error = io::Error::last_os_error();
            return if error.raw_os_error() == Some(libc::EINTR) {
                Ok(None)
            } else {
                Err(error)
            };
        }
        if result == 0 {
            return Ok(None);
        }
        if result != self.pid {
            return Err(invalid());
        }
        self.reaped = libc::WIFEXITED(status) || libc::WIFSIGNALED(status);
        Ok(Some(status))
    }
    fn wait(&mut self) -> io::Result<i32> {
        let end = Instant::now() + LIMIT;
        loop {
            if let Some(status) = self.poll()? {
                return Ok(status);
            }
            if Instant::now() >= end {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "owned probe wait refused",
                ));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    fn terminate(&mut self) -> io::Result<()> {
        if self.reaped {
            return Ok(());
        }
        // SAFETY: pidfd is exclusively this original child. Only before acquisition,
        // PID fallback refers to our own still-unreaped fork result, never an adopted PID.
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
        let end = Instant::now() + LIMIT;
        while !self.reaped {
            self.poll()?;
            if !self.reaped && Instant::now() >= end {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "owned probe reap refused",
                ));
            }
            if !self.reaped {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        Ok(())
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        if self.terminate().is_err() {
            eprintln!("Fixed owned probe cleanup refused.");
        }
    }
}
fn child(
    fds: [i32; 3],
    parent: libc::pid_t,
    argv: &[*const libc::c_char; 3],
    env: &[*const libc::c_char; 1],
) -> ! {
    // SAFETY: original fork child; fixed scalar/libc operations on prepared data
    // only. Every failure exits without Rust allocation, unwinding or destructors.
    unsafe {
        let mut installed = 0i32;
        let core = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if libc::prctl(
            libc::PR_SET_PDEATHSIG,
            libc::SIGKILL as usize,
            0usize,
            0usize,
            0usize,
        ) != 0
            || libc::prctl(
                libc::PR_GET_PDEATHSIG,
                &mut installed,
                0usize,
                0usize,
                0usize,
            ) != 0
            || installed != libc::SIGKILL
            || libc::getppid() != parent
            || libc::setrlimit(libc::RLIMIT_CORE, &core) != 0
        {
            libc::_exit(70);
        }
        if libc::dup2(fds[0], 0) < 0 || libc::dup2(fds[1], 1) < 0 || libc::dup2(fds[1], 2) < 0 {
            libc::_exit(71);
        }
        let keep = fds[2] as u32;
        if (keep > 3 && libc::syscall(libc::SYS_close_range, 3u32, keep - 1, 0u32) != 0)
            || libc::syscall(libc::SYS_close_range, keep + 1, u32::MAX, 0u32) != 0
        {
            libc::_exit(72);
        }
        if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1usize, 0usize, 0usize, 0usize) != 0
            || libc::prctl(libc::PR_GET_NO_NEW_PRIVS, 0usize, 0usize, 0usize, 0usize) != 1
            || libc::ptrace(
                libc::PTRACE_TRACEME,
                0,
                std::ptr::null_mut::<libc::c_void>(),
                0usize,
            ) != 0
            || libc::kill(libc::getpid(), libc::SIGSTOP) != 0
        {
            libc::_exit(73);
        }
        libc::syscall(
            libc::SYS_execveat,
            fds[2],
            c"".as_ptr(),
            argv.as_ptr(),
            env.as_ptr(),
            libc::AT_EMPTY_PATH,
        );
        libc::_exit(74);
    }
}
fn run(case: Case) -> io::Result<()> {
    parent_context()?;
    // Fixed self magic link only, never caller-selected path/image or receipt input.
    let expected = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC)
        .open("/proc/self/exe")?;
    let mut own = Owner::create(expected, case)?;
    own.first()?;
    own.resume()?;
    own.finish()
}
pub(super) fn entry() -> ! {
    let args: Vec<_> = std::env::args_os().skip(1).take(2).collect();
    if let [arg] = args.as_slice() {
        let child = match arg.to_str() {
            Some("--child-native") => Some(FixedProbe::Native),
            Some("--child-x32") => Some(FixedProbe::X32),
            Some("--child-i386") => Some(FixedProbe::I386),
            _ => None,
        };
        if let Some(child) = child {
            let mut signal = 0i32;
            // SAFETY: current child query only; not proof of launcher identity or authority.
            if unsafe { libc::prctl(libc::PR_GET_PDEATHSIG, &mut signal, 0usize, 0usize, 0usize) }
                == 0
                && signal == libc::SIGKILL
            {
                run_fixed_probe(child);
            }
        } else {
            let case = match arg.to_str() {
                Some("native") => Some(Case::Native),
                Some("x32") => Some(Case::X32),
                Some("i386") => Some(Case::I386),
                _ => None,
            };
            if case.is_some_and(|case| run(case).is_ok()) {
                println!("owned original self-exec probe passed; admission=false");
                std::process::exit(0);
            }
        }
    }
    eprintln!("Fixed owned probe refused.");
    std::process::exit(2)
}

#[cfg(test)]
#[path = "diagnostic_signer_owned_probe_tests.rs"]
mod tests;
