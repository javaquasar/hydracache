//! Autonomous pending custody only. No collector, authenticated send or start.
use super::*;
use std::os::fd::FromRawFd;

#[derive(Debug, thiserror::Error)]
pub(crate) enum BarredCustodyError {
    #[error("original validation refused: {0}")]
    Original(#[from] OriginalCustodyError),
    #[error("pending custodian syscall failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("pending custodian is terminal or its notice differs")]
    Child,
    #[error("original custodian alarm expired")]
    Expired,
    #[error("pending custody is already refused")]
    Refused,
}
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
fn child(keep: [RawFd; 3], alarm: RawFd, peer: RawFd) -> ! {
    let mut first = 0u32;
    for fd in keep {
        if first < fd as u32
            && unsafe { libc::syscall(libc::SYS_close_range, first, fd as u32 - 1, 0u32) } != 0
        {
            notice(peer, b'F');
            park();
        }
        first = fd as u32 + 1;
    }
    if unsafe { libc::syscall(libc::SYS_close_range, first, u32::MAX, 0u32) } != 0 {
        notice(peer, b'F');
        park();
    }
    notice(peer, b'P');
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
            notice(peer, b'F');
            park();
        }
        if poll[1].revents != 0 {
            notice(peer, b'L');
            park();
        }
        if poll[0].revents != 0 {
            notice(
                peer,
                if poll[0].revents == libc::POLLIN {
                    b'E'
                } else {
                    b'F'
                },
            );
            // Sticky refusal: disable readiness polling, never consume/rearm
            // the original timer or release custody after its absolute expiry.
            poll[0].fd = -1;
        }
    }
}
fn clone_arguments(pidfd: &mut RawFd) -> [u64; 11] {
    let mut arguments = [0u64; 11];
    arguments[0] = libc::CLONE_PIDFD as u64;
    arguments[1] = (pidfd as *mut RawFd) as u64;
    arguments[4] = libc::SIGCHLD as u64;
    arguments
}
pub(crate) struct KernelCustodian {
    pid: libc::pid_t,
    pinned: File,
    peer: File,
    #[cfg(test)]
    child_peer: RawFd,
}
impl KernelCustodian {
    fn spawn(original: [RawFd; 2], #[cfg(test)] fault: u8) -> Result<Self, BarredCustodyError> {
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
        let mut keep = [original[0], original[1], send.as_raw_fd()];
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
            child(keep, original[1], send.as_raw_fd());
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
    fn alive(&self) -> Result<(), BarredCustodyError> {
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
    fn observe(&self, alarm: RawFd, initial: bool) -> Result<(), BarredCustodyError> {
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
                if n == 1 && initial && byte == b'P' {
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
    fn stop(&mut self) -> std::io::Result<()> {
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
impl Drop for KernelCustodian {
    fn drop(&mut self) {
        // Cancellation only, while the production owner's original borrow is
        // still live. Drop is not a successful writer-cessation/completion API.
        let _ = self.stop();
    }
}

/// Pending custody cannot detach, start, collect or send in an ordinary build.
pub(crate) struct OriginalBarredCustody<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    // Stop/reap the pending child BEFORE ending the exclusive original borrow.
    kernel: Option<KernelCustodian>,
    original:
        &'held mut OriginalCustodiedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    inventory: Option<Inventory>,
    allow_fixture: bool,
    ready: bool,
}
pub(crate) struct BarredCustodyFailure<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: BarredCustodyError,
    held: Box<OriginalBarredCustody<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl std::fmt::Debug for OriginalBarredCustody<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OriginalBarredCustody")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for BarredCustodyFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl OriginalBarredCustody<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn is_refused(&self) -> bool {
        self.original.is_refused()
    }
    fn validate(&mut self) -> Result<(), BarredCustodyError> {
        let result = (|| {
            if self.is_refused() {
                return Err(BarredCustodyError::Refused);
            }
            let child = self.kernel.as_ref().ok_or(BarredCustodyError::Child)?;
            child.alive()?;
            self.original.validate(self.allow_fixture)?;
            let inventory = self.inventory.as_ref().ok_or(BarredCustodyError::Child)?;
            if capture(self.original)? != *inventory {
                return Err(OriginalCustodyError::Binding.into());
            }
            self.original.inspect_deadline()?;
            child.observe(inventory.fds[1], false)?;
            self.original.inspect_deadline()?;
            Ok(())
        })();
        if result.is_err() {
            self.original.refuse();
        }
        result
    }
    pub(crate) fn revalidate(&mut self) -> Result<(), BarredCustodyError> {
        if !self.ready {
            self.original.refuse();
            return Err(BarredCustodyError::Refused);
        }
        self.validate()
    }
}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalCustodiedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn prepare_barred_original(
        &mut self,
    ) -> Result<
        OriginalBarredCustody<'_, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        BarredCustodyFailure<'_, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        self.prepare_barred_with(
            false,
            #[cfg(test)]
            0,
            #[cfg(test)]
            |_| {},
        )
    }
    fn prepare_barred_with<'held>(
        &'held mut self,
        allow_fixture: bool,
        #[cfg(test)] fault: u8,
        #[cfg(test)] checkpoint: impl FnOnce(
            &mut OriginalBarredCustody<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        ),
    ) -> Result<
        OriginalBarredCustody<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        BarredCustodyFailure<'held, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut held = OriginalBarredCustody {
            kernel: None,
            original: self,
            inventory: None,
            allow_fixture,
            ready: false,
        };
        let result = (|| {
            if held.is_refused() || !held.original.ready {
                return Err(BarredCustodyError::Refused);
            }
            held.inventory = Some(capture(held.original)?);
            held.original.inspect_deadline()?;
            let inventory = held.inventory.as_ref().ok_or(BarredCustodyError::Child)?;
            held.kernel = Some(KernelCustodian::spawn(
                [inventory.fds[0], inventory.fds[1]],
                #[cfg(test)]
                fault,
            )?);
            #[cfg(test)]
            if fault == 3 {
                // Lose the REAL first notice, not a reconstructed acknowledgement.
                held.kernel
                    .as_ref()
                    .unwrap()
                    .observe(inventory.fds[1], true)?;
            }
            held.kernel
                .as_ref()
                .ok_or(BarredCustodyError::Child)?
                .observe(inventory.fds[1], true)?;
            #[cfg(test)]
            checkpoint(&mut held);
            held.validate()?;
            held.ready = true;
            Ok(())
        })();
        match result {
            Ok(()) => Ok(held),
            Err(error) => {
                held.original.refuse();
                Err(BarredCustodyFailure {
                    error,
                    held: Box::new(held),
                })
            }
        }
    }
}

#[cfg(test)]
#[path = "diagnostic_original_barred_custody_test_api.rs"]
mod test_api;

#[path = "diagnostic_original_barred_collector.rs"]
mod barred_collector;
