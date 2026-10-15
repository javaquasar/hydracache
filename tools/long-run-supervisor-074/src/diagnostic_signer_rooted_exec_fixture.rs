//! Test-only selected root plus original GNU exec and installation observation.
use super::{credentials, identity, invalid, magic, mapping, text, CapData, CapHeader, Root, STOP};
use crate::diagnostic_signer_seccomp_fixture::{decode, encoded, policy, Event};
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::marker::PhantomData;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::{FileExt, OpenOptionsExt};
use std::rc::Rc;
use std::time::{Duration, Instant};

#[path = "diagnostic_signer_rooted_maps.rs"]
mod maps;
#[path = "diagnostic_signer_namespace_fixture.rs"]
mod namespace;
const ARCH: u32 = 0xc000_003e;
const EXEC_STOP: i32 = (libc::PTRACE_EVENT_EXEC << 16) | (libc::SIGTRAP << 8) | 0x7f;
const SYSCALL_STOP: i32 = ((libc::SIGTRAP | 0x80) << 8) | 0x7f;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    SelfExec,
    OtherExec,
    Flags,
    Policy,
    Length,
    Header,
    Pointer,
    EarlyExec,
    Env,
}
impl Case {
    fn arg(self) -> &'static std::ffi::CStr {
        match self {
            Self::SelfExec | Self::Env => c"self",
            Self::OtherExec => c"other",
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
    Rooted,
    FirstExec,
    Installed,
    Terminal,
    Refused,
}
struct RootedExec {
    root: Root,
    pid: libc::pid_t,
    pidfd: Option<OwnedFd>,
    namespace: Option<File>,
    parent_namespace: File,
    reaped: bool,
    phase: Phase,
    case: Case,
    namespace_case: Option<namespace::Case>,
    environment: Option<(File, [u64; 2])>,
    fd_directory: Option<(File, [u64; 2])>,
    _thread: PhantomData<Rc<()>>,
}
impl RootedExec {
    fn new(case: Case) -> io::Result<Self> {
        Self::build(case, None)
    }
    fn namespace(case: namespace::Case) -> io::Result<Self> {
        Self::build(Case::SelfExec, Some(case))
    }
    fn build(case: Case, namespace_case: Option<namespace::Case>) -> io::Result<Self> {
        let mut root = if namespace_case.is_some() {
            namespace::root()?
        } else {
            Root::new()?
        };
        root.check()?;
        root.attempted = true;
        // SAFETY: local parent identity queries, no account or permission changes.
        if unsafe { libc::getuid() } != root.uid || unsafe { libc::getgid() } != root.gid {
            return Err(invalid());
        }
        let last = text("/proc/sys/kernel/cap_last_cap")?
            .trim()
            .parse::<u32>()
            .map_err(|_| invalid())?;
        if last > 63
            || root
                .path
                .to_str()
                .is_none_or(|p| p.bytes().any(|b| b.is_ascii_whitespace()))
        {
            return Err(invalid());
        }
        let parent_namespace = magic("/proc/self/ns/user")?;
        let null = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_CLOEXEC)
            .open("/dev/null")?;
        let fds = [
            null.as_raw_fd(),
            root.dirs[0].file.as_raw_fd(),
            root.files[0].file.as_raw_fd(),
        ];
        if fds.iter().any(|fd| *fd < 3) {
            return Err(invalid());
        }
        let argv = [
            c"owned-rooted-helper".as_ptr(),
            namespace_case
                .map_or_else(|| case.arg(), namespace::Case::arg)
                .as_ptr(),
            std::ptr::null(),
        ];
        let env = [
            if case == Case::Env || namespace_case == Some(namespace::Case::Env) {
                c"ROOTED_ENV_FAULT=1".as_ptr()
            } else {
                std::ptr::null()
            },
            std::ptr::null(),
        ];
        // SAFETY: prepared fixed scalars/strings; only libc/_exit after fork.
        let pid = unsafe { libc::fork() };
        if pid < 0 {
            return Err(io::Error::last_os_error());
        }
        if pid == 0 {
            child(fds, last, &argv, &env);
        }
        drop(null);
        let mut owner = Self {
            root,
            pid,
            pidfd: None,
            namespace: None,
            parent_namespace,
            reaped: false,
            phase: Phase::Initial,
            case,
            namespace_case,
            environment: None,
            fd_directory: None,
            _thread: PhantomData,
        };
        // SAFETY: own unreaped direct child, never caller PID adoption.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0u32) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: fresh original pidfd transferred once.
        owner.pidfd = Some(unsafe { OwnedFd::from_raw_fd(fd as i32) });
        if owner.wait()? != STOP {
            return Err(invalid());
        }
        owner.trace(
            libc::PTRACE_SETOPTIONS,
            (libc::PTRACE_O_EXITKILL | libc::PTRACE_O_TRACEEXEC | libc::PTRACE_O_TRACESYSGOOD)
                as usize,
        )?;
        Ok(owner)
    }
    fn trace(&self, op: u32, data: usize) -> io::Result<()> {
        // SAFETY: fixed ptrace scalar operation on owned stopped child only.
        if unsafe { libc::ptrace(op, self.pid, 0usize, data) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    fn wait(&mut self) -> io::Result<i32> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if self.reaped {
                return Err(invalid());
            }
            let mut status = 0;
            // SAFETY: exact original child only; no wait-any.
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
                    "owned root child stalled",
                ));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn enter(&mut self) -> io::Result<()> {
        let result = self.enter_inner();
        if result.is_err() {
            self.refuse()?;
        }
        result
    }
    fn enter_inner(&mut self) -> io::Result<()> {
        if self.phase != Phase::Initial {
            return Err(invalid());
        }
        self.root.check()?;
        self.trace(libc::PTRACE_CONT, 0)?;
        if self.wait()? != STOP {
            return Err(invalid());
        }
        let ns = magic(format!("/proc/{}/ns/user", self.pid))?;
        if identity(&ns)? == identity(&self.parent_namespace)? {
            return Err(invalid());
        }
        for name in ["uid_map", "gid_map"] {
            if !text(format!("/proc/{}/{name}", self.pid))?.is_empty() {
                return Err(invalid());
            }
        }
        for (name, value) in [
            ("setgroups", "deny\n".to_owned()),
            (
                "uid_map",
                format!("{} {} 1\n", self.root.uid, self.root.uid),
            ),
            (
                "gid_map",
                format!("{} {} 1\n", self.root.gid, self.root.gid),
            ),
        ] {
            OpenOptions::new()
                .write(true)
                .open(format!("/proc/{}/{name}", self.pid))?
                .write_all(value.as_bytes())?;
        }
        self.namespace = Some(ns);
        self.trace(libc::PTRACE_CONT, 0)?;
        if self.wait()? != STOP {
            return Err(invalid());
        }
        self.context(true)?;
        self.phase = Phase::Rooted;
        Ok(())
    }
    fn context(&mut self, before_exec: bool) -> io::Result<()> {
        self.root.check()?;
        let ns = magic(format!("/proc/{}/ns/user", self.pid))?;
        if identity(&ns)? != identity(self.namespace.as_ref().ok_or_else(invalid)?)? {
            return Err(invalid());
        }
        mapping(&text(format!("/proc/{}/uid_map", self.pid))?, self.root.uid)?;
        mapping(&text(format!("/proc/{}/gid_map", self.pid))?, self.root.gid)?;
        if text(format!("/proc/{}/setgroups", self.pid))? != "deny\n" {
            return Err(invalid());
        }
        for name in ["root", "cwd"] {
            if super::Stamp::of(magic(format!("/proc/{}/{name}", self.pid))?.metadata()?)
                != self.root.dirs[0].stamp
            {
                return Err(invalid());
            }
        }
        credentials(
            &text(format!("/proc/{}/status", self.pid))?,
            self.root.uid,
            self.root.gid,
        )?;
        let tasks = fs::read_dir(format!("/proc/{}/task", self.pid))?
            .take(2)
            .map(|e| e.map(|e| e.file_name()))
            .collect::<io::Result<Vec<_>>>()?;
        if tasks != [std::ffi::OsString::from(self.pid.to_string())] {
            return Err(invalid());
        }
        let names = if !before_exec && self.namespace_case.is_some() {
            namespace::descriptors(self.fd_directory.as_ref().ok_or_else(invalid)?)?
        } else {
            fs::read_dir(format!("/proc/{}/fd", self.pid))?
                .take(5)
                .map(|e| e.map(|e| e.file_name()))
                .collect::<io::Result<BTreeSet<_>>>()?
        };
        let mut expected: BTreeSet<_> = ["0", "1", "2"]
            .map(std::ffi::OsString::from)
            .into_iter()
            .collect();
        if before_exec {
            expected.insert(self.root.files[0].file.as_raw_fd().to_string().into());
        }
        if names != expected {
            return Err(invalid());
        }
        if !before_exec
            && (super::Stamp::of(magic(format!("/proc/{}/exe", self.pid))?.metadata()?)
                != self.root.files[0].stamp
                || !self.empty_environment()?)
        {
            return Err(invalid());
        }
        self.root.check()
    }
    fn empty_environment(&self) -> io::Result<bool> {
        if self.namespace_case.is_none() {
            return Ok(text(format!("/proc/{}/environ", self.pid))?.is_empty());
        }
        let (file, original) = self.environment.as_ref().ok_or_else(invalid)?;
        // SAFETY: initialized kernel statfs output and queries on an owned live FD.
        let mut fs: libc::statfs = unsafe { std::mem::zeroed() };
        // SAFETY: fixed metadata/flag queries, no ownership or permission changes.
        let (result, flags, fd_flags) = unsafe {
            (
                libc::fstatfs(file.as_raw_fd(), &mut fs),
                libc::fcntl(file.as_raw_fd(), libc::F_GETFL),
                libc::fcntl(file.as_raw_fd(), libc::F_GETFD),
            )
        };
        if result != 0
            || fs.f_type != 0x9fa0
            || flags < 0
            || fd_flags < 0
            || flags & (libc::O_ACCMODE | libc::O_PATH) != libc::O_RDONLY
            || fd_flags & libc::FD_CLOEXEC == 0
            || !file.metadata()?.is_file()
            || super::Stamp::of(file.metadata()?).identity() != *original
        {
            return Err(invalid());
        }
        let mut byte = [0];
        Ok(file.read_at(&mut byte, 0)? == 0)
    }
    fn first_exec(&mut self) -> io::Result<()> {
        let result = self.first_inner();
        if result.is_err() {
            self.refuse()?;
        }
        result
    }
    fn first_inner(&mut self) -> io::Result<()> {
        if self.phase != Phase::Rooted {
            return Err(invalid());
        }
        self.context(true)?;
        self.trace(libc::PTRACE_CONT, 0)?;
        let status = self.wait()?;
        if status != EXEC_STOP {
            return Err(io::Error::other(format!(
                "rooted original exec refused: {status:#x}"
            )));
        }
        if self.namespace_case.is_some() {
            let file = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
                .open(format!("/proc/{}/environ", self.pid))?;
            let identity = super::Stamp::of(file.metadata()?).identity();
            self.environment = Some((file, identity));
            let directory = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_DIRECTORY)
                .open(format!("/proc/{}/fd", self.pid))?;
            let identity = super::Stamp::of(directory.metadata()?).identity();
            self.fd_directory = Some((directory, identity));
        }
        self.context(false)?;
        maps::validate(
            &text(format!("/proc/{}/maps", self.pid))?,
            &self.root,
            false,
        )?;
        self.phase = Phase::FirstExec;
        Ok(())
    }
    fn event(&mut self) -> io::Result<Event> {
        self.trace(libc::PTRACE_SYSCALL, 0)?;
        let status = self.wait()?;
        if status != SYSCALL_STOP {
            return Err(io::Error::other(format!(
                "rooted syscall stop refused: {status:#x}"
            )));
        }
        // SAFETY: correctly sized initialized integral/union kernel output buffer.
        let mut info: libc::ptrace_syscall_info = unsafe { std::mem::zeroed() };
        // SAFETY: original stopped child, writable correctly sized initialized buffer.
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
    fn memory(&self, bounds: (usize, usize), address: usize, count: usize) -> io::Result<Vec<u8>> {
        let end = address.checked_add(count).ok_or_else(invalid)?;
        let start = address & !7;
        let padded = end.checked_add(7).ok_or_else(invalid)? & !7;
        if count == 0 || count > self.program_size() || start < bounds.0 || padded > bounds.1 {
            return Err(invalid());
        }
        let mut raw = Vec::with_capacity(count + 14);
        for cursor in (start..padded).step_by(8) {
            // SAFETY: tracer-thread errno and bounded original stopped stack word.
            unsafe {
                *libc::__errno_location() = 0;
            }
            // SAFETY: original stopped child; only a validated stack interval read.
            let word = unsafe { libc::ptrace(libc::PTRACE_PEEKDATA, self.pid, cursor, 0usize) };
            // SAFETY: same tracer-thread errno, valid -1 data is not an error.
            let errno = unsafe { *libc::__errno_location() };
            if word == -1 && errno != 0 {
                return Err(io::Error::from_raw_os_error(errno));
            }
            raw.extend_from_slice(&word.to_ne_bytes());
        }
        Ok(raw[address - start..address - start + count].to_vec())
    }
    fn installer(&self, stack: (usize, usize), header: usize, program: usize) -> io::Result<()> {
        let raw = self.memory(stack, header, 16)?;
        let expected = self
            .namespace_case
            .map_or_else(|| encoded(&policy()), namespace::expected);
        if u16::from_le_bytes(raw[..2].try_into().unwrap()) as usize * 8 != expected.len()
            || u64::from_le_bytes(raw[8..16].try_into().unwrap()) != program as u64
            || self.memory(stack, program, expected.len())? != expected
        {
            return Err(invalid());
        }
        Ok(())
    }
    fn program_size(&self) -> usize {
        if self.namespace_case.is_some_and(|case| !case.control()) {
            512
        } else {
            272
        }
    }
    fn install(&mut self) -> io::Result<()> {
        let result = self.install_inner();
        if result.is_err() {
            self.refuse()?;
        }
        result
    }
    fn install_inner(&mut self) -> io::Result<()> {
        if self.phase != Phase::FirstExec {
            return Err(invalid());
        }
        self.context(false)?;
        let mut event = self.event()?;
        if event
            == (Event::Exit {
                arch: ARCH,
                value: 0,
                error: false,
            })
        {
            event = self.event()?;
        }
        let mut header = None;
        for _ in 0..256 {
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
                    header = Some(usize::try_from(args[2]).map_err(|_| invalid())?);
                    break;
                }
                Event::Entry { arch: ARCH, nr, .. } if startup_allowed(nr) => {
                    if !matches!(self.event()?, Event::Exit { arch: ARCH, .. }) {
                        return Err(invalid());
                    }
                    event = self.event()?;
                }
                _ => return Err(invalid()),
            }
        }
        let header = header.ok_or_else(invalid)?;
        self.context(false)?;
        let stack = maps::validate(&text(format!("/proc/{}/maps", self.pid))?, &self.root, true)?;
        let raw = self.memory(stack, header, 16)?;
        let program = usize::try_from(u64::from_le_bytes(raw[8..16].try_into().unwrap()))
            .map_err(|_| invalid())?;
        self.installer(stack, header, program)?;
        if self.event()?
            != (Event::Exit {
                arch: ARCH,
                value: 0,
                error: false,
            })
        {
            return Err(invalid());
        }
        self.installer(stack, header, program)?;
        self.context(false)?;
        maps::validate(&text(format!("/proc/{}/maps", self.pid))?, &self.root, true)?;
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
    fn denied(&mut self) -> io::Result<()> {
        let result = self.denied_inner();
        if result.is_err() {
            self.refuse()?;
        }
        result
    }
    fn denied_inner(&mut self) -> io::Result<()> {
        if self.phase != Phase::Installed || ![Case::SelfExec, Case::OtherExec].contains(&self.case)
        {
            return Err(invalid());
        }
        match self.event()? {
            Event::Entry { arch: ARCH, nr, .. } if nr == libc::SYS_execve as u64 => {}
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
        if self.wait()? != 0 {
            return Err(invalid());
        }
        self.root.check()?;
        self.phase = Phase::Terminal;
        Ok(())
    }
    fn refuse(&mut self) -> io::Result<()> {
        self.phase = Phase::Refused;
        self.root.refused = true;
        self.terminate()
    }
    fn terminate(&mut self) -> io::Result<()> {
        if self.reaped {
            return Ok(());
        }
        // SAFETY: exact owned pidfd; PID used only for failed acquisition of own child.
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
impl Drop for RootedExec {
    fn drop(&mut self) {
        if let Err(e) = self.terminate() {
            eprintln!("owned root child cleanup failed: {e}");
        }
    }
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
            libc::SYS_unshare,
            libc::SYS_setns,
            libc::SYS_chroot,
            libc::SYS_chdir,
            libc::SYS_fchdir,
            libc::SYS_mount,
            libc::SYS_umount2,
            libc::SYS_pivot_root,
            libc::SYS_open_tree,
            libc::SYS_move_mount,
            libc::SYS_fsopen,
            libc::SYS_fsconfig,
            libc::SYS_fsmount,
            libc::SYS_fspick,
            libc::SYS_mount_setattr,
        ]
        .contains(&(nr as i64))
}
fn child(
    fds: [i32; 3],
    last: u32,
    argv: &[*const libc::c_char; 3],
    env: &[*const libc::c_char; 2],
) -> ! {
    // SAFETY: all inputs prepared; only scalar libc and stack storage after raw fork.
    unsafe {
        if libc::dup2(fds[0], 0) < 0 || libc::dup2(fds[0], 1) < 0 || libc::dup2(fds[0], 2) < 0 {
            libc::_exit(70);
        }
        let lo = fds[1].min(fds[2]) as u32;
        let hi = fds[1].max(fds[2]) as u32;
        let limit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if (lo > 3 && libc::syscall(libc::SYS_close_range, 3u32, lo - 1, 0u32) < 0)
            || (hi > lo + 1 && libc::syscall(libc::SYS_close_range, lo + 1, hi - 1, 0u32) < 0)
            || libc::syscall(libc::SYS_close_range, hi + 1, u32::MAX, 0u32) < 0
            || libc::setrlimit(libc::RLIMIT_CORE, &limit) < 0
            || libc::ptrace(libc::PTRACE_TRACEME, 0, 0usize, 0usize) < 0
            || libc::kill(libc::getpid(), libc::SIGSTOP) < 0
        {
            libc::_exit(71);
        }
        if libc::unshare(libc::CLONE_NEWUSER) < 0 || libc::kill(libc::getpid(), libc::SIGSTOP) < 0 {
            libc::_exit(72);
        }
        if libc::fchdir(fds[1]) < 0
            || libc::chroot(c".".as_ptr()) < 0
            || libc::chdir(c"/".as_ptr()) < 0
            || libc::close(fds[1]) < 0
        {
            libc::_exit(73);
        }
        for cap in 0..=last {
            if libc::prctl(libc::PR_CAPBSET_DROP, cap as usize, 0usize, 0usize, 0usize) < 0 {
                libc::_exit(74);
            }
        }
        let header = CapHeader {
            version: 0x2008_0522,
            pid: 0,
        };
        let data = [CapData {
            effective: 0,
            permitted: 0,
            inheritable: 0,
        }; 2];
        if libc::prctl(
            libc::PR_CAP_AMBIENT,
            libc::PR_CAP_AMBIENT_CLEAR_ALL,
            0usize,
            0usize,
            0usize,
        ) < 0
            || libc::syscall(libc::SYS_capset, &header, data.as_ptr()) < 0
            || libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1usize, 0usize, 0usize, 0usize) < 0
            || libc::kill(libc::getpid(), libc::SIGSTOP) < 0
        {
            libc::_exit(75);
        }
        libc::syscall(
            libc::SYS_execveat,
            fds[2],
            c"".as_ptr(),
            argv.as_ptr(),
            env.as_ptr(),
            libc::AT_EMPTY_PATH,
        );
        libc::_exit(80);
    }
}

#[path = "diagnostic_signer_rooted_exec_fixture_tests.rs"]
mod tests;
