//! The connects the runtime makes on its own poller and timer, with no thread to wait on.

use std::{future::poll_fn, io, net::SocketAddr, os::fd::OwnedFd, path::Path, time::Duration};

use socket2::{Domain, SockAddr, SockRef, Socket, Type};
use zbus::runtime::{
    Interest, IoSource,
    traits::{self, PollIo},
};

use super::Handle;

/// A stream socket connected to the TCP endpoint at `address`.
pub(super) async fn tcp(handle: &Handle, address: SocketAddr) -> io::Result<IoSource> {
    connect(handle, Domain::for_address(address), &address.into()).await
}

/// A stream socket connected to the unix-domain socket at `path`.
///
/// A non-blocking connect to a listener whose backlog is full is turned away at once, where a
/// blocking one would wait in the kernel for room in it. So the wait is done here instead, on the
/// runtime's own timer: the connect is tried again for as long as the backlog stays full.
pub(super) async fn unix(handle: &Handle, path: &Path) -> io::Result<IoSource> {
    let address = SockAddr::unix(path)?;

    loop {
        match connect(handle, Domain::UNIX, &address).await {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                traits::Runtime::sleep(handle, BACKLOG_INTERVAL).await
            }
            result => return result,
        }
    }
}

/// One try at a stream socket of `domain` connected to `address`.
///
/// The socket is non-blocking from the moment it exists, so a connection the kernel cannot
/// complete straight away is waited for on the poller. The wait is for the socket to become
/// writable, which the kernel reports once it is done with the connection either way, and
/// [`settled`] then says which way it went.
async fn connect(handle: &Handle, domain: Domain, address: &SockAddr) -> io::Result<IoSource> {
    let socket = Socket::new(domain, Type::STREAM, None)?;
    socket.set_nonblocking(true)?;

    let connected = socket.connect(address);
    let source = IoSource::from(OwnedFd::from(socket));
    match connected {
        Ok(()) => return Ok(source),
        Err(e) if e.raw_os_error() == Some(libc::EINPROGRESS) => {}
        Err(e) => return Err(e),
    }

    let registration = traits::Runtime::register_io_source(handle, source.clone())?;
    let outcome =
        poll_fn(|cx| registration.poll_io(cx, Interest::Writable, || settled(&source))).await;
    // The poller has no more to watch the socket for, and zbus is about to register it again for
    // the connection's own traffic.
    drop(registration);
    outcome?;

    Ok(source)
}

/// How long a connect that a full listen backlog turned away waits before it is tried again.
const BACKLOG_INTERVAL: Duration = Duration::from_millis(20);

/// What has become of the connection under way on `source`.
///
/// A socket has no peer until its connection is made, so `getpeername` reporting `ENOTCONN` — which
/// arrives here as [`io::ErrorKind::NotConnected`] — is the mark of one still under way, and
/// `SO_ERROR` holds the reason for one that failed. A connection still under way is reported as a
/// would-block, which is what makes the wait a readiness wait like any other.
///
/// Both of the polling orders [`PollIo`] allows an implementation to choose between end up at the
/// same answer. Asked before the socket is known to be ready, this says the connection is still
/// under way and the caller waits; asked once the kernel has reported writability, it finds the
/// settled answer.
fn settled(source: &IoSource) -> io::Result<()> {
    let socket = SockRef::from(source);

    match socket.take_error()? {
        Some(e) => Err(e),
        None => match socket.peer_addr() {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotConnected => {
                Err(io::ErrorKind::WouldBlock.into())
            }
            Err(e) => Err(e),
        },
    }
}
