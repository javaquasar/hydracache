//! An async adapter for a blocking I/O handle, [`Unblock`].

use std::{
    any::Any,
    collections::VecDeque,
    fmt,
    future::{self, Future},
    io::{self, Read, Seek, SeekFrom, Write},
    mem,
    num::NonZeroUsize,
    panic::{self, AssertUnwindSafe},
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    task::{Context, Poll, Wake, Waker, ready},
};

use futures_core::Stream;
use futures_io::{AsyncRead, AsyncSeek, AsyncWrite};

use super::{BlockingWork, dispose, unblock};

/// An async adapter for a blocking I/O handle.
///
/// Each operation on the handle runs as blocking work through [`unblock()`], so the handle never
/// blocks the thread that polls a task.
///
/// The handle can be anything that implements [`Read`], [`Write`], [`Seek`] or [`Iterator`]: a
/// file, the standard input or output, a pipe to a child process, or the entries of a directory. If
/// the handle is `Send + 'static`, the adapter implements:
///
/// * [`AsyncRead`] of `futures-io`, if the handle implements [`Read`].
/// * [`AsyncWrite`] of `futures-io`, if it implements [`Write`].
/// * [`AsyncSeek`] of `futures-io`, if it implements [`Seek`].
/// * [`Stream`] of `futures-core`, if it is an [`Iterator`] whose items are `Send + 'static`.
///
/// Like [`unblock()`], the adapter needs no runtime and works under any executor. It is `Send` and
/// `Sync` if the handle is `Send`, and it is always `Unpin`.
///
/// # One operation at a time
///
/// The adapter runs one operation on the handle at a time. Each operation waits for the one in
/// progress to finish, whether that is a read, a write or anything else. A read that is served from
/// bytes already read ahead does not wait.
///
/// Two tasks can wait at once, for example the two halves of a split adapter. Both are woken when
/// the operation ends, even if one of the wakers panics.
///
/// This is harmless for a file. It can deadlock a handle with separate read and write streams, such
/// as a socket or a serial port. A read that waits for the peer to send data blocks a write that
/// the peer is waiting for first.
///
/// # Reading
///
/// A read from the handle reads up to the adapter's capacity, whatever the size of the buffer it
/// was given. The bytes the buffer does not take are kept for the reads that follow. So the
/// handle's own position is ahead of what the reads have returned.
///
/// A seek accounts for this, including a seek from [`SeekFrom::Current`]. A write does not. If the
/// handle's reads and writes share one position, as a file's do, seek to `SeekFrom::Current(0)`
/// before writing after a read. That moves the handle's position back to where the reads got to.
///
/// The bytes read ahead are kept across writes. A handle with two separate streams loses none of
/// the bytes it read.
///
/// # Writing
///
/// A write takes as many of the given bytes as the capacity allows. It completes as soon as it has
/// passed them to blocking work, which writes all of them to the handle. The next operation waits
/// for that work to finish. If the work failed, the next write or flush returns the error.
///
/// A flush waits for the earlier writes to finish, then flushes the handle. Closing the adapter
/// flushes it but leaves the handle open. The handle is closed when it is dropped, either with the
/// adapter or after [`into_inner`](Unblock::into_inner).
///
/// Once a write completes, its bytes no longer depend on the adapter. Dropping the adapter gives up
/// the wait for the write in progress, not the write itself. The bytes are still written, as for
/// [`unblock()`], but an error goes unreported. Flush the adapter before dropping it to learn of
/// errors.
///
/// # Iterating
///
/// The stream pulls items from the iterator in batches, in one piece of blocking work, and yields
/// them one at a time. A batch holds as many items as the capacity, but at most 16, or fewer if the
/// iterator ends first.
///
/// The first item of a batch is therefore delayed until the whole batch has been pulled. This
/// matters for iterators with slow items, such as the lines of the standard input. With a capacity
/// of 1, each item is yielded as soon as it arrives.
///
/// The stream yields `None` when the iterator ends. If it is polled again after that, it pulls from
/// the iterator again.
///
/// # Using the handle directly
///
/// [`get_mut`](Unblock::get_mut), [`with_mut`](Unblock::with_mut) and
/// [`into_inner`](Unblock::into_inner) give access to the handle itself, once the operation in
/// progress has finished. They drop the bytes read ahead and the items pulled ahead. These are
/// lost: the handle is already past them, and the adapter cannot know what is done to the handle
/// directly.
///
/// # Panics
///
/// If an operation on the handle panics, the poll that waits for it raises the panic again with its
/// original payload, as for [`unblock()`]. The handle is lost with the panic, and any later use of
/// the adapter panics too.
///
/// # Examples
///
/// A [`Cursor`](std::io::Cursor) stands in for a blocking handle to read, and a `Vec` for one to
/// write. The examples drive the futures with `block_on` from the `futures` crate, but the
/// `block_on` of any executor works, as the adapter needs no runtime:
///
/// ```
/// use std::io::Cursor;
///
/// use futures::{AsyncReadExt, AsyncWriteExt, executor::block_on};
/// use zruntime::Unblock;
///
/// block_on(async {
///     let mut reader = Unblock::new(Cursor::new("hello, world"));
///     let mut text = String::new();
///     reader.read_to_string(&mut text).await?;
///     assert_eq!(text, "hello, world");
///
///     let mut writer = Unblock::new(Vec::new());
///     writer.write_all(b"goodbye").await?;
///     writer.flush().await?;
///     assert_eq!(writer.into_inner().await, b"goodbye");
///
///     std::io::Result::Ok(())
/// })
/// .unwrap();
/// ```
///
/// Reading the standard input line by line, as the lines arrive, through a `BufReader` and `lines`
/// from the `futures` crate. A `Cursor` stands in for `std::io::stdin()`:
///
/// ```
/// use std::io::Cursor;
///
/// use futures::{AsyncBufReadExt, StreamExt, executor::block_on, io::BufReader};
/// use zruntime::Unblock;
///
/// block_on(async {
///     let stdin = Cursor::new("first line\nsecond line\n");
///     let mut lines = BufReader::new(Unblock::new(stdin)).lines();
///     while let Some(line) = lines.next().await {
///         println!("{}", line?);
///     }
///
///     std::io::Result::Ok(())
/// })
/// .unwrap();
/// ```
#[doc = not_on_wasi!()]
pub struct Unblock<T> {
    /// Behind a mutex that is never locked: the adapter is only ever used through a mutable
    /// borrow, which reaches what the mutex holds through its `get_mut`. The mutex is there for
    /// its `Sync`, which it has wherever what it holds is `Send`, as nothing of the handle is
    /// reachable through a shared reference to the adapter.
    inner: Mutex<Inner<T>>,
}

impl<T> Unblock<T> {
    /// Creates an adapter for `io` with the default capacity of 8 KiB.
    ///
    /// See [`with_capacity`](Unblock::with_capacity) for what the capacity means.
    pub fn new(io: T) -> Self {
        Self::with_capacity(DEFAULT_CAPACITY, io)
    }

    /// Creates an adapter for `io` with a capacity of `cap`.
    ///
    /// The capacity is the most bytes that one operation reads ahead or writes. It is also the most
    /// items that one operation pulls from an iterator, with an upper limit of 16. A larger
    /// capacity means fewer and larger calls on the handle, at the cost of memory. Once the adapter
    /// has read or written, it keeps one buffer of that size for reads and one for writes.
    pub fn with_capacity(cap: NonZeroUsize, io: T) -> Self {
        Self {
            inner: Mutex::new(Inner {
                state: State::Idle(io),
                read_ahead: ReadAhead::default(),
                write_buf: Vec::new(),
                write_error: None,
                items: None,
                waiters: Arc::default(),
                cap,
            }),
        }
    }

    /// Borrows the handle mutably, once the operation in progress has finished.
    ///
    /// By then, every byte a write took has been passed to the handle, though the handle may still
    /// buffer some of them itself. If the last write failed and no write or flush has reported the
    /// error yet, it is kept for the next write or flush.
    ///
    /// The bytes read ahead and the items pulled ahead are dropped. See
    /// [using the handle directly](Unblock#using-the-handle-directly).
    ///
    /// # Example
    ///
    /// ```
    /// use std::io::Cursor;
    ///
    /// use futures::{AsyncReadExt, executor::block_on};
    /// use zruntime::Unblock;
    ///
    /// block_on(async {
    ///     let mut reader = Unblock::new(Cursor::new("hello"));
    ///     let mut text = String::new();
    ///     reader.read_to_string(&mut text).await?;
    ///
    ///     assert_eq!(reader.get_mut().await.position(), 5);
    ///
    ///     std::io::Result::Ok(())
    /// })
    /// .unwrap();
    /// ```
    pub async fn get_mut(&mut self) -> &mut T {
        let inner = self.inner();
        future::poll_fn(|cx| inner.poll_idle(cx)).await;
        inner.drop_ahead();

        let State::Idle(handle) = &mut inner.state else {
            unreachable!("the handle is back once no operation is in flight");
        };
        handle
    }

    /// Runs `op` on the handle as blocking work, once the operation in progress has finished, and
    /// returns what `op` returns.
    ///
    /// Use it for what the async traits do not cover, for example reading a file's metadata or
    /// setting its length. As with [`get_mut`](Unblock::get_mut), every byte a write took has been
    /// passed to the handle before `op` runs. An error from the last write is kept for the next
    /// write or flush. The bytes read ahead and the items pulled ahead are dropped.
    ///
    /// Once `op` has started, nothing can stop it. Dropping the future gives up the wait, but `op`
    /// still runs to the end. The next operation waits for it and discards what it returned. If the
    /// future is dropped before `op` starts, while it waits for the operation in progress, `op`
    /// never runs.
    ///
    /// # Panics
    ///
    /// If `op` panics, the poll of the future that would have returned its value raises the panic
    /// again. The adapter loses the handle, as for any operation on it.
    ///
    /// # Example
    ///
    /// ```
    /// use std::io::Cursor;
    ///
    /// use futures::executor::block_on;
    /// use zruntime::Unblock;
    ///
    /// let mut reader = Unblock::new(Cursor::new(vec![1, 2, 3]));
    ///
    /// let len = block_on(reader.with_mut(|cursor| cursor.get_ref().len()));
    ///
    /// assert_eq!(len, 3);
    /// ```
    pub async fn with_mut<R, F>(&mut self, op: F) -> R
    where
        T: Send + 'static,
        F: FnOnce(&mut T) -> R + Send + 'static,
        R: Send + 'static,
    {
        let inner = self.inner();
        future::poll_fn(|cx| inner.poll_idle(cx)).await;
        inner.drop_ahead();

        inner.start(move |handle| Outcome::Ran(Box::new(op(handle))));
        // The future holds the adapter borrowed, so the operation that ends is the one it started.
        let Some(Outcome::Ran(value)) = future::poll_fn(|cx| inner.poll_job(cx)).await else {
            unreachable!("the operation in flight is the one `with_mut` started");
        };
        match value.downcast() {
            Ok(value) => *value,
            Err(_) => unreachable!("`with_mut` hands back what its own operation returned"),
        }
    }

    /// Takes the handle out of the adapter, once the operation in progress has finished.
    ///
    /// By then, every byte a write took has been passed to the handle, though the handle may still
    /// buffer some of them itself. If the last write failed and no write or flush has reported the
    /// error yet, the error is lost, so flush the adapter first to learn of it. The bytes read
    /// ahead and the items pulled ahead are lost too. See
    /// [using the handle directly](Unblock#using-the-handle-directly).
    ///
    /// # Example
    ///
    /// ```
    /// use futures::{AsyncWriteExt, executor::block_on};
    /// use zruntime::Unblock;
    ///
    /// block_on(async {
    ///     let mut writer = Unblock::new(Vec::new());
    ///     writer.write_all(b"hello").await?;
    ///
    ///     assert_eq!(writer.into_inner().await, b"hello");
    ///
    ///     std::io::Result::Ok(())
    /// })
    /// .unwrap();
    /// ```
    pub async fn into_inner(mut self) -> T {
        let inner = self.inner();
        future::poll_fn(|cx| inner.poll_idle(cx)).await;

        let inner = self
            .inner
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner);
        let State::Idle(handle) = inner.state else {
            unreachable!("the handle is back once no operation is in flight");
        };
        handle
    }

    /// What the adapter holds.
    ///
    /// An adapter whose handle went with a panic is of no use any more, so this panics for one.
    fn inner(&mut self) -> &mut Inner<T> {
        let inner = self.inner.get_mut().unwrap_or_else(PoisonError::into_inner);
        if let State::Lost = inner.state {
            lost();
        }
        inner
    }
}

// The handle is never pinned, so the adapter is `Unpin` whether the handle is or not.
impl<T> Unpin for Unblock<T> {}

impl<T> fmt::Debug for Unblock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Unblock").finish_non_exhaustive()
    }
}

impl<T> AsyncRead for Unblock<T>
where
    T: Read + Send + 'static,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let inner = self.get_mut().inner();
        // There is nothing to read into, which std's reads take as their cue to read nothing.
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }

        loop {
            // What was read ahead goes first, without waiting for a write in flight: those bytes
            // came before it.
            if let Some(read) = inner.read_ahead.hand_out(buf) {
                return Poll::Ready(read);
            }

            match ready!(inner.poll_job(cx)) {
                Some(outcome) => inner.keep(outcome),
                None => {
                    let mut ahead = inner.read_ahead.take_buf();
                    // A no-op past the first read, which leaves the buffer as long as the capacity.
                    ahead.resize(inner.cap.get(), 0);
                    inner.start(move |handle| {
                        let read = uninterrupted(|| handle.read(&mut ahead));
                        Outcome::Read(ahead, read)
                    });
                }
            }
        }
    }
}

impl<T> AsyncWrite for Unblock<T>
where
    T: Write + Send + 'static,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let inner = self.get_mut().inner();
        ready!(inner.poll_write_ready(cx))?;
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }

        let len = buf.len().min(inner.cap.get());
        let mut bytes = mem::take(&mut inner.write_buf);
        bytes.extend_from_slice(&buf[..len]);
        inner.start(move |handle| {
            let written = handle.write_all(&bytes);
            bytes.clear();
            Outcome::Written(bytes, written)
        });

        // Taken: the bytes are the job's to write, and the next operation waits for it.
        Poll::Ready(Ok(len))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let inner = self.get_mut().inner();

        loop {
            match ready!(inner.poll_job(cx)) {
                // Whether this flush started it or one given up on did, no write can have started
                // after it, as a write waits for the operation in flight: every byte written
                // before this flush has been flushed.
                Some(Outcome::Flushed(flushed)) => return Poll::Ready(flushed),
                Some(outcome) => inner.keep(outcome),
                None => {
                    if let Some(error) = inner.write_error.take() {
                        return Poll::Ready(Err(error));
                    }
                    inner.start(|handle| Outcome::Flushed(uninterrupted(|| handle.flush())));
                }
            }
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // The handle stays open, to be closed when it is dropped: a handle has no close of its own
        // to call, and the adapter has the handle to hand back after this.
        self.poll_flush(cx)
    }
}

impl<T> AsyncSeek for Unblock<T>
where
    T: Seek + Send + 'static,
{
    fn poll_seek(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<io::Result<u64>> {
        let inner = self.get_mut().inner();

        loop {
            match ready!(inner.poll_job(cx)) {
                // A seek polled again after it was pending is asked for the same position. One
                // asked for another is a new seek, after one given up on.
                Some(Outcome::Sought {
                    asked,
                    sought,
                    read_ahead,
                }) if asked == pos => {
                    inner.settle_seek(&sought, read_ahead);
                    return Poll::Ready(sought);
                }
                Some(outcome) => inner.keep(outcome),
                None => {
                    let to = match pos {
                        SeekFrom::Current(offset) => {
                            let Some(offset) = inner.read_ahead.offset_from_handle(offset) else {
                                // Nothing was sought, so what was read ahead stays where it is.
                                return Poll::Ready(Err(io::Error::new(
                                    io::ErrorKind::InvalidInput,
                                    "seek from the current position out of the range of an i64",
                                )));
                            };
                            SeekFrom::Current(offset)
                        }
                        SeekFrom::Start(_) | SeekFrom::End(_) => pos,
                    };
                    let read_ahead = mem::take(&mut inner.read_ahead);
                    inner.start(move |handle| Outcome::Sought {
                        asked: pos,
                        sought: uninterrupted(|| handle.seek(to)),
                        read_ahead,
                    });
                }
            }
        }
    }
}

impl<T> Stream for Unblock<T>
where
    T: Iterator + Send + 'static,
    T::Item: Send + 'static,
{
    type Item = T::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T::Item>> {
        let inner = self.get_mut().inner();

        loop {
            if let Some(batch) = &mut inner.items {
                let Some(batch) = batch.downcast_mut::<Batch<T::Item>>() else {
                    unreachable!("the items pulled ahead are the iterator's own");
                };
                if let Some(item) = batch.items.pop_front() {
                    return Poll::Ready(Some(item));
                }
                // Reported once: polled again, the stream pulls from the iterator again, as a
                // call of `next` after the end of an iterator does.
                if mem::take(&mut batch.ended) {
                    return Poll::Ready(None);
                }
            }

            match ready!(inner.poll_job(cx)) {
                Some(outcome) => inner.keep(outcome),
                None => {
                    // The batch before, emptied, is filled again, so as not to allocate another.
                    let mut batch = match inner.items.take().map(|batch| batch.downcast()) {
                        Some(Ok(batch)) => batch,
                        Some(Err(_)) => {
                            unreachable!("the items pulled ahead are the iterator's own")
                        }
                        None => Box::new(Batch {
                            items: VecDeque::new(),
                            ended: false,
                        }),
                    };
                    let size = inner.cap.get().min(BATCH);
                    inner.start(move |iter| {
                        batch.pull(iter, size);
                        Outcome::Pulled(batch)
                    });
                }
            }
        }
    }
}

/// What an adapter holds: the handle, or the operation that has it, and what the operations keep
/// for those that come after them.
struct Inner<T> {
    state: State<T>,
    /// The bytes read from the handle ahead of the reads of the adapter.
    read_ahead: ReadAhead,
    /// The buffer that a write copies its bytes into for the job to write, handed back by the job
    /// for the next write, emptied.
    write_buf: Vec<u8>,
    /// The error that a write or flush job ran into, found by an operation of another kind, kept
    /// for the next write or flush to report.
    write_error: Option<io::Error>,
    /// The items pulled from the iterator ahead of the stream: a `Batch<T::Item>`, in a box that
    /// does not name its type, as this one does not know that `T` is an iterator.
    items: Option<Box<dyn Any + Send>>,
    /// The tasks waiting for the job in flight.
    waiters: Arc<Waiters>,
    /// The most bytes a job reads ahead or writes.
    cap: NonZeroUsize,
}

impl<T> Inner<T> {
    /// Waits for the job in flight, if there is one, and hands back its outcome, the handle being
    /// back in place by then.
    fn poll_job(&mut self, cx: &mut Context<'_>) -> Poll<Option<Outcome>> {
        // Out of its place while it is polled, so that a panic in the job, which the poll raises
        // again, leaves the adapter without the handle that went with it.
        let mut work = match mem::replace(&mut self.state, State::Lost) {
            State::Busy(work) => work,
            state @ State::Idle(_) => {
                self.state = state;
                return Poll::Ready(None);
            }
            State::Lost => lost(),
        };

        // Counted in before the poll, so that a job that ends right after it does not wake the
        // others waiting and leave this task out.
        self.waiters.add(cx.waker());
        let waker = Waker::from(self.waiters.clone());
        let polled = panic::catch_unwind(AssertUnwindSafe(|| {
            Pin::new(&mut work).poll(&mut Context::from_waker(&waker))
        }));

        match polled {
            Ok(Poll::Pending) => {
                self.state = State::Busy(work);
                Poll::Pending
            }
            Ok(Poll::Ready(Done { handle, outcome })) => {
                self.state = State::Idle(handle);
                // The job wakes the others itself, unless this poll took the outcome before it
                // could.
                if let Some(panic) = self.waiters.wake_others(cx.waker()) {
                    // Kept for the operations it is for, as a later poll would keep it, so that a
                    // waker that panics loses nothing of what the job did: the bytes it read, say.
                    self.keep(outcome);
                    panic::resume_unwind(panic);
                }
                Poll::Ready(Some(outcome))
            }
            Err(panic) => {
                // Those waiting with this task find the handle gone and panic in turn, rather than
                // wait for a job that is over. The panic of the job is the one raised: that of a
                // waker woken after it goes no further.
                if let Some(waker_panic) = self.waiters.wake_others(cx.waker()) {
                    dispose(waker_panic);
                }
                panic::resume_unwind(panic)
            }
        }
    }

    /// Waits until no job is in flight, keeping the outcome of the one there was for the
    /// operations it was for.
    fn poll_idle(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        while let Some(outcome) = ready!(self.poll_job(cx)) {
            self.keep(outcome);
        }
        Poll::Ready(())
    }

    /// Waits until no job is in flight, and hands back the error of a write or flush that no write
    /// or flush has reported yet.
    fn poll_write_ready(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        ready!(self.poll_idle(cx));
        Poll::Ready(self.write_error.take().map_or(Ok(()), Err))
    }

    /// Keeps the outcome of a job for the operations it is for: what a read read for the reads
    /// after it, the error a write ran into for the next write or flush, and so on.
    fn keep(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Read(buf, read) => self.read_ahead.fill(buf, read),
            Outcome::Written(buf, written) => {
                self.write_buf = buf;
                if let Err(error) = written {
                    self.write_error = Some(error);
                }
            }
            Outcome::Flushed(flushed) => {
                if let Err(error) = flushed {
                    self.write_error = Some(error);
                }
            }
            // Given up on, the seek has moved the handle's position all the same.
            Outcome::Sought {
                sought, read_ahead, ..
            } => self.settle_seek(&sought, read_ahead),
            Outcome::Pulled(batch) => self.items = Some(batch),
            // What a `with_mut` whose future was dropped returned, which is nobody's to take.
            Outcome::Ran(_) => {}
        }
    }

    /// Puts back what was read ahead before a seek, if the seek failed, and drops it otherwise.
    fn settle_seek(&mut self, sought: &io::Result<u64>, mut read_ahead: ReadAhead) {
        // A seek that fails leaves the handle's position where it was, on most handles, and so
        // past the bytes read ahead as before: they are kept for the reads that follow, as std's
        // `BufReader` keeps its own.
        if sought.is_ok() {
            read_ahead.clear();
        }
        self.read_ahead = read_ahead;
    }

    /// Drops the bytes read ahead and the items pulled ahead, for the handle to be used directly.
    fn drop_ahead(&mut self) {
        self.read_ahead.clear();
        self.items = None;
    }

    /// Hands the handle over to a job that runs `op` on it, and hands it back along with what `op`
    /// returned.
    ///
    /// No job may be in flight.
    fn start<F>(&mut self, op: F)
    where
        T: Send + 'static,
        F: FnOnce(&mut T) -> Outcome + Send + 'static,
    {
        let State::Idle(mut handle) = mem::replace(&mut self.state, State::Lost) else {
            unreachable!("a job starts only once the one before it is over");
        };
        self.state = State::Busy(unblock(move || {
            let outcome = op(&mut handle);
            Done { handle, outcome }
        }));
    }
}

/// Where the handle of an adapter is.
enum State<T> {
    /// In the adapter, with no job in flight.
    Idle(T),
    /// With the job in flight, which hands it back when it is over.
    Busy(BlockingWork<Done<T>>),
    /// Gone with a panic in a job.
    Lost,
}

/// What a job hands back: the handle, and what it did with it.
struct Done<T> {
    handle: T,
    outcome: Outcome,
}

/// What a job did with the handle.
enum Outcome {
    /// Read into the buffer of the bytes read ahead, which comes back with the bytes.
    Read(Vec<u8>, io::Result<usize>),
    /// Wrote the bytes of the buffer, which comes back for the next write.
    Written(Vec<u8>, io::Result<()>),
    /// Flushed the handle.
    Flushed(io::Result<()>),
    /// Sought, for the position asked for, along with the bytes read ahead before it, which a seek
    /// that fails leaves valid.
    Sought {
        asked: SeekFrom,
        sought: io::Result<u64>,
        read_ahead: ReadAhead,
    },
    /// Pulled a batch of items from the iterator: a `Box<Batch<T::Item>>`.
    Pulled(Box<dyn Any + Send>),
    /// Ran the operation that `with_mut` was given, which returned this.
    Ran(Box<dyn Any + Send>),
}

/// The bytes read from the handle ahead of the reads of an adapter, and what ended the last read
/// that read none.
#[derive(Default)]
struct ReadAhead {
    /// What the reads read into, as long as the capacity once one has.
    buf: Vec<u8>,
    /// Where the bytes not handed out yet start in `buf`.
    start: usize,
    /// Where they end.
    end: usize,
    /// The end of the handle, or the error, that the last read ran into, if no read has reported
    /// it yet.
    stop: Option<io::Result<()>>,
}

impl ReadAhead {
    /// Hands out as many bytes read ahead as `buf` takes, or else what ended the last read, if
    /// there are any.
    fn hand_out(&mut self, buf: &mut [u8]) -> Option<io::Result<usize>> {
        let ahead = &self.buf[self.start..self.end];
        if ahead.is_empty() {
            // The end of the handle is a read of nothing.
            return self.stop.take().map(|stop| stop.map(|()| 0));
        }

        let len = ahead.len().min(buf.len());
        buf[..len].copy_from_slice(&ahead[..len]);
        self.start += len;
        Some(Ok(len))
    }

    /// The buffer, for a read to read into, every byte in it having been handed out.
    fn take_buf(&mut self) -> Vec<u8> {
        self.start = 0;
        self.end = 0;
        mem::take(&mut self.buf)
    }

    /// Takes in what a read read into `buf`.
    fn fill(&mut self, buf: Vec<u8>, read: io::Result<usize>) {
        self.start = 0;
        self.end = 0;
        match read {
            Ok(0) => self.stop = Some(Ok(())),
            // A reader that claims more than the buffer holds read no more than the buffer.
            Ok(len) => self.end = len.min(buf.len()),
            Err(error) => self.stop = Some(Err(error)),
        }
        self.buf = buf;
    }

    /// The offset from the handle's position of the one `offset` away from where the reads got to:
    /// behind the reads by the bytes read ahead, if that is in the range of an `i64`.
    fn offset_from_handle(&self, offset: i64) -> Option<i64> {
        offset.checked_sub(i64::try_from(self.end - self.start).ok()?)
    }

    /// Drops the bytes read ahead and what ended the last read, keeping the buffer.
    fn clear(&mut self) {
        self.start = 0;
        self.end = 0;
        self.stop = None;
    }
}

/// Items pulled from an iterator ahead of the stream of an adapter.
struct Batch<I> {
    items: VecDeque<I>,
    /// Whether the iterator ended after these items.
    ended: bool,
}

impl<I> Batch<I> {
    /// Pulls items from `iter` until the batch holds `size` of them, or the iterator ends.
    fn pull<T>(&mut self, iter: &mut T, size: usize)
    where
        T: Iterator<Item = I>,
    {
        while self.items.len() < size {
            let Some(item) = iter.next() else {
                self.ended = true;
                return;
            };
            self.items.push_back(item);
        }
    }
}

/// The tasks waiting for the job in flight of an adapter.
///
/// A job wakes one waker, but more than one task may wait for the same job: the reading and the
/// writing half of a split adapter, say, the one polled after the other while a read is in flight.
/// The job is polled with the waker of this instead, which wakes every one of them.
#[derive(Default)]
struct Waiters(Mutex<Vec<Waker>>);

impl Waiters {
    /// Counts in the task that `waker` wakes, unless it is in already.
    fn add(&self, waker: &Waker) {
        let mut wakers = self.wakers();
        if !wakers.iter().any(|waiting| waiting.will_wake(waker)) {
            wakers.push(waker.clone());
        }
    }

    /// Wakes the tasks waiting, but for the one that `waker` wakes, which is running already, and
    /// hands back the panic of the first waker that panicked, if one did: see [`wake_all`].
    fn wake_others(&self, waker: &Waker) -> Option<Box<dyn Any + Send>> {
        let wakers = mem::take(&mut *self.wakers());
        // Past the lock, which a waker that polls the adapter there and then takes again.
        wake_all(
            wakers
                .into_iter()
                .filter(|waiting| !waiting.will_wake(waker)),
        )
    }

    /// The wakers of the tasks waiting, behind their lock, taken whether or not a panic poisoned
    /// it.
    fn wakers(&self) -> MutexGuard<'_, Vec<Waker>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Wake for Waiters {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        let wakers = mem::take(&mut *self.wakers());
        // Past the lock, as in `wake_others`. The panic of a waker reaches the job that wakes this,
        // once every task waiting has been woken.
        if let Some(panic) = wake_all(wakers) {
            panic::resume_unwind(panic);
        }
    }
}

/// Wakes each of `wakers`, every one of them even where one before it panics, and hands back the
/// panic of the first that panicked, if one did.
///
/// A task left unwoken would wait for good for an operation that is over. The payload of any panic
/// after the first is disposed of: dropped with a panic of its own destructor caught, as that would
/// otherwise escape the loop, past the wakers still to wake.
fn wake_all<I>(wakers: I) -> Option<Box<dyn Any + Send>>
where
    I: IntoIterator<Item = Waker>,
{
    let mut first_panic = None;
    for waker in wakers {
        if let Err(panic) = panic::catch_unwind(AssertUnwindSafe(|| waker.wake())) {
            match first_panic {
                None => first_panic = Some(panic),
                Some(_) => dispose(panic),
            }
        }
    }

    first_panic
}

/// Runs `op`, a read, a flush or a seek of the handle, again for as long as it is interrupted, as
/// std's own loops of reads and writes do.
///
/// An interruption is no failure of the operation, and nothing but trying again is left to do.
/// `futures-io` does not let a read, a flush or a seek report one either, nor does the adapter's
/// write, which std's `write_all` already tries again.
fn uninterrupted<F, R>(mut op: F) -> io::Result<R>
where
    F: FnMut() -> io::Result<R>,
{
    loop {
        match op() {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            outcome => return outcome,
        }
    }
}

/// Panics, for an adapter whose handle went with a panic in a job.
fn lost() -> ! {
    panic!("the handle of an `Unblock` was lost to a panic in an operation on it");
}

/// The capacity of an adapter made by [`Unblock::new`].
const DEFAULT_CAPACITY: NonZeroUsize = NonZeroUsize::new(8 * 1024).unwrap();

/// The most items a job pulls from an iterator.
const BATCH: usize = 16;
