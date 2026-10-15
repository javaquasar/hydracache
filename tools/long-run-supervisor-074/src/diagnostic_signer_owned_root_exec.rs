//! Ordinary binary-private launch mechanics. No credential or authority inputs.
use super::{refused, require, Root, Stamp};
use hydracache_long_run_supervisor_074::diagnostic_signer_isolation::FixedProbe;
use std::collections::BTreeSet;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::time::{Duration, Instant};

#[path = "diagnostic_signer_isolation_core.rs"]
mod core;
#[path = "diagnostic_signer_owned_root_isolation.rs"]
mod isolation;
#[path = "diagnostic_signer_owned_root_loss.rs"]
mod loss;
#[path = "diagnostic_signer_owned_root_maps.rs"]
mod maps;
const STOP: i32 = (libc::SIGSTOP << 8) | 0x7f;
const EXEC: i32 = (libc::PTRACE_EVENT_EXEC << 16) | (libc::SIGTRAP << 8) | 0x7f;
const MARKER: &[u8; 16] = b"HC074ROOTEXECv1\n";
#[derive(Debug, PartialEq, Eq)]
enum Phase {
    Initial,
    Rooted,
    Exec,
    Loaded,
    Installed,
    Terminal,
    Refused,
}
fn file(fd: i32) -> io::Result<File> {
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful syscall returned a fresh owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn open(path: &str) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC)
        .open(path)
}
fn bounded(file: File) -> io::Result<String> {
    let mut value = String::new();
    file.take(65537).read_to_string(&mut value)?;
    require(value.len() <= 65536)?;
    Ok(value)
}
fn names(directory: &File, maximum: usize) -> io::Result<BTreeSet<String>> {
    std::fs::read_dir(format!("/proc/self/fd/{}", directory.as_raw_fd()))?
        .take(maximum + 1)
        .map(|e| e?.file_name().into_string().map_err(|_| refused()))
        .collect()
}
pub(super) fn preflight() -> io::Result<()> {
    // Empty environment/main-thread/nonroot identity also checked by acquire().
    let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
    require(unsafe { libc::sigaction(libc::SIGCHLD, std::ptr::null(), &mut action) } == 0)?;
    require(action.sa_sigaction == libc::SIG_DFL && action.sa_flags & libc::SA_NOCLDWAIT == 0)?;
    require(
        unsafe { libc::getgid() } != 0 && unsafe { libc::sysconf(libc::_SC_PAGESIZE) } == 4096,
    )?;
    for fd in 0..3 {
        require(unsafe { libc::fcntl(fd, libc::F_GETFD) } >= 0)?;
    }
    let descriptors = std::fs::read_dir("/proc/self/fd")?
        .take(5)
        .map(|e| e?.file_name().into_string().map_err(|_| refused()))
        .collect::<io::Result<BTreeSet<_>>>()?;
    require(descriptors == ["0", "1", "2", "3"].map(str::to_owned).into())
}
struct Owner {
    root: Root, // Drop explicitly suppressed if lifetime/EOF proof fails.
    pid: i32,
    pidfd: Option<File>,
    process: Option<File>,
    namespace: Option<File>,
    pipe: File,
    pipe_stamp: Stamp,
    null_stamp: Stamp,
    exec_fd: i32,
    uid: u32,
    gid: u32,
    creator: i32,
    tracer: i32,
    reaped: bool,
    terminal: Option<i32>,
    phase: Phase,
    probe: Option<FixedProbe>,
    readers: Option<isolation::Readers>,
    stack: Option<(usize, usize)>,
    filters: Option<u64>,
}
impl Owner {
    fn new(root: Root) -> io::Result<Self> {
        Self::build(root, None)
    }
    fn build(mut root: Root, probe: Option<FixedProbe>) -> io::Result<Self> {
        root.check()?;
        let last: u32 = bounded(open("/proc/sys/kernel/cap_last_cap")?)?
            .trim()
            .parse()
            .map_err(|_| refused())?;
        require(last <= 63)?;
        let null = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open("/dev/null")?;
        let null_stamp = Stamp::of(&null.metadata()?);
        require(
            null_stamp.mode & libc::S_IFMT == libc::S_IFCHR
                && null.metadata()?.rdev() == libc::makedev(1, 3),
        )?;
        let mut pipes = [-1; 2];
        require(
            unsafe { libc::pipe2(pipes.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) } == 0,
        )?;
        let pipe = file(pipes[0])?;
        let writer = file(pipes[1])?;
        let pipe_stamp = Stamp::of(&writer.metadata()?);
        let root_fd =
            file(unsafe { libc::fcntl(root.dirs[0].file.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 10) })?;
        let exec_fd = file(unsafe {
            libc::fcntl(
                root.files[0].file.as_raw_fd(),
                libc::F_DUPFD_CLOEXEC,
                root_fd.as_raw_fd() + 1,
            )
        })?;
        let creator = unsafe { libc::getpid() };
        let argv = [
            c"/signer".as_ptr(),
            match probe {
                None => c"--root-child",
                Some(FixedProbe::Native) => c"--root-native-child",
                Some(FixedProbe::X32) => c"--root-x32-child",
                Some(FixedProbe::I386) => c"--root-i386-child",
            }
            .as_ptr(),
            std::ptr::null(),
        ];
        let env = [std::ptr::null::<libc::c_char>()];
        let pid = unsafe { libc::fork() };
        if pid == 0 {
            raw_child(
                [
                    null.as_raw_fd(),
                    root_fd.as_raw_fd(),
                    exec_fd.as_raw_fd(),
                    writer.as_raw_fd(),
                ],
                last,
                creator,
                &argv,
                &env,
            );
        }
        require(pid > 0)?;
        // Ownership and cleanup exist BEFORE any fallible pidfd/proc acquisition.
        let mut owner = Self {
            root,
            pid,
            pidfd: None,
            process: None,
            namespace: None,
            pipe,
            pipe_stamp,
            null_stamp,
            exec_fd: exec_fd.as_raw_fd(),
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            creator,
            tracer: unsafe { libc::syscall(libc::SYS_gettid) as i32 },
            reaped: false,
            terminal: None,
            phase: Phase::Initial,
            probe,
            readers: None,
            stack: None,
            filters: None,
        };
        drop(writer);
        owner.pidfd = Some(file(unsafe {
            libc::syscall(libc::SYS_pidfd_open, pid, 0u32) as i32
        })?);
        require(owner.wait()? == STOP)?;
        owner.process = Some(
            OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(format!("/proc/{pid}"))?,
        );
        let mut filesystem: libc::statfs = unsafe { std::mem::zeroed() };
        require(
            unsafe {
                libc::fstatfs(
                    owner.process.as_ref().ok_or_else(refused)?.as_raw_fd(),
                    &mut filesystem,
                )
            } == 0
                && filesystem.f_type == 0x9fa0,
        )?;
        owner.trace(
            libc::PTRACE_SETOPTIONS,
            (libc::PTRACE_O_TRACEEXEC | libc::PTRACE_O_EXITKILL | libc::PTRACE_O_TRACESYSGOOD)
                as usize,
        )?;
        Ok(owner)
    }
    fn relative(&self, name: &str, flags: i32) -> io::Result<File> {
        let name = std::ffi::CString::new(name).map_err(|_| refused())?;
        file(unsafe {
            libc::openat(
                self.process.as_ref().ok_or_else(refused)?.as_raw_fd(),
                name.as_ptr(),
                flags | libc::O_CLOEXEC,
            )
        })
    }
    fn text(&self, name: &str) -> io::Result<String> {
        bounded(self.relative(name, libc::O_RDONLY)?)
    }
    fn trace(&self, request: libc::c_uint, data: usize) -> io::Result<()> {
        require(!self.reaped)?;
        if unsafe { libc::ptrace(request, self.pid, 0usize, data) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    fn wait(&mut self) -> io::Result<i32> {
        require(!self.reaped)?;
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            let mut status = 0;
            let got = unsafe { libc::waitpid(self.pid, &mut status, libc::WNOHANG | libc::__WALL) };
            if got == self.pid {
                if libc::WIFEXITED(status) || libc::WIFSIGNALED(status) {
                    self.reaped = true;
                    self.terminal = Some(status);
                }
                return Ok(status);
            }
            if got < 0 && io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
                return Err(io::Error::last_os_error());
            }
            require(Instant::now() < until)?;
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    fn enter(&mut self) -> io::Result<()> {
        require(self.phase == Phase::Initial)?;
        self.root.check()?;
        let parent_ns = open("/proc/self/ns/user")?;
        self.trace(libc::PTRACE_CONT, 0)?;
        require(self.wait()? == STOP)?;
        let namespace = self.relative("ns/user", libc::O_RDONLY)?;
        require(
            !Stamp::of(&namespace.metadata()?).same_object(&Stamp::of(&parent_ns.metadata()?)),
        )?;
        require(self.text("uid_map")?.is_empty() && self.text("gid_map")?.is_empty())?;
        for (name, value) in [
            ("setgroups", "deny\n".to_owned()),
            ("uid_map", format!("{} {} 1\n", self.uid, self.uid)),
            ("gid_map", format!("{} {} 1\n", self.gid, self.gid)),
        ] {
            self.relative(name, libc::O_WRONLY)?
                .write_all(value.as_bytes())?;
        }
        self.namespace = Some(namespace);
        self.trace(libc::PTRACE_CONT, 0)?;
        require(self.wait()? == STOP)?;
        self.context(true)?;
        self.phase = Phase::Rooted;
        Ok(())
    }
    fn context(&mut self, before_exec: bool) -> io::Result<()> {
        require(!self.reaped)?;
        let mut readiness = libc::pollfd {
            fd: self.pidfd.as_ref().ok_or_else(refused)?.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        require(unsafe { libc::poll(&mut readiness, 1, 0) } == 0 && readiness.revents == 0)?;
        self.root.check()?;
        let ns = self.relative("ns/user", libc::O_RDONLY)?;
        require(Stamp::of(&ns.metadata()?).same_object(&Stamp::of(
            &self.namespace.as_ref().ok_or_else(refused)?.metadata()?,
        )))?;
        for (name, id) in [("uid_map", self.uid), ("gid_map", self.gid)] {
            require(
                self.text(name)?.split_whitespace().collect::<Vec<_>>()
                    == [id.to_string(), id.to_string(), "1".to_owned()],
            )?;
        }
        require(self.text("setgroups")? == "deny\n")?;
        for name in ["root", "cwd"] {
            require(
                Stamp::of(&self.relative(name, libc::O_PATH)?.metadata()?)
                    == self.root.dirs[0].stamp,
            )?;
        }
        status(
            &self.text("status")?,
            self.uid,
            self.gid,
            self.pid,
            self.creator,
            self.tracer,
        )?;
        let tasks = names(
            &self.relative("task", libc::O_RDONLY | libc::O_DIRECTORY)?,
            1,
        )?;
        require(tasks == [self.pid.to_string()].into())?;
        let mut expected: BTreeSet<_> = ["0", "1", "2", "3"].map(str::to_owned).into();
        if before_exec {
            expected.insert(self.exec_fd.to_string());
        }
        let descriptors = if let Some(readers) = &self.readers {
            readers.names()?
        } else {
            names(&self.relative("fd", libc::O_RDONLY | libc::O_DIRECTORY)?, 5)?
        };
        require(descriptors == expected)?;
        // After ND0, original FD object bindings follow from the fully observed
        // install sequence (which permits no FD mutation), not fresh proc opens.
        for fd in 0..if self.phase == Phase::Installed { 0 } else { 4 } {
            let original = if fd == 3 {
                &self.pipe_stamp
            } else {
                &self.null_stamp
            };
            require(
                Stamp::of(
                    &self
                        .relative(&format!("fd/{fd}"), libc::O_PATH)?
                        .metadata()?,
                )
                .same_object(original),
            )?;
        }
        if before_exec {
            require(
                Stamp::of(
                    &self
                        .relative(&format!("fd/{}", self.exec_fd), libc::O_PATH)?
                        .metadata()?,
                ) == self.root.files[0].stamp,
            )?;
        } else {
            require(
                Stamp::of(&self.relative("exe", libc::O_PATH)?.metadata()?)
                    == self.root.files[0].stamp
                    && if let Some(readers) = &self.readers {
                        readers.empty_environment()?
                    } else {
                        self.text("environ")?.is_empty()
                    },
            )?;
        }
        if self.phase == Phase::Installed {
            require(self.readers.is_some())?;
            let (mode, filters) = isolation::filter_state(&self.text("status")?)?;
            require(mode == 2 && filters == self.filters.ok_or_else(refused)? + 1)?;
        }
        self.root.check()
    }
    fn first_exec(&mut self) -> io::Result<()> {
        require(self.phase == Phase::Rooted)?;
        self.context(true)?;
        self.trace(libc::PTRACE_CONT, 0)?;
        require(self.wait()? == EXEC)?;
        self.context(false)?;
        maps::validate(&self.text("maps")?, &self.root, false)?;
        self.phase = Phase::Exec;
        Ok(())
    }
    fn loaded(&mut self) -> io::Result<()> {
        require(self.phase == Phase::Exec)?;
        self.context(false)?;
        self.trace(libc::PTRACE_CONT, 0)?;
        require(self.wait()? == STOP)?;
        self.context(false)?;
        self.stack = Some(maps::validate(&self.text("maps")?, &self.root, true)?);
        let mut marker = [0; 17];
        require(self.pipe.read(&mut marker)? == MARKER.len() && &marker[..16] == MARKER)?;
        require(
            self.pipe
                .read(&mut marker)
                .is_err_and(|e| e.kind() == io::ErrorKind::WouldBlock),
        )?;
        self.phase = Phase::Loaded;
        Ok(())
    }
    fn terminate(&mut self) -> io::Result<()> {
        if !self.reaped {
            let result = if let Some(fd) = &self.pidfd {
                unsafe {
                    libc::syscall(
                        libc::SYS_pidfd_send_signal,
                        fd.as_raw_fd(),
                        libc::SIGKILL,
                        std::ptr::null::<libc::siginfo_t>(),
                        0u32,
                    ) as i32
                }
            } else {
                // Only our own unreaped fork; SIGCHLD default checked before fork.
                unsafe { libc::kill(self.pid, libc::SIGKILL) }
            };
            if result != 0 && io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
                return Err(io::Error::last_os_error());
            }
            // A pending ptrace stop may precede the terminal event. Never resume.
            for _ in 0..8 {
                if self.reaped {
                    break;
                }
                self.wait()?;
            }
            require(self.reaped)?;
        }
        let mut status = 0;
        require(
            unsafe { libc::waitpid(self.pid, &mut status, libc::WNOHANG | libc::__WALL) } == -1
                && io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD),
        )?;
        let mut bytes = [0; 17];
        let mut count = 0;
        loop {
            let got = self.pipe.read(&mut bytes)?;
            if got == 0 {
                break;
            }
            count += got;
            require(count <= if self.probe.is_some() { 48 } else { 16 })?;
        }
        Ok(())
    }
    fn finish(&mut self) -> io::Result<()> {
        require(self.phase == Phase::Loaded)?;
        self.terminate()?;
        require(self.terminal == Some(libc::SIGKILL))?;
        self.root.check()?;
        self.root.cleanup()?;
        self.phase = Phase::Terminal;
        Ok(())
    }
    fn advance(
        &mut self,
        expected: Phase,
        action: fn(&mut Self) -> io::Result<()>,
    ) -> io::Result<()> {
        let result = require(self.phase == expected).and_then(|()| action(self));
        if result.is_err() {
            self.phase = Phase::Refused;
            self.terminate()?;
        }
        result
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        if self.terminate().is_err() {
            // No automatic Root cleanup when original-child/stream quiescence is unknown.
            self.root.cleanup_attempted = true;
            eprintln!("Original root child quiescence refused; root retained.");
        }
    }
}
pub(super) fn run(root: Root) -> io::Result<()> {
    let mut owner = Owner::new(root)?;
    owner.advance(Phase::Initial, Owner::enter)?;
    owner.advance(Phase::Rooted, Owner::first_exec)?;
    owner.advance(Phase::Exec, Owner::loaded)?;
    owner.advance(Phase::Loaded, Owner::finish)
}
pub(super) fn run_isolated(root: Root, probe: FixedProbe) -> io::Result<()> {
    let mut owner = Owner::build(root, Some(probe))?;
    owner.advance(Phase::Initial, Owner::enter)?;
    owner.advance(Phase::Rooted, Owner::first_exec)?;
    owner.advance(Phase::Exec, Owner::loaded)?;
    owner.advance(Phase::Loaded, isolation::install)?;
    owner.advance(Phase::Installed, isolation::finish)
}
pub(super) fn installed_cleanup(root: Root, kill: bool) -> io::Result<()> {
    let mut owner = Owner::build(root, Some(FixedProbe::Native))?;
    owner.advance(Phase::Initial, Owner::enter)?;
    owner.advance(Phase::Rooted, Owner::first_exec)?;
    owner.advance(Phase::Exec, Owner::loaded)?;
    owner.advance(Phase::Loaded, isolation::install)?;
    if kill {
        owner.terminate()?;
        require(owner.terminal == Some(libc::SIGKILL))?;
        owner.root.check()?;
        owner.root.cleanup()?;
        owner.phase = Phase::Terminal;
    } else {
        // Verify ordinary Drop using originals retained BEFORE releasing ownership.
        let pid = owner.pid;
        let pidfd = owner.pidfd.as_ref().ok_or_else(refused)?.try_clone()?;
        let parent = owner.root.parent.try_clone()?;
        let name = owner.root.name.clone();
        drop(owner);
        let mut ready = libc::pollfd {
            fd: pidfd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        require(
            unsafe { libc::poll(&mut ready, 1, 0) } == 1
                && ready.revents & libc::POLLIN != 0
                && ready.revents & !(libc::POLLIN | libc::POLLHUP) == 0,
        )?;
        let mut status = 0;
        require(
            unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG | libc::__WALL) } == -1
                && io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD),
        )?;
        let mut metadata: libc::stat = unsafe { std::mem::zeroed() };
        require(
            unsafe {
                libc::fstatat(
                    parent.as_raw_fd(),
                    name.as_ptr(),
                    &mut metadata,
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            } == -1
                && io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT),
        )?;
    }
    Ok(())
}
pub(super) fn creator_loss(mode: &str) -> io::Result<()> {
    loss::run(mode)
}
fn status(raw: &str, uid: u32, gid: u32, pid: i32, creator: i32, tracer: i32) -> io::Result<()> {
    require(!raw.is_empty() && raw.len() <= 65536 && raw.ends_with('\n'))?;
    for (key, expected) in [
        ("Uid", vec![uid.to_string(); 4]),
        ("Gid", vec![gid.to_string(); 4]),
        ("Pid", vec![pid.to_string()]),
        ("Tgid", vec![pid.to_string()]),
        ("PPid", vec![creator.to_string()]),
        ("TracerPid", vec![tracer.to_string()]),
        ("Threads", vec!["1".into()]),
        ("NoNewPrivs", vec!["1".into()]),
        ("CapInh", vec!["0000000000000000".into()]),
        ("CapPrm", vec!["0000000000000000".into()]),
        ("CapEff", vec!["0000000000000000".into()]),
        ("CapBnd", vec!["0000000000000000".into()]),
        ("CapAmb", vec!["0000000000000000".into()]),
    ] {
        let values: Vec<_> = raw
            .lines()
            .filter_map(|line| line.strip_prefix(key)?.strip_prefix(':'))
            .collect();
        require(values.len() == 1 && values[0].split_whitespace().collect::<Vec<_>>() == expected)?;
    }
    Ok(())
}
#[repr(C)]
struct CapHeader {
    version: u32,
    pid: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct CapData {
    effective: u32,
    permitted: u32,
    inheritable: u32,
}
fn raw_child(
    fds: [i32; 4],
    last: u32,
    parent: i32,
    argv: &[*const libc::c_char; 3],
    env: &[*const libc::c_char; 1],
) -> ! {
    // SAFETY: only prepared scalars, libc syscalls, stack output and _exit after fork.
    unsafe {
        let mut armed = 0i32;
        if libc::prctl(
            libc::PR_SET_PDEATHSIG,
            libc::SIGKILL,
            0usize,
            0usize,
            0usize,
        ) != 0
            || libc::prctl(libc::PR_GET_PDEATHSIG, &mut armed, 0usize, 0usize, 0usize) != 0
            || armed != libc::SIGKILL
            || libc::getppid() != parent
        {
            libc::_exit(81);
        }
        let core = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        for fd in 0..3 {
            if libc::dup2(fds[0], fd) < 0 {
                libc::_exit(70);
            }
        }
        if libc::dup2(fds[3], 3) < 0
            || libc::syscall(libc::SYS_close_range, 4u32, fds[1] as u32 - 1, 0u32) < 0
            || (fds[2] > fds[1] + 1
                && libc::syscall(
                    libc::SYS_close_range,
                    fds[1] as u32 + 1,
                    fds[2] as u32 - 1,
                    0u32,
                ) < 0)
            || libc::syscall(libc::SYS_close_range, fds[2] as u32 + 1, u32::MAX, 0u32) < 0
            || libc::setrlimit(libc::RLIMIT_CORE, &core) < 0
            || libc::ptrace(libc::PTRACE_TRACEME, 0, 0usize, 0usize) < 0
            || libc::kill(libc::getpid(), libc::SIGSTOP) < 0
        {
            libc::_exit(71);
        }
        // EXITKILL is installed before this resume; overlap then clear PDEATHSIG.
        if libc::prctl(libc::PR_SET_PDEATHSIG, 0usize, 0usize, 0usize, 0usize) < 0
            || libc::unshare(libc::CLONE_NEWUSER) < 0
            || libc::kill(libc::getpid(), libc::SIGSTOP) < 0
        {
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
        let zero = [CapData {
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
            || libc::syscall(libc::SYS_capset, &header, zero.as_ptr()) < 0
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
fn child_context() -> io::Result<()> {
    // Fixed diagnostic only. These context assertions DO NOT authenticate a parent.
    require(std::env::vars_os().next().is_none())?;
    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };
    require(
        uid != 0
            && gid != 0
            && uid == unsafe { libc::geteuid() }
            && gid == unsafe { libc::getegid() },
    )?;
    let mut core: libc::rlimit = unsafe { std::mem::zeroed() };
    let mut signal = -1i32;
    require(
        unsafe { libc::getrlimit(libc::RLIMIT_CORE, &mut core) } == 0
            && core.rlim_cur == 0
            && core.rlim_max == 0,
    )?;
    require(
        unsafe { libc::prctl(libc::PR_GET_NO_NEW_PRIVS, 0usize, 0usize, 0usize, 0usize) } == 1,
    )?;
    require(
        unsafe { libc::prctl(libc::PR_GET_PDEATHSIG, &mut signal, 0usize, 0usize, 0usize) } == 0
            && signal == 0,
    )?;
    let mut header = CapHeader {
        version: 0x2008_0522,
        pid: 0,
    };
    let mut caps = [CapData {
        effective: 0,
        permitted: 0,
        inheritable: 0,
    }; 2];
    require(
        unsafe { libc::syscall(libc::SYS_capget, &mut header, caps.as_mut_ptr()) } == 0
            && caps
                .iter()
                .all(|c| c.effective == 0 && c.permitted == 0 && c.inheritable == 0),
    )?;
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    require(
        unsafe { libc::fstat(3, &mut stat) } == 0 && stat.st_mode & libc::S_IFMT == libc::S_IFIFO,
    )?;
    let flags = unsafe { libc::fcntl(3, libc::F_GETFL) };
    require(
        flags >= 0
            && flags & (libc::O_ACCMODE | libc::O_NONBLOCK) == (libc::O_WRONLY | libc::O_NONBLOCK)
            && unsafe { libc::fcntl(3, libc::F_GETFD) } == 0,
    )?;
    require(unsafe { libc::write(3, MARKER.as_ptr().cast(), MARKER.len()) } == 16)?;
    require(unsafe { libc::kill(libc::getpid(), libc::SIGSTOP) } == 0)
}
pub(super) fn child_entry() -> ! {
    let okay = child_context();
    // Never continue to a workload, second exec, signing or handoff even if resumed.
    unsafe { libc::_exit(if okay.is_ok() { 93 } else { 92 }) }
}
pub(super) fn isolated_child_entry(probe: FixedProbe) -> ! {
    if child_context().is_err() {
        unsafe { libc::_exit(92) }
    }
    isolation::child(probe)
}

#[cfg(test)]
#[path = "diagnostic_signer_owned_root_exec_tests.rs"]
mod tests;
