//! The operations of one end of a pipe to a helper process.

use std::{io, os::fd::BorrowedFd};

use super::SocketOps;
use crate::{connection::socket::RecvmsgResult, runtime::IoSource};

/// The operations of one end of a pipe.
///
/// A pipe carries a stream of bytes in one direction and nothing besides: it has no ancillary
/// data, no peer to ask about and no half that can be shut down on its own.
#[derive(Debug)]
pub(crate) struct PipeOps;

impl SocketOps for PipeOps {
    /// A read of nothing is the other end being gone, which ends the stream rather than failing
    /// it; a caller that needs more than it got says so itself.
    fn recv(&self, source: &IoSource, buffer: &mut [u8]) -> RecvmsgResult {
        let read = rustix::io::read(source, buffer)?;

        Ok((read, vec![]))
    }

    fn send(&self, source: &IoSource, buffer: &[u8], fds: &[BorrowedFd<'_>]) -> io::Result<usize> {
        if !fds.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "fds cannot be sent over a pipe",
            ));
        }

        Ok(rustix::io::write(source, buffer)?)
    }

    /// A pipe ends when the writer lets go of its end, which is also how the write half of one
    /// closes, so there is nothing to shut down here.
    fn shutdown(&self, _source: &IoSource) -> io::Result<()> {
        Ok(())
    }
}
