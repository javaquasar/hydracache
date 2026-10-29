//! Opening a socket on a thread that is free to wait in the kernel.

use std::io;

use socket2::{Domain, SockAddr, Socket, Type};

use crate::runtime::IoSource;

/// A new stream socket of `domain`, connected to `address` and in non-blocking mode.
///
/// The connect is a blocking one, so the kernel completes it before this returns: for a TCP
/// socket that is the handshake, and for a unix socket whose listener's backlog is full, it is a
/// wait for room in it. That is why this belongs on a thread made for blocking work, such as the
/// one [`Runtime::spawn_blocking`] hands out, and not on one that drives a runtime.
///
/// The socket is switched to non-blocking mode only once it is connected, which is how the
/// connection registers it with a runtime and does its I/O on it.
///
/// [`Runtime::spawn_blocking`]: crate::runtime::traits::Runtime::spawn_blocking
pub(crate) fn connect_blocking(domain: Domain, address: &SockAddr) -> io::Result<IoSource> {
    let socket = Socket::new(domain, Type::STREAM, None)?;
    // A signal handler without `SA_RESTART` interrupts a connect that has yet to complete, and the
    // connection carries on in the kernel. Asking again waits for it, or, where the kernel made it
    // in between, says the socket is connected already, which is the success asked for. That is
    // what the standard library's own connect does about it.
    loop {
        match socket.connect(address) {
            Ok(()) => break,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            #[cfg(unix)]
            Err(e) if e.raw_os_error() == Some(libc::EISCONN) => break,
            Err(e) => return Err(e),
        }
    }
    socket.set_nonblocking(true)?;

    Ok(IoSource::new(socket.into()))
}
