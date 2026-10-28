//! Async sockets on a [`Runtime`].
//!
//! The module has TCP sockets (`TcpListener` and `TcpStream`), a UDP socket (`UdpSocket`) and, on
//! unix platforms, unix-domain sockets in the `unix` module (`UnixListener`, `UnixStream` and
//! `UnixDatagram`). The families are behind the `tcp`, `udp` and `unix` cargo features, none on by
//! default.
//!
//! # The runtime
//!
//! A socket is created on a runtime. Its constructor takes the runtime as the first argument, and
//! the runtime's reactor watches the socket from then on.
//!
//! The socket's operations make progress while a thread is running [`Runtime::block_on`] on that
//! runtime. A runtime from `SharedRuntime::current` also has a helper thread, which drives the
//! runtime when no thread is running `block_on` on it.
//!
//! The socket's type carries the flavour of its runtime. A socket made on a [`LocalRuntime`] is,
//! for example, a `TcpStream<Local>` (`Local` is the default). It stays on the thread that made it.
//! A socket made on a [`SharedRuntime`] is a `TcpStream<Shared>`. It can be sent to and used from
//! any thread.
//!
//! # Operations
//!
//! Sockets are in non-blocking mode. When an operation would block, the task waits until the
//! runtime reports the socket ready, and other tasks run on the thread in the meantime. Connecting
//! is such an operation too, so `connect` never blocks the thread.
//!
//! TCP and UDP sockets are bound or connected to a socket address, never to a host name. Looking up
//! a name, for example with std's `ToSocketAddrs`, blocks the thread until the resolver answers,
//! and a task must not do that. To connect to a host name, look it up on another thread (for
//! example with `unblock`), then try the addresses it returns in turn.
//!
//! `TcpStream` and `UnixStream` implement the `AsyncRead` and `AsyncWrite` traits of
//! [`futures-io`], so the extension traits of [`futures`] work on them. A shared reference to a
//! stream implements the traits too, so a reader and a writer can share one stream. Closing a
//! stream shuts down its write half, and the peer then reads the end of the stream.
//!
//! # Several tasks on one socket
//!
//! Any number of tasks can wait in the async methods of one socket at once, through shared
//! references. These are the methods that accept, peek, receive or send, in either direction. When
//! the socket becomes ready, the waiting tasks wake. A task that finds another task took the data
//! first waits again.
//!
//! Two paths allow only one waiting task per direction: the `AsyncRead` and `AsyncWrite`
//! implementations of a stream, and the `Incoming` stream of a listener. If a second task waits in
//! the same direction through one of them, it replaces the first, which is then never woken. Tasks
//! that share a direction there must take turns, for example behind a lock. A task waiting in an
//! async method never replaces a task waiting through one of these paths, nor the other way round.
//!
//! # Limits on Windows
//!
//! On Windows, a runtime watches at most 1023 sockets at a time. A server there can hold at most
//! that many sockets on one runtime, its listener included.
//!
//! # Limits on WASI
//!
//! WASI has no unix-domain sockets, so the `unix` module is not there. Whether a program may use
//! the network at all is for the host to say, and where it does not, the TCP and UDP operations
//! that need it fail with the host's refusal.
//!
//! On `wasm32-wasip2`, the sockets work as they do on unix. The runtime watches them with
//! `poll(2)`, and a connect does not block the thread.
//!
//! `wasm32-wasip1` cannot create a socket: `TcpStream::connect` and `TcpListener::bind` fail there
//! with [`Unsupported`](std::io::ErrorKind::Unsupported). What the TCP sockets can do there is run
//! on one that the host opened before the program started and handed in as a file descriptor. Make
//! a std socket from the descriptor, for example with `FromRawFd::from_raw_fd`, and pass it to
//! `TcpStream::from_std` or `TcpListener::from_std`.
//!
//! UDP is not available on `wasm32-wasip1` at all: every operation of std's UDP socket fails there
//! with `Unsupported`, so `UdpSocket::bind` and `UdpSocket::from_std` do too, even on a socket that
//! the host handed in.
//!
//! [`futures-io`]: https://docs.rs/futures-io
//! [`futures`]: https://docs.rs/futures
//! [`LocalRuntime`]: crate::LocalRuntime
//! [`Runtime`]: crate::Runtime
//! [`Runtime::block_on`]: crate::Runtime::block_on
//! [`SharedRuntime`]: crate::SharedRuntime

#[cfg(any(feature = "tcp", all(feature = "unix", unix)))]
use std::io;
#[cfg(any(
    all(any(unix, target_os = "wasi"), feature = "tcp"),
    all(unix, feature = "unix")
))]
use std::os::fd::AsFd as AsSource;
#[cfg(all(windows, feature = "tcp"))]
use std::os::windows::io::AsSocket as AsSource;

#[cfg(any(
    all(feature = "tcp", not(all(target_os = "wasi", target_env = "p1"))),
    all(feature = "unix", unix)
))]
mod connect;
#[cfg(feature = "tcp")]
mod tcp;
#[cfg(feature = "udp")]
mod udp;
#[cfg(all(feature = "unix", unix))]
pub mod unix;

#[cfg(feature = "tcp")]
pub use tcp::{Incoming, TcpListener, TcpStream};
#[cfg(feature = "udp")]
pub use udp::UdpSocket;

/// Makes a write to `socket`, once its peer has gone, fail rather than raise `SIGPIPE`, on
/// Apple's platforms, and does nothing elsewhere.
///
/// Those platforms have no `MSG_NOSIGNAL` for a write to ask for that with, so the socket sees to
/// it itself, through its `SO_NOSIGPIPE` option. std and socket2 set the option on the sockets
/// they make, but std's `pair` sets it on neither socket, and a socket handed to a `from_std` may
/// come from elsewhere.
#[cfg(any(feature = "tcp", all(feature = "unix", unix)))]
pub(crate) fn set_nosigpipe<S>(socket: &S) -> io::Result<()>
where
    S: AsSource,
{
    #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "tvos",
        target_os = "visionos",
        target_os = "watchos"
    ))]
    {
        Ok(rustix::net::sockopt::set_socket_nosigpipe(socket, true)?)
    }
    #[cfg(not(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "tvos",
        target_os = "visionos",
        target_os = "watchos"
    )))]
    {
        let _ = socket;

        Ok(())
    }
}
