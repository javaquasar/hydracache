//! The channel a unix poller's wait is broken through: a pipe, whose read end the wait watches.

use std::{
    io,
    os::fd::{AsFd, BorrowedFd, OwnedFd},
};

use rustix::io::{Errno, read, write};

/// A pipe whose two ends are both non-blocking and neither of which a child process inherits.
///
/// Both ends have to be non-blocking: a `notify` that finds the pipe full is to be turned away
/// rather than left waiting for room, and the drain after a wait is to stop at the last byte
/// rather than wait for one more.
pub(super) struct Pipe {
    read: OwnedFd,
    write: OwnedFd,
}

impl Pipe {
    pub(super) fn new() -> io::Result<Self> {
        let (read, write) = pipe()?;

        Ok(Self { read, write })
    }

    /// Writes a wake-up for the wait watching the read end; callable from any thread.
    pub(super) fn notify(&self) -> io::Result<()> {
        match write(&self.write, &[1]) {
            // A full pipe holds a wake-up already.
            Ok(_) | Err(Errno::AGAIN) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    /// Takes every wake-up out, so that the next wait is not ended by one this wait answered.
    pub(super) fn drain(&self) {
        let mut buf = [0u8; 64];
        while read(&self.read, &mut buf).is_ok_and(|n| n == buf.len()) {}
    }

    /// The end a wait watches.
    pub(super) fn read_end(&self) -> BorrowedFd<'_> {
        self.read.as_fd()
    }
}

/// The two ends of a new pipe, set up as [`Pipe`] wants them.
#[cfg(not(target_vendor = "apple"))]
fn pipe() -> io::Result<(OwnedFd, OwnedFd)> {
    use rustix::pipe::{PipeFlags, pipe_with};

    Ok(pipe_with(PipeFlags::CLOEXEC | PipeFlags::NONBLOCK)?)
}

/// The two ends of a new pipe, set up as [`Pipe`] wants them.
///
/// Darwin has no `pipe2`, so both settings are made on the descriptors once the pipe exists.
#[cfg(target_vendor = "apple")]
fn pipe() -> io::Result<(OwnedFd, OwnedFd)> {
    use rustix::{
        io::{FdFlags, fcntl_setfd, ioctl_fionbio},
        pipe::pipe,
    };

    let (read, write) = pipe()?;
    for end in [&read, &write] {
        fcntl_setfd(end, FdFlags::CLOEXEC)?;
        ioctl_fionbio(end, true)?;
    }

    Ok((read, write))
}
