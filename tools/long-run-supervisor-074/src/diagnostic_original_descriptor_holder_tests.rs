//! Test-only raw holder: no product, manager, collector or guardian admission.
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};

pub(super) struct Holder {
    pid: libc::pid_t,
    pidfd: Option<File>,
    control: File,
    ack: File,
}
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
fn readable(file: &File, timeout: i32) -> i32 {
    let mut poll = libc::pollfd {
        fd: file.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let result = unsafe { libc::poll(&mut poll, 1, timeout) };
    assert!(result >= 0);
    result
}
fn receive(file: &File) -> (isize, u8) {
    let mut byte = 0u8;
    let n = unsafe {
        libc::recv(
            file.as_raw_fd(),
            (&mut byte as *mut u8).cast(),
            1,
            libc::MSG_TRUNC,
        )
    };
    (n, byte)
}
fn send(file: &File, byte: u8) {
    assert_eq!(
        unsafe {
            libc::send(
                file.as_raw_fd(),
                (&byte as *const u8).cast(),
                1,
                libc::MSG_NOSIGNAL,
            )
        },
        1
    );
}
fn refused(control: RawFd, byte: u8) -> ! {
    unsafe { libc::send(control, (&byte as *const u8).cast(), 1, libc::MSG_NOSIGNAL) };
    loop {
        unsafe { libc::poll(std::ptr::null_mut(), 0, -1) };
    }
}
// Child continuation: stack data and raw bounded syscalls only, never Rust IO,
// validation, allocation, logging, unwinding or destructors after SYS_fork.
fn child(keep: [RawFd; 8], fds: [RawFd; 10], control: RawFd, ack: RawFd, notify: bool) -> ! {
    let mut first = 0u32;
    for fd in keep {
        if first < fd as u32
            && unsafe { libc::syscall(libc::SYS_close_range, first, fd as u32 - 1, 0u32) } != 0
        {
            unsafe { libc::_exit(81) };
        }
        first = fd as u32 + 1;
    }
    if unsafe { libc::syscall(libc::SYS_close_range, first, u32::MAX, 0u32) } != 0 {
        unsafe { libc::_exit(82) };
    }
    for fd in &fds[6..10] {
        if unsafe { libc::fcntl(*fd, libc::F_GETFD) } != -1
            || unsafe { *libc::__errno_location() } != libc::EBADF
        {
            refused(control, b'W');
        }
    }
    let a = b'A';
    if notify {
        unsafe { libc::send(ack, (&a as *const u8).cast(), 1, libc::MSG_NOSIGNAL) };
    }
    let b = b'B';
    if unsafe { libc::send(control, (&b as *const u8).cast(), 1, libc::MSG_NOSIGNAL) } != 1 {
        refused(control, b'R');
    }
    let mut poll = libc::pollfd {
        fd: control,
        events: libc::POLLIN,
        revents: 0,
    };
    let mut byte = 0u8;
    if unsafe { libc::poll(&mut poll, 1, -1) } != 1
        || unsafe { libc::recv(control, (&mut byte as *mut u8).cast(), 1, libc::MSG_TRUNC) } != 1
        || byte != b'C'
    {
        refused(control, b'R');
    }
    let mut timer = libc::pollfd {
        fd: fds[1],
        events: libc::POLLIN,
        revents: 0,
    };
    if unsafe { libc::poll(&mut timer, 1, 0) } != 0 {
        refused(control, b'T');
    }
    for fd in &fds[2..4] {
        let n = unsafe { libc::read(*fd, (&mut byte as *mut u8).cast(), 1) };
        if n != 0 {
            if n < 0 && unsafe { *libc::__errno_location() } == libc::EAGAIN {
                refused(control, b'E');
            }
            refused(control, b'R');
        }
    }
    for fd in &fds[4..6] {
        if unsafe { libc::fsync(*fd) } != 0 || unsafe { libc::close(*fd) } != 0 {
            refused(control, b'R');
        }
    }
    let done = b'D';
    if unsafe { libc::send(control, (&done as *const u8).cast(), 1, libc::MSG_NOSIGNAL) } != 1 {
        refused(control, b'R');
    }
    if unsafe { libc::poll(&mut poll, 1, -1) } != 1
        || unsafe { libc::recv(control, (&mut byte as *mut u8).cast(), 1, libc::MSG_TRUNC) } != 1
        || byte != b'Q'
    {
        refused(control, b'R');
    }
    // No completion record: this test-only holder may exit after actual EOF.
    unsafe { libc::_exit(0) }
}
impl Holder {
    pub(super) fn spawn(resources: [RawFd; 6], fds: [RawFd; 10], notify: bool) -> Self {
        assert_ne!(unsafe { libc::geteuid() }, 0);
        let [control, child_control] = pair();
        let [ack, child_ack] = pair();
        let mut keep = [
            resources[0],
            resources[1],
            resources[2],
            resources[3],
            resources[4],
            resources[5],
            child_control.as_raw_fd(),
            child_ack.as_raw_fd(),
        ];
        keep.sort_unstable();
        assert!(keep
            .windows(2)
            .all(|pair| pair[0] >= 0 && pair[0] < pair[1]));
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
        let pid = unsafe { libc::syscall(libc::SYS_fork) } as libc::pid_t;
        if pid == 0 {
            child(
                keep,
                fds,
                child_control.as_raw_fd(),
                child_ack.as_raw_fd(),
                notify,
            );
        }
        let restored = unsafe {
            libc::syscall(
                libc::SYS_rt_sigprocmask,
                libc::SIG_SETMASK,
                &previous,
                0usize,
                8usize,
            )
        };
        assert!(pid > 0, "raw fork refused");
        // Establish cleanup ownership before any subsequent fallible check.
        let mut held = Self {
            pid,
            pidfd: None,
            control,
            ack,
        };
        assert_eq!(restored, 0);
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
        let pinned = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0u32) };
        assert!(pinned >= 0);
        held.pidfd = Some(unsafe { File::from_raw_fd(pinned as RawFd) });
        drop(child_control);
        drop(child_ack);
        assert_eq!(readable(&held.control, 5000), 1);
        assert_eq!(receive(&held.control), (1, b'B'));
        if notify {
            assert_eq!(receive(&held.ack), (1, b'A'));
        } else {
            assert_eq!(receive(&held.ack).0, -1);
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::EAGAIN)
            );
        }
        held.assert_retained();
        held
    }
    pub(super) fn assert_retained(&self) {
        assert_eq!(readable(self.pidfd.as_ref().unwrap(), 0), 0);
    }
    pub(super) fn observe_eof(&self, hidden: bool) -> bool {
        send(&self.control, b'C');
        assert_eq!(readable(&self.control, 5000), 1);
        assert_eq!(
            receive(&self.control),
            (1, if hidden { b'E' } else { b'D' })
        );
        !hidden
    }
    pub(super) fn finish(&mut self, killed: bool) {
        let pidfd = self.pidfd.as_ref().unwrap();
        if killed {
            assert_eq!(
                unsafe {
                    libc::syscall(
                        libc::SYS_pidfd_send_signal,
                        pidfd.as_raw_fd(),
                        libc::SIGKILL,
                        0usize,
                        0u32,
                    )
                },
                0
            );
        } else {
            send(&self.control, b'Q');
        }
        assert_eq!(readable(pidfd, 5000), 1);
        let mut status = 0;
        assert_eq!(unsafe { libc::waitpid(self.pid, &mut status, 0) }, self.pid);
        self.pid = 0;
        if killed {
            assert!(libc::WIFSIGNALED(status));
            assert_eq!(libc::WTERMSIG(status), libc::SIGKILL);
        } else {
            assert!(libc::WIFEXITED(status));
            assert_eq!(libc::WEXITSTATUS(status), 0);
        }
    }
}
impl Drop for Holder {
    fn drop(&mut self) {
        if self.pid == 0 {
            return;
        }
        if let Some(fd) = &self.pidfd {
            unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    fd.as_raw_fd(),
                    libc::SIGKILL,
                    0usize,
                    0u32,
                )
            };
        } else {
            unsafe { libc::kill(self.pid, libc::SIGKILL) };
        }
        unsafe { libc::waitpid(self.pid, std::ptr::null_mut(), 0) };
    }
}
