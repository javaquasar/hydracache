//! The operations of a TCP stream.

use std::io;
#[cfg(unix)]
use std::os::fd::BorrowedFd;

use super::SocketOps;
use crate::{connection::socket::RecvmsgResult, runtime::IoSource};

/// The operations of a TCP stream.
#[derive(Debug)]
pub(crate) struct TcpOps;

impl SocketOps for TcpOps {
    fn recv(&self, source: &IoSource, buffer: &mut [u8]) -> RecvmsgResult {
        super::recv(source, buffer)
    }

    fn send(
        &self,
        source: &IoSource,
        buffer: &[u8],
        #[cfg(unix)] fds: &[BorrowedFd<'_>],
    ) -> io::Result<usize> {
        #[cfg(unix)]
        if !fds.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "fds cannot be sent with a tcp stream",
            ));
        }

        super::send(source, buffer)
    }

    /// A TCP peer proves nothing about itself, so it authenticates anonymously.
    #[cfg(not(windows))]
    fn auth_mechanism(&self) -> crate::conn::AuthMechanism {
        crate::conn::AuthMechanism::Anonymous
    }

    /// Windows finds the peer in its own table of TCP connections, so `EXTERNAL` works there.
    #[cfg(windows)]
    fn peer_credentials(
        &self,
        source: &IoSource,
        runtime: &crate::runtime::Runtime,
    ) -> impl std::future::Future<Output = io::Result<crate::fdo::ConnectionCredentials>> + Send
    {
        peer_credentials(source, runtime)
    }
}

/// The credentials of the peer of a Windows TCP stream.
///
/// Finding the peer means walking the machine's whole table of TCP connections, which is not work
/// for the thread a connection is polled on, so it goes to `runtime`.
#[cfg(windows)]
async fn peer_credentials(
    source: &IoSource,
    runtime: &crate::runtime::Runtime,
) -> io::Result<crate::fdo::ConnectionCredentials> {
    let peer = socket2::SockRef::from(source)
        .peer_addr()
        .and_then(|peer| {
            peer.as_socket()
                .ok_or_else(|| io::Error::other("the peer of a TCP stream has no IP address"))
        })?;

    runtime
        .spawn_blocking(move || credentials_from_addr(&peer))
        .await
}

/// The credentials of the process Windows has a TCP connection from `addr` registered to.
#[cfg(windows)]
fn credentials_from_addr(
    addr: &std::net::SocketAddr,
) -> io::Result<crate::fdo::ConnectionCredentials> {
    use crate::win32::{ProcessToken, socket_addr_get_pid};

    let pid = socket_addr_get_pid(addr)? as _;
    let sid = ProcessToken::open(if pid != 0 { Some(pid as _) } else { None })
        .and_then(|process_token| process_token.sid())?;

    Ok(crate::fdo::ConnectionCredentials::default()
        .set_process_id(pid)
        .set_windows_sid(sid))
}
