//! Actual original creating-thread loss while its original TGID survives.
use super::super::super::constructor_guard::{Cut, Fault, Hook, CUTS};
use super::super::super::creator_task as task;
use super::*;
use std::sync::{mpsc, OnceLock};
const ENTRY_TASK: &str = "diagnostic_signer_root_fixture::rooted_exec::handoff::loss::creator_loss::signer_creator_loss_entry";
const ROLE_TASK: &str = "HYDRACACHE_LOCAL_SIGNER_TASK_074";
const SPEC_TASK: &str = "HYDRACACHE_LOCAL_SIGNER_TASK_SPEC_074";
const MAGIC_TASK: &[u8; 16] = b"HC074-TASK-v001!";
#[derive(Clone, Copy)]
struct TaskSpec {
    cut: Cut,
    control: bool,
    healthy: bool,
}
impl TaskSpec {
    fn new(cut: u8, control: bool, healthy: bool) -> io::Result<Self> {
        let cut = *CUTS.get(cut as usize).ok_or_else(invalid)?;
        if control && (cut != Cut::BeforeArm || healthy) {
            return Err(invalid());
        }
        Ok(Self {
            cut,
            control,
            healthy,
        })
    }
    fn environment() -> io::Result<Self> {
        let value = std::env::var(SPEC_TASK).map_err(|_| invalid())?;
        Self::parse(&value)
    }
    fn parse(value: &str) -> io::Result<Self> {
        let b = value.as_bytes();
        if b.len() != 3
            || !(b'0'..=b'4').contains(&b[0])
            || !matches!(b[1], b'0' | b'1')
            || !matches!(b[2], b'0' | b'1')
        {
            return Err(invalid());
        }
        Self::new(b[0] - b'0', b[1] == b'1', b[2] == b'1')
    }
    fn command(self, role: &'static str) -> io::Result<Command> {
        let mut cmd = Command::new(std::env::current_exe()?);
        cmd.args(["--exact", ENTRY_TASK, "--nocapture", "--test-threads=1"])
            .env_clear()
            .env("HOME", std::env::var_os("HOME").ok_or_else(invalid)?)
            .env(ROLE_TASK, role)
            .env(
                SPEC_TASK,
                format!(
                    "{}{}{}",
                    self.cut as u8,
                    u8::from(self.control),
                    u8::from(self.healthy)
                ),
            )
            .stdin(Stdio::null());
        Ok(cmd)
    }
    fn frame(self, tid: u32) -> [u8; 32] {
        let mut b = [0u8; 32];
        b[..16].copy_from_slice(MAGIC_TASK);
        b[16] = self.cut as u8;
        b[17] = u8::from(self.control);
        b[18] = u8::from(self.healthy);
        b[20..24].copy_from_slice(&tid.to_le_bytes());
        b
    }
    fn terminal(self, actual: (i32, i32)) -> bool {
        !self.healthy
            && actual
                == if self.control {
                    (libc::CLD_EXITED, 86)
                } else if self.cut == Cut::BeforeArm {
                    (libc::CLD_EXITED, 81)
                } else {
                    (libc::CLD_KILLED, libc::SIGKILL)
                }
    }
}
fn decode_task(spec: TaskSpec, b: &[u8]) -> io::Result<u32> {
    if b.len() != 32 {
        return Err(invalid());
    }
    let tid = u32::from_le_bytes(b[20..24].try_into().map_err(|_| invalid())?);
    if tid == 0 || tid > i32::MAX as u32 || b != spec.frame(tid) {
        return Err(invalid());
    }
    Ok(tid)
}
fn pending(fd: &impl AsRawFd) -> io::Result<bool> {
    let mut p = libc::pollfd {
        fd: fd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: zero-timeout readonly observation of the original pidfd only.
    let n = unsafe { libc::poll(&mut p, 1, 0) };
    if n < 0 || p.revents & (libc::POLLERR | libc::POLLNVAL) != 0 {
        return Err(invalid());
    }
    Ok(n == 0)
}
fn gone(original: &File) -> io::Result<bool> {
    // SAFETY: fixed fresh component relative to the retained original proc task, never TID reopen.
    let raw = unsafe {
        libc::openat(
            original.as_raw_fd(),
            c"status".as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if raw < 0 {
        let e = io::Error::last_os_error();
        if matches!(e.raw_os_error(), Some(libc::ESRCH | libc::ENOENT)) {
            return Ok(true);
        }
        return Err(e);
    }
    // SAFETY: fresh readonly file transferred exactly once and immediately closed.
    drop(unsafe { File::from_raw_fd(raw) });
    Ok(false)
}
fn wait_gone(original: &File) -> io::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !gone(original)? {
        if Instant::now() >= deadline {
            return Err(io::Error::other("original creator did not exit"));
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}
fn plain(socket: &File, b: &[u8]) -> io::Result<()> {
    if b.is_empty() || b.len() > 32 {
        return Err(invalid());
    }
    // SAFETY: bounded nonsecret bytes on the original private observer endpoint.
    if unsafe {
        libc::send(
            socket.as_raw_fd(),
            b.as_ptr().cast(),
            b.len(),
            libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT,
        )
    } != b.len() as isize
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
fn exact(socket: &File, peer: u32, value: &[u8]) -> io::Result<()> {
    let (b, fds) = channel::packet(socket, peer)?;
    if b != value || !fds.is_empty() {
        return Err(invalid());
    }
    Ok(())
}
fn creator_target(
    child: &File,
    tracer: u32,
    tid: u32,
    spec: TaskSpec,
    original: &File,
) -> io::Result<()> {
    let raw = child.as_raw_fd();
    if fs::read_link(format!("/proc/self/fd/{raw}"))? != std::path::Path::new("anon_inode:[pidfd]")
    {
        return Err(invalid());
    }
    let info = text(format!("/proc/self/fdinfo/{raw}"))?;
    let pid = scalar(&info, "Pid")?;
    if pid == 0 || pid > i32::MAX as u32 || scalar(&info, "NSpid")? != pid {
        return Err(invalid());
    }
    let state = text(format!("/proc/{pid}/status"))?;
    if scalar(&state, "PPid")? != tracer
        || scalar(&state, "TracerPid")?
            != if spec.cut as u8 >= Cut::Traceme as u8 {
                tid
            } else {
                0
            }
    {
        return Err(invalid());
    }
    let selected = magic(format!("/proc/{tracer}/task/{tid}"))?;
    if !task::directory(original)
        || identity(original)? != identity(&selected)?
        || !task::live(original.as_raw_fd(), tracer as i32, tid as i32)
    {
        return Err(invalid());
    }
    // All fixture children are held at fixed cuts and none can exit/create another child in this bracket.
    let raw = unsafe {
        libc::openat(
            original.as_raw_fd(),
            c"children".as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if raw < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: original fixed-component descriptor transferred once.
    let children = unsafe { File::from_raw_fd(raw) };
    let mut b = [0u8; 129];
    let n = children.read_at(&mut b, 0)?;
    if n > 128 || std::str::from_utf8(&b[..n]).map_err(|_| invalid())?.trim() != pid.to_string() {
        return Err(invalid());
    }
    Ok(())
}
enum Keeper {
    Stop,
    Observe {
        socket: File,
        child: File,
        creator: File,
    },
}
static KEEPER: OnceLock<mpsc::SyncSender<Keeper>> = OnceLock::new();
fn keeper(rx: mpsc::Receiver<Keeper>) -> io::Result<()> {
    match rx.recv().map_err(|_| invalid())? {
        Keeper::Stop => Ok(()),
        Keeper::Observe {
            socket,
            child,
            creator,
        } => {
            wait_gone(&creator)?;
            channel::response(&socket, 2)?;
            readable(&child, 5000)?;
            let actual = wait_info(&child, libc::WNOWAIT)?;
            let code = u8::try_from(actual.0).map_err(|_| invalid())?;
            let status = u8::try_from(actual.1).map_err(|_| invalid())?;
            plain(&socket, &[3, code, status])?;
            if channel::command(&socket)? != 1 {
                return Err(invalid());
            }
            // SAFETY: only this dedicated owned tracer group exits, after the original child is terminal.
            unsafe {
                libc::_exit(0);
            }
        }
    }
}
fn thread_cut(own: &RootedExec, hook: &Hook) -> io::Result<()> {
    if hook.cut != Cut::Stopped {
        exact(&hook.parent, own.pid as u32, &[hook.cut as u8])?;
    }
    let spec = TaskSpec::environment()?;
    // SAFETY: original report FD3 stays owned by task_tracer throughout this callback.
    let socket = unsafe { std::os::fd::BorrowedFd::borrow_raw(3) }.try_clone_to_owned()?;
    let socket = File::from(socket);
    channel::send(
        &socket,
        &[1],
        &[hook.parent.as_raw_fd(), own.creator.task.as_raw_fd()],
    )?;
    let mut fds = vec![own.pidfd.as_ref().ok_or_else(invalid)?.as_raw_fd()];
    fds.extend(own.root.dirs.iter().map(|d| d.file.as_raw_fd()));
    channel::send(&socket, &spec.frame(own.creator.tid as u32), &fds)?;
    let command = channel::command(&socket)?;
    if spec.healthy {
        if command != 0 {
            return Err(invalid());
        }
        if hook.cut != Cut::Stopped {
            channel::response(&hook.parent, 1)?;
        }
        return Ok(());
    }
    if command != 1 {
        return Err(invalid());
    }
    KEEPER
        .get()
        .ok_or_else(invalid)?
        .send(Keeper::Observe {
            socket,
            child: File::from(own.pidfd.as_ref().ok_or_else(invalid)?.try_clone()?),
            creator: own.creator.task.try_clone()?,
        })
        .map_err(|_| invalid())?;
    // SAFETY: dedicated original creating thread only, no locks/allocations/destructors after this point.
    unsafe {
        libc::syscall(libc::SYS_exit, 0i32);
        libc::_exit(92);
    }
}
fn task_tracer(spec: TaskSpec) -> io::Result<()> {
    // SAFETY: privately inherited fixed endpoint transferred exactly once in this exact role.
    let socket = unsafe { File::from_raw_fd(3) };
    if unsafe { libc::fcntl(socket.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let (tx, rx) = mpsc::sync_channel(1);
    KEEPER.set(tx).map_err(|_| invalid())?;
    let join = std::thread::spawn(move || {
        if let Err(e) = keeper(rx) {
            eprintln!("original keeper refused: {e}");
            unsafe {
                libc::_exit(91);
            }
        }
    });
    let (parent, child) = channel::sockets()?;
    let hook = Hook {
        cut: spec.cut,
        fault: if spec.control {
            Fault::NoCreatorCheck
        } else {
            Fault::None
        },
        parent,
        child,
        observe: thread_cut,
    };
    channel::response(&socket, 0)?;
    let mut own = RootedExec::build_observed(
        super::super::super::Case::SelfExec,
        Some(super::super::super::namespace::Case::Probe(
            super::super::super::namespace::Probe::Unshare,
        )),
        Some(Case::Good),
        Some(&hook),
    )?;
    channel::response(&socket, 1)?;
    own.enter()?;
    channel::response(&socket, 2)?;
    own.first_exec()?;
    channel::response(&socket, 3)?;
    own.install()?;
    channel::response(&socket, 4)?;
    ready(&mut own)?;
    channel::response(&socket, 5)?;
    deliver(&mut own)?;
    channel::response(&socket, 6)?;
    complete(&mut own)?;
    channel::response(&socket, 7)?;
    drop(own);
    KEEPER
        .get()
        .ok_or_else(invalid)?
        .send(Keeper::Stop)
        .map_err(|_| invalid())?;
    join.join().map_err(|_| invalid())?;
    Ok(())
}
fn task_monitor(spec: TaskSpec) -> io::Result<()> {
    // SAFETY: only this isolated subprocess becomes a subreaper.
    if unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1usize, 0usize, 0usize, 0usize) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let parent = tempfile::tempdir()?;
    let bound = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(parent.path())?;
    if unsafe { libc::fchmod(bound.as_raw_fd(), 0o700) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let (socket, passed) = channel::sockets()?;
    let raw = passed.as_raw_fd();
    let mut cmd = spec.command("tracer")?;
    cmd.env("TMPDIR", parent.path());
    // SAFETY: original scalar dup2 only in Command's child, no Rust allocation.
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
    let peer = tracer.child.id();
    exact(&socket, peer, &[0])?;
    let (prefix, extra) = channel::packet(&socket, peer)?;
    if prefix != [1] || extra.len() != 2 {
        return Err(invalid());
    }
    let (bytes, fds) = channel::packet(&socket, peer)?;
    if fds.len() != 5 {
        return Err(invalid());
    }
    let tid = decode_task(spec, &bytes)?;
    creator_target(&fds[0], peer, tid, spec, &extra[1])?;
    directories(&parent, &bound, &fds[1..])?;
    if spec.healthy {
        channel::response(&socket, 0)?;
        for ordinal in 1..=7 {
            exact(&socket, peer, &[ordinal])?;
        }
        if !tracer.wait(5000)?.success() {
            return Err(invalid());
        }
        readable(&fds[0], 5000)?;
        if wait_info(&fds[0], libc::WNOHANG)
            .err()
            .and_then(|e| e.raw_os_error())
            != Some(libc::ECHILD)
            || fs::read_dir(parent.path())?.next().is_some()
        {
            return Err(invalid());
        }
        println!("creator_terminal cut={} control=false healthy=true original_ACK_close_exit0=true keeper_joined=true",spec.cut as u8);
    } else {
        if !pending(&tracer.fd)? {
            return Err(invalid());
        }
        channel::response(&socket, 1)?;
        exact(&socket, peer, &[2])?;
        if !gone(&extra[1])? || !pending(&tracer.fd)? {
            return Err(invalid());
        }
        if spec.cut == Cut::BeforeArm {
            if !pending(&fds[0])? {
                return Err(invalid());
            }
            channel::response(&extra[0], 1)?;
        }
        let (terminal, rights) = channel::packet(&socket, peer)?;
        if terminal.len() != 3 || terminal[0] != 3 || !rights.is_empty() {
            return Err(invalid());
        }
        let actual = (terminal[1] as i32, terminal[2] as i32);
        readable(&fds[0], 5000)?;
        if !spec.terminal(actual) || !pending(&tracer.fd)? {
            return Err(io::Error::other(format!(
                "creator terminal refused {actual:?}"
            )));
        }
        // Until its original TGID exits, this independent monitor must not be the child's waiter.
        if wait_info(&fds[0], libc::WNOHANG)
            .err()
            .and_then(|e| e.raw_os_error())
            != Some(libc::ECHILD)
        {
            return Err(invalid());
        }
        channel::response(&socket, 1)?;
        if !tracer.wait(5000)?.success() {
            return Err(invalid());
        }
        if wait_info(&fds[0], libc::WNOWAIT)? != actual
            || wait_info(&fds[0], 0)? != actual
            || wait_info(&fds[0], libc::WNOHANG)
                .err()
                .and_then(|e| e.raw_os_error())
                != Some(libc::ECHILD)
        {
            return Err(invalid());
        }
        directories(&parent, &bound, &fds[1..])?;
        for dir in fds[1..].iter().rev() {
            // SAFETY: original checked temporary dirs only, after independent exact original-child reap.
            if unsafe { libc::fchmod(dir.as_raw_fd(), 0o755) } != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        println!("creator_terminal cut={} control={} healthy=false code={} status={} original_tgid_alive_at_child_terminal=true independent_reaped=true child_signaled=false dummy_sends=0",spec.cut as u8,spec.control,actual.0,actual.1);
    }
    parent.close()?;
    println!("creator_original proof=complete");
    Ok(())
}
fn creator_loss_case(cut: u8, control: bool, healthy: bool) -> io::Result<()> {
    let spec = TaskSpec::new(cut, control, healthy)?;
    observe_task("monitor", spec)
}
fn observe_task(role: &'static str, spec: TaskSpec) -> io::Result<()> {
    let before = subreaper()?;
    let capture = tempfile::tempfile()?;
    let mut cmd = spec.command(role)?;
    cmd.stdout(capture.try_clone()?)
        .stderr(capture.try_clone()?);
    let mut own = Process::spawn(&mut cmd, true)?;
    readable(&own.fd, 30000)?;
    let actual = wait_info(&own.fd, libc::WNOWAIT)?;
    let mut b = [0u8; 8193];
    let n = capture.read_at(&mut b, 0)?;
    if actual != (libc::CLD_EXITED, 0) || n > 8192 {
        eprintln!(
            "creator observer refused: {}",
            String::from_utf8_lossy(&b[..n.min(8192)])
        );
        return Err(invalid());
    }
    if !own.wait(5000)?.success() || subreaper()? != before {
        return Err(invalid());
    }
    let output = std::str::from_utf8(&b[..n]).map_err(|_| invalid())?;
    if output.matches("creator_original proof=complete").count() != 1 {
        return Err(invalid());
    }
    for line in output
        .lines()
        .filter(|l| l.starts_with("creator_terminal ") || l.starts_with("creator_task_lifetime "))
    {
        println!("{line}");
    }
    Ok(())
}
#[test]
fn signer_creator_loss_entry() {
    match std::env::var(ROLE_TASK).ok().as_deref() {
        None => {}
        Some("monitor") => task_monitor(TaskSpec::environment().unwrap()).unwrap(),
        Some("tracer") => task_tracer(TaskSpec::environment().unwrap()).unwrap(),
        Some("task-lifetime") => task_lifetime().unwrap(),
        _ => panic!("unknown closed creator role"),
    }
}
#[test]
fn signer_creator_loss_before_arm_refuses_dead_original_task() {
    creator_loss_case(0, false, false).unwrap();
}
#[test]
fn signer_creator_loss_armed_boundaries_require_sigkill_before_group_exit() {
    for cut in 1..5 {
        creator_loss_case(cut, false, false).unwrap();
    }
}
#[test]
fn signer_creator_loss_old_getppid_control_misses_creator_death() {
    creator_loss_case(0, true, false).unwrap();
}
#[test]
fn signer_creator_loss_healthy_controls_finish_original_handoff() {
    for cut in 0..5 {
        creator_loss_case(cut, false, true).unwrap();
    }
}
fn task_lifetime() -> io::Result<()> {
    let (tx, rx) = mpsc::sync_channel(1);
    let (release, waiting) = mpsc::sync_channel(1);
    let join = std::thread::spawn(move || {
        let original = task::Creator::capture(unsafe { libc::getpid() }).unwrap();
        tx.send(original).unwrap();
        waiting.recv().unwrap();
    });
    let original = rx
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| invalid())?;
    if !task::directory(&original.task)
        || !task::live(original.task.as_raw_fd(), original.tgid, original.tid)
    {
        return Err(invalid());
    }
    let duplicate = original.task.try_clone()?;
    let stamp = identity(&duplicate)?;
    release.send(()).map_err(|_| invalid())?;
    join.join().map_err(|_| invalid())?;
    wait_gone(&original.task)?;
    if task::live(original.task.as_raw_fd(), original.tgid, original.tid)
        || !gone(&duplicate)?
        || identity(&duplicate)? != stamp
    {
        return Err(invalid());
    }
    let raw = original.task.as_raw_fd();
    drop(original);
    // Only this closed post-exec role operates on FDs here; no concurrent test opens can reuse it.
    if unsafe { libc::fcntl(raw, libc::F_GETFD) } != -1
        || io::Error::last_os_error().raw_os_error() != Some(libc::EBADF)
    {
        return Err(invalid());
    }
    let raw = duplicate.as_raw_fd();
    drop(duplicate);
    if unsafe { libc::fcntl(raw, libc::F_GETFD) } != -1
        || io::Error::last_os_error().raw_os_error() != Some(libc::EBADF)
    {
        return Err(invalid());
    }
    let regular = tempfile::tempfile()?;
    let directory = tempfile::tempdir()?;
    if task::directory(&regular) || task::directory(&File::open(directory.path())?) {
        return Err(invalid());
    }
    let current = task::Creator::capture(unsafe { libc::getpid() })?;
    let path_only = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_PATH | libc::O_DIRECTORY | libc::O_CLOEXEC)
        .open("/proc/thread-self")?;
    if task::directory(&path_only) {
        return Err(invalid());
    }
    let fixture = File::open(directory.path())?;
    fs::write(directory.path().join("status"), vec![b'A'; 8193])?;
    let count = || fs::read_dir("/proc/self/fd").map(|entries| entries.count());
    let before = count()?;
    for _ in 0..32 {
        if !task::live(current.task.as_raw_fd(), current.tgid, current.tid)
            || task::live(current.task.as_raw_fd(), current.tgid, current.tid + 1)
            || task::live(fixture.as_raw_fd(), current.tgid, current.tid)
        {
            return Err(invalid());
        }
    }
    if count()? != before {
        return Err(invalid());
    }
    println!("\ncreator_task_lifetime original_dead_object_not_refreshed=true duplicates_closed=true nonproc_opath_refused=true reader_fds_reconciled=true");
    println!("creator_original proof=complete");
    Ok(())
}
#[test]
fn signer_creator_loss_original_task_descriptor_death_and_release() {
    observe_task("task-lifetime", TaskSpec::new(0, false, false).unwrap()).unwrap();
}
#[test]
fn signer_creator_loss_status_and_terminal_frame_mutations_refuse() {
    for state in [b'R', b'S', b'D', b'T', b't', b'I'] {
        let valid = format!(
            "Name:\ttest\nState:\t{} (live)\nTgid:\t123\nPid:\t124\n",
            state as char
        );
        assert!(task::status(valid.as_bytes(), 123, 124));
        for (group, tid) in [(0, 124), (123, 0), (124, 124), (123, 125)] {
            assert!(!task::status(valid.as_bytes(), group, tid));
        }
        for extra in ["Pid:\t124\n", "Tgid:\t123\n", "State:\tR (running)\n"] {
            assert!(!task::status(
                format!("{valid}{extra}").as_bytes(),
                123,
                124
            ));
        }
    }
    for state in [
        "Z (zombie)",
        "X (dead)",
        "x (dead)",
        "? (future)",
        "R",
        "RR (bad)",
        "",
    ] {
        assert!(!task::status(
            format!("State:\t{state}\nTgid:\t123\nPid:\t124\n").as_bytes(),
            123,
            124
        ));
    }
    for pid in ["", "0", "-1", "+124", "2147483648", "124x"] {
        assert!(!task::status(
            format!("State:\tS (sleeping)\nTgid:\t123\nPid:\t{pid}\n").as_bytes(),
            123,
            124
        ));
    }
    for bytes in [
        b"".as_slice(),
        b"State:\tS (sleeping)\nPid:\t124\n",
        b"Tgid:\t123\nPid:\t124\n",
        b"State:\tS (sleeping)\nTgid:\t123\nPid:\t124",
    ] {
        assert!(!task::status(bytes, 123, 124));
    }
    let valid = b"State:\tS (sleeping)\nTgid:\t123\nPid:\t124\n";
    let mut exact = vec![b'A'; 8192 - valid.len() - 1];
    exact.push(b'\n');
    exact.extend_from_slice(valid);
    assert!(task::status(&exact, 123, 124));
    exact.insert(0, b'A');
    assert!(!task::status(&exact, 123, 124));
    for cut in 0..5 {
        let spec = TaskSpec::new(cut, false, false).unwrap();
        let b = spec.frame(123);
        assert_eq!(decode_task(spec, &b).unwrap(), 123);
        for index in (0..20).chain(24..32) {
            let mut changed = b;
            changed[index] ^= 0x80;
            assert!(decode_task(spec, &changed).is_err());
        }
        for tid in [0, u32::MAX, i32::MAX as u32 + 1] {
            assert!(decode_task(spec, &spec.frame(tid)).is_err());
        }
        for size in [0, 1, 31, 33, 256] {
            assert!(decode_task(spec, &vec![0; size]).is_err());
        }
        for status in 0..=255 {
            assert_eq!(
                spec.terminal((libc::CLD_KILLED, status)),
                cut != 0 && status == libc::SIGKILL
            );
            assert_eq!(
                spec.terminal((libc::CLD_EXITED, status)),
                cut == 0 && status == 81
            );
            assert!(!spec.terminal((libc::CLD_DUMPED, status)));
            assert!(!TaskSpec::new(cut, false, true)
                .unwrap()
                .terminal((libc::CLD_EXITED, status)));
        }
    }
    for input in ["", "0000", "500", "020", "002", "111", "?00"] {
        assert!(TaskSpec::parse(input).is_err());
    }
    for status in 0..=255 {
        let c = TaskSpec::new(0, true, false).unwrap();
        assert_eq!(c.terminal((libc::CLD_EXITED, status)), status == 86);
        assert!(!c.terminal((libc::CLD_KILLED, status)));
        assert!(!TaskSpec::new(0, false, false)
            .unwrap()
            .terminal((libc::CLD_EXITED, 86)));
    }
    let mut seed = 0x0740_c072_u64;
    for _ in 0..256 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let spec = TaskSpec::new(seed as u8 % 5, false, false).unwrap();
        let mut b = spec.frame(123);
        b[seed as usize % 16] ^= (seed as u8).max(1);
        assert!(decode_task(spec, &b).is_err());
        let invalid = format!("State:\tZ (zombie)\nTgid:\t123\nPid:\t{}\n", seed as u32);
        assert!(!task::status(invalid.as_bytes(), 123, 124));
    }
    println!("creator_mutations seed=0x0740c072 cases=256 exact_status_frame_terminal=true control_never_positive=true");
}
#[test]
fn signer_creator_loss_seeded_fresh_and_independent_observers() {
    let mut seed = 0x0740_c072_u64;
    for _ in 0..8 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        creator_loss_case(seed as u8 % 5, false, false).unwrap();
    }
    let joins: Vec<_> = (1..5)
        .map(|cut| std::thread::spawn(move || creator_loss_case(cut, false, false).unwrap()))
        .collect();
    for join in joins {
        join.join().unwrap();
    }
    println!("creator_concurrency seed=0x0740c072 fresh=8 independent=4 ordinary_subreaper_unchanged=true");
}
