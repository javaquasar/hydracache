//! Test-only observation of the unchanged private Owner, not a new launch mode.
use super::*;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Child, Command, ExitStatus, Stdio};

const ROLE: &str = "HC074_OWNED_LOSS_ROLE";
const CASE: &str = "HC074_OWNED_LOSS_CASE";
const ENTRY: &str = "owned::tests::loss::owned_probe_parent_loss_role";
const CASES: [&str; 4] = [
    "initial-loss",
    "exec-loss",
    "initial-healthy",
    "exec-healthy",
];

fn command(role: &str, case: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", ENTRY, "--nocapture", "--test-threads=1"])
        .env_clear()
        .env(ROLE, role)
        .env(CASE, case)
        .stdin(Stdio::null());
    let parent = unsafe { libc::getpid() };
    // SAFETY: fixed test role's post-fork child only; losing its observer must
    // not leave an acknowledgement-gated owner running indefinitely.
    unsafe {
        command.pre_exec(move || {
            if libc::prctl(
                libc::PR_SET_PDEATHSIG,
                libc::SIGKILL as usize,
                0usize,
                0usize,
                0usize,
            ) != 0
            {
                return Err(io::Error::last_os_error());
            }
            if libc::getppid() != parent {
                return Err(io::Error::from_raw_os_error(libc::ESRCH));
            }
            Ok(())
        });
    }
    command
}
fn spec() -> (bool, bool) {
    let case = std::env::var(CASE).unwrap();
    assert!(CASES.contains(&case.as_str()));
    (case.starts_with("exec-"), case.ends_with("-healthy"))
}
fn frame(pid: u32, tid: u32, first: bool, healthy: bool) -> [u8; 24] {
    let mut frame = [0; 24];
    frame[..8].copy_from_slice(b"HC074L01");
    frame[8..12].copy_from_slice(&pid.to_le_bytes());
    frame[12..16].copy_from_slice(&tid.to_le_bytes());
    frame[16] = u8::from(first);
    frame[17] = u8::from(healthy);
    frame
}
fn decode(raw: &[u8], first: bool, healthy: bool) -> Option<(u32, u32)> {
    if raw.len() != 24 {
        return None;
    }
    let pid = u32::from_le_bytes(raw[8..12].try_into().ok()?);
    let tid = u32::from_le_bytes(raw[12..16].try_into().ok()?);
    if [pid, tid]
        .iter()
        .any(|id| *id == 0 || *id > i32::MAX as u32)
        || raw != frame(pid, tid, first, healthy)
    {
        return None;
    }
    Some((pid, tid))
}
fn original(pid: u32) -> OwnedFd {
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0u32) };
    assert!(fd >= 0, "pidfd_open: {}", io::Error::last_os_error());
    unsafe { OwnedFd::from_raw_fd(fd as i32) }
}
fn signal(fd: &OwnedFd, signal: i32) {
    assert_eq!(
        unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                fd.as_raw_fd(),
                signal,
                std::ptr::null::<libc::siginfo_t>(),
                0u32,
            )
        },
        0,
        "pidfd signal: {}",
        io::Error::last_os_error()
    );
}
fn ready(fd: &OwnedFd) {
    let mut poll = libc::pollfd {
        fd: fd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    assert_eq!(unsafe { libc::poll(&mut poll, 1, 5000) }, 1);
    assert_eq!(
        poll.revents & (libc::POLLIN | libc::POLLERR | libc::POLLNVAL),
        libc::POLLIN
    );
}
struct Process {
    child: Child,
    fd: Option<OwnedFd>,
    finished: bool,
}
impl Process {
    fn spawn(command: &mut Command) -> Self {
        let mut own = Self {
            child: command.spawn().unwrap(),
            fd: None,
            finished: false,
        };
        own.fd = Some(original(own.child.id()));
        own
    }
    fn fd(&self) -> &OwnedFd {
        self.fd.as_ref().unwrap()
    }
    fn wait(&mut self) -> ExitStatus {
        let until = Instant::now() + LIMIT;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                self.finished = true;
                return status;
            }
            assert!(Instant::now() < until, "process wait timeout");
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        if !self.finished {
            // Failure-only cleanup of this exact original, never wait-any or group kill.
            if let Some(fd) = &self.fd {
                unsafe {
                    libc::syscall(
                        libc::SYS_pidfd_send_signal,
                        fd.as_raw_fd(),
                        libc::SIGKILL,
                        std::ptr::null::<libc::siginfo_t>(),
                        0u32,
                    );
                }
            } else {
                let _ = self.child.kill();
            }
            let until = Instant::now() + LIMIT;
            while self.child.try_wait().ok().flatten().is_none() && Instant::now() < until {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}
fn subreaper() -> i32 {
    let mut setting = -1i32;
    assert_eq!(
        unsafe {
            libc::prctl(
                libc::PR_GET_CHILD_SUBREAPER,
                &mut setting,
                0usize,
                0usize,
                0usize,
            )
        },
        0
    );
    setting
}
fn text(path: impl AsRef<std::path::Path>) -> String {
    let mut bytes = Vec::new();
    File::open(path)
        .unwrap()
        .take(65537)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= 65536);
    String::from_utf8(bytes).unwrap()
}
fn scalar(raw: &str, name: &str) -> u32 {
    let mut values = raw.lines().filter_map(|line| line.strip_prefix(name));
    let value = values.next().unwrap().trim().parse().unwrap();
    assert!(values.next().is_none());
    value
}
fn wait_info(fd: &OwnedFd, flags: i32) -> io::Result<(i32, i32, u32)> {
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    let result = unsafe {
        libc::waitid(
            libc::P_PIDFD,
            fd.as_raw_fd() as u32,
            &mut info,
            libc::WEXITED | libc::WNOHANG | flags,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((info.si_code, unsafe { info.si_status() }, unsafe {
        info.si_pid() as u32
    }))
}
struct Orphan {
    fd: OwnedFd,
    done: bool,
}
impl Drop for Orphan {
    fn drop(&mut self) {
        if !self.done {
            // Only failure cleanup may signal the monitored child. Passing proof never does.
            unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    self.fd.as_raw_fd(),
                    libc::SIGKILL,
                    std::ptr::null::<libc::siginfo_t>(),
                    0u32,
                );
            }
            let until = Instant::now() + LIMIT;
            while Instant::now() < until {
                match wait_info(&self.fd, 0) {
                    Ok((_, _, 0)) => std::thread::sleep(Duration::from_millis(1)),
                    _ => break,
                }
            }
        }
    }
}
fn owner(first: bool, healthy: bool) {
    // Monitor owns the temporary directory, including if this owner is killed.
    let original = unsafe { std::os::fd::BorrowedFd::borrow_raw(4) };
    let file = File::from(original.try_clone_to_owned().unwrap());
    let mut own = Owner::create(file, Case::Native).unwrap();
    if first {
        own.first().unwrap();
    }
    // Test-only channel in the owner, closed by unchanged child close_range before stop.
    let borrowed = unsafe { std::os::fd::BorrowedFd::borrow_raw(3) };
    let mut report = File::from(borrowed.try_clone_to_owned().unwrap());
    let tid = unsafe { libc::syscall(libc::SYS_gettid) } as u32;
    report
        .write_all(&frame(own.pid as u32, tid, first, healthy))
        .unwrap();
    // A private acknowledgement gate, not asynchronous self-SIGSTOP: kill()
    // returning does not establish that the calling thread has stopped.
    let mut grant = [0u8; 1];
    report.read_exact(&mut grant).unwrap();
    assert_eq!(grant, [1]);
    assert!(healthy, "loss owner must never be resumed");
    drop(own);
}
fn monitor(first: bool, healthy: bool) {
    assert_eq!(
        unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1usize, 0usize, 0usize, 0usize) },
        0
    );
    assert_eq!(subreaper(), 1);
    let (_temp, file) = fixture();
    let capture = tempfile::tempfile().unwrap();
    let (mut reader, writer) = UnixStream::pair().unwrap();
    reader.set_read_timeout(Some(LIMIT)).unwrap();
    // Keep sources out of fixed child destinations before Command's pre_exec.
    let duplicate = |fd: i32| {
        let raw = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 10) };
        assert!(raw >= 10);
        unsafe { OwnedFd::from_raw_fd(raw) }
    };
    let report_source = duplicate(writer.as_raw_fd());
    let image_source = duplicate(file.as_raw_fd());
    let raw = report_source.as_raw_fd();
    let image = image_source.as_raw_fd();
    let case = std::env::var(CASE).unwrap();
    let mut cmd = command("owner", &case);
    cmd.stdout(capture.try_clone().unwrap())
        .stderr(capture.try_clone().unwrap());
    // SAFETY: fresh test owner child only, fixed private report FD, no parent mutation.
    unsafe {
        cmd.pre_exec(move || {
            if libc::dup2(raw, 3) < 0
                || libc::dup2(image, 4) < 0
                || libc::fcntl(3, libc::F_SETFD, 0) != 0
                || libc::fcntl(4, libc::F_SETFD, 0) != 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut owner = Process::spawn(&mut cmd);
    drop(writer);
    drop(report_source);
    drop(image_source);
    let mut raw = [0u8; 24];
    reader.read_exact(&mut raw).unwrap();
    assert!(!dead(owner.fd()));
    let (child, tid) = decode(&raw, first, healthy).unwrap();
    let owner_pid = owner.child.id();
    assert_eq!(
        text(format!("/proc/{owner_pid}/task/{tid}/children")).trim(),
        child.to_string()
    );
    assert_eq!(
        scalar(
            &text(format!("/proc/{owner_pid}/task/{tid}/status")),
            "Tgid:"
        ),
        owner_pid
    );
    let mut orphan = Orphan {
        fd: original(child),
        done: false,
    };
    let info = text(format!("/proc/self/fdinfo/{}", orphan.fd.as_raw_fd()));
    assert_eq!(scalar(&info, "Pid:"), child);
    let status = text(format!("/proc/{child}/status"));
    assert_eq!(scalar(&status, "PPid:"), owner_pid);
    assert_eq!(scalar(&status, "TracerPid:"), tid);
    assert_eq!(scalar(&status, "Tgid:"), child);
    assert!(
        !dead(&orphan.fd),
        "unexpected original pidfd readiness before owner kill: {info}\n{status}\nwait_info={:?}",
        wait_info(&orphan.fd, libc::WNOWAIT)
    );
    let mut dummy_cmd = Command::new("/bin/cat");
    dummy_cmd
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut dummy = Process::spawn(&mut dummy_cmd);
    assert!(!dead(dummy.fd()));
    if healthy {
        reader.write_all(&[1]).unwrap();
        assert_eq!(owner.wait().code(), Some(0));
        ready(&orphan.fd);
    } else {
        signal(owner.fd(), libc::SIGKILL);
        assert_eq!(owner.wait().signal(), Some(libc::SIGKILL));
        ready(&orphan.fd);
        let expected = (libc::CLD_KILLED, libc::SIGKILL, child);
        assert_eq!(wait_info(&orphan.fd, libc::WNOWAIT).unwrap(), expected);
        assert_eq!(wait_info(&orphan.fd, 0).unwrap(), expected);
    }
    assert_eq!(
        wait_info(&orphan.fd, 0).unwrap_err().raw_os_error(),
        Some(libc::ECHILD)
    );
    orphan.done = true;
    assert!(!dead(dummy.fd()), "unrelated original child was affected");
    signal(dummy.fd(), libc::SIGKILL);
    assert_eq!(dummy.wait().signal(), Some(libc::SIGKILL));
    println!("\nowner_loss first_exec={first} healthy={healthy} original_pidfd_dead=true monitor_ECHILD=true monitor_child_signaled=false unrelated_alive=true admission=false");
}

#[test]
fn owned_probe_parent_loss_role() {
    match std::env::var(ROLE).ok().as_deref() {
        None => {}
        Some("owner") => {
            let (first, healthy) = spec();
            owner(first, healthy);
        }
        Some("monitor") => {
            let (first, healthy) = spec();
            monitor(first, healthy);
        }
        _ => panic!("unknown private owner loss role"),
    }
}
#[test]
fn owned_probe_parent_loss_matrix() {
    let before = subreaper();
    for case in CASES {
        let capture = tempfile::tempfile().unwrap();
        let mut cmd = command("monitor", case);
        cmd.stdout(capture.try_clone().unwrap())
            .stderr(capture.try_clone().unwrap());
        let mut monitor = Process::spawn(&mut cmd);
        let result = monitor.wait();
        let mut raw = [0u8; 16385];
        let n = capture.read_at(&mut raw, 0).unwrap();
        assert!(n <= 16384);
        let output = std::str::from_utf8(&raw[..n]).unwrap();
        assert!(result.success(), "{case}: {output}");
        let lines: Vec<_> = output
            .lines()
            .filter(|line| line.starts_with("owner_loss "))
            .collect();
        let first = case.starts_with("exec-");
        let healthy = case.ends_with("-healthy");
        assert_eq!(lines, [format!("owner_loss first_exec={first} healthy={healthy} original_pidfd_dead=true monitor_ECHILD=true monitor_child_signaled=false unrelated_alive=true admission=false")]);
        for line in output
            .lines()
            .filter(|line| line.starts_with("owner_loss "))
        {
            println!("{line}");
        }
    }
    assert_eq!(before, subreaper());
    println!("owner_loss_matrix=4 actual_owner_SIGKILL=2 healthy_drop_controls=2 parent_subreaper_unchanged=true full_normal_entry_loss_proved=false");
}
#[test]
fn owned_probe_loss_report_mutations_refuse_without_pid_adoption() {
    for first in [false, true] {
        for healthy in [false, true] {
            let good = frame(123, 456, first, healthy);
            assert_eq!(decode(&good, first, healthy), Some((123, 456)));
            assert!(decode(&good[..23], first, healthy).is_none());
            assert!(decode(&[good.as_slice(), &[0]].concat(), first, healthy).is_none());
            for index in (0..8).chain(16..24) {
                for bit in 0..8 {
                    let mut raw = good;
                    raw[index] ^= 1 << bit;
                    assert!(decode(&raw, first, healthy).is_none());
                }
            }
            for pid in [0, i32::MAX as u32 + 1, u32::MAX] {
                assert!(decode(&frame(pid, 456, first, healthy), first, healthy).is_none());
                assert!(decode(&frame(123, pid, first, healthy), first, healthy).is_none());
            }
        }
    }
    // Mutations in the identity fields can remain structurally valid, but must
    // still pass the separate live original-parent/tracer/sole-child checks.
    let mut seed = 0x0740c07au32;
    for _ in 0..128 {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        assert_eq!(
            decode(&frame(seed, 456, false, false), false, false).is_some(),
            seed != 0 && seed <= i32::MAX as u32
        );
    }
    println!("owner_loss_report_fixed_bit_mutations=512 seeded_identity_shapes=128 seed=0x0740c07a report_is_not_authority=true");
}
