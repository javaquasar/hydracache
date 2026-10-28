#![cfg_attr(feature = "runtime", doc = include_str!("../README.md"))]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/z-galaxy/zruntime/f7ba9e0ca073c3d2981bbb443a74ad62a863255e/data/logo.png"
)]
// The README's examples need the runtime, so a build without it gets an overview of its own.
#![cfg_attr(
    all(feature = "event", not(feature = "runtime")),
    doc = include_str!("event-only.md")
)]
#![deny(rust_2018_idioms)]
#![doc(test(attr(
    warn(unused),
    deny(warnings),
    allow(dead_code),
    // W/o this, we seem to get some bogus warning about `extern crate zbus`.
    allow(unused_extern_crates),
)))]

#[cfg(feature = "runtime")]
mod async_io;
#[cfg(feature = "broadcast")]
pub mod broadcast;
#[cfg(any(feature = "runtime", feature = "fs"))]
mod coop;
#[cfg(feature = "helper")]
mod driver;
#[cfg(feature = "event")]
mod event;
#[cfg(feature = "fs")]
pub mod fs;
#[cfg(feature = "lock")]
pub mod lock;
#[cfg(feature = "runtime")]
mod log;
#[cfg(feature = "runtime")]
mod mode;
#[cfg(feature = "mpmc")]
pub mod mpmc;
#[cfg(any(feature = "tcp", feature = "udp", all(feature = "unix", unix)))]
pub mod net;
#[cfg(feature = "runtime")]
mod poll;
#[cfg(all(feature = "process", not(target_os = "wasi")))]
pub mod process;
#[cfg(feature = "runtime")]
mod reactor;
#[cfg(feature = "runtime")]
mod runtime;
#[cfg(feature = "runtime")]
mod scheduler;
#[cfg(feature = "runtime")]
mod time;
#[cfg(all(feature = "unblock", not(target_os = "wasi")))]
mod unblock;

#[cfg(all(feature = "runtime", any(unix, target_os = "wasi")))]
use std::os::fd::AsFd as AsSource;
#[cfg(all(feature = "runtime", windows))]
use std::os::windows::io::AsSocket as AsSource;
#[cfg(feature = "runtime")]
use std::{
    borrow::Cow,
    fmt,
    future::{Future, IntoFuture},
    io,
    panic::Location,
    pin::Pin,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll},
    time::{Duration, Instant},
};

#[cfg(feature = "runtime")]
pub use async_io::AsyncIo;
#[cfg(feature = "runtime")]
pub use coop::unconstrained;
#[cfg(feature = "event")]
pub use event::{Event, EventListener};
#[cfg(feature = "runtime")]
use mode::sealed::Sealed as _;
#[cfg(feature = "runtime")]
pub use mode::{Local, Mode, Shared, Source};
#[cfg(feature = "runtime")]
pub use reactor::{Readiness, Registration};
#[cfg(feature = "runtime")]
use runtime::Core;
#[cfg(feature = "runtime")]
use scheduler::{JoinHandle, Name};
#[cfg(feature = "runtime")]
pub use time::{Interval, MissedTickBehavior, Sleep, TimedOut, Timeout};
#[cfg(all(feature = "unblock", not(target_os = "wasi")))]
pub use unblock::{BlockingWork, Unblock, unblock};

/// Runs `future` to completion on the calling thread, together with the thread's own runtime.
///
/// Each thread has its own [`SharedRuntime`] for this function, which [`SharedRuntime::current`]
/// returns from inside `future`. The call blocks the thread until `future` completes. Meanwhile,
/// the thread also runs the tasks, timers and I/O of its runtime. Threads that call this each run
/// their own runtime, in parallel.
///
/// Work that is still alive when the call returns, such as a detached task, runs on a helper thread
/// until the next call, or until it is done. A call that starts while the helper is running the
/// runtime takes the runtime over from it. So a program that calls this once per operation still
/// runs each operation on the thread that called it.
///
/// One thread at a time runs a runtime. If another thread is running it already, the call waits for
/// its turn, and polls `future` whenever it is woken in the meantime.
///
/// `future` must not block the thread to wait for work on the runtime, for example by waiting
/// synchronously for a task's result. That work runs on this same thread, between polls of
/// `future`, so such a wait never ends.
///
/// Do not call this from a task of another executor either. It blocks that executor's thread until
/// `future` completes, which deadlocks if `future` needs that thread.
///
/// On WASI, there is no helper thread, as no thread can be started: see
/// [`SharedRuntime::current`] for what that means for the work left when the call returns.
///
/// # Panics
///
/// Panics if the calling thread is already running a runtime, for example when called from a task
/// or from the future passed to another `block_on`. Such a call would wait for its own thread.
///
/// Requires the `helper` feature.
#[cfg(feature = "helper")]
pub fn block_on<F>(future: F) -> F::Output
where
    F: Future,
{
    driver::block_on(driver::Target::Own, &driver::own, future)
}

/// Spawns `future` on the shared runtime that runs the calling code.
///
/// This is `SharedRuntime::spawn` for code that has no runtime handle, such as a function called
/// from a task. The task is named after the place it was spawned from, for its `Debug` output and
/// for the log message if it panics. As with any task, dropping the returned [`Task`] cancels it,
/// and [`Task::detach`] lets it run on.
///
/// The task goes on the first of these runtimes that exists:
///
/// 1. The shared runtime that the calling thread is running: the runtime it called
///    [`Runtime::block_on`] on, its own runtime inside the free `block_on`, or the runtime whose
///    task it is running, on a thread that called `block_on` or on a helper thread.
/// 2. With the `helper` feature, the runtime that `SharedRuntime::current` returns.
///
/// So the task goes on the runtime that runs the code spawning it. For a runtime created by
/// [`Runtime::new`], that is not the runtime `SharedRuntime::current` would return.
///
/// On WASI, there is no helper thread, so a task spawned outside any `block_on` runs only once a
/// `block_on` on its runtime does, and is dropped with that runtime if nothing holds it until then.
/// The documentation of `SharedRuntime::current` has the details.
///
/// For a future that is not `Send`, use [`spawn_local`] on a local runtime.
///
/// # Panics
///
/// Without the `helper` feature, panics if the calling thread is not running a shared runtime. With
/// it, panics only if `SharedRuntime::current` fails, because the OS cannot provide the resources a
/// new runtime needs.
///
/// # Example
///
/// ```
/// use zruntime::{Shared, SharedRuntime, Task};
///
/// // A function that is given no runtime to spawn on, and spawns on whichever one is running it.
/// fn double(number: u32) -> Task<u32, Shared> {
///     zruntime::spawn(async move { number * 2 })
/// }
///
/// let runtime = SharedRuntime::new().unwrap();
/// let doubled = runtime.block_on(async { double(21).await.unwrap() });
///
/// assert_eq!(doubled, 42);
/// ```
#[cfg(feature = "runtime")]
#[track_caller]
pub fn spawn<F>(future: F) -> Task<F::Output, Shared>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    // Taken here rather than in a helper, which `track_caller` would have to be on all the way
    // down for the location to be the caller's.
    let name = Name::SpawnedAt(Location::caller());

    spawn_on_shared(&shared_to_spawn_on(), name, future)
}

/// Spawns `future` on the local runtime that runs the calling code.
///
/// This is `LocalRuntime::spawn` for code that has no runtime handle, as [`spawn`] is for a shared
/// runtime. The task is named after the place it was spawned from. It goes on the local runtime
/// that the calling thread is running: the one it called [`Runtime::block_on`] on, or the one whose
/// task it is running.
///
/// # Panics
///
/// Panics if the calling thread is not running a local runtime. That includes calls from the future
/// passed to `block_on` on a shared runtime, and from tasks of a shared runtime.
///
/// # Example
///
/// ```
/// use std::{cell::Cell, rc::Rc};
///
/// use zruntime::LocalRuntime;
///
/// let runtime = LocalRuntime::new().unwrap();
/// let total = Rc::new(Cell::new(0));
///
/// runtime.block_on(async {
///     // A future that holds an `Rc` is no problem here, and nothing names the runtime.
///     let adding = zruntime::spawn_local({
///         let total = total.clone();
///         async move { total.set(total.get() + 42) }
///     });
///
///     adding.await.unwrap();
/// });
///
/// assert_eq!(total.get(), 42);
/// ```
#[cfg(feature = "runtime")]
#[track_caller]
pub fn spawn_local<F>(future: F) -> Task<F::Output, Local>
where
    F: Future + 'static,
    F::Output: 'static,
{
    let name = Name::SpawnedAt(Location::caller());
    let Some(core) = Local::driven() else {
        panic!(
            "spawn_local called where no local runtime is running: call it from inside \
             `LocalRuntime::block_on`, or from a task of that runtime"
        );
    };

    Task(scheduler::spawn_local(&core, name, future))
}

/// A runtime that stays on the thread that created it, and runs futures that need not be `Send`.
#[cfg(feature = "runtime")]
pub type LocalRuntime = Runtime<Local>;

/// A runtime that can be used from, and run on, any thread, and runs `Send` futures.
#[cfg(feature = "runtime")]
pub type SharedRuntime = Runtime<Shared>;

/// A handle to a runtime: a scheduler and a reactor, which run on the thread that calls
/// [`Runtime::block_on`] on it.
///
/// Cloning the handle is cheap, and all clones refer to the same runtime. The runtime is dropped
/// once its last handle, timer and registration are dropped. Any unfinished task is dropped with
/// it, and awaiting its [`Task`] then fails. A [`Task`] does not keep its runtime alive, but a task
/// whose future holds a handle, a timer or a registration does, for as long as the task lives.
///
/// `M` is the flavour of the runtime: [`Local`], the default, for a runtime that stays on its
/// thread, or [`Shared`] for one that can be used from any thread.
///
/// Prefer [`Local`] unless the runtime, a task or a future built on it must cross threads, or be
/// polled by another executor. It is the cheaper flavour: it uses `Rc` and `RefCell` where
/// [`Shared`] uses `Arc` and `Mutex`. [`Shared`] also only runs `Send` futures.
///
/// Either way, one thread at a time runs a runtime, so all its tasks share one CPU core. To use
/// more cores, use several runtimes: see
/// [Running on several threads](crate#running-on-several-threads).
#[cfg(feature = "runtime")]
pub struct Runtime<M = Local>
where
    M: Mode,
{
    core: M::Ptr<Core<M>>,
}

#[cfg(feature = "runtime")]
impl<M> Runtime<M>
where
    M: Mode,
{
    /// Creates a runtime.
    ///
    /// The runtime has no thread of its own. Its tasks, timers and I/O make progress only while a
    /// thread is running [`Runtime::block_on`] on it.
    ///
    /// A task whose future holds a handle, a timer or a registration of the runtime keeps the
    /// runtime, and the OS resources of its reactor, alive until the task ends. No helper thread
    /// ever runs a runtime created here, so a detached task that never ends is never dropped. Drive
    /// such a task to completion, or keep its [`Task`] to cancel it, by dropping it or with
    /// [`Task::cancel`].
    ///
    /// A plain `Runtime::new()` leaves the compiler unable to infer `M`. Call [`LocalRuntime::new`]
    /// or [`SharedRuntime::new`] instead, or name the flavour with a turbofish.
    ///
    /// # Errors
    ///
    /// Fails if the OS cannot provide the resources the reactor needs.
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            core: Core::<M>::new()?,
        })
    }

    /// Runs `future` to completion on the calling thread, and runs this runtime alongside it.
    ///
    /// The call blocks the thread until `future` completes. Meanwhile, the thread also runs the
    /// runtime's tasks, timers and I/O. Everything spawned, registered or timed on the runtime runs
    /// on this thread for as long as the call lasts.
    ///
    /// `future` must not block the thread to wait for that work, for example by waiting
    /// synchronously for a task's result, or by spinning until a task sets a flag. That work runs
    /// on this same thread, between polls of `future`, so such a wait never ends.
    ///
    /// # Panics
    ///
    /// Panics if the calling thread is already running a runtime, for example when called from a
    /// task or from the future passed to another `block_on`. Such a call would wait for its own
    /// thread.
    ///
    /// A shared runtime created by [`Runtime::new`] is run by one thread at a time, so the call
    /// also panics if another thread is running `block_on` on it. A runtime from
    /// `SharedRuntime::current` does not panic in that case: the call waits for its turn instead,
    /// and takes the runtime over from the helper thread, as the free `block_on` does.
    ///
    /// With the `helper` feature, a call on a runtime created by [`Runtime::new`] also panics when
    /// made from the future passed to the free `block_on`, or to `block_on` on a runtime from
    /// `SharedRuntime::current`.
    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: Future,
    {
        M::block_on(&self.core, future)
    }

    /// A future that completes once `duration` has passed. Dropping it cancels the timer.
    ///
    /// A `duration` too long for the clock to represent, such as `Duration::MAX`, gives a sleep
    /// that never completes.
    pub fn sleep(&self, duration: Duration) -> Sleep<M> {
        // This runtime's timers run on the standard clock, so a length of time is a deadline on
        // it — where the clock has a moment that far ahead. `Duration::MAX`, which a wait of
        // "however long it takes" comes to, has none, and asks for a timer that never fires
        // rather than for a moment the clock cannot name.
        let deadline = Instant::now().checked_add(duration);

        Sleep(reactor::sleep::<M>(&self.core, deadline))
    }

    /// A future that completes once `deadline` has passed, or on its first poll if it already has.
    /// Dropping it cancels the timer.
    ///
    /// Use this to keep a loop to a schedule. A loop that sleeps for a period in each round drifts
    /// by the time the work of each round takes, and the error adds up. A loop that adds the period
    /// to a deadline and sleeps until that deadline starts each round on time.
    ///
    /// # Example
    ///
    /// ```
    /// use std::time::{Duration, Instant};
    ///
    /// use zruntime::LocalRuntime;
    ///
    /// let runtime = LocalRuntime::new().unwrap();
    /// let period = Duration::from_millis(2);
    /// let started = Instant::now();
    ///
    /// runtime.block_on(async {
    ///     let mut deadline = started;
    ///     for _ in 0..3 {
    ///         deadline += period;
    ///         runtime.sleep_until(deadline).await;
    ///     }
    /// });
    ///
    /// assert!(started.elapsed() >= 3 * period);
    /// ```
    pub fn sleep_until(&self, deadline: Instant) -> Sleep<M> {
        Sleep(reactor::sleep::<M>(&self.core, Some(deadline)))
    }

    /// Puts a time limit of `duration` on `future`.
    ///
    /// The returned future resolves to `Ok` with the output of `future` if it completes in time,
    /// and to `Err` with [`TimedOut`] otherwise. Dropping it drops `future` and cancels the timer.
    ///
    /// The clock starts at this call, not at the first poll. A `duration` too long for the clock to
    /// represent, such as `Duration::MAX`, never times out. Each poll polls `future` before it
    /// checks the clock, so a future that completes on the poll in which its time runs out still
    /// returns its output. A future that times out is not dropped: [`Timeout::into_inner`] returns
    /// it, to retry it or keep driving it.
    ///
    /// # Example
    ///
    /// ```
    /// use std::{future, io, time::Duration};
    ///
    /// use zruntime::LocalRuntime;
    ///
    /// let runtime = LocalRuntime::new().unwrap();
    ///
    /// runtime.block_on(async {
    ///     let in_time = runtime.timeout(Duration::from_secs(60), async { 7 }).await;
    ///     assert_eq!(in_time, Ok(7));
    ///
    ///     let late = runtime.timeout(Duration::from_millis(2), future::pending::<()>());
    ///     assert!(late.await.is_err());
    /// });
    ///
    /// // A time-out is an I/O error of its own kind, for code that returns `io::Result`.
    /// fn wait(runtime: &LocalRuntime) -> io::Result<()> {
    ///     runtime.block_on(runtime.timeout(Duration::from_millis(2), future::pending::<()>()))?;
    ///
    ///     Ok(())
    /// }
    /// assert_eq!(wait(&runtime).unwrap_err().kind(), io::ErrorKind::TimedOut);
    /// ```
    pub fn timeout<F>(&self, duration: Duration, future: F) -> Timeout<F::IntoFuture, M>
    where
        F: IntoFuture,
    {
        Timeout::new(future.into_future(), self.sleep(duration))
    }

    /// Puts a deadline on `future`, as [`timeout`](Self::timeout) puts a time limit on it.
    ///
    /// If `deadline` has already passed, a future that is not ready on its first poll times out on
    /// that poll. Dropping the returned future drops `future` and cancels the timer.
    ///
    /// Use this to keep several steps of one piece of work to one deadline, rather than give each
    /// step a duration of its own.
    ///
    /// # Example
    ///
    /// ```
    /// use std::time::{Duration, Instant};
    ///
    /// use zruntime::LocalRuntime;
    ///
    /// let runtime = LocalRuntime::new().unwrap();
    /// let deadline = Instant::now() + Duration::from_millis(5);
    ///
    /// runtime.block_on(async {
    ///     // Steps of 2 ms each, for as long as each finishes before the deadline.
    ///     while runtime
    ///         .timeout_at(deadline, runtime.sleep(Duration::from_millis(2)))
    ///         .await
    ///         .is_ok()
    ///     {}
    /// });
    ///
    /// // However many steps fit, the loop ended once the deadline had passed.
    /// assert!(Instant::now() >= deadline);
    /// ```
    pub fn timeout_at<F>(&self, deadline: Instant, future: F) -> Timeout<F::IntoFuture, M>
    where
        F: IntoFuture,
    {
        Timeout::new(future.into_future(), self.sleep_until(deadline))
    }

    /// A timer that ticks once every `period`, starting one period from now.
    ///
    /// Each tick returns the moment it was scheduled for. The ticks keep to the schedule however
    /// long the work done at each tick takes. If ticks are missed, because the task was busy or its
    /// thread blocked for a period or more, the interval's [`MissedTickBehavior`] decides how to
    /// catch up. By default, the missed ticks fire at once.
    ///
    /// A `period` too long for the clock to represent gives an interval that never ticks.
    ///
    /// # Panics
    ///
    /// Panics if `period` is zero.
    ///
    /// # Example
    ///
    /// ```
    /// use std::time::{Duration, Instant};
    ///
    /// use zruntime::LocalRuntime;
    ///
    /// let runtime = LocalRuntime::new().unwrap();
    /// let period = Duration::from_millis(2);
    /// let started = Instant::now();
    ///
    /// let ticks = runtime.block_on(async {
    ///     let mut interval = runtime.interval(period);
    ///     [interval.tick().await, interval.tick().await, interval.tick().await]
    /// });
    ///
    /// // Each tick is the moment it was scheduled for, a period after the one before.
    /// assert!(ticks[0] >= started + period);
    /// assert_eq!(ticks[1] - ticks[0], period);
    /// assert_eq!(ticks[2] - ticks[1], period);
    /// ```
    pub fn interval(&self, period: Duration) -> Interval<M> {
        Interval::new(self.sleep(period), period)
    }

    /// A timer that ticks once every `period`, as [`interval`](Self::interval) does, starting at
    /// `start`.
    ///
    /// If `start` has already passed, the first tick comes on the first poll, as it does for
    /// `interval_at(Instant::now(), period)`.
    ///
    /// # Panics
    ///
    /// Panics if `period` is zero.
    pub fn interval_at(&self, start: Instant, period: Duration) -> Interval<M> {
        Interval::new(self.sleep_until(start), period)
    }
}

#[cfg(feature = "runtime")]
impl Runtime<Local> {
    /// Spawns `future` as a task named `name`, and returns its [`Task`].
    ///
    /// The task runs concurrently with the caller, on the thread that runs [`Runtime::block_on`] on
    /// this runtime. Awaiting the [`Task`] returns the output of `future`, or `Err` if the runtime
    /// lost the task, for example because it panicked. Dropping the [`Task`] cancels the task, and
    /// [`Task::detach`] lets it run on.
    ///
    /// `name` says what the task is for, such as `"socket reader"`. It is only used for
    /// diagnostics, for example in the log message if the task panics. Code that has no runtime
    /// handle can spawn with [`spawn_local`] instead, which takes no name.
    pub fn spawn<T>(
        &self,
        name: impl Into<Cow<'static, str>>,
        future: impl Future<Output = T> + 'static,
    ) -> Task<T, Local>
    where
        T: 'static,
    {
        Task(scheduler::spawn_local(
            &self.core,
            Name::Given(name.into()),
            future,
        ))
    }

    /// Starts watching `source` for readiness.
    ///
    /// `source` must already be in non-blocking mode. The runtime only waits for readiness: a read
    /// or write on a blocking source blocks the thread, and every other task with it. The runtime
    /// gets the descriptor of `source` once, here, and watches that descriptor from then on. So
    /// `source` must keep the same descriptor open for as long as it lives, as every std type does.
    ///
    /// Dropping the returned [`Registration`] stops the watch. Do each read or write through
    /// [`Registration::poll_io`], which turns a `WouldBlock` into a wait for readiness instead of a
    /// busy loop. The registration keeps `source`, so do the I/O through another handle to the same
    /// socket: an `Rc` of it, for example, or a clone of its descriptor.
    ///
    /// [`AsyncIo`] does all of this for you: it registers a source, keeps it and does the I/O.
    /// [`Registration::ready`] only waits for readiness, for code that does its I/O some other way.
    ///
    /// # Errors
    ///
    /// Fails if the OS cannot watch `source`. A runtime also watches each descriptor through only
    /// one registration at a time. While another registration watches the descriptor of `source`,
    /// this fails with [`AlreadyExists`](io::ErrorKind::AlreadyExists). A cloned descriptor, such
    /// as `try_clone` returns, is a different descriptor.
    ///
    /// # Example
    ///
    /// ```
    /// use std::{
    ///     future::poll_fn,
    ///     io::{Read, Write},
    ///     net::{TcpListener, TcpStream},
    ///     rc::Rc,
    /// };
    ///
    /// use zruntime::{Interest, LocalRuntime};
    ///
    /// let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    /// let mut peer = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    /// let stream = Rc::new(listener.accept().unwrap().0);
    /// stream.set_nonblocking(true).unwrap();
    ///
    /// let runtime = LocalRuntime::new().unwrap();
    /// let received = runtime.block_on(async {
    ///     let registration = runtime.register(stream.clone()).unwrap();
    ///     peer.write_all(b"!").unwrap();
    ///
    ///     let mut byte = [0];
    ///     poll_fn(|cx| {
    ///         registration.poll_io(cx, Interest::Readable, || (&*stream).read(&mut byte))
    ///     })
    ///     .await
    ///     .unwrap();
    ///
    ///     byte
    /// });
    ///
    /// assert_eq!(&received, b"!");
    /// ```
    pub fn register<S>(&self, source: S) -> io::Result<Registration<Local>>
    where
        S: AsSource + 'static,
    {
        reactor::register::<Local>(&self.core, Rc::new(source))
    }
}

#[cfg(feature = "runtime")]
impl Runtime<Shared> {
    /// The runtime for the calling code, created if none is alive.
    ///
    /// Which runtime that is depends on where it is called from:
    ///
    /// 1. From the future passed to the free [`block_on`], the calling thread's own runtime.
    /// 2. From the future passed to `block_on` on a runtime from this function, or from a task of
    ///    such a runtime, that runtime.
    /// 3. From anywhere else, a runtime that the whole process shares. A helper thread runs it.
    ///    This is the runtime for work that another executor polls, which has no thread of its own
    ///    to run it.
    ///
    /// `block_on` on a [`LocalRuntime`], or on a runtime created by [`Runtime::new`], does not
    /// count here: it only runs its own runtime. Called from its future or its tasks, this returns
    /// the runtime the process shares, and a helper thread runs what is built on it.
    ///
    /// A runtime from this function is dropped once its last handle, and the last work built on it,
    /// are gone. The next call then creates a new one. Its work runs on the thread that is running
    /// `block_on` on it, if there is one. Otherwise a helper thread runs it. The helper starts when
    /// there is work and no thread runs the runtime, and stops once nothing is left to run, watch
    /// or time.
    ///
    /// # On WASI
    ///
    /// On WASI, no helper thread is ever started. `wasm32-wasip1` and `wasm32-wasip2` cannot start
    /// a thread, and a WASI target with threads, such as `wasm32-wasip1-threads`, cannot be told
    /// apart from them, so it is treated the same way. A runtime from this function then works as
    /// one from [`Runtime::new`] does: its tasks, timers and I/O make progress only while a thread
    /// is running `block_on` on that very runtime, and wait for the next such call otherwise. That
    /// includes the work left on it when a call of the free [`block_on`] returns, and the work
    /// built outside any `block_on`, which goes on the runtime the process shares rather than
    /// on the calling thread's own. Awaiting such work from a `block_on` on another runtime
    /// waits for good.
    ///
    /// Nor does a helper keep such a runtime alive: only its handles do, along with the work that
    /// holds one. Once the last of them is dropped, the runtime is dropped, with whatever work is
    /// left on it. To have work that outlives a `block_on` call carry on in a later one, keep a
    /// handle from this function, and call `block_on` on it:
    ///
    /// ```
    /// use zruntime::SharedRuntime;
    ///
    /// // Outside any `block_on`, this is the runtime the process shares, and the one the free
    /// // `spawn` puts the task on. The handle keeps it, and the task, alive.
    /// let runtime = SharedRuntime::current().unwrap();
    /// let task = zruntime::spawn(async { 7 });
    ///
    /// assert_eq!(runtime.block_on(task).unwrap(), 7);
    /// ```
    ///
    /// # Errors
    ///
    /// Fails if the OS cannot provide the resources the reactor of a new runtime needs.
    ///
    /// # Example
    ///
    /// ```
    /// use std::{
    ///     future::poll_fn,
    ///     io::{Read, Write},
    ///     net::{TcpListener, TcpStream},
    ///     sync::Arc,
    /// };
    ///
    /// use zruntime::{Interest, SharedRuntime};
    ///
    /// let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    /// let mut peer = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    /// let stream = Arc::new(listener.accept().unwrap().0);
    /// stream.set_nonblocking(true).unwrap();
    ///
    /// let received = zruntime::block_on(async {
    ///     let runtime = SharedRuntime::current().expect("a runtime for this thread");
    ///     let registration = runtime.register(stream.clone()).unwrap();
    ///     peer.write_all(b"!").unwrap();
    ///
    ///     let mut byte = [0];
    ///     poll_fn(|cx| {
    ///         registration.poll_io(cx, Interest::Readable, || (&*stream).read(&mut byte))
    ///     })
    ///     .await
    ///     .unwrap();
    ///
    ///     byte
    /// });
    ///
    /// assert_eq!(&received, b"!");
    /// ```
    ///
    /// Requires the `helper` feature.
    #[cfg(feature = "helper")]
    pub fn current() -> io::Result<Self> {
        Ok(Self {
            core: driver::current()?,
        })
    }

    /// Spawns `future` as a task named `name`, and returns its [`Task`].
    ///
    /// The task runs concurrently with the caller, on the thread that runs [`Runtime::block_on`] on
    /// this runtime. On a runtime from `SharedRuntime::current`, a helper thread runs it when no
    /// such thread does, except on WASI, where it waits for one (see `SharedRuntime::current`).
    /// Awaiting the [`Task`] returns the output of `future`, or `Err` if the runtime lost the task,
    /// for example because it panicked. Dropping the [`Task`] cancels the task, and
    /// [`Task::detach`] lets it run on.
    ///
    /// `name` says what the task is for, such as `"socket reader"`. It is only used for
    /// diagnostics, for example in the log message if the task panics. Code that has no runtime
    /// handle can spawn with [`spawn`] instead, which takes no name.
    pub fn spawn<T>(
        &self,
        name: impl Into<Cow<'static, str>>,
        future: impl Future<Output = T> + Send + 'static,
    ) -> Task<T, Shared>
    where
        T: Send + 'static,
    {
        spawn_on_shared(&self.core, Name::Given(name.into()), future)
    }

    /// Starts watching `source` for readiness.
    ///
    /// `source` must already be in non-blocking mode. The runtime only waits for readiness: a read
    /// or write on a blocking source blocks the thread, and every other task with it. The runtime
    /// gets the descriptor of `source` once, here, and watches that descriptor from then on. So
    /// `source` must keep the same descriptor open for as long as it lives, as every std type does.
    ///
    /// Dropping the returned [`Registration`] stops the watch. Do each read or write through
    /// [`Registration::poll_io`], which turns a `WouldBlock` into a wait for readiness instead of a
    /// busy loop. The registration keeps `source`, so do the I/O through another handle to the same
    /// socket: an `Arc` of it, for example, or a clone of its descriptor.
    ///
    /// [`AsyncIo`] does all of this for you: it registers a source, keeps it and does the I/O.
    /// [`Registration::ready`] only waits for readiness, for code that does its I/O some other way.
    ///
    /// # Errors
    ///
    /// Fails if the OS cannot watch `source`. A runtime also watches each descriptor through only
    /// one registration at a time. While another registration watches the descriptor of `source`,
    /// this fails with [`AlreadyExists`](io::ErrorKind::AlreadyExists). A cloned descriptor, such
    /// as `try_clone` returns, is a different descriptor.
    pub fn register<S>(&self, source: S) -> io::Result<Registration<Shared>>
    where
        S: AsSource + Send + Sync + 'static,
    {
        let registered = reactor::register::<Shared>(&self.core, Arc::new(source))?;
        // Asked for once the source is in the reactor's map, so that a helper starting here
        // takes it into its very first wait.
        self.core.ensure_progress();

        Ok(registered)
    }

    /// A handle on `core`, whatever registry it is or is not in.
    #[cfg(all(test, feature = "helper"))]
    pub(crate) fn from_inner(core: Arc<Core<Shared>>) -> Self {
        Self { core }
    }

    /// What this handle is on.
    #[cfg(all(test, feature = "helper"))]
    pub(crate) fn inner(&self) -> &Arc<Core<Shared>> {
        &self.core
    }

    /// Whether the helper thread is running.
    #[cfg(all(test, feature = "helper"))]
    pub(crate) fn helper_running(&self) -> bool {
        driver::seat(&self.core).helper_running()
    }

    /// Whether the helper thread is parked for want of the seat.
    ///
    /// Only tests that wait for an `Event` ask this, so it is built with the `event` feature.
    #[cfg(all(test, feature = "helper", feature = "event"))]
    pub(crate) fn helper_parked(&self) -> bool {
        driver::seat(&self.core).helper_parked()
    }
}

#[cfg(feature = "runtime")]
impl<M> Clone for Runtime<M>
where
    M: Mode,
{
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
        }
    }
}

#[cfg(feature = "runtime")]
impl<M> fmt::Debug for Runtime<M>
where
    M: Mode,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Runtime").finish_non_exhaustive()
    }
}

/// The readiness an I/O operation waits for.
#[cfg(feature = "runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Interest {
    /// The source has bytes to be read, or has reached its end.
    Readable,
    /// The source has room for bytes to be written, or a connect under way has settled.
    Writable,
}

/// A handle to a task spawned on a [`Runtime`]. Dropping it cancels the task.
///
/// Awaiting it returns the output of the task, or an error if the task panicked or was dropped with
/// its runtime. [`Task::cancel`] cancels the task and waits for it to stop.
#[cfg(feature = "runtime")]
pub struct Task<T, M = Local>(JoinHandle<T, M>)
where
    M: Mode;

#[cfg(feature = "runtime")]
impl<T, M> Task<T, M>
where
    M: Mode,
{
    /// Lets the task run to completion without a handle.
    ///
    /// The runtime keeps a detached task until it ends. The task runs whenever a thread is running
    /// [`Runtime::block_on`] on its runtime. On a runtime from `SharedRuntime::current`, a helper
    /// thread runs it when no such thread does, except on WASI (see `SharedRuntime::current`). A
    /// task that never ends is kept, with everything its future holds, for as long as the runtime
    /// lives. If a helper thread runs it, it keeps that helper for the life of the process.
    pub fn detach(self) {
        self.0.detach();
    }

    /// Cancels the task, and returns a future that resolves once the task's future is dropped.
    ///
    /// Dropping a [`Task`] cancels it too, but does not wait. If another thread is polling the task
    /// at that moment, its future, and everything the future holds, is only dropped once that poll
    /// returns. Use this method when you need to know when that is, for example to bind to the
    /// address of a socket the task held, or to take a lock it held. Use it also to get the output
    /// of a task that may have finished already.
    ///
    /// This call cancels the task, whether or not the returned future is ever polled:
    ///
    /// * If the task is waiting, or has not been polled yet, its future is dropped right here, on
    ///   the calling thread.
    /// * If the task is being polled, its future is dropped once that poll returns. That happens on
    ///   another thread, or on this one if the task cancels itself.
    /// * If the runtime is being dropped at the same time, perhaps on another thread, the runtime
    ///   drops the future, possibly after this call has returned.
    ///
    /// The returned future resolves once the task's future has been dropped, in whichever of these
    /// ways. It resolves to the task's output if the task finished before it was cancelled, and to
    /// `None` otherwise. It is also `None` if the task panicked, was dropped with its runtime, or
    /// already returned its output to an `.await`. Dropping the returned future is the same as
    /// dropping the [`Task`].
    ///
    /// # Panics
    ///
    /// If the destructor of the task's future panics as it is dropped here, the panic propagates
    /// out of this call, as it would out of dropping the [`Task`].
    ///
    /// # Example
    ///
    /// ```
    /// use std::{future, rc::Rc, time::Duration};
    ///
    /// use zruntime::LocalRuntime;
    ///
    /// let runtime = LocalRuntime::new().unwrap();
    /// let connection = Rc::new("a connection");
    ///
    /// runtime.block_on(async {
    ///     let answer = runtime.spawn("an answer", async { 42 });
    ///     let held = connection.clone();
    ///     let reader = runtime.spawn("a reader that never finishes", async move {
    ///         let _connection = held;
    ///         future::pending::<u32>().await
    ///     });
    ///     // Each wait gives the tasks a turn, until the first one has finished.
    ///     while !answer.is_finished() {
    ///         runtime.sleep(Duration::from_millis(1)).await;
    ///     }
    ///
    ///     // A task that finished hands its output back...
    ///     assert_eq!(answer.cancel().await, Some(42));
    ///     // ...and one that did not has let go of what it held by the time the wait is over.
    ///     assert_eq!(reader.cancel().await, None);
    ///     assert_eq!(Rc::strong_count(&connection), 1);
    /// });
    /// ```
    pub fn cancel(self) -> impl Future<Output = Option<T>> {
        self.0.cancel()
    }

    /// Whether the task has ended, without polling it or taking its output.
    ///
    /// A task has ended once its future has completed or panicked, or was dropped with its runtime.
    /// In each case, the future, and everything it held, has been dropped by the time this returns
    /// `true`.
    ///
    /// It stays `true` after awaiting the task has taken its output. Use it to find out that a task
    /// is over when you do not want to take its output.
    ///
    /// # Example
    ///
    /// ```
    /// use zruntime::LocalRuntime;
    ///
    /// let runtime = LocalRuntime::new().unwrap();
    /// let mut task = runtime.spawn("an answer", async { 42 });
    /// assert!(!task.is_finished());
    ///
    /// assert_eq!(runtime.block_on(&mut task).unwrap(), 42);
    /// assert!(task.is_finished());
    /// ```
    pub fn is_finished(&self) -> bool {
        self.0.is_finished()
    }
}

#[cfg(feature = "runtime")]
impl<T, M> fmt::Debug for Task<T, M>
where
    M: Mode,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Task").field(&self.0).finish()
    }
}

#[cfg(feature = "runtime")]
impl<T, M> Future for Task<T, M>
where
    M: Mode,
{
    type Output = io::Result<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.poll_join(cx)
    }
}

/// Queues `future` on the shared runtime `core` as a task named `name`: what [`Runtime::spawn`]
/// and the free [`spawn`] have in common, so that a task spawned either way is seen to alike.
#[cfg(feature = "runtime")]
fn spawn_on_shared<F>(core: &Arc<Core<Shared>>, name: Name, future: F) -> Task<F::Output, Shared>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let task = Task(scheduler::spawn_shared(core, name, future));
    // Asked for once the task is on the scheduler's queue, so that a helper starting here finds it
    // there.
    core.ensure_progress();

    task
}

/// The shared runtime the free [`spawn`] puts a task on: the one the calling thread drives, and
/// where it drives none, the one `SharedRuntime::current` hands out, if the `helper` feature is
/// there to make it.
#[cfg(feature = "runtime")]
#[track_caller]
fn shared_to_spawn_on() -> Arc<Core<Shared>> {
    if let Some(core) = Shared::driven() {
        return core;
    }
    #[cfg(feature = "helper")]
    return match driver::current_to_spawn_on() {
        Ok(core) => core,
        Err(error) => panic!("spawn found no runtime to spawn on, and could not make one: {error}"),
    };
    #[cfg(not(feature = "helper"))]
    panic!(
        "spawn called where no shared runtime is running: call it from inside \
         `SharedRuntime::block_on`, or from a task of that runtime, or enable the `helper` feature"
    );
}

#[cfg(any(test, doctest))]
mod tests;
