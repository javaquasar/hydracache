//! Private original-parent construction loss, never issuer or worker authority.
use super::super::super::constructor_guard::{Cut, Fault, Hook, CUTS};
use super::*;
const CTOR_ENTRY: &str = "diagnostic_signer_root_fixture::rooted_exec::handoff::loss::early::signer_constructor_loss_entry";
const CTOR_ROLE: &str = "HYDRACACHE_LOCAL_SIGNER_CTOR_074";
const CTOR_SPEC: &str = "HYDRACACHE_LOCAL_SIGNER_CTOR_SPEC_074";
const CTOR_MAGIC: &[u8; 16] = b"HC074-CTOR-v001!";
#[derive(Clone, Copy)]
struct EarlySpec {
    cut: Cut,
    fault: Fault,
    healthy: bool,
}
impl EarlySpec {
    fn new(cut: u8, fault: u8, healthy: bool) -> io::Result<Self> {
        let cut = *CUTS.get(cut as usize).ok_or_else(invalid)?;
        let fault = *[Fault::None, Fault::NoArm, Fault::NoParentCheck]
            .get(fault as usize)
            .ok_or_else(invalid)?;
        if (fault == Fault::NoArm && cut != Cut::Armed)
            || (fault == Fault::NoParentCheck && cut != Cut::BeforeArm)
            || (healthy && fault != Fault::None)
        {
            return Err(invalid());
        }
        Ok(Self {
            cut,
            fault,
            healthy,
        })
    }
    fn environment() -> io::Result<Self> {
        let value = std::env::var(CTOR_SPEC).map_err(|_| invalid())?;
        let bytes = value.as_bytes();
        if bytes.len() != 3 || !bytes.iter().all(u8::is_ascii_digit) || bytes[2] > b'1' {
            return Err(invalid());
        }
        Self::new(bytes[0] - b'0', bytes[1] - b'0', bytes[2] == b'1')
    }
    fn command(self, role: &'static str) -> io::Result<Command> {
        let mut cmd = Command::new(std::env::current_exe()?);
        cmd.args(["--exact", CTOR_ENTRY, "--nocapture", "--test-threads=1"])
            .env_clear()
            .env("HOME", std::env::var_os("HOME").ok_or_else(invalid)?)
            .env(CTOR_ROLE, role)
            .env(
                CTOR_SPEC,
                format!(
                    "{}{}{}",
                    self.cut as u8,
                    self.fault as u8,
                    u8::from(self.healthy)
                ),
            )
            .stdin(Stdio::null());
        Ok(cmd)
    }
    fn frame(self, tid: u32) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[..16].copy_from_slice(CTOR_MAGIC);
        bytes[16] = self.cut as u8;
        bytes[17] = self.fault as u8;
        bytes[20..24].copy_from_slice(&tid.to_le_bytes());
        bytes
    }
    fn terminal(self, actual: (i32, i32)) -> bool {
        if self.healthy {
            return false;
        }
        let expected = match self.fault {
            Fault::NoArm => (libc::CLD_EXITED, 83),
            Fault::NoParentCheck => (libc::CLD_EXITED, 84),
            Fault::NoCreatorCheck => return false,
            Fault::None if self.cut == Cut::BeforeArm => (libc::CLD_EXITED, 81),
            Fault::None => (libc::CLD_KILLED, libc::SIGKILL),
        };
        actual == expected
    }
}
fn decode_frame(spec: EarlySpec, bytes: &[u8]) -> io::Result<u32> {
    if bytes.len() != 32 {
        return Err(invalid());
    }
    let tid = u32::from_le_bytes(bytes[20..24].try_into().map_err(|_| invalid())?);
    if tid == 0 || tid > i32::MAX as u32 || bytes != spec.frame(tid) {
        return Err(invalid());
    }
    Ok(tid)
}
#[test]
fn signer_constructor_loss_entry() {
    match std::env::var(CTOR_ROLE).ok().as_deref() {
        None => {}
        Some("monitor") => early_monitor(EarlySpec::environment().unwrap()).unwrap(),
        Some("tracer") => early_tracer(EarlySpec::environment().unwrap()).unwrap(),
        _ => panic!("unknown closed constructor role"),
    }
}
fn constructor_loss_case(cut: u8, fault: u8, healthy: bool) -> io::Result<()> {
    let spec = EarlySpec::new(cut, fault, healthy)?;
    let before = subreaper()?;
    let capture = tempfile::tempfile()?;
    let mut cmd = spec.command("monitor")?;
    cmd.stdout(capture.try_clone()?)
        .stderr(capture.try_clone()?);
    let mut process = Process::spawn(&mut cmd, true)?;
    readable(&process.fd, 30000)?;
    let actual = wait_info(&process.fd, libc::WNOWAIT)?;
    let mut bytes = [0u8; 8193];
    let n = capture.read_at(&mut bytes, 0)?;
    if actual != (libc::CLD_EXITED, 0) || n > 8192 {
        eprintln!(
            "constructor monitor refused: {}",
            String::from_utf8_lossy(&bytes[..n.min(8192)])
        );
        return Err(invalid());
    }
    if !process.wait(5000)?.success() || subreaper()? != before {
        return Err(invalid());
    }
    let output = String::from_utf8(bytes[..n].to_vec()).map_err(|_| invalid())?;
    if output
        .matches("constructor_original proof=complete")
        .count()
        != 1
    {
        return Err(invalid());
    }
    for line in output
        .lines()
        .filter(|l| l.starts_with("constructor_terminal "))
    {
        println!("{line}");
    }
    Ok(())
}
fn original_target(file: &File, tracer: u32, spec: EarlySpec, tid: u32) -> io::Result<()> {
    if tid == 0 || tid > i32::MAX as u32 {
        return Err(invalid());
    }
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
    let tracing = if spec.cut as u8 >= Cut::Traceme as u8 {
        tid
    } else {
        0
    };
    if scalar(&state, "PPid")? != tracer
        || scalar(&state, "TracerPid")? != tracing
        || scalar(&text(format!("/proc/{tracer}/task/{tid}/status"))?, "Tgid")? != tracer
    {
        return Err(invalid());
    }
    Ok(())
}
fn early_monitor(spec: EarlySpec) -> io::Result<()> {
    // SAFETY: this dedicated isolated observer only; ordinary test process unchanged.
    if unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1usize, 0usize, 0usize, 0usize) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let parent = tempfile::tempdir()?;
    let bound_parent = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(parent.path())?;
    // SAFETY: newly owned retained temporary parent, before any tracer creation.
    if unsafe { libc::fchmod(bound_parent.as_raw_fd(), 0o700) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let (socket, passed) = channel::sockets()?;
    let raw = passed.as_raw_fd();
    let mut cmd = spec.command("tracer")?;
    cmd.env("TMPDIR", parent.path());
    // SAFETY: only original scalar dup2 in Command's child; no allocation or caller descriptor.
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
    let (progress, fds) = channel::packet(&socket, tracer.child.id())?;
    if progress != [0] || !fds.is_empty() {
        return Err(invalid());
    }
    let (bytes, fds) = channel::packet(&socket, tracer.child.id())?;
    if bytes.len() != 32 || fds.len() != 5 {
        return Err(invalid());
    }
    let tid = decode_frame(spec, &bytes)?;
    original_target(&fds[0], tracer.child.id(), spec, tid)?;
    directories(&parent, &bound_parent, &fds[1..])?;
    if spec.healthy {
        channel::response(&socket, 1)?;
        for ordinal in 1..=7 {
            let (bytes, progress_fds) =
                channel::packet(&socket, tracer.child.id()).map_err(|e| {
                    io::Error::other(format!("healthy constructor stage {ordinal}: {e}"))
                })?;
            if bytes != [ordinal] || !progress_fds.is_empty() {
                return Err(invalid());
            }
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
            "constructor_terminal cut={} fault=0 healthy=true original_ACK_close_exit0=true healthy_progress=7",
            spec.cut as u8
        );
    } else {
        signal(&tracer.fd)?;
        if tracer.wait(5000)?.signal() != Some(libc::SIGKILL) {
            return Err(invalid());
        }
        readable(&fds[0], 5000)?;
        let actual = wait_info(&fds[0], libc::WNOWAIT)?;
        if !spec.terminal(actual) {
            return Err(io::Error::other(format!(
                "constructor death {actual:?} refused"
            )));
        }
        if wait_info(&fds[0], 0)? != actual
            || wait_info(&fds[0], libc::WNOHANG)
                .err()
                .and_then(|e| e.raw_os_error())
                != Some(libc::ECHILD)
        {
            return Err(invalid());
        }
        directories(&parent, &bound_parent, &fds[1..])?;
        for dir in fds[1..].iter().rev() {
            // SAFETY: checked original temporary dirs only, after exact original child death/reap.
            if unsafe { libc::fchmod(dir.as_raw_fd(), 0o755) } != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        println!("constructor_terminal cut={} fault={} healthy=false code={} status={} original_pidfd_reaped=true child_signaled=false dummy_sends=0",spec.cut as u8,spec.fault as u8,actual.0,actual.1);
    }
    parent.close()?;
    println!("constructor_original proof=complete");
    Ok(())
}
fn report_cut(own: &RootedExec, hook: &Hook) -> io::Result<()> {
    if hook.cut != Cut::Stopped {
        let (bytes, fds) = channel::packet(&hook.parent, own.pid as u32)?;
        if bytes != [hook.cut as u8] || !fds.is_empty() {
            return Err(invalid());
        }
    }
    let spec = EarlySpec::environment()?;
    // SAFETY: borrowed private FD3 is owned by early_tracer throughout this callback.
    let socket = unsafe { std::os::fd::BorrowedFd::borrow_raw(3) };
    // Use the original owner to retain the report descriptor, not a second owner of FD3.
    let report = File::from(socket.try_clone_to_owned()?);
    let mut fds = vec![own.pidfd.as_ref().ok_or_else(invalid)?.as_raw_fd()];
    fds.extend(own.root.dirs.iter().map(|d| d.file.as_raw_fd()));
    // SAFETY: current original tracer thread only, sent as a bounded corroborated scalar.
    let tid = unsafe { libc::syscall(libc::SYS_gettid) } as u32;
    channel::send(&report, &spec.frame(tid), &fds)?;
    if channel::command(&report)? != 1 {
        return Err(invalid());
    }
    if hook.cut != Cut::Stopped {
        channel::response(&hook.parent, 1)?;
    }
    Ok(())
}
fn early_tracer(spec: EarlySpec) -> io::Result<()> {
    // SAFETY: original privately inherited report endpoint, owned once in this exact role.
    let socket = unsafe { File::from_raw_fd(3) };
    if unsafe { libc::fcntl(socket.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let (parent, child) = channel::sockets()?;
    let hook = Hook {
        cut: spec.cut,
        fault: spec.fault,
        parent,
        child,
        observe: report_cut,
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
    Ok(())
}
#[test]
fn signer_constructor_loss_before_arm_refuses_original_parent_mismatch() {
    constructor_loss_case(0, 0, false).unwrap();
}
#[test]
fn signer_constructor_loss_armed_cuts_require_original_child_sigkill() {
    for cut in 1..5 {
        constructor_loss_case(cut, 0, false).unwrap();
    }
}
#[test]
fn signer_constructor_loss_healthy_cuts_finish_original_handoff() {
    for cut in 0..5 {
        constructor_loss_case(cut, 0, true).unwrap();
    }
}
#[test]
fn signer_constructor_loss_guard_and_parent_check_controls_are_not_positive_proof() {
    constructor_loss_case(1, 1, false).unwrap();
    constructor_loss_case(0, 2, false).unwrap();
}
#[test]
fn signer_constructor_loss_finite_oracle_and_closed_frame_mutations() {
    for cut in 0..5 {
        let spec = EarlySpec::new(cut, 0, false).unwrap();
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
        }
        let frame = spec.frame(123);
        assert_eq!(decode_frame(spec, &frame).unwrap(), 123);
        for index in (0..20).chain(24..32) {
            let mut changed = frame;
            changed[index] ^= 0x80;
            assert!(decode_frame(spec, &changed).is_err());
        }
        for tid in [0, u32::MAX, i32::MAX as u32 + 1] {
            assert!(decode_frame(spec, &spec.frame(tid)).is_err());
        }
        for size in [0, 1, 31, 33, 256] {
            assert!(decode_frame(spec, &vec![0; size]).is_err());
        }
        assert!(!EarlySpec::new(cut, 0, true)
            .unwrap()
            .terminal((libc::CLD_EXITED, 0)));
        for fault in 1..3 {
            let permitted = (cut, fault) == (1, 1) || (cut, fault) == (0, 2);
            assert_eq!(EarlySpec::new(cut, fault, false).is_ok(), permitted);
            assert!(EarlySpec::new(cut, fault, true).is_err());
        }
    }
    for (cut, fault, status) in [(1, 1, 83), (0, 2, 84)] {
        let control = EarlySpec::new(cut, fault, false).unwrap();
        for value in 0..=255 {
            assert_eq!(control.terminal((libc::CLD_EXITED, value)), value == status);
            assert!(!control.terminal((libc::CLD_KILLED, value)));
        }
        assert!(!EarlySpec::new(cut, 0, false)
            .unwrap()
            .terminal((libc::CLD_EXITED, status)));
    }
    for (cut, fault) in [(5, 0), (255, 0), (0, 3), (0, 255)] {
        assert!(EarlySpec::new(cut, fault, false).is_err());
    }
    let mut seed = 0x0740_c071_u64;
    for _ in 0..256 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let spec = EarlySpec::new(seed as u8 % 5, 0, false).unwrap();
        let mut bytes = spec.frame(123);
        bytes[seed as usize % 16] ^= (seed as u8).max(1);
        assert!(decode_frame(spec, &bytes).is_err());
    }
    println!("constructor_frame seed=0x0740c071 mutations=256 exact_terminal_oracle=true controls_never_positive=true");
}
#[test]
fn signer_constructor_loss_seeded_fresh_and_independent_observers() {
    let mut seed = 0x0740_c071_u64;
    for _ in 0..8 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        constructor_loss_case(seed as u8 % 5, 0, false).unwrap();
    }
    let threads: Vec<_> = (1..5)
        .map(|cut| {
            std::thread::spawn(move || {
                constructor_loss_case(cut, 0, false).unwrap();
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    println!("constructor_concurrency seed=0x0740c071 fresh=8 independent=4 ordinary_subreaper_unchanged=true");
}
