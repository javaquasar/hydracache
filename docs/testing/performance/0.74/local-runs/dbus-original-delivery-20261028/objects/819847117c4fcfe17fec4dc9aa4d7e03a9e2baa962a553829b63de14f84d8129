//! Private borrowed-FD mechanics only. No target enrollment or original permit.
use super::*;
use std::os::fd::BorrowedFd;

fn keep_roles(fds: [RawFd; 4]) -> io::Result<[RawFd; 4]> {
    let mut keep = fds;
    keep.sort_unstable();
    if keep[0] < 0 || keep.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(refused());
    }
    Ok(keep)
}
struct OwnedPending(Watchdog);
impl OwnedPending {
    fn cancel(&mut self) -> io::Result<()> {
        if self.0.pid == 0 {
            return Ok(());
        }
        loop {
            let result = unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    self.0.pinned.as_raw_fd(),
                    libc::SIGKILL,
                    0usize,
                    0u32,
                )
            };
            if result == 0 || unsafe { *libc::__errno_location() } == libc::ESRCH {
                break;
            }
            if unsafe { *libc::__errno_location() } != libc::EINTR {
                return Err(io::Error::last_os_error());
            }
        }
        loop {
            let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
            if unsafe {
                libc::waitid(
                    libc::P_PIDFD,
                    self.0.pinned.as_raw_fd() as _,
                    &mut info,
                    libc::WEXITED,
                )
            } == 0
            {
                self.0.pid = 0;
                return Ok(());
            }
            if unsafe { *libc::__errno_location() } != libc::EINTR {
                return Err(io::Error::last_os_error());
            }
        }
    }
}
impl Drop for OwnedPending {
    fn drop(&mut self) {
        // Pending has never received a target. Cancellation is NOT completion.
        // Original holders are retained by the caller throughout this cleanup.
        let _ = self.cancel();
    }
}
fn spawn_one(
    fence: BorrowedFd<'_>,
    alarm: BorrowedFd<'_>,
    #[cfg(test)] fault: u8,
) -> io::Result<OwnedPending> {
    let origin = Origin::current();
    let pinned = unsafe { libc::syscall(libc::SYS_pidfd_open, origin.pid, 0u32) };
    if pinned < 0 {
        return Err(io::Error::last_os_error());
    }
    let owner = unsafe { File::from_raw_fd(pinned as RawFd) };
    // Alias the existing timer OFD. Never create, consume, reset or rearm it.
    let timer = File::from(alarm.try_clone_to_owned()?);
    let [peer, send] = pair()?;
    let keep = keep_roles([
        fence.as_raw_fd(),
        timer.as_raw_fd(),
        owner.as_raw_fd(),
        send.as_raw_fd(),
    ])?;
    if terminal(timer.as_raw_fd()) || terminal(owner.as_raw_fd()) {
        return Err(refused());
    }
    let mut pinned = -1i32;
    let mut arguments = [0u64; 11];
    arguments[0] = libc::CLONE_PIDFD as u64;
    arguments[1] = (&mut pinned as *mut RawFd) as u64;
    arguments[4] = libc::SIGCHLD as u64;
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
        return Err(io::Error::last_os_error());
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
            &keep,
            send.as_raw_fd(),
            timer.as_raw_fd(),
            owner.as_raw_fd(),
            origin,
            false,
        );
    }
    let creation_error = (pid < 0).then(io::Error::last_os_error);
    // Own exact-child cancellation BEFORE restoring the parent's signal mask.
    let owned = (pid > 0).then(|| {
        OwnedPending(Watchdog {
            pid: pid as i32,
            pinned: unsafe { File::from_raw_fd(pinned) },
            peer,
            alarm: timer,
            origin,
            pending: false,
            consumed: false,
            armed: false,
            refused: false,
        })
    });
    if unsafe {
        libc::syscall(
            libc::SYS_rt_sigprocmask,
            libc::SIG_SETMASK,
            &previous,
            0usize,
            8usize,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    #[cfg(test)]
    if fault == 2 {
        // Inject only AFTER real restoration, never corrupt the test thread.
        return Err(io::Error::from_raw_os_error(libc::EIO));
    }
    if let Some(error) = creation_error {
        return Err(error);
    }
    let mut owned = owned.ok_or_else(refused)?;
    owned.0.pending()?;
    Ok(owned)
}

/// Kernel mechanics, not a constructor for an admitted original attempt.
pub(crate) struct InheritedPending {
    guardians: [OwnedPending; 2],
    origin: Origin,
}
impl InheritedPending {
    pub(crate) fn spawn(
        fence: BorrowedFd<'_>,
        alarm: BorrowedFd<'_>,
        #[cfg(test)] fault: u8,
    ) -> io::Result<Self> {
        let origin = Origin::current();
        let first = spawn_one(
            fence,
            alarm,
            #[cfg(test)]
            match fault {
                1 => 1,
                3 => 2,
                _ => 0,
            },
        )?;
        let second = spawn_one(
            fence,
            alarm,
            #[cfg(test)]
            match fault {
                2 => 1,
                4 => 2,
                _ => 0,
            },
        )?;
        let result = Self {
            guardians: [first, second],
            origin,
        };
        result.observe()?;
        Ok(result)
    }
    pub(crate) fn observe(&self) -> io::Result<()> {
        if Origin::current() != self.origin
            || self.guardians.iter().any(|owned| {
                let child = &owned.0;
                child.origin != self.origin
                    || !child.pending
                    || child.consumed
                    || child.armed
                    || child.refused
                    || terminal(child.alarm.as_raw_fd())
                    || terminal(child.pinned.as_raw_fd())
                    || terminal(child.peer.as_raw_fd())
            })
        {
            return Err(refused());
        }
        // Nonterminal pidfds do NOT prove responsiveness or writer cessation.
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn kill_for_test(&mut self, index: usize) {
        self.guardians[index].cancel().unwrap();
    }
    #[cfg(test)]
    pub(crate) fn pids_for_test(&self) -> [i32; 2] {
        std::array::from_fn(|i| self.guardians[i].0.pid)
    }
}

#[cfg(test)]
#[path = "diagnostic_inherited_pending_watchdog_tests.rs"]
mod tests;
