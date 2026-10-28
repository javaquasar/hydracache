//! The TCP sockets: [`TcpListener`], [`TcpStream`], and the [`Incoming`] stream of the connections
//! a listener accepts.

#[cfg(any(unix, target_os = "wasi"))]
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, RawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawSocket, AsSocket, BorrowedSocket, RawSocket};
use std::{
    fmt,
    io::{self, Read, Write},
    net::{Shutdown, SocketAddr},
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll, ready},
};

use futures_core::Stream;
use futures_io::{AsyncRead, AsyncWrite};

use super::set_nosigpipe;
use crate::{AsyncIo, Local, Mode, Runtime};

/// A TCP socket server, listening for connections.
///
/// Create a listener with [`TcpListener::bind`]. Then call [`accept`](TcpListener::accept) to take
/// one connection, or [`incoming`](TcpListener::incoming) for a stream of connections. Each
/// connection is a [`TcpStream`]. The listener is the async counterpart of
/// [`std::net::TcpListener`]: waiting for a connection lets other tasks run instead of blocking the
/// thread.
///
/// The listener runs on the runtime passed to its constructor, and so do the streams it accepts.
/// Its type carries that runtime's flavour, as the [module documentation](super) explains: a
/// `TcpListener<Local>` (the default) stays on the thread that made it, and a `TcpListener<Shared>`
/// can be sent to and used from any thread.
///
/// Any number of tasks can wait in [`accept`](TcpListener::accept) at once through a reference to
/// the listener, and each connection goes to one of them. Only one task at a time can wait for the
/// next item of an [`Incoming`] stream, counting all the streams of the listener. Tasks waiting in
/// `accept` do not count.
///
/// # Example
///
/// A listener accepts a connection and reads a greeting from it. A client makes the connection and
/// sends the greeting.
///
/// ```
/// use std::net::Ipv4Addr;
///
/// use futures::{AsyncReadExt, AsyncWriteExt};
/// use zruntime::{
///     LocalRuntime,
///     net::{TcpListener, TcpStream},
/// };
///
/// let runtime = LocalRuntime::new()?;
/// // Port `0` lets the system pick a free port.
/// let listener = TcpListener::bind(&runtime, (Ipv4Addr::LOCALHOST, 0))?;
/// let address = listener.local_addr()?;
///
/// runtime.block_on(async {
///     let mut client = TcpStream::connect(&runtime, address).await?;
///     client.write_all(b"hello").await?;
///
///     let (mut server, peer) = listener.accept().await?;
///     let mut greeting = [0; 5];
///     server.read_exact(&mut greeting).await?;
///
///     assert_eq!(&greeting, b"hello");
///     assert_eq!(peer, client.local_addr()?);
///     # Ok::<_, std::io::Error>(())
/// })?;
/// # Ok::<_, std::io::Error>(())
/// ```
pub struct TcpListener<M = Local>
where
    M: Mode,
{
    io: AsyncIo<std::net::TcpListener, M>,
}

impl<M> TcpListener<M>
where
    M: Mode,
{
    /// Creates a listener bound to `addr`, on `runtime`.
    ///
    /// `addr` is a socket address: a [`SocketAddr`], or anything that converts into one, such as a
    /// pair of an [`Ipv4Addr`](std::net::Ipv4Addr) and a port. It is never a host name; the
    /// [module documentation](super) explains what to do instead. Binding to port `0` lets the
    /// system pick a free port, which [`local_addr`](TcpListener::local_addr) returns.
    ///
    /// The listener is bound and listening when this returns.
    ///
    /// # Errors
    ///
    /// Fails if the socket cannot be bound to `addr`, or for the reasons
    /// [`from_std`](TcpListener::from_std) fails.
    ///
    /// On `wasm32-wasip1` it always fails with [`Unsupported`](io::ErrorKind::Unsupported): that
    /// target cannot create a socket (see the [module documentation](super)).
    /// [`from_std`](TcpListener::from_std) takes a listener that the host handed in instead.
    pub fn bind<A>(runtime: &Runtime<M>, addr: A) -> io::Result<Self>
    where
        A: Into<SocketAddr>,
    {
        Self::from_std(runtime, std::net::TcpListener::bind(addr.into())?)
    }

    /// Creates a listener on `runtime` from a std listener.
    ///
    /// `listener` is switched to non-blocking mode, as every listener of this type is. On unix, the
    /// mode belongs to the open socket, so a duplicate made with
    /// [`try_clone`](std::net::TcpListener::try_clone) is switched too. An `accept` on that
    /// duplicate then fails with [`WouldBlock`](io::ErrorKind::WouldBlock) instead of waiting.
    ///
    /// # Errors
    ///
    /// Fails if switching to non-blocking mode fails, or if the runtime cannot start watching the
    /// socket. On Windows a runtime watches a limited number of sockets; see the
    /// [module documentation](super).
    pub fn from_std(runtime: &Runtime<M>, listener: std::net::TcpListener) -> io::Result<Self> {
        listener.set_nonblocking(true)?;

        Ok(Self {
            io: AsyncIo::from_nonblocking(runtime, listener)?,
        })
    }

    /// Waits for a connection to this listener, and accepts it.
    ///
    /// Returns the stream of the connection and the address of the peer that made it. The stream
    /// runs on the listener's runtime and is in non-blocking mode.
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
    /// On Windows a runtime watches a limited number of sockets (see the
    /// [module documentation](super)). An accept that would exceed the limit fails, and the
    /// connection it took off the queue is closed. Its peer sees the connection end.
    ///
    /// # Cancel safety
    ///
    /// Dropping the future before it completes gives up the wait. No connection is lost: a
    /// connection that arrives meanwhile stays queued for the next call.
    pub async fn accept(&self) -> io::Result<(TcpStream<M>, SocketAddr)> {
        let (stream, address) = self.io.read_with(|listener| listener.accept()).await?;

        self.accepted(stream, address)
    }

    /// Returns a stream of the connections made to this listener.
    ///
    /// Each item is the [`TcpStream`] of a connection, accepted as [`accept`](TcpListener::accept)
    /// accepts it, or the error of a failed accept. See [`Incoming`] for how errors and waiting
    /// tasks behave.
    ///
    /// # Example
    ///
    /// The stream yields the first two connections made to a listener, in the order they were made.
    ///
    /// ```
    /// use std::net::Ipv4Addr;
    ///
    /// use futures::StreamExt;
    /// use zruntime::{
    ///     LocalRuntime,
    ///     net::{TcpListener, TcpStream},
    /// };
    ///
    /// let runtime = LocalRuntime::new()?;
    /// let listener = TcpListener::bind(&runtime, (Ipv4Addr::LOCALHOST, 0))?;
    /// let address = listener.local_addr()?;
    ///
    /// runtime.block_on(async {
    ///     let first = TcpStream::connect(&runtime, address).await?;
    ///     let second = TcpStream::connect(&runtime, address).await?;
    ///
    ///     let mut incoming = listener.incoming();
    ///     let first_accepted = incoming.next().await.expect("the stream never ends")?;
    ///     let second_accepted = incoming.next().await.expect("the stream never ends")?;
    ///
    ///     assert_eq!(first_accepted.peer_addr()?, first.local_addr()?);
    ///     assert_eq!(second_accepted.peer_addr()?, second.local_addr()?);
    ///     # Ok::<_, std::io::Error>(())
    /// })?;
    /// # Ok::<_, std::io::Error>(())
    /// ```
    pub fn incoming(&self) -> Incoming<'_, M> {
        Incoming { listener: self }
    }

    /// The socket address this listener is bound to.
    ///
    /// After binding to port `0`, this returns the port the system picked.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.io.get_ref().local_addr()
    }

    /// The `IP_TTL` option of this listener's socket.
    ///
    /// This is the time-to-live field of the IP packets sent from the socket. See
    /// [`set_ttl`](TcpListener::set_ttl).
    pub fn ttl(&self) -> io::Result<u32> {
        self.io.get_ref().ttl()
    }

    /// Sets the `IP_TTL` option of this listener's socket.
    ///
    /// This is the time-to-live field of the IP packets sent from the socket.
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.io.get_ref().set_ttl(ttl)
    }
}

impl<M> fmt::Debug for TcpListener<M>
where
    M: Mode,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.io.get_ref(), f)
    }
}

#[cfg(any(unix, target_os = "wasi"))]
impl<M> AsFd for TcpListener<M>
where
    M: Mode,
{
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.io.get_ref().as_fd()
    }
}

#[cfg(any(unix, target_os = "wasi"))]
impl<M> AsRawFd for TcpListener<M>
where
    M: Mode,
{
    fn as_raw_fd(&self) -> RawFd {
        self.io.get_ref().as_raw_fd()
    }
}

#[cfg(windows)]
impl<M> AsSocket for TcpListener<M>
where
    M: Mode,
{
    fn as_socket(&self) -> BorrowedSocket<'_> {
        self.io.get_ref().as_socket()
    }
}

#[cfg(windows)]
impl<M> AsRawSocket for TcpListener<M>
where
    M: Mode,
{
    fn as_raw_socket(&self) -> RawSocket {
        self.io.get_ref().as_raw_socket()
    }
}

/// A TCP connection, to read from and write to.
///
/// Create a stream with [`TcpStream::connect`], or get one from [`TcpListener::accept`]. The stream
/// is the async counterpart of [`std::net::TcpStream`]: a read or write that has to wait lets other
/// tasks run instead of blocking the thread.
///
/// The stream implements the `AsyncRead` and `AsyncWrite` traits of [`futures-io`], so the
/// extension traits of [`futures`] work on it. So does `&TcpStream`, which lets a reader and a
/// writer share one stream, as std's `Read` and `Write` do for a `&std::net::TcpStream`. A write
/// sends as much as the kernel takes at once, which may be less than the whole buffer. Nothing is
/// buffered, so flushing does nothing.
///
/// Closing the stream (for example with the `close` method of `AsyncWriteExt`) shuts down the write
/// half of the socket, like [`shutdown`](TcpStream::shutdown) with [`Shutdown::Write`]. The peer
/// reads the end of the stream once it has read everything sent. The read half stays open. It is
/// the socket that is shut down, so closing the stream through one `&TcpStream` ends it for every
/// handle on it. Closing a stream twice is fine on every platform: the second close does not shut
/// down the write half again, so the read half stays open. The same holds if the write half was
/// already shut down with `shutdown`.
///
/// The stream runs on the runtime passed to its constructor. A stream accepted by a [`TcpListener`]
/// runs on the listener's runtime. The stream's type carries that runtime's flavour, as the
/// [module documentation](super) explains: a `TcpStream<Local>` (the default) stays on the thread
/// that made it, and a `TcpStream<Shared>` can be sent to and used from any thread.
///
/// Only one task at a time can wait to read through the `AsyncRead` implementation, and only one to
/// write through `AsyncWrite`, counting all references to the stream. If a second task waits in the
/// same direction, it replaces the first, which is then never woken. Tasks that read, or write,
/// together must take turns, for example behind a lock. [`peek`](TcpStream::peek) has no such
/// limit: any number of tasks can wait in it at once, and it never replaces a task waiting to read,
/// nor the other way round.
///
/// # Example
///
/// A client sends a message and closes its end of the stream. A server reads the message up to the
/// end of the stream. The server can still send an answer to the client.
///
/// ```
/// use std::net::Ipv4Addr;
///
/// use futures::{AsyncReadExt, AsyncWriteExt};
/// use zruntime::{
///     LocalRuntime,
///     net::{TcpListener, TcpStream},
/// };
///
/// let runtime = LocalRuntime::new()?;
/// let listener = TcpListener::bind(&runtime, (Ipv4Addr::LOCALHOST, 0))?;
///
/// runtime.block_on(async {
///     let mut client = TcpStream::connect(&runtime, listener.local_addr()?).await?;
///     let (mut server, _) = listener.accept().await?;
///
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
pub struct TcpStream<M = Local>
where
    M: Mode,
{
    io: AsyncIo<std::net::TcpStream, M>,
    /// Whether the write half of the socket is shut down already, by a close or by a `shutdown`
    /// of it, so that a close finding it so shuts nothing down again.
    ///
    /// A second shutdown of the write half is harmless on most platforms, but not on FreeBSD and
    /// NetBSD. Once the peer has acknowledged the first, the connection is in the state
    /// `FIN_WAIT_2`, and there `tcp_usrclosed` takes the socket for disconnected, as it does in
    /// every state from `FIN_WAIT_2` on (it calls `soisdisconnected`), which ends the read half
    /// too. Linux and OpenBSD ignore the second shutdown, and macOS answers it with `ENOTCONN`,
    /// which a close reads as success. So closing a stream that is closed already is fine, as the
    /// docs promise, only where the close does not reach the system again.
    ///
    /// The flag is set before the system is asked, and it stays set whatever the answer is: a
    /// shutdown that fails may have shut the write half down all the same. On FreeBSD and macOS
    /// the error comes from sending the FIN (`tcp_output`), after the half is marked shut, so
    /// clearing the flag on an error would allow exactly the second shutdown the flag is there to
    /// stop. Setting it first also leaves no gap, between the call and the flag, for a close
    /// through another handle to slip into.
    ///
    /// An atomic because a close goes through a shared reference, and a `TcpStream<Shared>` is
    /// `Sync`. `Relaxed` is enough: nothing is published through the flag, which only decides
    /// whether a call is made to the system.
    write_shut: AtomicBool,
}

impl<M> TcpStream<M>
where
    M: Mode,
{
    /// Creates a stream connected to `addr`, on `runtime`.
    ///
    /// `addr` is a socket address: a [`SocketAddr`], or anything that converts into one, such as a
    /// pair of an [`Ipv4Addr`](std::net::Ipv4Addr) and a port. It is never a host name; the
    /// [module documentation](super) explains what to do instead.
    ///
    /// The connection does not block the thread. The future waits until the runtime reports that
    /// the connection is made or has failed.
    ///
    /// # Errors
    ///
    /// Returns the error of the connection attempt. On Windows a runtime watches a limited number
    /// of sockets (see the [module documentation](super)), and a connect that would exceed the
    /// limit fails.
    ///
    /// On `wasm32-wasip1` it always fails with [`Unsupported`](io::ErrorKind::Unsupported): that
    /// target cannot create a socket (see the [module documentation](super)).
    /// [`from_std`](TcpStream::from_std) takes a stream that the host handed in instead.
    ///
    /// # Cancel safety
    ///
    /// Dropping the future before it completes gives up the connection attempt.
    pub async fn connect<A>(runtime: &Runtime<M>, addr: A) -> io::Result<Self>
    where
        A: Into<SocketAddr>,
    {
        let io = connect_io(runtime, addr.into()).await?;

        Ok(Self {
            io,
            write_shut: AtomicBool::new(false),
        })
    }

    /// Creates a stream on `runtime` from a std stream.
    ///
    /// `stream` is switched to non-blocking mode, as every stream of this type is. On unix, the
    /// mode belongs to the open socket, so a duplicate made with
    /// [`try_clone`](std::net::TcpStream::try_clone) is switched too. A read or write on that
    /// duplicate then fails with [`WouldBlock`](io::ErrorKind::WouldBlock) instead of waiting.
    ///
    /// On Apple's platforms the socket also gets `SO_NOSIGPIPE`, so that a write to a peer that has
    /// gone fails instead of raising `SIGPIPE`.
    ///
    /// # Errors
    ///
    /// Fails if switching to non-blocking mode fails, if setting `SO_NOSIGPIPE` fails (on Apple's
    /// platforms), or if the runtime cannot start watching the socket. On Windows a runtime watches
    /// a limited number of sockets; see the [module documentation](super).
    pub fn from_std(runtime: &Runtime<M>, stream: std::net::TcpStream) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        set_nosigpipe(&stream)?;

        Ok(Self {
            io: AsyncIo::from_nonblocking(runtime, stream)?,
            write_shut: AtomicBool::new(false),
        })
    }

    /// The socket address of this end of the connection.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.io.get_ref().local_addr()
    }

    /// The socket address of the peer: the other end of the connection.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.io.get_ref().peer_addr()
    }

    /// Stops watching the stream, and returns the socket as a std stream, still in non-blocking
    /// mode.
    ///
    /// The connection is left as it was: the std stream can read the bytes that arrived but were
    /// not read yet. The socket stays in non-blocking mode, so a read or write on the std stream
    /// fails with [`WouldBlock`](io::ErrorKind::WouldBlock) instead of waiting, until
    /// [`set_nonblocking`](std::net::TcpStream::set_nonblocking) switches the mode back.
    ///
    /// A stream created from the socket again with [`from_std`](TcpStream::from_std) does not know
    /// that its write half was shut down before; see [`shutdown`](TcpStream::shutdown).
    ///
    /// On a shared runtime this may wait briefly, until a thread driving the runtime returns from a
    /// wait that was watching the socket.
    pub fn into_std(self) -> std::net::TcpStream {
        self.io.into_inner()
    }

    /// Shuts down the read half, the write half or both halves of the connection, as `how` says.
    ///
    /// Shutting down the write half makes the peer read the end of the stream once it has read
    /// everything sent, as closing the stream does. The socket is shut down, so this affects every
    /// handle on the stream.
    ///
    /// Closing the stream after its write half was shut down here does nothing more. A second
    /// `shutdown` of the write half does reach the system, unlike a second close. That is harmless
    /// on most platforms, but on FreeBSD and NetBSD it also ends the read half once the peer has
    /// acknowledged the first. A stream created with [`from_std`](TcpStream::from_std) from a
    /// socket whose write half was shut down through std does not know that, so closing it shuts
    /// the write half down again.
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        if matches!(how, Shutdown::Write | Shutdown::Both) {
            // Noted before the system is asked, whatever it answers: see `write_shut`. Nothing
            // else is done with the flag here: a second `shutdown` goes to the system as it is.
            self.write_shut.store(true, Ordering::Relaxed);
        }

        self.io.get_ref().shutdown(how)
    }

    /// Waits for bytes to arrive, and copies them into `buf` without removing them from the stream.
    ///
    /// Returns the number of bytes copied. It is zero only if `buf` is empty or the peer has closed
    /// its end and nothing is left to read. The next read or `peek` returns the same bytes again.
    ///
    /// Any number of tasks can wait in `peek` at once, alongside the one task that waits to read;
    /// see [the stream's documentation](TcpStream).
    pub async fn peek(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.io.read_with(|stream| stream.peek(buf)).await
    }

    /// The `TCP_NODELAY` option of this stream's socket.
    ///
    /// See [`set_nodelay`](TcpStream::set_nodelay).
    pub fn nodelay(&self) -> io::Result<bool> {
        self.io.get_ref().nodelay()
    }

    /// Sets the `TCP_NODELAY` option of this stream's socket.
    ///
    /// With the option set, a write is sent as soon as it is made, instead of being held back to be
    /// sent together with later writes (Nagle's algorithm).
    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        self.io.get_ref().set_nodelay(nodelay)
    }

    /// The `IP_TTL` option of this stream's socket.
    ///
    /// This is the time-to-live field of the IP packets sent from the socket. See
    /// [`set_ttl`](TcpStream::set_ttl).
    pub fn ttl(&self) -> io::Result<u32> {
        self.io.get_ref().ttl()
    }

    /// Sets the `IP_TTL` option of this stream's socket.
    ///
    /// This is the time-to-live field of the IP packets sent from the socket.
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.io.get_ref().set_ttl(ttl)
    }
}

impl<M> AsyncRead for TcpStream<M>
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

impl<M> AsyncWrite for TcpStream<M>
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

impl<M> AsyncRead for &TcpStream<M>
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

// The vectored methods are left to the traits' defaults, which write the first buffer that is not
// empty: std's vectored write sends without `MSG_NOSIGNAL`, which its plain one sends with.
impl<M> AsyncWrite for &TcpStream<M>
where
    M: Mode,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.io.poll_write_with(cx, |mut stream| stream.write(buf))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // Nothing is buffered here: a write that reports success has gone to the kernel.
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // The write half is shut down once: see `write_shut` for what a second shutdown does on
        // some platforms, and for why the flag stays set where this one fails.
        if self.write_shut.swap(true, Ordering::Relaxed) {
            return Poll::Ready(Ok(()));
        }

        match self.io.get_ref().shutdown(Shutdown::Write) {
            // A stream whose connection is gone has nothing left to close: the system answers a
            // shutdown there with `NotConnected`, which means the stream is closed already, and
            // closing it is not an error, on any platform.
            Err(e) if e.kind() == io::ErrorKind::NotConnected => Poll::Ready(Ok(())),
            result => Poll::Ready(result),
        }
    }
}

impl<M> fmt::Debug for TcpStream<M>
where
    M: Mode,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.io.get_ref(), f)
    }
}

#[cfg(any(unix, target_os = "wasi"))]
impl<M> AsFd for TcpStream<M>
where
    M: Mode,
{
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.io.get_ref().as_fd()
    }
}

#[cfg(any(unix, target_os = "wasi"))]
impl<M> AsRawFd for TcpStream<M>
where
    M: Mode,
{
    fn as_raw_fd(&self) -> RawFd {
        self.io.get_ref().as_raw_fd()
    }
}

#[cfg(windows)]
impl<M> AsSocket for TcpStream<M>
where
    M: Mode,
{
    fn as_socket(&self) -> BorrowedSocket<'_> {
        self.io.get_ref().as_socket()
    }
}

#[cfg(windows)]
impl<M> AsRawSocket for TcpStream<M>
where
    M: Mode,
{
    fn as_raw_socket(&self) -> RawSocket {
        self.io.get_ref().as_raw_socket()
    }
}

/// A stream of the connections a [`TcpListener`] accepts, created by [`TcpListener::incoming`].
///
/// Each item is the [`TcpStream`] of a connection, on the listener's runtime, or the error of a
/// failed accept. The stream never ends. It is pending while no connection waits, and yields the
/// next connection when it arrives. It implements the [`Stream`] trait of [`futures-core`], so the
/// extension traits of [`futures`] work on it.
///
/// An error does not end the stream, and the stream does not stay pending until the next connection
/// arrives: polled again, it accepts again. The errors are those of
/// [`accept`](TcpListener::accept), which also describes what they leave behind. Some leave the
/// connection queued, so the next item is the same error at once. A loop that keeps taking items
/// after an error then spins the thread, so back off first, for example with [`Runtime::sleep`].
///
/// Only one task at a time can wait for the next item, counting every `Incoming` of the listener.
/// If a second task waits as well, it replaces the first, which is then never woken. Tasks that
/// take items from one listener's streams must take turns, for example behind a lock. Tasks waiting
/// in [`accept`](TcpListener::accept) do not count against this limit, nor the other way round.
///
/// [`futures-core`]: https://docs.rs/futures-core
/// [`futures`]: https://docs.rs/futures
pub struct Incoming<'a, M = Local>
where
    M: Mode,
{
    listener: &'a TcpListener<M>,
}

impl<M> Stream for Incoming<'_, M>
where
    M: Mode,
{
    type Item = io::Result<TcpStream<M>>;

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

impl<M> TcpListener<M>
where
    M: Mode,
{
    /// Accepts a connection if one is waiting, and otherwise arranges for `cx`'s waker to be woken
    /// once one is.
    ///
    /// What `accept` and the `Incoming` stream both poll.
    fn poll_accept(&self, cx: &mut Context<'_>) -> Poll<io::Result<(TcpStream<M>, SocketAddr)>> {
        let (stream, address) = ready!(self.io.poll_read_with(cx, |listener| listener.accept()))?;

        Poll::Ready(self.accepted(stream, address))
    }

    /// The stream of a connection the std listener accepted, on this listener's runtime, with the
    /// address of its peer.
    fn accepted(
        &self,
        stream: std::net::TcpStream,
        address: SocketAddr,
    ) -> io::Result<(TcpStream<M>, SocketAddr)> {
        // std's `accept` leaves the accepted socket in blocking mode on Linux, which `from_std`
        // sets right.
        TcpStream::from_std(&self.io.runtime(), stream).map(|stream| (stream, address))
    }
}

/// A connection to `address`, made and registered on `runtime`: what `TcpStream::connect` waits
/// for.
#[cfg(not(all(target_os = "wasi", target_env = "p1")))]
async fn connect_io<M>(
    runtime: &Runtime<M>,
    address: SocketAddr,
) -> io::Result<AsyncIo<std::net::TcpStream, M>>
where
    M: Mode,
{
    use socket2::{Domain, SockAddr};

    super::connect::connect(
        runtime,
        Domain::for_address(address),
        &SockAddr::from(address),
    )
    .await
}

/// What `connect_io` is on `wasm32-wasip1`, which cannot create a socket: an error, always.
///
/// socket2 does not build for that target, and std fails to make a socket there as well.
#[cfg(all(target_os = "wasi", target_env = "p1"))]
async fn connect_io<M>(
    _runtime: &Runtime<M>,
    _address: SocketAddr,
) -> io::Result<AsyncIo<std::net::TcpStream, M>>
where
    M: Mode,
{
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "wasm32-wasip1 cannot create a socket",
    ))
}
