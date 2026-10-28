//! An async multi-producer multi-consumer channel in which each message goes to one receiver.
//!
//! [`bounded`] and [`unbounded`] create a channel and return its first [`Sender`] and
//! [`Receiver`]. Both can be cloned. Both can be sent to other threads, and shared with them, if
//! the message type `T` is [`Send`](std::marker::Send).
//!
//! A sender moves each message into the channel, and a receiver moves it out. Messages are never
//! cloned, so `T` need not be [`Clone`]. Messages are received in the order they were sent. Each
//! message goes to exactly one receiver.
//!
//! A bounded channel holds at most as many messages as its capacity. [`send`](Sender::send) waits
//! while the channel is full, and [`try_send`](Sender::try_send) fails. An unbounded channel holds
//! any number of messages, and sending to it never waits.
//!
//! A receiver receives through `&self`, so several tasks can share one receiver by reference and
//! each wait for the next message. A receiver is also a [`Stream`] of messages. The stream ends
//! when the channel is closed and empty.
//!
//! A channel is closed by [`Sender::close`], by [`Receiver::close`], when its last sender is
//! dropped, and when its last receiver is dropped. Sending to a closed channel fails and returns
//! the message. Receiving still returns the messages left in the channel, and fails only when none
//! is left. When the last receiver is dropped, the messages left in the channel are dropped at
//! once, because nothing can receive them.
//!
//! The channel needs no runtime and works under any executor. It can be used from any thread,
//! inside a task or not. A thread with no task can wait for a message with the `block_on` of any
//! executor, as the example below does.
//!
//! # Example
//!
//! Two senders and two receivers on a channel that holds two messages:
//!
//! ```
//! use std::num::NonZeroUsize;
//!
//! use futures::executor::block_on;
//! use zruntime::mpmc::{RecvError, TrySendError, bounded};
//!
//! block_on(async {
//!     let (s1, r1) = bounded(NonZeroUsize::new(2).unwrap());
//!     let s2 = s1.clone();
//!     let r2 = r1.clone();
//!
//!     // Two messages, from two different senders.
//!     s1.send("hello").await.unwrap();
//!     s2.send("world").await.unwrap();
//!
//!     // The channel is full, so `try_send` fails. A `send` would wait for room.
//!     assert_eq!(s1.try_send("again"), Err(TrySendError::Full("again")));
//!
//!     // Each message goes to one receiver, oldest first.
//!     assert_eq!(r1.recv().await, Ok("hello"));
//!     assert_eq!(r2.recv().await, Ok("world"));
//!     assert!(r1.try_recv().unwrap_err().is_empty());
//!
//!     // Dropping the last sender closes the channel. The receivers see that once there is no
//!     // message left in it.
//!     drop(s1);
//!     drop(s2);
//!     assert_eq!(r1.recv().await, Err(RecvError));
//!     assert_eq!(r2.recv().await, Err(RecvError));
//! });
//! ```
//!
//! # Spreading work over threads
//!
//! A runtime of this crate runs its tasks on one thread at a time, so all of its tasks share one
//! core. To use more cores, run one runtime per thread and spread the work over them with a
//! channel. Each thread waits for the next job on its own clone of the same receiver. Each job goes
//! to whichever thread takes it first, so a busy thread leaves new jobs to the others. The results
//! come back through a second channel.
//!
//! ```
//! # #[cfg(feature = "runtime")]
//! # {
//! use std::{num::NonZeroUsize, thread};
//!
//! use zruntime::{LocalRuntime, mpmc};
//!
//! let (jobs, job_receiver) = mpmc::bounded::<u64>(NonZeroUsize::new(16).unwrap());
//! let (result_sender, results) = mpmc::unbounded();
//!
//! let workers: Vec<_> = (0..4)
//!     .map(|_| {
//!         let jobs = job_receiver.clone();
//!         let results = result_sender.clone();
//!
//!         thread::spawn(move || {
//!             // A runtime that belongs to this thread.
//!             let runtime = LocalRuntime::new().unwrap();
//!             runtime.block_on(async {
//!                 // The loop ends when the channel is closed and empty.
//!                 while let Ok(n) = jobs.recv().await {
//!                     let sum: u64 = (1..=n).sum();
//!                     results.send(sum).await.unwrap();
//!                 }
//!             });
//!         })
//!     })
//!     .collect();
//! // The workers are the only receivers of jobs and senders of results from here on. The loop
//! // over the results below ends only when every result sender is dropped, including this one.
//! drop(job_receiver);
//! drop(result_sender);
//!
//! let runtime = LocalRuntime::new().unwrap();
//! let total = runtime.block_on(async {
//!     for n in 1..=100 {
//!         jobs.send(n).await.unwrap();
//!     }
//!     // No more jobs. The workers' loops end after they take the last one.
//!     drop(jobs);
//!
//!     // Results arrive in the order the workers finish their jobs. The loop ends when every
//!     // worker has dropped its sender.
//!     let mut total = 0;
//!     while let Ok(sum) = results.recv().await {
//!         total += sum;
//!     }
//!     total
//! });
//!
//! for worker in workers {
//!     worker.join().unwrap();
//! }
//! assert_eq!(total, (1..=100u64).map(|n| n * (n + 1) / 2).sum());
//! # }
//! ```
//!
//! If jobs mostly wait for I/O, a worker can spawn a task for each job on its runtime. The jobs
//! then wait at the same time instead of one after the other.
//!
//! # Giving up a wait
//!
//! It is safe to drop the future returned by [`Sender::send`] or [`Receiver::recv`] before it
//! completes. A timeout or a `select` can do this. A dropped [`Recv`] has taken no message. A
//! dropped [`Send`] has sent none, and its message is dropped with it. If a future was woken for a
//! message or for room and is then dropped, the wake-up passes to the next task waiting, so no
//! waiting task is left stranded.
//!
//! # Difference with `broadcast`
//!
//! The [`broadcast`] module has the same two sides, a sender and a receiver. It delivers every
//! message to every receiver, as a clone, so its messages must be [`Clone`]. This channel delivers
//! each message to one receiver only, and moves it there. Use `broadcast` when every receiver
//! must see every message, and this channel when receivers share the work.
//!
//! The `broadcast` module requires the `broadcast` feature.
//!
//! # Difference with `async-channel`
//!
//! This channel is modelled on [`async-channel`]. It differs in these ways:
//!
//! * The capacity of [`bounded`] is a [`NonZeroUsize`], so the type rules out a zero capacity.
//!   `async-channel` panics on `bounded(0)`.
//! * Dropping the last receiver drops the messages left in the channel. `async-channel` only closes
//!   the channel, and the messages stay in it until the channel itself is freed.
//! * There is no blocking API. Instead of `send_blocking` and `recv_blocking`, a thread with no
//!   task can wait with the `block_on` of an executor.
//!
//! # Performance
//!
//! Benchmarks compared this channel with [`async-channel`] 2.5.0 and with the [`mpsc`] channel of
//! tokio 1.53.1. Tokio's channel has only one receiver, so it is left out of the rows with several
//! receivers. The messages are `u64`s. The machine is a shared x86-64 one with four cores. Each
//! thread drives its whole part with one `block_on` call of `futures-lite`. Tasks run on a tokio
//! runtime with four workers.
//!
//! In the rows with threads or tasks, each sender sends 10,000 messages, and the row times all of
//! them. The row marked "a message" times one message. The times are medians. Times on one thread
//! stayed within 3% from run to run. The others varied by up to 30%. Sending to tokio's unbounded
//! channel is not a future, so it needs no poll.
//!
//! | What is timed                                             | `mpmc` | async-channel |  tokio |
//! |-----------------------------------------------------------|-------:|--------------:|-------:|
//! | A send and a receive, one thread, capacity 1              |  55 ns |        112 ns | 100 ns |
//! | A send and a receive, one thread, unbounded               |  54 ns |        123 ns |  56 ns |
//! | 1024 `try_send`s then 1024 `try_recv`s, capacity 1024     |  36 µs |         99 µs |  67 µs |
//! | 1024 `try_send`s then 1024 `try_recv`s, unbounded         |  36 µs |        112 µs |  47 µs |
//! | 4 sender and 4 receiver threads, capacity 16              |  72 ms |        240 ms |        |
//! | 4 sender and 4 receiver threads, unbounded                | 9.7 ms |         16 ms |        |
//! | 4 sender tasks and 1 receiver task, capacity 16           |  12 ms |         33 ms |  13 ms |
//! | 4 sender and 4 receiver tasks, capacity 16                |  18 ms |         24 ms |        |
//! | 1 sender task and 1 receiver task, capacity 16, a message | 280 ns |        550 ns | 540 ns |
//! | 1 sender thread and 1 receiver thread, capacity 16        |  23 ms |         20 ms |  22 ms |
//! | 1 sender thread and 1 receiver thread, capacity 1024      | 2.3 ms |        1.7 ms | 2.8 ms |
//! | 4 sender threads and 1 receiver thread, capacity 16       | 470 ms |        500 ms | 460 ms |
//!
//! In the last three rows, the channel keeps filling up or running empty, so a thread has to wait
//! every few messages. Waiting parks the thread, and waking it takes the OS tens of microseconds
//! on that machine. That wake-up makes up nearly all of the time. So in these rows the channels
//! differ in how often a thread had to wait, not in the cost of an operation. Between tasks, which
//! wake cheaply, this channel takes half as long as the other two, as the "a message" row shows.
//!
//! [`Stream`]: futures_core::Stream
//! [`NonZeroUsize`]: std::num::NonZeroUsize
//! [`broadcast`]: https://docs.rs/zruntime/latest/zruntime/broadcast/index.html
//! [`async-channel`]: https://crates.io/crates/async-channel
//! [`mpsc`]: https://docs.rs/tokio/1.53.1/tokio/sync/mpsc/index.html

use std::{
    any::Any,
    collections::VecDeque,
    error, fmt,
    future::Future,
    mem,
    num::NonZeroUsize,
    panic::{self, AssertUnwindSafe},
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    task::{Context, Poll},
};

use futures_core::{
    ready,
    stream::{FusedStream, Stream},
};

use crate::{Event, EventListener};

/// Creates a channel that holds at most `cap` messages.
///
/// Returns the first sender and receiver. [`send`](Sender::send) waits while the channel is full,
/// and [`try_send`](Sender::try_send) fails.
///
/// The channel allocates room as messages arrive, not `cap` messages up front. A very large
/// capacity costs nothing until the messages are there.
///
/// # Example
///
/// ```
/// use std::num::NonZeroUsize;
///
/// use zruntime::mpmc::{TrySendError, bounded};
///
/// let (s, r) = bounded(NonZeroUsize::MIN);
///
/// s.try_send(1).unwrap();
/// // The channel holds one message at most.
/// assert_eq!(s.try_send(2), Err(TrySendError::Full(2)));
///
/// assert_eq!(r.try_recv(), Ok(1));
/// s.try_send(2).unwrap();
/// ```
pub fn bounded<T>(cap: NonZeroUsize) -> (Sender<T>, Receiver<T>) {
    channel(Some(cap))
}

/// Creates a channel that holds any number of messages.
///
/// Returns the first sender and receiver. Sending to the channel never waits for room.
///
/// # Example
///
/// ```
/// use zruntime::mpmc::unbounded;
///
/// let (s, r) = unbounded();
///
/// for i in 0..100 {
///     s.try_send(i).unwrap();
/// }
/// assert_eq!(s.len(), 100);
/// assert_eq!(r.try_recv(), Ok(0));
/// ```
pub fn unbounded<T>() -> (Sender<T>, Receiver<T>) {
    channel(None)
}

/// The sending side of a channel.
///
/// Senders can be cloned and shared among threads. When the last sender is dropped, the channel is
/// closed. No more messages can be sent, but the messages in the channel can still be received.
///
/// [`Sender::close`] also closes the channel.
pub struct Sender<T> {
    channel: Arc<Channel<T>>,
}

impl<T> Sender<T> {
    /// Sends `msg` into the channel.
    ///
    /// The returned future completes when the message is in the channel. If the channel is full,
    /// it waits for a receiver to make room. An unbounded channel is never full.
    ///
    /// # Errors
    ///
    /// Fails with a [`SendError`], which returns the message, if the channel is closed. This
    /// includes a channel that is closed while the future waits for room.
    ///
    /// # Cancel safety
    ///
    /// Dropping the future before it completes is safe. Nothing is sent, and the message is
    /// dropped.
    ///
    /// # Example
    ///
    /// Sending to a full channel waits until a receive makes room:
    ///
    /// ```
    /// use std::num::NonZeroUsize;
    ///
    /// use futures::{executor::block_on, future::join};
    /// use zruntime::mpmc::bounded;
    ///
    /// let (s, r) = bounded(NonZeroUsize::MIN);
    ///
    /// block_on(async {
    ///     s.send(1).await.unwrap();
    ///
    ///     let (sent, received) = join(s.send(2), r.recv()).await;
    ///     assert_eq!(sent, Ok(()));
    ///     assert_eq!(received, Ok(1));
    ///
    ///     assert_eq!(r.recv().await, Ok(2));
    /// });
    /// ```
    pub fn send(&self, msg: T) -> Send<'_, T> {
        Send {
            sender: self,
            listener: None,
            msg: Some(msg),
        }
    }

    /// Tries to send `msg` into the channel without waiting.
    ///
    /// # Errors
    ///
    /// Fails with [`TrySendError::Full`] if the channel is full. Fails with
    /// [`TrySendError::Closed`] if the channel is closed, even if it has room. Both errors return
    /// the message.
    ///
    /// # Example
    ///
    /// ```
    /// use std::num::NonZeroUsize;
    ///
    /// use zruntime::mpmc::{TrySendError, bounded};
    ///
    /// let (s, r) = bounded(NonZeroUsize::MIN);
    ///
    /// assert_eq!(s.try_send(1), Ok(()));
    /// assert_eq!(s.try_send(2), Err(TrySendError::Full(2)));
    ///
    /// drop(r);
    /// assert_eq!(s.try_send(3), Err(TrySendError::Closed(3)));
    /// ```
    pub fn try_send(&self, msg: T) -> Result<(), TrySendError<T>> {
        self.channel.try_send(msg)
    }

    /// Closes the channel.
    ///
    /// Returns `true` if this call closed the channel, and `false` if it was already closed.
    ///
    /// Closing makes every send fail, including sends that are waiting for room. The messages in
    /// the channel can still be received. Once none is left, receives fail too, including receives
    /// that are waiting for a message.
    ///
    /// # Example
    ///
    /// ```
    /// use zruntime::mpmc::{TryRecvError, TrySendError, unbounded};
    ///
    /// let (s, r) = unbounded();
    /// s.try_send(1).unwrap();
    ///
    /// assert!(s.close());
    /// assert!(!s.close());
    /// assert_eq!(s.try_send(2), Err(TrySendError::Closed(2)));
    ///
    /// assert_eq!(r.try_recv(), Ok(1));
    /// assert_eq!(r.try_recv(), Err(TryRecvError::Closed));
    /// ```
    pub fn close(&self) -> bool {
        self.channel.close()
    }

    /// Whether the channel is closed.
    ///
    /// A channel is closed by [`Sender::close`], by [`Receiver::close`], when its last sender is
    /// dropped, and when its last receiver is dropped. Receivers can still receive the messages
    /// that a closed channel holds.
    ///
    /// # Example
    ///
    /// ```
    /// use zruntime::mpmc::unbounded;
    ///
    /// let (s, r) = unbounded::<()>();
    /// assert!(!s.is_closed());
    ///
    /// drop(r);
    /// assert!(s.is_closed());
    /// ```
    pub fn is_closed(&self) -> bool {
        lock(&self.channel.state).closed
    }

    /// Whether the channel holds no messages.
    ///
    /// # Example
    ///
    /// ```
    /// use zruntime::mpmc::unbounded;
    ///
    /// let (s, _r) = unbounded();
    /// assert!(s.is_empty());
    ///
    /// s.try_send(1).unwrap();
    /// assert!(!s.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        lock(&self.channel.state).queue.is_empty()
    }

    /// Whether the channel holds as many messages as its capacity.
    ///
    /// An unbounded channel is never full.
    ///
    /// # Example
    ///
    /// ```
    /// use std::num::NonZeroUsize;
    ///
    /// use zruntime::mpmc::{bounded, unbounded};
    ///
    /// let (s, _r) = bounded(NonZeroUsize::MIN);
    /// assert!(!s.is_full());
    /// s.try_send(1).unwrap();
    /// assert!(s.is_full());
    ///
    /// let (s, _r) = unbounded();
    /// s.try_send(1).unwrap();
    /// assert!(!s.is_full());
    /// ```
    pub fn is_full(&self) -> bool {
        lock(&self.channel.state).is_full()
    }

    /// The number of messages in the channel.
    ///
    /// Other threads can send and receive at the same time, so the number can be out of date as
    /// soon as this returns.
    ///
    /// # Example
    ///
    /// ```
    /// use zruntime::mpmc::unbounded;
    ///
    /// let (s, r) = unbounded();
    /// assert_eq!(s.len(), 0);
    ///
    /// s.try_send(1).unwrap();
    /// s.try_send(2).unwrap();
    /// assert_eq!(s.len(), 2);
    ///
    /// r.try_recv().unwrap();
    /// assert_eq!(s.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        lock(&self.channel.state).queue.len()
    }

    /// The maximum number of messages the channel holds, or `None` if it is unbounded.
    ///
    /// # Example
    ///
    /// ```
    /// use std::num::NonZeroUsize;
    ///
    /// use zruntime::mpmc::{bounded, unbounded};
    ///
    /// let (s, _r) = bounded::<()>(NonZeroUsize::new(5).unwrap());
    /// assert_eq!(s.capacity(), NonZeroUsize::new(5));
    ///
    /// let (s, _r) = unbounded::<()>();
    /// assert_eq!(s.capacity(), None);
    /// ```
    pub fn capacity(&self) -> Option<NonZeroUsize> {
        lock(&self.channel.state).capacity
    }

    /// The number of senders of the channel.
    ///
    /// # Example
    ///
    /// ```
    /// use zruntime::mpmc::unbounded;
    ///
    /// let (s1, _r) = unbounded::<()>();
    /// assert_eq!(s1.sender_count(), 1);
    ///
    /// let s2 = s1.clone();
    /// assert_eq!(s1.sender_count(), 2);
    ///
    /// drop(s2);
    /// assert_eq!(s1.sender_count(), 1);
    /// ```
    pub fn sender_count(&self) -> usize {
        lock(&self.channel.state).senders
    }

    /// The number of receivers of the channel.
    ///
    /// # Example
    ///
    /// ```
    /// use zruntime::mpmc::unbounded;
    ///
    /// let (s, r1) = unbounded::<()>();
    /// assert_eq!(s.receiver_count(), 1);
    ///
    /// let r2 = r1.clone();
    /// assert_eq!(s.receiver_count(), 2);
    ///
    /// drop((r1, r2));
    /// assert_eq!(s.receiver_count(), 0);
    /// ```
    pub fn receiver_count(&self) -> usize {
        lock(&self.channel.state).receivers
    }
}

impl<T> Clone for Sender<T> {
    /// Creates another sender of the same channel.
    ///
    /// # Example
    ///
    /// ```
    /// use zruntime::mpmc::{TryRecvError, unbounded};
    ///
    /// let (s1, r) = unbounded();
    /// let s2 = s1.clone();
    ///
    /// // The channel stays open while at least one sender exists.
    /// drop(s1);
    /// s2.try_send(1).unwrap();
    /// assert_eq!(r.try_recv(), Ok(1));
    /// assert_eq!(r.try_recv(), Err(TryRecvError::Empty));
    ///
    /// drop(s2);
    /// assert_eq!(r.try_recv(), Err(TryRecvError::Closed));
    /// ```
    fn clone(&self) -> Self {
        lock(&self.channel.state).senders += 1;

        Sender {
            channel: self.channel.clone(),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let closed = {
            let mut state = lock(&self.channel.state);
            state.senders -= 1;

            state.senders == 0 && state.close()
        };

        if closed {
            self.channel.notify_closed();
        }
    }
}

impl<T> fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.channel.fmt_end("Sender", f)
    }
}

/// The receiving side of a channel.
///
/// Receivers can be cloned and shared among threads. [`Receiver::recv`] takes `&self`, so a
/// receiver can also be shared by reference. Each message goes to one receiver. When the last
/// receiver is dropped, the channel is closed and the messages left in it are dropped, because
/// nothing can receive them.
///
/// [`Receiver::close`] also closes the channel.
///
/// A receiver is also a [`Stream`] of messages and a [`FusedStream`]. The stream ends when the
/// channel is closed and empty.
///
/// # Example
///
/// ```
/// use futures::{executor::block_on, stream::StreamExt};
/// use zruntime::mpmc::unbounded;
///
/// let (s, r) = unbounded();
/// s.try_send(1).unwrap();
/// s.try_send(2).unwrap();
/// drop(s);
///
/// assert_eq!(block_on(r.collect::<Vec<_>>()), [1, 2]);
/// ```
///
/// [`Stream`]: futures_core::Stream
/// [`FusedStream`]: futures_core::stream::FusedStream
pub struct Receiver<T> {
    channel: Arc<Channel<T>>,
    /// What the stream of this receiver waits on, kept from one poll of it to the next. A `recv`
    /// keeps its own listener in its future, so the two never share one.
    listener: Option<EventListener>,
}

impl<T> Receiver<T> {
    /// Receives the oldest message in the channel.
    ///
    /// The returned future completes with the message. If the channel has none, it waits for one
    /// to be sent.
    ///
    /// # Errors
    ///
    /// Fails with a [`RecvError`] if the channel is closed and has no message left. A closed
    /// channel still returns the messages it holds.
    ///
    /// # Cancel safety
    ///
    /// Dropping the future before it completes is safe. It takes no message from the channel.
    ///
    /// # Example
    ///
    /// ```
    /// use futures::executor::block_on;
    /// use zruntime::mpmc::{RecvError, unbounded};
    ///
    /// let (s, r) = unbounded();
    /// s.try_send(1).unwrap();
    /// drop(s);
    ///
    /// block_on(async {
    ///     assert_eq!(r.recv().await, Ok(1));
    ///     // The channel is closed, and there is no message left in it.
    ///     assert_eq!(r.recv().await, Err(RecvError));
    /// });
    /// ```
    pub fn recv(&self) -> Recv<'_, T> {
        Recv {
            receiver: self,
            listener: None,
        }
    }

    /// Tries to receive the oldest message in the channel without waiting.
    ///
    /// # Errors
    ///
    /// Fails with [`TryRecvError::Empty`] if the channel has no message and is not closed. Fails
    /// with [`TryRecvError::Closed`] if it has no message and is closed. A closed channel still
    /// returns the messages left in it.
    ///
    /// # Example
    ///
    /// ```
    /// use zruntime::mpmc::{TryRecvError, unbounded};
    ///
    /// let (s, r) = unbounded();
    /// assert_eq!(r.try_recv(), Err(TryRecvError::Empty));
    ///
    /// s.try_send(1).unwrap();
    /// assert_eq!(r.try_recv(), Ok(1));
    ///
    /// drop(s);
    /// assert_eq!(r.try_recv(), Err(TryRecvError::Closed));
    /// ```
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        self.channel.try_recv()
    }

    /// Closes the channel.
    ///
    /// Returns `true` if this call closed the channel, and `false` if it was already closed.
    ///
    /// Closing makes every send fail, including sends that are waiting for room. The messages in
    /// the channel can still be received. Once none is left, receives fail too, including receives
    /// that are waiting for a message.
    ///
    /// # Example
    ///
    /// ```
    /// use zruntime::mpmc::{TryRecvError, TrySendError, unbounded};
    ///
    /// let (s, r) = unbounded();
    /// s.try_send(1).unwrap();
    ///
    /// assert!(r.close());
    /// assert!(!r.close());
    /// assert_eq!(s.try_send(2), Err(TrySendError::Closed(2)));
    ///
    /// assert_eq!(r.try_recv(), Ok(1));
    /// assert_eq!(r.try_recv(), Err(TryRecvError::Closed));
    /// ```
    pub fn close(&self) -> bool {
        self.channel.close()
    }

    /// Whether the channel is closed.
    ///
    /// See [`Sender::is_closed`] for what closes a channel.
    ///
    /// # Example
    ///
    /// ```
    /// use zruntime::mpmc::unbounded;
    ///
    /// let (s, r) = unbounded::<()>();
    /// assert!(!r.is_closed());
    ///
    /// drop(s);
    /// assert!(r.is_closed());
    /// ```
    pub fn is_closed(&self) -> bool {
        lock(&self.channel.state).closed
    }

    /// Whether the channel holds no messages.
    ///
    /// # Example
    ///
    /// ```
    /// use zruntime::mpmc::unbounded;
    ///
    /// let (s, r) = unbounded();
    /// assert!(r.is_empty());
    ///
    /// s.try_send(1).unwrap();
    /// assert!(!r.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        lock(&self.channel.state).queue.is_empty()
    }

    /// Whether the channel holds as many messages as its capacity.
    ///
    /// An unbounded channel is never full.
    ///
    /// # Example
    ///
    /// ```
    /// use std::num::NonZeroUsize;
    ///
    /// use zruntime::mpmc::bounded;
    ///
    /// let (s, r) = bounded(NonZeroUsize::MIN);
    /// assert!(!r.is_full());
    ///
    /// s.try_send(1).unwrap();
    /// assert!(r.is_full());
    /// ```
    pub fn is_full(&self) -> bool {
        lock(&self.channel.state).is_full()
    }

    /// The number of messages in the channel.
    ///
    /// Other threads can send and receive at the same time, so the number can be out of date as
    /// soon as this returns.
    ///
    /// # Example
    ///
    /// ```
    /// use zruntime::mpmc::unbounded;
    ///
    /// let (s, r) = unbounded();
    /// s.try_send(1).unwrap();
    /// s.try_send(2).unwrap();
    /// assert_eq!(r.len(), 2);
    ///
    /// r.try_recv().unwrap();
    /// assert_eq!(r.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        lock(&self.channel.state).queue.len()
    }

    /// The maximum number of messages the channel holds, or `None` if it is unbounded.
    ///
    /// # Example
    ///
    /// ```
    /// use std::num::NonZeroUsize;
    ///
    /// use zruntime::mpmc::{bounded, unbounded};
    ///
    /// let (_s, r) = bounded::<()>(NonZeroUsize::new(5).unwrap());
    /// assert_eq!(r.capacity(), NonZeroUsize::new(5));
    ///
    /// let (_s, r) = unbounded::<()>();
    /// assert_eq!(r.capacity(), None);
    /// ```
    pub fn capacity(&self) -> Option<NonZeroUsize> {
        lock(&self.channel.state).capacity
    }

    /// The number of senders of the channel.
    ///
    /// # Example
    ///
    /// ```
    /// use zruntime::mpmc::unbounded;
    ///
    /// let (s1, r) = unbounded::<()>();
    /// let s2 = s1.clone();
    /// assert_eq!(r.sender_count(), 2);
    ///
    /// drop((s1, s2));
    /// assert_eq!(r.sender_count(), 0);
    /// ```
    pub fn sender_count(&self) -> usize {
        lock(&self.channel.state).senders
    }

    /// The number of receivers of the channel.
    ///
    /// # Example
    ///
    /// ```
    /// use zruntime::mpmc::unbounded;
    ///
    /// let (_s, r1) = unbounded::<()>();
    /// assert_eq!(r1.receiver_count(), 1);
    ///
    /// let r2 = r1.clone();
    /// assert_eq!(r1.receiver_count(), 2);
    ///
    /// drop(r2);
    /// assert_eq!(r1.receiver_count(), 1);
    /// ```
    pub fn receiver_count(&self) -> usize {
        lock(&self.channel.state).receivers
    }
}

impl<T> Clone for Receiver<T> {
    /// Creates another receiver of the same channel.
    ///
    /// The receivers share the messages in the channel. A message is not copied: it goes to one
    /// receiver.
    ///
    /// # Example
    ///
    /// ```
    /// use zruntime::mpmc::{TryRecvError, unbounded};
    ///
    /// let (s, r1) = unbounded();
    /// let r2 = r1.clone();
    /// s.try_send(1).unwrap();
    ///
    /// // The message goes to whichever receiver asks first, and to no other.
    /// assert_eq!(r2.try_recv(), Ok(1));
    /// assert_eq!(r1.try_recv(), Err(TryRecvError::Empty));
    /// ```
    fn clone(&self) -> Self {
        lock(&self.channel.state).receivers += 1;

        Receiver {
            channel: self.channel.clone(),
            listener: None,
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        // The receiver is counted out, and the channel closed if it was the last, before any of
        // somebody else's code runs below: the waker the stream's listener holds, the wakers of
        // the operations waiting, and the `Drop` of the messages left. Each may panic, and none is
        // to leave the channel counting a receiver that is gone.
        let (closed, left) = {
            let mut state = lock(&self.channel.state);
            state.receivers -= 1;
            if state.receivers > 0 {
                (false, VecDeque::new())
            } else {
                // Nothing can receive the messages any more, and no sender can make a receiver.
                (state.close(), mem::take(&mut state.queue))
            }
        };

        let listener = self.listener.take();
        run_both(
            // Let go of the stream's listener first: closing the channel would notify it, and
            // wake the task that last polled the stream, for a receiver that is gone.
            || drop(listener),
            || {
                run_both(
                    || {
                        if closed {
                            self.channel.notify_closed();
                        }
                    },
                    // The messages left go last, once every operation waiting has been told.
                    || drop(left),
                )
            },
        );
    }
}

impl<T> fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.channel.fmt_end("Receiver", f)
    }
}

impl<T> Stream for Receiver<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        let this = self.get_mut();

        loop {
            // A listener this stream holds is polled before anything else, as `Recv::poll` does.
            if let Some(listener) = &mut this.listener {
                ready!(Pin::new(listener).poll(cx));
                this.listener = None;
            }

            loop {
                match this.channel.try_recv() {
                    Ok(msg) => {
                        this.listener = None;

                        return Poll::Ready(Some(msg));
                    }
                    Err(TryRecvError::Closed) => {
                        this.listener = None;

                        return Poll::Ready(None);
                    }
                    Err(TryRecvError::Empty) => {}
                }

                // Nothing to receive yet: listen, then try again, and wait on the listener if that
                // finds nothing too.
                match this.listener {
                    None => this.listener = Some(this.channel.stream_ops.listen_unfenced()),
                    Some(_) => break,
                }
            }
        }
    }
}

impl<T> FusedStream for Receiver<T> {
    fn is_terminated(&self) -> bool {
        let state = lock(&self.channel.state);

        state.closed && state.queue.is_empty()
    }
}

/// The future returned by [`Sender::send`].
///
/// It completes with `Ok(())` when the message is in the channel. If the channel is closed, it
/// completes with a [`SendError`] that returns the message.
///
/// # Panics
///
/// Panics if it is polled again after it completed, because the message is gone by then.
#[must_use = "futures do nothing unless .awaited"]
pub struct Send<'a, T> {
    sender: &'a Sender<T>,
    /// What this future waits on while the channel is full, taken before its last try.
    listener: Option<EventListener>,
    /// The message, until it is in the channel or handed back.
    msg: Option<T>,
}

// The message is never pinned, so the future is `Unpin` whether the message is or not.
impl<T> Unpin for Send<'_, T> {}

impl<T> Future for Send<'_, T> {
    type Output = Result<(), SendError<T>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        loop {
            // A listener this future holds is polled before anything else. One that completes has
            // taken its notification. One dropped while notified, as it would be by a try that
            // succeeds, passes it on to the next operation waiting, which would be woken for
            // nothing at every ordinary wakeup.
            if let Some(listener) = &mut this.listener {
                ready!(Pin::new(listener).poll(cx));
                this.listener = None;
            }

            loop {
                let msg = this
                    .msg
                    .take()
                    .expect("a `Send` is not polled again once it completed: its message is gone");

                match this.sender.try_send(msg) {
                    Ok(()) => {
                        // Let go of a listener this future took just now, which may have been
                        // notified since: it passes the notification on.
                        this.listener = None;

                        return Poll::Ready(Ok(()));
                    }
                    Err(TrySendError::Closed(msg)) => {
                        // Let go of a listener, as on success.
                        this.listener = None;

                        return Poll::Ready(Err(SendError(msg)));
                    }
                    Err(TrySendError::Full(msg)) => this.msg = Some(msg),
                }

                // No room yet: listen, then try again, and wait on the listener if that finds none
                // either.
                match this.listener {
                    None => this.listener = Some(this.sender.channel.send_ops.listen_unfenced()),
                    Some(_) => break,
                }
            }
        }
    }
}

impl<T> fmt::Debug for Send<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The message is left out: `T` need not be `Debug`.
        f.debug_struct("Send")
            .field("sender", self.sender)
            .field("waiting", &self.listener.is_some())
            .finish_non_exhaustive()
    }
}

/// The future returned by [`Receiver::recv`].
///
/// It completes with the message, or with a [`RecvError`] if the channel is closed and has no
/// message left.
#[must_use = "futures do nothing unless .awaited"]
pub struct Recv<'a, T> {
    receiver: &'a Receiver<T>,
    /// What this future waits on while the channel has no message, taken before its last try.
    listener: Option<EventListener>,
}

impl<T> Future for Recv<'_, T> {
    type Output = Result<T, RecvError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        loop {
            // A listener this future holds is polled before anything else, as `Send::poll` does.
            if let Some(listener) = &mut this.listener {
                ready!(Pin::new(listener).poll(cx));
                this.listener = None;
            }

            loop {
                match this.receiver.try_recv() {
                    Ok(msg) => {
                        // Let go of a listener this future took just now, which may have been
                        // notified since: it passes the notification on.
                        this.listener = None;

                        return Poll::Ready(Ok(msg));
                    }
                    Err(TryRecvError::Closed) => {
                        // Let go of a listener, as on success.
                        this.listener = None;

                        return Poll::Ready(Err(RecvError));
                    }
                    Err(TryRecvError::Empty) => {}
                }

                // Nothing to receive yet: listen, then try again, and wait on the listener if that
                // finds nothing too.
                match this.listener {
                    None => {
                        this.listener = Some(this.receiver.channel.recv_ops.listen_unfenced());
                    }
                    Some(_) => break,
                }
            }
        }
    }
}

impl<T> fmt::Debug for Recv<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Recv")
            .field("receiver", self.receiver)
            .field("waiting", &self.listener.is_some())
            .finish_non_exhaustive()
    }
}

/// The error returned by a [`Sender::send`] that failed because the channel is closed.
///
/// It holds the message that was not sent.
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct SendError<T>(pub T);

impl<T> SendError<T> {
    /// The message that was not sent.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> error::Error for SendError<T> {}

impl<T> fmt::Debug for SendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SendError(..)")
    }
}

impl<T> fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the channel is closed")
    }
}

/// The error returned by a [`Sender::try_send`] that failed.
///
/// It holds the message that was not sent.
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum TrySendError<T> {
    /// The channel is full but not closed.
    Full(T),
    /// The channel is closed.
    Closed(T),
}

impl<T> TrySendError<T> {
    /// The message that was not sent.
    pub fn into_inner(self) -> T {
        match self {
            TrySendError::Full(msg) | TrySendError::Closed(msg) => msg,
        }
    }

    /// Whether the channel is full but not closed.
    pub fn is_full(&self) -> bool {
        matches!(self, TrySendError::Full(_))
    }

    /// Whether the channel is closed.
    pub fn is_closed(&self) -> bool {
        matches!(self, TrySendError::Closed(_))
    }
}

impl<T> error::Error for TrySendError<T> {}

impl<T> fmt::Debug for TrySendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrySendError::Full(_) => f.write_str("Full(..)"),
            TrySendError::Closed(_) => f.write_str("Closed(..)"),
        }
    }
}

impl<T> fmt::Display for TrySendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrySendError::Full(_) => f.write_str("the channel is full"),
            TrySendError::Closed(_) => f.write_str("the channel is closed"),
        }
    }
}

/// The error returned by a [`Receiver::recv`] that failed.
///
/// The channel is closed and has no message left.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct RecvError;

impl error::Error for RecvError {}

impl fmt::Display for RecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the channel is closed and has no message left")
    }
}

/// The error returned by a [`Receiver::try_recv`] that failed.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TryRecvError {
    /// The channel has no message but is not closed.
    Empty,
    /// The channel is closed and has no message left.
    Closed,
}

impl TryRecvError {
    /// Whether the channel has no message but is not closed.
    pub fn is_empty(&self) -> bool {
        matches!(self, TryRecvError::Empty)
    }

    /// Whether the channel is closed and has no message left.
    pub fn is_closed(&self) -> bool {
        matches!(self, TryRecvError::Closed)
    }
}

impl error::Error for TryRecvError {}

impl fmt::Display for TryRecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TryRecvError::Empty => f.write_str("the channel has no message"),
            TryRecvError::Closed => f.write_str("the channel is closed and has no message left"),
        }
    }
}

/// Makes a channel with room for `capacity` messages, or for any number if it is `None`, and its
/// first sender and receiver.
fn channel<T>(capacity: Option<NonZeroUsize>) -> (Sender<T>, Receiver<T>) {
    let channel = Arc::new(Channel {
        state: Mutex::new(State {
            // Empty and unallocated, whatever the capacity. It says how many messages the queue may
            // hold, not how much room to get for them up front, which would fail for a capacity
            // as large as `NonZeroUsize::MAX`.
            queue: VecDeque::new(),
            capacity,
            senders: 1,
            receivers: 1,
            closed: false,
        }),
        send_ops: Event::new(),
        recv_ops: Event::new(),
        stream_ops: Event::new(),
    });

    let sender = Sender {
        channel: channel.clone(),
    };
    let receiver = Receiver {
        channel,
        listener: None,
    };

    (sender, receiver)
}

/// What the ends of a channel share: its state, behind a lock, and the three events that tell the
/// operations waiting on it that the state changed.
///
/// The events sit beside the lock and not behind it, because an operation notifies them only once
/// it has let go of the lock. A notification wakes tasks, and waking one runs its waker, which is
/// somebody else's code: it may panic, and it may come straight back to the channel. Run under the
/// lock, a waker that panics would leave the operation half done, with the poison-tolerant [`lock`]
/// hiding it, and a waker that came back would deadlock. So each operation locks, changes the
/// state and decides what to notify, lets go of the lock, notifies, and only then drops any message
/// that left the channel, which is somebody else's code too.
///
/// A panic in any of that leaves the rest of it done. The state is settled before the lock is let
/// go of, so a sender or receiver dropped is counted out, and the channel closed, whatever panics
/// after. And every notification is sent, and every message and listener dropped, even where one
/// before it panics, through [`run_both`]: a waker that panics on one event does not leave an
/// operation waiting on another for good.
///
/// Listening takes no lock either. [`Event`] keeps a notification from slipping in between a
/// listener being taken and its caller checking what it waits for, as long as whoever notifies has
/// changed that before it notifies, which is the order every operation here keeps. And every
/// operation that waits listens first and checks after, as [`Event`] asks: it takes a listener,
/// tries again, and waits on the listener only if that fails too.
///
/// The channel listens and notifies without the event's fences, through `Event::listen_unfenced`,
/// `Event::notify_unfenced` and `Event::notify_additional_unfenced`. Every operation checks what it
/// waits for under this lock, and every one that changes it does so under this lock and notifies
/// after: the lock then orders a check after the change, or the listener taken before the check
/// ahead of the notification.
///
/// What a change notifies, and how:
///
/// * A message sent is one more for a receiver to take. It notifies `recv_ops` with
///   `notify_additional(1)`, which reaches one more waiting [`Recv`]. The counting `notify(1)`
///   would not do: it reaches nobody where a `Recv` was notified already and has not been polled
///   yet, and that `Recv` takes one message at most, so another one would go on waiting with a
///   message in the channel. It notifies `stream_ops` as well, in full, for the reason below.
/// * A message received from a bounded channel is room for one more message. It notifies `send_ops`
///   with `notify_additional(1)`, for the same reason: one more waiting [`Send`], and whether or
///   not an earlier one is notified already. An unbounded channel has no sender waiting for room,
///   so a receive from it notifies nobody.
/// * A channel closed is news for every operation waiting on it, which are all notified, in full,
///   on all three events. Only the call that closed it notifies: an operation that listens after
///   that finds the channel closed by its check, and does not wait.
///
/// A receiver's stream waits on `stream_ops`, and not on `recv_ops` as a [`Recv`] does, and every
/// send notifies every stream waiting there. A stream keeps its listener in the [`Receiver`], from
/// one poll to the next, and so beyond the life of any one `next()` future that polled it. One that
/// is notified and then not polled again, because its `next()` lost a `select!` or its task is busy
/// elsewhere, keeps that notification until it is. Had it waited on `recv_ops`, the notification
/// could be the one a waiting `Recv` needed for the message sent, which would then wait with the
/// message in the channel, possibly for good. With the streams on an event of their own, and all of
/// them notified by every send, a stream never holds a notification anybody else needs: each one is
/// woken for each message, the one that polls first gets it, and the others find the channel empty
/// again and wait for the next.
///
/// A [`Send`] or a [`Recv`] that completes, either way, lets go of its listener before it returns,
/// so a future kept after that holds no notification. One dropped before it completes passes on the
/// notification its listener holds, as [`Event`] does for a listener dropped while notified, which
/// is what a cancelled operation is to do. And one that holds a listener polls it before it tries
/// again, and not after: a listener that completes has taken its notification, where one dropped
/// as the try succeeds would pass it on to the next operation waiting, for a wakeup it has no use
/// for.
struct Channel<T> {
    state: Mutex<State<T>>,
    /// Send operations waiting for room, in a bounded channel.
    send_ops: Event,
    /// [`Recv`] operations waiting for a message.
    recv_ops: Event,
    /// Receivers polled as streams, waiting for a message.
    stream_ops: Event,
}

impl<T> Channel<T> {
    /// Puts `msg` in the channel if it is open and has room, and tells the receivers waiting for a
    /// message if it did. Otherwise hands `msg` back, in the error.
    fn try_send(&self, msg: T) -> Result<(), TrySendError<T>> {
        {
            let mut state = lock(&self.state);
            if state.closed {
                return Err(TrySendError::Closed(msg));
            }
            if state.is_full() {
                return Err(TrySendError::Full(msg));
            }

            state.queue.push_back(msg);
        }

        // Both, even if a waker panics on the first: a stream left unwoken would wait with the
        // message in the channel.
        run_both(
            || {
                self.recv_ops.notify_additional_unfenced(1);
            },
            || {
                self.stream_ops.notify_unfenced(usize::MAX);
            },
        );

        Ok(())
    }

    /// Takes the oldest message out of the channel, if there is one, and tells a sender waiting for
    /// room if that made some.
    fn try_recv(&self) -> Result<T, TryRecvError> {
        let (msg, bounded) = {
            let mut state = lock(&self.state);
            let Some(msg) = state.queue.pop_front() else {
                return Err(if state.closed {
                    TryRecvError::Closed
                } else {
                    TryRecvError::Empty
                });
            };

            (msg, state.capacity.is_some())
        };

        if bounded {
            self.send_ops.notify_additional_unfenced(1);
        }

        Ok(msg)
    }

    /// Closes the channel, and tells the operations waiting on it.
    ///
    /// Returns `true` if this call closed the channel, and `false` if it was closed already.
    fn close(&self) -> bool {
        let closed = lock(&self.state).close();

        if closed {
            self.notify_closed();
        }

        closed
    }

    /// Notifies every operation waiting on the channel that it is closed, on all three events
    /// even if a waker panics on one of them.
    fn notify_closed(&self) {
        run_both(
            || {
                self.send_ops.notify_unfenced(usize::MAX);
            },
            || {
                run_both(
                    || {
                        self.recv_ops.notify_unfenced(usize::MAX);
                    },
                    || {
                        self.stream_ops.notify_unfenced(usize::MAX);
                    },
                )
            },
        );
    }

    /// Writes what an end of the channel, named `end`, says of the channel for its `Debug`.
    ///
    /// The numbers are copied out under the lock, and written once it is let go of: a `Formatter`
    /// is somebody else's code, which may panic, and may come back to the channel.
    fn fmt_end(&self, end: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (len, capacity, senders, receivers, closed) = {
            let state = lock(&self.state);

            (
                state.queue.len(),
                state.capacity,
                state.senders,
                state.receivers,
                state.closed,
            )
        };

        f.debug_struct(end)
            .field("len", &len)
            .field("capacity", &capacity)
            .field("senders", &senders)
            .field("receivers", &receivers)
            .field("closed", &closed)
            .finish()
    }
}

/// What a channel's lock keeps.
struct State<T> {
    /// The messages in the channel, the oldest first.
    queue: VecDeque<T>,
    /// How many messages the channel holds at most, or `None` for any number.
    capacity: Option<NonZeroUsize>,
    /// How many [`Sender`]s there are.
    senders: usize,
    /// How many [`Receiver`]s there are.
    receivers: usize,
    /// Whether the channel is closed.
    closed: bool,
}

impl<T> State<T> {
    /// Whether the channel holds as many messages as it may.
    fn is_full(&self) -> bool {
        self.capacity
            .is_some_and(|capacity| self.queue.len() >= capacity.get())
    }

    /// Closes the channel, and tells whether this call did, and not an earlier one.
    ///
    /// The caller that closed it is to notify every waiting operation once it has let go of the
    /// lock.
    fn close(&mut self) -> bool {
        !mem::replace(&mut self.closed, true)
    }
}

/// The value behind a lock, taken whether or not a panic poisoned it.
///
/// No user code runs under the channel's lock. A message is moved, never cloned, so there is no
/// `T::clone` to run. A message that leaves the channel is dropped once the lock is let go of, and
/// the wakers of the operations waiting are woken, cloned and dropped by [`Event`], clear of it.
/// Not even a `Formatter` is written to under it. What does run is this module's own code and the
/// `VecDeque`'s, which can panic only on a capacity that overflows, before it has changed anything.
/// So no panic can have left the state half-changed, and a lock that one poisoned is as good as one
/// that none did.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Runs `first`, and then `second`, even where `first` panics.
///
/// For what the channel runs once it has let go of its lock: the notifications of its events and
/// the drops of its messages and listeners. Each runs somebody else's code, a waker or a message's
/// `Drop`, and one that panics is not to leave the rest undone, an operation waiting for a
/// notification that never comes or a message never dropped. Three steps nest as
/// `run_both(a, || run_both(b, c))`.
///
/// Where `first` panics, `second` runs as that panic unwinds, under `catch_unwind`: a panic of its
/// own is caught there, rather than unwinding out of a drop already unwinding, which would abort
/// the process, and the first panic goes on to the caller. Where `first` does not panic, `second`
/// runs as any call does, and so does a panic of its own. That path costs nothing beyond the two
/// calls, which inline: a send makes it on every message.
fn run_both<A, B>(first: A, second: B)
where
    A: FnOnce(),
    B: FnOnce(),
{
    let mut on_unwind = OnUnwind(Some(second));
    first();
    if let Some(second) = on_unwind.0.take() {
        second();
    }
}

/// A step that runs if a panic unwinds past it, unless it was taken out to run before.
///
/// A panic of the step's own is caught, and its payload disposed of: the first panic is the one
/// that goes on to the caller.
struct OnUnwind<F>(Option<F>)
where
    F: FnOnce();

impl<F> Drop for OnUnwind<F>
where
    F: FnOnce(),
{
    fn drop(&mut self) {
        // Still here only as a panic unwinds: [`run_both`] takes it out to run otherwise.
        if let Some(step) = self.0.take()
            && let Err(panic) = panic::catch_unwind(AssertUnwindSafe(step))
        {
            dispose(panic);
        }
    }
}

/// Drops the payload of a panic that is to go no further, with a panic of its destructor caught.
///
/// The payload is somebody else's value, and its `Drop` may panic in turn: out of the drop of an
/// [`OnUnwind`], as the first panic unwinds, that would abort the process. The payload of such a
/// second panic is dropped too, the same way, and only one that panics a third time is leaked, so
/// as not to follow a chain of destructors that each panic. The scheduler has a function like this
/// one too, but this module does not use it: the runtime may be left out of the build.
fn dispose(payload: Box<dyn Any + std::marker::Send>) {
    let Err(payload) = panic::catch_unwind(AssertUnwindSafe(move || drop(payload))) else {
        return;
    };
    if let Err(payload) = panic::catch_unwind(AssertUnwindSafe(move || drop(payload))) {
        mem::forget(payload);
    }
}
