//! Unix-domain sockets, on unix platforms only.
//!
//! This module has [`UnixListener`], [`UnixStream`] and [`UnixDatagram`], and the [`Incoming`]
//! stream of the connections a listener accepts.
//!
//! A unix-domain socket is named by a path in the file system. Binding a listener or a datagram
//! socket to a path creates a socket file there. A stream connects to a listener, and a datagram
//! socket sends to another, by the path the target is bound to.
//!
//! Nothing removes the socket file when the socket is dropped, so remove it when you are done with
//! the socket. Binding to a path that already has a file fails with
//! [`AddrInUse`](std::io::ErrorKind::AddrInUse). The system limits the length of a path to about a
//! hundred bytes. A longer path fails with [`InvalidInput`](std::io::ErrorKind::InvalidInput).
//!
//! On Linux and Android a socket can also be named in the abstract namespace, by a string of bytes
//! that has no file behind it. std's `SocketAddrExt` creates and reads such names, and
//! [`UnixStream::connect_addr`] connects to one.
//!
//! The sockets run on a runtime like the sockets of the [parent module](super). Its documentation
//! explains which threads drive a socket's operations, how a socket's type carries its runtime's
//! flavour, and how many tasks can wait on one socket at once.

#[cfg(target_os = "android")]
use std::os::android::net::SocketAddrExt;
#[cfg(target_os = "linux")]
use std::os::linux::net::SocketAddrExt;
use std::{
    fmt,
    future::poll_fn,
    io::{self, Read},
    net::Shutdown,
    os::{
        fd::{AsFd, AsRawFd, BorrowedFd, RawFd},
        unix::{ffi::OsStrExt, net::SocketAddr},
    },
    path::Path,
    pin::Pin,
    task::{Context, Poll, ready},
    time::Duration,
};

use futures_core::Stream;
use futures_io::{AsyncRead, AsyncWrite};
use socket2::SockAddr;

use super::{connect, set_nosigpipe};
use crate::{AsyncIo, Local, Mode, Runtime, coop};

/// A unix-domain socket server, listening for connections.
///
/// Create a listener with [`UnixListener::bind`]. Then call [`accept`](UnixListener::accept) to
/// take one connection, or [`incoming`](UnixListener::incoming) for a stream of connections. Each
/// connection is a [`UnixStream`]. The listener is the async counterpart of
/// [`std::os::unix::net::UnixListener`]: waiting for a connection lets other tasks run instead of
/// blocking the thread.
///
/// The listener runs on the runtime passed to its constructor, and so do the streams it accepts.
/// Its type carries that runtime's flavour, as the [module documentation](super) explains: a
/// `UnixListener<Local>` (the default) stays on the thread that made it, and a
/// `UnixListener<Shared>` can be sent to and used from any thread.
///
/// Any number of tasks can wait in [`accept`](UnixListener::accept) at once through a reference to
/// the listener, and each connection goes to one of them. Only one task at a time can wait for the
/// next item of an [`Incoming`] stream, counting all the streams of the listener. Tasks waiting in
/// `accept` do not count.
///
/// # Example
///
/// A listener accepts a connection and reads a greeting from it. A client makes the connection and
/// sends the greeting. The socket file is in a directory of its own, which the example removes at
/// the end.
///
/// ```
/// use futures::{AsyncReadExt, AsyncWriteExt};
/// use zruntime::{
///     LocalRuntime,
///     net::unix::{UnixListener, UnixStream},
/// };
///
/// let directory = std::env::temp_dir()
///     .join(format!("zruntime-unix-listener-doc-{}", std::process::id()));
/// std::fs::create_dir(&directory)?;
/// let path = directory.join("socket");
///
/// let runtime = LocalRuntime::new()?;
/// let listener = UnixListener::bind(&runtime, &path)?;
///
/// runtime.block_on(async {
///     let mut client = UnixStream::connect(&runtime, &path).await?;
///     client.write_all(b"hello").await?;
///
///     let (mut server, _) = listener.accept().await?;
///     let mut greeting = [0; 5];
///     server.read_exact(&mut greeting).await?;
///
///     assert_eq!(&greeting, b"hello");
///     assert_eq!(server.local_addr()?.as_pathname(), Some(path.as_path()));
///     # Ok::<_, std::io::Error>(())
/// })?;
///
/// std::fs::remove_dir_all(&directory)?;
/// # Ok::<_, std::io::Error>(())
/// ```
pub struct UnixListener<M = Local>
where
    M: Mode,
{
    io: AsyncIo<std::os::unix::net::UnixListener, M>,
}

impl<M> UnixListener<M>
where
    M: Mode,
{
    /// Creates a listener bound to `path`, on `runtime`.
    ///
    /// Binding creates a socket file at `path`. The listener is bound and listening when this
    /// returns. Dropping the listener does not remove the file, so remove it when you are done with
    /// the listener.
    ///
    /// # Errors
    ///
    /// Fails with [`AddrInUse`](io::ErrorKind::AddrInUse) if `path` already has a file. This
    /// includes a stale socket file left by an earlier listener, which the caller must remove
    /// first. Fails with [`InvalidInput`](io::ErrorKind::InvalidInput) if `path` is too long to
    /// name a socket (about a hundred bytes). Also fails for the reasons
    /// [`from_std`](UnixListener::from_std) fails.
    pub fn bind<P>(runtime: &Runtime<M>, path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        Self::from_std(runtime, std::os::unix::net::UnixListener::bind(path)?)
    }

    /// Creates a listener on `runtime` from a std listener.
    ///
    /// `listener` is switched to non-blocking mode, as every listener of this type is. The mode
    /// belongs to the open socket, so a duplicate made with
    /// [`try_clone`](std::os::unix::net::UnixListener::try_clone) is switched too. An `accept` on
    /// that duplicate then fails with [`WouldBlock`](io::ErrorKind::WouldBlock) instead of waiting.
    ///
    /// On Apple's platforms the socket also gets `SO_NOSIGPIPE`, and so do the streams it accepts,
    /// so that a write to a peer that has gone fails instead of raising `SIGPIPE`.
    ///
    /// # Errors
    ///
    /// Fails if switching to non-blocking mode fails, if setting `SO_NOSIGPIPE` fails (on Apple's
    /// platforms), or if the runtime cannot start watching the socket.
    pub fn from_std(
        runtime: &Runtime<M>,
        listener: std::os::unix::net::UnixListener,
    ) -> io::Result<Self> {
        listener.set_nonblocking(true)?;
        set_nosigpipe(&listener)?;

        Ok(Self {
            io: AsyncIo::from_nonblocking(runtime, listener)?,
        })
    }

    /// Waits for a connection to this listener, and accepts it.
    ///
    /// Returns the stream of the connection and the address of the peer that made it. The stream
    /// runs on the listener's runtime and is in non-blocking mode. The address is unnamed unless
    /// the peer bound its socket to a path before it connected.
    ///
    /// # Errors
    ///
    /// Returns the error of the accept.
    ///
    /// Some errors leave the connection queued, for example running out of file descriptors
    /// (`EMFILE`) on Linux and the BSDs. The next call then fails again at once, and so on until
    /// the connection leaves the queue, so a loop that keeps accepting after an error spins the
    /// thread. Back off after an error, for example with [`Runtime::sleep`]. On Apple's platforms
    /// an accept that finds no descriptor left closes the connection instead: the error does not
    /// repeat, but the connection is lost.
    ///
    /// # Cancel safety
    ///
    /// Dropping the future before it completes gives up the wait. No connection is lost: a
    /// connection that arrives meanwhile stays queued for the next call.
    pub async fn accept(&self) -> io::Result<(UnixStream<M>, SocketAddr)> {
        let (stream, address) = self.io.read_with(|listener| listener.accept()).await?;

        self.accepted(stream, address)
    }

    /// Returns a stream of the connections made to this listener.
    ///
    /// Each item is the [`UnixStream`] of a connection, accepted as
    /// [`accept`](UnixListener::accept) accepts it, or the error of a failed accept. See
    /// [`Incoming`] for how errors and waiting tasks behave.
    pub fn incoming(&self) -> Incoming<'_, M> {
        Incoming { listener: self }
    }

    /// The socket address this listener is bound to.
    ///
    /// This is the path the listener was bound to, which [`SocketAddr::as_pathname`] returns.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.io.get_ref().local_addr()
    }
}

impl<M> fmt::Debug for UnixListener<M>
where
    M: Mode,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.io.get_ref(), f)
    }
}

impl<M> AsFd for UnixListener<M>
where
    M: Mode,
{
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.io.get_ref().as_fd()
    }
}

impl<M> AsRawFd for UnixListener<M>
where
    M: Mode,
{
    fn as_raw_fd(&self) -> RawFd {
        self.io.get_ref().as_raw_fd()
    }
}

/// A unix-domain connection, to read from and write to.
///
/// Create a stream with [`UnixStream::connect`] (by path), [`UnixStream::connect_addr`] (by
/// address) or [`UnixStream::pair`], or get one from [`UnixListener::accept`]. The stream is the
/// async counterpart of [`std::os::unix::net::UnixStream`]: a read or write that has to wait lets
/// other tasks run instead of blocking the thread.
///
/// The stream implements the `AsyncRead` and `AsyncWrite` traits of [`futures-io`], so the
/// extension traits of [`futures`] work on it. So does `&UnixStream`, which lets a reader and a
/// writer share one stream, as std's `Read` and `Write` do for a `&std::os::unix::net::UnixStream`.
/// A write sends as much as the kernel takes at once, which may be less than the whole buffer.
/// Nothing is buffered, so flushing does nothing. A write to a peer that has gone away fails with
/// [`BrokenPipe`](io::ErrorKind::BrokenPipe).
///
/// Closing the stream (for example with the `close` method of `AsyncWriteExt`) shuts down the write
/// half of the socket, like [`shutdown`](UnixStream::shutdown) with [`Shutdown::Write`]. The peer
/// reads the end of the stream once it has read everything sent. The read half stays open. It is
/// the socket that is shut down, so closing the stream through one `&UnixStream` ends it for every
/// handle on it. Closing a stream that is already closed is not an error.
///
/// The stream runs on the runtime passed to its constructor. A stream accepted by a
/// [`UnixListener`] runs on the listener's runtime. The stream's type carries that runtime's
/// flavour, as the [module documentation](super) explains: a `UnixStream<Local>` (the default)
/// stays on the thread that made it, and a `UnixStream<Shared>` can be sent to and used from any
/// thread.
///
/// Only one task at a time can wait to read through the `AsyncRead` implementation, and only one to
/// write through `AsyncWrite`, counting all references to the stream. If a second task waits in the
/// same direction, it replaces the first, which is then never woken. Tasks that read, or write,
/// together must take turns, for example behind a lock.
///
/// # Example
///
/// A pair of connected streams. The client sends a message and closes its end of the stream. The
/// server reads the message up to the end of the stream. The server can still send an answer to the
/// client.
///
/// ```
/// use futures::{AsyncReadExt, AsyncWriteExt};
/// use zruntime::{LocalRuntime, net::unix::UnixStream};
///
/// let runtime = LocalRuntime::new()?;
/// let (mut client, mut server) = UnixStream::pair(&runtime)?;
///
/// runtime.block_on(async {
///     client.write_all(b"ping").await?;
///     // Closing shuts down the client's write half. The server reads the end of the stream
///     // after the bytes the client sent.
///     client.close().await?;
///
///     let mut message = Vec::new();
///     server.read_to_end(&mut message).await?;
///     assert_eq!(message, b"ping");
///
///     // The other direction is still open.
///     server.write_all(b"pong").await?;
///     let mut answer = [0; 4];
///     client.read_exact(&mut answer).await?;
///     assert_eq!(&answer, b"pong");
///     # Ok::<_, std::io::Error>(())
/// })?;
/// # Ok::<_, std::io::Error>(())
/// ```
///
/// [`futures-io`]: https://docs.rs/futures-io
/// [`futures`]: https://docs.rs/futures
pub struct UnixStream<M = Local>
where
    M: Mode,
{
    io: AsyncIo<std::os::unix::net::UnixStream, M>,
}

impl<M> UnixStream<M>
where
    M: Mode,
{
    /// Creates a stream connected to the socket at `path`, on `runtime`.
    ///
    /// `path` is the path a [`UnixListener`] is bound to. The connection does not block the thread.
    ///
    /// On Linux and Android, a listener whose backlog is full has no room for another connection.
    /// The connect then waits for room to open up, without blocking the thread: it tries again
    /// every 20 milliseconds for as long as the caller awaits it. Only the caller bounds this wait,
    /// with a timeout or by dropping the future. Other platforms refuse a connection to a listener
    /// whose backlog is full: the connect fails at once with
    /// [`ConnectionRefused`](io::ErrorKind::ConnectionRefused).
    ///
    /// To connect by a socket address as std gives it, rather than by a path, see
    /// [`connect_addr`](UnixStream::connect_addr). On Linux and Android that is the only way to
    /// reach a socket in the abstract namespace.
    ///
    /// # Errors
    ///
    /// Fails with [`NotFound`](io::ErrorKind::NotFound) if there is no file at `path`, and with
    /// [`ConnectionRefused`](io::ErrorKind::ConnectionRefused) if the file is a socket that nothing
    /// listens on any more. Fails with [`InvalidInput`](io::ErrorKind::InvalidInput) if `path` is
    /// too long to name a socket (about a hundred bytes) or contains a zero byte.
    ///
    /// # Cancel safety
    ///
    /// Dropping the future before it completes gives up the connection attempt.
    pub async fn connect<P>(runtime: &Runtime<M>, path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        Self::connect_sockaddr(runtime, &pathname_sockaddr(path.as_ref())?).await
    }

    /// Creates a stream connected to the socket at `address`, on `runtime`.
    ///
    /// `address` is a socket address as std gives it, such as the
    /// [`local_addr`](UnixListener::local_addr) of a listener. It names a socket by the path it is
    /// bound to, which [`connect`](UnixStream::connect) takes as it is, or, on Linux and Android,
    /// by a name in the abstract namespace. Such a socket has no file behind it, and its name is a
    /// string of bytes, any of which may be a zero byte. std's [`SocketAddrExt`] creates an address
    /// from a name and reads the name of an address.
    ///
    /// The connection does not block the thread. A full listener backlog is handled as for
    /// [`connect`](UnixStream::connect).
    ///
    /// # Errors
    ///
    /// For a path, the errors are those of [`connect`](UnixStream::connect). A name in the abstract
    /// namespace has no file to be missing, so a name that nothing listens at fails with
    /// [`ConnectionRefused`](io::ErrorKind::ConnectionRefused). An unnamed address belongs to a
    /// socket bound to nothing, such as either end of a [pair](UnixStream::pair). It names no
    /// socket to connect to, and fails with [`InvalidInput`](io::ErrorKind::InvalidInput).
    ///
    /// # Cancel safety
    ///
    /// Dropping the future before it completes gives up the connection attempt.
    ///
    /// [`SocketAddrExt`]: https://doc.rust-lang.org/std/os/linux/net/trait.SocketAddrExt.html
    pub async fn connect_addr(runtime: &Runtime<M>, address: &SocketAddr) -> io::Result<Self> {
        Self::connect_sockaddr(runtime, &named_sockaddr(address)?).await
    }

    /// Creates a connected pair of streams, on `runtime`.
    ///
    /// What is written to one stream is read from the other, and the other way round. The streams
    /// have no path to name them by. Both are in non-blocking mode, as every stream of this type
    /// is.
    ///
    /// # Errors
    ///
    /// Fails if the system cannot create the pair, or if the runtime cannot start watching either
    /// socket.
    pub fn pair(runtime: &Runtime<M>) -> io::Result<(Self, Self)> {
        let (first, second) = std::os::unix::net::UnixStream::pair()?;

        Ok((
            Self::from_std(runtime, first)?,
            Self::from_std(runtime, second)?,
        ))
    }

    /// Creates a stream on `runtime` from a std stream.
    ///
    /// `stream` is switched to non-blocking mode, as every stream of this type is. The mode belongs
    /// to the open socket, so a duplicate made with
    /// [`try_clone`](std::os::unix::net::UnixStream::try_clone) is switched too. A read or write on
    /// that duplicate then fails with [`WouldBlock`](io::ErrorKind::WouldBlock) instead of waiting.
    ///
    /// On Apple's platforms the socket also gets `SO_NOSIGPIPE`, so that a write to a peer that has
    /// gone fails instead of raising `SIGPIPE`.
    ///
    /// # Errors
    ///
    /// Fails if switching to non-blocking mode fails, if setting `SO_NOSIGPIPE` fails (on Apple's
    /// platforms), or if the runtime cannot start watching the socket.
    pub fn from_std(
        runtime: &Runtime<M>,
        stream: std::os::unix::net::UnixStream,
    ) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        set_nosigpipe(&stream)?;

        Ok(Self {
            io: AsyncIo::from_nonblocking(runtime, stream)?,
        })
    }

    /// The socket address of this end of the connection.
    ///
    /// It names the path this end is bound to, if any. A stream accepted by a listener is on the
    /// listener's path. A stream that connected to a path, or is one of a pair, is bound to no
    /// path, and its address is unnamed.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.io.get_ref().local_addr()
    }

    /// The socket address of the peer: the other end of the connection.
    ///
    /// It names the path the peer is bound to, if any, as [`local_addr`](UnixStream::local_addr)
    /// does for this end.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.io.get_ref().peer_addr()
    }

    /// Stops watching the stream, and returns the socket as a std stream, still in non-blocking
    /// mode.
    ///
    /// The connection is left as it was: the std stream can read the bytes that arrived but were
    /// not read yet. The socket stays in non-blocking mode, so a read or write on the std stream
    /// fails with [`WouldBlock`](io::ErrorKind::WouldBlock) instead of waiting, until
    /// [`set_nonblocking`](std::os::unix::net::UnixStream::set_nonblocking) switches the mode back.
    ///
    /// On a shared runtime this may wait briefly, until a thread driving the runtime returns from a
    /// wait that was watching the socket.
    pub fn into_std(self) -> std::os::unix::net::UnixStream {
        self.io.into_inner()
    }

    /// Shuts down the read half, the write half or both halves of the connection, as `how` says.
    ///
    /// Shutting down the write half makes the peer read the end of the stream, once it has read
    /// everything sent. Closing the stream does the same. It is the socket that is shut down, so
    /// this affects every handle on the stream.
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.io.get_ref().shutdown(how)
    }
}

impl<M> AsyncRead for UnixStream<M>
where
    M: Mode,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &*self).poll_read(cx, buf)
    }
}

impl<M> AsyncWrite for UnixStream<M>
where
    M: Mode,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &*self).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut &*self).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut &*self).poll_close(cx)
    }
}

impl<M> AsyncRead for &UnixStream<M>
where
    M: Mode,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        self.io.poll_read_with(cx, |mut stream| stream.read(buf))
    }
}

// A write is a `send`, not std's `Write for &UnixStream`, which sends with `MSG_NOSIGNAL` only
// from a release later than this crate's MSRV: a write to a peer that has gone must be an error
// and never a `SIGPIPE`. The vectored methods are left to the traits' defaults, which write the
// first buffer that is not empty, so that they send as this does: std's vectored write sends
// without the flag.
impl<M> AsyncWrite for &UnixStream<M>
where
    M: Mode,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.io
            .poll_write_with(cx, |stream| Ok(rustix::net::send(stream, buf, SEND_FLAGS)?))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // Nothing is buffered here: a write that reports success has gone to the kernel.
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.io.get_ref().shutdown(Shutdown::Write) {
            // A stream whose connection is gone has nothing left to close, and some BSDs report
            // `NotConnected` for a second shutdown: either way the stream is closed already, and
            // closing it again is not an error, on any platform.
            Err(e) if e.kind() == io::ErrorKind::NotConnected => Poll::Ready(Ok(())),
            result => Poll::Ready(result),
        }
    }
}

impl<M> fmt::Debug for UnixStream<M>
where
    M: Mode,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.io.get_ref(), f)
    }
}

impl<M> AsFd for UnixStream<M>
where
    M: Mode,
{
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.io.get_ref().as_fd()
    }
}

impl<M> AsRawFd for UnixStream<M>
where
    M: Mode,
{
    fn as_raw_fd(&self) -> RawFd {
        self.io.get_ref().as_raw_fd()
    }
}

/// A unix-domain datagram socket, to send datagrams from and receive datagrams on.
///
/// A datagram socket exchanges datagrams, each a message of its own, instead of a stream of bytes.
/// The socket is the async counterpart of [`std::os::unix::net::UnixDatagram`]: a receive or send
/// that has to wait lets other tasks run instead of blocking the thread.
///
/// Create a socket in one of these ways:
///
/// * [`bind`](UnixDatagram::bind) binds it to a path, so that others can send to it by that path.
/// * [`unbound`](UnixDatagram::unbound) creates a socket that can send but has no address to be
///   sent to.
/// * [`pair`](UnixDatagram::pair) creates two sockets that are connected to each other.
///
/// [`send_to`] sends a datagram to the socket bound to a path. [`connect`] fixes the one socket to
/// send to and receive from, which [`send`] and [`recv`] then use. A datagram keeps its boundaries:
/// one receive takes one datagram, and what does not fit into the buffer is discarded.
///
/// The socket runs on the runtime passed to its constructor. Its type carries that runtime's
/// flavour, as the [module documentation](super) explains: a `UnixDatagram<Local>` (the default)
/// stays on the thread that made it, and a `UnixDatagram<Shared>` can be sent to and used from any
/// thread.
///
/// Any number of tasks can wait to receive at once, with [`recv`](UnixDatagram::recv) or
/// [`recv_from`](UnixDatagram::recv_from), and any number can wait to send, with
/// [`send`](UnixDatagram::send) or [`send_to`](UnixDatagram::send_to), each through a reference to
/// the socket. Each receive takes a datagram of its own, so tasks that receive together get one
/// each.
///
/// # Example
///
/// A pair of datagram sockets: one sends a datagram and the other receives it, then the answer goes
/// the other way.
///
/// ```
/// use zruntime::{LocalRuntime, net::unix::UnixDatagram};
///
/// let runtime = LocalRuntime::new()?;
/// let (client, server) = UnixDatagram::pair(&runtime)?;
///
/// runtime.block_on(async {
///     client.send(b"ping").await?;
///     let mut datagram = [0; 16];
///     let received = server.recv(&mut datagram).await?;
///     assert_eq!(&datagram[..received], b"ping");
///
///     server.send(b"pong").await?;
///     let received = client.recv(&mut datagram).await?;
///     assert_eq!(&datagram[..received], b"pong");
///     # Ok::<_, std::io::Error>(())
/// })?;
/// # Ok::<_, std::io::Error>(())
/// ```
///
/// [`send_to`]: UnixDatagram::send_to
/// [`connect`]: UnixDatagram::connect
/// [`send`]: UnixDatagram::send
/// [`recv`]: UnixDatagram::recv
pub struct UnixDatagram<M = Local>
where
    M: Mode,
{
    io: AsyncIo<std::os::unix::net::UnixDatagram, M>,
}

impl<M> UnixDatagram<M>
where
    M: Mode,
{
    /// Creates a datagram socket bound to `path`, on `runtime`.
    ///
    /// Binding creates a socket file at `path`. Dropping the socket does not remove the file, so
    /// remove it when you are done with the socket.
    ///
    /// # Errors
    ///
    /// Fails with [`AddrInUse`](io::ErrorKind::AddrInUse) if `path` already has a file. This
    /// includes a stale socket file left by an earlier socket, which the caller must remove first.
    /// Fails with [`InvalidInput`](io::ErrorKind::InvalidInput) if `path` is too long to name a
    /// socket (about a hundred bytes). Also fails for the reasons
    /// [`from_std`](UnixDatagram::from_std) fails.
    pub fn bind<P>(runtime: &Runtime<M>, path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        Self::from_std(runtime, std::os::unix::net::UnixDatagram::bind(path)?)
    }

    /// Creates a datagram socket on `runtime` that is bound to no path.
    ///
    /// Such a socket can send, to a path with [`send_to`](UnixDatagram::send_to) or to the socket
    /// it is [connected](UnixDatagram::connect) to. It has no address of its own, so nothing can
    /// send to it, and the socket that receives its datagrams sees the address they came from as
    /// unnamed.
    ///
    /// # Errors
    ///
    /// Fails if the system cannot create the socket, or for the reasons
    /// [`from_std`](UnixDatagram::from_std) fails.
    pub fn unbound(runtime: &Runtime<M>) -> io::Result<Self> {
        Self::from_std(runtime, std::os::unix::net::UnixDatagram::unbound()?)
    }

    /// Creates a connected pair of datagram sockets, on `runtime`.
    ///
    /// The datagrams one socket sends are received by the other, and the other way round. The
    /// sockets have no path to name them by.
    ///
    /// # Errors
    ///
    /// Fails if the system cannot create the pair, or for the reasons
    /// [`from_std`](UnixDatagram::from_std) fails.
    pub fn pair(runtime: &Runtime<M>) -> io::Result<(Self, Self)> {
        let (first, second) = std::os::unix::net::UnixDatagram::pair()?;

        Ok((
            Self::from_std(runtime, first)?,
            Self::from_std(runtime, second)?,
        ))
    }

    /// Creates a datagram socket on `runtime` from a std socket.
    ///
    /// `socket` is switched to non-blocking mode, as every socket of this type is. The mode belongs
    /// to the open socket, so a duplicate made with
    /// [`try_clone`](std::os::unix::net::UnixDatagram::try_clone) is switched too. A receive or
    /// send on that duplicate then fails with [`WouldBlock`](io::ErrorKind::WouldBlock) instead of
    /// waiting.
    ///
    /// On Apple's platforms the socket also gets `SO_NOSIGPIPE`, so that a send to a peer that has
    /// gone fails instead of raising `SIGPIPE`.
    ///
    /// # Errors
    ///
    /// Fails if switching to non-blocking mode fails, if setting `SO_NOSIGPIPE` fails (on Apple's
    /// platforms), or if the runtime cannot start watching the socket.
    pub fn from_std(
        runtime: &Runtime<M>,
        socket: std::os::unix::net::UnixDatagram,
    ) -> io::Result<Self> {
        socket.set_nonblocking(true)?;
        set_nosigpipe(&socket)?;

        Ok(Self {
            io: AsyncIo::from_nonblocking(runtime, socket)?,
        })
    }

    /// Connects the socket to the socket bound to `path`.
    ///
    /// From then on [`send`](UnixDatagram::send) sends to that socket, and
    /// [`recv`](UnixDatagram::recv) and [`recv_from`](UnixDatagram::recv_from) take only the
    /// datagrams it sends. A datagram that another socket sent before the connect may still be
    /// queued, and is received like any other.
    ///
    /// Datagram sockets have no connection to wait for, so this does not block. The socket can be
    /// connected again, to another socket.
    ///
    /// # Errors
    ///
    /// Fails if `path` names no socket that is bound.
    pub fn connect<P>(&self, path: P) -> io::Result<()>
    where
        P: AsRef<Path>,
    {
        self.io.get_ref().connect(path)
    }

    /// Sends `buf` as a datagram to the socket bound to `path`.
    ///
    /// Returns the number of bytes sent.
    ///
    /// If the system refuses the datagram with [`WouldBlock`](io::ErrorKind::WouldBlock), the send
    /// is tried again every 20 milliseconds, on the runtime's timer and without blocking the
    /// thread. Linux does this when the receiving socket has no room. Only the caller bounds this
    /// wait, with a timeout or by dropping the future. In contrast, [`send`](UnixDatagram::send) on
    /// a [connected](UnixDatagram::connect) socket waits for room and sends as soon as there is
    /// some.
    ///
    /// # Errors
    ///
    /// Fails with [`NotFound`](io::ErrorKind::NotFound) if there is no file at `path`, and with
    /// [`ConnectionRefused`](io::ErrorKind::ConnectionRefused) if the file is a socket that is
    /// gone.
    ///
    /// # Cancel safety
    ///
    /// Dropping the future before it completes gives up the send.
    pub async fn send_to<P>(&self, buf: &[u8], path: P) -> io::Result<usize>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        // Not a readiness wait: see `SEND_TO_INTERVAL`. For that reason the send does not go
        // through `AsyncIo`, which counts the operations it runs against the budget of the poll
        // (see the `coop` module), so this counts its own.
        loop {
            poll_fn(coop::poll_proceed).await;
            match self.io.get_ref().send_to(buf, path) {
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    self.io.runtime().sleep(SEND_TO_INTERVAL).await;
                }
                result => {
                    coop::spend();

                    return result;
                }
            }
        }
    }

    /// Waits for a datagram to arrive, and receives it into `buf`.
    ///
    /// Returns the number of bytes received and the address of the socket that sent the datagram.
    /// The address is unnamed unless that socket is bound to a path. A datagram longer than `buf`
    /// is cut short, and the rest of it is discarded.
    ///
    /// # Cancel safety
    ///
    /// Dropping the future before it completes gives up the wait. No datagram is lost: a datagram
    /// that arrives meanwhile stays queued for the next receive.
    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.io.read_with(|socket| socket.recv_from(buf)).await
    }

    /// Waits until there is room to send, and sends `buf` as a datagram to the socket this one is
    /// connected to.
    ///
    /// Returns the number of bytes sent.
    ///
    /// # Errors
    ///
    /// Fails if the socket has no peer to send to. Connect it first with
    /// [`connect`](UnixDatagram::connect), or create it with [`pair`](UnixDatagram::pair).
    pub async fn send(&self, buf: &[u8]) -> io::Result<usize> {
        // Not std's `send`: that is a plain `write(2)`, which raises `SIGPIPE` on the BSDs and on
        // Apple's platforms after a `shutdown` of the write half. `send(2)` takes `MSG_NOSIGNAL`
        // where there is one, as a stream's writes do (see `SEND_FLAGS`). `send_to` stays std's,
        // which sends with that flag where there is one.
        self.io
            .write_with(|socket| Ok(rustix::net::send(socket, buf, SEND_FLAGS)?))
            .await
    }

    /// Waits for a datagram to arrive, and receives it into `buf`, without the address it came
    /// from.
    ///
    /// Returns the number of bytes received. This is [`recv_from`](UnixDatagram::recv_from) for a
    /// socket that is connected, to a path with [`connect`](UnixDatagram::connect) or as one of a
    /// [pair](UnixDatagram::pair). Such a socket takes only its peer's datagrams from then on, so
    /// it has no use for the sender's address. A datagram that another socket sent before the
    /// connect may still be queued, and is received all the same, without saying which socket sent
    /// it. A datagram longer than `buf` is cut short, and the rest of it is discarded.
    ///
    /// # Cancel safety
    ///
    /// As for [`recv_from`](UnixDatagram::recv_from): dropping the future gives up the wait and
    /// loses no datagram.
    pub async fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.io.read_with(|socket| socket.recv(buf)).await
    }

    /// The socket address this socket is bound to.
    ///
    /// This is the path the socket is bound to, which [`SocketAddr::as_pathname`] returns, or an
    /// unnamed address if it is bound to none.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.io.get_ref().local_addr()
    }

    /// The socket address of the socket this one is connected to.
    ///
    /// The address is unnamed if that socket is bound to no path.
    ///
    /// # Errors
    ///
    /// Fails with [`NotConnected`](io::ErrorKind::NotConnected) if the socket is not connected.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.io.get_ref().peer_addr()
    }

    /// Shuts down the receiving half, the sending half or both halves of the socket, as `how` says.
    ///
    /// A send after the sending half is shut down fails with
    /// [`BrokenPipe`](io::ErrorKind::BrokenPipe).
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.io.get_ref().shutdown(how)
    }
}

/// How long a `send_to` that the receiver has no room for waits before it tries again.
///
/// The wait is on the timer, and not for the socket to become writable as a `send`'s is, because
/// the socket's readiness says nothing here. Linux reports a socket that sends to an address of
/// its choosing as writable whether or not the receiver has room (`unix_dgram_poll` clears
/// writability only for a connected peer), and arms no wake-up for the receiver having room either
/// (`unix_dgram_sendmsg` does so only for a connected peer). So a wait for writability returns at
/// once, and the send would be tried over and over, a core spinning, for as long as the receiver's
/// queue stays full.
const SEND_TO_INTERVAL: Duration = Duration::from_millis(20);

impl<M> fmt::Debug for UnixDatagram<M>
where
    M: Mode,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.io.get_ref(), f)
    }
}

impl<M> AsFd for UnixDatagram<M>
where
    M: Mode,
{
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.io.get_ref().as_fd()
    }
}

impl<M> AsRawFd for UnixDatagram<M>
where
    M: Mode,
{
    fn as_raw_fd(&self) -> RawFd {
        self.io.get_ref().as_raw_fd()
    }
}

/// A stream of the connections a [`UnixListener`] accepts, created by [`UnixListener::incoming`].
///
/// Each item is the [`UnixStream`] of a connection, on the listener's runtime, or the error of a
/// failed accept. The stream never ends. It is pending while no connection waits, and yields the
/// next connection when it arrives. It implements the [`Stream`] trait of [`futures-core`], so the
/// extension traits of [`futures`] work on it.
///
/// An error does not end the stream, and the stream does not stay pending until the next connection
/// arrives: polled again, it accepts again. The errors are those of
/// [`accept`](UnixListener::accept), which also describes what they leave behind. Some leave the
/// connection queued, so the next item is the same error at once. A loop that keeps taking items
/// after an error then spins the thread, so back off first, for example with [`Runtime::sleep`].
///
/// Only one task at a time can wait for the next item, counting every `Incoming` of the listener.
/// If a second task waits as well, it replaces the first, which is then never woken. Tasks that
/// take items from one listener's streams must take turns, for example behind a lock. Tasks waiting
/// in [`accept`](UnixListener::accept) do not count against this limit, nor the other way round.
///
/// [`futures-core`]: https://docs.rs/futures-core
/// [`futures`]: https://docs.rs/futures
pub struct Incoming<'a, M = Local>
where
    M: Mode,
{
    listener: &'a UnixListener<M>,
}

impl<M> Stream for Incoming<'_, M>
where
    M: Mode,
{
    type Item = io::Result<UnixStream<M>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.listener
            .poll_accept(cx)
            .map(|accepted| Some(accepted.map(|(stream, _address)| stream)))
    }
}

impl<M> fmt::Debug for Incoming<'_, M>
where
    M: Mode,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Incoming")
            .field("listener", &self.listener)
            .finish()
    }
}

impl<M> UnixListener<M>
where
    M: Mode,
{
    /// Accepts a connection if one is waiting, and otherwise arranges for `cx`'s waker to be woken
    /// once one is.
    ///
    /// What `accept` and the `Incoming` stream both poll.
    fn poll_accept(&self, cx: &mut Context<'_>) -> Poll<io::Result<(UnixStream<M>, SocketAddr)>> {
        let (stream, address) = ready!(self.io.poll_read_with(cx, |listener| listener.accept()))?;

        Poll::Ready(self.accepted(stream, address))
    }

    /// The stream of a connection the std listener accepted, on this listener's runtime, with the
    /// address of its peer.
    fn accepted(
        &self,
        stream: std::os::unix::net::UnixStream,
        address: SocketAddr,
    ) -> io::Result<(UnixStream<M>, SocketAddr)> {
        // std's `accept` leaves the accepted socket in blocking mode on Linux, which `from_std`
        // sets right.
        UnixStream::from_std(&self.io.runtime(), stream).map(|stream| (stream, address))
    }
}

impl<M> UnixStream<M>
where
    M: Mode,
{
    /// A stream connected to the socket at `address`, on `runtime`.
    ///
    /// What `connect` and `connect_addr` both end in, so that the wait for room in a full backlog
    /// is the same for either.
    async fn connect_sockaddr(runtime: &Runtime<M>, address: &SockAddr) -> io::Result<Self> {
        let io = connect::connect_unix(runtime, address).await?;

        Ok(Self { io })
    }
}

/// The address of the socket that `address` names, as the system takes it to connect to.
///
/// A path is taken as `connect` takes one. A name in the abstract namespace, which only Linux and
/// Android have, is taken without the checks of a path, and `SockAddr::unix` takes it for what it
/// is by the zero byte it leads with. An unnamed address names nothing to connect to.
fn named_sockaddr(address: &SocketAddr) -> io::Result<SockAddr> {
    if let Some(path) = address.as_pathname() {
        return pathname_sockaddr(path);
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    if let Some(name) = address.as_abstract_name() {
        // The zero byte in front is what makes the name abstract, and the name follows it as it
        // is, a zero byte inside included: socket2 counts the whole of it into the address's
        // length, with no terminator after it, as an abstract name has none.
        let path = [&[0u8][..], name].concat();

        return SockAddr::unix(std::ffi::OsStr::from_bytes(&path));
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "an unnamed address names no socket to connect to",
    ))
}

/// The address of the socket file at `path`, as the system takes it to connect to.
///
/// `SockAddr::unix` takes a zero byte as it comes: one that leads the path names a socket in
/// Linux's abstract namespace instead, and one inside it cuts the path short. Neither is what a
/// path means anywhere else, so a path with a zero byte in it is refused, as std refuses it.
fn pathname_sockaddr(path: &Path) -> io::Result<SockAddr> {
    if path.as_os_str().as_bytes().contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "paths must not contain interior null bytes",
        ));
    }

    SockAddr::unix(path)
}

/// The flags every write to a stream carries.
///
/// `MSG_NOSIGNAL` makes a write to a peer that has gone away an error rather than a signal.
/// Apple's platforms and Redox have no such flag, so a write there goes without it.
#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "tvos",
    target_os = "visionos",
    target_os = "watchos",
    target_os = "redox"
)))]
const SEND_FLAGS: rustix::net::SendFlags = rustix::net::SendFlags::NOSIGNAL;
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "tvos",
    target_os = "visionos",
    target_os = "watchos",
    target_os = "redox"
))]
const SEND_FLAGS: rustix::net::SendFlags = rustix::net::SendFlags::empty();
