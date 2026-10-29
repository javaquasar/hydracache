use super::*;
use std::process::{Child, Command, Stdio};

const TOKEN: [u8; TOKEN_BYTES] = [74; TOKEN_BYTES];
fn pair() -> [File; 2] {
    let mut fds = [-1; 2];
    assert_eq!(
        unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
                0,
                fds.as_mut_ptr(),
            )
        },
        0
    );
    unsafe { [File::from_raw_fd(fds[0]), File::from_raw_fd(fds[1])] }
}
fn pipe() -> [File; 2] {
    let mut fds = [-1; 2];
    assert_eq!(
        unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_NONBLOCK | libc::O_CLOEXEC) },
        0
    );
    unsafe { [File::from_raw_fd(fds[0]), File::from_raw_fd(fds[1])] }
}
fn readable(fd: RawFd, timeout: i32) -> i16 {
    let mut p = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    assert!(unsafe { libc::poll(&mut p, 1, timeout) } >= 0);
    p.revents
}
fn receive(fd: RawFd) -> u8 {
    assert_ne!(readable(fd, 5000) & libc::POLLIN, 0);
    let mut byte = 0u8;
    assert_eq!(
        unsafe { libc::recv(fd, (&mut byte as *mut u8).cast(), 1, libc::MSG_TRUNC) },
        1
    );
    byte
}
fn send(fd: RawFd, bytes: &[u8]) {
    assert_eq!(
        unsafe { libc::send(fd, bytes.as_ptr().cast(), bytes.len(), libc::MSG_NOSIGNAL) },
        bytes.len() as isize
    );
}
fn pidfd(pid: libc::pid_t) -> File {
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0u32) };
    assert!(fd >= 0);
    unsafe { File::from_raw_fd(fd as RawFd) }
}
fn kill(fd: &File) {
    let result = unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            fd.as_raw_fd(),
            libc::SIGKILL,
            0usize,
            0u32,
        )
    };
    assert!(result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH));
}
struct Sleeper {
    child: Child,
    pin: File,
}
impl Sleeper {
    fn new() -> Self {
        let child = Command::new("/bin/sleep")
            .arg("60")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let pin = pidfd(child.id() as _);
        Self { child, pin }
    }
    fn stop(&mut self) {
        kill(&self.pin);
        self.child.wait().unwrap();
    }
}
impl Drop for Sleeper {
    fn drop(&mut self) {
        self.stop();
    }
}
struct Held {
    pid: libc::pid_t,
    pin: File,
    release: ReleaseAttempt,
    output: File,
    owner: Sleeper,
    guardian: Sleeper,
    deadline: u64,
    status: Option<i32>,
}
impl Held {
    fn new(expire: bool, stop_before_ack: bool) -> Self {
        assert_ne!(
            unsafe { libc::geteuid() },
            0,
            "owned nonroot kernel fixture only"
        );
        // These helpers are ONLY liveness pins, not an armed real watchdog.
        let owner = Sleeper::new();
        let guardian = Sleeper::new();
        let [sender, receiver] = pair();
        let [reader, writer] = pipe();
        let deadline = now().unwrap() + if expire { 200_000_000 } else { 10 * NS };
        let timer = alarm(deadline).unwrap();
        let raw = [
            receiver.as_raw_fd(),
            owner.pin.as_raw_fd(),
            guardian.pin.as_raw_fd(),
            timer.as_raw_fd(),
            writer.as_raw_fd(),
        ];
        let mut keep = raw;
        keep.sort_unstable();
        assert!(keep.windows(2).all(|w| w[0] < w[1]));
        let blocked = u64::MAX;
        let mut previous = 0u64;
        assert_eq!(
            unsafe {
                libc::syscall(
                    libc::SYS_rt_sigprocmask,
                    libc::SIG_SETMASK,
                    &blocked,
                    &mut previous,
                    8usize,
                )
            },
            0
        );
        let pid = unsafe { libc::syscall(libc::SYS_fork) };
        if pid == 0 {
            // Raw child path: no allocations, std IO, panic, unwinding or runtime.
            let mut begin = 0u32;
            for fd in keep {
                if begin < fd as u32
                    && unsafe { libc::syscall(libc::SYS_close_range, begin, fd as u32 - 1, 0u32) }
                        != 0
                {
                    unsafe { libc::_exit(80) };
                }
                begin = fd as u32 + 1;
            }
            if unsafe { libc::syscall(libc::SYS_close_range, begin, u32::MAX, 0u32) } != 0 {
                unsafe { libc::_exit(81) };
            }
            let ready = b'R';
            if unsafe { libc::send(raw[0], (&ready as *const u8).cast(), 1, libc::MSG_NOSIGNAL) }
                != 1
            {
                unsafe { libc::_exit(82) };
            }
            let code = wait_packet(raw[0], raw[1], raw[2], raw[3], &TOKEN);
            if code != 0 {
                unsafe { libc::_exit(code) };
            }
            // Constant nonproduct effect, no executable or evidence artifact.
            if unsafe { libc::write(raw[4], b"effect".as_ptr().cast(), 6) } != 6 {
                unsafe { libc::_exit(83) };
            }
            if stop_before_ack {
                let pid = unsafe { libc::getpid() };
                unsafe { libc::kill(pid, libc::SIGSTOP) };
                unsafe { libc::_exit(84) };
            }
            let ack = b'A';
            if unsafe { libc::send(raw[0], (&ack as *const u8).cast(), 1, libc::MSG_NOSIGNAL) } != 1
            {
                unsafe { libc::_exit(85) };
            }
            unsafe { libc::_exit(0) };
        }
        let fork_errno = std::io::Error::last_os_error();
        assert_eq!(
            unsafe {
                libc::syscall(
                    libc::SYS_rt_sigprocmask,
                    libc::SIG_SETMASK,
                    &previous,
                    0usize,
                    8usize,
                )
            },
            0
        );
        assert!(pid > 0, "fork failed: {fork_errno}");
        let mut current = 0u64;
        assert_eq!(
            unsafe {
                libc::syscall(
                    libc::SYS_rt_sigprocmask,
                    libc::SIG_SETMASK,
                    0usize,
                    &mut current,
                    8usize,
                )
            },
            0
        );
        assert_eq!(current, previous);
        drop(receiver);
        drop(writer);
        drop(timer);
        let pin = pidfd(pid as _);
        let held = Self {
            pid: pid as _,
            pin,
            release: ReleaseAttempt {
                socket: sender,
                token: TOKEN,
                consumed: false,
            },
            output: reader,
            owner,
            guardian,
            deadline,
            status: None,
        };
        assert_eq!(receive(held.release.socket.as_raw_fd()), b'R');
        assert_eq!(
            readable(held.output.as_raw_fd(), 0),
            0,
            "output before release"
        );
        held
    }
    fn reap(&mut self) -> i32 {
        if let Some(s) = self.status {
            return s;
        }
        assert_ne!(
            readable(self.pin.as_raw_fd(), 5000),
            0,
            "child did not terminate"
        );
        let mut status = 0;
        assert_eq!(unsafe { libc::waitpid(self.pid, &mut status, 0) }, self.pid);
        self.status = Some(status);
        status
    }
    fn empty(&mut self) {
        let status = self.reap();
        assert!(libc::WIFEXITED(status) && libc::WEXITSTATUS(status) != 0);
        let mut bytes = [0u8; 8];
        assert_eq!(
            unsafe {
                libc::read(
                    self.output.as_raw_fd(),
                    bytes.as_mut_ptr().cast(),
                    bytes.len(),
                )
            },
            0
        );
    }
    fn effect(&self) {
        assert_ne!(readable(self.output.as_raw_fd(), 5000) & libc::POLLIN, 0);
        let mut bytes = [0u8; 6];
        assert_eq!(
            unsafe {
                libc::read(
                    self.output.as_raw_fd(),
                    bytes.as_mut_ptr().cast(),
                    bytes.len(),
                )
            },
            6
        );
        assert_eq!(&bytes, b"effect");
    }
}
impl Drop for Held {
    fn drop(&mut self) {
        if self.status.is_none() {
            kill(&self.pin);
            self.reap();
        }
    }
}

#[test]
fn two_phase_barrier_packet_flags_reject_every_bit_except_cloexec_policy() {
    for flags in [0, libc::MSG_CMSG_CLOEXEC] {
        assert!(packet_matches(32, flags, &TOKEN, &TOKEN));
        for length in [-1, 0, 31, 33] {
            assert!(!packet_matches(length, flags, &TOKEN, &TOKEN));
        }
        assert!(!packet_matches(32, flags, &[75; TOKEN_BYTES], &TOKEN));
    }
    for bit in 0..32 {
        let flags = 1i32.wrapping_shl(bit);
        if flags != libc::MSG_CMSG_CLOEXEC {
            assert!(!packet_matches(32, flags, &TOKEN, &TOKEN));
            assert!(!packet_matches(
                32,
                flags | libc::MSG_CMSG_CLOEXEC,
                &TOKEN,
                &TOKEN
            ));
        }
    }
    let [sender, receiver] = pair();
    send(sender.as_raw_fd(), &TOKEN);
    let mut bytes = [0u8; TOKEN_BYTES];
    let mut iov = libc::iovec {
        iov_base: bytes.as_mut_ptr().cast(),
        iov_len: bytes.len(),
    };
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    let n = unsafe {
        libc::recvmsg(
            receiver.as_raw_fd(),
            &mut msg,
            libc::MSG_DONTWAIT | libc::MSG_TRUNC | libc::MSG_CMSG_CLOEXEC,
        )
    };
    assert_eq!(msg.msg_flags, libc::MSG_CMSG_CLOEXEC);
    assert!(packet_matches(n, msg.msg_flags, &bytes, &TOKEN));
}
#[test]
fn two_phase_barrier_queue_pressure_cannot_rearm_failed_send() {
    let [socket, peer] = pair();
    let mut full = false;
    for _ in 0..4096 {
        let n = unsafe {
            libc::send(
                socket.as_raw_fd(),
                TOKEN.as_ptr().cast(),
                TOKEN.len(),
                libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT,
            )
        };
        if n < 0 {
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::EAGAIN)
            );
            full = true;
            break;
        }
        assert_eq!(n, TOKEN.len() as isize);
    }
    assert!(full);
    let mut attempt = ReleaseAttempt {
        socket,
        token: TOKEN,
        consumed: false,
    };
    assert!(attempt.send_once().is_err());
    let mut bytes = [0u8; TOKEN_BYTES];
    assert_eq!(
        unsafe { libc::recv(peer.as_raw_fd(), bytes.as_mut_ptr().cast(), bytes.len(), 0) },
        TOKEN_BYTES as isize
    );
    assert!(attempt.send_once().is_err());
}
#[test]
fn two_phase_barrier_send_is_sticky_after_success_failure_and_lost_ack() {
    for close_peer in [false, true] {
        let [socket, peer] = pair();
        let mut release = ReleaseAttempt {
            socket,
            token: TOKEN,
            consumed: false,
        };
        let peer = if close_peer {
            drop(peer);
            None
        } else {
            Some(peer)
        };
        assert_eq!(release.send_once().is_ok(), !close_peer);
        assert!(release.consumed);
        // Drop ACK route without inspecting a reply. No result enables resend.
        drop(peer);
        assert!(release.send_once().is_err());
        assert!(release.send_once().is_err());
    }
}
#[test]
fn two_phase_barrier_absolute_alarm_refuses_past_and_out_of_range() {
    assert!(alarm(0).is_err());
    assert!(alarm(now().unwrap()).is_err());
    assert!(alarm(u64::MAX).is_err());
    let deadline = now().unwrap() + 10 * NS;
    let timer = alarm(deadline).unwrap();
    let mut value: libc::itimerspec = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe { libc::timerfd_gettime(timer.as_raw_fd(), &mut value) },
        0
    );
    assert_eq!(value.it_interval.tv_sec, 0);
    assert_eq!(value.it_interval.tv_nsec, 0);
    assert_eq!(readable(timer.as_raw_fd(), 0), 0);
}

#[test]
#[ignore = "owned nonroot fork/pidfd fault matrix; no product or systemd"]
fn two_phase_barrier_owned_kernel_fault_matrix() {
    let cases = [
        "healthy",
        "guard-wins-over-queued-release",
        "owner-death",
        "guardian-death",
        "expiry",
        "socket-loss",
        "short",
        "long",
        "wrong",
        "rights",
        "sigkill-held",
        "sigkill-after-effect-before-ack",
    ];
    for case in cases {
        let mut held = Held::new(case == "expiry", case == "sigkill-after-effect-before-ack");
        let original_deadline = held.deadline;
        match case {
            "healthy" => {
                held.release.send_once().unwrap();
                assert_eq!(receive(held.release.socket.as_raw_fd()), b'A');
                held.effect();
                assert_eq!(held.reap(), 0);
                assert!(held.release.send_once().is_err());
            }
            "owner-death" => {
                held.owner.stop();
                held.empty();
            }
            "guard-wins-over-queued-release" => {
                assert_eq!(
                    unsafe {
                        libc::syscall(
                            libc::SYS_pidfd_send_signal,
                            held.pin.as_raw_fd(),
                            libc::SIGSTOP,
                            0usize,
                            0u32,
                        )
                    },
                    0
                );
                let mut status = 0;
                assert_eq!(
                    unsafe { libc::waitpid(held.pid, &mut status, libc::WUNTRACED) },
                    held.pid
                );
                assert!(libc::WIFSTOPPED(status));
                held.owner.stop();
                held.release.send_once().unwrap();
                assert_eq!(
                    unsafe {
                        libc::syscall(
                            libc::SYS_pidfd_send_signal,
                            held.pin.as_raw_fd(),
                            libc::SIGCONT,
                            0usize,
                            0u32,
                        )
                    },
                    0
                );
                held.empty();
                assert!(held.release.send_once().is_err());
            }
            "guardian-death" => {
                held.guardian.stop();
                held.empty();
            }
            "expiry" => {
                held.empty();
                assert!(now().unwrap() >= original_deadline);
            }
            "socket-loss" => {
                assert_eq!(
                    unsafe { libc::shutdown(held.release.socket.as_raw_fd(), libc::SHUT_RDWR) },
                    0
                );
                held.empty();
            }
            "short" => {
                send(held.release.socket.as_raw_fd(), &TOKEN[..31]);
                held.empty();
            }
            "long" => {
                send(held.release.socket.as_raw_fd(), &[74; 33]);
                held.empty();
            }
            "wrong" => {
                send(held.release.socket.as_raw_fd(), &[75; 32]);
                held.empty();
            }
            "rights" => {
                let right = File::open("/dev/null").unwrap();
                let mut iov = libc::iovec {
                    iov_base: TOKEN.as_ptr() as *mut _,
                    iov_len: TOKEN.len(),
                };
                let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
                let mut control = [0usize; 8];
                msg.msg_iov = &mut iov;
                msg.msg_iovlen = 1;
                msg.msg_control = control.as_mut_ptr().cast();
                msg.msg_controllen =
                    unsafe { libc::CMSG_SPACE(std::mem::size_of::<RawFd>() as _) } as _;
                unsafe {
                    let header = libc::CMSG_FIRSTHDR(&msg);
                    (*header).cmsg_level = libc::SOL_SOCKET;
                    (*header).cmsg_type = libc::SCM_RIGHTS;
                    (*header).cmsg_len = libc::CMSG_LEN(std::mem::size_of::<RawFd>() as _) as _;
                    libc::CMSG_DATA(header)
                        .cast::<RawFd>()
                        .write(right.as_raw_fd());
                    assert_eq!(
                        libc::sendmsg(held.release.socket.as_raw_fd(), &msg, libc::MSG_NOSIGNAL),
                        TOKEN.len() as isize
                    );
                }
                held.empty();
            }
            "sigkill-held" => {
                kill(&held.pin);
                let status = held.reap();
                assert!(libc::WIFSIGNALED(status) && libc::WTERMSIG(status) == libc::SIGKILL);
                let mut byte = 0u8;
                assert_eq!(
                    unsafe {
                        libc::read(held.output.as_raw_fd(), (&mut byte as *mut u8).cast(), 1)
                    },
                    0
                );
            }
            "sigkill-after-effect-before-ack" => {
                held.release.send_once().unwrap();
                held.effect();
                // The child stops itself before ACK; kill its exact pidfd.
                let mut status = 0;
                assert_eq!(
                    unsafe { libc::waitpid(held.pid, &mut status, libc::WUNTRACED) },
                    held.pid
                );
                assert!(libc::WIFSTOPPED(status));
                kill(&held.pin);
                let status = held.reap();
                assert!(libc::WIFSIGNALED(status) && libc::WTERMSIG(status) == libc::SIGKILL);
                let mut ack = 0u8;
                assert_eq!(
                    unsafe {
                        libc::recv(
                            held.release.socket.as_raw_fd(),
                            (&mut ack as *mut u8).cast(),
                            1,
                            0,
                        )
                    },
                    0
                );
                assert!(held.release.send_once().is_err());
            }
            _ => unreachable!(),
        }
        assert_eq!(held.deadline, original_deadline);
        println!("two_phase_barrier case={case} deadline_unchanged=true product=false original_authority=false all_cgroup_revocation=false");
    }
}
