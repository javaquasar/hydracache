//! The original creator's one-shot permission to start work after exec.
use std::{
    io,
    os::fd::{FromRawFd, OwnedFd},
};

mod exchange_imports {
    // Match the existing ancillary reader's payload budget.
    pub(super) const MAX_PACKET_BYTES: usize = 65_536;
    #[derive(Debug)]
    pub(super) enum KernelError {
        Invalid,
    }
    impl From<std::io::Error> for KernelError {
        fn from(_: std::io::Error) -> Self {
            Self::Invalid
        }
    }
}

#[path = "../../../../long-run-supervisor-074/src/diagnostic_held_child_exchange.rs"]
#[allow(dead_code)] // Compile the original peer verification leaf for this binary.
mod exchange;

pub(super) use exchange::WORKER_RUN_ARG;

pub(super) struct Hold<'a> {
    pub deadline: &'a str,
    pub peer: &'a str,
    pub ack: &'a str,
}

fn refused() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "held worker run refused")
}

fn number<T: std::str::FromStr + ToString>(value: &str) -> io::Result<T> {
    value
        .parse::<T>()
        .ok()
        .filter(|n| n.to_string() == value)
        .ok_or_else(refused)
}

impl Hold<'_> {
    pub(super) fn wait(self) -> io::Result<()> {
        let deadline: u64 = number(self.deadline)?;
        let peer = self
            .peer
            .split(':')
            .map(number::<u32>)
            .collect::<io::Result<Vec<_>>>()?;
        let peer: [u32; 3] = peer.try_into().map_err(|_| refused())?;
        if !(2..=i32::MAX as u32).contains(&peer[0])
            || self.ack.len() != 96
            || !self.ack.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || unsafe {
                libc::geteuid() == 0 || libc::prctl(libc::PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0) != 1
            }
        {
            return Err(refused());
        }
        let mut ack = [0; 48];
        for (i, byte) in ack.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&self.ack[2 * i..2 * i + 2], 16).map_err(|_| refused())?;
        }
        if &ack[..8] != b"HCRA0741" || ack[8..16] != 48u64.to_le_bytes() {
            return Err(refused());
        }
        let guards = [6, 7, 8, 9];
        for fd in [5, 6, 7, 8, 9, 17] {
            if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        // Ownership of these exact bootstrap slots ends when the gate returns.
        let _guards = guards.map(|fd| unsafe { OwnedFd::from_raw_fd(fd) });
        let endpoint = exchange::Endpoint::from_inherited(unsafe { OwnedFd::from_raw_fd(17) })?;
        exchange::require_live_peer(6, peer[0])?;
        let left = deadline
            .checked_sub(exchange::clock()?)
            .filter(|n| *n > 0)
            .ok_or_else(refused)?;
        let timeout = libc::timespec {
            tv_sec: (left / 1_000_000_000).try_into().map_err(|_| refused())?,
            tv_nsec: (left % 1_000_000_000) as _,
        };
        let mut fds = [6, 7, 8, 9, 17].map(|fd| libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        });
        if unsafe { libc::ppoll(fds.as_mut_ptr(), fds.len() as _, &timeout, std::ptr::null()) } != 1
            || fds[..4].iter().any(|p| p.revents != 0)
            || fds[4].revents != libc::POLLIN
            || endpoint.receive(peer)?.as_deref() != Some(&exchange::run_permission(&ack))
        {
            return Err(refused());
        }
        exchange::require_live_peer(6, peer[0])?;
        if exchange::clock()? >= deadline
            || unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as _, 0) } != 0
            || fds.iter().any(|fd| fd.revents != 0)
        {
            return Err(refused());
        }
        Ok(())
    }
}
