use super::*;
use fs2::FileExt;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn pidfd(pid: u32) -> File {
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0u32) };
    assert!(fd >= 0);
    unsafe { File::from_raw_fd(fd as RawFd) }
}
fn signal(fd: &File, signal: i32) {
    assert_eq!(
        unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                fd.as_raw_fd(),
                signal,
                0usize,
                0u32,
            )
        },
        0
    );
}
fn sleeping() -> Child {
    Command::new("/bin/sleep")
        .arg("60")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}
struct Fixture {
    root: tempfile::TempDir,
    controller: Child,
    controller_fd: File,
    writer: Child,
    writer_fd: File,
    guardian: Guardian,
    guardian_fd: File,
    collector_fd: Option<File>,
    collector_pid: u32,
    hidden: Option<File>,
    deadline: u64,
}
impl Fixture {
    fn new(notify: bool, hidden: bool, fault: u8) -> Self {
        assert_ne!(
            unsafe { libc::geteuid() },
            0,
            "nonroot synthetic mechanics only"
        );
        let root = tempfile::Builder::new()
            .prefix("hc074-autonomous-")
            .tempdir_in("/tmp")
            .unwrap();
        let fence_path = root.path().join("fence");
        let fence = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(&fence_path)
            .unwrap();
        fence.try_lock_exclusive().unwrap();
        let sinks = ["stdout", "stderr"].map(|name| {
            OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(root.path().join(name))
                .unwrap()
        });
        let [r0, mut w0] = pipe().unwrap();
        let [r1, w1] = pipe().unwrap();
        // No shell/product executable: only the constant sleep helper receives
        // pipe writers. The fixture parent writes exact binary oracle bytes.
        w0.write_all(b"original\0stdout\n").unwrap();
        let mut stderr = &w1;
        stderr.write_all(b"original\xffstderr\n").unwrap();
        let hidden = hidden.then(|| w0.try_clone().unwrap());
        let controller = sleeping();
        let controller_fd = pidfd(controller.id());
        let writer = Command::new("/bin/sleep")
            .arg("60")
            .stdin(Stdio::null())
            .stdout(Stdio::from(w0))
            .stderr(Stdio::from(w1))
            .spawn()
            .unwrap();
        let writer_fd = pidfd(writer.id());
        let input = Inputs {
            fence,
            readers: [r0, r1],
            sinks,
            controller: controller_fd.try_clone().unwrap(),
            writers: vec![writer_fd.try_clone().unwrap()],
            deadline: now().unwrap() + crate::diagnostic_lease::CONTROLLER_LOSS_SECONDS * NS,
        };
        // Use the exact absolute input, not the later guardian creation time.
        let original_deadline = input.deadline;
        future(original_deadline, now().unwrap()).unwrap();
        let guardian = if fault == 0 {
            spawn(input, notify)
        } else {
            spawn_inner(input, notify, fault)
        }
        .unwrap();
        let guardian_fd = pidfd(guardian.pid as u32);
        Self {
            root,
            controller,
            controller_fd,
            writer,
            writer_fd,
            guardian,
            guardian_fd,
            collector_fd: None,
            collector_pid: 0,
            hidden,
            deadline: original_deadline,
        }
    }
    fn busy(&self) {
        let other = OpenOptions::new()
            .read(true)
            .write(true)
            .open(self.root.path().join("fence"))
            .unwrap();
        assert_eq!(
            other.try_lock_exclusive().unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
    }
    fn pin_collector(&mut self) {
        let limit = Instant::now() + Duration::from_secs(5);
        loop {
            let raw =
                std::fs::read_to_string(format!("/proc/{0}/task/{0}/children", self.guardian.pid))
                    .unwrap();
            let children: Vec<u32> = raw.split_whitespace().map(|p| p.parse().unwrap()).collect();
            if children.len() == 1 {
                self.collector_pid = children[0];
                self.collector_fd = Some(pidfd(children[0]));
                return;
            }
            assert!(children.is_empty() && Instant::now() < limit);
            std::thread::yield_now();
        }
    }
    fn ready(&mut self) {
        let mut p = libc::pollfd {
            fd: self.guardian.ready.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        assert_eq!(unsafe { libc::poll(&mut p, 1, 5000) }, 1);
        assert_eq!(self.guardian.notice().unwrap(), 1);
        self.busy();
        for entry in std::fs::read_dir(format!("/proc/{}/fd", self.guardian.pid)).unwrap() {
            if let Ok(target) = std::fs::read_link(entry.unwrap().path()) {
                assert_ne!(target, self.root.path().join("stdout"));
                assert_ne!(target, self.root.path().join("stderr"));
            }
        }
    }
    fn ended(&mut self, seconds: i32) {
        let mut p = libc::pollfd {
            fd: self.guardian_fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        assert_eq!(unsafe { libc::poll(&mut p, 1, seconds * 1000) }, 1);
        let mut status = 0;
        assert_eq!(
            unsafe { libc::waitpid(self.guardian.pid, &mut status, 0) },
            self.guardian.pid
        );
        self.guardian.pid = 0;
        assert!(libc::WIFEXITED(status));
        assert_eq!(libc::WEXITSTATUS(status), 0);
        assert_eq!(terminal(self.writer_fd.as_raw_fd()), 1);
        let other = OpenOptions::new()
            .read(true)
            .write(true)
            .open(self.root.path().join("fence"))
            .unwrap();
        other.try_lock_exclusive().unwrap();
        assert_eq!(
            std::fs::read(self.root.path().join("stdout")).unwrap(),
            b"original\0stdout\n"
        );
        assert_eq!(
            std::fs::read(self.root.path().join("stderr")).unwrap(),
            b"original\xffstderr\n"
        );
    }
    fn held(&self) {
        // Scheduling margin only, never an admission threshold or lease change.
        std::thread::sleep(Duration::from_millis(120));
        assert_eq!(terminal(self.guardian_fd.as_raw_fd()), 0);
        self.busy();
    }
    fn failure(&mut self, expected: Failure) {
        let mut p = libc::pollfd {
            fd: self.guardian.ready.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let wait = if matches!(expected, Failure::Expired) {
            12000
        } else {
            5000
        };
        assert_eq!(unsafe { libc::poll(&mut p, 1, wait) }, 1);
        assert_eq!(self.guardian.notice().unwrap(), expected as u8);
        self.held();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // Only exact children/pidfds owned by this fixture, including failures.
        kill_pinned(self.writer_fd.as_raw_fd());
        kill_pinned(self.controller_fd.as_raw_fd());
        let _ = self.writer.wait();
        let _ = self.controller.wait();
        self.hidden = None;
        // The guardian's direct child is discovered only through this pinned,
        // unreaped owned guardian, never a process-name or global PID search.
        if self.collector_fd.is_none() && self.guardian.pid != 0 {
            if let Ok(raw) =
                std::fs::read_to_string(format!("/proc/{0}/task/{0}/children", self.guardian.pid))
            {
                if let Some(pid) = raw
                    .split_whitespace()
                    .next()
                    .and_then(|v| v.parse::<u32>().ok())
                {
                    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0u32) };
                    if fd >= 0 {
                        self.collector_fd = Some(unsafe { File::from_raw_fd(fd as RawFd) });
                    }
                }
            }
        }
        if let Some(fd) = &self.collector_fd {
            kill_pinned(fd.as_raw_fd());
            let mut p = libc::pollfd {
                fd: fd.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            unsafe { libc::poll(&mut p, 1, 5000) };
        }
        if self.guardian.pid != 0 {
            kill_pinned(self.guardian_fd.as_raw_fd());
            let mut status = 0;
            unsafe { libc::waitpid(self.guardian.pid, &mut status, 0) };
        }
    }
}

#[test]
fn autonomous_release_requires_both_eof_and_every_terminal_without_refusal() {
    for terminal in [false, true] {
        for a in [false, true] {
            for b in [false, true] {
                for refused in [false, true] {
                    assert_eq!(
                        releasable(terminal, [a, b], refused),
                        terminal && a && b && !refused
                    );
                }
            }
        }
    }
}
#[test]
fn autonomous_absolute_deadline_rejects_equality_and_past() {
    for origin in [0, 1, NS, u32::MAX as u64, i64::MAX as u64 - 1] {
        assert!(future(origin, origin).is_err());
        assert!(future(origin, origin.saturating_add(1)).is_err());
        if origin > 0 {
            assert!(future(origin, origin - 1).is_ok());
        }
    }
    assert!(future(u64::MAX, 1).is_err());
}

#[test]
fn autonomous_notice_rejects_unknown_oversized_and_disconnected_packets() {
    for data in [
        vec![1],
        vec![9],
        vec![10],
        vec![0],
        vec![11],
        vec![255],
        vec![1, 2],
    ] {
        let [ready, send] = notices().unwrap();
        let guardian = Guardian { pid: 0, ready };
        assert_eq!(
            unsafe {
                libc::send(
                    send.as_raw_fd(),
                    data.as_ptr().cast(),
                    data.len(),
                    libc::MSG_NOSIGNAL,
                )
            },
            data.len() as isize
        );
        assert_eq!(
            guardian.notice().is_ok(),
            data.len() == 1 && (1..=10).contains(&data[0])
        );
        drop(send);
        assert!(guardian.notice().is_err());
    }
}

#[test]
fn autonomous_descriptor_roles_and_bounded_writer_inventory_refuse_before_fork() {
    for case in [
        "valid",
        "empty",
        "too-many",
        "reader-write",
        "same-sink",
        "sink-read",
        "not-pidfd",
        "not-fence",
        "expired",
    ] {
        let root = tempfile::tempdir().unwrap();
        let [r0, w0] = pipe().unwrap();
        let [r1, _w1] = pipe().unwrap();
        let regular = |name| {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(root.path().join(name))
                .unwrap()
        };
        let mut input = Inputs {
            fence: regular("lock"),
            readers: [r0, r1],
            sinks: [regular("stdout"), regular("stderr")],
            controller: pidfd(std::process::id()),
            writers: vec![pidfd(std::process::id())],
            deadline: now().unwrap() + 10 * NS,
        };
        match case {
            "empty" => input.writers.clear(),
            "too-many" => {
                input.writers = (0..=MAX_WRITERS)
                    .map(|_| pidfd(std::process::id()))
                    .collect()
            }
            "reader-write" => input.readers[0] = w0.try_clone().unwrap(),
            "same-sink" => input.sinks[1] = input.sinks[0].try_clone().unwrap(),
            "sink-read" => input.sinks[0] = File::open(root.path().join("stdout")).unwrap(),
            "not-pidfd" => input.controller = File::open(root.path().join("lock")).unwrap(),
            "not-fence" => input.fence = w0.try_clone().unwrap(),
            "expired" => input.deadline = now().unwrap(),
            _ => (),
        }
        assert_eq!(validate(&input).is_ok(), case == "valid", "{case}");
    }
}

#[test]
#[ignore = "manual nonroot isolated kernel fault mechanics, no product workload"]
fn autonomous_writer_fence_matrix() {
    let seed = 0x0740_7a17u64;
    for repeat in 1..=3 {
        for case in [
            "normal",
            "controller-kill",
            "controller-stop",
            "hidden-writer",
            "stopped-writer",
            "ack-omitted",
            "ack-disconnected",
            "write-failure",
            "sync-failure",
            "size-limit",
            "overflow",
            "collector-stop",
        ] {
            let start = Instant::now();
            let fault = match case {
                "write-failure" => 1,
                "sync-failure" => 2,
                "ack-disconnected" => 3,
                "size-limit" => 4,
                "collector-stop" => 5,
                _ => 0,
            };
            let mut f = Fixture::new(
                case != "ack-omitted",
                case == "hidden-writer" || case == "overflow",
                fault,
            );
            f.pin_collector();
            if case == "ack-disconnected" {
                // The receiver was shut down before fork, so no ACK delivers.
                f.held();
            } else if case != "ack-omitted" {
                f.ready();
            }
            match case {
                "write-failure" | "size-limit" => {
                    f.failure(Failure::Write);
                    f.writer.wait().unwrap();
                    f.held();
                    assert_eq!(terminal(f.controller_fd.as_raw_fd()), 1);
                    assert!(std::fs::read(f.root.path().join("stdout"))
                        .unwrap()
                        .is_empty());
                }
                "sync-failure" => {
                    signal(&f.writer_fd, libc::SIGTERM);
                    f.failure(Failure::Sync);
                    f.writer.wait().unwrap();
                    f.held();
                    assert_eq!(
                        std::fs::read(f.root.path().join("stdout")).unwrap(),
                        b"original\0stdout\n"
                    );
                    assert_eq!(terminal(f.controller_fd.as_raw_fd()), 1);
                }
                "overflow" => {
                    let data = [b'x'; QUANTUM];
                    let deadline = Instant::now() + Duration::from_secs(5);
                    let mut total = 0;
                    while total <= crate::diagnostic_receipts::STREAM_BYTES {
                        assert!(Instant::now() < deadline);
                        match f.hidden.as_mut().unwrap().write(&data) {
                            Ok(n) => total += n,
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                                std::thread::yield_now()
                            }
                            other => panic!("overflow writer: {other:?}"),
                        }
                    }
                    f.writer.wait().unwrap();
                    f.held();
                    assert!(
                        std::fs::metadata(f.root.path().join("stdout"))
                            .unwrap()
                            .len()
                            <= crate::diagnostic_receipts::STREAM_BYTES as u64
                    );
                    f.failure(Failure::Overflow);
                    f.hidden = None;
                    f.held(); // Refusal is sticky even after the final alias closes.
                }
                "normal" => {
                    signal(&f.writer_fd, libc::SIGTERM);
                    f.ended(5);
                }
                "controller-stop" | "collector-stop" => {
                    if case == "controller-stop" {
                        signal(&f.controller_fd, libc::SIGSTOP);
                    }
                    f.held();
                    if case == "collector-stop" {
                        let raw =
                            std::fs::read_to_string(format!("/proc/{}/stat", f.collector_pid))
                                .unwrap();
                        assert!(raw.rsplit_once(") ").unwrap().1.starts_with("T "));
                    }
                    f.failure(Failure::Expired);
                    assert!(now().unwrap() >= f.deadline);
                    let status = f.controller.wait().unwrap();
                    use std::os::unix::process::ExitStatusExt;
                    assert_eq!(status.signal(), Some(libc::SIGKILL));
                    assert_eq!(f.writer.wait().unwrap().signal(), Some(libc::SIGKILL));
                    let mut p = libc::pollfd {
                        fd: f.collector_fd.as_ref().unwrap().as_raw_fd(),
                        events: libc::POLLIN,
                        revents: 0,
                    };
                    assert_eq!(unsafe { libc::poll(&mut p, 1, 5000) }, 1);
                    let raw =
                        std::fs::read_to_string(format!("/proc/{}/stat", f.collector_pid)).unwrap();
                    let status: i32 = raw.split_whitespace().last().unwrap().parse().unwrap();
                    assert!(libc::WIFSIGNALED(status));
                    assert_eq!(libc::WTERMSIG(status), libc::SIGKILL);
                }
                "hidden-writer" => {
                    signal(&f.controller_fd, libc::SIGKILL);
                    f.writer.wait().unwrap();
                    f.held();
                    f.hidden = None;
                    f.ended(5);
                }
                "stopped-writer" => {
                    signal(&f.writer_fd, libc::SIGSTOP);
                    f.held();
                    signal(&f.controller_fd, libc::SIGKILL);
                    f.ended(5);
                    use std::os::unix::process::ExitStatusExt;
                    assert_eq!(f.writer.wait().unwrap().signal(), Some(libc::SIGKILL));
                }
                _ => {
                    signal(&f.controller_fd, libc::SIGKILL);
                    f.ended(5);
                }
            }
            println!("HC074-AUTONOMOUS case={case} repeat={repeat} seed={seed:#x} elapsed_ms={} completed=true", start.elapsed().as_millis());
        }
    }
    println!("HC074-AUTONOMOUS cases=36 repeats=3 completed=true");
}
