//! Local observer of actual tracer death, never signing or worker authority.
use super::*;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Child, Command, ExitStatus, Stdio};
#[path = "diagnostic_signer_tracer_loss_channel.rs"]
mod channel;
const ENTRY: &str =
    "diagnostic_signer_root_fixture::rooted_exec::handoff::loss::tests::signer_tracer_loss_entry";
const ROLE: &str = "HYDRACACHE_LOCAL_SIGNER_LOSS_074";
const STAGE: &str = "HYDRACACHE_LOCAL_SIGNER_LOSS_STAGE_074";
const MODE: &str = "HYDRACACHE_LOCAL_SIGNER_LOSS_MODE_074";
const MAGIC: &[u8; 16] = b"HC074-LOSS-v001!";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum Stage {
    Initial,
    Rooted,
    Exec,
    Installed,
    Ready,
    Delivered,
}
const STAGES: [Stage; 6] = [
    Stage::Initial,
    Stage::Rooted,
    Stage::Exec,
    Stage::Installed,
    Stage::Ready,
    Stage::Delivered,
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Kill,
    NoExitkill,
    Healthy,
    WrongStage,
    Short,
    MissingFd,
    SelfPidfd,
    WrongDir,
    WrongProgress,
}
impl Mode {
    fn arg(self) -> &'static str {
        match self {
            Self::Kill => "kill",
            Self::NoExitkill => "no-exitkill",
            Self::Healthy => "healthy",
            Self::WrongStage => "wrong-stage",
            Self::Short => "short",
            Self::MissingFd => "missing-fd",
            Self::SelfPidfd => "self-pidfd",
            Self::WrongDir => "wrong-dir",
            Self::WrongProgress => "wrong-progress",
        }
    }
    fn rejected(self) -> bool {
        !matches!(self, Self::Kill | Self::NoExitkill | Self::Healthy)
    }
}
#[derive(Clone, Copy, Debug)]
struct Spec {
    stage: Stage,
    mode: Mode,
}
impl Spec {
    fn validate(self) -> io::Result<()> {
        if (self.mode == Mode::NoExitkill && (self.stage as u8) < Stage::Installed as u8)
            || (self.mode == Mode::Healthy && self.stage != Stage::Delivered)
            || (self.mode.rejected() && self.stage != Stage::Installed)
        {
            return Err(invalid());
        }
        Ok(())
    }
    fn environment() -> io::Result<Self> {
        let stage = std::env::var(STAGE)
            .map_err(|_| invalid())?
            .parse::<usize>()
            .map_err(|_| invalid())?;
        let stage = *STAGES.get(stage).ok_or_else(invalid)?;
        let input = std::env::var(MODE).map_err(|_| invalid())?;
        let mode = [
            Mode::Kill,
            Mode::NoExitkill,
            Mode::Healthy,
            Mode::WrongStage,
            Mode::Short,
            Mode::MissingFd,
            Mode::SelfPidfd,
            Mode::WrongDir,
            Mode::WrongProgress,
        ]
        .into_iter()
        .find(|m| m.arg() == input)
        .ok_or_else(invalid)?;
        let spec = Self { stage, mode };
        spec.validate()?;
        Ok(spec)
    }
}
fn frame(stage: Stage, tid: u32) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[..16].copy_from_slice(MAGIC);
    bytes[16] = stage as u8;
    bytes[17] = u8::from(stage == Stage::Delivered);
    bytes[20..24].copy_from_slice(&tid.to_le_bytes());
    bytes
}
fn decode(bytes: &[u8; 32], expected: Stage) -> io::Result<u32> {
    let tid = u32::from_le_bytes(bytes[20..24].try_into().unwrap());
    if tid == 0 || tid > i32::MAX as u32 || bytes != &frame(expected, tid) {
        return Err(invalid());
    }
    Ok(tid)
}
fn readable(fd: &impl AsRawFd, millis: i32) -> io::Result<()> {
    let mut poll = libc::pollfd {
        fd: fd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: one initialized owned-descriptor poll record, bounded stall.
    let result = unsafe { libc::poll(&mut poll, 1, millis) };
    if result != 1
        || poll.revents & libc::POLLIN == 0
        || poll.revents & (libc::POLLERR | libc::POLLNVAL) != 0
    {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "original loss observer stalled",
        ));
    }
    Ok(())
}
fn pidfd(pid: libc::pid_t) -> io::Result<OwnedFd> {
    // SAFETY: used only for a just-created unreaped direct child, or the self-pidfd fault.
    let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0u32) };
    if raw < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fresh kernel FD transferred exactly once.
    Ok(unsafe { OwnedFd::from_raw_fd(raw as i32) })
}
fn signal(fd: &impl AsRawFd) -> io::Result<()> {
    // SAFETY: only internally created or independently validated original pidfd.
    if unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            fd.as_raw_fd(),
            libc::SIGKILL,
            0usize,
            0u32,
        )
    } < 0
        && io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
struct Process {
    child: Child,
    fd: OwnedFd,
    group: bool,
    reaped: bool,
}
impl Process {
    fn spawn(command: &mut Command, group: bool) -> io::Result<Self> {
        if group {
            command.process_group(0);
        }
        let mut child = command.spawn()?;
        match pidfd(child.id() as i32) {
            Ok(fd) => Ok(Self {
                child,
                fd,
                group,
                reaped: false,
            }),
            Err(error) => {
                // SAFETY: only the just-created unreaped direct child, failed pidfd acquisition.
                unsafe {
                    libc::kill(
                        if group {
                            -(child.id() as i32)
                        } else {
                            child.id() as i32
                        },
                        libc::SIGKILL,
                    );
                }
                child.wait()?;
                Err(error)
            }
        }
    }
    fn wait(&mut self, bound: i32) -> io::Result<ExitStatus> {
        readable(&self.fd, bound)?;
        let status = self.child.wait()?;
        self.reaped = true;
        Ok(status)
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        if self.reaped {
            return;
        }
        if self.group {
            // SAFETY: isolated group created for this still-unreaped original leader only.
            unsafe {
                libc::kill(-(self.child.id() as i32), libc::SIGKILL);
            }
        } else if let Err(e) = signal(&self.fd) {
            eprintln!("loss emergency tracer signal failed: {e}");
        }
        if let Err(e) = self.child.wait() {
            eprintln!("loss emergency exact reap failed: {e}");
        }
    }
}
fn invocation(role: &'static str, spec: Spec) -> io::Result<Command> {
    spec.validate()?;
    let mut cmd = Command::new(std::env::current_exe()?);
    cmd.args(["--exact", ENTRY, "--nocapture", "--test-threads=1"])
        .env_clear()
        .env("HOME", std::env::var_os("HOME").ok_or_else(invalid)?)
        .env(ROLE, role)
        .env(STAGE, (spec.stage as u8).to_string())
        .env(MODE, spec.mode.arg())
        .stdin(Stdio::null());
    Ok(cmd)
}
fn outer(spec: Spec) -> io::Result<String> {
    observe("monitor", spec)
}
fn observe(role: &'static str, spec: Spec) -> io::Result<String> {
    let before = subreaper()?;
    let capture = tempfile::tempfile()?;
    let mut cmd = invocation(role, spec)?;
    cmd.stdout(capture.try_clone()?)
        .stderr(capture.try_clone()?);
    let mut monitor = Process::spawn(&mut cmd, true)?;
    readable(&monitor.fd, 30000)?;
    // Observe without reaping the original group leader: emergency group cleanup stays safe.
    let status = wait_info(&monitor.fd, libc::WNOWAIT)?;
    if status.0 != libc::CLD_EXITED || status.1 != 0 {
        let mut bytes = [0u8; 8193];
        let n = capture.read_at(&mut bytes, 0)?;
        eprintln!(
            "loss monitor failed: {}",
            String::from_utf8_lossy(&bytes[..n.min(8192)])
        );
        return Err(invalid());
    }
    if !monitor.wait(5000)?.success() {
        return Err(invalid());
    }
    let mut bytes = [0u8; 8193];
    let n = capture.read_at(&mut bytes, 0)?;
    if n > 8192 {
        return Err(invalid());
    }
    let output = String::from_utf8(bytes[..n].to_vec()).map_err(|_| invalid())?;
    let marker = if role == "fd-lifetime" {
        "received_rights proof=complete".to_owned()
    } else {
        format!(
            "original_loss stage={} mode={} proof=complete",
            spec.stage as u8,
            spec.mode.arg()
        )
    };
    if output.matches(&marker).count() != 1 || subreaper()? != before {
        return Err(invalid());
    }
    Ok(output)
}
fn subreaper() -> io::Result<i32> {
    let mut state = -1i32;
    // SAFETY: read-only query of this process's flag, correctly sized initialized output.
    if unsafe {
        libc::prctl(
            libc::PR_GET_CHILD_SUBREAPER,
            &mut state,
            0usize,
            0usize,
            0usize,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(state)
}
fn death_admitted(stage: Stage, mode: Mode, actual: (i32, i32)) -> bool {
    match (mode, actual) {
        (Mode::Kill, (libc::CLD_KILLED, libc::SIGKILL)) => true,
        (Mode::NoExitkill, (libc::CLD_EXITED, value)) => match stage {
            Stage::Installed => matches!(value, 73 | 74),
            Stage::Ready => value == 74,
            Stage::Delivered => matches!(value, 0 | 76),
            _ => false,
        },
        _ => false,
    }
}
fn scalar(text: &str, label: &str) -> io::Result<u32> {
    let prefix = format!("{label}:");
    let mut lines = text.lines().filter_map(|line| line.strip_prefix(&prefix));
    let value = lines
        .next()
        .ok_or_else(invalid)?
        .trim()
        .parse::<u32>()
        .map_err(|_| invalid())?;
    if lines.next().is_some() {
        return Err(invalid());
    }
    Ok(value)
}
fn target(file: &File, tracer: u32, tid: u32) -> io::Result<u32> {
    let raw = file.as_raw_fd();
    if fs::read_link(format!("/proc/self/fd/{raw}"))? != std::path::Path::new("anon_inode:[pidfd]")
    {
        return Err(invalid());
    }
    let info = text(format!("/proc/self/fdinfo/{raw}"))?;
    let child = scalar(&info, "Pid")?;
    if child == 0 || child > i32::MAX as u32 || scalar(&info, "NSpid")? != child {
        return Err(invalid());
    }
    let state = text(format!("/proc/{child}/status"))?;
    if scalar(&state, "PPid")? != tracer
        || scalar(&state, "TracerPid")? != tid
        || scalar(&text(format!("/proc/{tracer}/task/{tid}/status"))?, "Tgid")? != tracer
    {
        return Err(invalid());
    }
    Ok(child)
}
fn at(parent: &File, name: &std::ffi::CStr) -> io::Result<File> {
    // SAFETY: original owned directory plus bounded single component; never a caller path.
    let raw = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY
                | libc::O_CLOEXEC
                | libc::O_NOFOLLOW
                | libc::O_DIRECTORY
                | libc::O_NONBLOCK,
        )
    };
    if raw < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fresh readonly directory descriptor transferred once.
    Ok(unsafe { File::from_raw_fd(raw) })
}
fn directories(parent: &tempfile::TempDir, bound_parent: &File, dirs: &[File]) -> io::Result<()> {
    let parent_meta = bound_parent.metadata()?;
    // SAFETY: unchanged local nonroot observer identity only.
    let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
    if parent_meta.mode() & 0o7777 != 0o700 || parent_meta.uid() != uid || parent_meta.gid() != gid
    {
        return Err(invalid());
    }
    if dirs.len() != 4 || identity(bound_parent)? != identity(&magic(parent.path())?)? {
        return Err(invalid());
    }
    let path = fs::read_link(format!("/proc/self/fd/{}", dirs[0].as_raw_fd()))?;
    eprintln!(
        "loss_root_binding parent={} original_root={}",
        parent.path().display(),
        path.display()
    );
    let container = path.parent().ok_or_else(invalid)?;
    if path.file_name() != Some(std::ffi::OsStr::new("root"))
        || container.parent() != Some(parent.path())
    {
        return Err(invalid());
    }
    let name = container.file_name().ok_or_else(invalid)?.as_bytes();
    if name.len() > 64
        || !name.starts_with(b".tmp")
        || !name[1..].iter().all(u8::is_ascii_alphanumeric)
    {
        return Err(invalid());
    }
    let name = std::ffi::CString::new(name).map_err(|_| invalid())?;
    let container = at(bound_parent, &name)?;
    let m = container.metadata()?;
    eprintln!(
        "loss_container mode={:o} uid={}",
        m.mode() & 0o7777,
        m.uid()
    );
    if m.mode() & 0o7777 != 0o755 || m.uid() != uid || m.gid() != gid {
        return Err(invalid());
    }
    let root = at(&container, c"root")?;
    let lib = at(&root, c"lib")?;
    let arch = at(&lib, c"x86_64-linux-gnu")?;
    let lib64 = at(&root, c"lib64")?;
    for (actual, original) in [&root, &lib, &arch, &lib64].into_iter().zip(dirs) {
        let m = original.metadata()?;
        // SAFETY: fixed descriptor property queries on received owned directories.
        let flags = unsafe { libc::fcntl(original.as_raw_fd(), libc::F_GETFL) };
        eprintln!(
            "loss_directory actual={:?} original={:?} mode={:o} flags={flags}",
            identity(actual)?,
            identity(original)?,
            m.mode() & 0o7777
        );
        if !m.is_dir()
            || m.mode() & 0o7777 != 0o555
            || identity(actual)? != identity(original)?
            || flags < 0
            || flags & (libc::O_ACCMODE | libc::O_PATH) != libc::O_RDONLY
        {
            return Err(invalid());
        }
    }
    let after = bound_parent.metadata()?;
    if identity(bound_parent)? != identity(&magic(parent.path())?)?
        || after.mode() & 0o7777 != 0o700
        || after.uid() != uid
        || after.gid() != gid
    {
        return Err(invalid());
    }
    Ok(())
}
fn wait_info(fd: &impl AsRawFd, options: i32) -> io::Result<(i32, i32)> {
    // SAFETY: initialized siginfo kernel output, exact original pidfd only.
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    if unsafe {
        libc::waitid(
            libc::P_PIDFD,
            fd.as_raw_fd() as u32,
            &mut info,
            libc::WEXITED | options,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful WEXITED wait returns the SIGCHLD union member.
    Ok((info.si_code, unsafe { info.si_status() }))
}
fn monitor(spec: Spec) -> io::Result<()> {
    // SAFETY: only this isolated test subprocess becomes subreaper; host/test parent unchanged.
    if unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1usize, 0usize, 0usize, 0usize) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let parent = tempfile::tempdir()?;
    let bound_parent = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(parent.path())?;
    // SAFETY: this observer's just-created retained temporary parent only, before spawn.
    if unsafe { libc::fchmod(bound_parent.as_raw_fd(), 0o700) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let (socket, passed) = channel::sockets()?;
    let raw = passed.as_raw_fd();
    let mut cmd = invocation("tracer", spec)?;
    cmd.env("TMPDIR", parent.path());
    // SAFETY: only scalar libc descriptor operation after Command's fork; no Rust allocation.
    unsafe {
        cmd.pre_exec(move || {
            if libc::dup2(raw, 3) < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut tracer = Process::spawn(&mut cmd, false)?;
    drop(passed);
    let received = channel::receive(&socket, tracer.child.id(), spec.stage)
        .map_err(|e| io::Error::other(format!("report: {e}")));
    let checked = received.and_then(|r| {
        let tid =
            decode(&r.bytes, spec.stage).map_err(|e| io::Error::other(format!("frame: {e}")))?;
        target(&r.fds[0], tracer.child.id(), tid)
            .map_err(|e| io::Error::other(format!("target: {e}")))?;
        directories(&parent, &bound_parent, &r.fds[1..])
            .map_err(|e| io::Error::other(format!("directories: {e}")))?;
        Ok(r)
    });
    if spec.mode.rejected() {
        let error = checked.err().ok_or_else(invalid)?;
        let expected = match spec.mode {
            Mode::WrongStage => "frame:",
            Mode::Short | Mode::MissingFd | Mode::WrongProgress => "report:",
            Mode::SelfPidfd => "target:",
            Mode::WrongDir => "directories:",
            _ => return Err(invalid()),
        };
        if !error.to_string().starts_with(expected) {
            return Err(error);
        }
        channel::response(&socket, 0)?;
        if !tracer.wait(5000)?.success() || fs::read_dir(parent.path())?.next().is_some() {
            return Err(invalid());
        }
        parent.close()?;
    } else {
        let report = checked?;
        if spec.mode == Mode::Healthy {
            channel::response(&socket, 1)?;
            if !tracer.wait(5000)?.success() {
                return Err(invalid());
            }
            readable(&report.fds[0], 5000)?;
            if wait_info(&report.fds[0], libc::WNOHANG)
                .err()
                .and_then(|e| e.raw_os_error())
                != Some(libc::ECHILD)
                || fs::read_dir(parent.path())?.next().is_some()
            {
                return Err(invalid());
            }
            parent.close()?;
        } else {
            // The only successful-path kill target is the original tracer, never the child.
            signal(&tracer.fd)?;
            if tracer.wait(5000)?.signal() != Some(libc::SIGKILL) {
                return Err(invalid());
            }
            readable(&report.fds[0], 5000)?;
            let actual = wait_info(&report.fds[0], libc::WNOWAIT)?;
            if !death_admitted(spec.stage, spec.mode, actual) {
                return Err(io::Error::other(format!("child death refused: {actual:?}")));
            }
            eprintln!(
                "loss_child_terminal stage={} mode={} code={} status={}",
                spec.stage as u8,
                spec.mode.arg(),
                actual.0,
                actual.1
            );
            if wait_info(&report.fds[0], 0)? != actual
                || wait_info(&report.fds[0], libc::WNOHANG)
                    .err()
                    .and_then(|e| e.raw_os_error())
                    != Some(libc::ECHILD)
            {
                return Err(invalid());
            }
            directories(&parent, &bound_parent, &report.fds[1..])?;
            for dir in report.fds[1..].iter().rev() {
                // SAFETY: validated original fixture dirs only, after exact original child death/reap.
                if unsafe { libc::fchmod(dir.as_raw_fd(), 0o755) } != 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            parent.close()?;
        }
    }
    println!(
        "original_loss stage={} mode={} proof=complete",
        spec.stage as u8,
        spec.mode.arg()
    );
    Ok(())
}
fn tracer(spec: Spec) -> io::Result<()> {
    // SAFETY: fixed privately inherited observer endpoint transferred once in the dedicated role.
    let socket = unsafe { File::from_raw_fd(3) };
    // SAFETY: stop the private report endpoint from leaking into setup subprocess execs.
    if unsafe { libc::fcntl(socket.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } != 0 {
        return Err(io::Error::last_os_error());
    }
    channel::response(&socket, 0)?;
    let mut own = RootedExec::handoff(Case::Good)?;
    channel::response(&socket, 1)?;
    if spec.stage as u8 >= Stage::Rooted as u8 {
        own.enter()?;
        channel::response(&socket, 2)?;
    }
    if spec.stage as u8 >= Stage::Exec as u8 {
        own.first_exec()?;
        channel::response(&socket, 3)?;
    }
    if spec.stage as u8 >= Stage::Installed as u8 {
        own.install()?;
        channel::response(
            &socket,
            if spec.mode == Mode::WrongProgress {
                255
            } else {
                4
            },
        )?;
    }
    if spec.stage as u8 >= Stage::Ready as u8 {
        ready(&mut own)?;
        channel::response(&socket, 5)?;
    }
    if spec.stage == Stage::Delivered {
        deliver(&mut own)?;
        channel::response(&socket, 6)?;
    }
    if spec.mode == Mode::NoExitkill {
        own.trace(
            libc::PTRACE_SETOPTIONS,
            (libc::PTRACE_O_TRACEEXEC | libc::PTRACE_O_TRACESYSGOOD) as usize,
        )?;
    }
    // SAFETY: original trace owner thread, not a caller-supplied process assertion.
    let mut bytes = frame(spec.stage, unsafe { libc::syscall(libc::SYS_gettid) }
        as u32);
    let mut fds = [
        own.pidfd.as_ref().ok_or_else(invalid)?.as_raw_fd(),
        own.root.dirs[0].file.as_raw_fd(),
        own.root.dirs[1].file.as_raw_fd(),
        own.root.dirs[2].file.as_raw_fd(),
        own.root.dirs[3].file.as_raw_fd(),
    ];
    // SAFETY: self-pidfd is only an explicit refusal control; never a supplied PID.
    let foreign = if spec.mode == Mode::SelfPidfd {
        Some(pidfd(unsafe { libc::getpid() })?)
    } else {
        None
    };
    if let Some(fd) = &foreign {
        fds[0] = fd.as_raw_fd();
    }
    if spec.mode == Mode::WrongDir {
        fds[1] = own.root.files[0].file.as_raw_fd();
    }
    if spec.mode == Mode::WrongStage {
        bytes[16] ^= 1;
    }
    let count = if spec.mode == Mode::Short { 31 } else { 32 };
    let fds_count = if spec.mode == Mode::MissingFd { 4 } else { 5 };
    channel::send(&socket, &bytes[..count], &fds[..fds_count])?;
    match channel::command(&socket)? {
        0 if spec.mode.rejected() => own.refuse()?,
        1 if spec.mode == Mode::Healthy => complete(&mut own)?,
        _ => return Err(invalid()),
    }
    Ok(())
}
#[path = "diagnostic_signer_tracer_loss_tests.rs"]
mod tests;
