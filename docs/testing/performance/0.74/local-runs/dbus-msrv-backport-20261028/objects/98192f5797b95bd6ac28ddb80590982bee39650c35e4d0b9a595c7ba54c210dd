//! Connecting a stream socket without ever waiting in the kernel.

use std::io;
#[cfg(any(unix, target_os = "wasi"))]
use std::os::fd::{AsFd as AsSource, OwnedFd as Owned};
#[cfg(windows)]
use std::os::windows::io::{AsSocket as AsSource, OwnedSocket as Owned};
#[cfg(all(feature = "unix", unix))]
use std::time::Duration;

use socket2::{Domain, SockAddr, SockRef, Socket, Type};

use crate::{AsyncIo, Mode, Runtime};

/// [`connect`] for a unix socket, tried again for as long as a full listen backlog turns it away.
///
/// On Linux and Android, a blocking `connect(2)` on a unix socket waits in the kernel for room to
/// open up in the listener's backlog; a non-blocking one is turned away at once with `EAGAIN`
/// instead, and trying again needs a socket of its own. So there this loop is that same wait, done
/// on the runtime's timer rather than in the kernel: it keeps trying for as long as the backlog
/// stays full, and only whoever is awaiting this call bounds how long that is, through their own
/// timeout or by dropping the future. FreeBSD and macOS refuse a connection to a full listener with
/// `ECONNREFUSED` at once, blocking or not, so there is nothing to wait for, and the first
/// attempt's error is the result.
#[cfg(all(feature = "unix", unix))]
pub(super) async fn connect_unix<S, M>(
    runtime: &Runtime<M>,
    address: &SockAddr,
) -> io::Result<AsyncIo<S, M>>
where
    S: From<Owned> + AsSource + Send + Sync + 'static,
    M: Mode,
{
    loop {
        match connect(runtime, Domain::UNIX, address).await {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                runtime.sleep(BACKLOG_INTERVAL).await;
            }
            result => return result,
        }
    }
}

/// How long a connection a full listen backlog turned away waits before it is tried again.
#[cfg(all(feature = "unix", unix))]
const BACKLOG_INTERVAL: Duration = Duration::from_millis(20);

/// A stream socket of `domain`, connected to `address` and registered on `runtime`, as the std
/// type `S`.
///
/// The socket reaches `S` through what the platform owns a socket as, which every std socket
/// converts from: socket2 converts into the std types of unix sockets only with its `all` feature,
/// which this crate leaves off.
///
/// The socket is switched to non-blocking mode before the connect is issued, so a connection the
/// kernel cannot complete straight away is waited for on `runtime` instead of on the calling
/// thread: the wait is for writable readiness, which the kernel reports once it is done with the
/// connection either way, and [`outcome`] then says which way it went. The socket is registered
/// before that wait and stays registered for the traffic that follows.
///
/// On Linux and Android, a unix socket whose listener's backlog is full is turned away at once
/// rather than taken over, which fails with [`io::ErrorKind::WouldBlock`]: `connect_unix` waits
/// that out. FreeBSD and macOS refuse such a connection with `ECONNREFUSED` at once instead,
/// which fails the connect as it is.
pub(super) async fn connect<S, M>(
    runtime: &Runtime<M>,
    domain: Domain,
    address: &SockAddr,
) -> io::Result<AsyncIo<S, M>>
where
    S: From<Owned> + AsSource + Send + Sync + 'static,
    M: Mode,
{
    let socket = Socket::new(domain, Type::STREAM, None)?;
    socket.set_nonblocking(true)?;
    let under_way = match socket.connect(address) {
        Ok(()) => false,
        Err(e) if is_in_progress(&e) => true,
        Err(e) => return Err(e),
    };

    let io = AsyncIo::from_nonblocking(runtime, S::from(Owned::from(socket)))?;
    if under_way {
        io.write_with(|socket| outcome(socket)).await?;
    }

    Ok(io)
}

/// Whether `error` says the kernel is still working on the connection.
fn is_in_progress(error: &io::Error) -> bool {
    #[cfg(any(unix, target_os = "wasi"))]
    {
        rustix::io::Errno::from_io_error(error) == Some(rustix::io::Errno::INPROGRESS)
    }
    #[cfg(windows)]
    {
        error.kind() == io::ErrorKind::WouldBlock
    }
}

/// What has become of a connection the kernel took over.
///
/// The two platforms are asked different questions, because they answer differently. A unix
/// socket has no peer until its connection is made, so `getpeername` reporting `ENOTCONN` — which
/// arrives here as [`io::ErrorKind::NotConnected`] — is the mark of one still under way, and
/// `SO_ERROR` holds the reason for one that failed. Winsock names the peer from the moment a
/// connect is issued, so there the peer is evidence of nothing; what is, is a zero-timeout
/// `select`, which puts the socket in `writefds` once the connection is made and in `exceptfds`
/// once it has failed, and in neither while it is still under way.
///
/// WASI is asked as unix is, and wasi-libc answers the two questions as unix does, with one
/// addition: while the connection is under way, `SO_ERROR` holds `EINPROGRESS`. That is no reason
/// for a failure, but one more mark of a connection that is still under way.
///
/// A unix connection can settle in between the two questions. One that fails there leaves its
/// reason in `SO_ERROR` after the first question found none, and leaves the socket with no peer
/// for the second: Linux and FreeBSD say so with `ENOTCONN`, but macOS answers `getpeername` on a
/// socket that can neither send nor receive any more, as a refused one cannot, with `EINVAL`. So
/// a `getpeername` that fails any other way than `ENOTCONN` sends the question back to
/// `SO_ERROR`, which holds the reason by then.
///
/// On either platform, a connection still under way is reported as a would-block, which is what
/// makes the wait a readiness wait like any other. Asked before the socket is known to be ready, as
/// the registration's first attempt asks it, it says the connection is still under way and the
/// caller waits; asked once the kernel has reported writability — which it does as soon as it is
/// done with the connection, whichever way that went — it finds the settled answer.
fn outcome<S>(socket: &S) -> io::Result<()>
where
    S: AsSource,
{
    #[cfg(any(unix, target_os = "wasi"))]
    {
        let socket = SockRef::from(socket);

        match socket.take_error()? {
            #[cfg(target_os = "wasi")]
            Some(e) if is_in_progress(&e) => Err(io::ErrorKind::WouldBlock.into()),
            Some(e) => Err(e),
            None => match socket.peer_addr() {
                Ok(_) => Ok(()),
                Err(e) if e.kind() == io::ErrorKind::NotConnected => {
                    Err(io::ErrorKind::WouldBlock.into())
                }
                // The connection failed in between the two questions, and `SO_ERROR` holds the
                // reason now, which is what to report rather than how `getpeername` failed.
                Err(e) => Err(socket.take_error()?.unwrap_or(e)),
            },
        }
    }
    #[cfg(windows)]
    {
        match progress(socket)? {
            Progress::Connected => Ok(()),
            // Winsock is expected to keep the reason there, and a failure with none left is
            // still a failure: the connection is not to be reported as made.
            Progress::Failed => Err(SockRef::from(socket)
                .take_error()?
                .unwrap_or_else(|| io::Error::other("the connection failed"))),
            Progress::UnderWay => Err(io::ErrorKind::WouldBlock.into()),
        }
    }
}

/// How far Winsock has got with a connection.
#[cfg(windows)]
enum Progress {
    /// The connection is made.
    Connected,
    /// The connection failed, and `SO_ERROR` is where the reason is.
    Failed,
    /// Winsock is still working on the connection.
    UnderWay,
}

/// What a zero-timeout `select` says about the connection on `socket`.
///
/// `writefds` and `exceptfds` are the two answers Winsock has for a connect it has taken over,
/// and the zero timeout is what makes the call ask for them rather than wait for them: `select`
/// leaves a set holding only those of its sockets that the answer applies to, so an empty set is
/// a no.
#[cfg(windows)]
fn progress<S>(socket: &S) -> io::Result<Progress>
where
    S: AsSource,
{
    use std::{os::windows::io::AsRawSocket, ptr};

    use windows_sys::Win32::Networking::WinSock::{
        FD_SET, SOCKET, SOCKET_ERROR, TIMEVAL, WSAGetLastError, select,
    };

    let mut connected = FD_SET {
        fd_count: 1,
        ..FD_SET::default()
    };
    connected.fd_array[0] = socket.as_socket().as_raw_socket() as SOCKET;
    let mut failed = connected;
    // All zero, which is the timeout that makes `select` report and return.
    let immediately = TIMEVAL::default();

    // SAFETY: `select` reads and writes the two sets and reads the timeout for the length of the
    // call and no longer, and all three live in this frame across it. The sets are in the shape
    // `select` expects, a count of one against one entry, and that entry is a socket `socket`
    // holds open for the call. The first argument is ignored on Winsock.
    let ready = unsafe {
        select(
            1,
            ptr::null_mut(),
            &mut connected,
            &mut failed,
            &immediately,
        )
    };
    if ready == SOCKET_ERROR {
        // SAFETY: `WSAGetLastError` takes nothing and reads this thread's last Winsock error.
        return Err(io::Error::from_raw_os_error(unsafe { WSAGetLastError() }));
    }

    // `exceptfds` also reports urgent data waiting, which a connection just made may have, so a
    // socket in both sets is a connected one; a failed connect is never in `writefds`.
    if connected.fd_count != 0 {
        Ok(Progress::Connected)
    } else if failed.fd_count != 0 {
        Ok(Progress::Failed)
    } else {
        Ok(Progress::UnderWay)
    }
}
