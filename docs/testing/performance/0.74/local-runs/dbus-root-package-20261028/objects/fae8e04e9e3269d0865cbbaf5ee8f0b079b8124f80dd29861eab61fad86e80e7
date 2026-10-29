//! Private held-barrier mechanics. Live pidfds are NOT watchdog-arming proof.
//! No executable, cgroup, evidence sink, fence or original-owner capability is
//! accepted or returned. Production integration and enrollment remain closed.
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};

const TOKEN_BYTES: usize = 32;
const NS: u64 = 1_000_000_000;

fn refused() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "held barrier refused")
}
fn now() -> io::Result<u64> {
    let mut time: libc::timespec = unsafe { std::mem::zeroed() };
    if unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut time) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if time.tv_sec < 0 || !(0..NS as i64).contains(&time.tv_nsec) {
        return Err(refused());
    }
    (time.tv_sec as u64)
        .checked_mul(NS)
        .and_then(|v| v.checked_add(time.tv_nsec as u64))
        .ok_or_else(refused)
}
fn alarm(deadline: u64) -> io::Result<File> {
    if deadline <= now()? || deadline > i64::MAX as u64 {
        return Err(refused());
    }
    let fd = unsafe {
        libc::timerfd_create(libc::CLOCK_BOOTTIME, libc::TFD_CLOEXEC | libc::TFD_NONBLOCK)
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let file = unsafe { File::from_raw_fd(fd) };
    let spec = libc::itimerspec {
        it_interval: libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        it_value: libc::timespec {
            tv_sec: (deadline / NS) as libc::time_t,
            tv_nsec: (deadline % NS) as libc::c_long,
        },
    };
    if unsafe { libc::timerfd_settime(fd, libc::TFD_TIMER_ABSTIME, &spec, std::ptr::null_mut()) }
        != 0
    {
        return Err(io::Error::last_os_error());
    }
    if deadline <= now()? {
        return Err(refused());
    }
    Ok(file)
}

// Correlation and at-most-once send mechanics only, not a release permit. The
// future original adapter must validate the complete frame BEFORE calling it.
struct ReleaseAttempt {
    socket: File,
    token: [u8; TOKEN_BYTES],
    consumed: bool,
}
impl ReleaseAttempt {
    fn send_once(&mut self) -> io::Result<()> {
        if self.consumed {
            return Err(refused());
        }
        // Publish uncertainty before even a potentially failed syscall. No
        // observation, ACK, timeout or peer loss can reset this latch.
        self.consumed = true;
        let sent = unsafe {
            libc::send(
                self.socket.as_raw_fd(),
                self.token.as_ptr().cast(),
                TOKEN_BYTES,
                libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT,
            )
        };
        if sent != TOKEN_BYTES as isize {
            return Err(refused());
        }
        Ok(())
    }
}

// Raw post-fork-safe path: stack and kernel calls only, no inherited runtime,
// allocator, filesystem opens, callbacks or relative timeout reconstruction.
// 0 means only "one exact packet observed while three kernel guards were live".
// It proves neither actual watchdog readiness nor authenticated original start.
fn packet_matches(
    received: isize,
    flags: i32,
    bytes: &[u8; TOKEN_BYTES],
    token: &[u8; TOKEN_BYTES],
) -> bool {
    // Linux reports the requested CLOEXEC policy in output msg_flags, even
    // without ancillary data. No truncation, rights or other flag is accepted.
    received == TOKEN_BYTES as isize && flags & !libc::MSG_CMSG_CLOEXEC == 0 && bytes == token
}
fn wait_packet(
    socket: RawFd,
    owner: RawFd,
    guardian: RawFd,
    timer: RawFd,
    token: &[u8; TOKEN_BYTES],
) -> i32 {
    let mut guards = [timer, owner, guardian].map(|fd| libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    });
    let mut poll = [
        guards[0],
        guards[1],
        guards[2],
        libc::pollfd {
            fd: socket,
            events: libc::POLLIN,
            revents: 0,
        },
    ];
    loop {
        let n = unsafe { libc::poll(poll.as_mut_ptr(), poll.len() as _, -1) };
        if n < 0 {
            if unsafe { *libc::__errno_location() } == libc::EINTR {
                continue;
            }
            return 10;
        }
        // Guard failure wins even if release became readable in the same poll.
        if poll[..3].iter().any(|p| p.revents != 0) {
            return 11;
        }
        if poll[3].revents != libc::POLLIN {
            return 12;
        }
        let mut bytes = [0u8; TOKEN_BYTES];
        let mut iov = libc::iovec {
            iov_base: bytes.as_mut_ptr().cast(),
            iov_len: bytes.len(),
        };
        let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
        msg.msg_iov = &mut iov;
        msg.msg_iovlen = 1;
        // No ancillary space: unexpected SCM_RIGHTS is truncated, rejected,
        // and the kernel closes any rights not delivered into a control buffer.
        let received = unsafe {
            libc::recvmsg(
                socket,
                &mut msg,
                libc::MSG_DONTWAIT | libc::MSG_TRUNC | libc::MSG_CMSG_CLOEXEC,
            )
        };
        if !packet_matches(received, msg.msg_flags, &bytes, token) {
            return 13;
        }
        // Reobserve the original timer and both pidfds after packet reception.
        if unsafe { libc::poll(guards.as_mut_ptr(), guards.len() as _, 0) } != 0 {
            return 14;
        }
        return 0;
    }
}

#[cfg(test)]
#[path = "diagnostic_held_barrier_tests.rs"]
mod tests;

#[path = "diagnostic_prearmed_watchdog.rs"]
mod prearmed;

pub(crate) use prearmed::inherited_pending::InheritedPending;
