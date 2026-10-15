//! Original tracer thread loss after options; private local test roles only.
use super::*;
const ENTRY_THREAD: &str = "diagnostic_signer_root_fixture::rooted_exec::handoff::loss::creator_loss::thread_exit::signer_thread_exit_entry";
const ROLE_THREAD: &str = "HYDRACACHE_LOCAL_SIGNER_THREAD_EXIT_074";
const SPEC_THREAD: &str = "HYDRACACHE_LOCAL_SIGNER_THREAD_SPEC_074";
const MAGIC_THREAD: &[u8; 16] = b"HC074-THRD-v001!";
#[derive(Clone, Copy)]
struct ThreadSpec {
    stage: u8,
    mode: u8,
}
impl ThreadSpec {
    fn new(stage: u8, mode: u8) -> io::Result<Self> {
        if stage >= 6 || mode > 2 || (mode == 1 && stage != 1) {
            return Err(invalid());
        }
        Ok(Self { stage, mode })
    }
    fn parse(value: &str) -> io::Result<Self> {
        let b = value.as_bytes();
        if b.len() != 2 || !b.iter().all(u8::is_ascii_digit) {
            return Err(invalid());
        }
        Self::new(b[0] - b'0', b[1] - b'0')
    }
    fn environment() -> io::Result<Self> {
        Self::parse(&std::env::var(SPEC_THREAD).map_err(|_| invalid())?)
    }
    fn command(self, role: &'static str) -> io::Result<Command> {
        let mut cmd = Command::new(std::env::current_exe()?);
        cmd.args(["--exact", ENTRY_THREAD, "--nocapture", "--test-threads=1"])
            .env_clear()
            .env("HOME", std::env::var_os("HOME").ok_or_else(invalid)?)
            .env(ROLE_THREAD, role)
            .env(SPEC_THREAD, format!("{}{}", self.stage, self.mode))
            .stdin(Stdio::null());
        Ok(cmd)
    }
    fn frame(self, tid: u32) -> [u8; 32] {
        let mut b = [0u8; 32];
        b[..16].copy_from_slice(MAGIC_THREAD);
        b[16] = self.stage;
        b[17] = self.mode;
        b[20..24].copy_from_slice(&tid.to_le_bytes());
        b
    }
    fn decode(self, b: &[u8]) -> io::Result<u32> {
        if b.len() != 32 {
            return Err(invalid());
        }
        let tid = u32::from_le_bytes(b[20..24].try_into().unwrap());
        if tid == 0 || tid > i32::MAX as u32 || b != self.frame(tid) {
            return Err(invalid());
        }
        Ok(tid)
    }
    fn positive(self, actual: (i32, i32)) -> bool {
        self.mode == 0 && actual == (libc::CLD_KILLED, libc::SIGKILL)
    }
}
fn advance(own: &mut RootedExec, next: u8) -> io::Result<()> {
    match next {
        1 => own.enter(),
        2 => own.first_exec(),
        3 => own.install(),
        4 => ready(own),
        5 => deliver(own),
        _ => Err(invalid()),
    }
}
fn thread_tracer(spec: ThreadSpec) -> io::Result<()> {
    // SAFETY: this closed role owns its inherited original report endpoint once.
    let socket = unsafe { File::from_raw_fd(3) };
    if unsafe { libc::fcntl(3, libc::F_SETFD, libc::FD_CLOEXEC) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let (tx, rx) = mpsc::sync_channel(1);
    let join = std::thread::spawn(move || {
        if let Err(error) = keeper(rx) {
            eprintln!("thread keeper failed: {error}");
            // SAFETY: isolated owned test role only; failure is never positive proof.
            unsafe { libc::_exit(91) }
        }
    });
    channel::response(&socket, 0)?;
    let mut own = RootedExec::handoff(super::super::super::Case::Good)?;
    for next in 1..=spec.stage {
        advance(&mut own, next)?;
        channel::response(&socket, next)?;
    }
    if own.channel.as_ref().ok_or_else(invalid)?.sends != u32::from(spec.stage == 5) {
        return Err(invalid());
    }
    if spec.mode == 1 {
        // SAFETY: only this original tracer changes its original stopped child's options.
        if unsafe {
            libc::ptrace(
                libc::PTRACE_SETOPTIONS,
                own.pid,
                0usize,
                (libc::PTRACE_O_TRACEEXEC | libc::PTRACE_O_TRACESYSGOOD) as usize,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
    }
    channel::send(&socket, &[1], &[own.creator.task.as_raw_fd()])?;
    let mut fds = vec![own.pidfd.as_ref().ok_or_else(invalid)?.as_raw_fd()];
    fds.extend(own.root.dirs.iter().map(|d| d.file.as_raw_fd()));
    channel::send(&socket, &spec.frame(own.creator.tid as u32), &fds)?;
    if channel::command(&socket)? != u8::from(spec.mode != 2) {
        return Err(invalid());
    }
    if spec.mode == 2 {
        for next in spec.stage + 1..6 {
            advance(&mut own, next)?;
            channel::response(&socket, next)?;
        }
        complete(&mut own)?;
        channel::response(&socket, 6)?;
        drop(own);
        tx.send(Keeper::Stop).map_err(|_| invalid())?;
        join.join().map_err(|_| invalid())?;
        return Ok(());
    }
    tx.send(Keeper::Observe {
        socket: socket.try_clone()?,
        child: File::from(own.pidfd.as_ref().ok_or_else(invalid)?.try_clone()?),
        creator: own.creator.task.try_clone()?,
    })
    .map_err(|_| invalid())?;
    // SAFETY: only this creating/tracing task exits; no Rust unwinding or group exit.
    unsafe {
        libc::syscall(libc::SYS_exit, 0);
        libc::_exit(92);
    }
}
fn thread_monitor(spec: ThreadSpec) -> io::Result<()> {
    // SAFETY: only the isolated owned observer becomes a subreaper.
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
    // SAFETY: original scalar descriptor duplication only after Command's fork.
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
    for next in 1..=spec.stage {
        exact(&socket, peer, &[next])?;
    }
    let (prefix, original) = channel::packet(&socket, peer)?;
    if prefix != [1] || original.len() != 1 {
        return Err(invalid());
    }
    let (bytes, fds) = channel::packet(&socket, peer)?;
    if fds.len() != 5 {
        return Err(invalid());
    }
    let tid = spec.decode(&bytes)?;
    creator_target(
        &fds[0],
        peer,
        tid,
        TaskSpec::new(3, false, false)?,
        &original[0],
    )?;
    let child = target(&fds[0], peer, tid)?;
    // Retain this proc object before loss; never refresh by numeric PID after loss.
    let child_proc = magic(format!("/proc/{child}"))?;
    directories(&parent, &bound, &fds[1..])?;
    channel::response(&socket, u8::from(spec.mode != 2))?;
    if spec.mode == 2 {
        for next in spec.stage + 1..=6 {
            exact(&socket, peer, &[next])?;
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
        println!(
            "thread_terminal stage={} mode=2 original_ACK_close_exit0=true keeper_joined=true",
            spec.stage
        );
    } else {
        exact(&socket, peer, &[2])?;
        if !gone(&original[0])? || !pending(&tracer.fd)? {
            return Err(invalid());
        }
        if spec.mode == 1 {
            let until = Instant::now() + Duration::from_secs(5);
            loop {
                if !pending(&fds[0])? || !pending(&tracer.fd)? {
                    return Err(invalid());
                }
                let status = at_status(&child_proc)?;
                if scalar(&status, "TracerPid")? == 0 {
                    break;
                }
                if Instant::now() >= until {
                    return Err(invalid());
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            println!("thread_negative stage=1 detached=true original_child_nonterminal=true original_tgid_nonterminal=true cleanup_is_not_positive=true");
            // Explicit negative-control cleanup only, never an EXITKILL success.
            signal(&fds[0])?;
        }
        let (b, rights) = channel::packet(&socket, peer)?;
        if b.len() != 3 || b[0] != 3 || !rights.is_empty() {
            return Err(invalid());
        }
        let actual = (b[1] as i32, b[2] as i32);
        readable(&fds[0], 5000)?;
        if (spec.mode == 0 && !spec.positive(actual))
            || (spec.mode == 1 && actual != (libc::CLD_KILLED, libc::SIGKILL))
            || !pending(&tracer.fd)?
        {
            return Err(invalid());
        }
        if wait_info(&fds[0], libc::WNOHANG)
            .err()
            .and_then(|e| e.raw_os_error())
            != Some(libc::ECHILD)
        {
            return Err(invalid());
        }
        channel::response(&socket, 1)?;
        if !tracer.wait(5000)?.success()
            || wait_info(&fds[0], libc::WNOWAIT)? != actual
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
            // SAFETY: exact original temporary directories after independent child reap only.
            if unsafe { libc::fchmod(dir.as_raw_fd(), 0o755) } != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        println!("thread_terminal stage={} mode={} code={} status={} original_tgid_alive_at_child_terminal=true independent_reaped=true child_signaled={} dummy_sends={} positive={}",
            spec.stage, spec.mode, actual.0, actual.1, spec.mode == 1,
            u8::from(spec.stage == 5), spec.positive(actual));
    }
    parent.close()?;
    println!("thread_original proof=complete");
    Ok(())
}
fn at_status(original: &File) -> io::Result<String> {
    // SAFETY: fixed component of the retained original child's proc directory only.
    let raw = unsafe {
        libc::openat(
            original.as_raw_fd(),
            c"status".as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if raw < 0 {
        return Err(io::Error::last_os_error());
    }
    let file = unsafe { File::from_raw_fd(raw) };
    let mut b = [0u8; 8193];
    let n = file.read_at(&mut b, 0)?;
    if n > 8192 {
        return Err(invalid());
    }
    String::from_utf8(b[..n].to_vec()).map_err(|_| invalid())
}
fn thread_exit_case(stage: u8, mode: u8) -> io::Result<()> {
    let spec = ThreadSpec::new(stage, mode)?;
    let before = subreaper()?;
    let capture = tempfile::tempfile()?;
    let mut cmd = spec.command("monitor")?;
    cmd.stdout(capture.try_clone()?)
        .stderr(capture.try_clone()?);
    let mut monitor = Process::spawn(&mut cmd, true)?;
    readable(&monitor.fd, 30000)?;
    let status = wait_info(&monitor.fd, libc::WNOWAIT)?;
    let mut b = [0u8; 8193];
    let n = capture.read_at(&mut b, 0)?;
    if status != (libc::CLD_EXITED, 0) || n > 8192 {
        eprintln!(
            "thread observer refused: {}",
            String::from_utf8_lossy(&b[..n.min(8192)])
        );
        return Err(invalid());
    }
    if !monitor.wait(5000)?.success() || subreaper()? != before {
        return Err(invalid());
    }
    let output = std::str::from_utf8(&b[..n]).map_err(|_| invalid())?;
    if output.matches("thread_original proof=complete").count() != 1 {
        return Err(invalid());
    }
    for line in output
        .lines()
        .filter(|l| l.starts_with("thread_terminal ") || l.starts_with("thread_negative "))
    {
        println!("{line}");
    }
    Ok(())
}
#[test]
fn signer_thread_exit_entry() {
    match std::env::var(ROLE_THREAD).ok().as_deref() {
        None => {}
        Some("monitor") => thread_monitor(ThreadSpec::environment().unwrap()).unwrap(),
        Some("tracer") => thread_tracer(ThreadSpec::environment().unwrap()).unwrap(),
        _ => panic!("unknown closed thread exit role"),
    }
}

#[test]
fn signer_thread_exit_all_original_cuts() {
    for stage in 0..6 {
        thread_exit_case(stage, 0).unwrap();
    }
}
#[test]
fn signer_thread_exit_no_exitkill_is_not_positive() {
    thread_exit_case(1, 1).unwrap();
}
#[test]
fn signer_thread_exit_healthy_original_controls() {
    for stage in 0..6 {
        thread_exit_case(stage, 2).unwrap();
    }
}
#[test]
fn signer_thread_exit_closed_frames_and_terminal_property() {
    for stage in 0..6 {
        for mode in [0, 2] {
            let spec = ThreadSpec::new(stage, mode).unwrap();
            let b = spec.frame(123);
            assert_eq!(spec.decode(&b).unwrap(), 123);
            for index in (0..20).chain(24..32) {
                let mut changed = b;
                changed[index] ^= 0x80;
                assert!(spec.decode(&changed).is_err());
            }
            for tid in [0, u32::MAX, i32::MAX as u32 + 1] {
                assert!(spec.decode(&spec.frame(tid)).is_err());
            }
            for size in [0, 1, 31, 33, 256] {
                assert!(spec.decode(&vec![0; size]).is_err());
            }
            for code in [libc::CLD_EXITED, libc::CLD_KILLED, libc::CLD_DUMPED] {
                for status in 0..=255 {
                    assert_eq!(
                        spec.positive((code, status)),
                        mode == 0 && code == libc::CLD_KILLED && status == libc::SIGKILL
                    );
                    assert!(!ThreadSpec::new(1, 1).unwrap().positive((code, status)));
                }
            }
        }
    }
    for value in ["", "000", "60", "03", "01", "21", "?0", "-1"] {
        assert!(ThreadSpec::parse(value).is_err());
    }
    println!("thread_mutations exact_frame_terminal=true control_never_positive=true");
}
#[test]
fn signer_thread_exit_seeded_fresh_and_concurrent() {
    let mut seed = 0x0740_d073_u64;
    for _ in 0..8 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        thread_exit_case(seed as u8 % 6, 0).unwrap();
    }
    let joins: Vec<_> = (1..5)
        .map(|stage| std::thread::spawn(move || thread_exit_case(stage, 0).unwrap()))
        .collect();
    for join in joins {
        join.join().unwrap();
    }
    println!("thread_concurrency seed=0x0740d073 fresh=8 independent=4 ordinary_subreaper_unchanged=true");
}
