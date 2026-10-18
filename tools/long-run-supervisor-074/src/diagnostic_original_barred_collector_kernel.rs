//! Barred original I/O quarantine only; no stream IO, activation or completion.
use super::*;
use std::os::fd::FromRawFd;

fn errno() -> i32 {
    unsafe { *libc::__errno_location() }
}
fn park() -> ! {
    loop {
        unsafe { libc::poll(std::ptr::null_mut(), 0, -1) };
    }
}
fn notice(fd: RawFd, byte: u8) {
    unsafe { libc::send(fd, (&byte as *const u8).cast(), 1, libc::MSG_NOSIGNAL) };
}
// Raw clone continuation: stack and bounded syscalls only. Never invoke Rust
// IO, validation, allocation, logging, unwinding, atfork hooks or destructors.
fn quarantine(io: [RawFd; 4], peer: RawFd, reason: u8) -> ! {
    let mut closed = true;
    for fd in io {
        // Linux releases the descriptor even on EINTR. Never retry close against
        // a possibly reused number; this child allocates no new descriptors.
        if unsafe { libc::close(fd) } != 0 {
            closed = false;
        }
    }
    notice(peer, if closed { reason } else { b'F' });
    // Neither a delivered notice nor successful close releases the fence.
    park();
}
fn child(keep: [RawFd; 7], alarm: RawFd, peer: RawFd, io: [RawFd; 4]) -> ! {
    let mut first = 0u32;
    for fd in keep {
        if first < fd as u32
            && unsafe { libc::syscall(libc::SYS_close_range, first, fd as u32 - 1, 0u32) } != 0
        {
            quarantine(io, peer, b'F');
        }
        first = fd as u32 + 1;
    }
    if unsafe { libc::syscall(libc::SYS_close_range, first, u32::MAX, 0u32) } != 0 {
        quarantine(io, peer, b'F');
    }
    notice(peer, b'B');
    let mut poll = [
        libc::pollfd {
            fd: alarm,
            events: libc::POLLIN,
            revents: 0,
        },
        libc::pollfd {
            fd: peer,
            events: libc::POLLIN,
            revents: 0,
        },
    ];
    loop {
        let n = unsafe { libc::poll(poll.as_mut_ptr(), 2, -1) };
        if n < 0 {
            if errno() == libc::EINTR {
                continue;
            }
            quarantine(io, peer, b'F');
        }
        if poll[1].revents != 0 {
            quarantine(io, peer, b'L');
        }
        if poll[0].revents != 0 {
            quarantine(
                io,
                peer,
                if poll[0].revents == libc::POLLIN {
                    b'E'
                } else {
                    b'F'
                },
            );
        }
    }
}
pub(crate) struct KernelCollector {
    pub(super) pid: libc::pid_t,
    pub(super) pinned: File,
    pub(super) peer: File,
    #[cfg(test)]
    pub(super) child_peer: RawFd,
}
impl KernelCollector {
    pub(super) fn spawn(
        original: [RawFd; 6],
        #[cfg(test)] fault: u8,
    ) -> Result<Self, BarredCustodyError> {
        let mut fds = [-1; 2];
        if unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
                0,
                fds.as_mut_ptr(),
            )
        } != 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        let peer = unsafe { File::from_raw_fd(fds[0]) };
        let send = unsafe { File::from_raw_fd(fds[1]) };
        let mut keep = [
            original[0],
            original[1],
            original[2],
            original[3],
            original[4],
            original[5],
            send.as_raw_fd(),
        ];
        keep.sort_unstable();
        if keep[0] < 0 || keep.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(BarredCustodyError::Child);
        }
        let mut pinned = -1;
        let arguments = clone_arguments(&mut pinned);
        let blocked = u64::MAX;
        let mut previous = 0u64;
        if unsafe {
            libc::syscall(
                libc::SYS_rt_sigprocmask,
                libc::SIG_SETMASK,
                &blocked,
                &mut previous,
                8usize,
            )
        } != 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        let pid = {
            #[cfg(test)]
            if fault == 1 {
                unsafe { *libc::__errno_location() = libc::EAGAIN };
                -1
            } else {
                unsafe {
                    libc::syscall(
                        libc::SYS_clone3,
                        arguments.as_ptr(),
                        std::mem::size_of_val(&arguments),
                    )
                }
            }
            #[cfg(not(test))]
            unsafe {
                libc::syscall(
                    libc::SYS_clone3,
                    arguments.as_ptr(),
                    std::mem::size_of_val(&arguments),
                )
            }
        };
        if pid == 0 {
            child(
                keep,
                original[1],
                send.as_raw_fd(),
                [original[2], original[3], original[4], original[5]],
            );
        }
        let creation_error = (pid < 0).then(std::io::Error::last_os_error);
        // CLONE_PIDFD creates the exact owned pidfd atomically, with no PID-only
        // pinning window. Own cleanup BEFORE signal restoration can fail.
        let owned = (pid > 0).then(|| Self {
            pid: pid as libc::pid_t,
            pinned: unsafe { File::from_raw_fd(pinned) },
            peer,
            #[cfg(test)]
            child_peer: send.as_raw_fd(),
        });
        let restored = unsafe {
            libc::syscall(
                libc::SYS_rt_sigprocmask,
                libc::SIG_SETMASK,
                &previous,
                0usize,
                8usize,
            )
        };
        if restored != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        #[cfg(test)]
        if fault == 2 {
            // Parent-only error propagation after REAL restoration; never
            // leave the host thread's inherited handler mask changed.
            return Err(std::io::Error::from_raw_os_error(libc::EIO).into());
        }
        if let Some(error) = creation_error {
            return Err(error.into());
        }
        let owned = owned.ok_or(BarredCustodyError::Child)?;
        if unsafe { libc::fcntl(owned.pinned.as_raw_fd(), libc::F_GETFD) } != libc::FD_CLOEXEC {
            return Err(BarredCustodyError::Child);
        }
        Ok(owned)
    }
    pub(super) fn alive(&self) -> Result<(), BarredCustodyError> {
        let mut poll = libc::pollfd {
            fd: self.pinned.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        if unsafe { libc::poll(&mut poll, 1, 0) } != 0 || poll.revents != 0 {
            return Err(BarredCustodyError::Child);
        }
        Ok(())
    }
    pub(super) fn observe(&self, alarm: RawFd, initial: bool) -> Result<(), BarredCustodyError> {
        loop {
            let mut poll = [
                libc::pollfd {
                    fd: self.pinned.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                },
                libc::pollfd {
                    fd: alarm,
                    events: libc::POLLIN,
                    revents: 0,
                },
                libc::pollfd {
                    fd: self.peer.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                },
            ];
            let n = unsafe { libc::poll(poll.as_mut_ptr(), 3, if initial { -1 } else { 0 }) };
            if n < 0 {
                if errno() == libc::EINTR {
                    continue;
                }
                return Err(std::io::Error::last_os_error().into());
            }
            if poll[0].revents != 0 {
                return Err(BarredCustodyError::Child);
            }
            if poll[1].revents != 0 {
                return Err(BarredCustodyError::Expired);
            }
            if poll[2].revents != 0 {
                let mut byte = 0u8;
                let n = unsafe {
                    libc::recv(
                        self.peer.as_raw_fd(),
                        (&mut byte as *mut u8).cast(),
                        1,
                        libc::MSG_TRUNC,
                    )
                };
                if n == 1 && initial && byte == b'B' {
                    return Ok(());
                }
                return Err(if n == 1 && byte == b'E' {
                    BarredCustodyError::Expired
                } else {
                    BarredCustodyError::Child
                });
            }
            if !initial {
                return Ok(());
            }
        }
    }
    pub(super) fn stop(&mut self) -> std::io::Result<()> {
        if self.pid == 0 {
            return Ok(());
        }
        loop {
            if unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    self.pinned.as_raw_fd(),
                    libc::SIGKILL,
                    0usize,
                    0u32,
                )
            } == 0
                || errno() == libc::ESRCH
            {
                break;
            }
            if errno() != libc::EINTR {
                return Err(std::io::Error::last_os_error());
            }
        }
        loop {
            let mut status = 0;
            let n = unsafe { libc::waitpid(self.pid, &mut status, 0) };
            if n == self.pid {
                self.pid = 0;
                if libc::WIFSIGNALED(status) && libc::WTERMSIG(status) == libc::SIGKILL {
                    return Ok(());
                }
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "unexpected barred custodian wait status",
                ));
            }
            if n < 0 && errno() == libc::EINTR {
                continue;
            }
            return Err(std::io::Error::last_os_error());
        }
    }
}
impl Drop for KernelCollector {
    fn drop(&mut self) {
        // Cancellation only, while the production owner's original borrow is
        // still live. Drop is not a successful writer-cessation/completion API.
        let _ = self.stop();
    }
}
