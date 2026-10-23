//! One authenticated readiness ACK, then a nonexecuting original-bound hold.
use std::{
    io,
    os::fd::{FromRawFd, OwnedFd},
};
mod exchange_imports {
    pub(super) use hydracache_long_run_supervisor_074::diagnostic_kernel_admission::KernelError;
    pub(super) use hydracache_long_run_supervisor_074::protocol::MAX_PACKET_BYTES;
}
#[path = "diagnostic_held_child_exchange.rs"]
#[allow(dead_code)] // Shared wire leaf also contains the parent half and old test API.
pub(crate) mod exchange;
fn stopped() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "held readiness stopped without release",
    )
}
fn wait(deadline: u64, first: bool) -> io::Result<()> {
    let left = deadline
        .checked_sub(exchange::clock()?)
        .filter(|n| *n > 0)
        .ok_or_else(stopped)?;
    let timeout = libc::timespec {
        tv_sec: (left / 1_000_000_000).try_into().map_err(|_| stopped())?,
        tv_nsec: (left % 1_000_000_000) as _,
    };
    let mut fds = [6, 7, 8, 9, 17].map(|fd| libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    });
    let result =
        unsafe { libc::ppoll(fds.as_mut_ptr(), fds.len() as _, &timeout, std::ptr::null()) };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }
    if first
        && result == 1
        && fds[..4].iter().all(|p| p.revents == 0)
        && fds[4].revents == libc::POLLIN
    {
        return Ok(());
    }
    // Any guardian/creator/alarm/channel event or expiry ends readiness. No retry,
    // release-channel write, worker exec, deadline renewal, completion or fallback.
    Err(stopped())
}
pub(crate) fn run(invocation: [u8; 16]) -> io::Result<()> {
    let observed = crate::reader::roles::installed::held::prepare()?;
    let expected = observed.expected(invocation)?;
    wait(expected.deadline, true)?;
    // Only the fixed slot validated above is adopted, exactly once.
    let fd = unsafe { OwnedFd::from_raw_fd(17) };
    let mut child = exchange::Child::from_owned(fd)?;
    child.answer_framed_once(&expected, || observed.revalidate())?;
    wait(expected.deadline, false)
}
