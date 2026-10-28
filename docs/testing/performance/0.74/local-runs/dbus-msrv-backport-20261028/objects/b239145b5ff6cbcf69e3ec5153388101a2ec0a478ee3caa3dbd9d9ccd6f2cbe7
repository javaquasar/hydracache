//! [`AsyncIo`]: a non-blocking I/O handle that waits for its readiness on a runtime, and the I/O
//! traits it implements.

#[cfg(any(
    all(any(unix, target_os = "wasi"), any(feature = "tcp", feature = "udp")),
    all(unix, any(feature = "unix", feature = "process"))
))]
use std::os::fd::AsFd as AsSource;
#[cfg(any(unix, target_os = "wasi"))]
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, RawFd};
#[cfg(all(windows, any(feature = "tcp", feature = "udp")))]
use std::os::windows::io::AsSocket as AsSource;
#[cfg(windows)]
use std::os::windows::io::{AsRawSocket, AsSocket, BorrowedSocket, RawSocket};
use std::{
    fmt,
    future::poll_fn,
    io::{self, IoSlice, IoSliceMut, Read, Write},
    pin::Pin,
    task::{Context, Poll},
};

use futures_io::{AsyncRead, AsyncWrite};
#[cfg(windows)]
use windows_sys::Win32::Networking::WinSock::{
    FIONBIO, SOCKET, SOCKET_ERROR, WSAGetLastError, ioctlsocket,
};

use crate::{Interest, Local, Mode, Readiness, Registration, Runtime, Source, coop, mode, reactor};

/// An async I/O handle: a source that the runtime watches for readiness. Like `smol::Async`.
///
/// It runs I/O on the source without blocking the thread. An operation that would block waits for
/// the runtime to report the source ready instead, and the thread runs other tasks in the meantime.
///
/// On unix, the source can be anything with a file descriptor: a pipe, a terminal, an eventfd, an
/// inotify instance, the standard I/O of a child process, or a socket type that the `net` module
/// does not cover. On Windows, it can only be a socket, because that is all the runtime's `select`
/// can watch there. [`readable`](AsyncIo::readable) and [`writable`](AsyncIo::writable) only wait
/// for readiness, for a descriptor that another library does the I/O on.
///
/// The handle uses the runtime passed to its constructor, which watches the source from then on.
/// Its operations make progress while a thread is running [`Runtime::block_on`] on that runtime,
/// or, on a runtime from `SharedRuntime::current`, while the helper thread runs it. The type
/// carries the flavour of the runtime:
///
/// * On a [`LocalRuntime`](crate::LocalRuntime), it is an `AsyncIo<T, Local>` ([`Local`] is the
///   default), and stays on the thread that created it.
/// * On a [`SharedRuntime`](crate::SharedRuntime), it is an `AsyncIo<T, Shared>`, which can be sent
///   to and used from any thread. A shared runtime only watches a source that is `Send` and `Sync`,
///   as [`Source`] says.
///
/// # Waiting
///
/// Any number of tasks can wait at once, in either direction, through
/// [`readable`](AsyncIo::readable), [`writable`](AsyncIo::writable),
/// [`read_with`](AsyncIo::read_with) and [`write_with`](AsyncIo::write_with). Each needs only a
/// shared reference to the handle.
///
/// [`poll_read_with`](AsyncIo::poll_read_with), [`poll_write_with`](AsyncIo::poll_write_with), and
/// the `AsyncRead` and `AsyncWrite` impls built on them, keep only one waiting task per direction.
/// If a second task waits to read through them, for example, it replaces the first one, which is
/// then never woken. Tasks that share a direction through them must take turns, for example behind
/// a lock.
///
/// # I/O traits
///
/// `AsyncIo<T, M>` and `&AsyncIo<T, M>` implement the `AsyncRead` and `AsyncWrite` traits of
/// [`futures-io`], so the extension traits of [`futures`] work with them. The impls for a shared
/// reference let a reader and a writer share one handle. `AsyncRead` needs `&T` to implement
/// `Read`, and `AsyncWrite` needs `&T` to implement `Write`. That is the case for
/// `std::io::PipeReader` and `PipeWriter`, and for std's stream sockets, `TcpStream` and
/// `UnixStream`. Listeners and datagram sockets implement neither.
///
/// The runtime's reactor shares the source, so the handle never gives out a `&mut T`. A type that
/// only implements `Read` or `Write` for `&mut self`, such as `std::process::ChildStdout` and
/// `ChildStdin`, must be converted first. On unix, convert it through `OwnedFd`: a `ChildStdout`
/// into a `PipeReader`, and a `ChildStdin` into a `PipeWriter`. The pipes of children spawned with
/// `zruntime::process` are async already.
///
/// # Regular files
///
/// `AsyncIo` is not for regular files. On Linux and Android, the runtime cannot watch one, so
/// waiting on one fails. Elsewhere, its readiness says nothing of whether a read would wait for the
/// disk, and reading it blocks the thread, whatever its mode. `zruntime::Unblock` runs each
/// operation on a pool of threads instead, and `zruntime::fs` runs each on the thread that polls
/// it, which it blocks for as long as the operation takes.
///
/// # Example
///
/// The reading end of a pipe, read to its end while a thread writes to the other end:
///
/// ```
/// # #[cfg(unix)]
/// # fn main() -> std::io::Result<()> {
/// use std::{io::Write, thread};
///
/// use futures::AsyncReadExt;
/// use zruntime::{AsyncIo, LocalRuntime};
///
/// let runtime = LocalRuntime::new()?;
/// let (reader, mut writer) = std::io::pipe()?;
/// // The reading end is switched to non-blocking mode, and watched by the runtime from here on.
/// let mut reader = AsyncIo::new(&runtime, reader)?;
///
/// // The writer is dropped as the thread ends, which is the end of what the reader reads.
/// let writing = thread::spawn(move || writer.write_all(b"hello"));
///
/// let mut message = Vec::new();
/// runtime.block_on(reader.read_to_end(&mut message))?;
/// writing.join().expect("the writer does not panic")?;
///
/// assert_eq!(message, b"hello");
/// # Ok(())
/// # }
/// # #[cfg(not(unix))]
/// # fn main() {}
/// ```
///
/// [`futures-io`]: https://docs.rs/futures-io
/// [`futures`]: https://docs.rs/futures
pub struct AsyncIo<T, M = Local>
where
    M: Mode,
{
    // Fields drop in the order they are declared: the watch ends before this handle on the
    // source goes.
    registration: Registration<M>,
    io: M::Ptr<T>,
}

impl<T, M> AsyncIo<T, M>
where
    M: Mode,
{
    /// Creates an `AsyncIo` on `runtime` that does its I/O on `io`, and switches `io` to
    /// non-blocking mode.
    ///
    /// On unix, the mode belongs to the open file description, not to the descriptor. So every
    /// duplicate of the descriptor shares it, in this process or another. For example, if the
    /// standard input is a terminal, the shell that started the program shares it, and is left with
    /// it in non-blocking mode. If the mode must not change for others, set it yourself where that
    /// is safe, and use [`new_nonblocking`](AsyncIo::new_nonblocking).
    ///
    /// # Errors
    ///
    /// Fails if `io` cannot be switched to non-blocking mode, or if the runtime cannot watch it. A
    /// runtime watches each descriptor through one handle at a time, so this fails with
    /// [`AlreadyExists`](io::ErrorKind::AlreadyExists) if the runtime already watches the
    /// descriptor of `io`. On Windows, a runtime watches at most 1023 sockets at a time.
    pub fn new(runtime: &Runtime<M>, io: T) -> io::Result<Self>
    where
        T: Source<M>,
    {
        set_nonblocking(&io)?;

        Self::new_nonblocking(runtime, io)
    }

    /// Creates an `AsyncIo` on `runtime` that does its I/O on `io`, which must already be in
    /// non-blocking mode.
    ///
    /// Making sure of that is up to the caller. Each operation runs on the source right away, and
    /// only waits for readiness if it fails with [`WouldBlock`](io::ErrorKind::WouldBlock). On a
    /// blocking source, an operation with nothing ready blocks the thread instead, and every other
    /// task with it.
    ///
    /// # Errors
    ///
    /// Fails if the runtime cannot watch `io`. A runtime watches each descriptor through one handle
    /// at a time, so this fails with [`AlreadyExists`](io::ErrorKind::AlreadyExists) if the runtime
    /// already watches the descriptor of `io`. On Windows, a runtime watches at most 1023 sockets
    /// at a time.
    pub fn new_nonblocking(runtime: &Runtime<M>, io: T) -> io::Result<Self>
    where
        T: Source<M>,
    {
        let io = M::new_ptr(io);
        let registration = reactor::register::<M>(
            &runtime.core,
            <T as mode::sealed::IntoSource<M>>::source_ptr(io.clone()),
        )?;
        // Asked for once the source is in the reactor's map, so that a helper starting here takes
        // it into its very first wait.
        M::ensure_progress(&runtime.core);

        Ok(Self { registration, io })
    }

    /// The source.
    pub fn get_ref(&self) -> &T {
        &self.io
    }

    /// Stops watching the source, and returns it, still in non-blocking mode.
    ///
    /// On a shared runtime, this may block for a moment, until a thread running the runtime returns
    /// from a wait that watches the source.
    pub fn into_inner(self) -> T {
        let Self { registration, io } = self;
        // The watch ends first: nothing the reactor holds is left to share the source then, but
        // for a wait under way on another thread, which this lets go of by breaking it.
        drop(registration);

        M::into_inner(io)
    }

    /// Waits until the source is readable, without running any operation on it.
    ///
    /// Any number of tasks can wait at once. Readiness is only a hint: another task may take the
    /// bytes first. So an operation run after the wait must still expect
    /// [`WouldBlock`](io::ErrorKind::WouldBlock), and wait again when it gets one, as
    /// [`read_with`](AsyncIo::read_with) does. See [`Registration::ready`] for details.
    ///
    /// # Errors
    ///
    /// The wait fails if the OS refuses to watch the source.
    pub fn readable(&self) -> Readiness<'_, M> {
        self.registration.ready(Interest::Readable)
    }

    /// Waits until the source is writable, without running any operation on it.
    ///
    /// Any number of tasks can wait at once. Readiness is only a hint: another task may take the
    /// room first. So an operation run after the wait must still expect
    /// [`WouldBlock`](io::ErrorKind::WouldBlock), and wait again when it gets one, as
    /// [`write_with`](AsyncIo::write_with) does. See [`Registration::ready`] for details.
    ///
    /// # Errors
    ///
    /// The wait fails if the OS refuses to watch the source.
    pub fn writable(&self) -> Readiness<'_, M> {
        self.registration.ready(Interest::Writable)
    }

    /// Runs `operation` on the source until it no longer fails with
    /// [`WouldBlock`](io::ErrorKind::WouldBlock), waiting for the source to be readable in between.
    ///
    /// Returns the first success of `operation`, or its first error other than `WouldBlock`. Also
    /// fails if a wait for readiness fails, as for [`readable`](AsyncIo::readable). A call that
    /// fails with [`Interrupted`](io::ErrorKind::Interrupted) is retried right away.
    ///
    /// `operation` must not block. It runs on the thread that all tasks of the runtime share.
    ///
    /// Any number of tasks can do this at once on one handle.
    ///
    /// Before it runs `operation`, this may yield to the other tasks, as
    /// [`Registration::poll_io`] describes, if the poll it is in has completed its share of I/O
    /// operations already.
    ///
    /// # Cancel safety
    ///
    /// Dropping the future gives up the wait, or the yield before an operation, and never leaves an
    /// operation half done: each call of `operation` runs to its end before the future can be
    /// dropped.
    pub async fn read_with<R>(&self, operation: impl FnMut(&T) -> io::Result<R>) -> io::Result<R> {
        self.run_with(Interest::Readable, operation).await
    }

    /// Runs `operation` on the source until it no longer fails with
    /// [`WouldBlock`](io::ErrorKind::WouldBlock), waiting for the source to be writable in between.
    ///
    /// Returns the first success of `operation`, a partial write included, or its first error other
    /// than `WouldBlock`. Also fails if a wait for readiness fails, as for
    /// [`writable`](AsyncIo::writable). A call that fails with
    /// [`Interrupted`](io::ErrorKind::Interrupted) is retried right away.
    ///
    /// `operation` must not block. It runs on the thread that all tasks of the runtime share.
    ///
    /// Any number of tasks can do this at once on one handle.
    ///
    /// Before it runs `operation`, this may yield to the other tasks, as
    /// [`Registration::poll_io`] describes, if the poll it is in has completed its share of I/O
    /// operations already.
    ///
    /// # Cancel safety
    ///
    /// Dropping the future gives up the wait, or the yield before an operation, and never leaves an
    /// operation half done: each call of `operation` runs to its end before the future can be
    /// dropped.
    pub async fn write_with<R>(&self, operation: impl FnMut(&T) -> io::Result<R>) -> io::Result<R> {
        self.run_with(Interest::Writable, operation).await
    }

    /// The polling version of [`read_with`](AsyncIo::read_with), for implementing poll-based
    /// traits.
    ///
    /// Runs `operation` on the source. If it fails with [`WouldBlock`](io::ErrorKind::WouldBlock),
    /// this arranges for the waker of `cx` to be woken once the source is readable, and returns
    /// [`Poll::Pending`]. It also returns `Pending`, after waking the task, without running
    /// `operation`, if the poll it is in has completed its share of I/O operations already: see
    /// [`Registration::poll_io`].
    ///
    /// Unlike `read_with`, this keeps only one waiting task for reading. A second task that waits
    /// replaces the first one, which is then never woken. See [`Registration::poll_io`].
    pub fn poll_read_with<R>(
        &self,
        cx: &mut Context<'_>,
        operation: impl FnMut(&T) -> io::Result<R>,
    ) -> Poll<io::Result<R>> {
        self.poll_io(cx, Interest::Readable, operation)
    }

    /// The polling version of [`write_with`](AsyncIo::write_with), for implementing poll-based
    /// traits.
    ///
    /// Runs `operation` on the source. If it fails with [`WouldBlock`](io::ErrorKind::WouldBlock),
    /// this arranges for the waker of `cx` to be woken once the source is writable, and returns
    /// [`Poll::Pending`]. It also returns `Pending`, after waking the task, without running
    /// `operation`, if the poll it is in has completed its share of I/O operations already: see
    /// [`Registration::poll_io`].
    ///
    /// Unlike `write_with`, this keeps only one waiting task for writing. A second task that waits
    /// replaces the first one, which is then never woken. See [`Registration::poll_io`].
    pub fn poll_write_with<R>(
        &self,
        cx: &mut Context<'_>,
        operation: impl FnMut(&T) -> io::Result<R>,
    ) -> Poll<io::Result<R>> {
        self.poll_io(cx, Interest::Writable, operation)
    }

    /// An `AsyncIo` on `runtime` that does its I/O on `io`, as it is: the constructor the sockets
    /// of the `net` module and the pipes and exit descriptors of the `process` module are built
    /// with.
    ///
    /// Those are generic over the flavour, and a [`Source<M>`](Source) bound cannot be met for an
    /// abstract `M`, so this takes what both flavours need of a source instead, `Send` and `Sync`
    /// included. `io` must be in non-blocking mode already, as for
    /// [`new_nonblocking`](AsyncIo::new_nonblocking), unless nothing is ever read from it or
    /// written to it, and only its readiness is waited for, as for the exit descriptors.
    #[cfg(any(
        feature = "tcp",
        feature = "udp",
        all(feature = "unix", unix),
        all(feature = "process", unix)
    ))]
    pub(crate) fn from_nonblocking(runtime: &Runtime<M>, io: T) -> io::Result<Self>
    where
        T: AsSource + Send + Sync + 'static,
    {
        let io = M::new_ptr(io);
        let registration = reactor::register::<M>(&runtime.core, M::source_ptr(io.clone()))?;
        // Asked for once the source is in the reactor's map, so that a helper starting here takes
        // it into its very first wait.
        M::ensure_progress(&runtime.core);

        Ok(Self { registration, io })
    }

    /// A handle on the runtime the source is registered on.
    #[cfg(any(
        feature = "tcp",
        all(feature = "unix", unix),
        all(
            feature = "process",
            any(
                target_vendor = "apple",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly"
            )
        )
    ))]
    pub(crate) fn runtime(&self) -> Runtime<M> {
        self.registration.runtime()
    }

    /// Runs `operation` on the source until it no longer fails with `WouldBlock`, waiting for the
    /// source to be ready for `interest` in between: [`read_with`](AsyncIo::read_with) and
    /// [`write_with`](AsyncIo::write_with).
    async fn run_with<R>(
        &self,
        interest: Interest,
        mut operation: impl FnMut(&T) -> io::Result<R>,
    ) -> io::Result<R> {
        loop {
            // Asked before the operation, not after it: a yield here costs nothing, while one
            // after an operation that completed would hold its result back.
            poll_fn(coop::poll_proceed).await;
            match operation(&self.io) {
                // A call the kernel interrupted is made again straight away: that is not a
                // readiness question, so it never reaches the reactor, and it spends nothing.
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                result => {
                    coop::spend();

                    return result;
                }
            }
            self.registration.ready(interest).await?;
        }
    }

    /// Runs `operation` on the source while it is ready for `interest`: see
    /// [`Registration::poll_io`].
    ///
    /// A call the kernel interrupts is made again straight away: that is not a readiness
    /// question, so it never reaches the reactor.
    fn poll_io<R>(
        &self,
        cx: &mut Context<'_>,
        interest: Interest,
        mut operation: impl FnMut(&T) -> io::Result<R>,
    ) -> Poll<io::Result<R>> {
        let io = &*self.io;

        self.registration.poll_io(cx, interest, || {
            loop {
                match operation(io) {
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                    result => return result,
                }
            }
        })
    }
}

impl<T, M> fmt::Debug for AsyncIo<T, M>
where
    M: Mode,
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncIo")
            .field("io", self.get_ref())
            .finish_non_exhaustive()
    }
}

#[cfg(any(unix, target_os = "wasi"))]
impl<T, M> AsFd for AsyncIo<T, M>
where
    M: Mode,
    T: AsFd,
{
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.get_ref().as_fd()
    }
}

#[cfg(any(unix, target_os = "wasi"))]
impl<T, M> AsRawFd for AsyncIo<T, M>
where
    M: Mode,
    T: AsRawFd,
{
    fn as_raw_fd(&self) -> RawFd {
        self.get_ref().as_raw_fd()
    }
}

#[cfg(windows)]
impl<T, M> AsSocket for AsyncIo<T, M>
where
    M: Mode,
    T: AsSocket,
{
    fn as_socket(&self) -> BorrowedSocket<'_> {
        self.get_ref().as_socket()
    }
}

#[cfg(windows)]
impl<T, M> AsRawSocket for AsyncIo<T, M>
where
    M: Mode,
    T: AsRawSocket,
{
    fn as_raw_socket(&self) -> RawSocket {
        self.get_ref().as_raw_socket()
    }
}

impl<T, M> AsyncRead for AsyncIo<T, M>
where
    M: Mode,
    for<'a> &'a T: Read,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &*self).poll_read(cx, buf)
    }

    fn poll_read_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &*self).poll_read_vectored(cx, bufs)
    }
}

/// Closing flushes, and leaves the source open: shutting a socket down, or closing the
/// descriptor, is the source's business, and is done by dropping the `AsyncIo` or by taking the
/// source back with [`into_inner`](AsyncIo::into_inner).
impl<T, M> AsyncWrite for AsyncIo<T, M>
where
    M: Mode,
    for<'a> &'a T: Write,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &*self).poll_write(cx, buf)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &*self).poll_write_vectored(cx, bufs)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut &*self).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut &*self).poll_close(cx)
    }
}

impl<T, M> AsyncRead for &AsyncIo<T, M>
where
    M: Mode,
    for<'a> &'a T: Read,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        self.poll_read_with(cx, |mut io| io.read(buf))
    }

    fn poll_read_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
    ) -> Poll<io::Result<usize>> {
        self.poll_read_with(cx, |mut io| io.read_vectored(bufs))
    }
}

/// Closing flushes, and leaves the source open: shutting a socket down, or closing the
/// descriptor, is the source's business, and is done by dropping the `AsyncIo` or by taking the
/// source back with [`into_inner`](AsyncIo::into_inner).
impl<T, M> AsyncWrite for &AsyncIo<T, M>
where
    M: Mode,
    for<'a> &'a T: Write,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.poll_write_with(cx, |mut io| io.write(buf))
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        self.poll_write_with(cx, |mut io| io.write_vectored(bufs))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_write_with(cx, |mut io| io.flush())
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}

/// Switches `io` to non-blocking mode.
#[cfg(any(unix, target_os = "wasi"))]
fn set_nonblocking<T>(io: &T) -> io::Result<()>
where
    T: AsFd,
{
    Ok(rustix::io::ioctl_fionbio(io, true)?)
}

/// Switches `io` to non-blocking mode.
#[cfg(windows)]
fn set_nonblocking<T>(io: &T) -> io::Result<()>
where
    T: AsSocket,
{
    let mut enable = 1u32;
    // SAFETY: `ioctlsocket` is given a socket that `io` keeps open for the call, and a pointer to
    // a `u32` that outlives it, which is all that `FIONBIO` reads or writes.
    let result = unsafe {
        ioctlsocket(
            io.as_socket().as_raw_socket() as SOCKET,
            FIONBIO,
            &mut enable,
        )
    };
    if result == SOCKET_ERROR {
        // SAFETY: `WSAGetLastError` takes nothing and reads this thread's last Winsock error.
        return Err(io::Error::from_raw_os_error(unsafe { WSAGetLastError() }));
    }

    Ok(())
}
