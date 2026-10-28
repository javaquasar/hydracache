//! The pipes to a child: [`ChildStdin`], [`ChildStdout`] and [`ChildStderr`].

use std::{
    fmt,
    future::poll_fn,
    io::{self, IoSlice, IoSliceMut},
    pin::Pin,
    process::Stdio,
    task::{Context, Poll, ready},
};
#[cfg(unix)]
use std::{
    io::{PipeReader, PipeWriter, Read, Write},
    os::fd::{AsFd, OwnedFd},
};
#[cfg(windows)]
use std::{
    io::{Read, Write},
    marker::PhantomData,
};

use futures_io::{AsyncRead, AsyncWrite};

#[cfg(unix)]
use crate::AsyncIo;
#[cfg(windows)]
use crate::Unblock;
use crate::{Local, Mode, Runtime};

/// The pipe to a child's standard input, to write to.
///
/// [`Child::stdin`](super::Child::stdin) holds it if the command asked for `Stdio::piped()`. It is
/// the async counterpart of [`std::process::ChildStdin`]: a write that has to wait lets other
/// tasks run instead of blocking the thread. It implements the `AsyncWrite` trait of
/// [`futures-io`], so the extension traits of [`futures`] work on it.
///
/// Closing the pipe, for example with `close` of `AsyncWriteExt`, flushes it and then closes it.
/// Dropping the pipe also closes it, without a flush. Either way, the child sees the end of its
/// input after it has read what was written. A child that reads its input to the end can only
/// finish after that, so close the pipe when you are done writing. A write after the close fails
/// with [`BrokenPipe`](io::ErrorKind::BrokenPipe). A later flush or close does nothing.
/// [`Child::status`](super::Child::status) and [`Child::output`](super::Child::output) drop the
/// pipe before they wait.
///
/// On unix, the pipe is non-blocking. A write that finds it full waits until the runtime reports
/// room in it. Writes go straight to the pipe, so a flush has nothing to do. On Windows, the
/// runtime cannot watch a pipe: a write hands its bytes to blocking work on a thread of the pool
/// of [`unblock()`](crate::unblock()), and a flush waits until the bytes handed over so far are
/// written. To learn of an error in those writes, flush or close the pipe before dropping it.
/// Bytes still pending when the pipe is dropped are written anyway, but an error they run into
/// is not reported.
///
/// The pipe belongs to the runtime the child was spawned on. Its type parameter is the flavour of
/// that runtime, `Local` by default, as for [`Child`](super::Child): see the
/// [module documentation](super).
///
/// # Example
///
/// Writes to a child that sorts the lines it reads, then closes the pipe to give it the end of its
/// input:
///
/// ```
/// # #[cfg(unix)]
/// # fn main() -> std::io::Result<()> {
/// use futures::{AsyncReadExt, AsyncWriteExt};
/// use zruntime::{
///     LocalRuntime,
///     process::{Command, Stdio},
/// };
///
/// let runtime = LocalRuntime::new()?;
///
/// runtime.block_on(async {
///     let mut child = Command::new("sort")
///         .stdin(Stdio::piped())
///         .stdout(Stdio::piped())
///         .spawn(&runtime)?;
///
///     let mut stdin = child.stdin.take().expect("stdin is piped");
///     stdin.write_all(b"pear\napple\n").await?;
///     // Dropping the pipe closes it, so the child reads the end of its input.
///     drop(stdin);
///
///     let mut sorted = String::new();
///     let mut stdout = child.stdout.take().expect("stdout is piped");
///     stdout.read_to_string(&mut sorted).await?;
///
///     assert_eq!(sorted, "apple\npear\n");
///     # Ok::<_, std::io::Error>(())
/// })?;
/// # Ok(())
/// # }
/// # #[cfg(not(unix))]
/// # fn main() {}
/// ```
///
/// [`futures-io`]: https://docs.rs/futures-io
/// [`futures`]: https://docs.rs/futures
pub struct ChildStdin<M = Local>
where
    M: Mode,
{
    /// The pipe, until it is closed.
    pipe: Option<Pipe<StdinPipe, M>>,
}

impl<M> ChildStdin<M>
where
    M: Mode,
{
    /// The pipe to a child's standard input, on `runtime`.
    pub(super) fn new(runtime: &Runtime<M>, pipe: std::process::ChildStdin) -> io::Result<Self> {
        Ok(Self {
            pipe: Some(Pipe::new(runtime, pipe)?),
        })
    }

    /// Converts the pipe into a [`Stdio`] for the standard output or error of another child.
    ///
    /// This connects the processes of a pipeline: the child that gets the pipe writes straight into
    /// the input of the child this pipe was made for, with no byte passing through your code. You
    /// cannot write to the pipe after this.
    ///
    /// The pipe is flushed first. On Windows, the bytes written to it may not have reached the pipe
    /// yet, and an error in writing them is reported here instead of being lost.
    ///
    /// The pipe is switched back to blocking mode, as a program that is given one expects. On unix,
    /// the mode belongs to the open pipe, which the other child shares, not to the handle.
    ///
    /// # Errors
    ///
    /// Fails if the flush fails, and, on unix, if the pipe cannot be switched back to blocking
    /// mode. Fails with [`BrokenPipe`](io::ErrorKind::BrokenPipe) if the pipe has been closed
    /// already, as there is no pipe left to hand over.
    pub async fn into_stdio(self) -> io::Result<Stdio> {
        let Some(mut pipe) = self.pipe else {
            return Err(closed());
        };
        poll_fn(|cx| Pin::new(&mut pipe).poll_flush(cx)).await?;

        pipe.into_stdio().await
    }

    /// The pipe, or the error of a write to a pipe that has been closed.
    fn pipe(&mut self) -> io::Result<&mut Pipe<StdinPipe, M>> {
        self.pipe.as_mut().ok_or_else(closed)
    }
}

impl<M> AsyncWrite for ChildStdin<M>
where
    M: Mode,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let pipe = match self.get_mut().pipe() {
            Ok(pipe) => pipe,
            Err(e) => return Poll::Ready(Err(e)),
        };

        Pin::new(pipe).poll_write(cx, buf)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        let pipe = match self.get_mut().pipe() {
            Ok(pipe) => pipe,
            Err(e) => return Poll::Ready(Err(e)),
        };

        Pin::new(pipe).poll_write_vectored(cx, bufs)
    }

    /// Flushes the pipe. Does nothing if the pipe has been closed.
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let Some(pipe) = &mut self.get_mut().pipe else {
            return Poll::Ready(Ok(()));
        };

        Pin::new(pipe).poll_flush(cx)
    }

    /// Flushes the pipe and then closes it, so the child reads the end of its input.
    ///
    /// The pipe is closed once the flush is over, whether or not it succeeded, and this returns the
    /// result of the flush. A close that fails has closed the pipe all the same. Does nothing if
    /// the pipe has been closed already.
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let Some(pipe) = &mut this.pipe else {
            return Poll::Ready(Ok(()));
        };

        let flushed = ready!(Pin::new(pipe).poll_flush(cx));
        this.pipe = None;

        Poll::Ready(flushed)
    }
}

impl<M> fmt::Debug for ChildStdin<M>
where
    M: Mode,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChildStdin").finish_non_exhaustive()
    }
}

/// The pipe from a child's standard output, to read from.
///
/// [`Child::stdout`](super::Child::stdout) holds it if the command asked for `Stdio::piped()`. It
/// is the async counterpart of [`std::process::ChildStdout`]: a read that has to wait lets other
/// tasks run instead of blocking the thread. It implements the `AsyncRead` trait of
/// [`futures-io`], so the extension traits of [`futures`] work on it.
///
/// A read returns zero bytes at the end of the pipe. The end comes once every byte the child wrote
/// has been read and every process that holds the write end has closed it. The child closes it
/// when it exits or closes its standard output. So must any process the child started that
/// inherited the pipe.
///
/// Dropping the pipe closes it. A child that then writes to its standard output gets a broken-pipe
/// error or, on unix, is killed by `SIGPIPE` unless it ignores that signal.
///
/// On unix, the pipe is non-blocking. A read that finds it empty waits until the runtime reports
/// data in it. On Windows, the runtime cannot watch a pipe, so each read is blocking work on a
/// thread of the pool of [`unblock()`](crate::unblock()). It reads ahead of what the caller asked
/// for, up to 8 KiB. A read that has started runs to its end, even if its future is dropped. A
/// pipe dropped during such a read stays open, and the thread stays held, until the read returns.
/// That happens when the child next writes, and those bytes are lost, or when the pipe ends.
///
/// The pipe belongs to the runtime the child was spawned on. Its type parameter is the flavour of
/// that runtime, `Local` by default, as for [`Child`](super::Child): see the
/// [module documentation](super).
///
/// # Example
///
/// Reads the lines a child prints, one at a time as it prints them:
///
/// ```
/// # #[cfg(unix)]
/// # fn main() -> std::io::Result<()> {
/// use futures::{AsyncBufReadExt, StreamExt, io::BufReader};
/// use zruntime::{
///     LocalRuntime,
///     process::{Command, Stdio},
/// };
///
/// let runtime = LocalRuntime::new()?;
///
/// runtime.block_on(async {
///     let mut child = Command::new("echo")
///         .arg("hello")
///         .stdout(Stdio::piped())
///         .spawn(&runtime)?;
///
///     let stdout = child.stdout.take().expect("stdout is piped");
///     let mut lines = BufReader::new(stdout).lines();
///     while let Some(line) = lines.next().await {
///         assert_eq!(line?, "hello");
///     }
///
///     assert!(child.status().await?.success());
///     # Ok::<_, std::io::Error>(())
/// })?;
/// # Ok(())
/// # }
/// # #[cfg(not(unix))]
/// # fn main() {}
/// ```
///
/// [`futures-io`]: https://docs.rs/futures-io
/// [`futures`]: https://docs.rs/futures
pub struct ChildStdout<M = Local>
where
    M: Mode,
{
    pipe: Pipe<StdoutPipe, M>,
}

impl<M> ChildStdout<M>
where
    M: Mode,
{
    /// The pipe from a child's standard output, on `runtime`.
    pub(super) fn new(runtime: &Runtime<M>, pipe: std::process::ChildStdout) -> io::Result<Self> {
        Ok(Self {
            pipe: Pipe::new(runtime, pipe)?,
        })
    }

    /// Converts the pipe into a [`Stdio`] for the standard input of another child.
    ///
    /// This connects the processes of a pipeline: the child that this pipe was made for writes into
    /// it, and the child that gets the [`Stdio`] reads what the first one wrote, with no byte
    /// passing through your code. You cannot read from the pipe after this, and the bytes already
    /// read from it are gone.
    ///
    /// On Windows, the bytes that the pipe read ahead of what was asked for are dropped. A read in
    /// progress there, even one whose future was dropped, is waited for first. That takes until the
    /// child writes again or the pipe ends, and the bytes it reads are dropped too.
    ///
    /// The pipe is switched back to blocking mode, as a program that is given one expects. On unix,
    /// the mode belongs to the open pipe, which the other child shares, not to the handle.
    ///
    /// # Errors
    ///
    /// On unix, fails if the pipe cannot be switched back to blocking mode.
    ///
    /// # Example
    ///
    /// Runs `echo hello | cat` and reads the output of the second child:
    ///
    /// ```
    /// # #[cfg(unix)]
    /// # fn main() -> std::io::Result<()> {
    /// use futures::AsyncReadExt;
    /// use zruntime::{
    ///     LocalRuntime,
    ///     process::{Command, Stdio},
    /// };
    ///
    /// let runtime = LocalRuntime::new()?;
    ///
    /// runtime.block_on(async {
    ///     let mut echo = Command::new("echo")
    ///         .arg("hello")
    ///         .stdout(Stdio::piped())
    ///         .spawn(&runtime)?;
    ///     let pipe = echo.stdout.take().expect("stdout is piped");
    ///
    ///     let mut cat = Command::new("cat")
    ///         .stdin(pipe.into_stdio().await?)
    ///         .stdout(Stdio::piped())
    ///         .spawn(&runtime)?;
    ///
    ///     let mut text = String::new();
    ///     let mut stdout = cat.stdout.take().expect("stdout is piped");
    ///     stdout.read_to_string(&mut text).await?;
    ///
    ///     assert_eq!(text, "hello\n");
    ///     assert!(echo.status().await?.success());
    ///     assert!(cat.status().await?.success());
    ///     # Ok::<_, std::io::Error>(())
    /// })?;
    /// # Ok(())
    /// # }
    /// # #[cfg(not(unix))]
    /// # fn main() {}
    /// ```
    pub async fn into_stdio(self) -> io::Result<Stdio> {
        self.pipe.into_stdio().await
    }
}

impl<M> AsyncRead for ChildStdout<M>
where
    M: Mode,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().pipe).poll_read(cx, buf)
    }

    fn poll_read_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().pipe).poll_read_vectored(cx, bufs)
    }
}

impl<M> fmt::Debug for ChildStdout<M>
where
    M: Mode,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChildStdout").finish_non_exhaustive()
    }
}

/// The pipe from a child's standard error, to read from.
///
/// [`Child::stderr`](super::Child::stderr) holds it if the command asked for `Stdio::piped()`. It
/// is the async counterpart of [`std::process::ChildStderr`], and is read exactly like the pipe of
/// the standard output, [`ChildStdout`]: see there for how reads wait, where the pipe ends, and
/// what the type parameter means.
///
/// A child that writes a lot to both its output and its error blocks once one of the pipes is
/// full, until that pipe is read. A task that reads one pipe to its end before the other waits
/// forever if the child is stuck on the other pipe. Read both pipes at the same time, as
/// [`Child::output`](super::Child::output) does.
pub struct ChildStderr<M = Local>
where
    M: Mode,
{
    pipe: Pipe<StderrPipe, M>,
}

impl<M> ChildStderr<M>
where
    M: Mode,
{
    /// The pipe from a child's standard error, on `runtime`.
    pub(super) fn new(runtime: &Runtime<M>, pipe: std::process::ChildStderr) -> io::Result<Self> {
        Ok(Self {
            pipe: Pipe::new(runtime, pipe)?,
        })
    }

    /// Converts the pipe into a [`Stdio`] for the standard input of another child.
    ///
    /// It works as [`ChildStdout::into_stdio`] does, for the pipe of the standard error.
    pub async fn into_stdio(self) -> io::Result<Stdio> {
        self.pipe.into_stdio().await
    }
}

impl<M> AsyncRead for ChildStderr<M>
where
    M: Mode,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().pipe).poll_read(cx, buf)
    }

    fn poll_read_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().pipe).poll_read_vectored(cx, bufs)
    }
}

impl<M> fmt::Debug for ChildStderr<M>
where
    M: Mode,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChildStderr").finish_non_exhaustive()
    }
}

/// What a pipe's I/O runs on, by the end of the pipe: the standard library's ends of an anonymous
/// pipe on unix, where the runtime watches them, and the types of a child's own standard streams
/// on Windows, which no runtime can watch and so are handed to blocking work.
#[cfg(unix)]
type StdinPipe = PipeWriter;
#[cfg(unix)]
type StdoutPipe = PipeReader;
#[cfg(unix)]
type StderrPipe = PipeReader;
#[cfg(windows)]
type StdinPipe = std::process::ChildStdin;
#[cfg(windows)]
type StdoutPipe = std::process::ChildStdout;
#[cfg(windows)]
type StderrPipe = std::process::ChildStderr;

/// A pipe to or from a child, with the I/O of the platform it runs on: the three public pipes are
/// this, over the type of their own end.
///
/// An `AsyncIo` of the runtime on unix, which watches the pipe for readiness.
#[cfg(unix)]
struct Pipe<T, M>(AsyncIo<T, M>)
where
    M: Mode;

/// A pipe to or from a child, with the I/O of the platform it runs on: the three public pipes are
/// this, over the type of their own end.
///
/// An `Unblock` on Windows, which runs each operation as blocking work. The pointer a runtime of
/// the flavour shares its state by is what the type is tied to its flavour through, so that a
/// pipe built on a local runtime stays on its thread on every platform, as the `AsyncIo` of unix
/// does of itself.
#[cfg(windows)]
struct Pipe<T, M>(Unblock<T>, PhantomData<M::Ptr<()>>)
where
    M: Mode;

#[cfg(unix)]
impl<T, M> Pipe<T, M>
where
    M: Mode,
    T: AsFd + From<OwnedFd> + Send + Sync + 'static,
{
    /// A pipe on `runtime` that does its I/O on `pipe`, one of std's ends of a pipe to a child.
    ///
    /// The end is switched to non-blocking mode here, which is shared by every descriptor of the
    /// open pipe end but is no concern of the child: what the child has is the other end of the
    /// pipe, an open file description of its own.
    fn new<S>(runtime: &Runtime<M>, pipe: S) -> io::Result<Self>
    where
        S: Into<OwnedFd>,
    {
        let pipe = T::from(pipe.into());
        rustix::io::ioctl_fionbio(&pipe, true)?;

        Ok(Self(AsyncIo::from_nonblocking(runtime, pipe)?))
    }
}

#[cfg(windows)]
impl<T, M> Pipe<T, M>
where
    M: Mode,
{
    /// A pipe that does its I/O on `pipe`, one of std's standard streams of a child, as blocking
    /// work.
    ///
    /// Nothing of the runtime is needed for that.
    fn new(_runtime: &Runtime<M>, pipe: T) -> io::Result<Self> {
        Ok(Self(Unblock::new(pipe), PhantomData))
    }
}

#[cfg(unix)]
impl<T, M> Pipe<T, M>
where
    M: Mode,
    T: AsFd + Into<Stdio>,
{
    /// The pipe as a `Stdio` for another child, in blocking mode.
    async fn into_stdio(self) -> io::Result<Stdio> {
        let pipe = self.0.into_inner();
        // The mode belongs to the open pipe, which the other child is given a descriptor of, so it
        // is put back as the child expects it to be.
        rustix::io::ioctl_fionbio(&pipe, false)?;

        Ok(pipe.into())
    }
}

#[cfg(windows)]
impl<T, M> Pipe<T, M>
where
    M: Mode,
    T: Into<Stdio> + Send + 'static,
{
    /// The pipe as a `Stdio` for another child, once the operation in flight is over.
    async fn into_stdio(self) -> io::Result<Stdio> {
        Ok(self.0.into_inner().await.into())
    }
}

#[cfg(unix)]
impl<T, M> AsyncRead for Pipe<T, M>
where
    M: Mode,
    for<'a> &'a T: Read,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &self.0).poll_read(cx, buf)
    }

    fn poll_read_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &self.0).poll_read_vectored(cx, bufs)
    }
}

#[cfg(windows)]
impl<T, M> AsyncRead for Pipe<T, M>
where
    M: Mode,
    T: Read + Send + 'static,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().0).poll_read(cx, buf)
    }

    fn poll_read_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().0).poll_read_vectored(cx, bufs)
    }
}

#[cfg(unix)]
impl<T, M> AsyncWrite for Pipe<T, M>
where
    M: Mode,
    for<'a> &'a T: Write,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &self.0).poll_write(cx, buf)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &self.0).poll_write_vectored(cx, bufs)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut &self.0).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut &self.0).poll_close(cx)
    }
}

#[cfg(windows)]
impl<T, M> AsyncWrite for Pipe<T, M>
where
    M: Mode,
    T: Write + Send + 'static,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().0).poll_write(cx, buf)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().0).poll_write_vectored(cx, bufs)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().0).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().0).poll_close(cx)
    }
}

/// The error of a write to the pipe to a child's input once it has been closed.
fn closed() -> io::Error {
    io::Error::new(
        io::ErrorKind::BrokenPipe,
        "the pipe to the child's standard input is closed",
    )
}
